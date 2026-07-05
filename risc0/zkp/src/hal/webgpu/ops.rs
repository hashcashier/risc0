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

//! Buffer creation and the async operation wrappers (mix, combos,
//! FRI fold, hash, gather/scatter, transpose).

use super::*;
#[allow(unused_imports)]
use super::{device::*, diagnostics::*, dispatch::*, eval_check::*, kernels_wgsl::*, resources::*};

impl WebGpuHal {
    /// Async-safe variant of [`Hal::mix_poly_coeffs`].
    ///
    /// Unlike most HAL operations, `mix_poly_coeffs` accumulates into `out`
    /// across several calls. In GPU-authoritative proving a later call can
    /// legitimately fall back to CPU after an earlier call wrote `out` on the
    /// GPU, so the fallback path must first materialize the accumulated GPU
    /// contents into the CPU shadow.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn mix_poly_coeffs_async(
        &self,
        out: &WebGpuBuffer<BabyBearExtElem>,
        mix_start: &BabyBearExtElem,
        mix: &BabyBearExtElem,
        input: &WebGpuBuffer<BabyBearElem>,
        combos: &WebGpuBuffer<u32>,
        input_size: usize,
        count: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative() {
            out.sync_gpu_to_cpu(self).await?;
            input.sync_gpu_to_cpu(self).await?;
            combos.sync_gpu_to_cpu(self).await?;
        }
        let gpu_mixed = self.dispatch_mix_poly_coeffs_inner(
            out, mix_start, mix, input, combos, input_size, count, true,
        )?;
        let gpu_mixed = if self.gpu_authoritative() && !gpu_mixed {
            self.dispatch_mix_poly_coeffs_chunked(
                out, mix_start, mix, input, combos, input_size, count,
            )?
        } else {
            gpu_mixed
        };
        if self.gpu_authoritative() {
            if gpu_mixed {
                self.record_gpu_result_authoritative("mix_poly_coeffs", true);
                out.mark_gpu_dirty();
            } else {
                out.sync_gpu_to_cpu(self).await?;
                input.sync_gpu_to_cpu(self).await?;
                combos.sync_gpu_to_cpu(self).await?;
                self.cpu.mix_poly_coeffs(
                    out.cpu(),
                    mix_start,
                    mix,
                    input.cpu(),
                    combos.cpu(),
                    input_size,
                    count,
                );
                self.record_gpu_result_authoritative("mix_poly_coeffs", false);
                out.mark_cpu_result(false);
            }
        } else {
            self.finish_hal_op("mix_poly_coeffs", gpu_mixed, out, || {
                self.cpu.mix_poly_coeffs(
                    out.cpu(),
                    mix_start,
                    mix,
                    input.cpu(),
                    combos.cpu(),
                    input_size,
                    count,
                );
            });
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn combos_prepare_async(
        &self,
        combos: &WebGpuBuffer<BabyBearExtElem>,
        coeff_u: &[BabyBearExtElem],
        combo_count: usize,
        cycles: usize,
        reg_sizes: &[u32],
        reg_combo_ids: &[u32],
        mix: &BabyBearExtElem,
    ) -> Result<()> {
        if !self.gpu_authoritative() {
            combos.sync_gpu_to_cpu(self).await?;
        }
        let gpu_prepared = self.dispatch_combos_prepare(
            combos,
            coeff_u,
            combo_count,
            cycles,
            reg_sizes,
            reg_combo_ids,
            mix,
        )?;
        if self.gpu_authoritative() {
            if gpu_prepared {
                self.record_gpu_result_authoritative("combos_prepare", true);
                combos.mark_gpu_dirty();
            } else {
                combos.sync_gpu_to_cpu(self).await?;
                self.cpu.combos_prepare(
                    combos.cpu(),
                    coeff_u,
                    combo_count,
                    cycles,
                    reg_sizes,
                    reg_combo_ids,
                    mix,
                );
                self.record_gpu_result_authoritative("combos_prepare", false);
                combos.mark_cpu_result(false);
            }
        } else {
            self.finish_hal_op("combos_prepare", gpu_prepared, combos, || {
                self.cpu.combos_prepare(
                    combos.cpu(),
                    coeff_u,
                    combo_count,
                    cycles,
                    reg_sizes,
                    reg_combo_ids,
                    mix,
                );
            });
        }
        Ok(())
    }

    pub(crate) async fn combos_divide_async(
        &self,
        combos: &WebGpuBuffer<BabyBearExtElem>,
        chunks: Vec<(usize, Vec<BabyBearExtElem>)>,
        cycles: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative() {
            combos.sync_gpu_to_cpu(self).await?;
        }
        let gpu_divided = self.dispatch_combos_divide(combos, &chunks, cycles)?;
        if self.gpu_authoritative() {
            if gpu_divided {
                self.record_gpu_result_authoritative("combos_divide", true);
                combos.mark_gpu_dirty();
            } else {
                combos.sync_gpu_to_cpu(self).await?;
                self.cpu.combos_divide(combos.cpu(), chunks, cycles);
                self.record_gpu_result_authoritative("combos_divide", false);
                combos.mark_cpu_result(false);
            }
        } else {
            self.finish_hal_op("combos_divide", gpu_divided, combos, || {
                self.cpu.combos_divide(combos.cpu(), chunks, cycles);
            });
        }
        Ok(())
    }

    /// Async-safe variant of [`Hal::eltwise_sum_extelem`].
    pub(crate) async fn eltwise_sum_extelem_async(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearExtElem>,
    ) -> Result<()> {
        if !self.gpu_authoritative() || !self.can_dispatch_eltwise_sum_extelem(output, input) {
            input.sync_gpu_to_cpu(self).await?;
        }
        self.eltwise_sum_extelem(output, input);
        Ok(())
    }

    /// Async-safe variant of [`Hal::fri_fold`].
    pub(crate) async fn fri_fold_async(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
        mix: &BabyBearExtElem,
    ) -> Result<()> {
        if !self.gpu_authoritative() || !self.can_dispatch_fri_fold(output, input) {
            input.sync_gpu_to_cpu(self).await?;
        }
        self.fri_fold(output, input, mix);
        Ok(())
    }

    /// Async-safe variant of [`Hal::hash_fold`].
    pub(crate) async fn hash_fold_async(
        &self,
        io: &WebGpuBuffer<Digest>,
        input_size: usize,
        output_size: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative() || !self.can_dispatch_hash_fold(io, output_size) {
            io.sync_gpu_to_cpu(self).await?;
        }
        self.hash_fold(io, input_size, output_size);
        Ok(())
    }

    /// Batch a chain of `hash_fold_async` calls
    /// (the merkle build loop) into a single submit. Saves
    /// ~`output_sizes.len() - 1` GPU-process IPC round-trips. Falls
    /// through to per-call hash_fold_async when GPU dispatch is
    /// unavailable so the diagnostic counters stay accurate.
    pub async fn hash_fold_chain_async(
        &self,
        io: &WebGpuBuffer<Digest>,
        output_sizes: &[usize],
    ) -> Result<()> {
        let can_chain = self.gpu_authoritative()
            && output_sizes
                .iter()
                .all(|&out| self.can_dispatch_hash_fold(io, out));
        if can_chain {
            if self.dispatch_poseidon2_hash_fold_chain(io, output_sizes)? {
                return Ok(());
            }
        }
        // Fallback: serial per-call path (CPU mirror branches still
        // need the per-call sync_gpu_to_cpu).
        for &output_size in output_sizes {
            self.hash_fold_async(io, 2 * output_size, output_size)
                .await?;
        }
        Ok(())
    }

    /// Async-safe variant of [`Hal::hash_rows`].
    pub(crate) async fn hash_rows_async(
        &self,
        output: &WebGpuBuffer<Digest>,
        matrix: &WebGpuBuffer<BabyBearElem>,
    ) -> Result<()> {
        let row_size = output.size();
        if !self.gpu_authoritative() || !self.can_dispatch_hash_rows(output, matrix, row_size) {
            matrix.sync_gpu_to_cpu(self).await?;
        }
        self.hash_rows(output, matrix);
        Ok(())
    }

    /// Async-safe variant of [`Hal::gather_sample`].
    pub(crate) async fn gather_sample_async(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src: &WebGpuBuffer<BabyBearElem>,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> Result<()> {
        let can_dispatch = self.can_dispatch_gather_sample(dst, src, idx, size, stride);
        if self.gpu_authoritative() && can_dispatch {
            self.gather_sample(dst, src, idx, size, stride);
            return Ok(());
        }

        if self.gpu_authoritative()
            && !src.cpu_is_current()
            && size <= dst.size()
            && gather_region_in_bounds(src.size(), idx, size, stride)
        {
            src.sync_cpu_to_gpu(self)?;
            if let Some(src_gpu) = src.raw_buffer() {
                let sample = self
                    .read_gathered_elem_sample(
                        src_gpu,
                        src.name(),
                        src.elem_offset,
                        idx,
                        size,
                        stride,
                    )
                    .await?;
                dst.cpu.view_mut(|cpu| {
                    cpu[..sample.len()].clone_from_slice(sample.as_slice());
                });
                dst.mark_cpu_result(false);
                self.diagnostics.record_cpu_fallback("gather_sample");
                return Ok(());
            }
        }

        if !self.gpu_authoritative() || !can_dispatch {
            src.sync_gpu_to_cpu(self).await?;
        }
        self.gather_sample(dst, src, idx, size, stride);
        Ok(())
    }

    /// Test hook for the async gather path used by Merkle query openings.
    #[doc(hidden)]
    pub async fn debug_gather_sample_async(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src: &WebGpuBuffer<BabyBearElem>,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> Result<()> {
        self.gather_sample_async(dst, src, idx, size, stride).await
    }

    /// Test hook for validating chunked gather bindings without changing the
    /// normal proving path.
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub fn debug_dispatch_gather_sample_chunked(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src: &WebGpuBuffer<BabyBearElem>,
        idx: usize,
        size: usize,
        stride: usize,
        chunk_cols: usize,
    ) -> Result<()> {
        if size == 0 {
            return Ok(());
        }
        ensure!(chunk_cols > 0, "WebGPU gather chunk size must be nonzero");
        ensure!(
            size <= dst.size() && gather_region_in_bounds(src.size(), idx, size, stride),
            "WebGPU gather chunk test region is out of bounds"
        );

        let (Some(dst_gpu), Some(src_gpu)) = (dst.raw_buffer(), src.raw_buffer()) else {
            return Err(anyhow!("WebGPU gather chunk test requires GPU buffers"));
        };
        ensure!(
            dst_gpu != src_gpu,
            "WebGPU gather chunk test aliases buffers"
        );

        src.sync_cpu_to_gpu(self)?;
        if size < dst.size() {
            dst.sync_cpu_to_gpu(self)?;
        }
        self.dispatch_gather_sample_chunked(
            dst, src, dst_gpu, src_gpu, idx, size, stride, chunk_cols,
        )?;
        dst.mark_gpu_dirty();
        Ok(())
    }

    /// Test hook: exercise `dispatch_gather_sample_tiled`
    /// over a `BufferPool` source. The pool's `layout` must match the
    /// caller's `stride` and `size`. Production callers will fall into
    /// this path automatically when the recursion data group is
    /// `BufferPool`-backed; the test exercises it directly.
    #[doc(hidden)]
    pub fn debug_dispatch_gather_sample_tiled(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src_pool: &buffer_pool::BufferPool,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> Result<()> {
        self.dispatch_gather_sample_tiled(dst, src_pool, idx, size, stride)?;
        dst.mark_gpu_dirty();
        Ok(())
    }

    /// Test hook for validating `mix_poly_coeffs` under GPU-authoritative state
    /// without enabling that path in production proving.
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub fn debug_dispatch_mix_poly_coeffs_authoritative(
        &self,
        output: &WebGpuBuffer<BabyBearExtElem>,
        mix_start: &BabyBearExtElem,
        mix: &BabyBearExtElem,
        input: &WebGpuBuffer<BabyBearElem>,
        combos: &WebGpuBuffer<u32>,
        input_size: usize,
        count: usize,
    ) -> Result<bool> {
        self.dispatch_mix_poly_coeffs_inner(
            output, mix_start, mix, input, combos, input_size, count, true,
        )
    }

    pub(crate) fn alloc_shadowed_buffer<T>(
        &self,
        name: &'static str,
        cpu: CpuBuffer<T>,
    ) -> WebGpuBuffer<T>
    where
        T: Clone,
    {
        let byte_len = byte_len_for::<T>(cpu.size());
        let gpu = if byte_len == 0 || !self.can_allocate_gpu_buffer(byte_len) {
            None
        } else {
            Some(Rc::new(WebGpuBufferOwner {
                buffer: self
                    .create_storage_buffer(name, byte_len)
                    .unwrap_or_else(|err| panic!("failed to allocate WebGPU buffer {name}: {err}")),
            }))
        };
        WebGpuBuffer::new(
            cpu,
            gpu,
            Rc::new(Cell::new(true)),
            Rc::new(Cell::new(false)),
            Rc::new(Cell::new(true)),
        )
    }

    pub(crate) fn copy_shadowed_buffer<T>(
        &self,
        name: &'static str,
        cpu: CpuBuffer<T>,
        slice: &[T],
    ) -> WebGpuBuffer<T>
    where
        T: bytemuck::NoUninit + Clone,
    {
        let buffer = self.alloc_shadowed_buffer(name, cpu);
        if let Some(gpu) = buffer.raw_buffer() {
            self.write_buffer_named(gpu, name, 0, bytemuck::cast_slice(slice))
                .unwrap_or_else(|err| panic!("failed to upload WebGPU buffer {name}: {err}"));
        }
        buffer.mark_synced();
        buffer
    }

    pub(crate) fn copy_elem_transpose_zero_pad_gpu(
        &self,
        name: &'static str,
        compact_name: &'static str,
        row_major: &[BabyBearElem],
        rows: usize,
        cols: usize,
        total_rows: usize,
        cache_key: Option<Digest>,
    ) -> Result<WebGpuBuffer<BabyBearElem>> {
        ensure!(
            rows <= total_rows,
            "transpose-zero-pad rows {rows} exceeds total_rows {total_rows}"
        );
        let compact_len = rows
            .checked_mul(cols)
            .ok_or_else(|| anyhow!("transpose-zero-pad compact length overflow"))?;
        ensure!(
            row_major.len() == compact_len,
            "transpose-zero-pad source length mismatch: got {}, expected {compact_len}",
            row_major.len()
        );
        let padded_len = total_rows
            .checked_mul(cols)
            .ok_or_else(|| anyhow!("transpose-zero-pad padded length overflow"))?;
        let cache_key = cache_key.map(|control_id| ElemTransposeZeroPadCacheKey {
            control_id,
            rows,
            cols,
            total_rows,
        });
        if let Some(cache_key) = cache_key {
            if let Some(buffer) = self.elem_transpose_zero_pad_cache.borrow().get(&cache_key) {
                return Ok(buffer.clone());
            }
        }

        let mut column_major = vec![BabyBearElem::ZERO; padded_len];
        for row in 0..rows {
            for col in 0..cols {
                column_major[col * total_rows + row] = row_major[row * cols + col];
            }
        }

        let buffer = self.alloc_shadowed_buffer(name, self.cpu.copy_from_elem(name, &column_major));
        let Some(dst_gpu) = buffer.raw_buffer() else {
            buffer.mark_synced();
            if let Some(cache_key) = cache_key {
                self.elem_transpose_zero_pad_cache
                    .borrow_mut()
                    .insert(cache_key, buffer.clone());
            }
            return Ok(buffer);
        };
        if compact_len == 0 {
            buffer.mark_synced();
            if let Some(cache_key) = cache_key {
                self.elem_transpose_zero_pad_cache
                    .borrow_mut()
                    .insert(cache_key, buffer.clone());
            }
            return Ok(buffer);
        }

        let rows_u32 =
            u32::try_from(rows).map_err(|_| anyhow!("transpose-zero-pad rows exceeds u32"))?;
        let cols_u32 =
            u32::try_from(cols).map_err(|_| anyhow!("transpose-zero-pad cols exceeds u32"))?;
        let total_rows_u32 = u32::try_from(total_rows)
            .map_err(|_| anyhow!("transpose-zero-pad total_rows exceeds u32"))?;
        let compact_len_u32 = u32::try_from(compact_len)
            .map_err(|_| anyhow!("transpose-zero-pad compact length exceeds u32"))?;

        let src =
            self.create_storage_buffer(compact_name, byte_len_for::<BabyBearElem>(compact_len))?;
        self.write_buffer_named(&src, compact_name, 0, bytemuck::cast_slice(row_major))?;
        let params = [rows_u32, cols_u32, total_rows_u32, compact_len_u32];
        let params_buf = self.create_uniform_buffer(
            "webgpu_transpose_zero_pad_params",
            bytemuck::cast_slice(&params),
        )?;
        let layout = self.create_bind_group_layout(
            "webgpu_transpose_zero_pad_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::uniform(2, 16),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_transpose_zero_pad",
            TRANSPOSE_ZERO_PAD_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_transpose_zero_pad_bg",
            &layout,
            &[
                WebGpuBufferBinding::new(0, &src),
                WebGpuBufferBinding::new(1, dst_gpu),
                WebGpuBufferBinding::new(2, &params_buf),
            ],
        )?;
        self.dispatch_compute_1d(&kernel, &bind_group, compact_len_u32.div_ceil(256));
        self.diagnostics
            .record_gpu_dispatch("copy_from_elem_transpose_zero_pad");
        buffer.mark_synced();
        if let Some(cache_key) = cache_key {
            self.elem_transpose_zero_pad_cache
                .borrow_mut()
                .insert(cache_key, buffer.clone());
        }
        Ok(buffer)
    }

    /// Create a raw WebGPU buffer with the requested usage flags.
    pub fn create_buffer(
        &self,
        label: &'static str,
        byte_len: u64,
        usage: u32,
    ) -> Result<web_sys::GpuBuffer> {
        let desc = web_sys::GpuBufferDescriptor::new(byte_len_as_f64(byte_len)?, usage);
        desc.set_label(label);
        let buffer = self.device.create_buffer(&desc).map_err(js_error)?;
        self.diagnostics.record_buffer_allocated(byte_len);
        Ok(buffer)
    }

    /// Create a storage buffer suitable for compute kernels and host uploads.
    pub fn create_storage_buffer(
        &self,
        label: &'static str,
        byte_len: u64,
    ) -> Result<web_sys::GpuBuffer> {
        self.create_buffer(
            label,
            byte_len,
            WEBGPU_BUFFER_USAGE_STORAGE
                | WEBGPU_BUFFER_USAGE_COPY_DST
                | WEBGPU_BUFFER_USAGE_COPY_SRC,
        )
    }

    /// Create a uniform buffer suitable for compute kernel parameters.
    pub fn create_uniform_buffer(
        &self,
        label: &'static str,
        bytes: &[u8],
    ) -> Result<web_sys::GpuBuffer> {
        let buffer = self.create_buffer(
            label,
            bytes.len() as u64,
            WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;
        self.write_buffer_named(&buffer, label, 0, bytes)?;
        Ok(buffer)
    }

    /// Upload raw bytes to a WebGPU buffer.
    pub fn write_buffer(
        &self,
        buffer: &web_sys::GpuBuffer,
        byte_offset: u64,
        bytes: &[u8],
    ) -> Result<()> {
        self.write_buffer_named(buffer, "unattributed", byte_offset, bytes)
    }

    /// Upload raw bytes to a WebGPU buffer and attribute diagnostics to `name`.
    pub fn write_buffer_named(
        &self,
        buffer: &web_sys::GpuBuffer,
        name: &'static str,
        byte_offset: u64,
        bytes: &[u8],
    ) -> Result<()> {
        self.queue
            .write_buffer_with_f64_and_u8_slice(buffer, byte_offset_as_f64(byte_offset)?, bytes)
            .map_err(js_error)?;
        self.diagnostics.record_upload(name, bytes.len() as u64);
        Ok(())
    }

    /// Create a bind group layout for compute kernels.
    pub fn create_bind_group_layout(
        &self,
        label: &'static str,
        entries: &[WebGpuBindingLayout],
    ) -> Result<web_sys::GpuBindGroupLayout> {
        // Cache layouts by (label_ptr,
        // entries-shape) so callers that ask for the same layout get
        // the same JS instance. WebGPU pipelines bind specifically to
        // the layout INSTANCE they were created with -- a later
        // pipeline cache requires this invariant to be sound.
        let cache_key = compute_bind_group_layout_cache_key(label, entries);
        if let Some(cached) = self
            .bind_group_layout_cache
            .borrow()
            .get(&cache_key)
            .cloned()
        {
            self.diagnostics.record_bind_group_layout_cache_hit();
            return Ok(cached);
        }

        let layout_entries = js_sys::Array::new();
        for entry in entries {
            let buffer = web_sys::GpuBufferBindingLayout::new();
            buffer.set_type(entry.ty);
            if entry.min_binding_size != 0 {
                buffer.set_min_binding_size(byte_len_as_f64(entry.min_binding_size)?);
            }
            if entry.has_dynamic_offset {
                buffer.set_has_dynamic_offset(true);
            }

            let layout_entry =
                web_sys::GpuBindGroupLayoutEntry::new(entry.binding, WEBGPU_SHADER_STAGE_COMPUTE);
            layout_entry.set_buffer(&buffer);
            layout_entries.push(layout_entry.as_ref());
        }

        let desc = web_sys::GpuBindGroupLayoutDescriptor::new(layout_entries.as_ref());
        desc.set_label(label);
        let layout = self
            .device
            .create_bind_group_layout(&desc)
            .map_err(js_error)?;
        self.diagnostics.record_bind_group_layout_creation();
        self.bind_group_layout_cache
            .borrow_mut()
            .insert(cache_key, layout.clone());
        self.bind_group_layout_key_map
            .set(layout.as_ref(), &JsValue::from_str(&cache_key.to_string()));
        Ok(layout)
    }

    /// Create a bind group from concrete WebGPU buffers.
    pub fn create_bind_group(
        &self,
        label: &'static str,
        layout: &web_sys::GpuBindGroupLayout,
        entries: &[WebGpuBufferBinding<'_>],
    ) -> Result<web_sys::GpuBindGroup> {
        let bind_entries = js_sys::Array::new();
        for entry in entries {
            let binding = web_sys::GpuBufferBinding::new(entry.buffer);
            if entry.offset != 0 {
                binding.set_offset(byte_offset_as_f64(entry.offset)?);
            }
            if let Some(size) = entry.size {
                binding.set_size(byte_len_as_f64(size)?);
            }

            let resource = JsValue::from(binding);
            let bind_entry = web_sys::GpuBindGroupEntry::new(entry.binding, &resource);
            bind_entries.push(bind_entry.as_ref());
        }

        let desc = web_sys::GpuBindGroupDescriptor::new(bind_entries.as_ref(), layout);
        desc.set_label(label);
        self.diagnostics.record_bind_group_creation();
        Ok(self.device.create_bind_group(&desc))
    }

    /// Compile a WGSL compute kernel with explicit bind group layouts.
    pub fn create_compute_kernel(
        &self,
        label: &'static str,
        wgsl: &str,
        entry_point: &str,
        bind_group_layouts: &[web_sys::GpuBindGroupLayout],
    ) -> Result<WebGpuKernel> {
        let cache_key = self.compute_kernel_cache_key(label, wgsl, entry_point, bind_group_layouts);
        if let Some(cache_key) = cache_key {
            if let Some(cached) = self.compute_kernel_cache.borrow().get(&cache_key).cloned() {
                self.diagnostics.record_compute_pipeline_cache_hit();
                return Ok(cached);
            }
        }

        let pipeline_desc =
            self.build_compute_pipeline_desc(label, wgsl, entry_point, bind_group_layouts);
        let kernel = WebGpuKernel {
            pipeline: self.device.create_compute_pipeline(&pipeline_desc),
        };
        self.diagnostics.record_compute_pipeline_creation();
        if let Some(cache_key) = cache_key {
            self.compute_kernel_cache
                .borrow_mut()
                .insert(cache_key, kernel.clone());
        }
        Ok(kernel)
    }

    /// Async-compile the same kernel via
    /// `device.createComputePipelineAsync()`. The Tint compile runs in
    /// the browser GPU process while the wasm thread does other work
    /// (guest execution, session setup); when the returned Future
    /// resolves, the kernel is ready to dispatch. Useful for the
    /// witgen probe so the ~60 s exec_TopChunk0 compile overlaps with
    /// xgboost's segment-level prover work instead of blocking the
    /// first witgen call.
    pub async fn create_compute_kernel_async(
        &self,
        label: &'static str,
        wgsl: &str,
        entry_point: &str,
        bind_group_layouts: &[web_sys::GpuBindGroupLayout],
    ) -> Result<WebGpuKernel> {
        let started =
            self.start_compute_kernel_async(label, wgsl, entry_point, bind_group_layouts)?;
        self.finish_compute_kernel_async(started).await
    }

    /// Start an async WGSL compute pipeline compile immediately and
    /// return a handle that can be awaited later.
    pub fn start_compute_kernel_async(
        &self,
        label: &'static str,
        wgsl: &str,
        entry_point: &str,
        bind_group_layouts: &[web_sys::GpuBindGroupLayout],
    ) -> Result<WebGpuStartedComputeKernel> {
        let cache_key = self.compute_kernel_cache_key(label, wgsl, entry_point, bind_group_layouts);
        if let Some(cache_key) = cache_key {
            if let Some(cached) = self.compute_kernel_cache.borrow().get(&cache_key).cloned() {
                return Ok(WebGpuStartedComputeKernel {
                    label,
                    cache_key: Some(cache_key),
                    cached: Some(cached),
                    promise: None,
                });
            }
        }

        let pipeline_desc =
            self.build_compute_pipeline_desc(label, wgsl, entry_point, bind_group_layouts);
        let promise = self.device.create_compute_pipeline_async(&pipeline_desc);
        Ok(WebGpuStartedComputeKernel {
            label,
            cache_key,
            cached: None,
            promise: Some(promise),
        })
    }

    /// Await a previously-started async compute pipeline compile.
    pub async fn finish_compute_kernel_async(
        &self,
        started: WebGpuStartedComputeKernel,
    ) -> Result<WebGpuKernel> {
        if let Some(cached) = started.cached {
            self.diagnostics.record_compute_pipeline_cache_hit();
            return Ok(cached);
        }

        let promise = started.promise.ok_or_else(|| {
            anyhow::anyhow!("started compute kernel {} missing promise", started.label)
        })?;
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        let pipeline_value = future.await.map_err(|err| {
            anyhow::anyhow!(
                "createComputePipelineAsync rejected label={}: {err:?}",
                started.label
            )
        })?;
        let pipeline: web_sys::GpuComputePipeline = pipeline_value.dyn_into().map_err(|_| {
            anyhow::anyhow!(
                "createComputePipelineAsync label={} resolved to non-pipeline value",
                started.label
            )
        })?;

        if let Some(cache_key) = started.cache_key {
            if let Some(cached) = self.compute_kernel_cache.borrow().get(&cache_key).cloned() {
                self.diagnostics.record_compute_pipeline_cache_hit();
                return Ok(cached);
            }
        }

        let kernel = WebGpuKernel { pipeline };
        self.diagnostics.record_compute_pipeline_creation();
        if let Some(cache_key) = started.cache_key {
            self.compute_kernel_cache
                .borrow_mut()
                .insert(cache_key, kernel.clone());
        }
        Ok(kernel)
    }

    pub(crate) fn compute_kernel_cache_key(
        &self,
        label: &'static str,
        wgsl: &str,
        entry_point: &str,
        bind_group_layouts: &[web_sys::GpuBindGroupLayout],
    ) -> Option<u64> {
        let layout_keys = bind_group_layouts
            .iter()
            .map(|layout| {
                self.bind_group_layout_key_map
                    .get(layout.as_ref())
                    .as_string()
            })
            .collect::<Option<Vec<_>>>()?;
        Some(compute_compute_pipeline_cache_key(
            label,
            wgsl,
            entry_point,
            layout_keys.as_slice(),
        ))
    }

    pub(crate) fn build_compute_pipeline_desc(
        &self,
        label: &'static str,
        wgsl: &str,
        entry_point: &str,
        bind_group_layouts: &[web_sys::GpuBindGroupLayout],
    ) -> web_sys::GpuComputePipelineDescriptor {
        let shader_desc = web_sys::GpuShaderModuleDescriptor::new(wgsl);
        shader_desc.set_label(label);
        let shader = self.device.create_shader_module(&shader_desc);

        let stage = web_sys::GpuProgrammableStage::new(&shader);
        stage.set_entry_point(entry_point);

        let layouts = js_sys::Array::new();
        for layout in bind_group_layouts {
            layouts.push(layout.as_ref());
        }
        let layout_desc = web_sys::GpuPipelineLayoutDescriptor::new(layouts.as_ref());
        let pipeline_layout = self.device.create_pipeline_layout(&layout_desc);
        let pipeline_layout = JsValue::from(pipeline_layout);

        let pipeline_desc = web_sys::GpuComputePipelineDescriptor::new(&pipeline_layout, &stage);
        pipeline_desc.set_label(label);
        pipeline_desc
    }

    /// Dispatch a compute kernel once and submit the command buffer.
    pub fn dispatch_compute(
        &self,
        kernel: &WebGpuKernel,
        bind_group: &web_sys::GpuBindGroup,
        workgroups_x: u32,
        workgroups_y: u32,
        workgroups_z: u32,
    ) {
        let encoder = self.device.create_command_encoder();
        let pass = encoder.begin_compute_pass();
        pass.set_pipeline(&kernel.pipeline);
        pass.set_bind_group(0, Some(bind_group));
        pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
            workgroups_x,
            workgroups_y,
            workgroups_z,
        );
        self.diagnostics.record_raw_compute_dispatch();
        pass.end();
        self.submit(encoder.finish());
    }

    /// Dispatch a logical 1D kernel, spilling workgroups into `y` when the
    /// `x` dimension would exceed WebGPU's portable per-dimension limit.
    pub fn dispatch_compute_1d(
        &self,
        kernel: &WebGpuKernel,
        bind_group: &web_sys::GpuBindGroup,
        workgroups: u32,
    ) {
        if workgroups <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION {
            self.dispatch_compute(kernel, bind_group, workgroups, 1, 1);
            return;
        }

        let workgroups_y = workgroups.div_ceil(WEBGPU_MAX_WORKGROUPS_PER_DIMENSION);
        assert!(
            workgroups_y <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION,
            "WebGPU 1D dispatch exceeds portable 2D workgroup capacity"
        );
        self.dispatch_compute(
            kernel,
            bind_group,
            WEBGPU_MAX_WORKGROUPS_PER_DIMENSION,
            workgroups_y,
            1,
        );
    }

    /// Dispatch multiple logical 1D kernels against the same bind group in one
    /// compute pass and queue submit.
    pub fn dispatch_compute_1d_sequence(
        &self,
        kernels: &[WebGpuKernel],
        bind_group: &web_sys::GpuBindGroup,
        workgroups: u32,
    ) {
        if kernels.is_empty() {
            return;
        }

        let (workgroups_x, workgroups_y) = if workgroups <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION {
            (workgroups, 1)
        } else {
            let workgroups_y = workgroups.div_ceil(WEBGPU_MAX_WORKGROUPS_PER_DIMENSION);
            assert!(
                workgroups_y <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION,
                "WebGPU 1D dispatch exceeds portable 2D workgroup capacity"
            );
            (WEBGPU_MAX_WORKGROUPS_PER_DIMENSION, workgroups_y)
        };

        let encoder = self.device.create_command_encoder();
        let pass = encoder.begin_compute_pass();
        pass.set_bind_group(0, Some(bind_group));
        for kernel in kernels {
            pass.set_pipeline(&kernel.pipeline);
            pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
                workgroups_x,
                workgroups_y,
                1,
            );
            self.diagnostics.record_raw_compute_dispatch();
        }
        pass.end();
        self.submit(encoder.finish());
    }

    /// Dispatch multiple logical 1D kernels, each with its own bind group and
    /// workgroup count, in one compute pass and queue submit.
    pub fn dispatch_compute_1d_bind_group_sequence(
        &self,
        dispatches: &[(&WebGpuKernel, &web_sys::GpuBindGroup, u32)],
    ) {
        if dispatches.is_empty() {
            return;
        }

        let encoder = self.device.create_command_encoder();
        let pass = encoder.begin_compute_pass();
        for (kernel, bind_group, workgroups) in dispatches {
            if *workgroups == 0 {
                continue;
            }
            let (workgroups_x, workgroups_y) = if *workgroups <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION
            {
                (*workgroups, 1)
            } else {
                let workgroups_y = (*workgroups).div_ceil(WEBGPU_MAX_WORKGROUPS_PER_DIMENSION);
                assert!(
                    workgroups_y <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION,
                    "WebGPU 1D dispatch exceeds portable 2D workgroup capacity"
                );
                (WEBGPU_MAX_WORKGROUPS_PER_DIMENSION, workgroups_y)
            };
            pass.set_pipeline(&kernel.pipeline);
            pass.set_bind_group(0, Some(bind_group));
            pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
                workgroups_x,
                workgroups_y,
                1,
            );
            self.diagnostics.record_raw_compute_dispatch();
        }
        pass.end();
        self.submit(encoder.finish());
    }

    /// Submit a finished command buffer to the WebGPU queue.
    pub fn submit(&self, command_buffer: web_sys::GpuCommandBuffer) {
        let commands = js_sys::Array::new();
        commands.push(command_buffer.as_ref());
        self.queue.submit(commands.as_ref());
        self.diagnostics.record_queue_submit();
    }

    /// Clear a byte range in a WebGPU buffer.
    pub fn clear_gpu_buffer(
        &self,
        buffer: &web_sys::GpuBuffer,
        byte_offset: u64,
        byte_len: u64,
    ) -> Result<()> {
        let encoder = self.device.create_command_encoder();
        encoder.clear_buffer_with_f64_and_f64(
            buffer,
            byte_offset_as_f64(byte_offset)?,
            byte_len_as_f64(byte_len)?,
        );
        self.submit(encoder.finish());
        Ok(())
    }

    /// Copy a byte range between WebGPU buffers.
    pub fn copy_gpu_buffer(
        &self,
        source: &web_sys::GpuBuffer,
        source_offset: u64,
        destination: &web_sys::GpuBuffer,
        destination_offset: u64,
        byte_len: u64,
    ) -> Result<()> {
        self.copy_gpu_buffer_named(
            "unattributed",
            source,
            source_offset,
            destination,
            destination_offset,
            byte_len,
        )
    }

    /// Copy a byte range between WebGPU buffers and attribute diagnostics to `name`.
    pub fn copy_gpu_buffer_named(
        &self,
        name: &'static str,
        source: &web_sys::GpuBuffer,
        source_offset: u64,
        destination: &web_sys::GpuBuffer,
        destination_offset: u64,
        byte_len: u64,
    ) -> Result<()> {
        let encoder = self.device.create_command_encoder();
        encoder
            .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                source,
                byte_offset_as_f64(source_offset)?,
                destination,
                byte_offset_as_f64(destination_offset)?,
                byte_len_as_f64(byte_len)?,
            )
            .map_err(js_error)?;
        self.submit(encoder.finish());
        self.diagnostics.record_device_copy(name, byte_len);
        Ok(())
    }

    /// Wait for all queued WebGPU work to complete.
    pub async fn wait_idle(&self) -> Result<()> {
        JsFuture::from(self.queue.on_submitted_work_done())
            .await
            .map_err(js_error)?;
        Ok(())
    }

    /// Copy a GPU buffer into WASM memory.
    pub async fn read_buffer(&self, source: &web_sys::GpuBuffer, byte_len: u64) -> Result<Vec<u8>> {
        self.read_buffer_range_named(source, 0, byte_len, "read_buffer")
            .await
    }

    /// Copy a byte range from a GPU buffer into WASM memory.
    pub async fn read_buffer_range(
        &self,
        source: &web_sys::GpuBuffer,
        source_offset: u64,
        byte_len: u64,
    ) -> Result<Vec<u8>> {
        self.read_buffer_range_named(source, source_offset, byte_len, "read_buffer_range")
            .await
    }

    /// Copy a byte range from a GPU buffer into WASM memory and attribute the
    /// readback to the source buffer's logical name.
    pub async fn read_buffer_range_named(
        &self,
        source: &web_sys::GpuBuffer,
        source_offset: u64,
        byte_len: u64,
        name: &'static str,
    ) -> Result<Vec<u8>> {
        let readback = self.create_buffer(
            "webgpu_readback",
            byte_len,
            WEBGPU_BUFFER_USAGE_MAP_READ | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;

        let encoder = self.device.create_command_encoder();
        let source_offset = byte_offset_as_f64(source_offset)?;
        let byte_len = byte_len_as_f64(byte_len)?;
        encoder
            .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                source,
                source_offset,
                &readback,
                0.0,
                byte_len,
            )
            .map_err(js_error)?;
        self.submit(encoder.finish());

        JsFuture::from(readback.map_async_with_f64_and_f64(WEBGPU_MAP_MODE_READ, 0.0, byte_len))
            .await
            .map_err(js_error)?;

        let mapped = readback
            .get_mapped_range_with_f64_and_f64(0.0, byte_len)
            .map_err(js_error)?;
        let bytes = js_sys::Uint8Array::new(&mapped).to_vec();
        readback.unmap();
        self.diagnostics.record_readback(name, bytes.len() as u64);
        Ok(bytes)
    }

    /// Copy fixed-size elements at arbitrary indices from a GPU buffer into
    /// contiguous WASM memory.
    pub async fn read_buffer_indices(
        &self,
        source: &web_sys::GpuBuffer,
        base_byte_offset: u64,
        elem_size: u64,
        indices: &[usize],
    ) -> Result<Vec<u8>> {
        self.read_buffer_indices_named(
            source,
            base_byte_offset,
            elem_size,
            indices,
            "indexed_readback",
        )
        .await
    }

    /// Copy fixed-size elements at arbitrary indices from several GPU buffers
    /// into one contiguous WASM allocation and one browser map operation.
    ///
    /// The output byte stream preserves group order, and preserves index order
    /// within each group.
    pub async fn read_buffer_index_groups_named(
        &self,
        groups: &[WebGpuIndexedReadbackGroup<'_>],
        name: &'static str,
    ) -> Result<Vec<u8>> {
        if groups.is_empty() {
            return Ok(Vec::new());
        }

        let mut elem_sizes = Vec::with_capacity(groups.len());
        let mut group_byte_lens = Vec::with_capacity(groups.len());
        let mut total_byte_len = 0usize;
        for group in groups {
            ensure!(
                group.elem_size > 0,
                "WebGPU indexed readback group element size is zero"
            );
            let elem_size = usize::try_from(group.elem_size)
                .map_err(|_| anyhow!("WebGPU indexed readback group element size exceeds usize"))?;
            let group_byte_len = group
                .indices
                .len()
                .checked_mul(elem_size)
                .ok_or_else(|| anyhow!("WebGPU indexed readback group length overflow"))?;
            total_byte_len = total_byte_len.checked_add(group_byte_len).ok_or_else(|| {
                anyhow!("WebGPU indexed readback groups combined length overflow")
            })?;
            elem_sizes.push(elem_size);
            group_byte_lens.push(group_byte_len);
        }

        if total_byte_len == 0 {
            return Ok(Vec::new());
        }

        let byte_len = total_byte_len
            .try_into()
            .map_err(|_| anyhow!("WebGPU indexed readback groups combined length exceeds u64"))?;
        let readback = self.create_buffer(
            "webgpu_indexed_group_readback",
            byte_len,
            WEBGPU_BUFFER_USAGE_MAP_READ | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;

        let encoder = self.device.create_command_encoder();
        let mut group_destination_base = 0usize;
        for ((group, elem_size), group_byte_len) in groups
            .iter()
            .zip(elem_sizes.iter().copied())
            .zip(group_byte_lens.iter().copied())
        {
            for (out_idx, source_idx) in group.indices.iter().copied().enumerate() {
                let indexed_byte_offset = u64::try_from(source_idx)
                    .ok()
                    .and_then(|idx| idx.checked_mul(group.elem_size))
                    .and_then(|offset| group.base_byte_offset.checked_add(offset))
                    .ok_or_else(|| {
                        anyhow!("WebGPU indexed readback group source offset overflow")
                    })?;
                let destination_byte_offset = out_idx
                    .checked_mul(elem_size)
                    .and_then(|offset| group_destination_base.checked_add(offset))
                    .and_then(|offset| offset.try_into().ok())
                    .ok_or_else(|| {
                        anyhow!("WebGPU indexed readback group destination offset overflow")
                    })?;
                encoder
                    .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                        group.source,
                        byte_offset_as_f64(indexed_byte_offset)?,
                        &readback,
                        byte_offset_as_f64(destination_byte_offset)?,
                        byte_len_as_f64(group.elem_size)?,
                    )
                    .map_err(js_error)?;
            }
            group_destination_base = group_destination_base
                .checked_add(group_byte_len)
                .ok_or_else(|| {
                    anyhow!("WebGPU indexed readback group destination base overflow")
                })?;
        }
        self.submit(encoder.finish());

        let byte_len_f64 = byte_len_as_f64(byte_len)?;
        JsFuture::from(readback.map_async_with_f64_and_f64(
            WEBGPU_MAP_MODE_READ,
            0.0,
            byte_len_f64,
        ))
        .await
        .map_err(js_error)?;

        let mapped = readback
            .get_mapped_range_with_f64_and_f64(0.0, byte_len_f64)
            .map_err(js_error)?;
        let bytes = js_sys::Uint8Array::new(&mapped).to_vec();
        readback.unmap();
        self.diagnostics.record_readback(name, bytes.len() as u64);
        Ok(bytes)
    }

    /// Copy fixed-size elements at arbitrary indices from a GPU buffer into
    /// contiguous WASM memory and attribute the readback to the source buffer's
    /// logical name.
    pub async fn read_buffer_indices_named(
        &self,
        source: &web_sys::GpuBuffer,
        base_byte_offset: u64,
        elem_size: u64,
        indices: &[usize],
        name: &'static str,
    ) -> Result<Vec<u8>> {
        if indices.is_empty() {
            return Ok(Vec::new());
        }
        ensure!(
            elem_size > 0,
            "WebGPU indexed readback element size is zero"
        );

        let byte_len = indices
            .len()
            .checked_mul(
                usize::try_from(elem_size)
                    .map_err(|_| anyhow!("WebGPU indexed readback element size exceeds usize"))?,
            )
            .and_then(|value| value.try_into().ok())
            .ok_or_else(|| anyhow!("WebGPU indexed readback length overflow"))?;
        let readback = self.create_buffer(
            "webgpu_indexed_readback",
            byte_len,
            WEBGPU_BUFFER_USAGE_MAP_READ | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;

        let encoder = self.device.create_command_encoder();
        for (out_idx, source_idx) in indices.iter().copied().enumerate() {
            let indexed_byte_offset = u64::try_from(source_idx)
                .ok()
                .and_then(|idx| idx.checked_mul(elem_size))
                .and_then(|offset| base_byte_offset.checked_add(offset))
                .ok_or_else(|| anyhow!("WebGPU indexed readback source offset overflow"))?;
            let destination_byte_offset = u64::try_from(out_idx)
                .ok()
                .and_then(|idx| idx.checked_mul(elem_size))
                .ok_or_else(|| anyhow!("WebGPU indexed readback destination offset overflow"))?;
            encoder
                .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                    source,
                    byte_offset_as_f64(indexed_byte_offset)?,
                    &readback,
                    byte_offset_as_f64(destination_byte_offset)?,
                    byte_len_as_f64(elem_size)?,
                )
                .map_err(js_error)?;
        }
        self.submit(encoder.finish());

        let byte_len_f64 = byte_len_as_f64(byte_len)?;
        JsFuture::from(readback.map_async_with_f64_and_f64(
            WEBGPU_MAP_MODE_READ,
            0.0,
            byte_len_f64,
        ))
        .await
        .map_err(js_error)?;

        let mapped = readback
            .get_mapped_range_with_f64_and_f64(0.0, byte_len_f64)
            .map_err(js_error)?;
        let bytes = js_sys::Uint8Array::new(&mapped).to_vec();
        readback.unmap();
        self.diagnostics.record_readback(name, bytes.len() as u64);
        Ok(bytes)
    }

    /// Copy contiguous ranges from several GPU buffers into one browser map
    /// operation. The output byte stream preserves input range order.
    pub async fn read_buffer_ranges_named(
        &self,
        ranges: &[(&web_sys::GpuBuffer, u64, u64)],
        name: &'static str,
    ) -> Result<Vec<u8>> {
        if ranges.is_empty() {
            return Ok(Vec::new());
        }

        let mut byte_len = 0u64;
        for (_, _, range_len) in ranges {
            byte_len = byte_len
                .checked_add(*range_len)
                .ok_or_else(|| anyhow!("WebGPU range readback combined length overflow"))?;
        }
        if byte_len == 0 {
            return Ok(Vec::new());
        }
        let readback = self.create_buffer(
            "webgpu_range_readback",
            byte_len,
            WEBGPU_BUFFER_USAGE_MAP_READ | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;

        let encoder = self.device.create_command_encoder();
        let mut destination_byte_offset = 0u64;
        for (source, source_byte_offset, range_len) in ranges {
            if *range_len == 0 {
                continue;
            }
            encoder
                .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                    source,
                    byte_offset_as_f64(*source_byte_offset)?,
                    &readback,
                    byte_offset_as_f64(destination_byte_offset)?,
                    byte_len_as_f64(*range_len)?,
                )
                .map_err(js_error)?;
            destination_byte_offset = destination_byte_offset
                .checked_add(*range_len)
                .ok_or_else(|| anyhow!("WebGPU range readback destination offset overflow"))?;
        }
        self.submit(encoder.finish());

        let byte_len_f64 = byte_len_as_f64(byte_len)?;
        JsFuture::from(readback.map_async_with_f64_and_f64(
            WEBGPU_MAP_MODE_READ,
            0.0,
            byte_len_f64,
        ))
        .await
        .map_err(js_error)?;

        let mapped = readback
            .get_mapped_range_with_f64_and_f64(0.0, byte_len_f64)
            .map_err(js_error)?;
        let bytes = js_sys::Uint8Array::new(&mapped).to_vec();
        readback.unmap();
        self.diagnostics.record_readback(name, bytes.len() as u64);
        Ok(bytes)
    }

    /// Copy fixed-size elements at arbitrary indices from two GPU buffers into
    /// one contiguous WASM allocation and one browser map operation.
    pub async fn read_two_buffer_indices_named(
        &self,
        first_source: &web_sys::GpuBuffer,
        first_base_byte_offset: u64,
        first_elem_size: u64,
        first_indices: &[usize],
        second_source: &web_sys::GpuBuffer,
        second_base_byte_offset: u64,
        second_elem_size: u64,
        second_indices: &[usize],
        name: &'static str,
    ) -> Result<Vec<u8>> {
        ensure!(
            first_elem_size > 0,
            "WebGPU indexed readback first element size is zero"
        );
        ensure!(
            second_elem_size > 0,
            "WebGPU indexed readback second element size is zero"
        );
        if first_indices.is_empty() && second_indices.is_empty() {
            return Ok(Vec::new());
        }

        let first_elem_size_usize = usize::try_from(first_elem_size)
            .map_err(|_| anyhow!("WebGPU indexed readback first element size exceeds usize"))?;
        let second_elem_size_usize = usize::try_from(second_elem_size)
            .map_err(|_| anyhow!("WebGPU indexed readback second element size exceeds usize"))?;
        let first_byte_len = first_indices
            .len()
            .checked_mul(first_elem_size_usize)
            .ok_or_else(|| anyhow!("WebGPU indexed readback first length overflow"))?;
        let second_byte_len = second_indices
            .len()
            .checked_mul(second_elem_size_usize)
            .ok_or_else(|| anyhow!("WebGPU indexed readback second length overflow"))?;
        let total_byte_len = first_byte_len
            .checked_add(second_byte_len)
            .ok_or_else(|| anyhow!("WebGPU indexed readback combined length overflow"))?;
        let byte_len = total_byte_len
            .try_into()
            .map_err(|_| anyhow!("WebGPU indexed readback combined length exceeds u64"))?;
        let readback = self.create_buffer(
            "webgpu_two_buffer_indexed_readback",
            byte_len,
            WEBGPU_BUFFER_USAGE_MAP_READ | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;

        let encoder = self.device.create_command_encoder();
        for (out_idx, source_idx) in first_indices.iter().copied().enumerate() {
            let indexed_byte_offset = u64::try_from(source_idx)
                .ok()
                .and_then(|idx| idx.checked_mul(first_elem_size))
                .and_then(|offset| first_base_byte_offset.checked_add(offset))
                .ok_or_else(|| anyhow!("WebGPU indexed readback first source offset overflow"))?;
            let destination_byte_offset = out_idx
                .checked_mul(first_elem_size_usize)
                .and_then(|offset| offset.try_into().ok())
                .ok_or_else(|| {
                    anyhow!("WebGPU indexed readback first destination offset overflow")
                })?;
            encoder
                .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                    first_source,
                    byte_offset_as_f64(indexed_byte_offset)?,
                    &readback,
                    byte_offset_as_f64(destination_byte_offset)?,
                    byte_len_as_f64(first_elem_size)?,
                )
                .map_err(js_error)?;
        }
        for (out_idx, source_idx) in second_indices.iter().copied().enumerate() {
            let indexed_byte_offset = u64::try_from(source_idx)
                .ok()
                .and_then(|idx| idx.checked_mul(second_elem_size))
                .and_then(|offset| second_base_byte_offset.checked_add(offset))
                .ok_or_else(|| anyhow!("WebGPU indexed readback second source offset overflow"))?;
            let destination_byte_offset = out_idx
                .checked_mul(second_elem_size_usize)
                .and_then(|offset| first_byte_len.checked_add(offset))
                .and_then(|offset| offset.try_into().ok())
                .ok_or_else(|| {
                    anyhow!("WebGPU indexed readback second destination offset overflow")
                })?;
            encoder
                .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                    second_source,
                    byte_offset_as_f64(indexed_byte_offset)?,
                    &readback,
                    byte_offset_as_f64(destination_byte_offset)?,
                    byte_len_as_f64(second_elem_size)?,
                )
                .map_err(js_error)?;
        }
        self.submit(encoder.finish());

        let byte_len_f64 = byte_len_as_f64(byte_len)?;
        JsFuture::from(readback.map_async_with_f64_and_f64(
            WEBGPU_MAP_MODE_READ,
            0.0,
            byte_len_f64,
        ))
        .await
        .map_err(js_error)?;

        let mapped = readback
            .get_mapped_range_with_f64_and_f64(0.0, byte_len_f64)
            .map_err(js_error)?;
        let bytes = js_sys::Uint8Array::new(&mapped).to_vec();
        readback.unmap();
        self.diagnostics.record_readback(name, bytes.len() as u64);
        Ok(bytes)
    }

    pub(crate) async fn read_gathered_elem_sample(
        &self,
        source: &web_sys::GpuBuffer,
        name: &'static str,
        source_elem_offset: usize,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> Result<Vec<BabyBearElem>> {
        if size == 0 {
            return Ok(Vec::new());
        }

        let byte_len = byte_len_for::<BabyBearElem>(size);
        let readback = self.create_buffer(
            "webgpu_gather_sample_readback",
            byte_len,
            WEBGPU_BUFFER_USAGE_MAP_READ | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;

        let encoder = self.device.create_command_encoder();
        let elem_bytes = byte_len_for::<BabyBearElem>(1);
        for out_idx in 0..size {
            let source_idx = source_elem_offset
                .checked_add(idx)
                .and_then(|base| {
                    out_idx
                        .checked_mul(stride)
                        .and_then(|offset| base.checked_add(offset))
                })
                .ok_or_else(|| anyhow!("WebGPU gather readback source offset overflow"))?;
            let source_offset = byte_offset_as_f64(byte_len_for::<BabyBearElem>(source_idx))?;
            let destination_offset = byte_offset_as_f64(byte_len_for::<BabyBearElem>(out_idx))?;
            encoder
                .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                    source,
                    source_offset,
                    &readback,
                    destination_offset,
                    byte_len_as_f64(elem_bytes)?,
                )
                .map_err(js_error)?;
        }
        self.submit(encoder.finish());

        JsFuture::from(readback.map_async_with_f64_and_f64(
            WEBGPU_MAP_MODE_READ,
            0.0,
            byte_len_as_f64(byte_len)?,
        ))
        .await
        .map_err(js_error)?;

        let mapped = readback
            .get_mapped_range_with_f64_and_f64(0.0, byte_len_as_f64(byte_len)?)
            .map_err(js_error)?;
        let bytes = js_sys::Uint8Array::new(&mapped).to_vec();
        readback.unmap();
        self.diagnostics.record_readback(name, bytes.len() as u64);

        let values = bytemuck::checked::try_cast_slice::<u8, BabyBearElem>(bytes.as_slice())
            .map_err(|err| anyhow!("invalid WebGPU gather readback: {err}"))?;
        ensure!(
            values.len() == size,
            "WebGPU gather readback size mismatch: got {} elems, expected {size}",
            values.len()
        );
        Ok(values.to_vec())
    }

    pub(crate) fn dispatch_zeroize_elem(&self, elems: &WebGpuBuffer<BabyBearElem>) -> Result<bool> {
        if elems.size() == 0 {
            return Ok(true);
        }
        if elems.byte_offset() != 0 {
            return Ok(false);
        }

        let Some(gpu) = elems.raw_buffer() else {
            return Ok(false);
        };

        match self.try_sparse_zeroize_upload(elems)? {
            SparseZeroizeUpload::Used {
                gpu_was_known_zero: true,
            } => {
                return Ok(true);
            }
            SparseZeroizeUpload::Used {
                gpu_was_known_zero: false,
            } => {}
            SparseZeroizeUpload::NotUsed => {
                elems.sync_cpu_to_gpu(self)?;
            }
        }

        let byte_len = byte_len_for::<BabyBearElem>(elems.size());
        // min_binding_size=0 keeps the
        // layout shape stable across byte_len-variant calls; the runtime
        // bind-validation still uses the actual buffer size at dispatch.
        let layout = self.create_bind_group_layout(
            "webgpu_zeroize_elem_layout",
            &[WebGpuBindingLayout::storage(0, 0)],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_zeroize_elem",
            ZEROIZE_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_zeroize_elem_bind_group",
            &layout,
            &[WebGpuBufferBinding {
                binding: 0,
                buffer: gpu,
                offset: elems.byte_offset(),
                size: Some(byte_len),
            }],
        )?;
        let workgroups = u32::try_from(elems.size())
            .expect("WebGPU zeroize element count exceeds u32")
            .div_ceil(WEBGPU_WORKGROUP_SIZE);
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
        Ok(true)
    }

    pub(crate) fn try_sparse_zeroize_upload(
        &self,
        elems: &WebGpuBuffer<BabyBearElem>,
    ) -> Result<SparseZeroizeUpload> {
        if !matches!(
            elems.name(),
            "data" | "recursion_data" | "keccak_data" | "accum"
        ) || !elems.cpu_dirty.get()
            || elems.cpu_stale.get()
        {
            return Ok(SparseZeroizeUpload::NotUsed);
        }
        let Some(gpu) = elems.raw_buffer() else {
            return Ok(SparseZeroizeUpload::NotUsed);
        };
        let gpu_was_known_zero = elems.gpu_known_zero();

        let dense_byte_len = byte_len_for::<BabyBearElem>(elems.size());
        let mut plan = Ok(SparseUploadPlan::new());
        elems.cpu.view(|cpu| {
            plan = build_sparse_zero_upload_plan(cpu);
        });
        let plan = plan?;

        if plan.values.is_empty() {
            log_webgpu_metric(&format!(
                "webgpu_zeroize_sparse_upload name={} values=0 ranges=0 sparse_bytes=0 dense_bytes={}",
                elems.name(),
                dense_byte_len,
            ));
            return Ok(SparseZeroizeUpload::Used { gpu_was_known_zero });
        }

        let value_byte_len = byte_len_for::<BabyBearElem>(plan.values.len());
        let range_byte_len = byte_len_for::<u32>(plan.ranges.len());
        let sparse_byte_len = value_byte_len
            .checked_add(range_byte_len)
            .and_then(|len| len.checked_add(16))
            .ok_or_else(|| anyhow!("sparse zeroize upload byte length overflow"))?;
        if sparse_byte_len >= dense_byte_len || sparse_byte_len > self.max_storage_binding_bytes() {
            log_webgpu_metric(&format!(
                "webgpu_zeroize_sparse_upload name={} fallback values={} ranges={} sparse_bytes={} dense_bytes={} max_binding={}",
                elems.name(),
                plan.values.len(),
                plan.ranges.len() / 2,
                sparse_byte_len,
                dense_byte_len,
                self.max_storage_binding_bytes(),
            ));
            return Ok(SparseZeroizeUpload::NotUsed);
        }

        dispatch_sparse_zero_upload(
            self,
            gpu,
            elems.byte_offset(),
            dense_byte_len,
            &plan,
            "webgpu_zeroize_sparse_values",
            "webgpu_zeroize_sparse_ranges",
            "webgpu_zeroize_sparse_params",
            "webgpu_zeroize_sparse_upload_layout",
            "webgpu_zeroize_sparse_upload",
            "webgpu_zeroize_sparse_upload_bind_group",
        )?;
        log_webgpu_metric(&format!(
            "webgpu_zeroize_sparse_upload name={} values={} ranges={} sparse_bytes={} dense_bytes={}",
            elems.name(),
            plan.values.len(),
            plan.ranges.len() / 2,
            sparse_byte_len,
            dense_byte_len,
        ));
        elems.mark_gpu_may_be_nonzero();
        Ok(SparseZeroizeUpload::Used { gpu_was_known_zero })
    }

    pub(crate) fn dispatch_fill_invalid_elem(
        &self,
        elems: &WebGpuBuffer<BabyBearElem>,
    ) -> Result<bool> {
        if elems.size() == 0 {
            return Ok(true);
        }
        if elems.byte_offset() != 0 {
            return Ok(false);
        }

        let Some(gpu) = elems.raw_buffer() else {
            return Ok(false);
        };

        let byte_len = byte_len_for::<BabyBearElem>(elems.size());
        let layout = self.create_bind_group_layout(
            "webgpu_fill_invalid_elem_layout",
            &[WebGpuBindingLayout::storage(0, 0)],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_fill_invalid_elem",
            FILL_INVALID_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_fill_invalid_elem_bind_group",
            &layout,
            &[WebGpuBufferBinding {
                binding: 0,
                buffer: gpu,
                offset: elems.byte_offset(),
                size: Some(byte_len),
            }],
        )?;
        let workgroups = u32::try_from(elems.size())
            .expect("WebGPU fill-invalid element count exceeds u32")
            .div_ceil(WEBGPU_WORKGROUP_SIZE);
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
        Ok(true)
    }

    pub fn init_invalid_elem(&self, into: &WebGpuBuffer<BabyBearElem>) -> Result<bool> {
        let gpu_filled = self.dispatch_fill_invalid_elem(into)?;
        if !gpu_filled {
            return Ok(false);
        }

        self.diagnostics
            .record_gpu_dispatch("witgen_data_invalid_fill");
        into.mark_synced();
        Ok(true)
    }

    pub(crate) fn dispatch_eltwise_add_elem(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input1: &WebGpuBuffer<BabyBearElem>,
        input2: &WebGpuBuffer<BabyBearElem>,
    ) -> Result<bool> {
        if output.size() != input1.size() || output.size() != input2.size() {
            return Ok(false);
        }
        if output.size() == 0 {
            return Ok(true);
        }
        if output.byte_offset() != 0 || input1.byte_offset() != 0 || input2.byte_offset() != 0 {
            return Ok(false);
        }

        let (Some(output_gpu), Some(input1_gpu), Some(input2_gpu)) = (
            output.raw_buffer(),
            input1.raw_buffer(),
            input2.raw_buffer(),
        ) else {
            return Ok(false);
        };

        if output_gpu == input1_gpu || output_gpu == input2_gpu {
            return Ok(false);
        }

        input1.sync_cpu_to_gpu(self)?;
        input2.sync_cpu_to_gpu(self)?;

        let byte_len = byte_len_for::<BabyBearElem>(output.size());
        // Stable layout shape via min_binding_size=0.
        let layout = self.create_bind_group_layout(
            "webgpu_eltwise_add_elem_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_eltwise_add_elem",
            ELTWISE_ADD_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_eltwise_add_elem_bind_group",
            &layout,
            &[
                WebGpuBufferBinding {
                    binding: 0,
                    buffer: output_gpu,
                    offset: output.byte_offset(),
                    size: Some(byte_len),
                },
                WebGpuBufferBinding {
                    binding: 1,
                    buffer: input1_gpu,
                    offset: input1.byte_offset(),
                    size: Some(byte_len),
                },
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: input2_gpu,
                    offset: input2.byte_offset(),
                    size: Some(byte_len),
                },
            ],
        )?;
        let workgroups = u32::try_from(output.size())
            .expect("WebGPU element count exceeds u32")
            .div_ceil(WEBGPU_WORKGROUP_SIZE);
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
        Ok(true)
    }

    pub(crate) fn dispatch_eltwise_sum_extelem(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearExtElem>,
    ) -> Result<bool> {
        if output.size() % BabyBearExtElem::EXT_SIZE != 0 {
            return Ok(false);
        }
        if output.size() == 0 {
            return Ok(true);
        }
        if output.byte_offset() != 0 || input.byte_offset() != 0 {
            return Ok(false);
        }

        let (Some(output_gpu), Some(input_gpu)) = (output.raw_buffer(), input.raw_buffer()) else {
            return Ok(false);
        };
        if !self.storage_binding_fits(output) || !self.storage_binding_fits(input) {
            return Ok(false);
        }

        input.sync_cpu_to_gpu(self)?;

        let count = output.size() / BabyBearExtElem::EXT_SIZE;
        if count == 0 || input.size() % count != 0 {
            return Ok(false);
        }
        let to_add = input.size() / count;
        let params = [
            u32::try_from(count).expect("WebGPU eltwise_sum count exceeds u32"),
            u32::try_from(to_add).expect("WebGPU eltwise_sum to_add exceeds u32"),
            0,
            0,
        ];
        let params = self.create_uniform_buffer(
            "webgpu_eltwise_sum_extelem_params",
            bytemuck::cast_slice(&params),
        )?;

        let output_byte_len = byte_len_for::<BabyBearElem>(output.size());
        let input_byte_len = byte_len_for::<BabyBearExtElem>(input.size());
        // Stable layout shape via min_binding_size=0.
        let layout = self.create_bind_group_layout(
            "webgpu_eltwise_sum_extelem_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 16),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_eltwise_sum_extelem",
            ELTWISE_SUM_EXTELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_eltwise_sum_extelem_bind_group",
            &layout,
            &[
                WebGpuBufferBinding {
                    binding: 0,
                    buffer: output_gpu,
                    offset: output.byte_offset(),
                    size: Some(output_byte_len),
                },
                WebGpuBufferBinding {
                    binding: 1,
                    buffer: input_gpu,
                    offset: input.byte_offset(),
                    size: Some(input_byte_len),
                },
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: &params,
                    offset: 0,
                    size: Some(16),
                },
            ],
        )?;
        let workgroups = u32::try_from(count)
            .expect("WebGPU eltwise_sum count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_eltwise_copy_elem_slice(
        &self,
        into: &WebGpuBuffer<BabyBearElem>,
        from: &[BabyBearElem],
        from_rows: usize,
        from_cols: usize,
        from_offset: usize,
        from_stride: usize,
        into_offset: usize,
        into_stride: usize,
    ) -> Result<bool> {
        let Some(total) = from_rows.checked_mul(from_cols) else {
            return Ok(false);
        };
        if total == 0 {
            return Ok(false);
        }
        if into.byte_offset() != 0 {
            return Ok(false);
        }

        if !slice_region_in_bounds(from.len(), from_rows, from_cols, from_offset, from_stride)
            || !slice_region_in_bounds(into.size(), from_rows, from_cols, into_offset, into_stride)
        {
            return Ok(false);
        }

        let Some(into_gpu) = into.raw_buffer() else {
            return Ok(false);
        };
        let sparse_primed_for_zeroize = matches!(into.name(), "recursion_data" | "accum")
            && matches!(
                self.try_sparse_zeroize_upload(into)?,
                SparseZeroizeUpload::Used { .. }
            );
        if !sparse_primed_for_zeroize {
            into.sync_cpu_to_gpu(self)?;
        }

        let from_buf = self.copy_from_elem("webgpu_eltwise_copy_elem_slice_from", from);
        let Some(from_gpu) = from_buf.raw_buffer() else {
            return Ok(false);
        };

        let encoder = self.device.create_command_encoder();
        let row_byte_len = byte_len_for::<BabyBearElem>(from_cols);
        for row in 0..from_rows {
            let source_idx = from_offset
                .checked_add(
                    row.checked_mul(from_stride)
                        .ok_or_else(|| anyhow!("WebGPU copy slice source row overflow"))?,
                )
                .ok_or_else(|| anyhow!("WebGPU copy slice source offset overflow"))?;
            let destination_idx = into_offset
                .checked_add(
                    row.checked_mul(into_stride)
                        .ok_or_else(|| anyhow!("WebGPU copy slice destination row overflow"))?,
                )
                .ok_or_else(|| anyhow!("WebGPU copy slice destination offset overflow"))?;
            encoder
                .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                    from_gpu,
                    byte_offset_as_f64(byte_len_for::<BabyBearElem>(source_idx))?,
                    into_gpu,
                    byte_offset_as_f64(byte_len_for::<BabyBearElem>(destination_idx))?,
                    byte_len_as_f64(row_byte_len)?,
                )
                .map_err(js_error)?;
        }
        self.submit(encoder.finish());
        self.diagnostics
            .record_device_copy(into.cpu.name(), byte_len_for::<BabyBearElem>(total));
        Ok(true)
    }

    pub(crate) fn dispatch_scatter(
        &self,
        into: &WebGpuBuffer<BabyBearElem>,
        index: &[u32],
        offsets: &[u32],
        values: &[BabyBearElem],
        sync_destination: bool,
    ) -> Result<bool> {
        if index.len() < 2 {
            return Ok(false);
        }
        if into.byte_offset() != 0 {
            return Ok(false);
        }

        let start = index[0] as usize;
        let end = *index.last().expect("index has at least two entries") as usize;
        if end <= start {
            return Ok(false);
        }
        if end > offsets.len() || end > values.len() {
            return Ok(false);
        }
        if !index.windows(2).all(|window| window[0] <= window[1]) {
            return Ok(false);
        }

        let mut seen_offsets = HashSet::with_capacity(end - start);
        for &offset in &offsets[start..end] {
            let offset = offset as usize;
            if offset >= into.size() || !seen_offsets.insert(offset) {
                return Ok(false);
            }
        }

        let Some(into_gpu) = into.raw_buffer() else {
            return Ok(false);
        };
        if sync_destination {
            into.sync_cpu_to_gpu(self)?;
        }

        let offsets = self.copy_from_u32("webgpu_scatter_offsets", offsets);
        let values = self.copy_from_elem("webgpu_scatter_values", values);
        let (Some(offsets_gpu), Some(values_gpu)) = (offsets.raw_buffer(), values.raw_buffer())
        else {
            return Ok(false);
        };

        let count = end - start;
        let params = [
            u32::try_from(start).expect("WebGPU scatter start exceeds u32"),
            u32::try_from(count).expect("WebGPU scatter count exceeds u32"),
            0,
            0,
        ];
        let params =
            self.create_uniform_buffer("webgpu_scatter_params", bytemuck::cast_slice(&params))?;

        let into_byte_len = byte_len_for::<BabyBearElem>(into.size());
        let offsets_byte_len = byte_len_for::<u32>(offsets.size());
        let values_byte_len = byte_len_for::<BabyBearElem>(values.size());
        // Stable layout shape via min_binding_size=0.
        let layout = self.create_bind_group_layout(
            "webgpu_scatter_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::uniform(3, 16),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_scatter",
            SCATTER_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_scatter_bind_group",
            &layout,
            &[
                WebGpuBufferBinding {
                    binding: 0,
                    buffer: into_gpu,
                    offset: 0,
                    size: Some(into_byte_len),
                },
                WebGpuBufferBinding {
                    binding: 1,
                    buffer: offsets_gpu,
                    offset: 0,
                    size: Some(offsets_byte_len),
                },
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: values_gpu,
                    offset: 0,
                    size: Some(values_byte_len),
                },
                WebGpuBufferBinding {
                    binding: 3,
                    buffer: &params,
                    offset: 0,
                    size: Some(16),
                },
            ],
        )?;
        let workgroups = u32::try_from(count)
            .expect("WebGPU scatter count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    pub(crate) fn dispatch_gather_sample(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src: &WebGpuBuffer<BabyBearElem>,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> Result<bool> {
        if size == 0 {
            return Ok(false);
        }
        if size > dst.size() || !gather_region_in_bounds(src.size(), idx, size, stride) {
            return Ok(false);
        }

        let (Some(dst_gpu), Some(src_gpu)) = (dst.raw_buffer(), src.raw_buffer()) else {
            return Ok(false);
        };
        if dst_gpu == src_gpu {
            return Ok(false);
        }
        if !self.storage_binding_fits(dst) || !self.storage_binding_fits(src) {
            return Ok(false);
        }

        src.sync_cpu_to_gpu(self)?;
        if size < dst.size() {
            dst.sync_cpu_to_gpu(self)?;
        }

        let params = [
            u32::try_from(dst.elem_offset).expect("WebGPU gather dst offset exceeds u32"),
            u32::try_from(src.elem_offset).expect("WebGPU gather src offset exceeds u32"),
            u32::try_from(idx).expect("WebGPU gather idx exceeds u32"),
            u32::try_from(size).expect("WebGPU gather size exceeds u32"),
            u32::try_from(stride).expect("WebGPU gather stride exceeds u32"),
            0,
            0,
            0,
        ];
        let params = self
            .create_uniform_buffer("webgpu_gather_sample_params", bytemuck::cast_slice(&params))?;

        let layout = self.create_bind_group_layout(
            "webgpu_gather_sample_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_gather_sample",
            GATHER_SAMPLE_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_gather_sample_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, dst_gpu),
                WebGpuBufferBinding::new(1, src_gpu),
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: &params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        let workgroups = u32::try_from(size)
            .expect("WebGPU gather size exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_gather_sample_chunked(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src: &WebGpuBuffer<BabyBearElem>,
        dst_gpu: &web_sys::GpuBuffer,
        src_gpu: &web_sys::GpuBuffer,
        idx: usize,
        size: usize,
        stride: usize,
        chunk_cols: usize,
    ) -> Result<()> {
        let layout = self.create_bind_group_layout(
            "webgpu_gather_sample_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_gather_sample",
            GATHER_SAMPLE_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;

        let mut params_buffers = Vec::new();
        let mut bind_groups = Vec::new();
        for col_start in (0..size).step_by(chunk_cols) {
            let cols = chunk_cols.min(size - col_start);
            let src_elem_offset = src
                .elem_offset
                .checked_add(
                    col_start
                        .checked_mul(stride)
                        .ok_or_else(|| anyhow!("WebGPU gather chunk offset overflow"))?,
                )
                .ok_or_else(|| anyhow!("WebGPU gather chunk offset overflow"))?;
            let src_byte_offset = byte_len_for::<BabyBearElem>(src_elem_offset);
            let src_binding_offset = src_byte_offset / WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT
                * WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT;
            let src_binding_base_bytes = src_byte_offset - src_binding_offset;
            let src_binding_base_elems =
                usize::try_from(src_binding_base_bytes / mem::size_of::<BabyBearElem>() as u64)
                    .expect("WebGPU gather chunk binding offset exceeds usize");
            let src_chunk_elems = cols
                .checked_sub(1)
                .and_then(|last_col| last_col.checked_mul(stride))
                .and_then(|last_col_start| last_col_start.checked_add(idx))
                .and_then(|last_idx| last_idx.checked_add(1))
                .ok_or_else(|| anyhow!("WebGPU gather chunk size overflow"))?;
            let src_chunk_bytes = src_binding_base_bytes
                .checked_add(byte_len_for::<BabyBearElem>(src_chunk_elems))
                .ok_or_else(|| anyhow!("WebGPU gather chunk binding size overflow"))?;
            ensure!(
                src_chunk_bytes <= WEBGPU_SAFE_STORAGE_BINDING_BYTES,
                "WebGPU gather chunk binding size exceeds safe WebGPU storage binding limit"
            );

            let params = [
                u32::try_from(dst.elem_offset + col_start)
                    .expect("WebGPU gather dst offset exceeds u32"),
                u32::try_from(src_binding_base_elems)
                    .expect("WebGPU gather source base exceeds u32"),
                u32::try_from(idx).expect("WebGPU gather idx exceeds u32"),
                u32::try_from(cols).expect("WebGPU gather chunk size exceeds u32"),
                u32::try_from(stride).expect("WebGPU gather stride exceeds u32"),
                0,
                0,
                0,
            ];
            let params = self.create_uniform_buffer(
                "webgpu_gather_sample_params",
                bytemuck::cast_slice(&params),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_gather_sample_bind_group",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, dst_gpu),
                    WebGpuBufferBinding {
                        binding: 1,
                        buffer: src_gpu,
                        offset: src_binding_offset,
                        size: Some(src_chunk_bytes),
                    },
                    WebGpuBufferBinding {
                        binding: 2,
                        buffer: &params,
                        offset: 0,
                        size: Some(32),
                    },
                ],
            )?;
            let workgroups = u32::try_from(cols)
                .expect("WebGPU gather chunk size exceeds u32")
                .div_ceil(WEBGPU_WORKGROUP_SIZE);
            self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
            params_buffers.push(params);
            bind_groups.push(bind_group);
        }

        drop(bind_groups);
        drop(params_buffers);
        Ok(())
    }

    /// Tiled gather variant operating over a
    /// `BufferPool`. Each tile holds a contiguous column-slab of the
    /// logical 2D source; the kernel runs per-tile with the same
    /// `GATHER_SAMPLE_ELEM_WGSL` shader as `dispatch_gather_sample`,
    /// but binding the per-tile buffer instead of a sub-range of one
    /// oversize buffer. Replaces the locked CPU fallback in
    /// `gather_sample_async` that previously fired when the source
    /// exceeded `maxStorageBufferBindingSize`.
    ///
    /// `idx`, `size`, and `stride` follow the same contract as
    /// `gather_sample`: read `pool[col * stride + idx]` into `dst[col]`
    /// for `col` in `[0, size)`. `size` must equal `pool.layout.total_cols`
    /// and `stride` must equal `pool.layout.stride`; the layout's
    /// `tile_cols` is what we iterate over.
    pub(crate) fn dispatch_gather_sample_tiled(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src_pool: &buffer_pool::BufferPool,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> Result<()> {
        ensure!(
            stride == src_pool.layout.stride,
            "dispatch_gather_sample_tiled: stride mismatch (caller={stride}, pool={})",
            src_pool.layout.stride
        );
        ensure!(
            size == src_pool.layout.total_cols,
            "dispatch_gather_sample_tiled: size mismatch (caller={size}, pool.total_cols={})",
            src_pool.layout.total_cols
        );
        ensure!(
            size <= dst.size(),
            "dispatch_gather_sample_tiled: dst capacity {} < size {size}",
            dst.size()
        );
        if size == 0 {
            return Ok(());
        }
        let dst_gpu = dst
            .raw_buffer()
            .ok_or_else(|| anyhow!("dispatch_gather_sample_tiled: dst has no GPU backing"))?;
        ensure!(
            self.storage_binding_fits(dst),
            "dispatch_gather_sample_tiled: dst exceeds max storage binding"
        );
        if size < dst.size() {
            dst.sync_cpu_to_gpu(self)?;
        }

        let layout = self.create_bind_group_layout(
            "webgpu_gather_sample_tiled_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_gather_sample_tiled",
            GATHER_SAMPLE_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;

        // Hold per-tile params + bind group references until submit
        // completes (we issue one dispatch per tile inside this fn).
        let mut params_buffers: Vec<web_sys::GpuBuffer> = Vec::with_capacity(src_pool.num_tiles());
        let mut bind_groups: Vec<web_sys::GpuBindGroup> = Vec::with_capacity(src_pool.num_tiles());

        for tile_idx in 0..src_pool.num_tiles() {
            let cols = src_pool.layout.cols_in_tile(tile_idx);
            if cols == 0 {
                continue;
            }
            let col_start = tile_idx * src_pool.layout.tile_cols;
            // Per-tile params: dst_base advances by col_start; src_base
            // is 0 because each tile buffer starts at the first column
            // it owns (no sub-range offset inside the buffer).
            let params = [
                u32::try_from(dst.elem_offset + col_start)
                    .expect("WebGPU gather dst offset exceeds u32"),
                0u32,
                u32::try_from(idx).expect("WebGPU gather idx exceeds u32"),
                u32::try_from(cols).expect("WebGPU gather tile cols exceeds u32"),
                u32::try_from(stride).expect("WebGPU gather stride exceeds u32"),
                0,
                0,
                0,
            ];
            let params_buf = self.create_uniform_buffer(
                "webgpu_gather_sample_tiled_params",
                bytemuck::cast_slice(&params),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_gather_sample_tiled_bind_group",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, dst_gpu),
                    WebGpuBufferBinding::new(1, src_pool.tile_buffer(tile_idx)),
                    WebGpuBufferBinding {
                        binding: 2,
                        buffer: &params_buf,
                        offset: 0,
                        size: Some(32),
                    },
                ],
            )?;
            let workgroups = u32::try_from(cols)
                .expect("WebGPU gather tile cols exceeds u32")
                .div_ceil(WEBGPU_WORKGROUP_SIZE);
            self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
            params_buffers.push(params_buf);
            bind_groups.push(bind_group);
        }

        drop(bind_groups);
        drop(params_buffers);
        Ok(())
    }

    pub(crate) fn dispatch_prefix_products(
        &self,
        io: &WebGpuBuffer<BabyBearExtElem>,
    ) -> Result<bool> {
        if io.size() == 0 {
            return Ok(true);
        }

        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };
        io.sync_cpu_to_gpu(self)?;

        let byte_len = byte_len_for::<BabyBearExtElem>(io.size());
        // Stable layout shape via min_binding_size=0.
        let layout = self.create_bind_group_layout(
            "webgpu_prefix_products_extelem_layout",
            &[WebGpuBindingLayout::storage(0, 0)],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_prefix_products_extelem",
            PREFIX_PRODUCTS_EXTELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_prefix_products_extelem_bind_group",
            &layout,
            &[WebGpuBufferBinding {
                binding: 0,
                buffer: io_gpu,
                offset: io.byte_offset(),
                size: Some(byte_len),
            }],
        )?;
        self.dispatch_compute(&kernel, &bind_group, 1, 1, 1);
        Ok(true)
    }

    pub(crate) fn dispatch_fri_fold(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
        mix: &BabyBearExtElem,
    ) -> Result<bool> {
        let count = output.size() / BabyBearExtElem::EXT_SIZE;
        if count == 0 {
            return Ok(true);
        }

        let (Some(output_gpu), Some(input_gpu)) = (output.raw_buffer(), input.raw_buffer()) else {
            return Ok(false);
        };
        if !self.storage_binding_fits(output) || !self.storage_binding_fits(input) {
            return Ok(false);
        }

        let mix = mix.subelems();
        let params = [
            u32::try_from(count).expect("WebGPU fri_fold count exceeds u32"),
            u32::try_from(output.elem_offset).expect("WebGPU fri_fold output offset exceeds u32"),
            u32::try_from(input.elem_offset).expect("WebGPU fri_fold input offset exceeds u32"),
            0,
            mix[0].as_u32_montgomery(),
            mix[1].as_u32_montgomery(),
            mix[2].as_u32_montgomery(),
            mix[3].as_u32_montgomery(),
        ];
        let params =
            self.create_uniform_buffer("webgpu_fri_fold_params", bytemuck::cast_slice(&params))?;

        input.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_fri_fold_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_fri_fold",
            FRI_FOLD_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_fri_fold_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, output_gpu),
                WebGpuBufferBinding::new(1, input_gpu),
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: &params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        let workgroups = u32::try_from(count)
            .expect("WebGPU fri_fold count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    pub(crate) fn dispatch_zk_shift(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        bits: usize,
    ) -> Result<bool> {
        if !self.zk_shift_gpu_enabled.get() {
            return Ok(false);
        }
        if io.size() == 0 {
            return Ok(true);
        }

        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(io) {
            return Ok(false);
        }

        let params = [
            u32::try_from(io.size()).expect("WebGPU zk_shift count exceeds u32"),
            u32::try_from(bits).expect("WebGPU zk_shift bits exceeds u32"),
            u32::try_from(io.elem_offset).expect("WebGPU zk_shift offset exceeds u32"),
            0,
        ];
        let params =
            self.create_uniform_buffer("webgpu_zk_shift_params", bytemuck::cast_slice(&params))?;

        io.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_zk_shift_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::uniform(1, 16),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_zk_shift",
            ZK_SHIFT_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_zk_shift_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, io_gpu),
                WebGpuBufferBinding {
                    binding: 1,
                    buffer: &params,
                    offset: 0,
                    size: Some(16),
                },
            ],
        )?;
        let workgroups = u32::try_from(io.size())
            .expect("WebGPU zk_shift count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_mix_poly_coeffs(
        &self,
        output: &WebGpuBuffer<BabyBearExtElem>,
        mix_start: &BabyBearExtElem,
        mix: &BabyBearExtElem,
        input: &WebGpuBuffer<BabyBearElem>,
        combos: &WebGpuBuffer<u32>,
        input_size: usize,
        count: usize,
    ) -> Result<bool> {
        self.dispatch_mix_poly_coeffs_inner(
            output, mix_start, mix, input, combos, input_size, count, false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_mix_poly_coeffs_inner(
        &self,
        output: &WebGpuBuffer<BabyBearExtElem>,
        mix_start: &BabyBearExtElem,
        mix: &BabyBearExtElem,
        input: &WebGpuBuffer<BabyBearElem>,
        combos: &WebGpuBuffer<u32>,
        input_size: usize,
        count: usize,
        allow_gpu_authoritative: bool,
    ) -> Result<bool> {
        if count == 0 {
            return Ok(true);
        }
        if self.gpu_authoritative() && !allow_gpu_authoritative {
            return Ok(false);
        }

        let (Some(output_gpu), Some(input_gpu), Some(combos_gpu)) =
            (output.raw_buffer(), input.raw_buffer(), combos.raw_buffer())
        else {
            return Ok(false);
        };
        if !self.storage_binding_fits(output)
            || !self.storage_binding_fits(input)
            || !self.storage_binding_fits(combos)
        {
            return Ok(false);
        }

        let Some(output_base) = output
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
        else {
            return Err(anyhow!("WebGPU mix_poly_coeffs output offset exceeds u32"));
        };
        let mix_start = mix_start.subelems();
        let mix = mix.subelems();
        let params = [
            u32::try_from(input_size).expect("WebGPU mix_poly_coeffs input_size exceeds u32"),
            u32::try_from(count).expect("WebGPU mix_poly_coeffs count exceeds u32"),
            output_base,
            u32::try_from(input.elem_offset)
                .expect("WebGPU mix_poly_coeffs input offset exceeds u32"),
            u32::try_from(combos.elem_offset)
                .expect("WebGPU mix_poly_coeffs combos offset exceeds u32"),
            0,
            0,
            0,
            mix_start[0].as_u32_montgomery(),
            mix_start[1].as_u32_montgomery(),
            mix_start[2].as_u32_montgomery(),
            mix_start[3].as_u32_montgomery(),
            mix[0].as_u32_montgomery(),
            mix[1].as_u32_montgomery(),
            mix[2].as_u32_montgomery(),
            mix[3].as_u32_montgomery(),
        ];
        let params = self.create_uniform_buffer(
            "webgpu_mix_poly_coeffs_params",
            bytemuck::cast_slice(&params),
        )?;

        output.sync_cpu_to_gpu(self)?;
        input.sync_cpu_to_gpu(self)?;
        combos.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_mix_poly_coeffs_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::uniform(3, 64),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_mix_poly_coeffs",
            MIX_POLY_COEFFS_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_mix_poly_coeffs_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, output_gpu),
                WebGpuBufferBinding::new(1, input_gpu),
                WebGpuBufferBinding::new(2, combos_gpu),
                WebGpuBufferBinding {
                    binding: 3,
                    buffer: &params,
                    offset: 0,
                    size: Some(64),
                },
            ],
        )?;
        let workgroups = u32::try_from(count)
            .expect("WebGPU mix_poly_coeffs count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }
}
