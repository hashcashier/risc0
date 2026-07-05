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

//! GPU resource wrappers: kernels, bind-group layouts, and the
//! shadowed `WebGpuBuffer`.

use super::*;
#[allow(unused_imports)]
use super::{device::*, diagnostics::*, dispatch::*, eval_check::*, kernels_wgsl::*, ops::*};

#[derive(Clone)]
pub(crate) struct WebGpuPoseidon2Hash {
    pub(crate) round_constants: WebGpuBuffer<BabyBearElem>,
    pub(crate) m_int_diag: WebGpuBuffer<BabyBearElem>,
    pub(crate) fold_layout: web_sys::GpuBindGroupLayout,
    pub(crate) fold_kernel: WebGpuKernel,
    pub(crate) fold_chain_layout: web_sys::GpuBindGroupLayout,
    pub(crate) fold_chain_kernel: WebGpuKernel,
    pub(crate) rows_layout: web_sys::GpuBindGroupLayout,
    pub(crate) rows_kernel: WebGpuKernel,
}

impl WebGpuPoseidon2Hash {
    pub(crate) fn new(hal: &WebGpuHal) -> Result<Self> {
        let round_constants = hal.copy_from_elem(
            "webgpu_poseidon2_round_constants",
            poseidon2::ROUND_CONSTANTS,
        );
        let m_int_diag =
            hal.copy_from_elem("webgpu_poseidon2_m_int_diag", poseidon2::M_INT_DIAG_HZN);

        let fold_layout = hal.create_bind_group_layout(
            "webgpu_poseidon2_fold_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let fold_kernel = hal.create_compute_kernel(
            "webgpu_poseidon2_fold",
            POSEIDON2_WGSL,
            "poseidon2_fold",
            &[fold_layout.clone()],
        )?;
        let fold_chain_layout = hal.create_bind_group_layout(
            "webgpu_poseidon2_fold_chain_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::uniform_dynamic(4, 32),
            ],
        )?;
        let fold_chain_kernel = hal.create_compute_kernel(
            "webgpu_poseidon2_fold_chain",
            POSEIDON2_WGSL,
            "poseidon2_fold",
            &[fold_chain_layout.clone()],
        )?;

        let rows_layout = hal.create_bind_group_layout(
            "webgpu_poseidon2_rows_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let rows_kernel = hal.create_compute_kernel(
            "webgpu_poseidon2_rows",
            POSEIDON2_WGSL,
            "poseidon2_rows",
            &[rows_layout.clone()],
        )?;

        Ok(Self {
            round_constants,
            m_int_diag,
            fold_layout,
            fold_kernel,
            fold_chain_layout,
            fold_chain_kernel,
            rows_layout,
            rows_kernel,
        })
    }
}

/// A compiled WebGPU compute pipeline.
#[derive(Clone)]
pub struct WebGpuKernel {
    pub(crate) pipeline: web_sys::GpuComputePipeline,
}

impl WebGpuKernel {
    /// Return the underlying browser `GPUComputePipeline`.
    pub fn pipeline(&self) -> &web_sys::GpuComputePipeline {
        &self.pipeline
    }
}

/// A WebGPU compute pipeline compile whose browser promise has already
/// been created. Constructing this starts `createComputePipelineAsync`
/// immediately; callers can await it later to overlap Tint compilation
/// with independent wasm work.
#[derive(Clone)]
pub struct WebGpuStartedComputeKernel {
    pub(crate) label: &'static str,
    pub(crate) cache_key: Option<u64>,
    pub(crate) cached: Option<WebGpuKernel>,
    pub(crate) promise: Option<js_sys::Promise>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct EvalCheckInterpreterPipelineKey {
    pub(crate) base_field_fp: bool,
    pub(crate) private_parallel: bool,
    pub(crate) fp_slots: usize,
    pub(crate) ext_slots: usize,
    pub(crate) mix_slots: usize,
    pub(crate) workgroup_size: u32,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ElemTransposeZeroPadCacheKey {
    pub(crate) control_id: Digest,
    pub(crate) rows: usize,
    pub(crate) cols: usize,
    pub(crate) total_rows: usize,
}

#[derive(Clone)]
pub(crate) struct EvalCheckInterpreterPipeline {
    pub(crate) layout: web_sys::GpuBindGroupLayout,
    pub(crate) kernel: WebGpuKernel,
}

/// A compiled multi-stage staged-WGSL `eval_check` pipeline
/// for a specific `(PolyExtStepDef, base_field_fp)` pair. The bind-group
/// layout has 12 active bindings (skipping the interpreter's `instrs`
/// at binding 6): the interpreter's 8 (check, group0..2, global0..1,
/// mix_pows, params) plus 4 new for multi-stage scratch (binding 9 =
/// `staged_scratch_params` UBO, 10/11/12 = `fp_scratch` /
/// `mix_tot_scratch` / `mix_mul_scratch` rw storage). All stages of
/// the same DEF share the same bind group; only the bound pipeline
/// changes per stage.
///
/// The four scratch buffers are cached on the pipeline so
/// they're allocated once per DEF (sized to `tile_size * stride * 4 B`,
/// independent of the prove's full domain) and reused across all
/// `eval_check` calls. This bounds total GPU memory by O(unique DEFs ×
/// per-tile scratch) instead of O(eval_check_call_count × per-domain
/// scratch).
#[derive(Clone)]
pub(crate) struct StagedEvalCheckPipeline {
    pub(crate) layout: web_sys::GpuBindGroupLayout,
    pub(crate) stages: Vec<WebGpuKernel>,
    /// `@compute @workgroup_size(N)` baked into every
    /// stage's WGSL by `choose_workgroup_size`. Dispatch shape is
    /// `(tile_size / workgroup_size, num_tiles, 1)` so the total
    /// thread count remains `tile_size * num_tiles` (= domain rounded
    /// up to `tile_size`). All stages of a pipeline share the same
    /// workgroup_size because the codegen picks it from `plan.fp_slots`
    /// / `plan.mix_slots`, which are cross-chunk high-water marks.
    pub(crate) workgroup_size: u32,
    pub(crate) base_field_fp: bool,
    /// Cached `tile_size * fp_stride * 4 B` storage buffer. Sized from the
    /// plan's per-cycle scratch strides (0-stride plans still cache 4-byte
    /// dummies so the validator's binding-count rules don't trip).
    pub(crate) fp_scratch: web_sys::GpuBuffer,
    /// Cached mix scratches (each `tile_size * mix_stride * 4 B`).
    pub(crate) mix_tot_scratch: web_sys::GpuBuffer,
    pub(crate) mix_mul_scratch: web_sys::GpuBuffer,
    /// Cached 16-byte UBO holding pipeline-constant
    /// `{fp_stride, mix_stride, num_stages, tile_size}`. Written once at
    /// pipeline create; every `eval_check` call binds it as-is.
    pub(crate) scratch_params_buf: web_sys::GpuBuffer,
    /// Cached storage buffer for `mix_pows` (sized to the
    /// DEF's `mix_expected * 4 u32`). Each `eval_check` call rewrites
    /// it with the call-specific `poly_mix^exp` values.
    pub(crate) mix_pows_buf: web_sys::GpuBuffer,
    /// Cached 96-byte uniform buffer for the main Params
    /// UBO at binding 8. Each `eval_check` call rewrites it.
    pub(crate) params_buf: web_sys::GpuBuffer,
    /// `mix_expected` for this DEF. Recorded here so the
    /// dispatch knows how many `u32` words to write into `mix_pows_buf`
    /// without recomputing.
    pub(crate) mix_pow_words: usize,
}

/// A single storage or uniform buffer binding in a WebGPU bind group layout.
#[derive(Clone, Copy)]
pub struct WebGpuBindingLayout {
    /// The WGSL binding number.
    pub binding: u32,
    /// The WebGPU buffer binding type.
    pub ty: web_sys::GpuBufferBindingType,
    /// Optional minimum binding size in bytes.
    pub min_binding_size: u64,
    /// When true, the bind group accepts a dynamic byte offset for this
    /// binding via `setBindGroup`. The staged eval_check path uses this to advance
    /// `tile_base` through a single `scratch_params` UBO without
    /// rebinding or resubmitting between tiles.
    pub has_dynamic_offset: bool,
}

impl WebGpuBindingLayout {
    /// Create a read/write storage buffer binding layout.
    pub fn storage(binding: u32, min_binding_size: u64) -> Self {
        Self {
            binding,
            ty: web_sys::GpuBufferBindingType::Storage,
            min_binding_size,
            has_dynamic_offset: false,
        }
    }

    /// Create a read-only storage buffer binding layout.
    pub fn read_only_storage(binding: u32, min_binding_size: u64) -> Self {
        Self {
            binding,
            ty: web_sys::GpuBufferBindingType::ReadOnlyStorage,
            min_binding_size,
            has_dynamic_offset: false,
        }
    }

    /// Create a uniform buffer binding layout.
    pub fn uniform(binding: u32, min_binding_size: u64) -> Self {
        Self {
            binding,
            ty: web_sys::GpuBufferBindingType::Uniform,
            min_binding_size,
            has_dynamic_offset: false,
        }
    }

    /// Create a uniform buffer binding layout with dynamic offset support.
    /// The bind group's `setBindGroup` call must then
    /// supply a `u32` byte offset for this binding.
    pub fn uniform_dynamic(binding: u32, min_binding_size: u64) -> Self {
        Self {
            binding,
            ty: web_sys::GpuBufferBindingType::Uniform,
            min_binding_size,
            has_dynamic_offset: true,
        }
    }
}

/// Hash a (label_ptr, entries-shape) tuple
/// for the bind-group-layout cache. label is a `&'static str` so its
/// pointer is a stable identity. Entry fields hash to a layout-shape
/// fingerprint; identical fingerprints share a layout instance.
pub(crate) fn compute_bind_group_layout_cache_key(
    label: &'static str,
    entries: &[WebGpuBindingLayout],
) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    let mut h = DefaultHasher::new();
    h.write_usize(label.as_ptr() as usize);
    h.write_usize(label.len());
    h.write_usize(entries.len());
    for entry in entries {
        h.write_u32(entry.binding);
        // GpuBufferBindingType has 3 variants on this target; encode
        // explicitly so the hash is stable across rebuilds.
        let ty_byte: u8 = if entry.ty == web_sys::GpuBufferBindingType::Storage {
            0
        } else if entry.ty == web_sys::GpuBufferBindingType::ReadOnlyStorage {
            1
        } else if entry.ty == web_sys::GpuBufferBindingType::Uniform {
            2
        } else {
            255
        };
        h.write_u8(ty_byte);
        h.write_u64(entry.min_binding_size);
        h.write_u8(entry.has_dynamic_offset as u8);
    }
    h.finish()
}

pub(crate) fn compute_compute_pipeline_cache_key(
    label: &'static str,
    wgsl: &str,
    entry_point: &str,
    layout_keys: &[String],
) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    let mut h = DefaultHasher::new();
    h.write_usize(label.as_ptr() as usize);
    h.write_usize(label.len());
    h.write(wgsl.as_bytes());
    h.write(entry_point.as_bytes());
    h.write_usize(layout_keys.len());
    for layout_key in layout_keys {
        h.write(layout_key.as_bytes());
    }
    h.finish()
}

/// A concrete buffer binding used to create a WebGPU bind group.
pub struct WebGpuBufferBinding<'a> {
    /// The WGSL binding number.
    pub binding: u32,
    /// The WebGPU buffer bound at this index.
    pub buffer: &'a web_sys::GpuBuffer,
    /// Byte offset into the buffer.
    pub offset: u64,
    /// Optional byte size for the binding.
    pub size: Option<u64>,
}

impl<'a> WebGpuBufferBinding<'a> {
    /// Bind a full buffer at `binding`.
    pub fn new(binding: u32, buffer: &'a web_sys::GpuBuffer) -> Self {
        Self {
            binding,
            buffer,
            offset: 0,
            size: None,
        }
    }
}

/// One source-buffer group for a combined indexed readback.
pub struct WebGpuIndexedReadbackGroup<'a> {
    pub source: &'a web_sys::GpuBuffer,
    pub base_byte_offset: u64,
    pub elem_size: u64,
    pub indices: &'a [usize],
}

/// Owning wrapper that calls `GpuBuffer.destroy()` when
/// the last Rc reference drops. Without explicit destruction, Chrome WebGPU's
/// per-context VRAM budget is exhausted across multi-segment recursion lifts
/// (D14 surfaced `VK_ERROR_OUT_OF_DEVICE_MEMORY` after 7-8 segments), because
/// JS GC does not promptly reclaim GpuBuffer handles between hot-loop dispatches.
pub(crate) struct WebGpuBufferOwner {
    pub(crate) buffer: web_sys::GpuBuffer,
}

impl Drop for WebGpuBufferOwner {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

/// A browser WebGPU buffer with a CPU shadow for the existing synchronous HAL API.
#[derive(Clone)]
pub struct WebGpuBuffer<T> {
    pub(crate) cpu: CpuBuffer<T>,
    pub(crate) gpu: Option<Rc<WebGpuBufferOwner>>,
    pub(crate) elem_offset: usize,
    /// CPU-side changes that have not been uploaded to the GPU buffer.
    pub(crate) cpu_dirty: Rc<Cell<bool>>,
    /// GPU-side changes that have not been read back into the CPU shadow.
    pub(crate) cpu_stale: Rc<Cell<bool>>,
    /// Conservative marker for browser-zero-initialized backing storage that
    /// has not yet been written with potentially nonzero GPU contents.
    pub(crate) gpu_known_zero: Rc<Cell<bool>>,
    pub(crate) marker: PhantomData<T>,
}

impl<T> WebGpuBuffer<T> {
    pub(crate) fn new(
        cpu: CpuBuffer<T>,
        gpu: Option<Rc<WebGpuBufferOwner>>,
        cpu_dirty: Rc<Cell<bool>>,
        cpu_stale: Rc<Cell<bool>>,
        gpu_known_zero: Rc<Cell<bool>>,
    ) -> Self {
        Self {
            cpu,
            gpu,
            elem_offset: 0,
            cpu_dirty,
            cpu_stale,
            gpu_known_zero,
            marker: PhantomData,
        }
    }

    pub(crate) fn cpu(&self) -> &CpuBuffer<T> {
        &self.cpu
    }

    pub(crate) fn mark_cpu_dirty(&self) {
        self.cpu_dirty.set(true);
        self.cpu_stale.set(false);
    }

    /// Mark the GPU buffer as containing newer data than the CPU shadow.
    pub fn mark_gpu_dirty(&self) {
        self.cpu_dirty.set(false);
        self.cpu_stale.set(true);
        self.gpu_known_zero.set(false);
    }

    pub(crate) fn mark_synced(&self) {
        self.cpu_dirty.set(false);
        self.cpu_stale.set(false);
        self.gpu_known_zero.set(false);
    }

    pub(crate) fn mark_cpu_result(&self, gpu_current: bool) {
        self.cpu_dirty.set(!gpu_current);
        self.cpu_stale.set(false);
        if gpu_current {
            self.gpu_known_zero.set(false);
        }
    }

    pub(crate) fn gpu_known_zero(&self) -> bool {
        self.gpu_known_zero.get()
    }

    pub(crate) fn mark_gpu_may_be_nonzero(&self) {
        self.gpu_known_zero.set(false);
    }

    /// Returns true when synchronous CPU views are current.
    pub fn cpu_is_current(&self) -> bool {
        !self.cpu_stale.get()
    }

    /// Returns true when the browser `GPUBuffer` is current.
    pub fn gpu_is_current(&self) -> bool {
        !self.cpu_dirty.get()
    }

    /// Return the underlying browser `GPUBuffer`, when the allocation is non-empty.
    pub fn raw_buffer(&self) -> Option<&web_sys::GpuBuffer> {
        self.gpu.as_ref().map(|owner| &owner.buffer)
    }

    /// Byte offset of this buffer view into the underlying browser `GPUBuffer`.
    pub fn byte_offset(&self) -> u64 {
        (self.elem_offset * mem::size_of::<T>()) as u64
    }

    /// Upload the CPU shadow to the browser `GPUBuffer` if it has changed.
    pub fn sync_cpu_to_gpu(&self, hal: &WebGpuHal) -> Result<()>
    where
        T: bytemuck::NoUninit + Clone,
    {
        if !self.cpu_dirty.get() {
            return Ok(());
        }

        ensure!(
            !self.cpu_stale.get(),
            "cannot upload stale CPU shadow for WebGPU buffer {}",
            self.cpu.name()
        );

        // Lazy shadows: a never-materialized shadow is logically all
        // `T::default()`. When that default is the zero bit pattern and the
        // browser-zero-initialized GPU allocation is still untouched, the
        // upload is a no-op — skip it without materializing the shadow.
        if let Some(fill) = self.cpu.pending_fill_value() {
            if self.gpu_known_zero.get() && bytemuck::bytes_of(&fill).iter().all(|&byte| byte == 0)
            {
                self.cpu_dirty.set(false);
                return Ok(());
            }
        }

        if let Some(gpu) = self.raw_buffer() {
            let mut upload = Ok(());
            self.cpu.view(|cpu| {
                let elems_per_chunk = (WEBGPU_SAFE_QUEUE_WRITE_BYTES / mem::size_of::<T>()).max(1);
                for (chunk_idx, chunk) in cpu.chunks(elems_per_chunk).enumerate() {
                    let chunk_offset = chunk_idx
                        .checked_mul(elems_per_chunk)
                        .expect("WebGPU upload chunk offset overflow");
                    upload = hal.write_buffer_named(
                        gpu,
                        self.cpu.name(),
                        self.byte_offset() + byte_len_for::<T>(chunk_offset),
                        bytemuck::cast_slice(chunk),
                    );
                    if upload.is_err() {
                        break;
                    }
                }
            });
            upload?;
        }

        self.cpu_dirty.set(false);
        self.gpu_known_zero.set(false);
        Ok(())
    }

    /// Read the browser `GPUBuffer` back into the CPU shadow when the GPU owns
    /// newer contents. This is the async boundary WebGPU needs before any
    /// synchronous transcript, Merkle, or verifier-facing CPU view.
    pub async fn sync_gpu_to_cpu(&self, hal: &WebGpuHal) -> Result<()>
    where
        T: bytemuck::CheckedBitPattern + Clone,
    {
        if !self.cpu_stale.get() {
            return Ok(());
        }

        let Some(gpu) = self.raw_buffer() else {
            ensure!(
                self.cpu.size() == 0,
                "cannot read back missing GPU buffer {}",
                self.cpu.name()
            );
            self.mark_synced();
            return Ok(());
        };

        let byte_len = byte_len_for::<T>(self.cpu.size());
        let bytes = hal
            .read_buffer_range_named(gpu, self.byte_offset(), byte_len, self.cpu.name())
            .await?;
        let values = bytemuck::checked::try_cast_slice::<u8, T>(bytes.as_slice())
            .map_err(|err| anyhow!("invalid WebGPU readback for {}: {err}", self.cpu.name()))?;
        ensure!(
            values.len() == self.cpu.size(),
            "readback size mismatch for WebGPU buffer {}: got {} elems, expected {}",
            self.cpu.name(),
            values.len(),
            self.cpu.size()
        );
        self.cpu.copy_in_from_slice(values);
        self.mark_synced();
        Ok(())
    }

    /// Bitwise GPU->CPU sync
    /// that ALLOWS `Val::INVALID` (0xffffffff) values to flow back into
    /// the CPU shadow without failing the `CheckedBitPattern` validator.
    /// Required for the pre-witgen dispatch path: GPU partially
    /// populates `data_buf` (shadow_init + per-arm chunks), other cells
    /// remain `INVALID`. Standard `sync_gpu_to_cpu` would error on the
    /// INVALID bytes; this variant transmutes raw u32 bytes into the
    /// target type via `repr(transparent)` semantics. Caller takes
    /// responsibility for ensuring T is bitwise-equivalent to u32 (i.e.,
    /// `BabyBearElem` is `#[repr(transparent)] struct(u32)`).
    pub async fn sync_gpu_to_cpu_unchecked(&self, hal: &WebGpuHal) -> Result<()>
    where
        T: Clone,
    {
        if !self.cpu_stale.get() {
            return Ok(());
        }

        let Some(gpu) = self.raw_buffer() else {
            ensure!(
                self.cpu.size() == 0,
                "cannot read back missing GPU buffer {}",
                self.cpu.name()
            );
            self.mark_synced();
            return Ok(());
        };

        let byte_len = byte_len_for::<T>(self.cpu.size());
        let bytes = hal
            .read_buffer_range_named(gpu, self.byte_offset(), byte_len, self.cpu.name())
            .await?;
        // SAFETY: caller asserts T is bitwise-equivalent to a sequence of
        // u32 (`repr(transparent)`). For `BabyBearElem` (T = Val) this
        // holds: it's `#[repr(transparent)] struct Elem(u32)`. We read
        // raw u32s (Pod) and reinterpret as T via unsafe transmute on
        // the slice pointer.
        let u32s: &[u32] = bytemuck::cast_slice(bytes.as_slice());
        ensure!(
            u32s.len() == self.cpu.size(),
            "readback size mismatch for WebGPU buffer {}: got {} elems, expected {}",
            self.cpu.name(),
            u32s.len(),
            self.cpu.size()
        );
        let values: &[T] =
            unsafe { std::slice::from_raw_parts(u32s.as_ptr() as *const T, u32s.len()) };
        self.cpu.copy_in_from_slice(values);
        self.mark_synced();
        Ok(())
    }

    /// Copy selected element ranges from the GPU buffer into the CPU shadow.
    ///
    /// Like `sync_gpu_to_cpu_unchecked`, this accepts bit patterns that are
    /// invalid for `CheckedBitPattern` users such as `BabyBearElem::INVALID`.
    /// The caller is responsible for choosing ranges that cover every GPU-side
    /// difference the CPU shadow needs before subsequent synchronous views.
    pub async fn sync_gpu_ranges_to_cpu_unchecked(
        &self,
        hal: &WebGpuHal,
        ranges: &[(usize, usize)],
        name: &'static str,
    ) -> Result<()>
    where
        T: Clone,
    {
        if ranges.is_empty() {
            self.mark_synced();
            return Ok(());
        }
        ensure!(
            mem::size_of::<T>() == mem::size_of::<u32>(),
            "unchecked partial readback for WebGPU buffer {} requires u32-sized elements",
            self.cpu.name()
        );

        let Some(gpu) = self.raw_buffer() else {
            ensure!(
                self.cpu.size() == 0,
                "cannot read back missing GPU buffer {}",
                self.cpu.name()
            );
            self.mark_synced();
            return Ok(());
        };

        let mut byte_ranges = Vec::with_capacity(ranges.len());
        let mut total_elems = 0usize;
        for &(elem_offset, elem_len) in ranges {
            let elem_end = elem_offset
                .checked_add(elem_len)
                .ok_or_else(|| anyhow!("WebGPU partial readback range overflow"))?;
            ensure!(
                elem_end <= self.cpu.size(),
                "partial readback range for WebGPU buffer {} exceeds size: end={}, size={}",
                self.cpu.name(),
                elem_end,
                self.cpu.size()
            );
            if elem_len == 0 {
                continue;
            }
            byte_ranges.push((
                gpu,
                self.byte_offset() + byte_len_for::<T>(elem_offset),
                byte_len_for::<T>(elem_len),
            ));
            total_elems = total_elems
                .checked_add(elem_len)
                .ok_or_else(|| anyhow!("WebGPU partial readback element count overflow"))?;
        }

        if byte_ranges.is_empty() {
            self.mark_synced();
            return Ok(());
        }

        let bytes = hal
            .read_buffer_ranges_named(byte_ranges.as_slice(), name)
            .await?;
        let u32s: &[u32] = bytemuck::cast_slice(bytes.as_slice());
        ensure!(
            u32s.len() == total_elems,
            "partial readback size mismatch for WebGPU buffer {}: got {} elems, expected {}",
            self.cpu.name(),
            u32s.len(),
            total_elems
        );
        let values: &[T] =
            unsafe { std::slice::from_raw_parts(u32s.as_ptr() as *const T, u32s.len()) };
        let mut cursor = 0usize;
        self.cpu.view_mut(|cpu| {
            for &(elem_offset, elem_len) in ranges {
                if elem_len == 0 {
                    continue;
                }
                let next = cursor + elem_len;
                cpu[elem_offset..elem_offset + elem_len].clone_from_slice(&values[cursor..next]);
                cursor = next;
            }
        });
        self.mark_synced();
        Ok(())
    }

    /// Pack a column prefix for selected rows on the GPU, then copy the packed
    /// result into the corresponding CPU-shadow cells.
    ///
    /// This is for partially authoritative witness paths where the GPU has
    /// written a sparse row set and the subsequent synchronous CPU path only
    /// needs those row/column cells repaired before `view_mut`.
    pub async fn sync_gpu_column_prefix_rows_to_cpu_unchecked(
        &self,
        hal: &WebGpuHal,
        total_rows: usize,
        column_count: usize,
        row_indices: &[u32],
        name: &'static str,
    ) -> Result<()>
    where
        T: Clone,
    {
        if column_count == 0 || row_indices.is_empty() {
            self.mark_synced();
            return Ok(());
        }
        ensure!(
            mem::size_of::<T>() == mem::size_of::<u32>(),
            "unchecked sparse readback for WebGPU buffer {} requires u32-sized elements",
            self.cpu.name()
        );
        ensure!(
            total_rows > 0,
            "sparse readback for WebGPU buffer {} requires nonzero total_rows",
            self.cpu.name()
        );
        let covered_elems = total_rows
            .checked_mul(column_count)
            .ok_or_else(|| anyhow!("WebGPU sparse readback covered range overflow"))?;
        ensure!(
            covered_elems <= self.cpu.size(),
            "sparse readback column prefix for WebGPU buffer {} exceeds size: covered={}, size={}",
            self.cpu.name(),
            covered_elems,
            self.cpu.size()
        );
        for &row in row_indices {
            ensure!(
                (row as usize) < total_rows,
                "sparse readback row for WebGPU buffer {} exceeds total_rows: row={}, total_rows={}",
                self.cpu.name(),
                row,
                total_rows
            );
        }

        let Some(gpu) = self.raw_buffer() else {
            ensure!(
                self.cpu.size() == 0,
                "cannot read back missing GPU buffer {}",
                self.cpu.name()
            );
            self.mark_synced();
            return Ok(());
        };

        let row_count = row_indices.len();
        let packed_elems = column_count
            .checked_mul(row_count)
            .ok_or_else(|| anyhow!("WebGPU sparse readback packed element count overflow"))?;
        let packed_byte_len = byte_len_for::<T>(packed_elems);

        let row_bytes = bytemuck::cast_slice(row_indices);
        let row_buf = hal.create_storage_buffer(
            "webgpu_sparse_column_prefix_rows",
            u64::try_from(row_bytes.len())
                .map_err(|_| anyhow!("WebGPU sparse row-index bytes exceed u64"))?,
        )?;
        hal.write_buffer_named(&row_buf, "webgpu_sparse_column_prefix_rows", 0, row_bytes)?;
        let packed_buf =
            hal.create_storage_buffer("webgpu_sparse_column_prefix_packed", packed_byte_len)?;
        let params = [
            u32::try_from(total_rows).expect("WebGPU sparse readback total_rows exceeds u32"),
            u32::try_from(column_count).expect("WebGPU sparse readback column_count exceeds u32"),
            u32::try_from(row_count).expect("WebGPU sparse readback row_count exceeds u32"),
            0,
        ];
        let params_buf = hal.create_uniform_buffer(
            "webgpu_sparse_column_prefix_params",
            bytemuck::cast_slice(&params),
        )?;
        let layout = hal.create_bind_group_layout(
            "webgpu_sparse_column_prefix_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::uniform(3, 16),
            ],
        )?;
        let kernel = hal.create_compute_kernel(
            "webgpu_sparse_column_prefix",
            PACK_COLUMN_PREFIX_ROWS_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = hal.create_bind_group(
            "webgpu_sparse_column_prefix_bind_group",
            &layout,
            &[
                WebGpuBufferBinding {
                    binding: 0,
                    buffer: gpu,
                    offset: self.byte_offset(),
                    size: Some(byte_len_for::<T>(self.cpu.size())),
                },
                WebGpuBufferBinding::new(1, &row_buf),
                WebGpuBufferBinding::new(2, &packed_buf),
                WebGpuBufferBinding::new(3, &params_buf),
            ],
        )?;
        let workgroups = u32::try_from(packed_elems)
            .expect("WebGPU sparse readback packed element count exceeds u32")
            .div_ceil(WEBGPU_WORKGROUP_SIZE);
        hal.dispatch_compute_1d(&kernel, &bind_group, workgroups);

        let bytes = hal
            .read_buffer_range_named(&packed_buf, 0, packed_byte_len, name)
            .await?;
        let u32s: &[u32] = bytemuck::cast_slice(bytes.as_slice());
        ensure!(
            u32s.len() == packed_elems,
            "sparse readback size mismatch for WebGPU buffer {}: got {} elems, expected {}",
            self.cpu.name(),
            u32s.len(),
            packed_elems
        );
        let values: &[T] =
            unsafe { std::slice::from_raw_parts(u32s.as_ptr() as *const T, u32s.len()) };
        self.cpu.view_mut(|cpu| {
            for col in 0..column_count {
                let packed_col_offset = col * row_count;
                let cpu_col_offset = col * total_rows;
                for (row_pos, &row) in row_indices.iter().enumerate() {
                    cpu[cpu_col_offset + row as usize] =
                        values[packed_col_offset + row_pos].clone();
                }
            }
        });
        self.mark_synced();
        Ok(())
    }

    /// Pack multiple sparse column-prefix row groups into one GPU buffer, then
    /// copy the packed result back into the matching CPU-shadow cells.
    ///
    /// This keeps groups with different prefix widths separate logically while
    /// avoiding one map/readback per group.
    pub async fn sync_gpu_column_prefix_row_groups_to_cpu_unchecked(
        &self,
        hal: &WebGpuHal,
        total_rows: usize,
        groups: &[(usize, &[u32])],
        name: &'static str,
    ) -> Result<()>
    where
        T: Clone,
    {
        ensure!(
            mem::size_of::<T>() == mem::size_of::<u32>(),
            "unchecked sparse grouped readback for WebGPU buffer {} requires u32-sized elements",
            self.cpu.name()
        );
        ensure!(
            total_rows > 0,
            "sparse grouped readback for WebGPU buffer {} requires nonzero total_rows",
            self.cpu.name()
        );

        let mut packed_elems = 0usize;
        let mut row_indices = Vec::new();
        let mut group_specs = Vec::<u32>::new();
        for &(column_count, rows) in groups {
            if column_count == 0 || rows.is_empty() {
                continue;
            }
            let covered_elems = total_rows
                .checked_mul(column_count)
                .ok_or_else(|| anyhow!("WebGPU sparse grouped readback covered range overflow"))?;
            ensure!(
                covered_elems <= self.cpu.size(),
                "sparse grouped readback column prefix for WebGPU buffer {} exceeds size: covered={}, size={}",
                self.cpu.name(),
                covered_elems,
                self.cpu.size()
            );
            for &row in rows {
                ensure!(
                    (row as usize) < total_rows,
                    "sparse grouped readback row for WebGPU buffer {} exceeds total_rows: row={}, total_rows={}",
                    self.cpu.name(),
                    row,
                    total_rows
                );
            }

            let row_base = row_indices.len();
            let row_count = rows.len();
            let group_packed = column_count
                .checked_mul(row_count)
                .ok_or_else(|| anyhow!("WebGPU sparse grouped readback packed group overflow"))?;
            group_specs.extend_from_slice(&[
                u32::try_from(row_base)
                    .expect("WebGPU sparse grouped readback row_base exceeds u32"),
                u32::try_from(row_count)
                    .expect("WebGPU sparse grouped readback row_count exceeds u32"),
                u32::try_from(column_count)
                    .expect("WebGPU sparse grouped readback column_count exceeds u32"),
                u32::try_from(packed_elems)
                    .expect("WebGPU sparse grouped readback packed_base exceeds u32"),
            ]);
            row_indices.extend_from_slice(rows);
            packed_elems = packed_elems
                .checked_add(group_packed)
                .ok_or_else(|| anyhow!("WebGPU sparse grouped readback packed total overflow"))?;
        }

        if packed_elems == 0 {
            self.mark_synced();
            return Ok(());
        }

        let Some(gpu) = self.raw_buffer() else {
            ensure!(
                self.cpu.size() == 0,
                "cannot read back missing GPU buffer {}",
                self.cpu.name()
            );
            self.mark_synced();
            return Ok(());
        };

        let packed_byte_len = byte_len_for::<T>(packed_elems);
        let row_bytes = bytemuck::cast_slice(row_indices.as_slice());
        let row_buf = hal.create_storage_buffer(
            "webgpu_sparse_column_prefix_group_rows",
            u64::try_from(row_bytes.len())
                .map_err(|_| anyhow!("WebGPU sparse grouped row-index bytes exceed u64"))?,
        )?;
        hal.write_buffer_named(
            &row_buf,
            "webgpu_sparse_column_prefix_group_rows",
            0,
            row_bytes,
        )?;
        let packed_buf =
            hal.create_storage_buffer("webgpu_sparse_column_prefix_group_packed", packed_byte_len)?;
        let group_bytes = bytemuck::cast_slice(group_specs.as_slice());
        let group_buf = hal.create_storage_buffer(
            "webgpu_sparse_column_prefix_group_specs",
            u64::try_from(group_bytes.len())
                .map_err(|_| anyhow!("WebGPU sparse grouped spec bytes exceed u64"))?,
        )?;
        hal.write_buffer_named(
            &group_buf,
            "webgpu_sparse_column_prefix_group_specs",
            0,
            group_bytes,
        )?;
        let params = [
            u32::try_from(total_rows)
                .expect("WebGPU sparse grouped readback total_rows exceeds u32"),
            u32::try_from(group_specs.len() / 4)
                .expect("WebGPU sparse grouped readback group_count exceeds u32"),
            u32::try_from(packed_elems)
                .expect("WebGPU sparse grouped readback packed_elems exceeds u32"),
            0,
        ];
        let params_buf = hal.create_uniform_buffer(
            "webgpu_sparse_column_prefix_group_params",
            bytemuck::cast_slice(&params),
        )?;
        let layout = hal.create_bind_group_layout(
            "webgpu_sparse_column_prefix_group_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::uniform(3, 16),
                WebGpuBindingLayout::read_only_storage(4, 0),
            ],
        )?;
        let kernel = hal.create_compute_kernel(
            "webgpu_sparse_column_prefix_group",
            PACK_COLUMN_PREFIX_ROW_GROUPS_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = hal.create_bind_group(
            "webgpu_sparse_column_prefix_group_bind_group",
            &layout,
            &[
                WebGpuBufferBinding {
                    binding: 0,
                    buffer: gpu,
                    offset: self.byte_offset(),
                    size: Some(byte_len_for::<T>(self.cpu.size())),
                },
                WebGpuBufferBinding::new(1, &row_buf),
                WebGpuBufferBinding::new(2, &packed_buf),
                WebGpuBufferBinding::new(3, &params_buf),
                WebGpuBufferBinding::new(4, &group_buf),
            ],
        )?;
        let workgroups = u32::try_from(packed_elems)
            .expect("WebGPU sparse grouped readback packed element count exceeds u32")
            .div_ceil(WEBGPU_WORKGROUP_SIZE);
        hal.dispatch_compute_1d(&kernel, &bind_group, workgroups);

        let bytes = hal
            .read_buffer_range_named(&packed_buf, 0, packed_byte_len, name)
            .await?;
        let u32s: &[u32] = bytemuck::cast_slice(bytes.as_slice());
        ensure!(
            u32s.len() == packed_elems,
            "sparse grouped readback size mismatch for WebGPU buffer {}: got {} elems, expected {}",
            self.cpu.name(),
            u32s.len(),
            packed_elems
        );
        let values: &[T] =
            unsafe { std::slice::from_raw_parts(u32s.as_ptr() as *const T, u32s.len()) };
        self.cpu.view_mut(|cpu| {
            for spec in group_specs.chunks_exact(4) {
                let row_base = spec[0] as usize;
                let row_count = spec[1] as usize;
                let column_count = spec[2] as usize;
                let packed_base = spec[3] as usize;
                for col in 0..column_count {
                    let packed_col_offset = packed_base + col * row_count;
                    let cpu_col_offset = col * total_rows;
                    for row_pos in 0..row_count {
                        let row = row_indices[row_base + row_pos] as usize;
                        cpu[cpu_col_offset + row] = values[packed_col_offset + row_pos].clone();
                    }
                }
            }
        });
        self.mark_synced();
        Ok(())
    }

    /// Pack multiple sparse row groups for explicit column lists into one GPU
    /// buffer, then copy the packed result back into the matching CPU-shadow
    /// cells.
    pub async fn sync_gpu_column_set_row_groups_to_cpu_unchecked(
        &self,
        hal: &WebGpuHal,
        total_rows: usize,
        groups: &[(&[u32], &[u32])],
        name: &'static str,
    ) -> Result<()>
    where
        T: Clone,
    {
        ensure!(
            mem::size_of::<T>() == mem::size_of::<u32>(),
            "unchecked sparse column-set readback for WebGPU buffer {} requires u32-sized elements",
            self.cpu.name()
        );
        ensure!(
            total_rows > 0,
            "sparse column-set readback for WebGPU buffer {} requires nonzero total_rows",
            self.cpu.name()
        );

        let mut packed_elems = 0usize;
        let mut row_indices = Vec::new();
        let mut column_indices = Vec::new();
        let mut group_specs = Vec::<u32>::new();
        for &(columns, rows) in groups {
            if columns.is_empty() || rows.is_empty() {
                continue;
            }
            for &col in columns {
                ensure!(
                    (col as usize) < self.cpu.size().div_ceil(total_rows),
                    "sparse column-set readback column for WebGPU buffer {} exceeds backing columns: col={}, total_rows={}, size={}",
                    self.cpu.name(),
                    col,
                    total_rows,
                    self.cpu.size()
                );
                let covered_elems = total_rows.checked_mul(col as usize + 1).ok_or_else(|| {
                    anyhow!("WebGPU sparse column-set readback covered range overflow")
                })?;
                ensure!(
                    covered_elems <= self.cpu.size(),
                    "sparse column-set readback column for WebGPU buffer {} exceeds size: covered={}, size={}",
                    self.cpu.name(),
                    covered_elems,
                    self.cpu.size()
                );
            }
            for &row in rows {
                ensure!(
                    (row as usize) < total_rows,
                    "sparse column-set readback row for WebGPU buffer {} exceeds total_rows: row={}, total_rows={}",
                    self.cpu.name(),
                    row,
                    total_rows
                );
            }

            let row_base = row_indices.len();
            let column_base = column_indices.len();
            let row_count = rows.len();
            let column_count = columns.len();
            let group_packed = column_count.checked_mul(row_count).ok_or_else(|| {
                anyhow!("WebGPU sparse column-set readback packed group overflow")
            })?;
            group_specs.extend_from_slice(&[
                u32::try_from(row_base)
                    .expect("WebGPU sparse column-set readback row_base exceeds u32"),
                u32::try_from(row_count)
                    .expect("WebGPU sparse column-set readback row_count exceeds u32"),
                u32::try_from(column_base)
                    .expect("WebGPU sparse column-set readback column_base exceeds u32"),
                u32::try_from(column_count)
                    .expect("WebGPU sparse column-set readback column_count exceeds u32"),
                u32::try_from(packed_elems)
                    .expect("WebGPU sparse column-set readback packed_base exceeds u32"),
            ]);
            row_indices.extend_from_slice(rows);
            column_indices.extend_from_slice(columns);
            packed_elems = packed_elems.checked_add(group_packed).ok_or_else(|| {
                anyhow!("WebGPU sparse column-set readback packed total overflow")
            })?;
        }

        if packed_elems == 0 {
            self.mark_synced();
            return Ok(());
        }

        let Some(gpu) = self.raw_buffer() else {
            ensure!(
                self.cpu.size() == 0,
                "cannot read back missing GPU buffer {}",
                self.cpu.name()
            );
            self.mark_synced();
            return Ok(());
        };

        let packed_byte_len = byte_len_for::<T>(packed_elems);
        let row_bytes = bytemuck::cast_slice(row_indices.as_slice());
        let row_buf = hal.create_storage_buffer(
            "webgpu_sparse_column_set_group_rows",
            u64::try_from(row_bytes.len())
                .map_err(|_| anyhow!("WebGPU sparse column-set row-index bytes exceed u64"))?,
        )?;
        hal.write_buffer_named(
            &row_buf,
            "webgpu_sparse_column_set_group_rows",
            0,
            row_bytes,
        )?;
        let column_bytes = bytemuck::cast_slice(column_indices.as_slice());
        let column_buf = hal.create_storage_buffer(
            "webgpu_sparse_column_set_group_columns",
            u64::try_from(column_bytes.len())
                .map_err(|_| anyhow!("WebGPU sparse column-set column-index bytes exceed u64"))?,
        )?;
        hal.write_buffer_named(
            &column_buf,
            "webgpu_sparse_column_set_group_columns",
            0,
            column_bytes,
        )?;
        let packed_buf =
            hal.create_storage_buffer("webgpu_sparse_column_set_group_packed", packed_byte_len)?;
        let group_bytes = bytemuck::cast_slice(group_specs.as_slice());
        let group_buf = hal.create_storage_buffer(
            "webgpu_sparse_column_set_group_specs",
            u64::try_from(group_bytes.len())
                .map_err(|_| anyhow!("WebGPU sparse column-set spec bytes exceed u64"))?,
        )?;
        hal.write_buffer_named(
            &group_buf,
            "webgpu_sparse_column_set_group_specs",
            0,
            group_bytes,
        )?;
        let params = [
            u32::try_from(total_rows)
                .expect("WebGPU sparse column-set readback total_rows exceeds u32"),
            u32::try_from(group_specs.len() / 5)
                .expect("WebGPU sparse column-set readback group_count exceeds u32"),
            u32::try_from(packed_elems)
                .expect("WebGPU sparse column-set readback packed_elems exceeds u32"),
            0,
        ];
        let params_buf = hal.create_uniform_buffer(
            "webgpu_sparse_column_set_group_params",
            bytemuck::cast_slice(&params),
        )?;
        let layout = hal.create_bind_group_layout(
            "webgpu_sparse_column_set_group_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::uniform(3, 16),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::read_only_storage(5, 0),
            ],
        )?;
        let kernel = hal.create_compute_kernel(
            "webgpu_sparse_column_set_group",
            PACK_COLUMN_SET_ROW_GROUPS_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = hal.create_bind_group(
            "webgpu_sparse_column_set_group_bind_group",
            &layout,
            &[
                WebGpuBufferBinding {
                    binding: 0,
                    buffer: gpu,
                    offset: self.byte_offset(),
                    size: Some(byte_len_for::<T>(self.cpu.size())),
                },
                WebGpuBufferBinding::new(1, &row_buf),
                WebGpuBufferBinding::new(2, &packed_buf),
                WebGpuBufferBinding::new(3, &params_buf),
                WebGpuBufferBinding::new(4, &group_buf),
                WebGpuBufferBinding::new(5, &column_buf),
            ],
        )?;
        let workgroups = u32::try_from(packed_elems)
            .expect("WebGPU sparse column-set readback packed element count exceeds u32")
            .div_ceil(WEBGPU_WORKGROUP_SIZE);
        hal.dispatch_compute_1d(&kernel, &bind_group, workgroups);

        let bytes = hal
            .read_buffer_range_named(&packed_buf, 0, packed_byte_len, name)
            .await?;
        let u32s: &[u32] = bytemuck::cast_slice(bytes.as_slice());
        ensure!(
            u32s.len() == packed_elems,
            "sparse column-set readback size mismatch for WebGPU buffer {}: got {} elems, expected {}",
            self.cpu.name(),
            u32s.len(),
            packed_elems
        );
        let values: &[T] =
            unsafe { std::slice::from_raw_parts(u32s.as_ptr() as *const T, u32s.len()) };
        self.cpu.view_mut(|cpu| {
            for spec in group_specs.chunks_exact(5) {
                let row_base = spec[0] as usize;
                let row_count = spec[1] as usize;
                let column_base = spec[2] as usize;
                let column_count = spec[3] as usize;
                let packed_base = spec[4] as usize;
                for column_pos in 0..column_count {
                    let packed_col_offset = packed_base + column_pos * row_count;
                    let col = column_indices[column_base + column_pos] as usize;
                    let cpu_col_offset = col * total_rows;
                    for row_pos in 0..row_count {
                        let row = row_indices[row_base + row_pos] as usize;
                        cpu[cpu_col_offset + row] = values[packed_col_offset + row_pos].clone();
                    }
                }
            }
        });
        self.mark_synced();
        Ok(())
    }

    pub(crate) fn assert_cpu_current(&self, op: &str)
    where
        T: Clone,
    {
        assert!(
            !self.cpu_stale.get(),
            "{op} requires a current CPU shadow for WebGPU buffer {}; call sync_gpu_to_cpu(...).await first",
            self.cpu.name()
        );
    }

    /// Begin a pool-offloaded mutable pass over the CPU shadow. Same
    /// currency discipline as [`Buffer::view_mut`], but instead of running
    /// the closure inline (which blocks this wasm thread and starves other
    /// in-flight proofs' readback callbacks), it hands back the
    /// `Send + Sync` [`CpuBuffer`] handle for a worker thread to mutate.
    /// Until the worker finishes, no other access to this buffer (through
    /// any clone) is allowed; afterwards the caller must invoke
    /// [`Self::finish_cpu_shadow_offload_mut`] to record the CPU write,
    /// exactly as `view_mut` would have.
    pub fn begin_cpu_shadow_offload_mut(&self) -> CpuBuffer<T>
    where
        T: Clone,
    {
        self.assert_cpu_current("begin_cpu_shadow_offload_mut");
        self.cpu.clone()
    }

    /// Read-only counterpart of
    /// [`Self::begin_cpu_shadow_offload_mut`]; pairs with a worker-side
    /// `view` and needs no completion call (matching [`Buffer::view`]).
    pub fn begin_cpu_shadow_offload(&self) -> CpuBuffer<T>
    where
        T: Clone,
    {
        self.assert_cpu_current("begin_cpu_shadow_offload");
        self.cpu.clone()
    }

    /// Complete an offloaded mutable shadow pass — the flag half of
    /// [`Buffer::view_mut`].
    pub fn finish_cpu_shadow_offload_mut(&self) {
        self.mark_cpu_dirty();
    }
}

impl WebGpuBuffer<BabyBearElem> {
    /// Upload only cells whose CPU value is neither zero nor INVALID, leaving
    /// every other GPU cell at WebGPU's zero-initialized default. This is for
    /// consumers that intentionally use the CPU `valid_or_zero` semantics on
    /// the GPU side.
    pub fn sync_cpu_to_gpu_zero_default_sparse_named(
        &self,
        hal: &WebGpuHal,
        name: &'static str,
    ) -> Result<bool> {
        if !self.cpu_dirty.get() {
            return Ok(true);
        }
        ensure!(
            !self.cpu_stale.get(),
            "cannot upload stale CPU shadow for WebGPU buffer {}",
            self.cpu.name()
        );
        if self.byte_offset() != 0 {
            return Ok(false);
        }

        let Some(gpu) = self.raw_buffer() else {
            self.cpu_dirty.set(false);
            return Ok(true);
        };

        let dense_byte_len = byte_len_for::<BabyBearElem>(self.cpu.size());
        let mut plan = Ok(SparseUploadPlan::new());
        self.cpu.view(|cpu| {
            plan = build_sparse_zero_upload_plan(cpu);
        });
        let plan = plan?;

        if plan.values.is_empty() {
            log_webgpu_metric(&format!(
                "webgpu_zero_default_sparse_upload name={name} values=0 ranges=0 sparse_bytes=0 dense_bytes={dense_byte_len}",
            ));
            self.cpu_dirty.set(false);
            return Ok(true);
        }

        let value_byte_len = byte_len_for::<BabyBearElem>(plan.values.len());
        let range_byte_len = byte_len_for::<u32>(plan.ranges.len());
        let sparse_byte_len = value_byte_len
            .checked_add(range_byte_len)
            .and_then(|len| len.checked_add(16))
            .ok_or_else(|| anyhow!("sparse zero-default upload byte length overflow"))?;
        if sparse_byte_len >= dense_byte_len || sparse_byte_len > hal.max_storage_binding_bytes() {
            log_webgpu_metric(&format!(
                "webgpu_zero_default_sparse_upload name={name} fallback values={} ranges={} sparse_bytes={} dense_bytes={} max_binding={}",
                plan.values.len(),
                plan.ranges.len() / 2,
                sparse_byte_len,
                dense_byte_len,
                hal.max_storage_binding_bytes(),
            ));
            return Ok(false);
        }

        dispatch_sparse_zero_upload(
            hal,
            gpu,
            self.byte_offset(),
            dense_byte_len,
            &plan,
            "webgpu_zero_default_sparse_values",
            "webgpu_zero_default_sparse_ranges",
            "webgpu_zero_default_sparse_params",
            "webgpu_zero_default_sparse_upload_layout",
            "webgpu_zero_default_sparse_upload",
            "webgpu_zero_default_sparse_upload_bind_group",
        )?;
        log_webgpu_metric(&format!(
            "webgpu_zero_default_sparse_upload name={name} values={} ranges={} sparse_bytes={} dense_bytes={dense_byte_len}",
            plan.values.len(),
            plan.ranges.len() / 2,
            sparse_byte_len,
        ));
        self.cpu_dirty.set(false);
        self.mark_gpu_may_be_nonzero();
        Ok(true)
    }
}

impl<T: Clone> super::Buffer<T> for WebGpuBuffer<T> {
    fn name(&self) -> &'static str {
        self.cpu.name()
    }

    fn size(&self) -> usize {
        self.cpu.size()
    }

    fn slice(&self, offset: usize, size: usize) -> Self {
        let cpu = self.cpu.slice(offset, size);
        Self {
            cpu,
            gpu: self.gpu.clone(),
            elem_offset: self.elem_offset + offset,
            cpu_dirty: self.cpu_dirty.clone(),
            cpu_stale: self.cpu_stale.clone(),
            gpu_known_zero: self.gpu_known_zero.clone(),
            marker: PhantomData,
        }
    }

    fn get_at(&self, idx: usize) -> T {
        self.assert_cpu_current("get_at");
        self.cpu.get_at(idx)
    }

    fn view<F: FnOnce(&[T])>(&self, f: F) {
        self.assert_cpu_current("view");
        self.cpu.view(f);
    }

    fn view_mut<F: FnOnce(&mut [T])>(&self, f: F) {
        self.assert_cpu_current("view_mut");
        self.cpu.view_mut(f);
        self.mark_cpu_dirty();
    }

    fn to_vec(&self) -> Vec<T> {
        self.assert_cpu_current("to_vec");
        self.cpu.to_vec()
    }
}
