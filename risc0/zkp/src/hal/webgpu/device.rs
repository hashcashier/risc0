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

//! The `WebGpuHal` device handle: construction, kernel/pipeline
//! caches, and the `eval_check` dispatch paths.

use super::*;
#[allow(unused_imports)]
use super::{diagnostics::*, dispatch::*, eval_check::*, kernels_wgsl::*, ops::*, resources::*};

/// A browser WebGPU HAL.
///
/// The type owns a real `GPUDevice` and is the integration point for WGSL
/// kernels. Buffers own browser `GPUBuffer` storage plus a CPU shadow so the
/// current synchronous prover interfaces can remain correct while operations
/// are moved to WebGPU incrementally.
pub struct WebGpuHal {
    pub device: web_sys::GpuDevice,
    pub queue: web_sys::GpuQueue,
    /// Unique per-HAL id so caches keyed by device identity (e.g. the
    /// program-constant code-group cache) never cross `GPUDevice` boundaries.
    pub(crate) instance_id: u64,
    pub(crate) cpu: CpuHal<BabyBear>,
    pub(crate) poseidon2: Option<WebGpuPoseidon2Hash>,
    pub(crate) diagnostics: WebGpuDiagnosticsState,
    // Per-HAL accumulator for GPU-active stage elapsed_ms.
    // Wrapped in Rc<Cell<_>> so WebGpuStageTimer instances can hold a
    // cheap clone without back-references. Replaces the thread-local
    // WEBGPU_GPU_ACTIVE_MS when running under a multi-HAL pool.
    pub(crate) gpu_active_ms: Rc<Cell<f64>>,
    pub(crate) gpu_authoritative: Cell<bool>,
    pub(crate) eval_check_gpu_enabled: Cell<bool>,
    pub(crate) batch_expand_into_evaluate_ntt_gpu_enabled: Cell<bool>,
    pub(crate) batch_interpolate_ntt_gpu_enabled: Cell<bool>,
    pub(crate) batch_bit_reverse_gpu_enabled: Cell<bool>,
    pub(crate) hash_fold_gpu_enabled: Cell<bool>,
    pub(crate) hash_rows_gpu_enabled: Cell<bool>,
    pub(crate) zk_shift_gpu_enabled: Cell<bool>,
    pub(crate) max_buffer_size: u64,
    pub(crate) max_storage_buffer_binding_size: u64,
    pub(crate) max_compute_workgroup_storage_size: u32,
    pub(crate) min_uniform_buffer_offset_alignment: u32,
    pub(crate) eval_check_interpreter_pipelines:
        RefCell<BTreeMap<EvalCheckInterpreterPipelineKey, EvalCheckInterpreterPipeline>>,
    // Runtime flag that opts a HAL into the
    // staged-WGSL eval_check path before the runtime interpreter. Default
    // false — the interpreter is the production path (it measured far
    // faster than staged emission for rv32im); browser parity tests opt
    // in per-fixture to keep the staged path validated.
    pub(crate) staged_eval_check_enabled: Cell<bool>,
    // Per-DEF cache of compiled staged eval_check
    // pipelines. Keyed by `def as *const PolyExtStepDef as usize` because
    // DEFs are `&'static` and program identity is the natural cache key.
    // Each cached entry holds the bind-group layout + the compiled compute
    // kernel so subsequent dispatches against the same DEF skip the
    // (potentially slow) WGSL compile step.
    pub(crate) staged_eval_check_pipelines: RefCell<BTreeMap<usize, StagedEvalCheckPipeline>>,
    // Cache per-po2 NTT twiddle tables so hot NTT shaders fetch `root^s`
    // instead of recomputing it independently in every butterfly lane.
    pub(crate) ntt_twiddle_cache: RefCell<BTreeMap<(bool, usize), WebGpuBuffer<BabyBearElem>>>,
    // Cache `GpuBindGroupLayout` instances by
    // (label.as_ptr(), entries-shape-hash). 31 `create_bind_group_layout`
    // sites in this file. Layouts are immutable shape descriptors;
    // identical shapes can safely share one instance. This is the
    // foundation for a later pipeline cache (pipelines must reference
    // the SAME layout INSTANCE as the bind groups dispatched against
    // them -- see 61d3163c9 failed-experiment ledger).
    pub(crate) bind_group_layout_cache: RefCell<BTreeMap<u64, web_sys::GpuBindGroupLayout>>,
    // Map HAL-created bind-group-layout JS objects
    // back to their structural cache keys. Compute pipeline caching is only
    // enabled when every supplied layout comes from this HAL, so cache hits
    // cannot accidentally reuse a pipeline with an unrelated layout instance.
    pub(crate) bind_group_layout_key_map: js_sys::WeakMap,
    // Cache compute pipelines after layout identity is
    // stable. Pipelines depend only on WGSL, entry point, and bind-group
    // layouts; unlike bind groups they do not retain per-proof buffers.
    pub(crate) compute_kernel_cache: RefCell<BTreeMap<u64, WebGpuKernel>>,
    // Eval-check interpreter programs are immutable for a given circuit DEF.
    // Cache by exact instruction words, separated by base-field mode, so
    // repeated segment/lift dispatches skip re-uploading the same program.
    pub(crate) eval_check_instruction_cache:
        RefCell<BTreeMap<(bool, Vec<u32>), web_sys::GpuBuffer>>,
    pub(crate) elem_transpose_zero_pad_cache:
        RefCell<BTreeMap<ElemTransposeZeroPadCacheKey, WebGpuBuffer<BabyBearElem>>>,
}

/// Restores the previous GPU-authoritative mode when dropped.
pub struct WebGpuAuthoritativeScope<'a> {
    pub(crate) hal: &'a WebGpuHal,
    pub(crate) previous: bool,
}

impl Drop for WebGpuAuthoritativeScope<'_> {
    fn drop(&mut self) {
        self.hal.set_gpu_authoritative(self.previous);
    }
}

/// Virtualizes the HAL's GPU-authoritative flag for one proof future so
/// two proofs can interleave on a single-threaded executor.
///
/// [`WebGpuAuthoritativeScope`] restores the flag with stack discipline,
/// which breaks when two futures' scopes overlap across `await` points
/// (A opens, B opens capturing A's value, A closes, and B now runs under
/// the wrong mode — silently flipping GPU dispatches to CPU fallbacks).
/// This wrapper gives the inner future a private copy of the flag: each
/// `poll` installs the future's copy, and on exit captures whatever the
/// future's own scopes changed it to before restoring the ambient value.
/// Scope guards created and dropped inside the future therefore observe
/// exactly the values they would see when run serially.
pub fn with_authoritative_context<F: Future>(
    hal: Rc<WebGpuHal>,
    inner: F,
) -> impl Future<Output = F::Output> {
    struct Ctx<F> {
        hal: Rc<WebGpuHal>,
        value: bool,
        inner: Option<Pin<Box<F>>>,
    }

    impl<F: Future> Future for Ctx<F> {
        type Output = F::Output;

        fn poll(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<F::Output> {
            let this = self.get_mut();
            let ambient = this.hal.gpu_authoritative();
            this.hal.set_gpu_authoritative(this.value);
            let result = this
                .inner
                .as_mut()
                .expect("authoritative-context future polled after drop")
                .as_mut()
                .poll(cx);
            this.value = this.hal.gpu_authoritative();
            this.hal.set_gpu_authoritative(ambient);
            result
        }
    }

    impl<F> Drop for Ctx<F> {
        fn drop(&mut self) {
            // Cancellation drops the inner future's scope guards; run them
            // under this future's flag copy so they cannot poison the
            // ambient value.
            if let Some(inner) = self.inner.take() {
                let ambient = self.hal.gpu_authoritative();
                self.hal.set_gpu_authoritative(self.value);
                drop(inner);
                self.hal.set_gpu_authoritative(ambient);
            }
        }
    }

    let value = hal.gpu_authoritative();
    Ctx {
        hal,
        value,
        inner: Some(Box::pin(inner)),
    }
}

impl WebGpuHal {
    /// Request a browser WebGPU device and construct a HAL with the given hash suite.
    pub async fn new(hash_suite: HashSuite<BabyBear>) -> Result<Self> {
        ensure_wasm_thread_pool().await;
        let device = request_device().await?;
        Ok(Self::from_device(device, hash_suite))
    }

    /// Construct a HAL from a browser `GPUDevice` supplied by the crate consumer.
    pub fn from_device(device: web_sys::GpuDevice, hash_suite: HashSuite<BabyBear>) -> Self {
        // Attribute which lazy CPU shadows still materialize (>= 1 MiB)
        // so heap-pinning buffers stay visible in the stage log.
        crate::hal::cpu::set_buffer_materialize_observer(log_cpu_shadow_materialize);
        let use_poseidon2 = hash_suite.name == "poseidon2";
        let limits = device.limits();
        let queue = device.queue();
        let max_buffer_size = limits.max_buffer_size() as u64;
        let max_storage_buffer_binding_size = limits.max_storage_buffer_binding_size() as u64;
        let max_compute_workgroup_storage_size = limits.max_compute_workgroup_storage_size();
        let min_uniform_buffer_offset_alignment = limits.min_uniform_buffer_offset_alignment();
        log_webgpu_stage(&format!(
            "browser-prove:webgpu-limits max_buffer_size={} max_storage_buffer_binding_size={} max_compute_workgroup_storage_size={}",
            max_buffer_size, max_storage_buffer_binding_size, max_compute_workgroup_storage_size
        ));

        // Surface Chrome WebGPU `uncapturederror` events
        // so silent validation/OOM failures during cumulative-pressure paths
        // (recursion lift/join at multi-segment scale) become visible. Without
        // this listener Chrome swallows async device errors and dispatches that
        // failed validation report success — see xgboost zero-roots regression.
        // The closure is leaked via `.forget()`; it lives for the device's
        // lifetime, which equals the HAL's lifetime.
        let uncaptured_error_listener =
            Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
                let error = js_sys::Reflect::get(&event, &JsValue::from_str("error"))
                    .unwrap_or(JsValue::NULL);
                let msg = js_sys::Reflect::get(&error, &JsValue::from_str("message"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_else(|| "(no message)".to_string());
                let kind = error
                    .dyn_ref::<js_sys::Object>()
                    .map(|obj| obj.constructor().name().as_string().unwrap_or_default())
                    .unwrap_or_else(|| "GPUError".to_string());
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "browser-prove:webgpu-uncaptured-error type={kind} msg={msg}"
                )));
            });
        device.set_onuncapturederror(Some(uncaptured_error_listener.as_ref().unchecked_ref()));
        uncaptured_error_listener.forget();

        let mut hal = Self {
            device,
            queue,
            instance_id: {
                thread_local! {
                    static NEXT_HAL_INSTANCE_ID: Cell<u64> = const { Cell::new(0) };
                }
                NEXT_HAL_INSTANCE_ID.with(|next| {
                    let id = next.get();
                    next.set(id + 1);
                    id
                })
            },
            cpu: CpuHal::new(hash_suite),
            poseidon2: None,
            diagnostics: WebGpuDiagnosticsState::default(),
            gpu_active_ms: Rc::new(Cell::new(0.0)),
            gpu_authoritative: Cell::new(false),
            eval_check_gpu_enabled: Cell::new(true),
            batch_expand_into_evaluate_ntt_gpu_enabled: Cell::new(true),
            batch_interpolate_ntt_gpu_enabled: Cell::new(true),
            batch_bit_reverse_gpu_enabled: Cell::new(true),
            hash_fold_gpu_enabled: Cell::new(true),
            hash_rows_gpu_enabled: Cell::new(true),
            zk_shift_gpu_enabled: Cell::new(true),
            max_buffer_size,
            max_storage_buffer_binding_size,
            max_compute_workgroup_storage_size,
            min_uniform_buffer_offset_alignment,
            eval_check_interpreter_pipelines: RefCell::new(BTreeMap::new()),
            staged_eval_check_enabled: Cell::new(false),
            staged_eval_check_pipelines: RefCell::new(BTreeMap::new()),
            ntt_twiddle_cache: RefCell::new(BTreeMap::new()),
            bind_group_layout_cache: RefCell::new(BTreeMap::new()),
            bind_group_layout_key_map: js_sys::WeakMap::new(),
            compute_kernel_cache: RefCell::new(BTreeMap::new()),
            eval_check_instruction_cache: RefCell::new(BTreeMap::new()),
            elem_transpose_zero_pad_cache: RefCell::new(BTreeMap::new()),
        };
        if use_poseidon2 {
            hal.poseidon2 = Some(
                WebGpuPoseidon2Hash::new(&hal)
                    .unwrap_or_else(|err| panic!("failed to initialize WebGPU Poseidon2: {err}")),
            );
        }
        hal
    }

    /// Return a snapshot of backend usage since construction or the last reset.
    pub fn diagnostics(&self) -> WebGpuDiagnostics {
        self.diagnostics.snapshot()
    }

    /// Reset backend usage diagnostics.
    pub fn reset_diagnostics(&self) {
        self.diagnostics.reset();
    }

    pub(crate) fn ntt_twiddles(
        &self,
        inverse: bool,
        n_bits: usize,
    ) -> Result<WebGpuBuffer<BabyBearElem>> {
        ensure!(
            n_bits < BabyBearElem::MAX_ROU_PO2,
            "WebGPU NTT twiddle po2 exceeds BabyBear roots"
        );
        let key = (inverse, n_bits);
        if let Some(buffer) = self.ntt_twiddle_cache.borrow().get(&key).cloned() {
            return Ok(buffer);
        }

        let mut twiddles = Vec::with_capacity((1usize << n_bits).saturating_sub(1));
        let roots = if inverse {
            BabyBearElem::ROU_REV
        } else {
            BabyBearElem::ROU_FWD
        };
        for s_bits in 1..=n_bits {
            let step = roots[s_bits];
            let s_size = 1usize << (s_bits - 1);
            let mut cur = BabyBearElem::ONE;
            for _ in 0..s_size {
                twiddles.push(cur);
                cur *= step;
            }
        }

        let name = if inverse {
            "webgpu_ntt_twiddles_rev"
        } else {
            "webgpu_ntt_twiddles_fwd"
        };
        let buffer = self.copy_from_elem(name, &twiddles);
        self.ntt_twiddle_cache
            .borrow_mut()
            .insert(key, buffer.clone());
        Ok(buffer)
    }

    /// Return the negotiated WebGPU device limits that materially affect the
    /// browser prover performance profile.
    #[doc(hidden)]
    pub fn performance_limits(&self) -> (u64, u64, u32) {
        (
            self.max_buffer_size,
            self.max_storage_buffer_binding_size,
            self.max_compute_workgroup_storage_size,
        )
    }

    /// Enable or disable GPU-authoritative HAL outputs.
    ///
    /// When enabled, WebGPU kernels that successfully dispatch may skip their
    /// CPU mirror work and mark output buffers as requiring async readback
    /// before any synchronous CPU view.
    pub fn set_gpu_authoritative(&self, enabled: bool) {
        self.gpu_authoritative.set(enabled);
    }

    /// Returns whether successful WebGPU kernels are allowed to own outputs
    /// without an immediate CPU mirror.
    pub fn gpu_authoritative(&self) -> bool {
        self.gpu_authoritative.get()
    }

    /// Unique id for this HAL instance. Buffers are device-scoped, so
    /// caches of GPU-resident artifacts must be keyed by this id.
    pub fn instance_id(&self) -> u64 {
        self.instance_id
    }

    /// Current gpu_active_ms for this HAL. Sum of all
    /// `WebGpuStageTimer::new_active_for(_, self)` scopes' elapsed_ms
    /// since the last reset.
    pub fn gpu_active_ms(&self) -> f64 {
        self.gpu_active_ms.get()
    }

    /// Reset this HAL's gpu_active_ms accumulator to 0.
    pub fn reset_gpu_active_ms(&self) {
        self.gpu_active_ms.set(0.0);
    }

    /// Clone the counter handle so a `WebGpuStageTimer`
    /// can increment it on drop without holding a back-reference to
    /// the HAL. Cheap (Rc::clone).
    pub fn gpu_active_ms_handle(&self) -> Rc<Cell<f64>> {
        self.gpu_active_ms.clone()
    }

    pub(crate) fn stage_diagnostics_handle(&self) -> Rc<RefCell<Vec<WebGpuStageDiagnostics>>> {
        self.diagnostics.stage_diagnostics_handle()
    }

    /// Enable or disable WebGPU eval_check dispatch.
    ///
    /// This is a diagnostic switch used by browser parity tests to isolate
    /// eval_check-specific correctness failures from the rest of the async
    /// GPU-authoritative proof path.
    pub fn set_eval_check_gpu_enabled(&self, enabled: bool) {
        self.eval_check_gpu_enabled.set(enabled);
    }

    /// Enable or disable the staged-WGSL `eval_check` fast path. When `true`,
    /// `dispatch_eval_check_poly_ext` tries the staged kernel first and
    /// falls through to the runtime interpreter on any failure (compile,
    /// dispatch, or `CodegenError`). Default `false` — browser parity tests
    /// enable this for individual fixtures once parity is established.
    pub fn set_staged_eval_check_enabled(&self, enabled: bool) {
        self.staged_eval_check_enabled.set(enabled);
    }

    /// Returns whether the staged-WGSL `eval_check` fast path is enabled.
    pub fn staged_eval_check_enabled(&self) -> bool {
        self.staged_eval_check_enabled.get()
    }

    /// Enable or disable specific WebGPU kernels for diagnostics.
    #[doc(hidden)]
    pub fn set_op_gpu_enabled(&self, op: &str, enabled: bool) {
        match op {
            "batch_expand_into_evaluate_ntt" => {
                self.batch_expand_into_evaluate_ntt_gpu_enabled.set(enabled)
            }
            "batch_interpolate_ntt" => self.batch_interpolate_ntt_gpu_enabled.set(enabled),
            "batch_bit_reverse" => self.batch_bit_reverse_gpu_enabled.set(enabled),
            "hash_fold" => self.hash_fold_gpu_enabled.set(enabled),
            "hash_rows" => self.hash_rows_gpu_enabled.set(enabled),
            "zk_shift" => self.zk_shift_gpu_enabled.set(enabled),
            _ => panic!("unknown WebGPU diagnostic op: {op}"),
        }
    }

    /// Temporarily set GPU-authoritative mode for a scoped proof stage.
    pub fn gpu_authoritative_scope(&self, enabled: bool) -> WebGpuAuthoritativeScope<'_> {
        let previous = self.gpu_authoritative();
        self.set_gpu_authoritative(enabled);
        WebGpuAuthoritativeScope {
            hal: self,
            previous,
        }
    }

    pub(crate) fn max_storage_binding_bytes(&self) -> u64 {
        self.max_storage_buffer_binding_size
            .min(WEBGPU_SAFE_STORAGE_BINDING_BYTES)
    }

    pub(crate) fn eval_check_base_workgroup_lanes(
        &self,
        fp_slots: usize,
        ext_slots: usize,
        mix_slots: usize,
    ) -> u32 {
        let bytes_per_lane = fp_slots
            .checked_mul(mem::size_of::<u32>())
            .and_then(|bytes| {
                let ext_bytes = ext_slots.max(1).checked_mul(mem::size_of::<[u32; 4]>())?;
                let mix_bytes = mix_slots
                    .checked_mul(mem::size_of::<[u32; 4]>())
                    .and_then(|bytes| bytes.checked_mul(2))?;
                bytes.checked_add(ext_bytes)?.checked_add(mix_bytes)
            })
            .expect("WebGPU eval_check base scratch size overflow");
        let lanes = (self.max_compute_workgroup_storage_size as usize / bytes_per_lane).max(1);
        lanes.min(WEBGPU_EVAL_CHECK_INTERPRETER_WORKGROUP_SIZE as usize) as u32
    }

    pub(crate) fn eval_check_interpreter_pipeline(
        &self,
        base_field_fp: bool,
        private_parallel: bool,
        fp_slots: usize,
        ext_slots: usize,
        mix_slots: usize,
        workgroup_size: u32,
    ) -> Result<EvalCheckInterpreterPipeline> {
        let key = EvalCheckInterpreterPipelineKey {
            base_field_fp,
            private_parallel,
            fp_slots,
            ext_slots,
            mix_slots,
            workgroup_size,
        };
        if let Some(pipeline) = self.eval_check_interpreter_pipelines.borrow().get(&key) {
            return Ok(pipeline.clone());
        }

        let layout = self.create_bind_group_layout(
            "webgpu_eval_check_interpreter_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::read_only_storage(6, 0),
                WebGpuBindingLayout::read_only_storage(7, 0),
                WebGpuBindingLayout::uniform(8, 96),
            ],
        )?;
        let (kernel_name, wgsl) = if private_parallel {
            (
                "webgpu_eval_check_base_private_interpreter",
                build_eval_check_base_interpreter_wgsl(
                    fp_slots,
                    ext_slots,
                    mix_slots,
                    true,
                    workgroup_size,
                ),
            )
        } else if base_field_fp {
            (
                "webgpu_eval_check_base_interpreter",
                build_eval_check_base_interpreter_wgsl(
                    fp_slots,
                    ext_slots,
                    mix_slots,
                    false,
                    workgroup_size,
                ),
            )
        } else {
            (
                "webgpu_eval_check_interpreter",
                build_eval_check_interpreter_wgsl(fp_slots, mix_slots),
            )
        };
        let kernel = self.create_compute_kernel(kernel_name, &wgsl, "main", &[layout.clone()])?;
        let pipeline = EvalCheckInterpreterPipeline { layout, kernel };
        self.eval_check_interpreter_pipelines
            .borrow_mut()
            .insert(key, pipeline.clone());
        Ok(pipeline)
    }

    /// Cache lookup (or compile + insert) of a staged-WGSL
    /// `eval_check` pipeline for a specific DEF + field-mode pair. Cache
    /// is keyed by `def as *const _ as usize` because `PolyExtStepDef`s
    /// are `&'static` and program identity is the natural key. Compilation
    /// may be slow on large DEFs — the rv32im production DEF emits a
    /// ~1.6 MB shader whose Chrome compile time is empirical — so caching
    /// per-DEF avoids paying that cost on every dispatch.
    pub(crate) fn staged_eval_check_pipeline(
        &self,
        def: &PolyExtStepDef,
        taps: &TapSet<'_>,
        base_field_fp: bool,
    ) -> Result<StagedEvalCheckPipeline> {
        let key = def as *const PolyExtStepDef as usize;
        if let Some(pipeline) = self.staged_eval_check_pipelines.borrow().get(&key) {
            if pipeline.base_field_fp == base_field_fp {
                return Ok(pipeline.clone());
            }
        }
        let field_mode = if base_field_fp {
            FieldMode::Base
        } else {
            FieldMode::Ext
        };
        let emitter_taps: Vec<EmitterTap> = taps
            .taps()
            .map(|t| EmitterTap {
                group: t.group() as u32,
                offset: t.offset() as u32,
                back_inv_rate: (t.back() * INV_RATE) as u32,
            })
            .collect();
        let multi = staged_multi_kernel_from_def(
            "staged_eval_check",
            def,
            &emitter_taps,
            field_mode,
            STAGED_EVAL_CHECK_TARGET_CHUNK_OPS,
        )
        .map_err(|err: CodegenError| anyhow!("staged eval_check codegen failed: {:?}", err))?;
        // Log per-stage fp_slots/mix_slots/workgroup_size
        // so we can see the per-chunk allocator's effect. Each stage
        // resets the slot allocator, so the high-water
        // varies per chunk and may be much smaller than the cross-chunk
        // global max.
        let per_stage_summary = multi
            .stages
            .iter()
            .enumerate()
            .map(|(i, s)| {
                format!(
                    "s{i}:fp={},mix={},wg={}",
                    s.fp_slots, s.mix_slots, s.workgroup_size
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        log_webgpu_stage(&format!(
            "browser-prove:staged-plan field_mode={field_mode:?} stages={} per_stage=[{per_stage_summary}] fp_scratch_stride_u32={} mix_scratch_stride_u32={} def_block_len={}",
            multi.stages.len(),
            multi.fp_scratch_stride_u32,
            multi.mix_scratch_stride_u32,
            def.block.len(),
        ));
        let layout = self.create_bind_group_layout(
            "webgpu_staged_eval_check_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::read_only_storage(5, 0),
                // binding 6 (instrs) intentionally omitted; staged kernel
                // inlines the DEF rather than reading an instruction stream.
                // Binding 7 (mix_pows) is now a uniform
                // buffer (CUDA `__constant__` analog). The full 256 KiB
                // is bound regardless of the DEF's actual mix_pow_words —
                // the WGSL declares a fixed-size array and only reads
                // the prefix the DEF needs.
                WebGpuBindingLayout::uniform(
                    7,
                    (STAGED_EVAL_CHECK_MIX_POWS_UBO_VEC4_CAPACITY * 16) as u64,
                ),
                WebGpuBindingLayout::uniform(8, 96),
                // New scratch bindings. Always present in the
                // layout — single-stage emissions bind a dummy 4-byte fp
                // / mix scratch buffer to satisfy the validator.
                // Binding 9 is a 16-byte UBO with
                // pipeline-constant {fp_stride, mix_stride, num_stages,
                // tile_size}, bound once and read by every stage.
                WebGpuBindingLayout::uniform(9, 16),
                WebGpuBindingLayout::storage(10, 0),
                WebGpuBindingLayout::storage(11, 0),
                WebGpuBindingLayout::storage(12, 0),
            ],
        )?;
        let mut stages = Vec::with_capacity(multi.stages.len());
        for (idx, stage) in multi.stages.iter().enumerate() {
            // Concatenate prelude + per-stage body for compilation.
            let mut wgsl = String::with_capacity(
                STAGED_EVAL_CHECK_PRELUDE_WGSL.len() + stage.wgsl_source.len() + 64,
            );
            wgsl.push_str(STAGED_EVAL_CHECK_PRELUDE_WGSL);
            wgsl.push('\n');
            wgsl.push_str(&stage.wgsl_source);
            let kernel_name = if multi.stages.len() == 1 {
                "webgpu_staged_eval_check"
            } else {
                "webgpu_staged_eval_check_stage"
            };
            // Time each stage's WGSL compile
            // separately so we can attribute the staged-path slowness.
            // Iter 7p (workgroup_size=32) didn't move total prove time,
            // so the 40 s overhead is either WGSL compile (one-shot per
            // pipeline) or kernel execution (per-call).
            let _stage_compile_timer = WebGpuStageTimer::new(format!(
                "staged_eval_check_compile stage={idx}/{} wgsl_bytes={}",
                multi.stages.len(),
                wgsl.len()
            ));
            let kernel =
                self.create_compute_kernel(kernel_name, &wgsl, "main", &[layout.clone()])?;
            stages.push(kernel);
        }
        // Allocate scratch buffers once at pipeline create
        // and cache them; they're reused across every eval_check call
        // that hits this pipeline. Size is `STAGED_EVAL_CHECK_TILE_SIZE *
        // stride * 4 B`, independent of any single call's domain.
        let fp_scratch_byte_len = if multi.fp_scratch_stride_u32 == 0 {
            4
        } else {
            byte_len_for::<u32>(multi.fp_scratch_stride_u32 * STAGED_EVAL_CHECK_TILE_SIZE as usize)
        };
        let mix_scratch_byte_len = if multi.mix_scratch_stride_u32 == 0 {
            4
        } else {
            byte_len_for::<u32>(multi.mix_scratch_stride_u32 * STAGED_EVAL_CHECK_TILE_SIZE as usize)
        };
        let fp_scratch =
            self.create_storage_buffer("webgpu_staged_eval_check_fp_scratch", fp_scratch_byte_len)?;
        let mix_tot_scratch = self.create_storage_buffer(
            "webgpu_staged_eval_check_mix_tot_scratch",
            mix_scratch_byte_len,
        )?;
        let mix_mul_scratch = self.create_storage_buffer(
            "webgpu_staged_eval_check_mix_mul_scratch",
            mix_scratch_byte_len,
        )?;
        // scratch_params is a single 16-byte UBO with
        // {fp_stride, mix_stride, num_stages, tile_size}. All fields
        // are pipeline-constant — written ONCE at pipeline create, then
        // every eval_check call reuses it. Threads compute their cycle
        // via `gid.y * tile_size + gid.x`, so no per-tile UBO offset is
        // needed. Replaces the earlier `MAX_TILES * 256 B` dynamic-
        // offset array with a single 16-byte entry.
        let scratch_params_buf = self.create_buffer(
            "webgpu_staged_eval_check_scratch_params",
            16,
            WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;
        let num_stages_u32 =
            u32::try_from(multi.stages.len()).expect("staged num_stages exceeds u32");
        let fp_stride_u32 = u32::try_from(multi.fp_scratch_stride_u32)
            .expect("staged fp_scratch_stride exceeds u32");
        let mix_stride_u32 = u32::try_from(multi.mix_scratch_stride_u32)
            .expect("staged mix_scratch_stride exceeds u32");
        let scratch_params_words: [u32; 4] = [
            fp_stride_u32,
            mix_stride_u32,
            num_stages_u32,
            STAGED_EVAL_CHECK_TILE_SIZE,
        ];
        self.write_buffer_named(
            &scratch_params_buf,
            "webgpu_staged_eval_check_scratch_params",
            0,
            bytemuck::cast_slice(&scratch_params_words),
        )?;
        // Cache mix_pows and params buffers on the
        // pipeline. Sizes depend only on the DEF (mix_pow_words for
        // mix_pows, fixed 96 bytes for params) so they're stable
        // across all eval_check calls hitting this pipeline.
        let mix_expected = def.ret + 1;
        let mix_pow_words = mix_expected * BabyBearExtElem::EXT_SIZE;
        // mix_pows is a uniform buffer (CUDA `__constant__`
        // analog). Sized to the full UBO capacity so the WGSL's
        // fixed-size array<vec4<u32>, STAGED_EVAL_CHECK_MIX_POWS_UBO_VEC4_CAPACITY>
        // declaration matches the buffer size exactly. Each `eval_check`
        // call writes only `mix_pow_words` u32s starting at offset 0;
        // the remaining slots are unused.
        let mix_pows_capacity_bytes = (STAGED_EVAL_CHECK_MIX_POWS_UBO_VEC4_CAPACITY * 16) as u64;
        if (mix_pow_words * 4) as u64 > mix_pows_capacity_bytes {
            return Err(anyhow!(
                "staged eval_check: mix_pow_words ({}) exceeds UBO capacity ({} u32)",
                mix_pow_words,
                mix_pows_capacity_bytes / 4
            ));
        }
        let mix_pows_buf = self.create_buffer(
            "webgpu_staged_eval_check_mix_pows",
            mix_pows_capacity_bytes,
            WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;
        let params_buf = self.create_buffer(
            "webgpu_staged_eval_check_params",
            96,
            WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;
        // workgroup_size is shared across stages (codegen
        // picks the same value from `plan.fp_slots`/`plan.mix_slots`).
        // If multi.stages is empty (defensive — should never happen),
        // fall back to 1.
        let workgroup_size = multi.stages.first().map(|s| s.workgroup_size).unwrap_or(1);
        let pipeline = StagedEvalCheckPipeline {
            layout,
            stages,
            workgroup_size,
            base_field_fp,
            fp_scratch,
            mix_tot_scratch,
            mix_mul_scratch,
            scratch_params_buf,
            mix_pows_buf,
            params_buf,
            mix_pow_words,
        };
        self.staged_eval_check_pipelines
            .borrow_mut()
            .insert(key, pipeline.clone());
        Ok(pipeline)
    }

    /// Dispatch the staged-WGSL `eval_check` kernel for a DEF.
    /// Mirrors `dispatch_eval_check_poly_ext_interpreted` but without the
    /// runtime opcode stream — the per-DEF body is baked into the cached
    /// pipeline at first call. Returns `Ok(false)` if any GPU buffer is
    /// missing (caller falls through to interpreter); returns `Err` on
    /// codegen / compile / WebGPU API failure.
    pub(crate) fn dispatch_eval_check_poly_ext_staged(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        logical_globals: &[&WebGpuBuffer<BabyBearElem>; 2],
        taps: &TapSet<'_>,
        def: &PolyExtStepDef,
        poly_mix: BabyBearExtElem,
        domain_u32: u32,
        params: [u32; 12],
        base_field_fp: bool,
    ) -> Result<bool> {
        let (Some(check_gpu), Some(group0_gpu), Some(group1_gpu), Some(group2_gpu)) = (
            check.raw_buffer(),
            groups[0].raw_buffer(),
            groups[1].raw_buffer(),
            groups[2].raw_buffer(),
        ) else {
            return Ok(false);
        };
        let (Some(global0_gpu), Some(global1_gpu)) = (
            logical_globals[0].raw_buffer(),
            logical_globals[1].raw_buffer(),
        ) else {
            return Ok(false);
        };

        let pipeline = self.staged_eval_check_pipeline(def, taps, base_field_fp)?;

        let _timer = WebGpuStageTimer::new(format!(
            "eval_check_staged_submit domain={domain_u32} base_field_fp={base_field_fp}"
        ));

        let mix_pows = eval_check_mix_pows(def, poly_mix)?;
        let mut mix_pow_words = Vec::with_capacity(mix_pows.len() * BabyBearExtElem::EXT_SIZE);
        for value in mix_pows {
            mix_pow_words.extend(ext_words(value));
        }
        debug_assert_eq!(
            mix_pow_words.len(),
            pipeline.mix_pow_words,
            "staged mix_pows word count must match pipeline reservation"
        );
        // Rewrite the cached `mix_pows` storage buffer
        // (owned by the pipeline) instead of creating a fresh one per
        // eval_check call.
        self.write_buffer_named(
            &pipeline.mix_pows_buf,
            "webgpu_staged_eval_check_mix_pows",
            0,
            bytemuck::cast_slice(&mix_pow_words),
        )?;

        // Params layout matches the interpreter's; staged kernel ignores the
        // `instr_count` / `instr_base` / `ret_mix_slot` slots.
        let params_words = [
            params[0], params[1], params[2], params[3], params[4], params[5], domain_u32,
            0u32, // instr_count
            0u32, // instr_base
            0u32, // mix_pows_base (fresh buffer, base = 0)
            0u32, // ret_mix_slot (baked into write_check at codegen time)
            domain_u32, 0u32,       // cycle_base
            0u32,       // group0_chunk_base
            domain_u32, // group0_chunk_rows
            0u32,       // group1_chunk_base
            domain_u32, // group1_chunk_rows
            0u32,       // group2_chunk_base
            domain_u32, // group2_chunk_rows
            0u32,       // _pad0
            params[8], params[9], params[10], params[11],
        ];
        // Rewrite the cached params UBO (owned by the
        // pipeline) instead of allocating a fresh 96-byte uniform per
        // call.
        self.write_buffer_named(
            &pipeline.params_buf,
            "webgpu_staged_eval_check_params",
            0,
            bytemuck::cast_slice(&params_words),
        )?;

        // CUDA-shape dispatch. Closest analog to CUDA's
        // single `eval_check<<<grid, block>>>` launch covering the whole
        // domain — we issue ONE `dispatch_workgroups(tile_size,
        // num_tiles, 1)` per stage instead of a
        // `num_tiles * num_stages` setBindGroup+dispatch loop. Threads
        // compute `cycle = gid.y * tile_size + gid.x` inline, so no
        // per-tile UBO offset is needed. scratch_params is a single
        // 16-byte pipeline-constant UBO; pipeline/bind_group are bound
        // once and reused across all stages.
        let tile_size = STAGED_EVAL_CHECK_TILE_SIZE.min(domain_u32);
        let num_tiles = (domain_u32 + tile_size - 1) / tile_size;

        let bind_group = self.create_bind_group(
            "webgpu_staged_eval_check_bind_group",
            &pipeline.layout,
            &[
                WebGpuBufferBinding::new(0, check_gpu),
                WebGpuBufferBinding::new(1, group0_gpu),
                WebGpuBufferBinding::new(2, group1_gpu),
                WebGpuBufferBinding::new(3, group2_gpu),
                WebGpuBufferBinding::new(4, global0_gpu),
                WebGpuBufferBinding::new(5, global1_gpu),
                // binding 6 (instrs) omitted; not in pipeline layout
                WebGpuBufferBinding::new(7, &pipeline.mix_pows_buf),
                WebGpuBufferBinding {
                    binding: 8,
                    buffer: &pipeline.params_buf,
                    offset: 0,
                    size: Some(96),
                },
                WebGpuBufferBinding {
                    binding: 9,
                    buffer: &pipeline.scratch_params_buf,
                    offset: 0,
                    size: Some(16),
                },
                WebGpuBufferBinding::new(10, &pipeline.fp_scratch),
                WebGpuBufferBinding::new(11, &pipeline.mix_tot_scratch),
                WebGpuBufferBinding::new(12, &pipeline.mix_mul_scratch),
            ],
        )?;

        // Each stage gets its OWN compute pass within one
        // encoder. WebGPU does not guarantee that storage writes from
        // dispatch N are visible to dispatch N+1 inside the same compute
        // pass — only across pass boundaries does the implementation
        // insert a memory barrier. Iter 7c through 7g packed all stages
        // into one pass; if stage K+1 reads scratch that stage K wrote,
        // it could read stale data, corrupt the check buffer, and
        // cascade into a bad merkle/FRI allocation that SIGKILLs Chrome
        // during finalize_async. CUDA's sequential kernel launches each
        // implicitly synchronize — this is the closest WebGPU analog.
        // Single submit is preserved (one encoder.finish()), so queue
        // pressure stays flat.
        // Dispatch `(tile_size / workgroup_size, num_tiles,
        // 1)` workgroups per stage. Each workgroup runs `workgroup_size`
        // threads. Total threads = `tile_size * num_tiles` >= domain.
        // `tile_size` is 4096 and `workgroup_size` is a power of 2 in
        // [1, 64], so the division is exact.
        let workgroup_count_x = tile_size / pipeline.workgroup_size;
        let encoder = self.device.create_command_encoder();
        for stage in &pipeline.stages {
            let pass = encoder.begin_compute_pass();
            pass.set_pipeline(&stage.pipeline);
            pass.set_bind_group(0, Some(&bind_group));
            pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
                workgroup_count_x,
                num_tiles,
                1,
            );
            self.diagnostics.record_raw_compute_dispatch();
            pass.end();
        }
        self.submit(encoder.finish());

        // Pipeline cache is re-enabled.
        // Iter 7j evicted per-call as a diagnostic — proved persistent
        // cached state isn't the SIGKILL cause, but added significant
        // recompile cost (each call recompiled 2-4 staged shaders).
        // Revert to keep the pipeline cache so subsequent eval_check
        // calls hitting the same DEF reuse compiled stages.
        self.record_gpu_result_authoritative("eval_check", true);
        Ok(true)
    }

    pub(crate) fn can_allocate_gpu_buffer(&self, byte_len: u64) -> bool {
        byte_len <= self.max_buffer_size && byte_len <= MAX_EXACT_JS_INTEGER
    }

    pub(crate) fn record_gpu_result_with_cpu_mirror(&self, name: &'static str, gpu_used: bool) {
        if gpu_used {
            self.diagnostics.record_gpu_dispatch(name);
            self.diagnostics.record_cpu_mirror(name);
        } else {
            self.diagnostics.record_cpu_fallback(name);
        }
    }

    pub(crate) fn record_gpu_result_authoritative(&self, name: &'static str, gpu_used: bool) {
        if gpu_used {
            self.diagnostics.record_gpu_dispatch(name);
        } else {
            self.diagnostics.record_cpu_fallback(name);
        }
    }

    pub(crate) fn finish_hal_op<T>(
        &self,
        name: &'static str,
        gpu_used: bool,
        output: &WebGpuBuffer<T>,
        cpu_mirror: impl FnOnce(),
    ) {
        if self.gpu_authoritative() && gpu_used {
            self.record_gpu_result_authoritative(name, gpu_used);
            output.mark_gpu_dirty();
        } else {
            cpu_mirror();
            self.record_gpu_result_with_cpu_mirror(name, gpu_used);
            output.mark_cpu_result(gpu_used);
        }
    }

    pub(crate) fn storage_binding_fits<T>(&self, buffer: &WebGpuBuffer<T>) -> bool
    where
        T: Clone + Debug + PartialEq,
    {
        byte_len_for::<T>(buffer.size()) <= self.max_storage_binding_bytes()
    }

    pub(crate) fn eval_check_instruction_buffer(
        &self,
        base_field_fp: bool,
        instructions: &[u32],
    ) -> Result<web_sys::GpuBuffer> {
        let key = (base_field_fp, instructions.to_vec());
        if let Some(buffer) = self.eval_check_instruction_cache.borrow().get(&key) {
            return Ok(buffer.clone());
        }

        let instructions_name = if base_field_fp {
            "webgpu_eval_check_base_interpreter_instructions"
        } else {
            "webgpu_eval_check_interpreter_instructions"
        };
        let instructions_gpu =
            self.create_storage_buffer(instructions_name, byte_len_for::<u32>(instructions.len()))?;
        self.write_buffer_named(
            &instructions_gpu,
            instructions_name,
            0,
            bytemuck::cast_slice(instructions),
        )?;
        self.eval_check_instruction_cache
            .borrow_mut()
            .insert(key, instructions_gpu.clone());
        Ok(instructions_gpu)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_eval_check_poly_ext_interpreted(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        logical_globals: &[&WebGpuBuffer<BabyBearElem>; 2],
        taps: &TapSet<'_>,
        def: &PolyExtStepDef,
        poly_mix: BabyBearExtElem,
        domain_u32: u32,
        params: [u32; 12],
        base_field_fp: bool,
    ) -> Result<bool> {
        let (Some(group0_gpu), Some(group1_gpu), Some(group2_gpu)) = (
            groups[0].raw_buffer(),
            groups[1].raw_buffer(),
            groups[2].raw_buffer(),
        ) else {
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        };
        self.dispatch_eval_check_poly_ext_interpreted_with_groups(
            check,
            logical_globals,
            [group0_gpu, group1_gpu, group2_gpu],
            [params[1], params[2], params[3]],
            [0, 0, 0],
            [domain_u32, domain_u32, domain_u32],
            domain_u32,
            0,
            taps,
            def,
            poly_mix,
            domain_u32,
            params,
            base_field_fp,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_eval_check_poly_ext_interpreted_with_groups(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        logical_globals: &[&WebGpuBuffer<BabyBearElem>; 2],
        group_gpus: [&web_sys::GpuBuffer; 3],
        group_bases: [u32; 3],
        group_chunk_bases: [u32; 3],
        group_chunk_rows: [u32; 3],
        dispatch_count: u32,
        cycle_base: u32,
        taps: &TapSet<'_>,
        def: &PolyExtStepDef,
        poly_mix: BabyBearExtElem,
        domain_u32: u32,
        params: [u32; 12],
        base_field_fp: bool,
    ) -> Result<bool> {
        let (instructions, fp_slots, ext_slots, mix_slots, ret_mix_slot) = match if base_field_fp {
            eval_check_base_interpreter_instructions(taps, def)
        } else {
            eval_check_interpreter_instructions(taps, def).map(
                |(instructions, fp_slots, mix_slots, ret)| {
                    (instructions, fp_slots, 0, mix_slots, ret)
                },
            )
        } {
            Ok(program) => program,
            Err(err) => {
                log_webgpu_stage(&format!(
                    "webgpu eval_check interpreter build failed: {err}"
                ));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
        };
        if instructions.is_empty() {
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }
        let instruction_count = instructions.len() / WEBGPU_EVAL_CHECK_INSTRUCTION_WORDS;
        let base_private_parallel =
            base_field_fp && fp_slots <= WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS;
        let base_workgroup_size = if base_private_parallel {
            WEBGPU_EVAL_CHECK_INTERPRETER_WORKGROUP_SIZE
        } else if base_field_fp {
            self.eval_check_base_workgroup_lanes(fp_slots, ext_slots, mix_slots)
        } else {
            WEBGPU_EVAL_CHECK_INTERPRETER_WORKGROUP_SIZE
        };
        let interpreter_label = if base_private_parallel {
            "eval_check_base_private_interpreter_submit"
        } else if base_field_fp {
            "eval_check_base_interpreter_submit"
        } else {
            "eval_check_interpreter_submit"
        };
        let _timer = WebGpuStageTimer::new(format!(
            "{interpreter_label} domain={} dispatch_count={} cycle_base={} instructions={} fp_slots={} ext_slots={} mix_slots={} workgroup_size={}",
            domain_u32,
            dispatch_count,
            cycle_base,
            instruction_count,
            fp_slots,
            ext_slots,
            mix_slots,
            base_workgroup_size
        ));

        let mix_pows = eval_check_mix_pows(def, poly_mix)?;
        let mut mix_pow_words = Vec::with_capacity(mix_pows.len() * BabyBearExtElem::EXT_SIZE);
        for value in mix_pows {
            mix_pow_words.extend(ext_words(value));
        }

        let instructions_gpu =
            self.eval_check_instruction_buffer(base_field_fp, instructions.as_slice())?;
        let mix_pows_name = if base_field_fp {
            "webgpu_eval_check_base_interpreter_mix_pows"
        } else {
            "webgpu_eval_check_interpreter_mix_pows"
        };
        let mix_pows_gpu =
            self.create_storage_buffer(mix_pows_name, byte_len_for::<u32>(mix_pow_words.len()))?;
        self.write_buffer_named(
            &mix_pows_gpu,
            mix_pows_name,
            0,
            bytemuck::cast_slice(&mix_pow_words),
        )?;

        let params = [
            params[0],
            group_bases[0],
            group_bases[1],
            group_bases[2],
            params[4],
            params[5],
            domain_u32,
            u32::try_from(instruction_count)
                .expect("WebGPU eval_check instruction count exceeds u32"),
            0,
            0,
            u32::try_from(ret_mix_slot).expect("WebGPU eval_check ret slot exceeds u32"),
            dispatch_count,
            cycle_base,
            group_chunk_bases[0],
            group_chunk_rows[0],
            group_chunk_bases[1],
            group_chunk_rows[1],
            group_chunk_bases[2],
            group_chunk_rows[2],
            0,
            params[8],
            params[9],
            params[10],
            params[11],
        ];
        let params = self.create_uniform_buffer(
            "webgpu_eval_check_interpreter_params",
            bytemuck::cast_slice(&params),
        )?;

        let pipeline = match self.eval_check_interpreter_pipeline(
            base_field_fp,
            base_private_parallel,
            fp_slots,
            ext_slots,
            mix_slots,
            base_workgroup_size,
        ) {
            Ok(pipeline) => pipeline,
            Err(err) => {
                log_webgpu_stage(&format!(
                    "webgpu eval_check interpreter pipeline failed: {err}"
                ));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
        };

        let bind_group = self.create_bind_group(
            "webgpu_eval_check_interpreter_bind_group",
            &pipeline.layout,
            &[
                WebGpuBufferBinding::new(
                    0,
                    check
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing interpreted eval_check check buffer"))?,
                ),
                WebGpuBufferBinding::new(1, group_gpus[0]),
                WebGpuBufferBinding::new(2, group_gpus[1]),
                WebGpuBufferBinding::new(3, group_gpus[2]),
                WebGpuBufferBinding::new(
                    4,
                    logical_globals[0]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing interpreted eval_check global 0 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    5,
                    logical_globals[1]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing interpreted eval_check global 1 buffer"))?,
                ),
                WebGpuBufferBinding::new(6, &instructions_gpu),
                WebGpuBufferBinding::new(7, &mix_pows_gpu),
                WebGpuBufferBinding {
                    binding: 8,
                    buffer: &params,
                    offset: 0,
                    size: Some(96),
                },
            ],
        )?;
        let workgroups = if base_field_fp && !base_private_parallel {
            dispatch_count.div_ceil(base_workgroup_size)
        } else {
            dispatch_count.div_ceil(WEBGPU_EVAL_CHECK_INTERPRETER_WORKGROUP_SIZE)
        };
        self.dispatch_compute_1d(&pipeline.kernel, &bind_group, workgroups);
        check.mark_gpu_dirty();
        self.record_gpu_result_authoritative("eval_check", true);
        Ok(true)
    }

    pub(crate) fn dispatch_eval_check_pack_group_chunk(
        &self,
        group: &WebGpuBuffer<BabyBearElem>,
        cols: usize,
        domain: usize,
        chunk_base: usize,
        chunk_rows: usize,
    ) -> Result<Option<web_sys::GpuBuffer>> {
        if cols == 0 || chunk_rows == 0 {
            return Ok(None);
        }
        let Some(group_gpu) = group.raw_buffer() else {
            return Ok(None);
        };
        if !group.gpu_is_current() {
            return Ok(None);
        }

        let domain_bytes = byte_len_for::<BabyBearElem>(domain);
        if domain_bytes == 0
            || domain_bytes > self.max_storage_binding_bytes()
            || domain_bytes % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0
        {
            return Ok(None);
        }

        let group_base = group.byte_offset();
        if group_base % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0 {
            return Ok(None);
        }
        let max_cols_per_binding = (self.max_storage_binding_bytes() / domain_bytes) as usize;
        if max_cols_per_binding == 0 {
            return Ok(None);
        }

        let chunk_len = cols
            .checked_mul(chunk_rows)
            .ok_or_else(|| anyhow!("WebGPU eval_check group chunk length overflow"))?;
        let chunk_bytes = byte_len_for::<BabyBearElem>(chunk_len);
        if chunk_bytes == 0 || chunk_bytes > self.max_storage_binding_bytes() {
            return Ok(None);
        }

        let chunk_gpu =
            self.create_storage_buffer("webgpu_eval_check_interpreter_group_chunk", chunk_bytes)?;
        let layout = self.create_bind_group_layout(
            "webgpu_eval_check_pack_group_chunk_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_eval_check_pack_group_chunk",
            EVAL_CHECK_PACK_GROUP_CHUNK_WGSL,
            "main",
            &[layout.clone()],
        )?;

        for col_start in (0..cols).step_by(max_cols_per_binding) {
            let col_count = (cols - col_start).min(max_cols_per_binding);
            let col_offset = group_base
                .checked_add(byte_len_for::<BabyBearElem>(
                    col_start
                        .checked_mul(domain)
                        .ok_or_else(|| anyhow!("WebGPU eval_check pack column overflow"))?,
                ))
                .ok_or_else(|| anyhow!("WebGPU eval_check pack column offset overflow"))?;
            if col_offset % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0 {
                return Ok(None);
            }
            let binding_bytes = domain_bytes
                .checked_mul(
                    u64::try_from(col_count)
                        .expect("WebGPU eval_check pack column count exceeds u64"),
                )
                .ok_or_else(|| anyhow!("WebGPU eval_check pack binding size overflow"))?;
            let params = [
                u32::try_from(domain).expect("WebGPU eval_check pack domain exceeds u32"),
                u32::try_from(chunk_base).expect("WebGPU eval_check pack chunk base exceeds u32"),
                u32::try_from(chunk_rows).expect("WebGPU eval_check pack chunk rows exceeds u32"),
                u32::try_from(
                    col_start
                        .checked_mul(chunk_rows)
                        .ok_or_else(|| anyhow!("WebGPU eval_check pack dst offset overflow"))?,
                )
                .expect("WebGPU eval_check pack dst offset exceeds u32"),
                u32::try_from(col_count).expect("WebGPU eval_check pack col count exceeds u32"),
                0,
                0,
                0,
            ];
            let params = self.create_uniform_buffer(
                "webgpu_eval_check_pack_group_chunk_params",
                bytemuck::cast_slice(&params),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_eval_check_pack_group_chunk_bind_group",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, &chunk_gpu),
                    WebGpuBufferBinding {
                        binding: 1,
                        buffer: group_gpu,
                        offset: col_offset,
                        size: Some(binding_bytes),
                    },
                    WebGpuBufferBinding {
                        binding: 2,
                        buffer: &params,
                        offset: 0,
                        size: Some(32),
                    },
                ],
            )?;
            self.dispatch_compute(
                &kernel,
                &bind_group,
                u32::try_from(chunk_rows)
                    .expect("WebGPU eval_check pack chunk rows exceeds u32")
                    .div_ceil(WEBGPU_WORKGROUP_SIZE),
                u32::try_from(col_count).expect("WebGPU eval_check pack col count exceeds u32"),
                1,
            );
        }

        Ok(Some(chunk_gpu))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_eval_check_poly_ext_interpreted_group_chunks(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        logical_globals: &[&WebGpuBuffer<BabyBearElem>; 2],
        taps: &TapSet<'_>,
        def: &PolyExtStepDef,
        poly_mix: BabyBearExtElem,
        domain_u32: u32,
        params: [u32; 12],
        base_field_fp: bool,
    ) -> Result<bool> {
        let chunked_groups = [
            groups[0].raw_buffer().is_none() || !self.storage_binding_fits(groups[0]),
            groups[1].raw_buffer().is_none() || !self.storage_binding_fits(groups[1]),
            groups[2].raw_buffer().is_none() || !self.storage_binding_fits(groups[2]),
        ];

        for (name, buffer) in [
            ("check", check),
            ("global0", logical_globals[0]),
            ("global1", logical_globals[1]),
        ] {
            if buffer.raw_buffer().is_none() {
                log_webgpu_stage(&format!(
                    "browser-prove:stage eval_check chunked_groups unavailable missing_gpu_buffer name={name}"
                ));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
            if !self.storage_binding_fits(buffer) {
                log_webgpu_stage(&format!(
                    "browser-prove:stage eval_check chunked_groups unavailable storage_binding name={} bytes={} max_binding={}",
                    name,
                    byte_len_for::<BabyBearElem>(buffer.size()),
                    self.max_storage_binding_bytes()
                ));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
        }

        for group_id in 0..3 {
            if chunked_groups[group_id] {
                if !groups[group_id].cpu_is_current()
                    && !(groups[group_id].raw_buffer().is_some()
                        && groups[group_id].gpu_is_current())
                {
                    log_webgpu_stage(&format!(
                        "browser-prove:stage eval_check chunked_groups unavailable stale_group group={group_id}"
                    ));
                    self.record_gpu_result_authoritative("eval_check", false);
                    return Ok(false);
                }
            } else if groups[group_id].raw_buffer().is_none()
                || !self.storage_binding_fits(groups[group_id])
            {
                log_webgpu_stage(&format!(
                    "browser-prove:stage eval_check chunked_groups unavailable group={group_id}"
                ));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
        }

        let domain = domain_u32 as usize;
        if domain == 0 {
            return Ok(true);
        }
        let max_back_rows = taps
            .taps()
            .filter(|tap| chunked_groups[tap.group()])
            .map(|tap| (tap.back() * INV_RATE) % domain)
            .max()
            .unwrap_or(0);
        let max_chunk_elems =
            (self.max_storage_binding_bytes() / mem::size_of::<BabyBearElem>() as u64) as usize;
        let mut max_dispatch_rows = domain;
        for group_id in 0..3 {
            if !chunked_groups[group_id] {
                continue;
            }
            let cols = taps.group_size(group_id);
            if cols == 0 {
                continue;
            }
            let max_chunk_rows = max_chunk_elems / cols;
            if max_chunk_rows <= max_back_rows {
                log_webgpu_stage(&format!(
                    "browser-prove:stage eval_check chunked_groups unavailable group={} rows_per_chunk={} max_back_rows={} cols={}",
                    group_id, max_chunk_rows, max_back_rows, cols
                ));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
            max_dispatch_rows = max_dispatch_rows.min(max_chunk_rows - max_back_rows);
        }
        log_webgpu_stage(&format!(
            "browser-prove:stage eval_check chunked_groups start domain={} groups={:?} max_back_rows={} dispatch_rows={}",
            domain, chunked_groups, max_back_rows, max_dispatch_rows
        ));

        let mut cycle_base = 0usize;
        while cycle_base < domain {
            let dispatch_rows = max_dispatch_rows.min(domain - cycle_base);
            let chunk_rows = dispatch_rows + max_back_rows;
            let chunk_base = (cycle_base + domain - max_back_rows) % domain;
            let mut chunk_gpus: [Option<web_sys::GpuBuffer>; 3] = [None, None, None];

            for group_id in 0..3 {
                if !chunked_groups[group_id] {
                    continue;
                }
                let cols = taps.group_size(group_id);
                if let Some(chunk_gpu) = self.dispatch_eval_check_pack_group_chunk(
                    groups[group_id],
                    cols,
                    domain,
                    chunk_base,
                    chunk_rows,
                )? {
                    chunk_gpus[group_id] = Some(chunk_gpu);
                    continue;
                }

                ensure!(
                    groups[group_id].cpu_is_current(),
                    "WebGPU chunked eval_check group {group_id} requires a current CPU shadow"
                );
                let mut chunk = vec![BabyBearElem::ZERO; cols * chunk_rows];
                groups[group_id].cpu.view(|source| {
                    for col in 0..cols {
                        let src_col = &source[col * domain..(col + 1) * domain];
                        let dst_col = &mut chunk[col * chunk_rows..(col + 1) * chunk_rows];
                        for (local_row, dst) in dst_col.iter_mut().enumerate() {
                            *dst = src_col[(chunk_base + local_row) % domain];
                        }
                    }
                });

                let chunk_gpu = self.create_storage_buffer(
                    "webgpu_eval_check_interpreter_group_chunk",
                    byte_len_for::<BabyBearElem>(chunk.len()),
                )?;
                self.write_buffer_named(
                    &chunk_gpu,
                    "webgpu_eval_check_interpreter_group_chunk",
                    0,
                    bytemuck::cast_slice(&chunk),
                )?;
                chunk_gpus[group_id] = Some(chunk_gpu);
            }

            let group0_gpu = chunk_gpus[0]
                .as_ref()
                .or_else(|| groups[0].raw_buffer())
                .ok_or_else(|| anyhow!("missing eval_check group 0 chunk buffer"))?;
            let group1_gpu = chunk_gpus[1]
                .as_ref()
                .or_else(|| groups[1].raw_buffer())
                .ok_or_else(|| anyhow!("missing eval_check group 1 chunk buffer"))?;
            let group2_gpu = chunk_gpus[2]
                .as_ref()
                .or_else(|| groups[2].raw_buffer())
                .ok_or_else(|| anyhow!("missing eval_check group 2 chunk buffer"))?;

            let group_bases = [
                if chunked_groups[0] { 0 } else { params[1] },
                if chunked_groups[1] { 0 } else { params[2] },
                if chunked_groups[2] { 0 } else { params[3] },
            ];
            let group_chunk_bases = [
                if chunked_groups[0] {
                    u32::try_from(chunk_base)
                        .expect("WebGPU eval_check group 0 chunk base exceeds u32")
                } else {
                    0
                },
                if chunked_groups[1] {
                    u32::try_from(chunk_base)
                        .expect("WebGPU eval_check group 1 chunk base exceeds u32")
                } else {
                    0
                },
                if chunked_groups[2] {
                    u32::try_from(chunk_base)
                        .expect("WebGPU eval_check group 2 chunk base exceeds u32")
                } else {
                    0
                },
            ];
            let group_chunk_rows = [
                if chunked_groups[0] {
                    u32::try_from(chunk_rows)
                        .expect("WebGPU eval_check group 0 chunk rows exceeds u32")
                } else {
                    domain_u32
                },
                if chunked_groups[1] {
                    u32::try_from(chunk_rows)
                        .expect("WebGPU eval_check group 1 chunk rows exceeds u32")
                } else {
                    domain_u32
                },
                if chunked_groups[2] {
                    u32::try_from(chunk_rows)
                        .expect("WebGPU eval_check group 2 chunk rows exceeds u32")
                } else {
                    domain_u32
                },
            ];

            let dispatched = self.dispatch_eval_check_poly_ext_interpreted_with_groups(
                check,
                logical_globals,
                [group0_gpu, group1_gpu, group2_gpu],
                group_bases,
                group_chunk_bases,
                group_chunk_rows,
                u32::try_from(dispatch_rows).expect("WebGPU eval_check dispatch rows exceeds u32"),
                u32::try_from(cycle_base).expect("WebGPU eval_check cycle base exceeds u32"),
                taps,
                def,
                poly_mix,
                domain_u32,
                params,
                base_field_fp,
            )?;
            if !dispatched {
                return Ok(false);
            }
            cycle_base += dispatch_rows;
        }

        log_webgpu_stage("browser-prove:stage eval_check chunked_groups done");
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_eval_check_poly_ext_split(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        logical_globals: &[&WebGpuBuffer<BabyBearElem>; 2],
        taps: &TapSet<'_>,
        def: &PolyExtStepDef,
        poly_mix: BabyBearExtElem,
        domain_u32: u32,
        params: [u32; 12],
    ) -> Result<bool> {
        let program = eval_check_program(def)?;
        let terms = eval_check_flatten_terms(&program, def.ret)?;
        if terms.is_empty() {
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }

        let term_chunks = match eval_check_split_term_chunks(&program, &terms) {
            Ok(chunks) => chunks,
            Err(err) => {
                log_webgpu_stage(&format!("webgpu eval_check split chunking failed: {err}"));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
        };

        let mut wgsl_chunks = Vec::new();
        for (chunk_idx, terms) in term_chunks.iter().enumerate() {
            let wgsl = match build_eval_check_split_wgsl(taps, &program, terms, chunk_idx == 0) {
                Ok(wgsl) => wgsl,
                Err(err) => {
                    log_webgpu_stage(&format!(
                        "webgpu eval_check split WGSL build failed at chunk {chunk_idx}: {err}"
                    ));
                    self.record_gpu_result_authoritative("eval_check", false);
                    return Ok(false);
                }
            };
            wgsl_chunks.push(wgsl);
        }

        let mix_pows = eval_check_all_mix_pows(def, poly_mix)?;
        let mut mix_pow_words = Vec::with_capacity(mix_pows.len() * BabyBearExtElem::EXT_SIZE);
        for value in mix_pows {
            mix_pow_words.extend(ext_words(value));
        }
        let _timer = WebGpuStageTimer::new(format!(
            "eval_check_split_submit domain={} chunks={}",
            domain_u32,
            term_chunks.len()
        ));
        let mix_pows_gpu = self.create_storage_buffer(
            "webgpu_eval_check_split_mix_pows",
            byte_len_for::<u32>(mix_pow_words.len()),
        )?;
        self.write_buffer_named(
            &mix_pows_gpu,
            "webgpu_eval_check_split_mix_pows",
            0,
            bytemuck::cast_slice(&mix_pow_words),
        )?;

        let params = self.create_uniform_buffer(
            "webgpu_eval_check_split_params",
            bytemuck::cast_slice(&params),
        )?;
        let layout = self.create_bind_group_layout(
            "webgpu_eval_check_split_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::read_only_storage(6, 0),
                WebGpuBindingLayout::uniform(7, 48),
            ],
        )?;

        let mut kernels = Vec::with_capacity(wgsl_chunks.len());
        for (chunk_idx, wgsl) in wgsl_chunks.iter().enumerate() {
            let kernel = match self.create_compute_kernel(
                "webgpu_eval_check_split",
                wgsl,
                "main",
                &[layout.clone()],
            ) {
                Ok(kernel) => kernel,
                Err(err) => {
                    log_webgpu_stage(&format!(
                        "webgpu eval_check split pipeline failed at chunk {chunk_idx}: {err}"
                    ));
                    self.record_gpu_result_authoritative("eval_check", false);
                    return Ok(false);
                }
            };
            kernels.push(kernel);
        }

        let bind_group = self.create_bind_group(
            "webgpu_eval_check_split_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(
                    0,
                    check
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing split eval_check check buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    1,
                    groups[0]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing split eval_check group 0 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    2,
                    groups[1]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing split eval_check group 1 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    3,
                    groups[2]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing split eval_check group 2 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    4,
                    logical_globals[0]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing split eval_check global 0 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    5,
                    logical_globals[1]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing split eval_check global 1 buffer"))?,
                ),
                WebGpuBufferBinding::new(6, &mix_pows_gpu),
                WebGpuBufferBinding {
                    binding: 7,
                    buffer: &params,
                    offset: 0,
                    size: Some(48),
                },
            ],
        )?;
        let workgroups = domain_u32.div_ceil(WEBGPU_WORKGROUP_SIZE);
        self.dispatch_compute_1d_sequence(kernels.as_slice(), &bind_group, workgroups);
        check.mark_gpu_dirty();
        self.record_gpu_result_authoritative("eval_check", true);
        Ok(true)
    }

    /// Dispatch a generated straight-line WebGPU check-polynomial evaluator.
    ///
    /// This is intentionally conservative while the full circuit path is being
    /// moved to GPU-authoritative execution: unsupported shapes return
    /// `Ok(false)` so callers can use the portable CPU fallback.
    pub fn dispatch_eval_check_poly_ext(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        globals: &[&WebGpuBuffer<BabyBearElem>],
        taps: &TapSet<'_>,
        def: &PolyExtStepDef,
        poly_mix: BabyBearExtElem,
        po2: usize,
        steps: usize,
    ) -> Result<bool> {
        if !self.eval_check_gpu_enabled.get() {
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }
        if INV_RATE != 4 || taps.num_groups() != 3 || groups.len() != 3 || globals.len() != 2 {
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }

        let domain = steps
            .checked_mul(INV_RATE)
            .ok_or_else(|| anyhow!("WebGPU eval_check domain overflow"))?;
        if domain == 0 {
            return Ok(true);
        }
        ensure!(
            check.size() == BabyBearExtElem::EXT_SIZE * domain,
            "WebGPU eval_check check size mismatch: got {}, expected {}",
            check.size(),
            BabyBearExtElem::EXT_SIZE * domain
        );
        for (group_id, group) in groups.iter().enumerate() {
            let expected = taps.group_size(group_id) * domain;
            ensure!(
                group.size() == expected,
                "WebGPU eval_check group {group_id} size mismatch: got {}, expected {expected}",
                group.size()
            );
        }

        let logical_globals = [globals[1], globals[0]];
        let mut min_global_sizes = [0usize; 2];
        for op in def.block {
            if let PolyExtStep::GetGlobal(arg, offset) = op {
                if *arg >= min_global_sizes.len() {
                    self.record_gpu_result_authoritative("eval_check", false);
                    return Ok(false);
                }
                min_global_sizes[*arg] = min_global_sizes[*arg].max(offset + 1);
            }
        }
        for (idx, min_size) in min_global_sizes.into_iter().enumerate() {
            ensure!(
                logical_globals[idx].size() >= min_size,
                "WebGPU eval_check global {idx} too small: got {}, need at least {min_size}",
                logical_globals[idx].size()
            );
        }

        let Some(check_gpu) = check.raw_buffer() else {
            log_webgpu_stage(
                "browser-prove:stage eval_check fallback missing_gpu_buffer name=check",
            );
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        };
        let group_gpus = [
            groups[0].raw_buffer(),
            groups[1].raw_buffer(),
            groups[2].raw_buffer(),
        ];
        for group_id in 0..3 {
            if group_gpus[group_id].is_none() {
                log_webgpu_stage(&format!(
                    "browser-prove:stage eval_check chunked_groups candidate missing_gpu_buffer group={group_id}"
                ));
            }
        }
        let Some(global0_gpu) = logical_globals[0].raw_buffer() else {
            log_webgpu_stage(
                "browser-prove:stage eval_check fallback missing_gpu_buffer name=global0",
            );
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        };
        let Some(global1_gpu) = logical_globals[1].raw_buffer() else {
            log_webgpu_stage(
                "browser-prove:stage eval_check fallback missing_gpu_buffer name=global1",
            );
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        };
        let group_storage_fits = [
            self.storage_binding_fits(groups[0]),
            self.storage_binding_fits(groups[1]),
            self.storage_binding_fits(groups[2]),
        ];
        if !self.storage_binding_fits(check)
            || !self.storage_binding_fits(logical_globals[0])
            || !self.storage_binding_fits(logical_globals[1])
        {
            let max_binding = self.max_storage_binding_bytes();
            for (name, elem_count) in [
                ("check", check.size()),
                ("group0", groups[0].size()),
                ("group1", groups[1].size()),
                ("global0", logical_globals[0].size()),
                ("global1", logical_globals[1].size()),
            ] {
                let byte_len = byte_len_for::<BabyBearElem>(elem_count);
                if byte_len > max_binding {
                    log_webgpu_stage(&format!(
                        "browser-prove:stage eval_check fallback storage_binding name={} bytes={} max_binding={}",
                        name, byte_len, max_binding
                    ));
                }
            }
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }
        for group_id in 0..3 {
            if !group_storage_fits[group_id] {
                log_webgpu_stage(&format!(
                    "browser-prove:stage eval_check chunked_groups candidate storage_binding group={} bytes={} max_binding={}",
                    group_id,
                    byte_len_for::<BabyBearElem>(groups[group_id].size()),
                    self.max_storage_binding_bytes()
                ));
            }
        }
        let group_can_bind = [
            group_gpus[0].is_some() && group_storage_fits[0],
            group_gpus[1].is_some() && group_storage_fits[1],
            group_gpus[2].is_some() && group_storage_fits[2],
        ];

        let check_base =
            u32::try_from(check.elem_offset).map_err(|_| anyhow!("check offset exceeds u32"))?;
        let group0_base = u32::try_from(groups[0].elem_offset)
            .map_err(|_| anyhow!("group 0 offset exceeds u32"))?;
        let group1_base = u32::try_from(groups[1].elem_offset)
            .map_err(|_| anyhow!("group 1 offset exceeds u32"))?;
        let group2_base = u32::try_from(groups[2].elem_offset)
            .map_err(|_| anyhow!("group 2 offset exceeds u32"))?;
        let global0_base = u32::try_from(logical_globals[0].elem_offset)
            .map_err(|_| anyhow!("global 0 offset exceeds u32"))?;
        let global1_base = u32::try_from(logical_globals[1].elem_offset)
            .map_err(|_| anyhow!("global 1 offset exceeds u32"))?;
        let domain_u32 =
            u32::try_from(domain).map_err(|_| anyhow!("WebGPU eval_check domain exceeds u32"))?;

        let use_chunked_groups = def.block.len() > WEBGPU_EVAL_CHECK_MAX_POLY_EXT_STEPS
            && !WEBGPU_EVAL_CHECK_ENABLE_SPLIT
            && !group_can_bind.iter().all(|can_bind| *can_bind);

        for (idx, group) in groups.iter().enumerate() {
            if !use_chunked_groups || group_can_bind[idx] {
                group.sync_cpu_to_gpu(self)?;
            }
        }
        for global in &logical_globals {
            global.sync_cpu_to_gpu(self)?;
        }

        let invs = eval_check_zerofier_inv_words(po2, steps);
        let params = [
            check_base,
            group0_base,
            group1_base,
            group2_base,
            global0_base,
            global1_base,
            domain_u32,
            0,
            invs[0],
            invs[1],
            invs[2],
            invs[3],
        ];

        // Try the staged-WGSL fast path first when
        // the runtime flag is set and all groups bind in a single dispatch.
        // Default `staged_eval_check_enabled = false`, so this branch is
        // dead code on the production path unless browser parity tests
        // opt in per-fixture. Any failure (codegen, compile,
        // dispatch) falls through to the interpreter below.
        //
        // Only attempt staged for DEFs with
        // enough ops to make staging worthwhile (rv32im production has
        // ~20k+ ops; recursion's DEF is much smaller and currently
        // SIGKILLs Chrome somewhere in the lift's finalize_async flow
        // when staged. Limit the staged path to the heavy DEFs it was
        // validated on; the recursion lift stays on the proven interpreter
        // path (staged emission for the recursion DEF misbehaved and was
        // never root-caused). Threshold of 8000 keeps rv32im above and
        // recursion / smaller DEFs below.
        if self.staged_eval_check_enabled.get()
            && def.block.len() >= STAGED_EVAL_CHECK_MIN_BLOCK_OPS
            && group_can_bind.iter().all(|can_bind| *can_bind)
        {
            // Base-field DEFs (rv32im) take the optimized `FieldMode::Base`
            // path; DEFs that use `ConstExt` must use `FieldMode::Ext`.
            let base_field_fp = !def
                .block
                .iter()
                .any(|op| matches!(op, PolyExtStep::ConstExt(_, _, _, _)));
            match self.dispatch_eval_check_poly_ext_staged(
                check,
                groups,
                &logical_globals,
                taps,
                def,
                poly_mix,
                domain_u32,
                params,
                base_field_fp,
            ) {
                Ok(true) => return Ok(true),
                Ok(false) => {
                    log_webgpu_stage(
                        "browser-prove:stage eval_check staged unavailable; falling back to interpreter",
                    );
                }
                Err(err) => {
                    log_webgpu_stage(&format!(
                        "browser-prove:stage eval_check staged failed: {err}; falling back to interpreter"
                    ));
                }
            }
        }

        if def.block.len() > WEBGPU_EVAL_CHECK_MAX_POLY_EXT_STEPS {
            let base_interpreter_program = eval_check_base_interpreter_instructions(taps, def);
            if let Ok((_, fp_slots, _, _, _)) = &base_interpreter_program {
                if WEBGPU_EVAL_CHECK_ENABLE_SPLIT
                    && *fp_slots > WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS
                    && group_can_bind.iter().all(|can_bind| *can_bind)
                {
                    match self.dispatch_eval_check_poly_ext_split(
                        check,
                        groups,
                        &logical_globals,
                        taps,
                        def,
                        poly_mix,
                        domain_u32,
                        params,
                    ) {
                        Ok(true) => return Ok(true),
                        Ok(false) => {
                            log_webgpu_stage(
                                "browser-prove:stage eval_check split unavailable; falling back to base interpreter",
                            );
                        }
                        Err(err) => {
                            log_webgpu_stage(&format!(
                                "browser-prove:stage eval_check split failed: {err}; falling back to base interpreter"
                            ));
                        }
                    }
                }
                if !group_can_bind.iter().all(|can_bind| *can_bind) {
                    return self.dispatch_eval_check_poly_ext_interpreted_group_chunks(
                        check,
                        groups,
                        &logical_globals,
                        taps,
                        def,
                        poly_mix,
                        domain_u32,
                        params,
                        true,
                    );
                }
                return self.dispatch_eval_check_poly_ext_interpreted(
                    check,
                    groups,
                    &logical_globals,
                    taps,
                    def,
                    poly_mix,
                    domain_u32,
                    params,
                    true,
                );
            }

            let interpreter_fits = eval_check_interpreter_instructions(taps, def).is_ok();
            if interpreter_fits {
                if !group_can_bind.iter().all(|can_bind| *can_bind) {
                    return self.dispatch_eval_check_poly_ext_interpreted_group_chunks(
                        check,
                        groups,
                        &logical_globals,
                        taps,
                        def,
                        poly_mix,
                        domain_u32,
                        params,
                        false,
                    );
                }
                return self.dispatch_eval_check_poly_ext_interpreted(
                    check,
                    groups,
                    &logical_globals,
                    taps,
                    def,
                    poly_mix,
                    domain_u32,
                    params,
                    false,
                );
            }

            if WEBGPU_EVAL_CHECK_ENABLE_SPLIT && group_can_bind.iter().all(|can_bind| *can_bind) {
                return self.dispatch_eval_check_poly_ext_split(
                    check,
                    groups,
                    &logical_globals,
                    taps,
                    def,
                    poly_mix,
                    domain_u32,
                    params,
                );
            }

            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }
        if !group_can_bind.iter().all(|can_bind| *can_bind) {
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }

        let mix_pows = eval_check_mix_pows(def, poly_mix)?;
        let mut mix_pow_words = Vec::with_capacity(mix_pows.len() * BabyBearExtElem::EXT_SIZE);
        for value in mix_pows {
            mix_pow_words.extend(ext_words(value));
        }
        let mix_pows_gpu = self.create_storage_buffer(
            "webgpu_eval_check_mix_pows",
            byte_len_for::<u32>(mix_pow_words.len()),
        )?;
        self.write_buffer_named(
            &mix_pows_gpu,
            "webgpu_eval_check_mix_pows",
            0,
            bytemuck::cast_slice(&mix_pow_words),
        )?;

        let params =
            self.create_uniform_buffer("webgpu_eval_check_params", bytemuck::cast_slice(&params))?;

        let wgsl = build_eval_check_wgsl(taps, def)?;
        let layout = self.create_bind_group_layout(
            "webgpu_eval_check_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::read_only_storage(6, 0),
                WebGpuBindingLayout::uniform(7, 48),
            ],
        )?;
        let kernel =
            self.create_compute_kernel("webgpu_eval_check", &wgsl, "main", &[layout.clone()])?;
        let bind_group = self.create_bind_group(
            "webgpu_eval_check_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, check_gpu),
                WebGpuBufferBinding::new(
                    1,
                    group_gpus[0].ok_or_else(|| anyhow!("missing eval_check group 0 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    2,
                    group_gpus[1].ok_or_else(|| anyhow!("missing eval_check group 1 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    3,
                    group_gpus[2].ok_or_else(|| anyhow!("missing eval_check group 2 buffer"))?,
                ),
                WebGpuBufferBinding::new(4, global0_gpu),
                WebGpuBufferBinding::new(5, global1_gpu),
                WebGpuBufferBinding::new(6, &mix_pows_gpu),
                WebGpuBufferBinding {
                    binding: 7,
                    buffer: &params,
                    offset: 0,
                    size: Some(48),
                },
            ],
        )?;
        let workgroups = domain_u32.div_ceil(WEBGPU_WORKGROUP_SIZE);
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
        check.mark_gpu_dirty();
        self.record_gpu_result_authoritative("eval_check", true);
        Ok(true)
    }

    pub(crate) fn can_dispatch_batch_interpolate_ntt(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
    ) -> bool {
        self.batch_interpolate_ntt_gpu_enabled.get()
            && (io.size() == 0 || (io.raw_buffer().is_some() && self.storage_binding_fits(io)))
    }

    pub(crate) fn can_dispatch_batch_interpolate_ntt_from(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
    ) -> bool {
        if !self.batch_interpolate_ntt_gpu_enabled.get() || output.size() != input.size() {
            return false;
        }
        if output.size() == 0 {
            return true;
        }
        let (Some(output_gpu), Some(input_gpu)) = (output.raw_buffer(), input.raw_buffer()) else {
            return false;
        };
        output_gpu != input_gpu
            && self.storage_binding_fits(output)
            && self.storage_binding_fits(input)
    }

    pub(crate) fn can_dispatch_zk_shift(&self, io: &WebGpuBuffer<BabyBearElem>) -> bool {
        self.zk_shift_gpu_enabled.get()
            && (io.size() == 0 || (io.raw_buffer().is_some() && self.storage_binding_fits(io)))
    }

    pub(crate) fn can_dispatch_batch_bit_reverse(&self, io: &WebGpuBuffer<BabyBearElem>) -> bool {
        self.batch_bit_reverse_gpu_enabled.get()
            && (io.size() == 0 || (io.raw_buffer().is_some() && self.storage_binding_fits(io)))
    }

    pub(crate) fn can_dispatch_batch_expand_into_evaluate_ntt(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
    ) -> bool {
        if !self.batch_expand_into_evaluate_ntt_gpu_enabled.get() {
            return false;
        }
        if output.size() == 0 {
            return true;
        }
        let (Some(output_gpu), Some(input_gpu)) = (output.raw_buffer(), input.raw_buffer()) else {
            return false;
        };
        output_gpu != input_gpu
            && self.storage_binding_fits(output)
            && self.storage_binding_fits(input)
    }

    pub(crate) fn can_dispatch_batch_evaluate_any(
        &self,
        out: &WebGpuBuffer<BabyBearExtElem>,
        coeffs: &WebGpuBuffer<BabyBearElem>,
        poly_count: usize,
        which: &WebGpuBuffer<u32>,
        xs: &WebGpuBuffer<BabyBearExtElem>,
    ) -> bool {
        if which.size() == 0 {
            return true;
        }
        if poly_count == 0 || coeffs.size() % poly_count != 0 {
            return false;
        }
        out.raw_buffer().is_some()
            && coeffs.raw_buffer().is_some()
            && which.raw_buffer().is_some()
            && xs.raw_buffer().is_some()
            && self.storage_binding_fits(out)
            && self.storage_binding_fits(coeffs)
            && self.storage_binding_fits(which)
            && self.storage_binding_fits(xs)
    }

    pub(crate) fn can_dispatch_batch_evaluate_any_chunked(
        &self,
        out: &WebGpuBuffer<BabyBearExtElem>,
        coeffs: &WebGpuBuffer<BabyBearElem>,
        poly_count: usize,
        which: &WebGpuBuffer<u32>,
        xs: &WebGpuBuffer<BabyBearExtElem>,
    ) -> bool {
        if which.size() == 0 {
            return true;
        }
        if poly_count == 0 || coeffs.size() % poly_count != 0 {
            return false;
        }
        let deg = coeffs.size() / poly_count;
        let coeff_slice_bytes = byte_len_for::<BabyBearElem>(deg);
        out.size() == which.size()
            && xs.size() == which.size()
            && out.raw_buffer().is_some()
            && coeffs.raw_buffer().is_some()
            && xs.raw_buffer().is_some()
            && coeff_slice_bytes != 0
            && coeff_slice_bytes <= self.max_storage_binding_bytes()
            && coeff_slice_bytes <= MAX_EXACT_JS_INTEGER
            && coeffs.byte_offset() % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT == 0
            && coeff_slice_bytes % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT == 0
            && self.storage_binding_fits(out)
            && self.storage_binding_fits(xs)
    }

    pub(crate) fn can_dispatch_eltwise_sum_extelem(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearExtElem>,
    ) -> bool {
        if output.size() == 0 {
            return true;
        }
        output.size() % BabyBearExtElem::EXT_SIZE == 0
            && output.raw_buffer().is_some()
            && input.raw_buffer().is_some()
            && self.storage_binding_fits(output)
            && self.storage_binding_fits(input)
    }

    pub(crate) fn can_dispatch_fri_fold(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
    ) -> bool {
        let count = output.size() / BabyBearExtElem::EXT_SIZE;
        count == 0
            || (output.raw_buffer().is_some()
                && input.raw_buffer().is_some()
                && self.storage_binding_fits(output)
                && self.storage_binding_fits(input))
    }

    pub(crate) fn can_dispatch_hash_fold(
        &self,
        io: &WebGpuBuffer<Digest>,
        output_size: usize,
    ) -> bool {
        if !self.hash_fold_gpu_enabled.get() {
            return false;
        }
        if output_size == 0 {
            return true;
        }
        // Mirror `dispatch_poseidon2_hash_fold`'s
        // `round_constants` / `m_int_diag` checks (this file, near line 9339).
        let Some(hash) = self.poseidon2.as_ref() else {
            return false;
        };
        io.raw_buffer().is_some()
            && hash.round_constants.raw_buffer().is_some()
            && hash.m_int_diag.raw_buffer().is_some()
            && self.storage_binding_fits(io)
    }

    pub(crate) fn can_dispatch_hash_rows(
        &self,
        output: &WebGpuBuffer<Digest>,
        matrix: &WebGpuBuffer<BabyBearElem>,
        row_size: usize,
    ) -> bool {
        if !self.hash_rows_gpu_enabled.get() {
            return false;
        }
        if row_size == 0 {
            return true;
        }
        // `dispatch_poseidon2_hash_rows` (this file, near
        // line 9397) requires both `round_constants` and `m_int_diag` GPU
        // buffers to be materialized. If `can_dispatch_*` is more permissive
        // than the actual dispatch, the `_async` wrapper skips the input sync
        // and `finish_hal_op` invokes `cpu_mirror` against stale CPU shadows,
        // producing all-zero hash outputs — see `01.5-root-cause.md` D11.
        let Some(hash) = self.poseidon2.as_ref() else {
            return false;
        };
        output.raw_buffer().is_some()
            && matrix.raw_buffer().is_some()
            && hash.round_constants.raw_buffer().is_some()
            && hash.m_int_diag.raw_buffer().is_some()
            && self.storage_binding_fits(output)
            && self.storage_binding_fits(matrix)
    }

    pub(crate) fn can_dispatch_gather_sample(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src: &WebGpuBuffer<BabyBearElem>,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> bool {
        if size == 0 {
            return true;
        }
        let (Some(dst_gpu), Some(src_gpu)) = (dst.raw_buffer(), src.raw_buffer()) else {
            return false;
        };
        size <= dst.size()
            && gather_region_in_bounds(src.size(), idx, size, stride)
            && dst_gpu != src_gpu
            && self.storage_binding_fits(dst)
            && self.storage_binding_fits(src)
    }

    /// Async-safe variant of [`Hal::batch_interpolate_ntt`] for GPU-authoritative
    /// proof stages. If the WebGPU dispatch would fall back to CPU, make the
    /// CPU shadow current before running the synchronous HAL method.
    pub(crate) async fn batch_interpolate_ntt_async(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        count: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative() || !self.can_dispatch_batch_interpolate_ntt(io) {
            io.sync_gpu_to_cpu(self).await?;
        }
        self.batch_interpolate_ntt(io, count);
        Ok(())
    }

    /// Copy-preserving inverse NTT used by WebGPU `make_coeffs`: reads the
    /// witness from `input` and writes the first inverse-NTT stage directly to
    /// `output`, avoiding a standalone device-to-device copy into coeffs.
    pub(crate) async fn batch_interpolate_ntt_from_async(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
        count: usize,
    ) -> Result<bool> {
        if !self.gpu_authoritative() || !self.can_dispatch_batch_interpolate_ntt_from(output, input)
        {
            return Ok(false);
        }
        self.dispatch_batch_interpolate_ntt_from(output, input, count)
    }

    /// Async-safe variant of [`Hal::zk_shift`].
    pub(crate) async fn zk_shift_async(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        count: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative() || !self.can_dispatch_zk_shift(io) {
            io.sync_gpu_to_cpu(self).await?;
        }
        self.zk_shift(io, count);
        Ok(())
    }

    /// Async-safe variant of [`Hal::batch_bit_reverse`].
    pub(crate) async fn batch_bit_reverse_async(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        count: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative() || !self.can_dispatch_batch_bit_reverse(io) {
            io.sync_gpu_to_cpu(self).await?;
        }
        self.batch_bit_reverse(io, count);
        Ok(())
    }

    /// Async-safe variant of [`Hal::batch_expand_into_evaluate_ntt`].
    pub(crate) async fn batch_expand_into_evaluate_ntt_async(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
        count: usize,
        expand_bits: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative()
            || !self.can_dispatch_batch_expand_into_evaluate_ntt(output, input)
        {
            input.sync_gpu_to_cpu(self).await?;
        }
        self.batch_expand_into_evaluate_ntt(output, input, count, expand_bits);
        Ok(())
    }

    /// Async-safe variant of [`Hal::batch_evaluate_any`].
    pub(crate) async fn batch_evaluate_any_async(
        &self,
        coeffs: &WebGpuBuffer<BabyBearElem>,
        poly_count: usize,
        which: &WebGpuBuffer<u32>,
        xs: &WebGpuBuffer<BabyBearExtElem>,
        out: &WebGpuBuffer<BabyBearExtElem>,
    ) -> Result<()> {
        // Prefer chunked when eval_count is VERY small
        // AND deg is large. The normal chunked path collapses to
        // one 2D partial dispatch when buffers fit full storage
        // bindings, but the oversized-buffer fallback is still
        // per-eval-sequential. The win comes from parallelizing the
        // Horner reduction WITHIN each (x, which) pair into
        // `chunk_count` chunks. So chunked is only a net win when:
        //   - eval_count is small enough that sequential dispatches
        //     don't dominate (empirically ≤ 64 on this machine), AND
        //   - deg is large enough that the Horner per non-chunked
        //     thread leaves the GPU underutilized (deg > 8192 means
        //     each thread does 8K+ iterations, with chunked it
        //     parallelizes into 8 chunks of 1024 each).
        //
        // Hits: recursion lift groups 0/1 (16, 23 evals, deg=1048576).
        // Misses: rv32im finalize all groups (evals ≥ 119) and
        // recursion group 2 (604 evals). Iter 3 used a broader
        // heuristic and regressed rv32im finalize from 30 ms to
        // 712 ms because it forced chunked on the high-eval-count
        // case.
        let prefer_chunked =
            if self.gpu_authoritative() && poly_count > 0 && which.size() > 0 && which.size() <= 64
            {
                let deg = coeffs.size() / poly_count.max(1);
                deg > 8192
            } else {
                false
            };

        if !prefer_chunked
            && self.gpu_authoritative()
            && self.can_dispatch_batch_evaluate_any(out, coeffs, poly_count, which, xs)
        {
            self.batch_evaluate_any(coeffs, poly_count, which, xs, out);
            return Ok(());
        }

        if self.gpu_authoritative()
            && self.can_dispatch_batch_evaluate_any_chunked(out, coeffs, poly_count, which, xs)
            && self
                .batch_evaluate_any_chunked_async(coeffs, poly_count, which, xs, out)
                .await?
        {
            return Ok(());
        }

        if self.gpu_authoritative()
            && self.can_dispatch_batch_evaluate_any(out, coeffs, poly_count, which, xs)
        {
            self.batch_evaluate_any(coeffs, poly_count, which, xs, out);
            return Ok(());
        }

        coeffs.sync_gpu_to_cpu(self).await?;
        which.sync_gpu_to_cpu(self).await?;
        xs.sync_gpu_to_cpu(self).await?;
        self.batch_evaluate_any(coeffs, poly_count, which, xs, out);
        Ok(())
    }

    /// Test hook for the chunked `batch_evaluate_any` path used by recursion
    /// lift groups with small eval counts and very large degrees.
    #[doc(hidden)]
    pub async fn debug_batch_evaluate_any_chunked(
        &self,
        coeffs: &WebGpuBuffer<BabyBearElem>,
        poly_count: usize,
        which: &WebGpuBuffer<u32>,
        xs: &WebGpuBuffer<BabyBearExtElem>,
        out: &WebGpuBuffer<BabyBearExtElem>,
    ) -> Result<bool> {
        self.batch_evaluate_any_chunked_async(coeffs, poly_count, which, xs, out)
            .await
    }

    pub(crate) async fn batch_evaluate_any_chunked_async(
        &self,
        coeffs: &WebGpuBuffer<BabyBearElem>,
        poly_count: usize,
        which: &WebGpuBuffer<u32>,
        xs: &WebGpuBuffer<BabyBearExtElem>,
        out: &WebGpuBuffer<BabyBearExtElem>,
    ) -> Result<bool> {
        let eval_count = which.size();
        if eval_count == 0 {
            return Ok(true);
        }
        if poly_count == 0 || coeffs.size() % poly_count != 0 {
            return Ok(false);
        }
        let deg = coeffs.size() / poly_count;
        let chunk_size = WEBGPU_BATCH_EVALUATE_CHUNK_SIZE.min(deg).max(1);
        let chunk_count = deg.div_ceil(chunk_size);
        let coeff_slice_bytes = byte_len_for::<BabyBearElem>(deg);
        if coeff_slice_bytes == 0
            || coeff_slice_bytes > self.max_storage_binding_bytes()
            || coeff_slice_bytes > MAX_EXACT_JS_INTEGER
            || coeffs.byte_offset() % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0
            || coeff_slice_bytes % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0
        {
            return Ok(false);
        }

        let (Some(coeffs_gpu), Some(xs_gpu), Some(out_gpu)) =
            (coeffs.raw_buffer(), xs.raw_buffer(), out.raw_buffer())
        else {
            return Ok(false);
        };
        if !self.storage_binding_fits(xs) || !self.storage_binding_fits(out) {
            return Ok(false);
        }

        let partial_count = eval_count
            .checked_mul(chunk_count)
            .ok_or_else(|| anyhow!("WebGPU batch_evaluate_any partial count overflow"))?;
        let partials = self.alloc_extelem("batch_evaluate_partials", partial_count);
        if !self.storage_binding_fits(&partials) {
            return Ok(false);
        }
        let Some(partials_gpu) = partials.raw_buffer() else {
            return Ok(false);
        };

        coeffs.sync_cpu_to_gpu(self)?;
        xs.sync_cpu_to_gpu(self)?;

        let xs_base = xs
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU batch_evaluate_any xs offset exceeds u32"))?;
        let partials_base = partials
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU batch_evaluate_any partials offset exceeds u32"))?;
        let output_base = out
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU batch_evaluate_any output offset exceeds u32"))?;

        if self.storage_binding_fits(coeffs) && self.storage_binding_fits(which) {
            let Some(which_gpu) = which.raw_buffer() else {
                return Ok(false);
            };
            which.sync_cpu_to_gpu(self)?;
            let layout = self.create_bind_group_layout(
                "webgpu_batch_evaluate_any_partial_2d_layout",
                &[
                    WebGpuBindingLayout::storage(0, 0),
                    WebGpuBindingLayout::read_only_storage(1, 0),
                    WebGpuBindingLayout::read_only_storage(2, 0),
                    WebGpuBindingLayout::read_only_storage(3, 0),
                    WebGpuBindingLayout::uniform(4, 48),
                ],
            )?;
            let kernel = self.create_compute_kernel(
                "webgpu_batch_evaluate_any_partial_2d",
                BATCH_EVALUATE_ANY_PARTIAL_2D_WGSL,
                "main",
                &[layout.clone()],
            )?;
            let params = [
                u32::try_from(deg).expect("WebGPU batch_evaluate_any degree exceeds u32"),
                u32::try_from(chunk_count)
                    .expect("WebGPU batch_evaluate_any chunk count exceeds u32"),
                u32::try_from(chunk_size)
                    .expect("WebGPU batch_evaluate_any chunk size exceeds u32"),
                u32::try_from(eval_count)
                    .expect("WebGPU batch_evaluate_any eval count exceeds u32"),
                partials_base,
                u32::try_from(coeffs.elem_offset)
                    .expect("WebGPU batch_evaluate_any coeffs offset exceeds u32"),
                u32::try_from(which.elem_offset)
                    .expect("WebGPU batch_evaluate_any which offset exceeds u32"),
                xs_base,
                u32::try_from(deg).expect("WebGPU batch_evaluate_any degree exceeds u32"),
                0,
                0,
                0,
            ];
            let params = self.create_uniform_buffer(
                "webgpu_batch_evaluate_any_partial_2d_params",
                bytemuck::cast_slice(&params),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_batch_evaluate_any_partial_2d_bind_group",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, partials_gpu),
                    WebGpuBufferBinding::new(1, coeffs_gpu),
                    WebGpuBufferBinding::new(2, which_gpu),
                    WebGpuBufferBinding::new(3, xs_gpu),
                    WebGpuBufferBinding {
                        binding: 4,
                        buffer: &params,
                        offset: 0,
                        size: Some(48),
                    },
                ],
            )?;
            self.dispatch_compute(
                &kernel,
                &bind_group,
                u32::try_from(chunk_count)
                    .expect("WebGPU batch_evaluate_any chunk count exceeds u32")
                    .div_ceil(WEBGPU_WORKGROUP_SIZE),
                u32::try_from(eval_count)
                    .expect("WebGPU batch_evaluate_any eval count exceeds u32"),
                1,
            );
        } else {
            which.sync_gpu_to_cpu(self).await?;
            let layout = self.create_bind_group_layout(
                "webgpu_batch_evaluate_any_partial_layout",
                &[
                    WebGpuBindingLayout::storage(0, 0),
                    WebGpuBindingLayout::read_only_storage(1, 0),
                    WebGpuBindingLayout::read_only_storage(2, 0),
                    WebGpuBindingLayout::uniform(3, 32),
                ],
            )?;
            let kernel = self.create_compute_kernel(
                "webgpu_batch_evaluate_any_partial",
                BATCH_EVALUATE_ANY_SLICE_PARTIAL_WGSL,
                "main",
                &[layout.clone()],
            )?;

            let which_values = which.to_vec();
            for (eval_idx, poly_id) in which_values.iter().copied().enumerate() {
                let poly_id = poly_id as usize;
                if poly_id >= poly_count {
                    return Err(anyhow!(
                        "WebGPU batch_evaluate_any poly id {poly_id} exceeds poly count {poly_count}"
                    ));
                }
                let coeff_offset = coeffs
                    .byte_offset()
                    .checked_add(
                        u64::try_from(poly_id)
                            .ok()
                            .and_then(|id| id.checked_mul(coeff_slice_bytes))
                            .ok_or_else(|| {
                                anyhow!("WebGPU batch_evaluate_any coeff slice overflow")
                            })?,
                    )
                    .ok_or_else(|| anyhow!("WebGPU batch_evaluate_any coeff offset overflow"))?;
                let eval_partials_base = partials_base
                    .checked_add(
                        eval_idx
                            .checked_mul(chunk_count)
                            .and_then(|offset| offset.checked_mul(BabyBearExtElem::EXT_SIZE))
                            .and_then(|offset| u32::try_from(offset).ok())
                            .ok_or_else(|| {
                                anyhow!("WebGPU batch_evaluate_any partial offset overflow")
                            })?,
                    )
                    .ok_or_else(|| anyhow!("WebGPU batch_evaluate_any partial base overflow"))?;
                let params = [
                    u32::try_from(deg).expect("WebGPU batch_evaluate_any degree exceeds u32"),
                    u32::try_from(chunk_count)
                        .expect("WebGPU batch_evaluate_any chunk count exceeds u32"),
                    eval_partials_base,
                    xs_base,
                    u32::try_from(eval_idx)
                        .expect("WebGPU batch_evaluate_any eval index exceeds u32"),
                    u32::try_from(chunk_size)
                        .expect("WebGPU batch_evaluate_any chunk size exceeds u32"),
                    0,
                    0,
                ];
                let params = self.create_uniform_buffer(
                    "webgpu_batch_evaluate_any_partial_params",
                    bytemuck::cast_slice(&params),
                )?;
                let bind_group = self.create_bind_group(
                    "webgpu_batch_evaluate_any_partial_bind_group",
                    &layout,
                    &[
                        WebGpuBufferBinding::new(0, partials_gpu),
                        WebGpuBufferBinding {
                            binding: 1,
                            buffer: coeffs_gpu,
                            offset: coeff_offset,
                            size: Some(coeff_slice_bytes),
                        },
                        WebGpuBufferBinding::new(2, xs_gpu),
                        WebGpuBufferBinding {
                            binding: 3,
                            buffer: &params,
                            offset: 0,
                            size: Some(32),
                        },
                    ],
                )?;
                self.dispatch_compute(
                    &kernel,
                    &bind_group,
                    u32::try_from(chunk_count)
                        .expect("WebGPU batch_evaluate_any chunk count exceeds u32")
                        .div_ceil(WEBGPU_WORKGROUP_SIZE),
                    1,
                    1,
                );
            }
        }

        partials.mark_gpu_dirty();

        let reduce_layout = self.create_bind_group_layout(
            "webgpu_batch_evaluate_any_reduce_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let reduce_kernel = self.create_compute_kernel(
            "webgpu_batch_evaluate_any_reduce",
            BATCH_EVALUATE_ANY_PARTIAL_REDUCE_WGSL,
            "main",
            &[reduce_layout.clone()],
        )?;
        let reduce_params = [
            u32::try_from(eval_count).expect("WebGPU batch_evaluate_any eval count exceeds u32"),
            u32::try_from(chunk_count).expect("WebGPU batch_evaluate_any chunk count exceeds u32"),
            partials_base,
            output_base,
            0,
            0,
            0,
            0,
        ];
        let reduce_params = self.create_uniform_buffer(
            "webgpu_batch_evaluate_any_reduce_params",
            bytemuck::cast_slice(&reduce_params),
        )?;
        let reduce_bind_group = self.create_bind_group(
            "webgpu_batch_evaluate_any_reduce_bind_group",
            &reduce_layout,
            &[
                WebGpuBufferBinding::new(0, out_gpu),
                WebGpuBufferBinding::new(1, partials_gpu),
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: &reduce_params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        self.dispatch_compute(
            &reduce_kernel,
            &reduce_bind_group,
            u32::try_from(eval_count)
                .expect("WebGPU batch_evaluate_any eval count exceeds u32")
                .div_ceil(WEBGPU_WORKGROUP_SIZE),
            1,
            1,
        );
        out.mark_gpu_dirty();
        self.record_gpu_result_authoritative("batch_evaluate_any", true);
        Ok(true)
    }
}
