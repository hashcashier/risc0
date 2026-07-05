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

//! Kernel dispatchers for the field/NTT/Poseidon2 operations and the
//! synchronous `Hal` trait implementation.

use super::*;
#[allow(unused_imports)]
use super::{device::*, diagnostics::*, eval_check::*, kernels_wgsl::*, ops::*, resources::*};

impl WebGpuHal {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_mix_poly_coeffs_chunked(
        &self,
        output: &WebGpuBuffer<BabyBearExtElem>,
        mix_start: &BabyBearExtElem,
        mix: &BabyBearExtElem,
        input: &WebGpuBuffer<BabyBearElem>,
        combos: &WebGpuBuffer<u32>,
        input_size: usize,
        count: usize,
    ) -> Result<bool> {
        if count == 0 || input_size == 0 {
            return Ok(true);
        }
        let (Some(output_gpu), Some(input_gpu), Some(combos_gpu)) =
            (output.raw_buffer(), input.raw_buffer(), combos.raw_buffer())
        else {
            return Ok(false);
        };
        if !self.storage_binding_fits(output) || !self.storage_binding_fits(combos) {
            return Ok(false);
        }

        let input_poly_bytes = byte_len_for::<BabyBearElem>(count);
        if input_poly_bytes == 0
            || input_poly_bytes > self.max_storage_binding_bytes()
            || input.byte_offset() % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0
            || input_poly_bytes % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0
        {
            return Ok(false);
        }
        let max_polys_per_chunk =
            (self.max_storage_binding_bytes() / input_poly_bytes).max(1) as usize;
        if max_polys_per_chunk == 0 {
            return Ok(false);
        }

        let Some(output_base) = output
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
        else {
            return Err(anyhow!("WebGPU mix_poly_coeffs output offset exceeds u32"));
        };
        let combos_base = u32::try_from(combos.elem_offset)
            .expect("WebGPU mix_poly_coeffs combos offset exceeds u32");

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

        let mix_words = mix.subelems();
        for chunk_start in (0..input_size).step_by(max_polys_per_chunk) {
            let chunk_size = (input_size - chunk_start).min(max_polys_per_chunk);
            let chunk_input_bytes = input_poly_bytes
                .checked_mul(u64::try_from(chunk_size).expect("WebGPU mix chunk size exceeds u64"))
                .ok_or_else(|| anyhow!("WebGPU mix_poly_coeffs chunk byte length overflow"))?;
            let input_offset = input
                .byte_offset()
                .checked_add(
                    u64::try_from(chunk_start)
                        .ok()
                        .and_then(|start| start.checked_mul(input_poly_bytes))
                        .ok_or_else(|| anyhow!("WebGPU mix_poly_coeffs input offset overflow"))?,
                )
                .ok_or_else(|| anyhow!("WebGPU mix_poly_coeffs input offset overflow"))?;
            let chunk_mix_start = *mix_start * mix.pow(chunk_start);
            let chunk_mix_start = chunk_mix_start.subelems();
            let params = [
                u32::try_from(chunk_size).expect("WebGPU mix_poly_coeffs input_size exceeds u32"),
                u32::try_from(count).expect("WebGPU mix_poly_coeffs count exceeds u32"),
                output_base,
                0,
                combos_base
                    .checked_add(
                        u32::try_from(chunk_start)
                            .expect("WebGPU mix_poly_coeffs combo chunk offset exceeds u32"),
                    )
                    .ok_or_else(|| anyhow!("WebGPU mix_poly_coeffs combo offset overflow"))?,
                0,
                0,
                0,
                chunk_mix_start[0].as_u32_montgomery(),
                chunk_mix_start[1].as_u32_montgomery(),
                chunk_mix_start[2].as_u32_montgomery(),
                chunk_mix_start[3].as_u32_montgomery(),
                mix_words[0].as_u32_montgomery(),
                mix_words[1].as_u32_montgomery(),
                mix_words[2].as_u32_montgomery(),
                mix_words[3].as_u32_montgomery(),
            ];
            let params = self.create_uniform_buffer(
                "webgpu_mix_poly_coeffs_params",
                bytemuck::cast_slice(&params),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_mix_poly_coeffs_bind_group",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, output_gpu),
                    WebGpuBufferBinding {
                        binding: 1,
                        buffer: input_gpu,
                        offset: input_offset,
                        size: Some(chunk_input_bytes),
                    },
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
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_combos_prepare(
        &self,
        combos: &WebGpuBuffer<BabyBearExtElem>,
        coeff_u: &[BabyBearExtElem],
        combo_count: usize,
        cycles: usize,
        reg_sizes: &[u32],
        reg_combo_ids: &[u32],
        mix: &BabyBearExtElem,
    ) -> Result<bool> {
        if reg_sizes.len() != reg_combo_ids.len() {
            return Ok(false);
        }
        let total_reg_coeffs = reg_sizes
            .iter()
            .try_fold(0usize, |acc, size| acc.checked_add(*size as usize))
            .ok_or_else(|| anyhow!("WebGPU combos_prepare register size overflow"))?;
        ensure!(
            coeff_u.len() == total_reg_coeffs + <Self as Hal>::CHECK_SIZE,
            "WebGPU combos_prepare coeff_u length mismatch: got {}, expected {}",
            coeff_u.len(),
            total_reg_coeffs + <Self as Hal>::CHECK_SIZE
        );
        if combos.size() == 0 {
            return Ok(true);
        }

        let Some(combos_gpu) = combos.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(combos) {
            return Ok(false);
        }

        let coeff_u = self.copy_from_extelem("webgpu_combos_prepare_coeff_u", coeff_u);
        let reg_sizes = self.copy_from_u32("webgpu_combos_prepare_reg_sizes", reg_sizes);
        let reg_combo_ids =
            self.copy_from_u32("webgpu_combos_prepare_reg_combo_ids", reg_combo_ids);

        let mut mix_pows = Vec::with_capacity(reg_sizes.size() + <Self as Hal>::CHECK_SIZE);
        let mut cur = BabyBearExtElem::ONE;
        for _ in 0..reg_sizes.size() {
            mix_pows.push(cur);
            cur *= *mix;
        }
        for _ in 0..<Self as Hal>::CHECK_SIZE {
            mix_pows.push(cur);
            cur *= *mix;
        }
        let mix_pows = self.copy_from_extelem("webgpu_combos_prepare_mix_pows", &mix_pows);

        let (Some(coeff_u_gpu), Some(reg_sizes_gpu), Some(reg_combo_ids_gpu), Some(mix_pows_gpu)) = (
            coeff_u.raw_buffer(),
            reg_sizes.raw_buffer(),
            reg_combo_ids.raw_buffer(),
            mix_pows.raw_buffer(),
        ) else {
            return Ok(false);
        };
        if !self.storage_binding_fits(&coeff_u)
            || !self.storage_binding_fits(&reg_sizes)
            || !self.storage_binding_fits(&reg_combo_ids)
            || !self.storage_binding_fits(&mix_pows)
        {
            return Ok(false);
        }

        let combos_base = combos
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU combos_prepare combos offset exceeds u32"))?;
        let coeff_u_base = coeff_u
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU combos_prepare coeff_u offset exceeds u32"))?;
        let mix_pows_base = mix_pows
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU combos_prepare mix_pows offset exceeds u32"))?;
        let params = [
            u32::try_from(total_reg_coeffs)
                .expect("WebGPU combos_prepare total_reg_coeffs exceeds u32"),
            u32::try_from(combo_count).expect("WebGPU combos_prepare combo_count exceeds u32"),
            u32::try_from(cycles).expect("WebGPU combos_prepare cycles exceeds u32"),
            u32::try_from(reg_sizes.size()).expect("WebGPU combos_prepare regs_count exceeds u32"),
            combos_base,
            coeff_u_base,
            u32::try_from(reg_sizes.elem_offset)
                .expect("WebGPU combos_prepare reg_sizes offset exceeds u32"),
            u32::try_from(reg_combo_ids.elem_offset)
                .expect("WebGPU combos_prepare reg_combo_ids offset exceeds u32"),
            mix_pows_base,
            u32::try_from(<Self as Hal>::CHECK_SIZE)
                .expect("WebGPU combos_prepare check size exceeds u32"),
            0,
            0,
        ];
        let params = self.create_uniform_buffer(
            "webgpu_combos_prepare_params",
            bytemuck::cast_slice(&params),
        )?;

        combos.sync_cpu_to_gpu(self)?;
        coeff_u.sync_cpu_to_gpu(self)?;
        reg_sizes.sync_cpu_to_gpu(self)?;
        reg_combo_ids.sync_cpu_to_gpu(self)?;
        mix_pows.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_combos_prepare_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::uniform(5, 48),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_combos_prepare",
            COMBOS_PREPARE_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_combos_prepare_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, combos_gpu),
                WebGpuBufferBinding::new(1, coeff_u_gpu),
                WebGpuBufferBinding::new(2, reg_sizes_gpu),
                WebGpuBufferBinding::new(3, reg_combo_ids_gpu),
                WebGpuBufferBinding::new(4, mix_pows_gpu),
                WebGpuBufferBinding {
                    binding: 5,
                    buffer: &params,
                    offset: 0,
                    size: Some(48),
                },
            ],
        )?;
        let work_items = u32::try_from(total_reg_coeffs + 1)
            .expect("WebGPU combos_prepare work item count exceeds u32");
        self.dispatch_compute(
            &kernel,
            &bind_group,
            work_items.div_ceil(WEBGPU_WORKGROUP_SIZE),
            1,
            1,
        );
        Ok(true)
    }

    pub(crate) fn dispatch_combos_divide(
        &self,
        combos: &WebGpuBuffer<BabyBearExtElem>,
        chunks: &[(usize, Vec<BabyBearExtElem>)],
        cycles: usize,
    ) -> Result<bool> {
        if chunks.is_empty() || cycles == 0 {
            return Ok(true);
        }

        let Some(combos_gpu) = combos.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(combos) {
            return Ok(false);
        }

        let mut chunk_indices = Vec::with_capacity(chunks.len());
        let mut chunk_offsets = Vec::with_capacity(chunks.len() + 1);
        let mut pows = Vec::new();
        chunk_offsets.push(0u32);
        for (combo_idx, chunk_pows) in chunks {
            ensure!(
                combo_idx
                    .checked_mul(cycles)
                    .and_then(|start| start.checked_add(cycles))
                    .is_some_and(|end| end <= combos.size()),
                "WebGPU combos_divide combo index {combo_idx} is out of range"
            );
            chunk_indices.push(
                u32::try_from(*combo_idx).expect("WebGPU combos_divide combo index exceeds u32"),
            );
            pows.extend(chunk_pows.iter().copied());
            chunk_offsets.push(
                u32::try_from(pows.len()).expect("WebGPU combos_divide pow count exceeds u32"),
            );
        }

        let pows = self.copy_from_extelem("webgpu_combos_divide_pows", &pows);
        let chunk_indices =
            self.copy_from_u32("webgpu_combos_divide_chunk_indices", &chunk_indices);
        let chunk_offsets =
            self.copy_from_u32("webgpu_combos_divide_chunk_offsets", &chunk_offsets);
        let (Some(pows_gpu), Some(chunk_indices_gpu), Some(chunk_offsets_gpu)) = (
            pows.raw_buffer(),
            chunk_indices.raw_buffer(),
            chunk_offsets.raw_buffer(),
        ) else {
            return Ok(false);
        };
        if !self.storage_binding_fits(&pows)
            || !self.storage_binding_fits(&chunk_indices)
            || !self.storage_binding_fits(&chunk_offsets)
        {
            return Ok(false);
        }

        let combos_base = combos
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU combos_divide combos offset exceeds u32"))?;
        let pows_base = pows
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU combos_divide pows offset exceeds u32"))?;

        if combos_divide_parallel_enabled() {
            return self.dispatch_combos_divide_parallel(
                combos,
                combos_gpu,
                &pows,
                pows_gpu,
                &chunk_indices,
                chunk_indices_gpu,
                &chunk_offsets,
                chunk_offsets_gpu,
                chunks,
                cycles,
                combos_base,
                pows_base,
            );
        }

        let params = [
            u32::try_from(chunks.len()).expect("WebGPU combos_divide chunk count exceeds u32"),
            u32::try_from(cycles).expect("WebGPU combos_divide cycles exceeds u32"),
            combos_base,
            pows_base,
            u32::try_from(chunk_indices.elem_offset)
                .expect("WebGPU combos_divide chunk index offset exceeds u32"),
            u32::try_from(chunk_offsets.elem_offset)
                .expect("WebGPU combos_divide chunk offset offset exceeds u32"),
            0,
            0,
        ];
        let params = self
            .create_uniform_buffer("webgpu_combos_divide_params", bytemuck::cast_slice(&params))?;

        combos.sync_cpu_to_gpu(self)?;
        pows.sync_cpu_to_gpu(self)?;
        chunk_indices.sync_cpu_to_gpu(self)?;
        chunk_offsets.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_combos_divide_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_combos_divide",
            COMBOS_DIVIDE_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_combos_divide_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, combos_gpu),
                WebGpuBufferBinding::new(1, pows_gpu),
                WebGpuBufferBinding::new(2, chunk_indices_gpu),
                WebGpuBufferBinding::new(3, chunk_offsets_gpu),
                WebGpuBufferBinding {
                    binding: 4,
                    buffer: &params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        self.dispatch_compute(
            &kernel,
            &bind_group,
            u32::try_from(chunks.len()).expect("WebGPU combos_divide chunk count exceeds u32"),
            1,
            1,
        );
        Ok(true)
    }

    /// Parallel-scan combos_divide: block-local suffix scan + per-chunk carry
    /// scan + element-wise fixup, one round per successive divisor. See the
    /// COMBOS_DIVIDE_SCAN_WGSL comment for the decomposition.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_combos_divide_parallel(
        &self,
        combos: &WebGpuBuffer<BabyBearExtElem>,
        combos_gpu: &web_sys::GpuBuffer,
        pows: &WebGpuBuffer<BabyBearExtElem>,
        pows_gpu: &web_sys::GpuBuffer,
        chunk_indices: &WebGpuBuffer<u32>,
        chunk_indices_gpu: &web_sys::GpuBuffer,
        chunk_offsets: &WebGpuBuffer<u32>,
        chunk_offsets_gpu: &web_sys::GpuBuffer,
        chunks: &[(usize, Vec<BabyBearExtElem>)],
        cycles: usize,
        combos_base: u32,
        pows_base: u32,
    ) -> Result<bool> {
        const BLOCK: usize = 256;
        let chunk_count = chunks.len();
        let nblocks = cycles.div_ceil(BLOCK);
        let max_rounds = chunks.iter().map(|(_, p)| p.len()).max().unwrap_or(0);
        if max_rounds == 0 {
            return Ok(true);
        }
        let total_groups = chunk_count
            .checked_mul(nblocks)
            .and_then(|total| u32::try_from(total).ok())
            .ok_or_else(|| anyhow!("WebGPU combos_divide scan group count overflow"))?;

        combos.sync_cpu_to_gpu(self)?;
        pows.sync_cpu_to_gpu(self)?;
        chunk_indices.sync_cpu_to_gpu(self)?;
        chunk_offsets.sync_cpu_to_gpu(self)?;

        let ext_bytes = (BabyBearExtElem::EXT_SIZE * mem::size_of::<BabyBearElem>()) as u64;
        let scratch = self.create_buffer(
            "webgpu_combos_divide_scratch",
            combos.size() as u64 * ext_bytes,
            WEBGPU_BUFFER_USAGE_STORAGE,
        )?;
        let carries = self.create_buffer(
            "webgpu_combos_divide_carries",
            (2 * chunk_count * nblocks) as u64 * ext_bytes,
            WEBGPU_BUFFER_USAGE_STORAGE,
        )?;

        let scan_layout = self.create_bind_group_layout(
            "webgpu_combos_divide_scan_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::uniform(6, 32),
            ],
        )?;
        let carry_layout = self.create_bind_group_layout(
            "webgpu_combos_divide_carry_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::uniform(3, 32),
            ],
        )?;
        let fixup_layout = self.create_bind_group_layout(
            "webgpu_combos_divide_fixup_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::uniform(6, 32),
            ],
        )?;
        let scan_kernel = self.create_compute_kernel(
            "webgpu_combos_divide_scan",
            COMBOS_DIVIDE_SCAN_WGSL,
            "main",
            &[scan_layout.clone()],
        )?;
        let carry_kernel = self.create_compute_kernel(
            "webgpu_combos_divide_carry",
            COMBOS_DIVIDE_CARRY_WGSL,
            "main",
            &[carry_layout.clone()],
        )?;
        let fixup_kernel = self.create_compute_kernel(
            "webgpu_combos_divide_fixup",
            COMBOS_DIVIDE_FIXUP_WGSL,
            "main",
            &[fixup_layout.clone()],
        )?;

        let mut round_params = Vec::with_capacity(max_rounds);
        let mut round_bind_groups = Vec::with_capacity(max_rounds);
        for round in 0..max_rounds {
            let params = [
                u32::try_from(chunk_count).expect("WebGPU combos_divide chunk count exceeds u32"),
                u32::try_from(cycles).expect("WebGPU combos_divide cycles exceeds u32"),
                combos_base,
                pows_base,
                u32::try_from(chunk_indices.elem_offset)
                    .expect("WebGPU combos_divide chunk index offset exceeds u32"),
                u32::try_from(chunk_offsets.elem_offset)
                    .expect("WebGPU combos_divide chunk offset offset exceeds u32"),
                u32::try_from(nblocks).expect("WebGPU combos_divide block count exceeds u32"),
                u32::try_from(round).expect("WebGPU combos_divide round exceeds u32"),
            ];
            let params = self.create_uniform_buffer(
                "webgpu_combos_divide_parallel_params",
                bytemuck::cast_slice(&params),
            )?;
            round_params.push(params);
        }
        for params in &round_params {
            let params_binding = |binding| WebGpuBufferBinding {
                binding,
                buffer: params,
                offset: 0,
                size: Some(32),
            };
            let scan_bind_group = self.create_bind_group(
                "webgpu_combos_divide_scan_bind_group",
                &scan_layout,
                &[
                    WebGpuBufferBinding::new(0, combos_gpu),
                    WebGpuBufferBinding::new(1, &scratch),
                    WebGpuBufferBinding::new(2, &carries),
                    WebGpuBufferBinding::new(3, pows_gpu),
                    WebGpuBufferBinding::new(4, chunk_indices_gpu),
                    WebGpuBufferBinding::new(5, chunk_offsets_gpu),
                    params_binding(6),
                ],
            )?;
            let carry_bind_group = self.create_bind_group(
                "webgpu_combos_divide_carry_bind_group",
                &carry_layout,
                &[
                    WebGpuBufferBinding::new(0, &carries),
                    WebGpuBufferBinding::new(1, pows_gpu),
                    WebGpuBufferBinding::new(2, chunk_offsets_gpu),
                    params_binding(3),
                ],
            )?;
            let fixup_bind_group = self.create_bind_group(
                "webgpu_combos_divide_fixup_bind_group",
                &fixup_layout,
                &[
                    WebGpuBufferBinding::new(0, combos_gpu),
                    WebGpuBufferBinding::new(1, &scratch),
                    WebGpuBufferBinding::new(2, &carries),
                    WebGpuBufferBinding::new(3, pows_gpu),
                    WebGpuBufferBinding::new(4, chunk_indices_gpu),
                    WebGpuBufferBinding::new(5, chunk_offsets_gpu),
                    params_binding(6),
                ],
            )?;
            round_bind_groups.push((scan_bind_group, carry_bind_group, fixup_bind_group));
        }

        let chunk_groups =
            u32::try_from(chunk_count).expect("WebGPU combos_divide chunk count exceeds u32");
        let mut dispatches = Vec::with_capacity(3 * max_rounds);
        for (scan_bind_group, carry_bind_group, fixup_bind_group) in &round_bind_groups {
            dispatches.push((&scan_kernel, scan_bind_group, total_groups));
            dispatches.push((&carry_kernel, carry_bind_group, chunk_groups));
            dispatches.push((&fixup_kernel, fixup_bind_group, total_groups));
        }
        self.dispatch_compute_1d_bind_group_sequence(&dispatches);

        // Queued work keeps its allocations alive until execution completes;
        // destroying here releases the VRAM as soon as the pass retires
        // instead of waiting for JS garbage collection (D14 lesson).
        scratch.destroy();
        carries.destroy();

        record_combos_divide_parallel_dispatch();
        Ok(true)
    }

    pub(crate) fn dispatch_batch_expand_into_evaluate_ntt(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
        count: usize,
        expand_bits: usize,
    ) -> Result<bool> {
        if !self.batch_expand_into_evaluate_ntt_gpu_enabled.get() {
            return Ok(false);
        }
        if output.size() == 0 {
            return Ok(true);
        }

        let (Some(output_gpu), Some(input_gpu)) = (output.raw_buffer(), input.raw_buffer()) else {
            return Ok(false);
        };
        if output_gpu == input_gpu {
            return Ok(false);
        }
        if !self.storage_binding_fits(output) || !self.storage_binding_fits(input) {
            return Ok(false);
        }

        let out_size = output.size() / count;
        let in_size = input.size() / count;
        let actual_expand_bits = crate::core::log2_ceil(out_size / in_size);
        assert_eq!(output.size(), out_size * count);
        assert_eq!(input.size(), in_size * count);
        assert_eq!(out_size, in_size * (1 << actual_expand_bits));

        let row_size = output.size() / count;
        assert_eq!(row_size * count, output.size());
        let n_bits = crate::core::log2_ceil(row_size);
        assert_eq!(row_size, 1 << n_bits);
        assert!(n_bits >= actual_expand_bits);
        assert_eq!(actual_expand_bits, expand_bits);
        assert!(n_bits < BabyBearElem::MAX_ROU_PO2);

        input.sync_cpu_to_gpu(self)?;

        let twiddles = self.ntt_twiddles(false, n_bits)?;
        let Some(twiddles_gpu) = twiddles.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(&twiddles) {
            return Ok(false);
        }
        const LOCAL_NTT_FUSED_BITS: usize = 10;
        let local_ntt_fused_bits = n_bits.min(LOCAL_NTT_FUSED_BITS);
        let local_ntt_block_size = 1usize << LOCAL_NTT_FUSED_BITS;
        let blocks_per_row = row_size.div_ceil(local_ntt_block_size);
        let total_blocks = count
            .checked_mul(blocks_per_row)
            .ok_or_else(|| anyhow!("WebGPU fused NTT block count overflow"))?;
        let expand_params = [
            u32::try_from(out_size).expect("WebGPU fused NTT out_size exceeds u32"),
            u32::try_from(in_size).expect("WebGPU fused NTT in_size exceeds u32"),
            u32::try_from(count).expect("WebGPU fused NTT row count exceeds u32"),
            u32::try_from(actual_expand_bits).expect("WebGPU fused NTT expand_bits exceeds u32"),
            u32::try_from(output.elem_offset).expect("WebGPU fused NTT output offset exceeds u32"),
            u32::try_from(input.elem_offset).expect("WebGPU fused NTT input offset exceeds u32"),
            u32::try_from(twiddles.elem_offset)
                .expect("WebGPU fused NTT twiddles offset exceeds u32"),
            u32::try_from(n_bits).expect("WebGPU fused NTT n_bits exceeds u32"),
            u32::try_from(blocks_per_row).expect("WebGPU fused NTT blocks per row exceeds u32"),
            u32::try_from(total_blocks).expect("WebGPU fused NTT total blocks exceeds u32"),
            0,
            0,
        ];
        let expand_layout = self.create_bind_group_layout(
            "webgpu_batch_expand_local_ntt_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::uniform(3, 48),
            ],
        )?;
        let expand_params = self.create_uniform_buffer(
            "webgpu_batch_expand_local_ntt_params",
            bytemuck::cast_slice(&expand_params),
        )?;
        let expand_kernel = self.create_compute_kernel(
            "webgpu_batch_expand_local_ntt",
            BATCH_EXPAND_LOCAL_NTT_WGSL,
            "main",
            &[expand_layout.clone()],
        )?;
        let expand_bind_group = self.create_bind_group(
            "webgpu_batch_expand_local_ntt_bind_group",
            &expand_layout,
            &[
                WebGpuBufferBinding::new(0, output_gpu),
                WebGpuBufferBinding::new(1, input_gpu),
                WebGpuBufferBinding::new(2, twiddles_gpu),
                WebGpuBufferBinding {
                    binding: 3,
                    buffer: &expand_params,
                    offset: 0,
                    size: Some(48),
                },
            ],
        )?;
        let expand_workgroups =
            u32::try_from(total_blocks).expect("WebGPU fused NTT total blocks exceeds u32");
        self.dispatch_compute_1d(&expand_kernel, &expand_bind_group, expand_workgroups);

        if n_bits <= local_ntt_fused_bits {
            return Ok(true);
        }

        let ntt_layout = self.create_bind_group_layout(
            "webgpu_ntt_step_dynamic_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform_dynamic(2, 32),
            ],
        )?;
        let pairs_per_row = row_size / 2;
        let total_pairs = pairs_per_row
            .checked_mul(count)
            .ok_or_else(|| anyhow!("WebGPU NTT total pair count overflow"))?;
        let ntt_kernel = self.create_compute_kernel(
            "webgpu_ntt_step_dynamic",
            NTT_STEP_WGSL,
            "main",
            &[ntt_layout.clone()],
        )?;
        let workgroups = u32::try_from(total_pairs)
            .expect("WebGPU NTT total pairs exceeds u32")
            .div_ceil(256);
        // Batch all NTT step
        // dispatches into ONE command encoder + submit instead of
        // per-step submit. Each NTT level reads its predecessor's
        // output; within a single compute pass, dispatches execute
        // serially on the queue so write-after-write ordering is
        // preserved without inserting barriers. For xgboost po2_18
        // this collapses 16 GPU-process IPC round-trips per NTT call
        // into 1. NTT runs many times per segment, so the cumulative
        // submission-overhead savings stack.
        //
        let params_len = mem::size_of::<[u32; 8]>();
        let params_stride = align_up(
            params_len,
            self.min_uniform_buffer_offset_alignment as usize,
        );
        let mut params_bytes = Vec::new();
        let first_remaining_s_bits = actual_expand_bits.max(local_ntt_fused_bits) + 1;
        let mut params_offsets: Vec<u32> =
            Vec::with_capacity((n_bits - first_remaining_s_bits + 1) as usize);
        for s_bits in first_remaining_s_bits..=n_bits {
            let params = [
                u32::try_from(n_bits).expect("WebGPU NTT n_bits exceeds u32"),
                u32::try_from(s_bits).expect("WebGPU NTT s_bits exceeds u32"),
                u32::try_from(count).expect("WebGPU NTT row count exceeds u32"),
                u32::try_from(total_pairs).expect("WebGPU NTT total pairs exceeds u32"),
                u32::try_from(output.elem_offset).expect("WebGPU NTT output offset exceeds u32"),
                u32::try_from(twiddles.elem_offset)
                    .expect("WebGPU NTT twiddles offset exceeds u32"),
                0,
                0,
            ];
            let params_offset = params_bytes.len();
            params_bytes.resize(params_offset + params_stride, 0);
            params_bytes[params_offset..params_offset + params_len]
                .copy_from_slice(bytemuck::cast_slice(&params));
            params_offsets.push(
                u32::try_from(params_offset)
                    .map_err(|_| anyhow!("WebGPU NTT params offset exceeds u32"))?,
            );
        }
        let params_buf = self.create_buffer(
            "webgpu_ntt_step_params",
            params_bytes.len() as u64,
            WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;
        self.write_buffer_named(
            &params_buf,
            "webgpu_ntt_step_params",
            0,
            params_bytes.as_slice(),
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_ntt_step_bind_group",
            &ntt_layout,
            &[
                WebGpuBufferBinding::new(0, output_gpu),
                WebGpuBufferBinding::new(1, twiddles_gpu),
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: &params_buf,
                    offset: 0,
                    size: Some(params_len as u64),
                },
            ],
        )?;
        let (workgroups_x, workgroups_y) = if workgroups <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION {
            (workgroups, 1)
        } else {
            let workgroups_y = workgroups.div_ceil(WEBGPU_MAX_WORKGROUPS_PER_DIMENSION);
            assert!(
                workgroups_y <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION,
                "WebGPU NTT 1D dispatch exceeds portable 2D workgroup capacity"
            );
            (WEBGPU_MAX_WORKGROUPS_PER_DIMENSION, workgroups_y)
        };
        let encoder = self.device.create_command_encoder();
        let pass = encoder.begin_compute_pass();
        pass.set_pipeline(&ntt_kernel.pipeline);
        for params_offset in &params_offsets {
            let dynamic_offsets = [*params_offset];
            pass.set_bind_group_with_u32_slice_and_u32_and_dynamic_offsets_data_length(
                0,
                Some(&bind_group),
                &dynamic_offsets,
                0,
                1,
            )
            .map_err(js_error)?;
            pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
                workgroups_x,
                workgroups_y,
                1,
            );
            self.diagnostics.record_raw_compute_dispatch();
        }
        pass.end();
        self.submit(encoder.finish());
        Ok(true)
    }

    pub(crate) fn dispatch_batch_interpolate_ntt(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        count: usize,
    ) -> Result<bool> {
        if !self.batch_interpolate_ntt_gpu_enabled.get() {
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

        let row_size = io.size() / count;
        assert_eq!(row_size * count, io.size());
        let n_bits = crate::core::log2_ceil(row_size);
        assert_eq!(row_size, 1 << n_bits);
        assert!(n_bits < BabyBearElem::MAX_ROU_PO2);

        io.sync_cpu_to_gpu(self)?;

        if n_bits != 0 {
            let twiddles = self.ntt_twiddles(true, n_bits)?;
            let Some(twiddles_gpu) = twiddles.raw_buffer() else {
                return Ok(false);
            };
            if !self.storage_binding_fits(&twiddles) {
                return Ok(false);
            }
            let ntt_layout = self.create_bind_group_layout(
                "webgpu_ntt_step_dynamic_layout",
                &[
                    WebGpuBindingLayout::storage(0, 0),
                    WebGpuBindingLayout::read_only_storage(1, 0),
                    WebGpuBindingLayout::uniform_dynamic(2, 32),
                ],
            )?;
            let ntt_kernel = self.create_compute_kernel(
                "webgpu_ntt_step_dynamic",
                NTT_STEP_WGSL,
                "main",
                &[ntt_layout.clone()],
            )?;
            let pairs_per_row = row_size / 2;
            let total_pairs = pairs_per_row
                .checked_mul(count)
                .ok_or_else(|| anyhow!("WebGPU inverse NTT total pair count overflow"))?;
            let workgroups = u32::try_from(total_pairs)
                .expect("WebGPU inverse NTT total pairs exceeds u32")
                .div_ceil(256);
            let params_len = mem::size_of::<[u32; 8]>();
            let params_stride = align_up(
                params_len,
                self.min_uniform_buffer_offset_alignment as usize,
            );
            let mut params_bytes = Vec::new();
            let mut params_offsets: Vec<u32> = Vec::with_capacity(n_bits as usize);
            for s_bits in (1..=n_bits).rev() {
                let params = [
                    u32::try_from(n_bits).expect("WebGPU inverse NTT n_bits exceeds u32"),
                    u32::try_from(s_bits).expect("WebGPU inverse NTT s_bits exceeds u32"),
                    u32::try_from(count).expect("WebGPU inverse NTT row count exceeds u32"),
                    u32::try_from(total_pairs).expect("WebGPU inverse NTT total pairs exceeds u32"),
                    u32::try_from(io.elem_offset)
                        .expect("WebGPU inverse NTT io offset exceeds u32"),
                    u32::try_from(twiddles.elem_offset)
                        .expect("WebGPU inverse NTT twiddles offset exceeds u32"),
                    1,
                    0,
                ];
                let params_offset = params_bytes.len();
                params_bytes.resize(params_offset + params_stride, 0);
                params_bytes[params_offset..params_offset + params_len]
                    .copy_from_slice(bytemuck::cast_slice(&params));
                params_offsets.push(
                    u32::try_from(params_offset)
                        .map_err(|_| anyhow!("WebGPU inverse NTT params offset exceeds u32"))?,
                );
            }
            let params_buf = self.create_buffer(
                "webgpu_ntt_step_params",
                params_bytes.len() as u64,
                WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
            )?;
            self.write_buffer_named(
                &params_buf,
                "webgpu_ntt_step_params",
                0,
                params_bytes.as_slice(),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_ntt_step_bind_group",
                &ntt_layout,
                &[
                    WebGpuBufferBinding::new(0, io_gpu),
                    WebGpuBufferBinding::new(1, twiddles_gpu),
                    WebGpuBufferBinding {
                        binding: 2,
                        buffer: &params_buf,
                        offset: 0,
                        size: Some(params_len as u64),
                    },
                ],
            )?;
            let (workgroups_x, workgroups_y) = if workgroups <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION
            {
                (workgroups, 1)
            } else {
                let workgroups_y = workgroups.div_ceil(WEBGPU_MAX_WORKGROUPS_PER_DIMENSION);
                assert!(
                    workgroups_y <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION,
                    "WebGPU inverse NTT 1D dispatch exceeds portable 2D workgroup capacity"
                );
                (WEBGPU_MAX_WORKGROUPS_PER_DIMENSION, workgroups_y)
            };
            let encoder = self.device.create_command_encoder();
            let pass = encoder.begin_compute_pass();
            pass.set_pipeline(&ntt_kernel.pipeline);
            for params_offset in &params_offsets {
                let dynamic_offsets = [*params_offset];
                pass.set_bind_group_with_u32_slice_and_u32_and_dynamic_offsets_data_length(
                    0,
                    Some(&bind_group),
                    &dynamic_offsets,
                    0,
                    1,
                )
                .map_err(js_error)?;
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

        let norm = BabyBearElem::new(row_size as u32).inv().as_u32_montgomery();
        let params = [
            u32::try_from(io.size()).expect("WebGPU inverse NTT size exceeds u32"),
            u32::try_from(io.elem_offset).expect("WebGPU inverse NTT io offset exceeds u32"),
            norm,
            0,
        ];
        let params = self
            .create_uniform_buffer("webgpu_ntt_normalize_params", bytemuck::cast_slice(&params))?;
        let layout = self.create_bind_group_layout(
            "webgpu_ntt_normalize_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::uniform(1, 16),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_ntt_normalize",
            NTT_NORMALIZE_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_ntt_normalize_bind_group",
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
            .expect("WebGPU inverse NTT size exceeds u32")
            .div_ceil(256);
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
        Ok(true)
    }

    pub(crate) fn dispatch_batch_interpolate_ntt_from(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
        count: usize,
    ) -> Result<bool> {
        if !self.batch_interpolate_ntt_gpu_enabled.get() {
            return Ok(false);
        }
        if output.size() == 0 {
            output.mark_gpu_dirty();
            self.record_gpu_result_authoritative("batch_interpolate_ntt", true);
            return Ok(true);
        }

        let (Some(output_gpu), Some(input_gpu)) = (output.raw_buffer(), input.raw_buffer()) else {
            return Ok(false);
        };
        if output_gpu == input_gpu
            || output.size() != input.size()
            || !self.storage_binding_fits(output)
            || !self.storage_binding_fits(input)
        {
            return Ok(false);
        }

        let row_size = output.size() / count;
        assert_eq!(row_size * count, output.size());
        let n_bits = crate::core::log2_ceil(row_size);
        assert_eq!(row_size, 1 << n_bits);
        assert!(n_bits < BabyBearElem::MAX_ROU_PO2);
        if n_bits == 0 {
            return Ok(false);
        }

        input.sync_cpu_to_gpu(self)?;

        let twiddles = self.ntt_twiddles(true, n_bits)?;
        let Some(twiddles_gpu) = twiddles.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(&twiddles) {
            return Ok(false);
        }

        let pairs_per_row = row_size / 2;
        let total_pairs = pairs_per_row
            .checked_mul(count)
            .ok_or_else(|| anyhow!("WebGPU fused inverse NTT total pair count overflow"))?;
        let workgroups = u32::try_from(total_pairs)
            .expect("WebGPU fused inverse NTT total pairs exceeds u32")
            .div_ceil(256);
        let (workgroups_x, workgroups_y) = if workgroups <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION {
            (workgroups, 1)
        } else {
            let workgroups_y = workgroups.div_ceil(WEBGPU_MAX_WORKGROUPS_PER_DIMENSION);
            assert!(
                workgroups_y <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION,
                "WebGPU fused inverse NTT 1D dispatch exceeds portable 2D workgroup capacity"
            );
            (WEBGPU_MAX_WORKGROUPS_PER_DIMENSION, workgroups_y)
        };

        let first_params = [
            u32::try_from(n_bits).expect("WebGPU fused inverse NTT n_bits exceeds u32"),
            u32::try_from(count).expect("WebGPU fused inverse NTT row count exceeds u32"),
            u32::try_from(total_pairs).expect("WebGPU fused inverse NTT total pairs exceeds u32"),
            u32::try_from(output.elem_offset)
                .expect("WebGPU fused inverse NTT output offset exceeds u32"),
            u32::try_from(input.elem_offset)
                .expect("WebGPU fused inverse NTT input offset exceeds u32"),
            u32::try_from(twiddles.elem_offset)
                .expect("WebGPU fused inverse NTT twiddles offset exceeds u32"),
            0,
            0,
        ];
        let first_params = self.create_uniform_buffer(
            "webgpu_ntt_interpolate_from_params",
            bytemuck::cast_slice(&first_params),
        )?;
        let first_layout = self.create_bind_group_layout(
            "webgpu_ntt_interpolate_from_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::uniform(3, 32),
            ],
        )?;
        let first_kernel = self.create_compute_kernel(
            "webgpu_ntt_interpolate_from",
            BATCH_INTERPOLATE_NTT_FROM_WGSL,
            "main",
            &[first_layout.clone()],
        )?;
        let first_bind_group = self.create_bind_group(
            "webgpu_ntt_interpolate_from_bind_group",
            &first_layout,
            &[
                WebGpuBufferBinding::new(0, output_gpu),
                WebGpuBufferBinding::new(1, input_gpu),
                WebGpuBufferBinding::new(2, twiddles_gpu),
                WebGpuBufferBinding {
                    binding: 3,
                    buffer: &first_params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;

        let ntt_layout = self.create_bind_group_layout(
            "webgpu_ntt_step_dynamic_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform_dynamic(2, 32),
            ],
        )?;
        let ntt_kernel = self.create_compute_kernel(
            "webgpu_ntt_step_dynamic",
            NTT_STEP_WGSL,
            "main",
            &[ntt_layout.clone()],
        )?;
        let params_len = mem::size_of::<[u32; 8]>();
        let params_stride = align_up(
            params_len,
            self.min_uniform_buffer_offset_alignment as usize,
        );
        let mut params_bytes = Vec::new();
        let mut params_offsets: Vec<u32> = Vec::with_capacity(n_bits.saturating_sub(1) as usize);
        for s_bits in (1..n_bits).rev() {
            let params = [
                u32::try_from(n_bits).expect("WebGPU fused inverse NTT n_bits exceeds u32"),
                u32::try_from(s_bits).expect("WebGPU fused inverse NTT s_bits exceeds u32"),
                u32::try_from(count).expect("WebGPU fused inverse NTT row count exceeds u32"),
                u32::try_from(total_pairs)
                    .expect("WebGPU fused inverse NTT total pairs exceeds u32"),
                u32::try_from(output.elem_offset)
                    .expect("WebGPU fused inverse NTT output offset exceeds u32"),
                u32::try_from(twiddles.elem_offset)
                    .expect("WebGPU fused inverse NTT twiddles offset exceeds u32"),
                1,
                0,
            ];
            let params_offset = params_bytes.len();
            params_bytes.resize(params_offset + params_stride, 0);
            params_bytes[params_offset..params_offset + params_len]
                .copy_from_slice(bytemuck::cast_slice(&params));
            params_offsets.push(
                u32::try_from(params_offset)
                    .map_err(|_| anyhow!("WebGPU fused inverse NTT params offset exceeds u32"))?,
            );
        }
        let ntt_bind_group = if params_offsets.is_empty() {
            None
        } else {
            let params_buf = self.create_buffer(
                "webgpu_ntt_step_params",
                params_bytes.len() as u64,
                WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
            )?;
            self.write_buffer_named(
                &params_buf,
                "webgpu_ntt_step_params",
                0,
                params_bytes.as_slice(),
            )?;
            Some(self.create_bind_group(
                "webgpu_ntt_step_bind_group",
                &ntt_layout,
                &[
                    WebGpuBufferBinding::new(0, output_gpu),
                    WebGpuBufferBinding::new(1, twiddles_gpu),
                    WebGpuBufferBinding {
                        binding: 2,
                        buffer: &params_buf,
                        offset: 0,
                        size: Some(params_len as u64),
                    },
                ],
            )?)
        };

        let encoder = self.device.create_command_encoder();
        let pass = encoder.begin_compute_pass();
        pass.set_pipeline(&first_kernel.pipeline);
        pass.set_bind_group(0, Some(&first_bind_group));
        pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
            workgroups_x,
            workgroups_y,
            1,
        );
        self.diagnostics.record_raw_compute_dispatch();
        if let Some(bind_group) = ntt_bind_group.as_ref() {
            pass.set_pipeline(&ntt_kernel.pipeline);
            for params_offset in &params_offsets {
                let dynamic_offsets = [*params_offset];
                pass.set_bind_group_with_u32_slice_and_u32_and_dynamic_offsets_data_length(
                    0,
                    Some(bind_group),
                    &dynamic_offsets,
                    0,
                    1,
                )
                .map_err(js_error)?;
                pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
                    workgroups_x,
                    workgroups_y,
                    1,
                );
                self.diagnostics.record_raw_compute_dispatch();
            }
        }
        pass.end();
        self.submit(encoder.finish());

        let norm = BabyBearElem::new(row_size as u32).inv().as_u32_montgomery();
        let params = [
            u32::try_from(output.size()).expect("WebGPU inverse NTT size exceeds u32"),
            u32::try_from(output.elem_offset)
                .expect("WebGPU inverse NTT output offset exceeds u32"),
            norm,
            0,
        ];
        let params = self
            .create_uniform_buffer("webgpu_ntt_normalize_params", bytemuck::cast_slice(&params))?;
        let layout = self.create_bind_group_layout(
            "webgpu_ntt_normalize_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::uniform(1, 16),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_ntt_normalize",
            NTT_NORMALIZE_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_ntt_normalize_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, output_gpu),
                WebGpuBufferBinding {
                    binding: 1,
                    buffer: &params,
                    offset: 0,
                    size: Some(16),
                },
            ],
        )?;
        let workgroups = u32::try_from(output.size())
            .expect("WebGPU inverse NTT size exceeds u32")
            .div_ceil(256);
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
        self.record_gpu_result_authoritative("batch_interpolate_ntt", true);
        output.mark_gpu_dirty();
        Ok(true)
    }

    pub(crate) fn dispatch_batch_bit_reverse(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        bits: usize,
    ) -> Result<bool> {
        if !self.batch_bit_reverse_gpu_enabled.get() {
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
            u32::try_from(io.size()).expect("WebGPU bit_reverse count exceeds u32"),
            u32::try_from(bits).expect("WebGPU bit_reverse bits exceeds u32"),
            u32::try_from(io.elem_offset).expect("WebGPU bit_reverse offset exceeds u32"),
            0,
        ];
        let params = self.create_uniform_buffer(
            "webgpu_batch_bit_reverse_params",
            bytemuck::cast_slice(&params),
        )?;

        io.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_batch_bit_reverse_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::uniform(1, 16),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_batch_bit_reverse",
            BATCH_BIT_REVERSE_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_batch_bit_reverse_bind_group",
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
            .expect("WebGPU bit_reverse count exceeds u32")
            .div_ceil(WEBGPU_WORKGROUP_SIZE);
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
        Ok(true)
    }

    pub(crate) fn dispatch_batch_evaluate_any(
        &self,
        coeffs: &WebGpuBuffer<BabyBearElem>,
        which: &WebGpuBuffer<u32>,
        xs: &WebGpuBuffer<BabyBearExtElem>,
        out: &WebGpuBuffer<BabyBearExtElem>,
        deg: usize,
    ) -> Result<bool> {
        let eval_count = which.size();
        if eval_count == 0 {
            return Ok(true);
        }

        let (Some(out_gpu), Some(coeffs_gpu), Some(which_gpu), Some(xs_gpu)) = (
            out.raw_buffer(),
            coeffs.raw_buffer(),
            which.raw_buffer(),
            xs.raw_buffer(),
        ) else {
            return Ok(false);
        };
        if !self.storage_binding_fits(out)
            || !self.storage_binding_fits(coeffs)
            || !self.storage_binding_fits(which)
            || !self.storage_binding_fits(xs)
        {
            return Ok(false);
        }

        let Some(output_base) = out
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
        else {
            return Err(anyhow!(
                "WebGPU batch_evaluate_any output offset exceeds u32"
            ));
        };
        let Some(xs_base) = xs
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
        else {
            return Err(anyhow!("WebGPU batch_evaluate_any xs offset exceeds u32"));
        };
        let params = [
            u32::try_from(deg).expect("WebGPU batch_evaluate_any degree exceeds u32"),
            u32::try_from(eval_count).expect("WebGPU batch_evaluate_any eval_count exceeds u32"),
            output_base,
            u32::try_from(coeffs.elem_offset)
                .expect("WebGPU batch_evaluate_any coeffs offset exceeds u32"),
            u32::try_from(which.elem_offset)
                .expect("WebGPU batch_evaluate_any which offset exceeds u32"),
            xs_base,
            0,
            u32::try_from(deg).expect("WebGPU batch_evaluate_any degree exceeds u32"),
        ];

        coeffs.sync_cpu_to_gpu(self)?;
        which.sync_cpu_to_gpu(self)?;
        xs.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_batch_evaluate_any_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_batch_evaluate_any",
            BATCH_EVALUATE_ANY_WGSL,
            "main",
            &[layout.clone()],
        )?;

        if self.storage_binding_fits(coeffs) {
            let params = self.create_uniform_buffer(
                "webgpu_batch_evaluate_any_params",
                bytemuck::cast_slice(&params),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_batch_evaluate_any_bind_group",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, out_gpu),
                    WebGpuBufferBinding::new(1, coeffs_gpu),
                    WebGpuBufferBinding::new(2, which_gpu),
                    WebGpuBufferBinding::new(3, xs_gpu),
                    WebGpuBufferBinding {
                        binding: 4,
                        buffer: &params,
                        offset: 0,
                        size: Some(32),
                    },
                ],
            )?;
            self.dispatch_compute(
                &kernel,
                &bind_group,
                u32::try_from(eval_count)
                    .expect("WebGPU batch_evaluate_any eval_count exceeds u32"),
                1,
                1,
            );
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) fn dispatch_poseidon2_hash_fold(
        &self,
        io: &WebGpuBuffer<Digest>,
        input_size: usize,
        output_size: usize,
    ) -> Result<bool> {
        if !self.hash_fold_gpu_enabled.get() {
            return Ok(false);
        }
        let Some(hash) = self.poseidon2.as_ref() else {
            return Ok(false);
        };
        if output_size == 0 {
            return Ok(true);
        }

        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(io) {
            return Ok(false);
        }
        let (Some(round_constants_gpu), Some(m_int_diag_gpu)) = (
            hash.round_constants.raw_buffer(),
            hash.m_int_diag.raw_buffer(),
        ) else {
            return Ok(false);
        };

        let output_base = digest_word_offset(io.elem_offset + output_size)?;
        let input_base = digest_word_offset(io.elem_offset + input_size)?;
        let params = [
            u32::try_from(output_size).expect("WebGPU hash_fold output size exceeds u32"),
            u32::try_from(input_size).expect("WebGPU hash_fold input size exceeds u32"),
            0,
            0,
            output_base,
            input_base,
            0,
            0,
        ];
        let params = self.create_uniform_buffer(
            "webgpu_poseidon2_fold_params",
            bytemuck::cast_slice(&params),
        )?;

        io.sync_cpu_to_gpu(self)?;

        let bind_group = self.create_bind_group(
            "webgpu_poseidon2_fold_bind_group",
            &hash.fold_layout,
            &[
                WebGpuBufferBinding::new(0, round_constants_gpu),
                WebGpuBufferBinding::new(1, m_int_diag_gpu),
                WebGpuBufferBinding::new(2, io_gpu),
                WebGpuBufferBinding {
                    binding: 4,
                    buffer: &params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        let workgroups = u32::try_from(output_size)
            .expect("WebGPU hash_fold output size exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&hash.fold_kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    /// Batch a chain of hash_folds
    /// (the merkle tree-build loop) into a single command encoder. Each
    /// individual fold writes to a different slice of the same `nodes`
    /// buffer; within a compute pass dispatches execute serially on the
    /// queue so write-after-write ordering across layers is preserved.
    ///
    /// `output_sizes` lists the per-layer output_size; input_size is
    /// always `2 * output_size` per the merkle tree-build invariant.
    /// Returns true when the GPU path is used; falls back through
    /// individual hash_fold dispatches on any short-circuit condition.
    pub(crate) fn dispatch_poseidon2_hash_fold_chain(
        &self,
        io: &WebGpuBuffer<Digest>,
        output_sizes: &[usize],
    ) -> Result<bool> {
        if !self.hash_fold_gpu_enabled.get() {
            return Ok(false);
        }
        let Some(hash) = self.poseidon2.as_ref() else {
            return Ok(false);
        };
        if output_sizes.is_empty() {
            return Ok(true);
        }
        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(io) {
            return Ok(false);
        }
        let (Some(round_constants_gpu), Some(m_int_diag_gpu)) = (
            hash.round_constants.raw_buffer(),
            hash.m_int_diag.raw_buffer(),
        ) else {
            return Ok(false);
        };
        io.sync_cpu_to_gpu(self)?;

        // Pack all per-layer params into one dynamic
        // uniform buffer and bind it once. This keeps the existing single-pass
        // dispatch batching while avoiding one bind group per Merkle layer.
        let params_len = mem::size_of::<[u32; 8]>();
        let params_stride = align_up(
            params_len,
            self.min_uniform_buffer_offset_alignment as usize,
        );
        let mut params_bytes = Vec::new();
        let mut params_offsets: Vec<u32> = Vec::with_capacity(output_sizes.len());
        let mut workgroups_per_layer: Vec<u32> = Vec::with_capacity(output_sizes.len());
        for &output_size in output_sizes {
            if output_size == 0 {
                continue;
            }
            let input_size = 2 * output_size;
            let output_base = digest_word_offset(io.elem_offset + output_size)?;
            let input_base = digest_word_offset(io.elem_offset + input_size)?;
            let params = [
                u32::try_from(output_size).expect("WebGPU hash_fold output size exceeds u32"),
                u32::try_from(input_size).expect("WebGPU hash_fold input size exceeds u32"),
                0,
                0,
                output_base,
                input_base,
                0,
                0,
            ];
            let params_offset = params_bytes.len();
            params_bytes.resize(params_offset + params_stride, 0);
            params_bytes[params_offset..params_offset + params_len]
                .copy_from_slice(bytemuck::cast_slice(&params));
            params_offsets.push(
                u32::try_from(params_offset)
                    .map_err(|_| anyhow!("WebGPU hash_fold params offset exceeds u32"))?,
            );
            workgroups_per_layer.push(
                u32::try_from(output_size)
                    .expect("WebGPU hash_fold output size exceeds u32")
                    .div_ceil(256),
            );
        }
        if params_offsets.is_empty() {
            return Ok(true);
        }
        let params_buf = self.create_buffer(
            "webgpu_poseidon2_fold_chain_params",
            params_bytes.len() as u64,
            WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;
        self.write_buffer_named(
            &params_buf,
            "webgpu_poseidon2_fold_chain_params",
            0,
            params_bytes.as_slice(),
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_poseidon2_fold_chain_bind_group",
            &hash.fold_chain_layout,
            &[
                WebGpuBufferBinding::new(0, round_constants_gpu),
                WebGpuBufferBinding::new(1, m_int_diag_gpu),
                WebGpuBufferBinding::new(2, io_gpu),
                WebGpuBufferBinding {
                    binding: 4,
                    buffer: &params_buf,
                    offset: 0,
                    size: Some(params_len as u64),
                },
            ],
        )?;
        let encoder = self.device.create_command_encoder();
        let pass = encoder.begin_compute_pass();
        pass.set_pipeline(&hash.fold_chain_kernel.pipeline);
        for (params_offset, workgroups) in params_offsets.iter().zip(workgroups_per_layer.iter()) {
            let dynamic_offsets = [*params_offset];
            pass.set_bind_group_with_u32_slice_and_u32_and_dynamic_offsets_data_length(
                0,
                Some(&bind_group),
                &dynamic_offsets,
                0,
                1,
            )
            .map_err(js_error)?;
            pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
                *workgroups,
                1,
                1,
            );
            self.diagnostics.record_raw_compute_dispatch();
        }
        pass.end();
        self.submit(encoder.finish());
        // Mirror finish_hal_op accounting for each layer.
        for _ in 0..params_offsets.len() {
            self.record_gpu_result_authoritative("hash_fold", true);
        }
        io.mark_gpu_dirty();
        Ok(true)
    }

    pub(crate) fn dispatch_poseidon2_hash_rows(
        &self,
        output: &WebGpuBuffer<Digest>,
        matrix: &WebGpuBuffer<BabyBearElem>,
        row_size: usize,
        col_size: usize,
    ) -> Result<bool> {
        if !self.hash_rows_gpu_enabled.get() {
            return Ok(false);
        }
        let Some(hash) = self.poseidon2.as_ref() else {
            return Ok(false);
        };
        if row_size == 0 {
            return Ok(true);
        }

        let (Some(output_gpu), Some(matrix_gpu)) = (output.raw_buffer(), matrix.raw_buffer())
        else {
            return Ok(false);
        };
        if !self.storage_binding_fits(output) || !self.storage_binding_fits(matrix) {
            return Ok(false);
        }
        let (Some(round_constants_gpu), Some(m_int_diag_gpu)) = (
            hash.round_constants.raw_buffer(),
            hash.m_int_diag.raw_buffer(),
        ) else {
            return Ok(false);
        };

        let output_base = digest_word_offset(output.elem_offset)?;
        let matrix_base =
            u32::try_from(matrix.elem_offset).expect("WebGPU hash_rows matrix offset exceeds u32");
        let params = [
            0,
            0,
            u32::try_from(row_size).expect("WebGPU hash_rows row size exceeds u32"),
            u32::try_from(col_size).expect("WebGPU hash_rows col size exceeds u32"),
            output_base,
            0,
            matrix_base,
            0,
        ];
        let params = self.create_uniform_buffer(
            "webgpu_poseidon2_rows_params",
            bytemuck::cast_slice(&params),
        )?;

        matrix.sync_cpu_to_gpu(self)?;

        let bind_group = self.create_bind_group(
            "webgpu_poseidon2_rows_bind_group",
            &hash.rows_layout,
            &[
                WebGpuBufferBinding::new(0, round_constants_gpu),
                WebGpuBufferBinding::new(1, m_int_diag_gpu),
                WebGpuBufferBinding::new(2, output_gpu),
                WebGpuBufferBinding::new(3, matrix_gpu),
                WebGpuBufferBinding {
                    binding: 4,
                    buffer: &params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        let workgroups = u32::try_from(row_size)
            .expect("WebGPU hash_rows row size exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&hash.rows_kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }
}

pub(crate) fn byte_len_as_f64(byte_len: u64) -> Result<f64> {
    ensure!(
        byte_len <= MAX_EXACT_JS_INTEGER,
        "WebGPU byte length exceeds JS integer precision: {byte_len}"
    );
    Ok(byte_len as f64)
}

pub(crate) fn byte_offset_as_f64(byte_offset: u64) -> Result<f64> {
    ensure!(
        byte_offset <= MAX_EXACT_JS_INTEGER,
        "WebGPU byte offset exceeds JS integer precision: {byte_offset}"
    );
    Ok(byte_offset as f64)
}

pub(crate) fn align_up(value: usize, alignment: usize) -> usize {
    if alignment == 0 {
        return value;
    }
    value.div_ceil(alignment) * alignment
}

pub(crate) fn byte_len_for<T>(size: usize) -> u64 {
    size.checked_mul(mem::size_of::<T>())
        .and_then(|bytes| bytes.try_into().ok())
        .expect("WebGPU buffer size overflow")
}

pub(crate) struct SparseUploadPlan {
    pub(crate) values: Vec<BabyBearElem>,
    pub(crate) ranges: Vec<u32>,
}

pub(crate) enum SparseZeroizeUpload {
    NotUsed,
    Used { gpu_was_known_zero: bool },
}

impl SparseUploadPlan {
    pub(crate) fn new() -> Self {
        Self {
            values: Vec::new(),
            ranges: Vec::new(),
        }
    }

    pub(crate) fn push_range(&mut self, dst_start: usize, values: &[BabyBearElem]) -> Result<()> {
        let len = values.len();
        ensure!(len > 0, "sparse upload range length must be nonzero");
        self.ranges.push(
            u32::try_from(self.values.len())
                .map_err(|_| anyhow!("sparse upload source range exceeds u32"))?,
        );
        self.ranges.push(
            u32::try_from(dst_start)
                .map_err(|_| anyhow!("sparse upload destination range exceeds u32"))?,
        );
        self.values.extend_from_slice(values);
        Ok(())
    }
}

pub(crate) fn build_sparse_zero_upload_plan(cpu: &[BabyBearElem]) -> Result<SparseUploadPlan> {
    let mut plan = SparseUploadPlan::new();
    let mut idx = 0usize;
    while idx < cpu.len() {
        while idx < cpu.len() {
            let raw = cpu[idx].as_u32_montgomery();
            if raw != 0 && raw != u32::MAX {
                break;
            }
            idx += 1;
        }
        if idx == cpu.len() {
            break;
        }
        let run_start = idx;
        while idx < cpu.len() {
            let raw = cpu[idx].as_u32_montgomery();
            if raw == 0 || raw == u32::MAX {
                break;
            }
            idx += 1;
        }
        let mut run_end = idx;
        loop {
            let gap_start = idx;
            if idx >= cpu.len() || cpu[idx].as_u32_montgomery() != 0 {
                break;
            }
            idx += 1;
            if idx >= cpu.len() {
                idx = gap_start;
                break;
            }
            let raw = cpu[idx].as_u32_montgomery();
            if raw == 0 || raw == u32::MAX {
                idx = gap_start;
                break;
            }
            while idx < cpu.len() {
                let raw = cpu[idx].as_u32_montgomery();
                if raw == 0 || raw == u32::MAX {
                    break;
                }
                idx += 1;
            }
            run_end = idx;
        }
        plan.push_range(run_start, &cpu[run_start..run_end])?;
    }
    Ok(plan)
}

pub(crate) fn dispatch_sparse_zero_upload(
    hal: &WebGpuHal,
    target: &web_sys::GpuBuffer,
    target_offset: u64,
    target_size: u64,
    plan: &SparseUploadPlan,
    values_name: &'static str,
    ranges_name: &'static str,
    params_name: &'static str,
    layout_name: &'static str,
    kernel_name: &'static str,
    bind_group_name: &'static str,
) -> Result<()> {
    let value_byte_len = byte_len_for::<BabyBearElem>(plan.values.len());
    let range_byte_len = byte_len_for::<u32>(plan.ranges.len());

    let values_buf = hal.create_storage_buffer(
        values_name,
        u64::try_from(value_byte_len)
            .map_err(|_| anyhow!("sparse values byte length exceeds u64"))?,
    )?;
    hal.write_buffer_named(
        &values_buf,
        values_name,
        0,
        bytemuck::cast_slice(plan.values.as_slice()),
    )?;
    let ranges_buf = hal.create_storage_buffer(
        ranges_name,
        u64::try_from(range_byte_len)
            .map_err(|_| anyhow!("sparse range byte length exceeds u64"))?,
    )?;
    hal.write_buffer_named(
        &ranges_buf,
        ranges_name,
        0,
        bytemuck::cast_slice(plan.ranges.as_slice()),
    )?;
    let params = [
        u32::try_from(plan.values.len()).expect("sparse value count exceeds u32"),
        u32::try_from(plan.ranges.len() / 2).expect("sparse range count exceeds u32"),
        0,
        0,
    ];
    let params_buf = hal.create_uniform_buffer(params_name, bytemuck::cast_slice(&params))?;
    let layout = hal.create_bind_group_layout(
        layout_name,
        &[
            WebGpuBindingLayout::storage(0, 0),
            WebGpuBindingLayout::read_only_storage(1, 0),
            WebGpuBindingLayout::read_only_storage(2, 0),
            WebGpuBindingLayout::uniform(3, 16),
        ],
    )?;
    let kernel = hal.create_compute_kernel(
        kernel_name,
        ZEROIZE_SPARSE_UPLOAD_ELEM_WGSL,
        "main",
        &[layout.clone()],
    )?;
    let bind_group = hal.create_bind_group(
        bind_group_name,
        &layout,
        &[
            WebGpuBufferBinding {
                binding: 0,
                buffer: target,
                offset: target_offset,
                size: Some(target_size),
            },
            WebGpuBufferBinding::new(1, &values_buf),
            WebGpuBufferBinding::new(2, &ranges_buf),
            WebGpuBufferBinding::new(3, &params_buf),
        ],
    )?;
    let workgroups = u32::try_from(plan.ranges.len() / 2)
        .expect("sparse range count exceeds u32")
        .div_ceil(WEBGPU_WORKGROUP_SIZE);
    hal.dispatch_compute_1d(&kernel, &bind_group, workgroups);
    Ok(())
}

pub(crate) fn digest_word_offset(digest_offset: usize) -> Result<u32> {
    digest_offset
        .checked_mul(DIGEST_WORDS)
        .and_then(|offset| offset.try_into().ok())
        .ok_or_else(|| anyhow!("WebGPU digest word offset exceeds u32"))
}

pub(crate) fn slice_region_in_bounds(
    len: usize,
    rows: usize,
    cols: usize,
    offset: usize,
    stride: usize,
) -> bool {
    if rows == 0 || cols == 0 {
        return true;
    }
    let Some(last_row_start) = rows.checked_sub(1).and_then(|row| row.checked_mul(stride)) else {
        return false;
    };
    let Some(last_col) = cols.checked_sub(1) else {
        return false;
    };
    let Some(last_idx) = offset
        .checked_add(last_row_start)
        .and_then(|idx| idx.checked_add(last_col))
    else {
        return false;
    };
    last_idx < len
}

pub(crate) fn gather_region_in_bounds(len: usize, idx: usize, size: usize, stride: usize) -> bool {
    if size == 0 {
        return true;
    }
    let Some(last_row) = size.checked_sub(1) else {
        return false;
    };
    let Some(last_idx) = last_row
        .checked_mul(stride)
        .and_then(|value| value.checked_add(idx))
    else {
        return false;
    };
    last_idx < len
}

pub(crate) async fn request_device() -> Result<web_sys::GpuDevice> {
    let global = js_sys::global();
    let navigator = js_sys::Reflect::get(&global, &JsValue::from_str("navigator"))
        .map_err(js_error)
        .and_then(required_js("globalThis.navigator"))?;
    let gpu = js_sys::Reflect::get(&navigator, &JsValue::from_str("gpu"))
        .map_err(js_error)
        .and_then(required_js("navigator.gpu"))?
        .dyn_into::<web_sys::Gpu>()
        .map_err(|_| anyhow!("navigator.gpu is not a GPU object"))?;

    let adapter = match request_adapter(&gpu, false).await? {
        Some(adapter) => adapter,
        None => request_adapter(&gpu, true)
            .await?
            .ok_or_else(|| anyhow!("GPUAdapter is not available"))?,
    };

    let adapter_limits = adapter.limits();
    let max_buffer_size = (adapter_limits.max_buffer_size() as u64)
        .min(WEBGPU_REQUESTED_MAX_BUFFER_BYTES)
        .min(MAX_EXACT_JS_INTEGER);
    let max_storage_buffer_binding_size = (adapter_limits.max_storage_buffer_binding_size() as u64)
        .min(WEBGPU_REQUESTED_MAX_STORAGE_BINDING_BYTES)
        .min(max_buffer_size)
        .min(MAX_EXACT_JS_INTEGER);
    let max_compute_workgroup_storage_size = adapter_limits
        .max_compute_workgroup_storage_size()
        .min(WEBGPU_REQUESTED_MAX_WORKGROUP_STORAGE_BYTES);
    let required_limits = js_sys::Object::new();
    set_required_limit(&required_limits, "maxBufferSize", max_buffer_size)?;
    set_required_limit(
        &required_limits,
        "maxStorageBufferBindingSize",
        max_storage_buffer_binding_size,
    )?;
    set_required_limit(
        &required_limits,
        "maxComputeWorkgroupStorageSize",
        max_compute_workgroup_storage_size as u64,
    )?;
    // Bump storage-buffers-per-stage so the staged multi-stage
    // bind group (10 storage bindings) fits. Default WebGPU is 8.
    let max_storage_buffers_per_stage = adapter_limits
        .max_storage_buffers_per_shader_stage()
        .min(WEBGPU_REQUESTED_MAX_STORAGE_BUFFERS_PER_STAGE);
    set_required_limit(
        &required_limits,
        "maxStorageBuffersPerShaderStage",
        max_storage_buffers_per_stage as u64,
    )?;
    // Bump `maxUniformBufferBindingSize` so `mix_pows`
    // can live in a UBO (CUDA `__constant__` analog).
    let max_uniform_buffer_binding_size = (adapter_limits.max_uniform_buffer_binding_size() as u64)
        .min(WEBGPU_REQUESTED_MAX_UNIFORM_BUFFER_BINDING_BYTES);
    set_required_limit(
        &required_limits,
        "maxUniformBufferBindingSize",
        max_uniform_buffer_binding_size,
    )?;
    let descriptor = web_sys::GpuDeviceDescriptor::new();
    descriptor.set_required_limits(&required_limits);

    JsFuture::from(adapter.request_device_with_descriptor(&descriptor))
        .await
        .map_err(js_error)
        .and_then(required_js("GPUDevice"))?
        .dyn_into::<web_sys::GpuDevice>()
        .map_err(|_| anyhow!("requestDevice did not return a GPUDevice"))
}

pub(crate) fn set_required_limit(
    limits: &js_sys::Object,
    name: &'static str,
    value: u64,
) -> Result<()> {
    js_sys::Reflect::set(
        limits,
        &JsValue::from_str(name),
        &JsValue::from_f64(value as f64),
    )
    .map_err(js_error)?;
    Ok(())
}

pub(crate) async fn request_adapter(
    gpu: &web_sys::Gpu,
    force_fallback: bool,
) -> Result<Option<web_sys::GpuAdapter>> {
    let options = web_sys::GpuRequestAdapterOptions::new();
    options.set_power_preference(web_sys::GpuPowerPreference::HighPerformance);
    let promise = if force_fallback {
        options.set_force_fallback_adapter(true);
        gpu.request_adapter_with_options(&options)
    } else {
        gpu.request_adapter_with_options(&options)
    };

    let value = JsFuture::from(promise).await.map_err(js_error)?;
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }

    Ok(Some(value.dyn_into::<web_sys::GpuAdapter>().map_err(
        |_| anyhow!("requestAdapter did not return a GPUAdapter"),
    )?))
}

pub(crate) fn required_js(name: &'static str) -> impl FnOnce(JsValue) -> Result<JsValue> {
    move |value| {
        if value.is_null() || value.is_undefined() {
            Err(anyhow!("{name} is not available"))
        } else {
            Ok(value)
        }
    }
}

pub(crate) fn js_error(value: JsValue) -> anyhow::Error {
    anyhow!("{value:?}")
}

impl Hal for WebGpuHal {
    type Field = BabyBear;
    type Elem = BabyBearElem;
    type ExtElem = BabyBearExtElem;
    type Buffer<T: Clone + Debug + PartialEq> = WebGpuBuffer<T>;

    fn has_unified_memory(&self) -> bool {
        false
    }

    fn get_hash_suite(&self) -> &HashSuite<Self::Field> {
        self.cpu.get_hash_suite()
    }

    fn alloc_digest(&self, name: &'static str, size: usize) -> Self::Buffer<Digest> {
        self.alloc_shadowed_buffer(name, self.cpu.alloc_digest(name, size))
    }

    fn alloc_elem(&self, name: &'static str, size: usize) -> Self::Buffer<Self::Elem> {
        self.alloc_shadowed_buffer(name, self.cpu.alloc_elem(name, size))
    }

    fn alloc_extelem(&self, name: &'static str, size: usize) -> Self::Buffer<Self::ExtElem> {
        self.alloc_shadowed_buffer(name, self.cpu.alloc_extelem(name, size))
    }

    fn alloc_u32(&self, name: &'static str, size: usize) -> Self::Buffer<u32> {
        self.alloc_shadowed_buffer(name, self.cpu.alloc_u32(name, size))
    }

    fn alloc_elem_init(
        &self,
        name: &'static str,
        size: usize,
        value: Self::Elem,
    ) -> Self::Buffer<Self::Elem> {
        let buffer = self.alloc_shadowed_buffer(name, self.cpu.alloc_elem_init(name, size, value));
        if value == BabyBearElem::ZERO {
            // WebGPU buffers are zero-initialized, matching CpuHal's zero-filled shadow.
            buffer.mark_synced();
        }
        buffer
    }

    fn alloc_extelem_zeroed(&self, name: &'static str, size: usize) -> Self::Buffer<Self::ExtElem> {
        let buffer = self.alloc_shadowed_buffer(name, self.cpu.alloc_extelem_zeroed(name, size));
        // WebGPU buffers are zero-initialized, matching CpuHal's zeroed shadow.
        buffer.mark_synced();
        buffer
    }

    fn copy_from_digest(&self, name: &'static str, slice: &[Digest]) -> Self::Buffer<Digest> {
        self.copy_shadowed_buffer(name, self.cpu.copy_from_digest(name, slice), slice)
    }

    fn copy_from_elem(&self, name: &'static str, slice: &[Self::Elem]) -> Self::Buffer<Self::Elem> {
        self.copy_shadowed_buffer(name, self.cpu.copy_from_elem(name, slice), slice)
    }

    fn copy_from_elem_transpose_zero_pad(
        &self,
        name: &'static str,
        compact_name: &'static str,
        row_major: &[Self::Elem],
        rows: usize,
        cols: usize,
        total_rows: usize,
        cache_key: Option<Digest>,
    ) -> Result<Self::Buffer<Self::Elem>> {
        self.copy_elem_transpose_zero_pad_gpu(
            name,
            compact_name,
            row_major,
            rows,
            cols,
            total_rows,
            cache_key,
        )
    }

    fn copy_from_extelem(
        &self,
        name: &'static str,
        slice: &[Self::ExtElem],
    ) -> Self::Buffer<Self::ExtElem> {
        self.copy_shadowed_buffer(name, self.cpu.copy_from_extelem(name, slice), slice)
    }

    fn copy_from_u32(&self, name: &'static str, slice: &[u32]) -> Self::Buffer<u32> {
        self.copy_shadowed_buffer(name, self.cpu.copy_from_u32(name, slice), slice)
    }

    fn batch_expand_into_evaluate_ntt(
        &self,
        output: &Self::Buffer<Self::Elem>,
        input: &Self::Buffer<Self::Elem>,
        count: usize,
        expand_bits: usize,
    ) {
        let gpu_evaluated = self
            .dispatch_batch_expand_into_evaluate_ntt(output, input, count, expand_bits)
            .unwrap_or_else(|err| panic!("failed to expand and evaluate NTT with WebGPU: {err}"));
        self.finish_hal_op(
            "batch_expand_into_evaluate_ntt",
            gpu_evaluated,
            output,
            || {
                self.cpu.batch_expand_into_evaluate_ntt(
                    output.cpu(),
                    input.cpu(),
                    count,
                    expand_bits,
                );
            },
        );
    }

    fn batch_interpolate_ntt(&self, io: &Self::Buffer<Self::Elem>, count: usize) {
        let gpu_interpolated = self
            .dispatch_batch_interpolate_ntt(io, count)
            .unwrap_or_else(|err| panic!("failed to interpolate NTT with WebGPU: {err}"));
        self.finish_hal_op("batch_interpolate_ntt", gpu_interpolated, io, || {
            self.cpu.batch_interpolate_ntt(io.cpu(), count);
        });
    }

    fn batch_bit_reverse(&self, io: &Self::Buffer<Self::Elem>, count: usize) {
        let row_size = io.size() / count;
        assert_eq!(row_size * count, io.size());
        let bits = crate::core::log2_ceil(row_size);
        assert_eq!(row_size, 1 << bits);
        let gpu_reversed = self
            .dispatch_batch_bit_reverse(io, bits)
            .unwrap_or_else(|err| panic!("failed to bit-reverse with WebGPU: {err}"));
        self.finish_hal_op("batch_bit_reverse", gpu_reversed, io, || {
            self.cpu.batch_bit_reverse(io.cpu(), count);
        });
    }

    fn batch_evaluate_any(
        &self,
        coeffs: &Self::Buffer<Self::Elem>,
        poly_count: usize,
        which: &Self::Buffer<u32>,
        xs: &Self::Buffer<Self::ExtElem>,
        out: &Self::Buffer<Self::ExtElem>,
    ) {
        let po2 = crate::core::log2_ceil(coeffs.size() / poly_count);
        let deg = 1 << po2;
        assert_eq!(poly_count * deg, coeffs.size());
        let eval_count = which.size();
        assert_eq!(xs.size(), eval_count);
        assert_eq!(out.size(), eval_count);
        let gpu_evaluated = self
            .dispatch_batch_evaluate_any(coeffs, which, xs, out, deg)
            .unwrap_or_else(|err| panic!("failed to batch evaluate with WebGPU: {err}"));
        self.finish_hal_op("batch_evaluate_any", gpu_evaluated, out, || {
            self.cpu
                .batch_evaluate_any(coeffs.cpu(), poly_count, which.cpu(), xs.cpu(), out.cpu());
        });
    }

    fn zk_shift(&self, io: &Self::Buffer<Self::Elem>, count: usize) {
        let bits = crate::core::log2_ceil(io.size() / count);
        assert_eq!(io.size(), count * (1 << bits));
        let gpu_shifted = self
            .dispatch_zk_shift(io, bits)
            .unwrap_or_else(|err| panic!("failed to zk_shift with WebGPU: {err}"));
        self.finish_hal_op("zk_shift", gpu_shifted, io, || {
            self.cpu.zk_shift(io.cpu(), count);
        });
    }

    fn mix_poly_coeffs(
        &self,
        out: &Self::Buffer<Self::ExtElem>,
        mix_start: &Self::ExtElem,
        mix: &Self::ExtElem,
        input: &Self::Buffer<Self::Elem>,
        combos: &Self::Buffer<u32>,
        input_size: usize,
        count: usize,
    ) {
        let gpu_mixed = self
            .dispatch_mix_poly_coeffs(out, mix_start, mix, input, combos, input_size, count)
            .unwrap_or_else(|err| panic!("failed to mix polynomial coeffs with WebGPU: {err}"));
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

    fn combos_prepare(
        &self,
        combos: &Self::Buffer<Self::ExtElem>,
        coeff_u: &[Self::ExtElem],
        combo_count: usize,
        cycles: usize,
        reg_sizes: &[u32],
        reg_combo_ids: &[u32],
        mix: &Self::ExtElem,
    ) {
        let gpu_prepared = self
            .dispatch_combos_prepare(
                combos,
                coeff_u,
                combo_count,
                cycles,
                reg_sizes,
                reg_combo_ids,
                mix,
            )
            .unwrap_or_else(|err| panic!("failed to prepare combos with WebGPU: {err}"));
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

    fn combos_divide(
        &self,
        combos: &Self::Buffer<Self::ExtElem>,
        chunks: Vec<(usize, Vec<Self::ExtElem>)>,
        cycles: usize,
    ) {
        let gpu_divided = self
            .dispatch_combos_divide(combos, &chunks, cycles)
            .unwrap_or_else(|err| panic!("failed to divide combos with WebGPU: {err}"));
        self.finish_hal_op("combos_divide", gpu_divided, combos, || {
            self.cpu.combos_divide(combos.cpu(), chunks, cycles);
        });
    }

    fn eltwise_add_elem(
        &self,
        output: &Self::Buffer<Self::Elem>,
        input1: &Self::Buffer<Self::Elem>,
        input2: &Self::Buffer<Self::Elem>,
    ) {
        let gpu_added = self
            .dispatch_eltwise_add_elem(output, input1, input2)
            .unwrap_or_else(|err| panic!("failed to add WebGPU buffers: {err}"));
        self.finish_hal_op("eltwise_add_elem", gpu_added, output, || {
            self.cpu
                .eltwise_add_elem(output.cpu(), input1.cpu(), input2.cpu());
        });
    }

    fn eltwise_sum_extelem(
        &self,
        output: &Self::Buffer<Self::Elem>,
        input: &Self::Buffer<Self::ExtElem>,
    ) {
        let gpu_summed = self
            .dispatch_eltwise_sum_extelem(output, input)
            .unwrap_or_else(|err| panic!("failed to sum WebGPU extension buffers: {err}"));
        self.finish_hal_op("eltwise_sum_extelem", gpu_summed, output, || {
            self.cpu.eltwise_sum_extelem(output.cpu(), input.cpu());
        });
    }

    fn eltwise_copy_elem(
        &self,
        output: &Self::Buffer<Self::Elem>,
        input: &Self::Buffer<Self::Elem>,
    ) {
        let gpu_copied = if output.size() == input.size() {
            if let (Some(input_gpu), Some(output_gpu)) = (input.raw_buffer(), output.raw_buffer()) {
                input
                    .sync_cpu_to_gpu(self)
                    .unwrap_or_else(|err| panic!("failed to sync WebGPU input buffer: {err}"));
                if input_gpu == output_gpu {
                    false
                } else {
                    self.copy_gpu_buffer_named(
                        output.name(),
                        input_gpu,
                        input.byte_offset(),
                        output_gpu,
                        output.byte_offset(),
                        byte_len_for::<Self::Elem>(output.size()),
                    )
                    .unwrap_or_else(|err| panic!("failed to copy WebGPU buffer: {err}"));
                    true
                }
            } else {
                false
            }
        } else {
            false
        };

        self.finish_hal_op("eltwise_copy_elem", gpu_copied, output, || {
            self.cpu.eltwise_copy_elem(output.cpu(), input.cpu());
        });
    }

    fn eltwise_copy_elem_slice(
        &self,
        into: &Self::Buffer<Self::Elem>,
        from: &[Self::Elem],
        from_rows: usize,
        from_cols: usize,
        from_offset: usize,
        from_stride: usize,
        into_offset: usize,
        into_stride: usize,
    ) {
        let gpu_copied = self
            .dispatch_eltwise_copy_elem_slice(
                into,
                from,
                from_rows,
                from_cols,
                from_offset,
                from_stride,
                into_offset,
                into_stride,
            )
            .unwrap_or_else(|err| panic!("failed to copy WebGPU element slice: {err}"));
        self.finish_hal_op("eltwise_copy_elem_slice", gpu_copied, into, || {
            self.cpu.eltwise_copy_elem_slice(
                into.cpu(),
                from,
                from_rows,
                from_cols,
                from_offset,
                from_stride,
                into_offset,
                into_stride,
            );
        });
    }

    fn eltwise_zeroize_elem(&self, elems: &Self::Buffer<Self::Elem>) {
        let gpu_zeroized = self
            .dispatch_zeroize_elem(elems)
            .unwrap_or_else(|err| panic!("failed to zeroize WebGPU buffer: {err}"));
        self.finish_hal_op("eltwise_zeroize_elem", gpu_zeroized, elems, || {
            self.cpu.eltwise_zeroize_elem(elems.cpu());
        });
    }

    fn fri_fold(
        &self,
        output: &Self::Buffer<Self::Elem>,
        input: &Self::Buffer<Self::Elem>,
        mix: &Self::ExtElem,
    ) {
        let count = output.size() / Self::ExtElem::EXT_SIZE;
        assert_eq!(output.size(), count * Self::ExtElem::EXT_SIZE);
        assert_eq!(input.size(), output.size() * crate::FRI_FOLD);
        let gpu_folded = self
            .dispatch_fri_fold(output, input, mix)
            .unwrap_or_else(|err| panic!("failed to FRI fold with WebGPU: {err}"));
        self.finish_hal_op("fri_fold", gpu_folded, output, || {
            self.cpu.fri_fold(output.cpu(), input.cpu(), mix);
        });
    }

    fn hash_rows(&self, output: &Self::Buffer<Digest>, matrix: &Self::Buffer<Self::Elem>) {
        let row_size = output.size();
        let col_size = matrix.size() / output.size();
        assert_eq!(matrix.size(), col_size * row_size);
        let gpu_hashed = self
            .dispatch_poseidon2_hash_rows(output, matrix, row_size, col_size)
            .unwrap_or_else(|err| panic!("failed to hash rows with WebGPU Poseidon2: {err}"));
        self.finish_hal_op("hash_rows", gpu_hashed, output, || {
            self.cpu.hash_rows(output.cpu(), matrix.cpu());
        });
    }

    fn hash_fold(&self, io: &Self::Buffer<Digest>, input_size: usize, output_size: usize) {
        assert!(io.size() >= 2 * input_size);
        assert_eq!(input_size, 2 * output_size);
        let gpu_hashed = self
            .dispatch_poseidon2_hash_fold(io, input_size, output_size)
            .unwrap_or_else(|err| panic!("failed to hash fold with WebGPU Poseidon2: {err}"));
        self.finish_hal_op("hash_fold", gpu_hashed, io, || {
            self.cpu.hash_fold(io.cpu(), input_size, output_size);
        });
    }

    fn gather_sample(
        &self,
        dst: &Self::Buffer<Self::Elem>,
        src: &Self::Buffer<Self::Elem>,
        idx: usize,
        size: usize,
        stride: usize,
    ) {
        let gpu_gathered = self
            .dispatch_gather_sample(dst, src, idx, size, stride)
            .unwrap_or_else(|err| panic!("failed to gather WebGPU sample: {err}"));
        self.finish_hal_op("gather_sample", gpu_gathered, dst, || {
            self.cpu
                .gather_sample(dst.cpu(), src.cpu(), idx, size, stride);
        });
    }

    fn scatter(
        &self,
        into: &Self::Buffer<Self::Elem>,
        index: &[u32],
        offsets: &[u32],
        values: &[Self::Elem],
    ) {
        if !index.windows(2).any(|window| window[0] < window[1]) {
            return;
        }
        let gpu_authoritative = self.gpu_authoritative();
        // A sparse GPU scatter cannot make a stale destination fully current.
        // In mirror mode, skip that unusable probe and keep CPU authoritative.
        if !gpu_authoritative && into.raw_buffer().is_some() && !into.gpu_is_current() {
            self.cpu.scatter(into.cpu(), index, offsets, values);
            self.diagnostics.record_cpu_mirror("scatter");
            into.mark_cpu_result(false);
            return;
        }
        let gpu_scattered = self
            .dispatch_scatter(into, index, offsets, values, gpu_authoritative)
            .unwrap_or_else(|err| panic!("failed to scatter WebGPU buffers: {err}"));
        if gpu_scattered && !gpu_authoritative {
            self.cpu.scatter(into.cpu(), index, offsets, values);
            self.record_gpu_result_with_cpu_mirror("scatter", true);
            into.mark_cpu_result(true);
            return;
        }
        self.finish_hal_op("scatter", gpu_scattered, into, || {
            self.cpu.scatter(into.cpu(), index, offsets, values);
        });
    }

    fn prefix_products(&self, io: &Self::Buffer<Self::ExtElem>) {
        let gpu_computed = self
            .dispatch_prefix_products(io)
            .unwrap_or_else(|err| panic!("failed to compute WebGPU prefix products: {err}"));
        self.finish_hal_op("prefix_products", gpu_computed, io, || {
            self.cpu.prefix_products(io.cpu());
        });
    }
}
