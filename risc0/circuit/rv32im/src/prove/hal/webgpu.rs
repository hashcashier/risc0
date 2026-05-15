// Copyright 2026 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use risc0_core::scope;
use risc0_zkp::{
    adapter::{CircuitInfo as _, PROOF_SYSTEM_INFO},
    field::Elem as _,
    hal::{
        webgpu::{
            WebGpuBindingLayout, WebGpuBuffer, WebGpuBufferBinding, WebGpuCircuitEvalCheck,
            WebGpuHal, WebGpuKernel, WebGpuStageTimer,
        },
        AccumPreflight, Buffer, CircuitHal, Hal,
    },
    prove::Prover,
};

use super::{
    CircuitAccumulator, CircuitWitnessGenerator, MetaBuffer, PreflightResults, SegmentProver,
    SegmentProverImpl, StepMode,
};
use crate::{
    prove::witgen::preflight::PreflightTrace,
    zirgen::{
        circuit::{
            ExtVal, Val, REGCOUNT_MIX, REGISTER_GROUP_ACCUM, REGISTER_GROUP_CODE,
            REGISTER_GROUP_DATA,
        },
        taps::TAPSET,
        CircuitImpl,
    },
    RV32IM_SEAL_VERSION,
};

// SP7 iter 6d-c (2026-05-15): hold the HAL handle so generate_witness
// can dispatch the GPU exec_TopChunk0 kernel alongside the CPU
// rust_steps reference. Probe-mode for now: GPU output is timed and
// dropped; rust_steps remains the authority.
//
// `witgen_gpu_probe_enabled` is opt-in (default off) since the first
// dispatch triggers a ~60 s Tint compile of the 1.08 MB pruned WGSL
// module -- enabling it on a baseline xgboost run would add ~60 s wall
// for ~6 s savings ceiling (see project_sp7_witgen_savings_ceiling).
// Tests set the flag explicitly; the probe is the measurement
// infrastructure iter-6d-d/e need to design pre-warm + dispatch.
//
// The first dispatch lazily fills `witgen_top_chunk0_kernel`; subsequent
// segments reuse the cached pipeline + layout for free.
/// SP7 iter 6d-c: process-global flag that turns on the probe-mode GPU
/// witgen dispatch. Tests flip this before `webgpu_prover()` is
/// constructed; production runs leave it off. Atomic so it can be read
/// from sync paths without RefCell borrow churn.
pub static WITGEN_GPU_PROBE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Public setter for the iter-6d-c probe flag.
pub fn set_witgen_gpu_probe_enabled(enabled: bool) {
    WITGEN_GPU_PROBE_ENABLED.store(enabled, Ordering::SeqCst);
}

thread_local! {
    /// SP7 iter 6d-d: session-local cache for the witgen kernel. Lives
    /// across WebGpuCircuitHal constructions so the spawn_local'd
    /// async prewarm task's result is reachable from every segment's
    /// `dispatch_witgen_top_chunk0_probe` call. (ProverImpl's
    /// segment_prover constructs a fresh WebGpuCircuitHal per
    /// segment, so a struct field would defeat the cache.)
    static WITGEN_TOP_CHUNK0_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// SP7 iter 6d-e: chunk1 sibling kernel cache.
    static WITGEN_TOP_CHUNK1_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// SP7 iter 6d-g step 4: per-arm kernel cache for TOP_CHUNK0_ARM_DELTAS.
    /// Keyed by arm label (the first element of each tuple in
    /// TOP_CHUNK0_ARM_DELTAS), one entry per major opcode arm.
    /// Populated by the spawn_local prewarm task; consumed by
    /// `dispatch_witgen_arms_probe`.
    static WITGEN_ARM_KERNELS: RefCell<std::collections::BTreeMap<&'static str, WebGpuKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Tracks whether the async prewarm task has been spawned this
    /// session, so multiple `prewarm_witgen_kernel()` calls (one per
    /// segment_prover) only fire the compile once.
    static WITGEN_PREWARM_SPAWNED: Cell<bool> = const { Cell::new(false) };
    /// SP7 iter 6d-g step 6.2.0: cached shadow_init pipeline.
    /// Compiled once per session and reused across all segments.
    static SHADOW_INIT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
}

/// SP7 iter 6d-g step 6.2.0 (2026-05-16): build the per-cycle preflight
/// metadata buffer consumed by `SHADOW_INIT_WGSL`. Layout: 4 u32 per
/// cycle = `[pc, state, machine_mode, packed_minor_major]`. The packed
/// field is `(major as u32) << 16 | (minor as u32)` so per-arm wrappers
/// can extract both with one buffer read.
fn build_preflight_meta(preflight: &PreflightTrace) -> Vec<u32> {
    let mut out = Vec::with_capacity(preflight.cycles.len() * 4);
    for cycle in &preflight.cycles {
        out.push(cycle.pc);
        out.push(cycle.state);
        out.push(cycle.machine_mode as u32);
        out.push(((cycle.major as u32) << 16) | (cycle.minor as u32));
    }
    out
}

/// SP7 iter 6d-g step 6.2.1c (2026-05-16): synthesize per-arm
/// @compute wrapper that replicates `exec_TopChunk0`'s logic to
/// construct `InstInputStruct` from preflight + shadow-init'd cells
/// and then calls the arm sub-fn directly. Replaces the no-op
/// `data_buf[cycle] = data_buf[cycle]` placeholder with the real
/// witgen call path.
///
/// Generalized over (arm_idx, sub_fn). For ECall0 (arm_idx 8), the
/// sub-fn signature has an extra `global: u32` arg. All other arms
/// use the standard 3-arg signature.
///
/// Only safe for the 8 zero-back_Reg arms identified in the
/// 2026-05-16 audit (MISC0/1/2, MUL0, DIV0, MEM0/1, ECALL0). Arms
/// with internal back_Reg deps (CONTROL0, POSEIDON0/1, SHA0, BIGINT0)
/// would read uninitialized cells and produce garbage; keep them
/// no-op until a deeper materialization scheme lands.
fn synth_arm_wrapper(label: &str, sub_fn: &str, arm_idx: usize) -> String {
    // ECall0 (index 8) takes an extra `global: u32` arg.
    let extra_arg = if arm_idx == 8 { ", buf_global" } else { "" };
    format!(
        "@group(0) @binding(5) var<storage, read> cycle_list: array<u32>;\n\
         @group(0) @binding(6) var<storage, read> preflight_meta: array<u32>;\n\
         \n\
         @compute @workgroup_size(64)\n\
         fn iter6d_g_{label}_main(@builtin(global_invocation_id) gid: vec3<u32>) {{\n\
           let lane = gid.x;\n\
           if (lane >= arrayLength(&cycle_list)) {{ return; }}\n\
           cycle = cycle_list[lane];\n\
           if (cycle >= params.data_rows) {{ return; }}\n\
           let bound_top = BoundLayout_TopLayout(kLayout_Top, buf_data);\n\
           let base = cycle * 4u;\n\
           let packed = preflight_meta[base + 3u];\n\
           let major_v = encode(packed >> 16u);\n\
           let minor_v = encode(packed & 0xFFFFu);\n\
           let is_first_v = select(0u, encode(1u), cycle == 0u);\n\
           let x3 = exec_NondetBitReg(is_first_v, lookup_TopLayout_isFirstCycle(bound_top));\n\
           let x4 = sub(MONT_ONE, x3._super);\n\
           let x9 = back_Reg(1, lookup_TopLayout_nextPcLow(bound_top));\n\
           let x10 = back_Reg(1, lookup_TopLayout_nextPcHigh(bound_top));\n\
           let x11 = back_Reg(1, lookup_TopLayout_nextState_0(bound_top));\n\
           let x12 = back_Reg(1, lookup_TopLayout_nextMachineMode(bound_top));\n\
           let x15 = exec_NondetReg(major_v, lookup_TopLayout_major(bound_top));\n\
           let x16 = exec_NondetReg(minor_v, lookup_TopLayout_minor(bound_top));\n\
           let x17 = exec_InstInput(\n\
             x15._super, x16._super,\n\
             ValU32Struct(mul(x4, x9._super), mul(x4, x10._super)),\n\
             mul(x4, x11._super),\n\
             add(mul(x4, x12._super), x3._super),\n\
             lookup_TopLayout_instInput(bound_top)\n\
           );\n\
           let x20 = back_Reg(0, lookup_TopCycleLayout__super(lookup_TopLayout_cycleRedef(bound_top)));\n\
           let _result = {sub_fn}(x20, x17, lookup_TopInstResultLayout_arm{arm_idx}(lookup_TopLayout_instResult(bound_top)){extra_arg});\n\
         }}\n",
        label = label,
        sub_fn = sub_fn,
        arm_idx = arm_idx,
        extra_arg = extra_arg,
    )
}

/// SP7 iter 6d-g step 6.2.1c: arms with zero internal back_Reg calls
/// per the 2026-05-16 audit. Safe to GPU-witgen-replace given outer
/// shadow-init. Indexed by major opcode.
const ZERO_BACK_REG_ARMS: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 8];

fn is_zero_back_reg_arm(arm_idx: usize) -> bool {
    ZERO_BACK_REG_ARMS.contains(&arm_idx)
}

#[allow(dead_code)]
pub(crate) struct WebGpuCircuitHal {
    hal: Rc<WebGpuHal>,
}

/// Concatenation of the vendored exec_TopChunk0 pruned module and the thin
/// `@compute @workgroup_size(64) fn exec_top_chunk0_main` entry wrapper.
/// naga-validated by `iter6d_a_compute_entry_concat_validates_with_naga`
/// (cargo-test side) and Tint-validated by
/// `iter6d_a_exec_top_chunk0_compiles_on_chrome` (wasm-bindgen side).
const WITGEN_TOP_CHUNK0_WGSL: &str = concat!(
    include_str!("../../zirgen/exec_top_chunk0.wgsl"),
    "\n",
    "@compute @workgroup_size(64)\n",
    "fn exec_top_chunk0_main(@builtin(global_invocation_id) gid: vec3<u32>) {\n",
    "  cycle = gid.x;\n",
    "  if (cycle >= params.data_rows) {\n",
    "    return;\n",
    "  }\n",
    "  let bound = BoundLayout_TopLayout(kLayout_Top, buf_data);\n",
    "  let _result = exec_TopChunk0(bound, buf_global);\n",
    "}\n",
);

/// SP7 iter 6d-e: chunk1 sibling of [`WITGEN_TOP_CHUNK0_WGSL`]. Same
/// shape but uses the chunk1-everywhere pruned module + an
/// `exec_top_chunk1_main` entry.
const WITGEN_TOP_CHUNK1_WGSL: &str = concat!(
    include_str!("../../zirgen/exec_top_chunk1.wgsl"),
    "\n",
    "@compute @workgroup_size(64)\n",
    "fn exec_top_chunk1_main(@builtin(global_invocation_id) gid: vec3<u32>) {\n",
    "  cycle = gid.x;\n",
    "  if (cycle >= params.data_rows) {\n",
    "    return;\n",
    "  }\n",
    "  let bound = BoundLayout_TopLayout(kLayout_Top, buf_data);\n",
    "  let _result = exec_TopChunk1(bound, buf_global);\n",
    "}\n",
);

impl WebGpuCircuitHal {
    pub(crate) fn new(hal: Rc<WebGpuHal>) -> Self {
        Self { hal }
    }

    /// SP7 iter 6d-d (2026-05-15): kick off the witgen kernel Tint
    /// compile asynchronously. The browser GPU process compiles in
    /// parallel with the wasm thread's guest execution + session
    /// setup; by the time `WebGpuCircuitHal::generate_witness` runs,
    /// the kernel may already be ready in the thread-local cache.
    ///
    /// Called from `segment_prover()` after `WebGpuCircuitHal::new()`.
    /// Idempotent across multiple calls in one session: the
    /// `WITGEN_PREWARM_SPAWNED` thread_local gate ensures the compile
    /// runs at most once per session.
    pub fn prewarm_witgen_kernel(&self) {
        if !WITGEN_GPU_PROBE_ENABLED.load(Ordering::SeqCst) {
            return;
        }
        if WITGEN_PREWARM_SPAWNED.with(|spawned| {
            if spawned.get() {
                true
            } else {
                spawned.set(true);
                false
            }
        }) {
            return; // already spawned this session
        }
        let hal = self.hal.clone();
        wasm_bindgen_futures::spawn_local(async move {
            // SP7 iter 6d-e: compile both top-mux chunks. Chrome
            // pipelines createComputePipelineAsync internally so the
            // two compiles can overlap with each other and with
            // session execution. Measured wall on xgboost: chunk0
            // compile ~2.65 s, chunk1 ~similar.
            let _t = WebGpuStageTimer::new("iter6d_d_witgen_prewarm_async");
            let layout = match hal.create_bind_group_layout(
                "iter6d_c_witgen_layout",
                &[
                    WebGpuBindingLayout::storage(0, 0),
                    WebGpuBindingLayout::storage(1, 0),
                    WebGpuBindingLayout::storage(2, 0),
                    WebGpuBindingLayout::storage(3, 0),
                    WebGpuBindingLayout::uniform(4, 32),
                ],
            ) {
                Ok(layout) => layout,
                Err(err) => {
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "iter6d_d_witgen_prewarm_async layout_FAILED err={err:?}"
                    ));
                    return;
                }
            };
            // Kick off both compiles before awaiting either; the
            // browser-side promises run in parallel.
            let layouts0 = [layout.clone()];
            let layouts1 = [layout.clone()];
            let chunk0_fut = hal.create_compute_kernel_async(
                "iter6d_c_witgen_kernel_chunk0",
                WITGEN_TOP_CHUNK0_WGSL,
                "exec_top_chunk0_main",
                &layouts0,
            );
            let chunk1_fut = hal.create_compute_kernel_async(
                "iter6d_e_witgen_kernel_chunk1",
                WITGEN_TOP_CHUNK1_WGSL,
                "exec_top_chunk1_main",
                &layouts1,
            );
            match chunk0_fut.await {
                Ok(kernel) => {
                    WITGEN_TOP_CHUNK0_KERNEL
                        .with(|cell| *cell.borrow_mut() = Some(kernel));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(
                        "iter6d_d_witgen_prewarm_async chunk0 DONE",
                    );
                }
                Err(err) => {
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "iter6d_d_witgen_prewarm_async chunk0_FAILED err={err:?}"
                    ));
                }
            }
            match chunk1_fut.await {
                Ok(kernel) => {
                    WITGEN_TOP_CHUNK1_KERNEL
                        .with(|cell| *cell.borrow_mut() = Some(kernel));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(
                        "iter6d_e_witgen_prewarm_async chunk1 DONE",
                    );
                }
                Err(err) => {
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "iter6d_e_witgen_prewarm_async chunk1_FAILED err={err:?}"
                    ));
                }
            }

            // SP7 iter 6d-g step 4: compile all per-arm kernels in
            // parallel. Each kernel is ~830 KB; Chrome pipelines the
            // create_compute_pipeline_async promises through its GPU
            // process queue. Total wall on the iter-6d-g step 3
            // smoke (all 13 arms): 8.46 s for full compile + dispatch,
            // ~650 ms average per kernel. With async pipelining + this
            // overlapping with rv32im segment 1 prove, the per-arm
            // kernels are ready by segments 2-11.
            use crate::prove::wgsl_pruner::{
                assemble_arm_kernel, TOP_CHUNK0_ARM_DELTAS,
            };
            // SP7 iter 6d-g step 6.2.1a: per-arm layout adds binding 6
            // `preflight_meta: array<u32>` so per-arm wrappers can
            // extract major/minor per cycle (4 u32 per cycle, packed
            // major<<16|minor at index 3). Bindings 0-4 are the
            // standard witgen bindings (data, global, accum, mix,
            // params); 5 is cycle_list; 6 is preflight_meta. The
            // wrapper body remains a no-op for now -- step 6.2.1b
            // wires real InstInputStruct synthesis for the 8
            // zero-back_Reg arms.
            let arm_layout = match hal.create_bind_group_layout(
                "iter6d_g_arm_layout",
                &[
                    WebGpuBindingLayout::storage(0, 0),
                    WebGpuBindingLayout::storage(1, 0),
                    WebGpuBindingLayout::storage(2, 0),
                    WebGpuBindingLayout::storage(3, 0),
                    WebGpuBindingLayout::uniform(4, 32),
                    WebGpuBindingLayout::storage(5, 0),
                    WebGpuBindingLayout::storage(6, 0),
                ],
            ) {
                Ok(l) => l,
                Err(err) => {
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "iter6d_g_prewarm arm_layout_FAILED err={err:?}"
                    ));
                    return;
                }
            };
            let arm_layouts = [arm_layout.clone()];
            let mut arm_modules: Vec<(String, String, &str)> =
                Vec::with_capacity(TOP_CHUNK0_ARM_DELTAS.len());
            for (arm_idx, (label, delta, sub_fn)) in TOP_CHUNK0_ARM_DELTAS.iter().enumerate() {
                // SP7 iter 6d-g step 6.2.1c: synthesize real per-arm
                // wrapper for the 8 zero-back_Reg arms; keep no-op for
                // the 5 inter-cycle-heavy arms (CONTROL0, BIGINT0,
                // POSEIDON0/1, SHA0) until a deeper materialization
                // scheme lands.
                let wrapper = if is_zero_back_reg_arm(arm_idx) {
                    synth_arm_wrapper(label, sub_fn, arm_idx)
                } else {
                    format!(
                        "@group(0) @binding(5) var<storage, read> cycle_list: array<u32>;\n\
                         @group(0) @binding(6) var<storage, read> preflight_meta: array<u32>;\n\
                         \n\
                         @compute @workgroup_size(64)\n\
                         fn iter6d_g_{}_main(@builtin(global_invocation_id) gid: vec3<u32>) {{\n\
                           let lane = gid.x;\n\
                           if (lane >= arrayLength(&cycle_list)) {{ return; }}\n\
                           cycle = cycle_list[lane];\n\
                           if (cycle >= params.data_rows) {{ return; }}\n\
                           let _meta = preflight_meta[cycle * 4u + 3u];\n\
                           data_buf[cycle] = data_buf[cycle];\n\
                         }}\n",
                        label,
                    )
                };
                let module = assemble_arm_kernel(delta, &wrapper);
                let entry = format!("iter6d_g_{}_main", label);
                arm_modules.push((module, entry, *label));
            }
            // Kick off all 13 compiles in parallel, then await each
            // in sequence -- Chrome pipelines the GPU-process work
            // even though Rust awaits serially.
            let label_strs: Vec<&'static str> =
                arm_modules.iter().map(|(_, _, l)| *l).collect();
            let entry_strs: Vec<String> =
                arm_modules.iter().map(|(_, e, _)| e.clone()).collect();
            let futures: Vec<_> = arm_modules
                .iter()
                .enumerate()
                .map(|(i, (module, entry, _))| {
                    let _label: &'static str = label_strs[i];
                    let entry: &str = entry;
                    hal.create_compute_kernel_async(
                        "iter6d_g_arm_kernel",
                        module,
                        entry,
                        &arm_layouts,
                    )
                })
                .collect();
            for (i, fut) in futures.into_iter().enumerate() {
                match fut.await {
                    Ok(kernel) => {
                        let label = label_strs[i];
                        WITGEN_ARM_KERNELS
                            .with(|cell| cell.borrow_mut().insert(label, kernel));
                        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                            "iter6d_g_prewarm arm={} DONE", label
                        ));
                    }
                    Err(err) => {
                        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                            "iter6d_g_prewarm arm={} FAILED err={err:?}",
                            label_strs[i]
                        ));
                    }
                }
            }
            let _ = entry_strs; // keep strings alive across the async run
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "iter6d_g_prewarm ALL arms requested={}",
                TOP_CHUNK0_ARM_DELTAS.len(),
            ));
        });
    }

    /// Returns the witgen kernel if the iter-6d-d async prewarm task
    /// finished. None means "not ready yet" -- caller skips the GPU
    /// dispatch and relies on rust_steps.
    fn lookup_witgen_top_chunk0_kernel(&self) -> Option<WebGpuKernel> {
        WITGEN_TOP_CHUNK0_KERNEL.with(|cell| cell.borrow().clone())
    }

    fn lookup_witgen_top_chunk1_kernel(&self) -> Option<WebGpuKernel> {
        WITGEN_TOP_CHUNK1_KERNEL.with(|cell| cell.borrow().clone())
    }

    /// SP7 iter 6d-g step 6 (partial -- dispatch infrastructure only):
    /// build 13 per-arm cycle-list buffers from preflight and dispatch
    /// each prewarmed per-arm kernel over its cycle subset. Kernels
    /// are still no-ops (data_buf[cycle] = data_buf[cycle]) per
    /// iter-6d-g step 4, so this contributes no perf yet -- but
    /// validates that the multi-kernel dispatch path runs cleanly in
    /// the prove pipeline.
    ///
    /// Future iter-6d-g step 6 work: replace no-op wrappers with
    /// arm-specific InstInputStruct + BoundLayout construction +
    /// sub-fn call. Then rust_steps::step_exec can be short-circuited
    /// for cycles covered by GPU dispatch.
    /// SP7 iter 6d-g step 6.2.0: dispatch the shadow_init kernel before
    /// any per-arm dispatch. Pre-populates the 5 outer Top layout cells
    /// (nextPcLow/High, nextState_0, nextMachineMode, isFirstCycle) in
    /// `data_buf` from a per-cycle preflight metadata buffer. After this
    /// runs, per-arm wrappers' `back_Reg(1, ...)` reads return correct
    /// values without rust_steps having executed.
    ///
    /// Self-contained kernel uses 3 bindings (data_buf, params,
    /// preflight_meta) -- doesn't need the witgen baseline.
    fn dispatch_shadow_init(
        &self,
        data: &MetaBuffer<WebGpuHal>,
        preflight: &PreflightTrace,
    ) -> Result<()> {
        use crate::prove::wgsl_pruner::SHADOW_INIT_WGSL;
        let _t = WebGpuStageTimer::new(format!(
            "iter6d_g_shadow_init cycles={}",
            preflight.cycles.len()
        ));
        let kernel =
            SHADOW_INIT_KERNEL.with(|cell| cell.borrow().clone());
        let kernel = match kernel {
            Some(k) => k,
            None => {
                let layout = self.hal.create_bind_group_layout(
                    "iter6d_g_shadow_init_layout",
                    &[
                        WebGpuBindingLayout::storage(0, 0),
                        WebGpuBindingLayout::uniform(1, 16),
                        WebGpuBindingLayout::storage(2, 0),
                    ],
                )?;
                let k = self.hal.create_compute_kernel(
                    "iter6d_g_shadow_init",
                    SHADOW_INIT_WGSL,
                    "shadow_init_main",
                    &[layout],
                )?;
                SHADOW_INIT_KERNEL.with(|cell| *cell.borrow_mut() = Some(k.clone()));
                k
            }
        };
        // Build + upload preflight metadata.
        let meta = build_preflight_meta(preflight);
        let meta_bytes: &[u8] = bytemuck::cast_slice(meta.as_slice());
        let meta_buf = self.hal.create_storage_buffer(
            "iter6d_g_shadow_meta",
            meta_bytes.len() as u64,
        )?;
        self.hal.write_buffer_named(
            &meta_buf,
            "iter6d_g_shadow_meta",
            0,
            meta_bytes,
        )?;
        let total_cycles = data.rows as u32;
        let params: [u32; 4] = [total_cycles, data.cols as u32, 0, 0];
        let params_buf = self.hal.create_uniform_buffer(
            "iter6d_g_shadow_params",
            bytemuck::cast_slice(&params),
        )?;
        let layout = self.hal.create_bind_group_layout(
            "iter6d_g_shadow_init_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::uniform(1, 16),
                WebGpuBindingLayout::storage(2, 0),
            ],
        )?;
        let data_gpu = data
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("iter-6d-g shadow_init: data missing GPU storage"))?;
        let bind_group = self.hal.create_bind_group(
            "iter6d_g_shadow_bg",
            &layout,
            &[
                WebGpuBufferBinding::new(0, data_gpu),
                WebGpuBufferBinding::new(1, &params_buf),
                WebGpuBufferBinding::new(2, &meta_buf),
            ],
        )?;
        let workgroups = total_cycles.div_ceil(64);
        self.hal.dispatch_compute_1d(&kernel, &bind_group, workgroups);
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "iter6d_g_shadow_init cycles={} meta_bytes={}",
            total_cycles, meta_bytes.len(),
        ));
        // Keep buffers alive until queue flushes.
        drop(bind_group);
        drop(meta_buf);
        drop(params_buf);
        Ok(())
    }

    fn dispatch_witgen_per_arm_probe(
        &self,
        data: &MetaBuffer<WebGpuHal>,
        preflight: &PreflightTrace,
    ) -> Result<()> {
        use crate::prove::wgsl_pruner::TOP_CHUNK0_ARM_DELTAS;
        // Build per-arm cycle lists from preflight (CPU-side, fast).
        let mut per_arm_cycles: Vec<Vec<u32>> =
            vec![Vec::new(); TOP_CHUNK0_ARM_DELTAS.len()];
        for (cycle_idx, cycle) in preflight.cycles.iter().enumerate() {
            let arm = cycle.major as usize;
            if arm < per_arm_cycles.len() {
                per_arm_cycles[arm].push(cycle_idx as u32);
            }
        }
        let _t = WebGpuStageTimer::new(format!(
            "iter6d_g_per_arm_dispatch arms={} total_cycles={}",
            TOP_CHUNK0_ARM_DELTAS.len(),
            preflight.cycles.len(),
        ));
        // iter-6d-g step 6.2.1a: layout now has binding 5 (cycle_list)
        // and binding 6 (preflight_meta). Per-arm wrappers read major/
        // minor from preflight_meta via packed_minor_major at index 3.
        let layout = self.hal.create_bind_group_layout(
            "iter6d_g_arm_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
                WebGpuBindingLayout::storage(5, 0),
                WebGpuBindingLayout::storage(6, 0),
            ],
        )?;
        let total_cycles = data.rows as u32;
        let placeholder_bytes: u64 = 256;
        let accum_buf = self
            .hal
            .create_storage_buffer("iter6d_g_arm_accum_ph", placeholder_bytes)?;
        let mix_buf = self
            .hal
            .create_storage_buffer("iter6d_g_arm_mix_ph", placeholder_bytes)?;
        let params: [u32; 8] = [total_cycles, 1, total_cycles, 1, 0, 0, 0, 0];
        let params_buf = self.hal.create_uniform_buffer(
            "iter6d_g_arm_params_ph",
            bytemuck::cast_slice(&params),
        )?;
        // Upload preflight_meta for per-arm wrappers (separate buffer
        // from the shadow_init kernel's upload; could be shared in a
        // future tightening, but keeping separate avoids cross-pass
        // ownership coupling).
        let meta = build_preflight_meta(preflight);
        let meta_bytes: &[u8] = bytemuck::cast_slice(meta.as_slice());
        let preflight_buf = self
            .hal
            .create_storage_buffer("iter6d_g_arm_preflight", meta_bytes.len() as u64)?;
        self.hal.write_buffer_named(
            &preflight_buf,
            "iter6d_g_arm_preflight",
            0,
            meta_bytes,
        )?;
        let data_gpu = data
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("iter-6d-g: data missing GPU storage"))?;
        let global_gpu = data
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("iter-6d-g: global missing GPU storage"))?;
        let mut dispatched = 0usize;
        let mut skipped = 0usize;
        // Keep per-arm cycle_list buffers alive across the dispatch
        // loop so the GPU encoder can reference them when the queue
        // flushes at end-of-scope.
        let mut cycle_list_buffers: Vec<_> = Vec::with_capacity(TOP_CHUNK0_ARM_DELTAS.len());
        let mut bind_groups: Vec<_> = Vec::with_capacity(TOP_CHUNK0_ARM_DELTAS.len());
        for (arm_idx, (label, _, _)) in TOP_CHUNK0_ARM_DELTAS.iter().enumerate() {
            let cycle_count = per_arm_cycles[arm_idx].len();
            if cycle_count == 0 {
                continue;
            }
            let kernel = WITGEN_ARM_KERNELS.with(|cell| cell.borrow().get(*label).cloned());
            let Some(kernel) = kernel else {
                skipped += 1;
                continue;
            };
            // Per-arm cycle list upload (storage buffer + queue write).
            let cycle_bytes: &[u8] = bytemuck::cast_slice(per_arm_cycles[arm_idx].as_slice());
            let cycle_buf = self.hal.create_storage_buffer(
                "iter6d_g_arm_cycle_list",
                cycle_bytes.len() as u64,
            )?;
            self.hal.write_buffer_named(
                &cycle_buf,
                "iter6d_g_arm_cycle_list",
                0,
                cycle_bytes,
            )?;
            let bind_group = self.hal.create_bind_group(
                "iter6d_g_arm_bg",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, data_gpu),
                    WebGpuBufferBinding::new(1, global_gpu),
                    WebGpuBufferBinding::new(2, &accum_buf),
                    WebGpuBufferBinding::new(3, &mix_buf),
                    WebGpuBufferBinding::new(4, &params_buf),
                    WebGpuBufferBinding::new(5, &cycle_buf),
                    WebGpuBufferBinding::new(6, &preflight_buf),
                ],
            )?;
            let workgroups = (cycle_count as u32).div_ceil(64);
            self.hal.dispatch_compute_1d(&kernel, &bind_group, workgroups);
            cycle_list_buffers.push(cycle_buf);
            bind_groups.push(bind_group);
            dispatched += 1;
        }
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "iter6d_g_per_arm_dispatch dispatched={} skipped={}",
            dispatched, skipped,
        ));
        // Keep buffers + bind groups alive until dispatch completes.
        drop(bind_groups);
        drop(cycle_list_buffers);
        drop(preflight_buf);
        Ok(())
    }

    fn dispatch_witgen_top_chunk0_probe(
        &self,
        data: &MetaBuffer<WebGpuHal>,
        global: &MetaBuffer<WebGpuHal>,
        total_cycles: u32,
    ) -> Result<()> {
        // iter-6d-e: dispatch chunk0 and chunk1 (if available). Each
        // kernel internally filters by major opcode arm via its mux
        // dispatch; cycles whose opcode is outside the kernel's arms
        // execute the trailing `unreachable` branch (effectively a
        // no-op since the kernel writes nothing in that case).
        // rust_steps still runs after and overwrites all cells so
        // output remains authoritative.
        let chunk0 = self.lookup_witgen_top_chunk0_kernel();
        let chunk1 = self.lookup_witgen_top_chunk1_kernel();
        let kernels: Vec<WebGpuKernel> = [chunk0, chunk1].into_iter().flatten().collect();
        if kernels.is_empty() {
            risc0_zkp::hal::webgpu::log_webgpu_metric(
                "iter6d_c_witgen_probe SKIP kernels_not_ready",
            );
            return Ok(());
        }
        let _t = WebGpuStageTimer::new(format!(
            "iter6d_c_witgen_probe cycles={} chunks={}",
            total_cycles,
            kernels.len(),
        ));
        let layout = self.hal.create_bind_group_layout(
            "iter6d_c_witgen_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;

        // The kernel's WGSL declares accum/mix/params bindings but the
        // witgen path only reads data + global; allocate small placeholders.
        let placeholder_bytes: u64 = 256;
        let accum_buf = self
            .hal
            .create_storage_buffer("iter6d_c_witgen_accum_placeholder", placeholder_bytes)?;
        let mix_buf = self
            .hal
            .create_storage_buffer("iter6d_c_witgen_mix_placeholder", placeholder_bytes)?;
        let params: [u32; 8] = [total_cycles, 1, total_cycles, 1, 0, 0, 0, 0];
        let params_bytes: &[u8] = bytemuck::cast_slice(&params);
        let params_buf = self
            .hal
            .create_uniform_buffer("iter6d_c_witgen_params_placeholder", params_bytes)?;

        let data_gpu = data
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("iter-6d-c: data buffer missing GPU storage"))?;
        let global_gpu = global
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("iter-6d-c: global buffer missing GPU storage"))?;
        let bind_group = self.hal.create_bind_group(
            "iter6d_c_witgen_bg",
            &layout,
            &[
                WebGpuBufferBinding::new(0, data_gpu),
                WebGpuBufferBinding::new(1, global_gpu),
                WebGpuBufferBinding::new(2, &accum_buf),
                WebGpuBufferBinding::new(3, &mix_buf),
                WebGpuBufferBinding::new(4, &params_buf),
            ],
        )?;

        let workgroups = total_cycles.div_ceil(64);
        for kernel in &kernels {
            self.hal.dispatch_compute_1d(kernel, &bind_group, workgroups);
        }
        Ok(())
    }
}

impl WebGpuCircuitEvalCheck for WebGpuCircuitHal {
    fn eval_check_webgpu(
        &self,
        hal: &WebGpuHal,
        check: &WebGpuBuffer<Val>,
        groups: &[&WebGpuBuffer<Val>],
        globals: &[&WebGpuBuffer<Val>],
        poly_mix: ExtVal,
        po2: usize,
        steps: usize,
    ) -> Result<bool> {
        hal.dispatch_eval_check_poly_ext(
            check,
            groups,
            globals,
            TAPSET,
            &crate::zirgen::poly_ext::DEF,
            poly_mix,
            po2,
            steps,
        )
    }
}

impl CircuitWitnessGenerator<WebGpuHal> for WebGpuCircuitHal {
    fn generate_witness(
        &self,
        mode: StepMode,
        preflight: &PreflightTrace,
        global: &MetaBuffer<WebGpuHal>,
        data: &MetaBuffer<WebGpuHal>,
    ) -> Result<()> {
        let _timer = WebGpuStageTimer::new(format!(
            "rv32im_witgen mode={} cycles={} txns={} bigint_bytes={} data_rows={} data_cols={}",
            step_mode_label(mode),
            preflight.cycles.len(),
            preflight.txns.len(),
            preflight.bigint_bytes.len(),
            data.rows,
            data.cols
        ));
        // SP7 iter 6d-c (2026-05-15): probe-mode GPU dispatch alongside
        // rust_steps. Default off. Tests flip the process-global flag
        // via `set_witgen_gpu_probe_enabled(true)` to measure the
        // per-segment GPU dispatch wall and the one-time Tint compile;
        // probe output is discarded so rust_steps remains the witness.
        if WITGEN_GPU_PROBE_ENABLED.load(Ordering::SeqCst) {
            let total_cycles = data.rows as u32;
            if let Err(err) = self.dispatch_witgen_top_chunk0_probe(data, global, total_cycles) {
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_c_witgen_probe FAILED err={err:?}"
                ));
            }
            // iter-6d-g step 6.2.0: shadow-init the 5 outer Top layout
            // cells (nextPcLow/High, nextState_0, nextMachineMode,
            // isFirstCycle) from preflight before any per-arm dispatch
            // so back_Reg(1, ...) reads return correct values without
            // needing rust_steps to have run first.
            if let Err(err) = self.dispatch_shadow_init(data, preflight) {
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_shadow_init FAILED err={err:?}"
                ));
            }
            // iter-6d-g step 6 (partial): per-arm dispatch validates
            // the multi-kernel dispatch path works in the prove
            // pipeline. Kernels are no-ops so no perf yet.
            if let Err(err) = self.dispatch_witgen_per_arm_probe(data, preflight) {
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_per_arm_dispatch FAILED err={err:?}"
                ));
            }
        }
        super::rust_steps::generate_witness(mode, preflight, global, data)
    }
}

impl CircuitAccumulator<WebGpuHal> for WebGpuCircuitHal {
    fn step_accum(
        &self,
        preflight: &PreflightTrace,
        data: &MetaBuffer<WebGpuHal>,
        accum: &MetaBuffer<WebGpuHal>,
        global: &MetaBuffer<WebGpuHal>,
        mix: &MetaBuffer<WebGpuHal>,
    ) -> Result<()> {
        let _timer = WebGpuStageTimer::new(format!(
            "rv32im_accumulate cycles={} data_rows={} accum_rows={}",
            preflight.cycles.len(),
            data.rows,
            accum.rows
        ));
        super::rust_steps::step_accum(preflight, data, accum, global, mix)
    }
}

impl CircuitHal<WebGpuHal> for WebGpuCircuitHal {
    fn eval_check(
        &self,
        check: &WebGpuBuffer<Val>,
        groups: &[&WebGpuBuffer<Val>],
        globals: &[&WebGpuBuffer<Val>],
        poly_mix: ExtVal,
        po2: usize,
        steps: usize,
    ) {
        let _timer = WebGpuStageTimer::new(format!(
            "rv32im_eval_check po2={} steps={} domain={}",
            po2,
            steps,
            steps * risc0_zkp::INV_RATE
        ));
        risc0_zkp::hal::portable::eval_check::<WebGpuHal, CircuitImpl>(
            &CircuitImpl,
            check,
            groups,
            globals,
            poly_mix,
            po2,
            steps,
        );
    }

    fn accumulate(
        &self,
        _preflight: &AccumPreflight,
        _ctrl: &WebGpuBuffer<Val>,
        _global: &WebGpuBuffer<Val>,
        _data: &WebGpuBuffer<Val>,
        _mix: &WebGpuBuffer<Val>,
        _accum: &WebGpuBuffer<Val>,
        _steps: usize,
    ) {
        unimplemented!("browser WebGPU rv32im accumulation kernel is not wired yet")
    }
}

struct WebGpuSegmentProver {
    hal: Rc<WebGpuHal>,
    circuit_hal: Rc<WebGpuCircuitHal>,
}

impl SegmentProver for WebGpuSegmentProver {
    fn preflight(&self, segment: &crate::execute::segment::Segment) -> Result<PreflightResults> {
        scope!("preflight");

        cfg_if::cfg_if! {
            if #[cfg(feature = "witgen_debug")] {
                let rand_z = ExtVal::ONE;
            } else {
                let mut rng = rand::rng();
                let rand_z = ExtVal::random(&mut rng);
            }
        }
        PreflightResults::new(segment, rand_z)
    }

    fn prove_core(&self, preflight_results: PreflightResults) -> Result<crate::prove::Seal> {
        let hal = self.hal.clone();
        let circuit_hal = self.circuit_hal.clone();
        let delegate = SegmentProverImpl::new(move || (hal.clone(), circuit_hal.clone()));
        delegate.prove_core(preflight_results)
    }

    fn prove_core_async<'a>(
        &'a self,
        preflight_results: PreflightResults,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<crate::prove::Seal>> + 'a>> {
        Box::pin(async move {
            scope!("prove_core");

            cfg_if::cfg_if! {
                if #[cfg(feature = "witgen_debug")] {
                    let mode = if std::env::var_os("RISC0_WITGEN_DEBUG").is_some() {
                        StepMode::SeqForward
                    } else {
                        StepMode::Parallel
                    };
                } else {
                    let mode = StepMode::Parallel;
                }
            }

            let hal = self.hal.as_ref();
            let circuit_hal = self.circuit_hal.as_ref();

            let po2 = preflight_results.po2();
            let witgen = super::super::witgen::WitnessGenerator::new(
                hal,
                circuit_hal,
                preflight_results,
                mode,
            )?;

            let code = &witgen.code.buf;
            let data = &witgen.data.buf;
            let global = &witgen.global.buf;

            tracing::debug!("prove_inner");

            let mut prover = Prover::new(hal, TAPSET);
            let hashfn = &hal.get_hash_suite().hashfn;

            prover.iop().write_u32_slice(&[RV32IM_SEAL_VERSION]);
            prover
                .iop()
                .commit(&hashfn.hash_elem_slice(&PROOF_SYSTEM_INFO.encode()));
            prover
                .iop()
                .commit(&hashfn.hash_elem_slice(&CircuitImpl::CIRCUIT_INFO.encode()));

            let global_len = global.size();
            let mut header = vec![Val::ZERO; global_len + 1];
            global.view_mut(|view| {
                for (i, elem) in view.iter_mut().enumerate() {
                    *elem = elem.valid_or_zero();
                    header[i] = *elem;
                }
                header[global_len] = Val::new_raw(po2);
            });

            let header_digest = hashfn.hash_elem_slice(&header);
            prover.iop().commit(&header_digest);
            prover.iop().write_field_elem_slice(header.as_slice());
            prover.set_po2(po2 as usize);

            let async_scopes = crate::prove::webgpu_async_authoritative_scopes();
            {
                let _gpu_scope = hal.gpu_authoritative_scope(async_scopes.code_data);
                {
                    let _t = WebGpuStageTimer::new_active_for("commit_group_async rv32im_code", hal);
                    prover.commit_group_async(REGISTER_GROUP_CODE, code).await?;
                }
                {
                    let _t = WebGpuStageTimer::new_active_for("commit_group_async rv32im_data", hal);
                    prover.commit_group_async(REGISTER_GROUP_DATA, data).await?;
                }
            }

            let mix: [Val; REGCOUNT_MIX] = std::array::from_fn(|_| prover.iop().random_elem());
            let mix = {
                let _t = WebGpuStageTimer::new("rv32im_witgen_accum");
                witgen.accum(hal, circuit_hal, &mix)?
            };

            let async_scopes = crate::prove::webgpu_async_authoritative_scopes();
            {
                let _t = WebGpuStageTimer::new_active_for("commit_group_async rv32im_accum", hal);
                prover
                    .commit_group_async_scoped(
                        REGISTER_GROUP_ACCUM,
                        &witgen.accum.buf,
                        async_scopes.accum_make_coeffs,
                        async_scopes.accum_poly_group,
                        async_scopes.accum_merkle,
                    )
                    .await?;
            }
            {
                let _gpu_scope = hal.gpu_authoritative_scope(async_scopes.finalize);
                prover
                    .finalize_async(&[&mix.buf, global], circuit_hal)
                    .await
            }
        })
    }
}

pub fn segment_prover(hal: Rc<WebGpuHal>) -> Result<Box<dyn SegmentProver>> {
    let circuit_hal = Rc::new(WebGpuCircuitHal::new(hal.clone()));
    // SP7 iter 6d-d: kick off the witgen kernel Tint compile in the
    // background. No-op when WITGEN_GPU_PROBE_ENABLED is false.
    circuit_hal.prewarm_witgen_kernel();
    Ok(Box::new(WebGpuSegmentProver { hal, circuit_hal }))
}

fn step_mode_label(mode: StepMode) -> &'static str {
    match mode {
        StepMode::Parallel => "parallel",
        StepMode::SeqForward => "seq_forward",
        StepMode::SeqReverse => "seq_reverse",
    }
}
