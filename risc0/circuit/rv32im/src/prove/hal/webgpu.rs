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
    /// Tracks whether the async prewarm task has been spawned this
    /// session, so multiple `prewarm_witgen_kernel()` calls (one per
    /// segment_prover) only fire the compile once.
    static WITGEN_PREWARM_SPAWNED: Cell<bool> = const { Cell::new(false) };
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
