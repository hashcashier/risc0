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

//! Browser WebGPU implementation of the ZKP HAL.

pub(crate) use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashSet},
    fmt::{Debug, Write as _},
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    rc::Rc,
};

pub(crate) use anyhow::{anyhow, bail, ensure, Result};
pub(crate) use risc0_core::field::{
    baby_bear::{BabyBear, BabyBearElem, BabyBearExtElem},
    Elem as _, ExtElem as _, RootsOfUnity,
};
pub(crate) use wasm_bindgen::closure::Closure;
pub(crate) use wasm_bindgen::{JsCast, JsValue};
pub(crate) use wasm_bindgen_futures::JsFuture;

pub(crate) use super::{
    cpu::{CpuBuffer, CpuHal},
    Buffer, Hal,
};
pub(crate) use crate::core::{
    digest::{Digest, DIGEST_WORDS},
    hash::{poseidon2, HashSuite},
    log2_ceil,
};
pub(crate) use crate::{
    adapter::{PolyExtStep, PolyExtStepDef},
    hal::webgpu_codegen::{
        eval_check_fp_slot, eval_check_last_uses, eval_check_last_uses_block, eval_check_mix_slot,
        eval_check_note_last, staged_multi_kernel_from_def, CodegenError, EmitterTap,
        EvalCheckSlotAllocator, FieldMode, STAGED_EVAL_CHECK_PRELUDE_WGSL,
    },
    taps::TapSet,
    INV_RATE,
};

// `BufferPool` + `TileLayout` for tiled multi-buffer source
// representations of recursion-sized data groups. Lives in a child
// module so the pure addressing math has its own unit tests without
// dragging in the whole HAL. Public so the browser-prove harness
// can construct a BufferPool from its regression test.
pub mod buffer_pool;

/// Target chunk size for multi-stage staged WGSL emission. Chosen so the
/// rv32im production DEF (~20k ops) emits ~4 stages, mirroring the CUDA
/// `eval_check_{0,1,2,3}.cu` layout the runtime interpreter parallels.
pub(crate) const STAGED_EVAL_CHECK_TARGET_CHUNK_OPS: usize = 7000;

/// Number of cycles processed per scratch-buffer tile. The
/// scratch buffer is sized to `tile_size * stride * 4 B` regardless of
/// the prove's full domain — the dispatch loop iterates
/// `ceil(domain / tile_size)` tiles, bumping `tile_base` between passes.
/// At 4096 with ~400 live ext fp vars + 5 live mix vars the per-cycle
/// stride is ~1620 u32, so fp_scratch = 4096 * 1620 * 4 ≈ 27 MiB; mix
/// scratches are < 1 MiB each. Comfortably within the device limits we
/// requested and independent of how many cycles the prove has overall.
pub(crate) const STAGED_EVAL_CHECK_TILE_SIZE: u32 = 4096;

/// Minimum `def.block.len()` to attempt the
/// staged WGSL fast path. Below this, the runtime interpreter is used
/// regardless of `staged_eval_check_enabled`. rv32im production DEFs
/// have ~20k+ ops; recursion's lift DEF and ad-hoc small DEFs sit
/// below and currently SIGKILL Chrome somewhere in the lift's
/// finalize_async flow when staged. Until that's root-caused, the gate
/// keeps staged where it wins (rv32im) and out of where it doesn't
/// (everything smaller).
pub(crate) const STAGED_EVAL_CHECK_MIN_BLOCK_OPS: usize = 8000;

/// `GPUBufferUsage.MAP_READ`.
pub const WEBGPU_BUFFER_USAGE_MAP_READ: u32 = 0x0001;
/// `GPUBufferUsage.MAP_WRITE`.
pub const WEBGPU_BUFFER_USAGE_MAP_WRITE: u32 = 0x0002;
/// `GPUBufferUsage.COPY_SRC`.
pub const WEBGPU_BUFFER_USAGE_COPY_SRC: u32 = 0x0004;
/// `GPUBufferUsage.COPY_DST`.
pub const WEBGPU_BUFFER_USAGE_COPY_DST: u32 = 0x0008;
/// `GPUBufferUsage.UNIFORM`.
pub const WEBGPU_BUFFER_USAGE_UNIFORM: u32 = 0x0040;
/// `GPUBufferUsage.STORAGE`.
pub const WEBGPU_BUFFER_USAGE_STORAGE: u32 = 0x0080;
/// `GPUShaderStage.COMPUTE`.
pub const WEBGPU_SHADER_STAGE_COMPUTE: u32 = 0x0004;
/// `GPUMapMode.READ`.
pub const WEBGPU_MAP_MODE_READ: u32 = 0x0001;

pub(crate) const MAX_EXACT_JS_INTEGER: u64 = 1 << 53;
pub(crate) const WEBGPU_WORKGROUP_SIZE: u32 = 256;
pub(crate) const WEBGPU_MAX_WORKGROUPS_PER_DIMENSION: u32 = 65_535;
pub(crate) const WEBGPU_REQUESTED_MAX_BUFFER_BYTES: u64 = 4 * 1024 * 1024 * 1024 - 4; // 4 GiB - alignment slack; many adapters cap at this
pub(crate) const WEBGPU_REQUESTED_MAX_STORAGE_BINDING_BYTES: u64 = 4 * 1024 * 1024 * 1024 - 4;
pub(crate) const WEBGPU_REQUESTED_MAX_WORKGROUP_STORAGE_BYTES: u32 = 128 * 1024;
/// Staged eval_check uses 7 read-only storage buffers
/// (group0..2, global0..1, mix_pows, plus mix_tot/mix_mul scratch) and
/// 3 read-write storage buffers (check, fp_scratch, mix_tot/mix_mul
/// scratch — overlap intentional, the binding type is `storage` not
/// `read_only_storage`). WebGPU's default `maxStorageBuffersPerShaderStage`
/// is 8; we request more so the multi-stage pipeline's bind group fits.
pub(crate) const WEBGPU_REQUESTED_MAX_STORAGE_BUFFERS_PER_STAGE: u32 = 16;

/// Bump `maxUniformBufferBindingSize` so the
/// staged eval_check's `mix_pows` can live in a uniform buffer instead
/// of a storage buffer. CUDA's reference uses `__constant__` memory
/// for `poly_mix` (broadcast-cached, low-latency); UBOs are the
/// WebGPU analog. For rv32im poseidon2_basic, `mix_pow_words = 25864`
/// (= 103 KiB), so we need at least 128 KiB. 256 KiB gives headroom
/// for larger DEFs.
pub(crate) const WEBGPU_REQUESTED_MAX_UNIFORM_BUFFER_BINDING_BYTES: u64 = 256 * 1024;

/// Capacity of the staged `mix_pows` uniform buffer in
/// `vec4<u32>` slots. The WGSL prelude declares
/// `array<vec4<u32>, STAGED_EVAL_CHECK_MIX_POWS_UBO_VEC4_CAPACITY>`; only the
/// first `mix_pow_words / 4` entries are populated per call. 16384
/// vec4 = 262144 B = 256 KiB matches the bumped UBO limit above.
pub(crate) const STAGED_EVAL_CHECK_MIX_POWS_UBO_VEC4_CAPACITY: usize = 16384;
pub(crate) const WEBGPU_SAFE_STORAGE_BINDING_BYTES: u64 = 1024 * 1024 * 1024;
pub(crate) const WEBGPU_SAFE_QUEUE_WRITE_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT: u64 = 256;
pub(crate) const WEBGPU_EVAL_CHECK_MAX_POLY_EXT_STEPS: usize = 4096;
pub(crate) const WEBGPU_EVAL_CHECK_ENABLE_SPLIT: bool = false;
pub(crate) const WEBGPU_EVAL_CHECK_SPLIT_TERMS_PER_SHADER: usize = 64;
pub(crate) const WEBGPU_EVAL_CHECK_SPLIT_FP_OPS_PER_SHADER: usize = 2048;
pub(crate) const WEBGPU_EVAL_CHECK_MAX_FP_SLOTS: usize = 1536;
pub(crate) const WEBGPU_EVAL_CHECK_MAX_FP_ELEM_SLOTS: usize = 8192;
/// Cap on the hybrid base interpreter's vec4 ext bank. rv32im's
/// production tape needs ~101 under the lazy reorder; DEFs that exceed
/// this fall through to the all-ext interpreter.
pub(crate) const WEBGPU_EVAL_CHECK_MAX_EXT_BANK_SLOTS: usize = 256;
pub(crate) const WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS: usize = 1536;
pub(crate) const WEBGPU_EVAL_CHECK_MAX_MIX_SLOTS: usize = 64;
/// Emit fp ops lazily (on first demand by a mix op,
/// post-order over the operand DAG) instead of in zirgen's eager tape
/// order. Same ops, same operands, a valid topological order — outputs
/// are bit-identical; only the peak number of simultaneously-live fp
/// values changes, which directly sizes the interpreter's per-thread
/// scratch array. Measured on the production tapes: rv32im max-live
/// 927 -> 684, recursion 1001 -> 297.
pub(crate) const WEBGPU_EVAL_CHECK_LAZY_REORDER: bool = true;
pub(crate) const WEBGPU_EVAL_CHECK_INTERPRETER_WORKGROUP_SIZE: u32 = 32;
pub(crate) const WEBGPU_EVAL_CHECK_INSTRUCTION_WORDS: usize = 8;
pub(crate) const WEBGPU_BATCH_EVALUATE_CHUNK_SIZE: usize = 1024;

pub(crate) const WEBGPU_EVAL_OP_CONST: u32 = 0;
pub(crate) const WEBGPU_EVAL_OP_CONST_EXT: u32 = 1;
pub(crate) const WEBGPU_EVAL_OP_GET: u32 = 2;
pub(crate) const WEBGPU_EVAL_OP_GET_GLOBAL: u32 = 3;
pub(crate) const WEBGPU_EVAL_OP_ADD: u32 = 4;
pub(crate) const WEBGPU_EVAL_OP_SUB: u32 = 5;
pub(crate) const WEBGPU_EVAL_OP_MUL: u32 = 6;
pub(crate) const WEBGPU_EVAL_OP_TRUE: u32 = 7;
pub(crate) const WEBGPU_EVAL_OP_AND_EQZ: u32 = 8;
pub(crate) const WEBGPU_EVAL_OP_AND_COND: u32 = 9;
// Hybrid base interpreter: ops 10+ operate on a second,
// small vec4 "ext bank" so tapes with a handful of `ConstExt` values
// (rv32im: 6 constants tainting ~100 simultaneously-live values) can run
// the cheap scalar base interpreter for everything untainted instead of
// paying vec4 arithmetic on every op. E/B suffixes name operand banks
// (Ext bank / Base bank); commutative EB/BE forms are normalized to EB
// by the encoder. All results land in the ext bank.
pub(crate) const WEBGPU_EVAL_OP_ADD_EE: u32 = 10;
pub(crate) const WEBGPU_EVAL_OP_ADD_EB: u32 = 11;
pub(crate) const WEBGPU_EVAL_OP_SUB_EE: u32 = 12;
pub(crate) const WEBGPU_EVAL_OP_SUB_EB: u32 = 13;
pub(crate) const WEBGPU_EVAL_OP_SUB_BE: u32 = 14;
pub(crate) const WEBGPU_EVAL_OP_MUL_EE: u32 = 15;
pub(crate) const WEBGPU_EVAL_OP_MUL_EB: u32 = 16;
pub(crate) const WEBGPU_EVAL_OP_AND_EQZ_EXT: u32 = 17;
pub(crate) const WEBGPU_EVAL_OP_AND_COND_EXT: u32 = 18;

/// Browser console timer for proof-stage telemetry.
///
/// This is intentionally scoped to the browser WebGPU HAL so circuit crates can
/// add wasm-only timing probes without depending directly on JS bindings.
pub struct WebGpuStageTimer {
    pub(crate) label: String,
    pub(crate) start_ms: f64,
    pub(crate) gpu_active: bool,
    // If set, increment THIS HAL's counter on drop instead
    // of the thread-local global. Each WebGpuHal owns its own counter via
    // `Rc<Cell<f64>>`. Multi-HAL pools need this so per-slot
    // gpu_active_ms doesn't over-count by other slots' activity.
    pub(crate) hal_active_counter: Option<Rc<Cell<f64>>>,
    pub(crate) stage_sink: Option<Rc<RefCell<Vec<WebGpuStageDiagnostics>>>>,
}

thread_local! {
    static WEBGPU_GPU_ACTIVE_MS: Cell<f64> = const { Cell::new(0.0) };
    static WEBGPU_POLY_GROUP_DRAIN_DIAGNOSTIC_ENABLED: Cell<bool> = const { Cell::new(false) };
    static WEBGPU_COMBOS_DIVIDE_PARALLEL_ENABLED: Cell<bool> = const { Cell::new(true) };
    static WEBGPU_COMBOS_DIVIDE_PARALLEL_DISPATCHES: Cell<u64> = const { Cell::new(0) };
    static WEBGPU_WASM_THREAD_POOL_WORKERS: Cell<usize> = const { Cell::new(0) };
    static WEBGPU_WASM_THREAD_POOL_STATE: Cell<WasmThreadPoolState> =
        const { Cell::new(WasmThreadPoolState::Uninit) };
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WasmThreadPoolState {
    Uninit,
    Ready,
    Failed,
}

/// Enable or disable explicit queue-drain diagnostics around WebGPU
/// PolyGroup/Merkle construction stages.
///
/// This is default-off because it inserts `queue.onSubmittedWorkDone()` waits
/// that intentionally perturb wall time. Use it only for attribution runs.
pub fn set_poly_group_drain_diagnostic_enabled(enabled: bool) {
    WEBGPU_POLY_GROUP_DRAIN_DIAGNOSTIC_ENABLED.with(|flag| flag.set(enabled));
}

pub(crate) fn poly_group_drain_diagnostic_enabled() -> bool {
    WEBGPU_POLY_GROUP_DRAIN_DIAGNOSTIC_ENABLED.with(|flag| flag.get())
}

/// Enable or disable the parallel-scan `combos_divide` implementation.
///
/// Default-on. The legacy path runs the whole synthetic division as one
/// sequential 1-thread loop per combo chunk (`workgroup_size(1)` over
/// `cycles` iterations); the parallel path decomposes the same recurrence
/// into a block-local suffix scan, a per-chunk carry scan, and an
/// element-wise fixup. Disabling this falls back to the legacy kernel.
pub fn set_combos_divide_parallel_enabled(enabled: bool) {
    WEBGPU_COMBOS_DIVIDE_PARALLEL_ENABLED.with(|flag| flag.set(enabled));
}

pub(crate) fn combos_divide_parallel_enabled() -> bool {
    WEBGPU_COMBOS_DIVIDE_PARALLEL_ENABLED.with(|flag| flag.get())
}

/// Number of `combos_divide` calls that took the parallel-scan path.
pub fn combos_divide_parallel_dispatches() -> u64 {
    WEBGPU_COMBOS_DIVIDE_PARALLEL_DISPATCHES.with(|count| count.get())
}

pub(crate) fn record_combos_divide_parallel_dispatch() {
    WEBGPU_COMBOS_DIVIDE_PARALLEL_DISPATCHES.with(|count| count.set(count.get() + 1));
}

/// Override the rayon web-worker pool size used by the CPU witness kernels
/// (0 = auto: `min(hardware_concurrency, 8)`). Takes effect only if set
/// before the first `WebGpuHal::new` on this thread.
pub fn set_wasm_thread_pool_workers(workers: usize) {
    WEBGPU_WASM_THREAD_POOL_WORKERS.with(|w| w.set(workers));
}

/// Initialize the rayon web-worker thread pool once per module instance.
///
/// Runs the `wasm-bindgen-rayon` handshake: spawns the workers, re-instantiates
/// the module in each against the shared memory, and installs them as the
/// rayon global pool. Requires the atomics build + SharedArrayBuffer (without
/// them the shared-memory module cannot instantiate at all, so reaching this
/// code implies SAB exists). On failure the pool stays uninstalled and any
/// later `par_iter` panics loudly — no silent serial fallback.
pub(crate) async fn ensure_wasm_thread_pool() {
    if WEBGPU_WASM_THREAD_POOL_STATE.with(|s| s.get()) != WasmThreadPoolState::Uninit {
        return;
    }
    let configured = WEBGPU_WASM_THREAD_POOL_WORKERS.with(|w| w.get());
    let workers = if configured == 0 {
        let hardware_concurrency = js_sys::global()
            .dyn_into::<web_sys::WorkerGlobalScope>()
            .ok()
            .map(|scope| scope.navigator().hardware_concurrency() as usize)
            .unwrap_or(0);
        if hardware_concurrency == 0 {
            8
        } else {
            // Probed on 32-thread hardware: 16 workers regressed
            // every gate ~20-25% (xgboost 17052→21159 ms) — oversubscribed
            // rayon idle-spinning on shared memory taxes the busy workers.
            // 8 is the measured optimum tier.
            hardware_concurrency.min(8)
        }
    } else {
        configured
    };
    let start_ms = js_sys::Date::now();
    match JsFuture::from(wasm_bindgen_rayon::init_thread_pool(workers)).await {
        Ok(_) => {
            WEBGPU_WASM_THREAD_POOL_STATE.with(|s| s.set(WasmThreadPoolState::Ready));
            log_webgpu_metric(&format!(
                "wasm_thread_pool workers={workers} init_ms={:.0}",
                js_sys::Date::now() - start_ms
            ));
        }
        Err(err) => {
            WEBGPU_WASM_THREAD_POOL_STATE.with(|s| s.set(WasmThreadPoolState::Failed));
            log_webgpu_metric(&format!("wasm_thread_pool init_failed err={err:?}"));
        }
    }
}

/// Optional circuit-specific WebGPU implementation of the check-polynomial
/// evaluation stage.
///
/// Returning `Ok(false)` means the circuit cannot handle the current buffers on
/// GPU and the prover should use the portable CPU-compatible fallback.
pub trait WebGpuCircuitEvalCheck {
    fn eval_check_webgpu(
        &self,
        _hal: &WebGpuHal,
        _check: &WebGpuBuffer<BabyBearElem>,
        _groups: &[&WebGpuBuffer<BabyBearElem>],
        _globals: &[&WebGpuBuffer<BabyBearElem>],
        _poly_mix: BabyBearExtElem,
        _po2: usize,
        _steps: usize,
    ) -> Result<bool> {
        Ok(false)
    }
}

impl WebGpuStageTimer {
    pub(crate) fn start(
        label: String,
        gpu_active: bool,
        hal_active_counter: Option<Rc<Cell<f64>>>,
        stage_sink: Option<Rc<RefCell<Vec<WebGpuStageDiagnostics>>>>,
    ) -> Self {
        let start_ms = js_sys::Date::now();
        log_webgpu_stage(&format!(
            "browser-prove:stage start t={start_ms:.1} {label}"
        ));
        Self {
            label,
            start_ms,
            gpu_active,
            hal_active_counter,
            stage_sink,
        }
    }

    pub fn new(label: impl Into<String>) -> Self {
        Self::start(label.into(), false, None, None)
    }

    pub fn new_active(label: impl Into<String>) -> Self {
        Self::start(label.into(), true, None, None)
    }

    /// HAL-scoped timer that records detailed stage diagnostics without adding
    /// to the HAL's aggregate `gpu_active_ms`.
    pub fn new_for(label: impl Into<String>, hal: &WebGpuHal) -> Self {
        Self::start(
            label.into(),
            false,
            None,
            Some(hal.stage_diagnostics_handle()),
        )
    }

    /// HAL-scoped active timer. The HAL's counter is
    /// incremented on drop instead of the thread-local global. Use this
    /// when running under a multi-HAL pool so each HAL's gpu_idle_ratio
    /// is accurate.
    pub fn new_active_for(label: impl Into<String>, hal: &WebGpuHal) -> Self {
        Self::start(
            label.into(),
            true,
            Some(hal.gpu_active_ms_handle()),
            Some(hal.stage_diagnostics_handle()),
        )
    }

    pub fn snapshot_gpu_active_ms() -> f64 {
        WEBGPU_GPU_ACTIVE_MS.with(|c| c.get())
    }

    pub fn reset_gpu_active_ms() {
        WEBGPU_GPU_ACTIVE_MS.with(|c| c.set(0.0));
    }
}

impl Drop for WebGpuStageTimer {
    fn drop(&mut self) {
        let end_ms = js_sys::Date::now();
        let elapsed_ms = end_ms - self.start_ms;
        if let Some(stage_sink) = &self.stage_sink {
            let elapsed_us = if elapsed_ms.is_finite() && elapsed_ms > 0.0 {
                (elapsed_ms * 1000.0).round() as u64
            } else {
                0
            };
            stage_sink.borrow_mut().push(WebGpuStageDiagnostics {
                label: self.label.clone(),
                elapsed_us,
                gpu_active: self.gpu_active,
            });
        }
        if self.gpu_active {
            if let Some(counter) = &self.hal_active_counter {
                counter.set(counter.get() + elapsed_ms);
            } else {
                WEBGPU_GPU_ACTIVE_MS.with(|c| c.set(c.get() + elapsed_ms));
            }
            log_webgpu_stage(&format!(
                "browser-prove:stage done t={end_ms:.1} {} elapsed_ms={elapsed_ms:.3} gpu_active=true",
                self.label
            ));
        } else {
            log_webgpu_stage(&format!(
                "browser-prove:stage done t={end_ms:.1} {} elapsed_ms={elapsed_ms:.3}",
                self.label
            ));
        }
    }
}

pub(crate) fn log_webgpu_stage(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(message));
}

pub fn log_webgpu_metric(message: &str) {
    log_webgpu_stage(&format!("browser-prove:metric {message}"));
}

/// Lazy-shadow attribution: log CPU shadow materializations of at least
/// 1 MiB so the stage log shows which buffers still pin wasm heap.
pub(crate) fn log_cpu_shadow_materialize(name: &'static str, bytes: usize) {
    if bytes >= (1 << 20) {
        log_webgpu_metric(&format!("cpu_shadow_materialize name={name} bytes={bytes}"));
    }
}

// The HAL is split by layer; the `pub use` globs keep every public item at
// its original `hal::webgpu::*` path. Promoted `pub(crate)` items stay
// crate-internal.
mod device;
mod diagnostics;
mod dispatch;
mod eval_check;
mod kernels_wgsl;
mod ops;
mod resources;

pub use device::*;
pub use diagnostics::*;
pub use resources::*;
