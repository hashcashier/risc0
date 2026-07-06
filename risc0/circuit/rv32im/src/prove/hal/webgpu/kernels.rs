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

//! Kernel construction and dispatch for the circuit HAL: witgen
//! per-arm prewarm/lookup/dispatch, the TopAccum arm-5 probes, the
//! accum scan kernels, and the direct-accum kernel cache.

use super::*;
#[allow(unused_imports)]
use super::{accum_wgsl::*, phases::*, session::*, traits::*, witgen_wgsl::*};

pub(crate) const ACCUM_MACHINE_COLUMN_CARRY_WGSL: &str = r#"
const P: u32 = 2013265921u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    rows: u32,
    cols: u32,
    split: u32,
    carry_cols: u32,
    base: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<storage, read_write> accum: ElemBuffer;
@group(0) @binding(1) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
}

@compute @workgroup_size(128)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let carry_col = gid.x;
    if (carry_col >= params.carry_cols) {
        return;
    }
    let lane = carry_col & 3u;
    let col = params.split + carry_col;
    let terminal_col = params.cols - 4u + lane;
    var row = 0u;
    loop {
        if (row >= params.rows) {
            break;
        }
        let back = select(row - 1u, params.rows - 1u, row == 0u);
        let prev_idx = params.base + terminal_col * params.rows + back;
        let out_idx = params.base + col * params.rows + row;
        accum.data[out_idx] = add(accum.data[out_idx], accum.data[prev_idx]);
        row = row + 1u;
    }
}
"#;

pub(crate) const ACCUM_TERMINAL_EXT_PREFIX_WGSL: &str = r#"
const P: u32 = 2013265921u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    rows: u32,
    cols: u32,
    base: u32,
    _pad0: u32,
};

@group(0) @binding(0) var<storage, read_write> accum: ElemBuffer;
@group(0) @binding(1) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
}

@compute @workgroup_size(4)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let lane = gid.x;
    if (lane >= 4u || params.rows == 0u || params.cols < 4u) {
        return;
    }
    let col = params.cols - 4u + lane;
    var running = 0u;
    var row = 0u;
    loop {
        if (row >= params.rows) {
            break;
        }
        let idx = params.base + col * params.rows + row;
        running = add(running, accum.data[idx]);
        accum.data[idx] = running;
        row = row + 1u;
    }
}
"#;

pub fn dispatch_accum_terminal_ext_prefix(
    hal: &WebGpuHal,
    accum: &WebGpuBuffer<Val>,
    rows: usize,
    cols: usize,
) -> Result<bool> {
    if rows == 0 || accum.size() == 0 {
        return Ok(true);
    }
    if accum.size() != rows * cols || cols < ExtVal::EXT_SIZE {
        return Ok(false);
    }

    let Some(accum_gpu) = accum.raw_buffer() else {
        return Ok(false);
    };
    if accum.name() == "accum" {
        if !accum.sync_cpu_to_gpu_zero_default_sparse_named(hal, "accum")? {
            accum.sync_cpu_to_gpu(hal)?;
        }
    } else {
        accum.sync_cpu_to_gpu(hal)?;
    }

    let params = [
        u32::try_from(rows).context("RV32IM terminal prefix rows exceeds u32")?,
        u32::try_from(cols).context("RV32IM terminal prefix cols exceeds u32")?,
        u32::try_from(accum.byte_offset() / std::mem::size_of::<Val>() as u64)
            .context("RV32IM terminal prefix buffer offset exceeds u32")?,
        0,
    ];
    let params = hal.create_uniform_buffer(
        "rv32im_accum_terminal_ext_prefix_params",
        bytemuck::cast_slice(&params),
    )?;
    let layout = hal.create_bind_group_layout(
        "rv32im_accum_terminal_ext_prefix_layout",
        &[
            WebGpuBindingLayout::storage(0, 0),
            WebGpuBindingLayout::uniform(1, 16),
        ],
    )?;
    let kernel = hal.create_compute_kernel(
        "rv32im_accum_terminal_ext_prefix",
        ACCUM_TERMINAL_EXT_PREFIX_WGSL,
        "main",
        &[layout.clone()],
    )?;
    let bind_group = hal.create_bind_group(
        "rv32im_accum_terminal_ext_prefix_bind_group",
        &layout,
        &[
            WebGpuBufferBinding::new(0, accum_gpu),
            WebGpuBufferBinding {
                binding: 1,
                buffer: &params,
                offset: 0,
                size: Some(16),
            },
        ],
    )?;
    hal.dispatch_compute_1d(&kernel, &bind_group, 1);
    accum.mark_gpu_dirty();
    Ok(true)
}

pub fn dispatch_accum_machine_column_carry(
    hal: &WebGpuHal,
    accum: &WebGpuBuffer<Val>,
    rows: usize,
    cols: usize,
    split: usize,
) -> Result<bool> {
    if rows == 0 || accum.size() == 0 {
        return Ok(true);
    }
    if accum.size() != rows * cols || cols <= split || (cols - split) % ExtVal::EXT_SIZE != 0 {
        return Ok(false);
    }
    let machine_columns = (cols - split) / ExtVal::EXT_SIZE;
    if machine_columns <= 1 {
        return Ok(true);
    }

    let Some(accum_gpu) = accum.raw_buffer() else {
        return Ok(false);
    };
    if accum.name() == "accum" {
        if !accum.sync_cpu_to_gpu_zero_default_sparse_named(hal, "accum")? {
            accum.sync_cpu_to_gpu(hal)?;
        }
    } else {
        accum.sync_cpu_to_gpu(hal)?;
    }

    let carry_cols = (machine_columns - 1) * ExtVal::EXT_SIZE;
    let params = [
        u32::try_from(rows).context("RV32IM accum carry rows exceeds u32")?,
        u32::try_from(cols).context("RV32IM accum carry cols exceeds u32")?,
        u32::try_from(split).context("RV32IM accum carry split exceeds u32")?,
        u32::try_from(carry_cols).context("RV32IM accum carry columns exceeds u32")?,
        u32::try_from(accum.byte_offset() / std::mem::size_of::<Val>() as u64)
            .context("RV32IM accum carry buffer offset exceeds u32")?,
        0,
        0,
        0,
    ];
    let params = hal.create_uniform_buffer(
        "rv32im_accum_machine_column_carry_params",
        bytemuck::cast_slice(&params),
    )?;
    let layout = hal.create_bind_group_layout(
        "rv32im_accum_machine_column_carry_layout",
        &[
            WebGpuBindingLayout::storage(0, 0),
            WebGpuBindingLayout::uniform(1, 32),
        ],
    )?;
    let kernel = hal.create_compute_kernel(
        "rv32im_accum_machine_column_carry",
        ACCUM_MACHINE_COLUMN_CARRY_WGSL,
        "main",
        &[layout.clone()],
    )?;
    let bind_group = hal.create_bind_group(
        "rv32im_accum_machine_column_carry_bind_group",
        &layout,
        &[
            WebGpuBufferBinding::new(0, accum_gpu),
            WebGpuBufferBinding {
                binding: 1,
                buffer: &params,
                offset: 0,
                size: Some(32),
            },
        ],
    )?;
    let workgroups = u32::try_from(carry_cols)
        .context("RV32IM accum carry dispatch columns exceeds u32")?
        .div_ceil(128);
    hal.dispatch_compute_1d(&kernel, &bind_group, workgroups);
    accum.mark_gpu_dirty();
    Ok(true)
}

impl WebGpuCircuitHal {
    pub(crate) fn new(hal: Rc<WebGpuHal>) -> Self {
        Self {
            hal,
            witgen_replace_arm_mask: Cell::new(0),
        }
    }

    /// Set this prove's replacement arm mask; mirrors into the legacy
    /// `rust_steps` static so diagnostics/tests keep seeing the mask of
    /// the most recent pre-witgen dispatch.
    pub(crate) fn set_witgen_replace_arm_mask(&self, mask: u16) {
        self.witgen_replace_arm_mask.set(mask);
        crate::prove::hal::rust_steps::set_witgen_gpu_replace_arm_mask(mask);
    }

    pub(crate) fn start_witgen_replacement_prewarm(&self) -> Result<WitgenReplacementPrewarm> {
        let mut prewarm = WitgenReplacementPrewarm::default();
        if !WITGEN_GPU_REPLACE_ENABLED.load(Ordering::SeqCst) {
            return Ok(prewarm);
        }

        use crate::prove::wgsl_pruner::{
            assemble_arm_kernel, patch_extern_get_diff_count, patch_extern_get_memory_txn,
            EXEC_TOP_CHUNK1_WGSL, MEM0_EXTRA_CHUNK_DELTAS, MEM1_EXTRA_CHUNK_DELTAS,
            MISC0_EXTRA_CHUNK_DELTAS, TOP_CHUNK0_ARM_DELTAS,
        };

        let arm_layout = self.hal.create_bind_group_layout(
            "witgen_arm_arm_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::read_only_storage(6, 0),
                WebGpuBindingLayout::read_only_storage(7, 0),
                WebGpuBindingLayout::read_only_storage(8, 0),
                WebGpuBindingLayout::read_only_storage(9, 0),
            ],
        )?;
        let arm_layouts = [arm_layout.clone()];

        for (arm_idx, (label, delta, sub_fn)) in TOP_CHUNK0_ARM_DELTAS.iter().enumerate() {
            if !is_zero_back_reg_arm(arm_idx) || !is_witgen_replace_supported_arm(arm_idx) {
                continue;
            }
            let ready = WITGEN_ARM_KERNELS.with(|cell| cell.borrow().contains_key(*label));
            let pending =
                WITGEN_ARM_PENDING_KERNELS.with(|cell| cell.borrow().contains_key(*label));
            if !ready && !pending {
                let wrapper = synth_arm_wrapper(label, sub_fn, arm_idx);
                let module = assemble_arm_kernel(delta, &wrapper);
                let entry = format!("witgen_arm_{}_main", label);
                let started = self.hal.start_compute_kernel_async(
                    "witgen_arm_arm_kernel",
                    &module,
                    &entry,
                    &arm_layouts,
                )?;
                WITGEN_ARM_PENDING_KERNELS
                    .with(|cell| cell.borrow_mut().insert(*label, started.clone()));
                prewarm.arm_chunk0.push((*label, started));
            }
        }

        for &(minor, extra_label, extra_delta, extra_sub_fn) in MISC0_EXTRA_CHUNK_DELTAS {
            let ready = WITGEN_MISC0_EXTRA_KERNELS.with(|cell| cell.borrow().contains_key(&minor));
            let pending =
                WITGEN_MISC0_EXTRA_PENDING_KERNELS.with(|cell| cell.borrow().contains_key(&minor));
            if ready || pending {
                continue;
            }
            let wrapper = synth_arm_wrapper(extra_label, extra_sub_fn, 0);
            let module = assemble_arm_kernel(extra_delta, &wrapper);
            let entry = format!("witgen_arm_{extra_label}_main");
            let started = self.hal.start_compute_kernel_async(
                "witgen_arm_misc0_extra_kernel",
                &module,
                &entry,
                &arm_layouts,
            )?;
            WITGEN_MISC0_EXTRA_PENDING_KERNELS
                .with(|cell| cell.borrow_mut().insert(minor, started.clone()));
            prewarm.misc0_extra.push((minor, extra_label, started));
        }

        if is_witgen_replace_supported_arm(5) {
            for &(minor, extra_label, extra_delta, extra_sub_fn) in MEM0_EXTRA_CHUNK_DELTAS {
                if !witgen_mem0_replace_minor_enabled(minor) {
                    continue;
                }
                let ready =
                    WITGEN_MEM0_EXTRA_KERNELS.with(|cell| cell.borrow().contains_key(&minor));
                let pending = WITGEN_MEM0_EXTRA_PENDING_KERNELS
                    .with(|cell| cell.borrow().contains_key(&minor));
                if ready || pending {
                    continue;
                }
                let wrapper = synth_arm_wrapper(extra_label, extra_sub_fn, 5);
                let module = assemble_arm_kernel(extra_delta, &wrapper);
                let entry = format!("witgen_arm_{extra_label}_main");
                let started = self.hal.start_compute_kernel_async(
                    "witgen_arm_mem0_extra_kernel",
                    &module,
                    &entry,
                    &arm_layouts,
                )?;
                WITGEN_MEM0_EXTRA_PENDING_KERNELS
                    .with(|cell| cell.borrow_mut().insert(minor, started.clone()));
                WITGEN_GPU_MEM0_EXTRA_PREWARM_REQUESTS.fetch_add(1, Ordering::SeqCst);
                prewarm.mem0_extra.push((minor, extra_label, started));
            }
        }

        if is_witgen_replace_supported_arm(6) {
            for &(minor, extra_label, extra_delta, extra_sub_fn) in MEM1_EXTRA_CHUNK_DELTAS {
                if !witgen_mem1_replace_minor_enabled(minor) {
                    continue;
                }
                let ready =
                    WITGEN_MEM1_EXTRA_KERNELS.with(|cell| cell.borrow().contains_key(&minor));
                let pending = WITGEN_MEM1_EXTRA_PENDING_KERNELS
                    .with(|cell| cell.borrow().contains_key(&minor));
                if ready || pending {
                    continue;
                }
                let wrapper = synth_arm_wrapper(extra_label, extra_sub_fn, 6);
                let module = assemble_arm_kernel(extra_delta, &wrapper);
                let entry = format!("witgen_arm_{extra_label}_main");
                let started = self.hal.start_compute_kernel_async(
                    "witgen_arm_mem1_extra_kernel",
                    &module,
                    &entry,
                    &arm_layouts,
                )?;
                WITGEN_MEM1_EXTRA_PENDING_KERNELS
                    .with(|cell| cell.borrow_mut().insert(minor, started.clone()));
                WITGEN_GPU_MEM1_EXTRA_PREWARM_REQUESTS.fetch_add(1, Ordering::SeqCst);
                prewarm.mem1_extra.push((minor, extra_label, started));
            }
        }

        let patched_chunk1 = patch_extern_get_diff_count(EXEC_TOP_CHUNK1_WGSL);
        let patched_chunk1 = patch_extern_get_memory_txn(&patched_chunk1);
        for (arm_idx, (label, _, _)) in TOP_CHUNK0_ARM_DELTAS.iter().enumerate() {
            if !is_zero_back_reg_arm(arm_idx) || !is_witgen_replace_supported_arm(arm_idx) {
                continue;
            }
            let ready = WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow().contains_key(*label));
            let pending =
                WITGEN_ARM_PENDING_KERNELS_CHUNK1.with(|cell| cell.borrow().contains_key(*label));
            if ready || pending {
                continue;
            }
            let wrapper = synth_arm_chunk1_wrapper(label, arm_idx);
            let mut module = String::with_capacity(patched_chunk1.len() + wrapper.len() + 2);
            module.push_str(&patched_chunk1);
            if !module.ends_with('\n') {
                module.push('\n');
            }
            module.push_str(&wrapper);
            let entry = format!("witgen_arm_{}_c1_main", label);
            let started = self.hal.start_compute_kernel_async(
                "witgen_arm_arm_chunk1_kernel",
                &module,
                &entry,
                &arm_layouts,
            )?;
            WITGEN_ARM_PENDING_KERNELS_CHUNK1
                .with(|cell| cell.borrow_mut().insert(*label, started.clone()));
            prewarm.arm_chunk1.push((*label, started));
        }

        Ok(prewarm)
    }

    pub(crate) async fn finish_witgen_replacement_prewarm(
        hal: Rc<WebGpuHal>,
        prewarm: WitgenReplacementPrewarm,
    ) {
        let arm_chunk0_requested = prewarm.arm_chunk0.len();
        let arm_chunk1_requested = prewarm.arm_chunk1.len();
        let misc0_extra_requested = prewarm.misc0_extra.len();
        let mem0_extra_requested = prewarm.mem0_extra.len();
        let mem1_extra_requested = prewarm.mem1_extra.len();

        for (label, started) in prewarm.arm_chunk0 {
            match hal.finish_compute_kernel_async(started).await {
                Ok(kernel) => {
                    WITGEN_ARM_KERNELS.with(|cell| cell.borrow_mut().insert(label, kernel));
                    WITGEN_ARM_PENDING_KERNELS.with(|cell| cell.borrow_mut().remove(label));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "witgen_arm_prewarm arm={} DONE",
                        label
                    ));
                }
                Err(err) => {
                    WITGEN_ARM_PENDING_KERNELS.with(|cell| cell.borrow_mut().remove(label));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "witgen_arm_prewarm arm={} FAILED err={err:?}",
                        label
                    ));
                }
            }
        }

        for (minor, label, started) in prewarm.misc0_extra {
            match hal.finish_compute_kernel_async(started).await {
                Ok(kernel) => {
                    WITGEN_MISC0_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(minor, kernel));
                    WITGEN_MISC0_EXTRA_PENDING_KERNELS
                        .with(|cell| cell.borrow_mut().remove(&minor));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "witgen_arm_prewarm arm={} DONE",
                        label
                    ));
                }
                Err(err) => {
                    WITGEN_MISC0_EXTRA_PENDING_KERNELS
                        .with(|cell| cell.borrow_mut().remove(&minor));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "witgen_arm_prewarm arm={} FAILED err={err:?}",
                        label
                    ));
                }
            }
        }

        for (minor, label, started) in prewarm.mem0_extra {
            match hal.finish_compute_kernel_async(started).await {
                Ok(kernel) => {
                    WITGEN_MEM0_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(minor, kernel));
                    WITGEN_MEM0_EXTRA_PENDING_KERNELS.with(|cell| cell.borrow_mut().remove(&minor));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "witgen_arm_prewarm arm={} DONE",
                        label
                    ));
                }
                Err(err) => {
                    WITGEN_MEM0_EXTRA_PENDING_KERNELS.with(|cell| cell.borrow_mut().remove(&minor));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "witgen_arm_prewarm arm={} FAILED err={err:?}",
                        label
                    ));
                }
            }
        }

        for (minor, label, started) in prewarm.mem1_extra {
            match hal.finish_compute_kernel_async(started).await {
                Ok(kernel) => {
                    WITGEN_MEM1_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(minor, kernel));
                    WITGEN_MEM1_EXTRA_PENDING_KERNELS.with(|cell| cell.borrow_mut().remove(&minor));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "witgen_arm_prewarm arm={} DONE",
                        label
                    ));
                }
                Err(err) => {
                    WITGEN_MEM1_EXTRA_PENDING_KERNELS.with(|cell| cell.borrow_mut().remove(&minor));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "witgen_arm_prewarm arm={} FAILED err={err:?}",
                        label
                    ));
                }
            }
        }

        for (label, started) in prewarm.arm_chunk1 {
            match hal.finish_compute_kernel_async(started).await {
                Ok(kernel) => {
                    WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(label, kernel));
                    WITGEN_ARM_PENDING_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().remove(label));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "witgen_arm_prewarm arm={}_chunk1 DONE",
                        label
                    ));
                }
                Err(err) => {
                    WITGEN_ARM_PENDING_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().remove(label));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "witgen_arm_prewarm arm={}_chunk1 FAILED err={err:?}",
                        label
                    ));
                }
            }
        }

        if misc0_extra_requested != 0 {
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "witgen_arm_prewarm misc0_extra requested={misc0_extra_requested}",
            ));
        }
        if mem0_extra_requested != 0 {
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "witgen_arm_prewarm mem0_extra requested={mem0_extra_requested}",
            ));
        }
        if mem1_extra_requested != 0 {
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "witgen_arm_prewarm mem1_extra requested={mem1_extra_requested}",
            ));
        }
        if arm_chunk0_requested != 0 {
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "witgen_arm_prewarm ALL arms requested={arm_chunk0_requested}",
            ));
        }
        if arm_chunk1_requested != 0 {
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "witgen_arm_prewarm chunk1 ALL requested={arm_chunk1_requested}",
            ));
        }
    }

    pub(crate) async fn finish_pending_witgen_replace_arm_async(
        &self,
        arm_idx: usize,
        preflight: &PreflightTrace,
        diff_only_mode: bool,
    ) -> Result<()> {
        use crate::prove::wgsl_pruner::{
            MEM0_EXTRA_CHUNK_DELTAS, MEM1_EXTRA_CHUNK_DELTAS, MISC0_EXTRA_CHUNK_DELTAS,
            TOP_CHUNK0_ARM_DELTAS,
        };

        let Some((label, _, _)) = TOP_CHUNK0_ARM_DELTAS.get(arm_idx) else {
            return Ok(());
        };

        if let Some(started) =
            WITGEN_ARM_PENDING_KERNELS.with(|cell| cell.borrow_mut().remove(label))
        {
            let kernel = self.hal.finish_compute_kernel_async(started).await?;
            WITGEN_ARM_KERNELS.with(|cell| cell.borrow_mut().insert(*label, kernel));
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "witgen_arm_prewarm_pending arm={} chunk0 DONE",
                label,
            ));
        }

        if let Some(started) =
            WITGEN_ARM_PENDING_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().remove(label))
        {
            let kernel = self.hal.finish_compute_kernel_async(started).await?;
            WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(*label, kernel));
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "witgen_arm_prewarm_pending arm={} chunk1 DONE",
                label,
            ));
        }

        if arm_idx == 0 {
            let needed =
                needed_extra_minors(preflight, arm_idx, MISC0_EXTRA_CHUNK_DELTAS, diff_only_mode);
            for minor in needed {
                let Some(started) = WITGEN_MISC0_EXTRA_PENDING_KERNELS
                    .with(|cell| cell.borrow_mut().remove(&minor))
                else {
                    continue;
                };
                let kernel = self.hal.finish_compute_kernel_async(started).await?;
                WITGEN_MISC0_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(minor, kernel));
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_prewarm_pending misc0_minor={} DONE",
                    minor,
                ));
            }
        }

        if arm_idx == 5 {
            let needed =
                needed_extra_minors(preflight, arm_idx, MEM0_EXTRA_CHUNK_DELTAS, diff_only_mode);
            for minor in needed {
                let Some(started) =
                    WITGEN_MEM0_EXTRA_PENDING_KERNELS.with(|cell| cell.borrow_mut().remove(&minor))
                else {
                    continue;
                };
                let kernel = self.hal.finish_compute_kernel_async(started).await?;
                WITGEN_MEM0_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(minor, kernel));
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_prewarm_pending mem0_minor={} DONE",
                    minor,
                ));
            }
        }

        if arm_idx == 6 {
            let needed =
                needed_extra_minors(preflight, arm_idx, MEM1_EXTRA_CHUNK_DELTAS, diff_only_mode);
            for minor in needed {
                let Some(started) =
                    WITGEN_MEM1_EXTRA_PENDING_KERNELS.with(|cell| cell.borrow_mut().remove(&minor))
                else {
                    continue;
                };
                let kernel = self.hal.finish_compute_kernel_async(started).await?;
                WITGEN_MEM1_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(minor, kernel));
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_prewarm_pending mem1_minor={} DONE",
                    minor,
                ));
            }
        }

        Ok(())
    }

    /// Kick off the witgen kernel Tint
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
        let replacement_prewarm = match self.start_witgen_replacement_prewarm() {
            Ok(prewarm) => prewarm,
            Err(err) => {
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_prewarm start_FAILED err={err:?}"
                ));
                WitgenReplacementPrewarm::default()
            }
        };
        let hal = self.hal.clone();
        wasm_bindgen_futures::spawn_local(async move {
            // Compile the per-arm replacement kernels in the
            // background. Chrome pipelines createComputePipelineAsync
            // internally, so the compiles overlap with each other and
            // with session execution.
            let _t = WebGpuStageTimer::new("witgen_prewarm_async");
            Self::finish_witgen_replacement_prewarm(hal.clone(), replacement_prewarm).await;
        });
    }

    pub(crate) fn ready_witgen_replace_mask(&self, preflight: &PreflightTrace) -> u16 {
        use crate::prove::wgsl_pruner::{
            MEM0_EXTRA_CHUNK_DELTAS, MEM1_EXTRA_CHUNK_DELTAS, MISC0_EXTRA_CHUNK_DELTAS,
            MISC2_EXTRA_CHUNK_DELTAS, TOP_CHUNK0_ARM_DELTAS,
        };

        let mut has_replace_cycle = [false; 13];
        let mut needs_misc0_extra_chunks = std::collections::BTreeSet::new();
        let mut needs_misc2_extra_chunks = std::collections::BTreeSet::new();
        let mut needs_mem0_extra_chunks = std::collections::BTreeSet::new();
        let mut needs_mem1_extra_chunks = std::collections::BTreeSet::new();
        for cycle in &preflight.cycles {
            let arm = cycle.major as usize;
            if arm < has_replace_cycle.len() && is_witgen_replace_cycle(cycle.major, cycle.minor) {
                has_replace_cycle[arm] = true;
                if cycle.major == 0
                    && MISC0_EXTRA_CHUNK_DELTAS
                        .iter()
                        .any(|(minor, _, _, _)| *minor == cycle.minor)
                {
                    needs_misc0_extra_chunks.insert(cycle.minor);
                }
                if cycle.major == 2
                    && MISC2_EXTRA_CHUNK_DELTAS
                        .iter()
                        .any(|(minor, _, _, _)| *minor == cycle.minor)
                {
                    needs_misc2_extra_chunks.insert(cycle.minor);
                }
                if cycle.major == 5
                    && MEM0_EXTRA_CHUNK_DELTAS
                        .iter()
                        .any(|(minor, _, _, _)| *minor == cycle.minor)
                {
                    needs_mem0_extra_chunks.insert(cycle.minor);
                }
                if cycle.major == 6
                    && MEM1_EXTRA_CHUNK_DELTAS
                        .iter()
                        .any(|(minor, _, _, _)| *minor == cycle.minor)
                {
                    needs_mem1_extra_chunks.insert(cycle.minor);
                }
            }
        }

        let mut mask = 0u16;
        for (arm_idx, (label, _, _)) in TOP_CHUNK0_ARM_DELTAS.iter().enumerate() {
            if arm_idx >= has_replace_cycle.len()
                || !has_replace_cycle[arm_idx]
                || !is_zero_back_reg_arm(arm_idx)
                || !is_witgen_replace_supported_arm(arm_idx)
            {
                continue;
            }
            let chunk0_ready = WITGEN_ARM_KERNELS.with(|cell| cell.borrow().contains_key(*label));
            let chunk1_ready =
                WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow().contains_key(*label));
            let misc0_extra_ready = arm_idx != 0
                || WITGEN_MISC0_EXTRA_KERNELS.with(|cell| {
                    let kernels = cell.borrow();
                    needs_misc0_extra_chunks
                        .iter()
                        .all(|minor| kernels.contains_key(minor))
                });
            let misc2_extra_ready = arm_idx != 2
                || WITGEN_MISC2_EXTRA_KERNELS.with(|cell| {
                    let kernels = cell.borrow();
                    needs_misc2_extra_chunks
                        .iter()
                        .all(|minor| kernels.contains_key(minor))
                });
            let mem0_extra_ready = arm_idx != 5
                || WITGEN_MEM0_EXTRA_KERNELS.with(|cell| {
                    let kernels = cell.borrow();
                    needs_mem0_extra_chunks
                        .iter()
                        .all(|minor| kernels.contains_key(minor))
                });
            let mem1_extra_ready = arm_idx != 6
                || WITGEN_MEM1_EXTRA_KERNELS.with(|cell| {
                    let kernels = cell.borrow();
                    needs_mem1_extra_chunks
                        .iter()
                        .all(|minor| kernels.contains_key(minor))
                });
            if chunk0_ready
                && chunk1_ready
                && misc0_extra_ready
                && misc2_extra_ready
                && mem0_extra_ready
                && mem1_extra_ready
            {
                mask |= 1u16 << arm_idx;
            }
        }
        mask
    }

    pub(crate) async fn ensure_witgen_replace_arm_ready_async(
        &self,
        arm_idx: usize,
        preflight: &PreflightTrace,
        diff_only_mode: bool,
    ) -> Result<()> {
        use crate::prove::wgsl_pruner::{
            assemble_arm_kernel, patch_extern_get_diff_count, patch_extern_get_memory_txn,
            EXEC_TOP_CHUNK1_WGSL, MEM0_EXTRA_CHUNK_DELTAS, MEM1_EXTRA_CHUNK_DELTAS,
            MISC0_EXTRA_CHUNK_DELTAS, MISC2_EXTRA_CHUNK_DELTAS, TOP_CHUNK0_ARM_DELTAS,
        };

        if !is_zero_back_reg_arm(arm_idx) {
            return Ok(());
        }
        let Some((label, delta, sub_fn)) = TOP_CHUNK0_ARM_DELTAS.get(arm_idx) else {
            return Ok(());
        };
        self.finish_pending_witgen_replace_arm_async(arm_idx, preflight, diff_only_mode)
            .await?;
        let chunk0_ready = WITGEN_ARM_KERNELS.with(|cell| cell.borrow().contains_key(*label));
        let chunk1_ready =
            WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow().contains_key(*label));
        let misc0_needed_extra_minors = if arm_idx == 0 {
            needed_extra_minors(preflight, arm_idx, MISC0_EXTRA_CHUNK_DELTAS, diff_only_mode)
        } else {
            std::collections::BTreeSet::new()
        };
        let misc2_needed_extra_minors = if arm_idx == 2 {
            needed_extra_minors(preflight, arm_idx, MISC2_EXTRA_CHUNK_DELTAS, diff_only_mode)
        } else {
            std::collections::BTreeSet::new()
        };
        let mem0_needed_extra_minors = if arm_idx == 5 {
            needed_extra_minors(preflight, arm_idx, MEM0_EXTRA_CHUNK_DELTAS, diff_only_mode)
        } else {
            std::collections::BTreeSet::new()
        };
        let mem1_needed_extra_minors = if arm_idx == 6 {
            needed_extra_minors(preflight, arm_idx, MEM1_EXTRA_CHUNK_DELTAS, diff_only_mode)
        } else {
            std::collections::BTreeSet::new()
        };
        let misc0_extras_ready = arm_idx != 0
            || WITGEN_MISC0_EXTRA_KERNELS.with(|cell| {
                let kernels = cell.borrow();
                misc0_needed_extra_minors
                    .iter()
                    .all(|minor| kernels.contains_key(minor))
            });
        let misc2_extras_ready = arm_idx != 2
            || WITGEN_MISC2_EXTRA_KERNELS.with(|cell| {
                let kernels = cell.borrow();
                misc2_needed_extra_minors
                    .iter()
                    .all(|minor| kernels.contains_key(minor))
            });
        let mem0_extras_ready = arm_idx != 5
            || WITGEN_MEM0_EXTRA_KERNELS.with(|cell| {
                let kernels = cell.borrow();
                mem0_needed_extra_minors
                    .iter()
                    .all(|minor| kernels.contains_key(minor))
            });
        let mem1_extras_ready = arm_idx != 6
            || WITGEN_MEM1_EXTRA_KERNELS.with(|cell| {
                let kernels = cell.borrow();
                mem1_needed_extra_minors
                    .iter()
                    .all(|minor| kernels.contains_key(minor))
            });
        if chunk0_ready
            && chunk1_ready
            && misc0_extras_ready
            && misc2_extras_ready
            && mem0_extras_ready
            && mem1_extras_ready
        {
            return Ok(());
        }

        let layout = self.hal.create_bind_group_layout(
            "witgen_arm_arm_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::read_only_storage(6, 0),
                WebGpuBindingLayout::read_only_storage(7, 0),
                WebGpuBindingLayout::read_only_storage(8, 0),
                WebGpuBindingLayout::read_only_storage(9, 0),
            ],
        )?;
        let layouts = [layout.clone()];

        if arm_idx == 0 {
            let chunk0_module = if !chunk0_ready {
                let wrapper = synth_arm_wrapper(label, sub_fn, arm_idx);
                assemble_arm_kernel(delta, &wrapper)
            } else {
                String::new()
            };
            let chunk0_entry = format!("witgen_arm_{}_main", label);
            let chunk1_module = if !chunk1_ready {
                let patched_chunk1 = patch_extern_get_diff_count(EXEC_TOP_CHUNK1_WGSL);
                let patched_chunk1 = patch_extern_get_memory_txn(&patched_chunk1);
                let wrapper = synth_arm_chunk1_wrapper(label, arm_idx);
                let mut module = String::with_capacity(patched_chunk1.len() + wrapper.len() + 2);
                module.push_str(&patched_chunk1);
                if !module.ends_with('\n') {
                    module.push('\n');
                }
                module.push_str(&wrapper);
                module
            } else {
                String::new()
            };
            let chunk1_entry = format!("witgen_arm_{}_c1_main", label);
            let misc0_extra_modules: Vec<(u8, String, String, &'static str)> =
                MISC0_EXTRA_CHUNK_DELTAS
                    .iter()
                    .filter_map(|(minor, extra_label, extra_delta, extra_sub_fn)| {
                        if !misc0_needed_extra_minors.contains(minor) {
                            return None;
                        }
                        let ready = WITGEN_MISC0_EXTRA_KERNELS
                            .with(|cell| cell.borrow().contains_key(minor));
                        if ready {
                            return None;
                        }
                        let wrapper = synth_arm_wrapper(extra_label, extra_sub_fn, 0);
                        let module = assemble_arm_kernel(extra_delta, &wrapper);
                        let entry = format!("witgen_arm_{extra_label}_main");
                        Some((*minor, module, entry, *extra_label))
                    })
                    .collect();
            let misc0_extra_futures: Vec<_> = misc0_extra_modules
                .iter()
                .map(|(_, module, entry, _)| {
                    self.hal.create_compute_kernel_async(
                        "witgen_arm_misc0_extra_kernel_on_demand",
                        module,
                        entry,
                        &layouts,
                    )
                })
                .collect();

            let chunk0_task = async {
                if chunk0_ready {
                    None
                } else {
                    Some(
                        self.hal
                            .create_compute_kernel_async(
                                "witgen_arm_arm_kernel_on_demand",
                                &chunk0_module,
                                &chunk0_entry,
                                &layouts,
                            )
                            .await,
                    )
                }
            };
            let chunk1_task = async {
                if chunk1_ready {
                    None
                } else {
                    Some(
                        self.hal
                            .create_compute_kernel_async(
                                "witgen_arm_arm_chunk1_kernel_on_demand",
                                &chunk1_module,
                                &chunk1_entry,
                                &layouts,
                            )
                            .await,
                    )
                }
            };

            let (chunk0_result, chunk1_result) = futures::join!(chunk0_task, chunk1_task);
            if let Some(result) = chunk0_result {
                let kernel = result?;
                WITGEN_ARM_KERNELS.with(|cell| cell.borrow_mut().insert(*label, kernel));
                record_witgen_replace_on_demand_kernel_compile(label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_on_demand arm={} chunk0 DONE",
                    label,
                ));
            }
            if let Some(result) = chunk1_result {
                let kernel = result?;
                WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(*label, kernel));
                record_witgen_replace_on_demand_kernel_compile(label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_on_demand arm={} chunk1 DONE",
                    label,
                ));
            }
            for (i, fut) in misc0_extra_futures.into_iter().enumerate() {
                let kernel = fut.await?;
                let (minor, _, _, extra_label) = &misc0_extra_modules[i];
                WITGEN_MISC0_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(*minor, kernel));
                record_witgen_replace_on_demand_kernel_compile(extra_label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_on_demand arm={} DONE",
                    extra_label,
                ));
            }
            return Ok(());
        }

        if arm_idx == 2 {
            let chunk0_module = if !chunk0_ready {
                let wrapper = synth_arm_wrapper(label, sub_fn, arm_idx);
                assemble_arm_kernel(delta, &wrapper)
            } else {
                String::new()
            };
            let chunk0_entry = format!("witgen_arm_{}_main", label);
            let chunk1_module = if !chunk1_ready {
                let patched_chunk1 = patch_extern_get_diff_count(EXEC_TOP_CHUNK1_WGSL);
                let patched_chunk1 = patch_extern_get_memory_txn(&patched_chunk1);
                let wrapper = synth_arm_chunk1_wrapper(label, arm_idx);
                let mut module = String::with_capacity(patched_chunk1.len() + wrapper.len() + 2);
                module.push_str(&patched_chunk1);
                if !module.ends_with('\n') {
                    module.push('\n');
                }
                module.push_str(&wrapper);
                module
            } else {
                String::new()
            };
            let chunk1_entry = format!("witgen_arm_{}_c1_main", label);
            let misc2_extra_modules: Vec<(u8, String, String, &'static str)> =
                MISC2_EXTRA_CHUNK_DELTAS
                    .iter()
                    .filter_map(|(minor, extra_label, extra_delta, extra_sub_fn)| {
                        if !misc2_needed_extra_minors.contains(minor) {
                            return None;
                        }
                        let ready = WITGEN_MISC2_EXTRA_KERNELS
                            .with(|cell| cell.borrow().contains_key(minor));
                        if ready {
                            return None;
                        }
                        let wrapper = synth_arm_wrapper(extra_label, extra_sub_fn, 2);
                        let module = assemble_arm_kernel(extra_delta, &wrapper);
                        let entry = format!("witgen_arm_{extra_label}_main");
                        Some((*minor, module, entry, *extra_label))
                    })
                    .collect();
            let misc2_extra_futures: Vec<_> = misc2_extra_modules
                .iter()
                .map(|(_, module, entry, _)| {
                    self.hal.create_compute_kernel_async(
                        "witgen_arm_misc2_extra_kernel_on_demand",
                        module,
                        entry,
                        &layouts,
                    )
                })
                .collect();

            let chunk0_task = async {
                if chunk0_ready {
                    None
                } else {
                    Some(
                        self.hal
                            .create_compute_kernel_async(
                                "witgen_arm_arm_kernel_on_demand",
                                &chunk0_module,
                                &chunk0_entry,
                                &layouts,
                            )
                            .await,
                    )
                }
            };
            let chunk1_task = async {
                if chunk1_ready {
                    None
                } else {
                    Some(
                        self.hal
                            .create_compute_kernel_async(
                                "witgen_arm_arm_chunk1_kernel_on_demand",
                                &chunk1_module,
                                &chunk1_entry,
                                &layouts,
                            )
                            .await,
                    )
                }
            };

            let (chunk0_result, chunk1_result) = futures::join!(chunk0_task, chunk1_task);
            if let Some(result) = chunk0_result {
                let kernel = result?;
                WITGEN_ARM_KERNELS.with(|cell| cell.borrow_mut().insert(*label, kernel));
                record_witgen_replace_on_demand_kernel_compile(label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_on_demand arm={} chunk0 DONE",
                    label,
                ));
            }
            if let Some(result) = chunk1_result {
                let kernel = result?;
                WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(*label, kernel));
                record_witgen_replace_on_demand_kernel_compile(label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_on_demand arm={} chunk1 DONE",
                    label,
                ));
            }
            for (i, fut) in misc2_extra_futures.into_iter().enumerate() {
                let kernel = fut.await?;
                let (minor, _, _, extra_label) = &misc2_extra_modules[i];
                WITGEN_MISC2_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(*minor, kernel));
                record_witgen_replace_on_demand_kernel_compile(extra_label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_on_demand arm={} DONE",
                    extra_label,
                ));
            }
            return Ok(());
        }

        if arm_idx == 5 {
            let chunk0_module = if !chunk0_ready {
                let wrapper = synth_arm_wrapper(label, sub_fn, arm_idx);
                assemble_arm_kernel(delta, &wrapper)
            } else {
                String::new()
            };
            let chunk0_entry = format!("witgen_arm_{}_main", label);
            let chunk1_module = if !chunk1_ready {
                let patched_chunk1 = patch_extern_get_diff_count(EXEC_TOP_CHUNK1_WGSL);
                let patched_chunk1 = patch_extern_get_memory_txn(&patched_chunk1);
                let wrapper = synth_arm_chunk1_wrapper(label, arm_idx);
                let mut module = String::with_capacity(patched_chunk1.len() + wrapper.len() + 2);
                module.push_str(&patched_chunk1);
                if !module.ends_with('\n') {
                    module.push('\n');
                }
                module.push_str(&wrapper);
                module
            } else {
                String::new()
            };
            let chunk1_entry = format!("witgen_arm_{}_c1_main", label);
            let mem0_extra_modules: Vec<(u8, String, String, &'static str)> =
                MEM0_EXTRA_CHUNK_DELTAS
                    .iter()
                    .filter_map(|(minor, extra_label, extra_delta, extra_sub_fn)| {
                        if !mem0_needed_extra_minors.contains(minor) {
                            return None;
                        }
                        let ready = WITGEN_MEM0_EXTRA_KERNELS
                            .with(|cell| cell.borrow().contains_key(minor));
                        if ready {
                            return None;
                        }
                        let wrapper = synth_arm_wrapper(extra_label, extra_sub_fn, 5);
                        let module = assemble_arm_kernel(extra_delta, &wrapper);
                        let entry = format!("witgen_arm_{extra_label}_main");
                        Some((*minor, module, entry, *extra_label))
                    })
                    .collect();
            let mem0_extra_futures: Vec<_> = mem0_extra_modules
                .iter()
                .map(|(_, module, entry, _)| {
                    self.hal.create_compute_kernel_async(
                        "witgen_arm_mem0_extra_kernel_on_demand",
                        module,
                        entry,
                        &layouts,
                    )
                })
                .collect();

            let chunk0_task = async {
                if chunk0_ready {
                    None
                } else {
                    Some(
                        self.hal
                            .create_compute_kernel_async(
                                "witgen_arm_arm_kernel_on_demand",
                                &chunk0_module,
                                &chunk0_entry,
                                &layouts,
                            )
                            .await,
                    )
                }
            };
            let chunk1_task = async {
                if chunk1_ready {
                    None
                } else {
                    Some(
                        self.hal
                            .create_compute_kernel_async(
                                "witgen_arm_arm_chunk1_kernel_on_demand",
                                &chunk1_module,
                                &chunk1_entry,
                                &layouts,
                            )
                            .await,
                    )
                }
            };

            let (chunk0_result, chunk1_result) = futures::join!(chunk0_task, chunk1_task);
            if let Some(result) = chunk0_result {
                let kernel = result?;
                WITGEN_ARM_KERNELS.with(|cell| cell.borrow_mut().insert(*label, kernel));
                record_witgen_replace_on_demand_kernel_compile(label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_on_demand arm={} chunk0 DONE",
                    label,
                ));
            }
            if let Some(result) = chunk1_result {
                let kernel = result?;
                WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(*label, kernel));
                record_witgen_replace_on_demand_kernel_compile(label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_on_demand arm={} chunk1 DONE",
                    label,
                ));
            }
            for (i, fut) in mem0_extra_futures.into_iter().enumerate() {
                let kernel = fut.await?;
                let (minor, _, _, extra_label) = &mem0_extra_modules[i];
                WITGEN_MEM0_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(*minor, kernel));
                record_witgen_replace_on_demand_kernel_compile(extra_label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_on_demand arm={} DONE",
                    extra_label,
                ));
            }
            return Ok(());
        }

        if arm_idx == 6 {
            let chunk0_module = if !chunk0_ready {
                let wrapper = synth_arm_wrapper(label, sub_fn, arm_idx);
                assemble_arm_kernel(delta, &wrapper)
            } else {
                String::new()
            };
            let chunk0_entry = format!("witgen_arm_{}_main", label);
            let chunk1_module = if !chunk1_ready {
                let patched_chunk1 = patch_extern_get_diff_count(EXEC_TOP_CHUNK1_WGSL);
                let patched_chunk1 = patch_extern_get_memory_txn(&patched_chunk1);
                let wrapper = synth_arm_chunk1_wrapper(label, arm_idx);
                let mut module = String::with_capacity(patched_chunk1.len() + wrapper.len() + 2);
                module.push_str(&patched_chunk1);
                if !module.ends_with('\n') {
                    module.push('\n');
                }
                module.push_str(&wrapper);
                module
            } else {
                String::new()
            };
            let chunk1_entry = format!("witgen_arm_{}_c1_main", label);
            let mem1_extra_modules: Vec<(u8, String, String, &'static str)> =
                MEM1_EXTRA_CHUNK_DELTAS
                    .iter()
                    .filter_map(|(minor, extra_label, extra_delta, extra_sub_fn)| {
                        if !mem1_needed_extra_minors.contains(minor) {
                            return None;
                        }
                        let ready = WITGEN_MEM1_EXTRA_KERNELS
                            .with(|cell| cell.borrow().contains_key(minor));
                        if ready {
                            return None;
                        }
                        let wrapper = synth_arm_wrapper(extra_label, extra_sub_fn, 6);
                        let module = assemble_arm_kernel(extra_delta, &wrapper);
                        let entry = format!("witgen_arm_{extra_label}_main");
                        Some((*minor, module, entry, *extra_label))
                    })
                    .collect();
            let mem1_extra_futures: Vec<_> = mem1_extra_modules
                .iter()
                .map(|(_, module, entry, _)| {
                    self.hal.create_compute_kernel_async(
                        "witgen_arm_mem1_extra_kernel_on_demand",
                        module,
                        entry,
                        &layouts,
                    )
                })
                .collect();

            let chunk0_task = async {
                if chunk0_ready {
                    None
                } else {
                    Some(
                        self.hal
                            .create_compute_kernel_async(
                                "witgen_arm_arm_kernel_on_demand",
                                &chunk0_module,
                                &chunk0_entry,
                                &layouts,
                            )
                            .await,
                    )
                }
            };
            let chunk1_task = async {
                if chunk1_ready {
                    None
                } else {
                    Some(
                        self.hal
                            .create_compute_kernel_async(
                                "witgen_arm_arm_chunk1_kernel_on_demand",
                                &chunk1_module,
                                &chunk1_entry,
                                &layouts,
                            )
                            .await,
                    )
                }
            };

            let (chunk0_result, chunk1_result) = futures::join!(chunk0_task, chunk1_task);
            if let Some(result) = chunk0_result {
                let kernel = result?;
                WITGEN_ARM_KERNELS.with(|cell| cell.borrow_mut().insert(*label, kernel));
                record_witgen_replace_on_demand_kernel_compile(label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_on_demand arm={} chunk0 DONE",
                    label,
                ));
            }
            if let Some(result) = chunk1_result {
                let kernel = result?;
                WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(*label, kernel));
                record_witgen_replace_on_demand_kernel_compile(label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_on_demand arm={} chunk1 DONE",
                    label,
                ));
            }
            for (i, fut) in mem1_extra_futures.into_iter().enumerate() {
                let kernel = fut.await?;
                let (minor, _, _, extra_label) = &mem1_extra_modules[i];
                WITGEN_MEM1_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(*minor, kernel));
                record_witgen_replace_on_demand_kernel_compile(extra_label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_on_demand arm={} DONE",
                    extra_label,
                ));
            }
            return Ok(());
        }

        if !chunk0_ready {
            let wrapper = synth_arm_wrapper(label, sub_fn, arm_idx);
            let module = assemble_arm_kernel(delta, &wrapper);
            let entry = format!("witgen_arm_{}_main", label);
            let kernel = self
                .hal
                .create_compute_kernel_async(
                    "witgen_arm_arm_kernel_on_demand",
                    &module,
                    &entry,
                    &layouts,
                )
                .await?;
            WITGEN_ARM_KERNELS.with(|cell| cell.borrow_mut().insert(*label, kernel));
            record_witgen_replace_on_demand_kernel_compile(label);
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "witgen_arm_on_demand arm={} chunk0 DONE",
                label,
            ));
        }

        if !chunk1_ready {
            let patched_chunk1 = patch_extern_get_diff_count(EXEC_TOP_CHUNK1_WGSL);
            let patched_chunk1 = patch_extern_get_memory_txn(&patched_chunk1);
            let wrapper = synth_arm_chunk1_wrapper(label, arm_idx);
            let mut module = String::with_capacity(patched_chunk1.len() + wrapper.len() + 2);
            module.push_str(&patched_chunk1);
            if !module.ends_with('\n') {
                module.push('\n');
            }
            module.push_str(&wrapper);
            let entry = format!("witgen_arm_{}_c1_main", label);
            let kernel = self
                .hal
                .create_compute_kernel_async(
                    "witgen_arm_arm_chunk1_kernel_on_demand",
                    &module,
                    &entry,
                    &layouts,
                )
                .await?;
            WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(*label, kernel));
            record_witgen_replace_on_demand_kernel_compile(label);
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "witgen_arm_on_demand arm={} chunk1 DONE",
                label,
            ));
        }

        Ok(())
    }

    /// Build 13 per-arm cycle-list buffers from preflight and dispatch
    /// each prewarmed per-arm kernel over its cycle subset. Wired arm
    /// kernels construct `InstInputStruct` + `BoundLayout` and call the
    /// arm sub-fn, writing real witness cells; `rust_steps` then
    /// short-circuits the cycles covered by GPU dispatch.
    ///
    /// Returns the set of arm_idx values that were actually dispatched on
    /// this call (kernel cached + cycles > 0). Caller can use this to
    /// decide whether `rust_steps` may short-circuit those arms.
    pub(crate) fn dispatch_witgen_per_arm_probe(
        &self,
        data: &MetaBuffer<WebGpuHal>,
        global: &MetaBuffer<WebGpuHal>,
        preflight: &PreflightTrace,
        dispatch_mask: Option<u16>,
        diff_only_mode: bool,
    ) -> Result<Vec<usize>> {
        use crate::prove::wgsl_pruner::{
            MEM0_EXTRA_CHUNK_DELTAS, MEM1_EXTRA_CHUNK_DELTAS, MISC0_EXTRA_CHUNK_DELTAS,
            MISC2_EXTRA_CHUNK_DELTAS, SHADOW_INIT_WGSL, TOP_CHUNK0_ARM_DELTAS,
        };
        // Build per-arm cycle lists from preflight (CPU-side, fast).
        let mut per_arm_cycles: Vec<Vec<u32>> = vec![Vec::new(); TOP_CHUNK0_ARM_DELTAS.len()];
        let replacement_only = dispatch_mask.is_some();
        for (cycle_idx, cycle) in preflight.cycles.iter().enumerate() {
            let arm = cycle.major as usize;
            if replacement_only && !is_witgen_replace_cycle(cycle.major, cycle.minor) {
                continue;
            }
            if diff_only_mode && !is_witgen_diff_cycle(cycle.major, cycle.minor) {
                continue;
            }
            if arm < per_arm_cycles.len() {
                per_arm_cycles[arm].push(cycle_idx as u32);
            }
        }
        let _t = WebGpuStageTimer::new(format!(
            "witgen_arm_per_arm_dispatch arms={} total_cycles={}",
            TOP_CHUNK0_ARM_DELTAS.len(),
            preflight.cycles.len(),
        ));
        // The layout has binding 5 (cycle_list)
        // and binding 6 (preflight_meta). Per-arm wrappers read major/
        // minor from preflight_meta via packed_minor_major at index 3.
        // step 6.2.5: binding 7 is preflight_diff_count_buf (patched
        // extern_getDiffCount reads it). step 6.2.6: bindings 8 and 9
        // are preflight_txn_start and preflight_txns_buf (patched
        // extern_getMemoryTxn reads them with a per-invocation counter).
        let layout = self.hal.create_bind_group_layout(
            "witgen_arm_arm_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::read_only_storage(6, 0),
                WebGpuBindingLayout::read_only_storage(7, 0),
                WebGpuBindingLayout::read_only_storage(8, 0),
                WebGpuBindingLayout::read_only_storage(9, 0),
            ],
        )?;
        let total_cycles = data.rows as u32;
        let placeholder_bytes: u64 = 256;
        let accum_buf = self
            .hal
            .create_storage_buffer("witgen_arm_arm_accum_ph", placeholder_bytes)?;
        let mix_buf = self
            .hal
            .create_storage_buffer("witgen_arm_arm_mix_ph", placeholder_bytes)?;
        let params: [u32; 8] = [total_cycles, 1, total_cycles, 1, 0, 0, 0, 0];
        let params_buf = self
            .hal
            .create_uniform_buffer("witgen_arm_arm_params_ph", bytemuck::cast_slice(&params))?;
        // Upload preflight_meta for per-arm wrappers (separate buffer
        // from the shadow_init kernel's upload; could be shared in a
        // future tightening, but keeping separate avoids cross-pass
        // ownership coupling).
        let meta = build_preflight_meta(preflight);
        let meta_bytes: &[u8] = bytemuck::cast_slice(meta.as_slice());
        let preflight_buf = self
            .hal
            .create_storage_buffer("witgen_arm_arm_preflight", meta_bytes.len() as u64)?;
        self.hal
            .write_buffer_named(&preflight_buf, "witgen_arm_arm_preflight", 0, meta_bytes)?;
        let data_gpu = data
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("pre-witgen dispatch: data missing GPU storage"))?;
        let global_gpu = global
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("pre-witgen dispatch: global missing GPU storage"))?;
        {
            let _t = WebGpuStageTimer::new(format!(
                "witgen_arm_shadow_init cycles={}",
                preflight.cycles.len()
            ));
            let kernel = SHADOW_INIT_KERNEL.with(|cell| cell.borrow().clone());
            let kernel = match kernel {
                Some(k) => k,
                None => {
                    let layout = self.hal.create_bind_group_layout(
                        "witgen_arm_shadow_init_layout",
                        &[
                            WebGpuBindingLayout::storage(0, 0),
                            WebGpuBindingLayout::uniform(1, 16),
                            WebGpuBindingLayout::read_only_storage(2, 0),
                        ],
                    )?;
                    let k = self.hal.create_compute_kernel(
                        "witgen_arm_shadow_init",
                        SHADOW_INIT_WGSL,
                        "shadow_init_main",
                        &[layout],
                    )?;
                    SHADOW_INIT_KERNEL.with(|cell| *cell.borrow_mut() = Some(k.clone()));
                    k
                }
            };
            let shadow_params: [u32; 4] = [total_cycles, data.cols as u32, 0, 0];
            let shadow_params_buf = self.hal.create_uniform_buffer(
                "witgen_arm_shadow_params",
                bytemuck::cast_slice(&shadow_params),
            )?;
            let shadow_layout = self.hal.create_bind_group_layout(
                "witgen_arm_shadow_init_layout",
                &[
                    WebGpuBindingLayout::storage(0, 0),
                    WebGpuBindingLayout::uniform(1, 16),
                    WebGpuBindingLayout::read_only_storage(2, 0),
                ],
            )?;
            let shadow_bind_group = self.hal.create_bind_group(
                "witgen_arm_shadow_bg",
                &shadow_layout,
                &[
                    WebGpuBufferBinding::new(0, data_gpu),
                    WebGpuBufferBinding::new(1, &shadow_params_buf),
                    WebGpuBufferBinding::new(2, &preflight_buf),
                ],
            )?;
            self.hal
                .dispatch_compute_1d(&kernel, &shadow_bind_group, total_cycles.div_ceil(64));
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "witgen_arm_shadow_init cycles={} meta_bytes={} reused_arm_preflight=true",
                total_cycles,
                meta_bytes.len(),
            ));
            drop(shadow_bind_group);
            drop(shadow_params_buf);
        }
        // step 6.2.5: per-cycle diff counts so patched extern_getDiffCount
        // returns the right value inside arm sub-fns (DoCycleTable).
        let diff_count = build_preflight_diff_count(preflight);
        let diff_count_bytes: &[u8] = bytemuck::cast_slice(diff_count.as_slice());
        let diff_count_buf = self
            .hal
            .create_storage_buffer("witgen_arm_arm_diff_count", diff_count_bytes.len() as u64)?;
        self.hal.write_buffer_named(
            &diff_count_buf,
            "witgen_arm_arm_diff_count",
            0,
            diff_count_bytes,
        )?;
        // step 6.2.6: per-cycle txn_start + packed memory txns so patched
        // extern_getMemoryTxn returns the right values inside arm sub-fns
        // (DecodeInst, ReadSourceRegs, per-arm memory ops).
        let txn_start = build_preflight_txn_start(preflight);
        let txns = build_preflight_txns(preflight);
        let txn_start_bytes: &[u8] = bytemuck::cast_slice(txn_start.as_slice());
        let txn_start_buf = self
            .hal
            .create_storage_buffer("witgen_arm_arm_txn_start", txn_start_bytes.len() as u64)?;
        self.hal.write_buffer_named(
            &txn_start_buf,
            "witgen_arm_arm_txn_start",
            0,
            txn_start_bytes,
        )?;
        let txns_bytes: &[u8] = bytemuck::cast_slice(txns.as_slice());
        let txns_buf = self
            .hal
            .create_storage_buffer("witgen_arm_arm_txns", txns_bytes.len() as u64)?;
        self.hal
            .write_buffer_named(&txns_buf, "witgen_arm_arm_txns", 0, txns_bytes)?;
        let mut dispatched = 0usize;
        let mut skipped = 0usize;
        let mut dispatched_arms: Vec<usize> = Vec::with_capacity(TOP_CHUNK0_ARM_DELTAS.len());
        // Keep per-arm cycle_list buffers alive across the dispatch
        // loop so the GPU encoder can reference them when the queue
        // flushes at end-of-scope.
        let mut cycle_list_buffers: Vec<_> = Vec::with_capacity(TOP_CHUNK0_ARM_DELTAS.len());
        let mut bind_groups: Vec<_> = Vec::with_capacity(TOP_CHUNK0_ARM_DELTAS.len());
        for (arm_idx, (label, _, _)) in TOP_CHUNK0_ARM_DELTAS.iter().enumerate() {
            if let Some(mask) = dispatch_mask {
                if (mask & (1u16 << arm_idx)) == 0 {
                    continue;
                }
            }
            let cycle_count = per_arm_cycles[arm_idx].len();
            if cycle_count == 0 {
                continue;
            }
            let kernel = WITGEN_ARM_KERNELS.with(|cell| cell.borrow().get(*label).cloned());
            let Some(kernel) = kernel else {
                skipped += 1;
                continue;
            };
            // For short-circuit safety the arm's
            // chunk1 kernel MUST also be dispatched (chunk0 alone covers
            // only some minor opcodes; chunk1 covers the rest).
            // No-op-wrapper arms (5 inter-cycle) don't have chunk1
            // kernels and aren't in ZERO_BACK_REG_ARMS so they're never
            // short-circuited; skip the chunk1 check for them.
            let chunk1_kernel = if is_zero_back_reg_arm(arm_idx) {
                let k = WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow().get(*label).cloned());
                if k.is_none() {
                    skipped += 1;
                    continue;
                }
                k
            } else {
                None
            };
            let extra_kernels: Vec<(Option<u8>, WebGpuKernel)> = if arm_idx == 0 {
                let needed: Vec<u8> = MISC0_EXTRA_CHUNK_DELTAS
                    .iter()
                    .filter_map(|(minor, _, _, _)| {
                        per_arm_cycles[arm_idx]
                            .iter()
                            .any(|&idx| preflight.cycles[idx as usize].minor == *minor)
                            .then_some(*minor)
                    })
                    .collect();
                let kernels = WITGEN_MISC0_EXTRA_KERNELS.with(|cell| {
                    let cached = cell.borrow();
                    needed
                        .iter()
                        .map(|minor| cached.get(minor).cloned().map(|kernel| (None, kernel)))
                        .collect::<Option<Vec<_>>>()
                });
                let Some(kernels) = kernels else {
                    skipped += 1;
                    continue;
                };
                kernels
            } else if arm_idx == 2 {
                let needed: Vec<u8> = MISC2_EXTRA_CHUNK_DELTAS
                    .iter()
                    .filter_map(|(minor, _, _, _)| {
                        per_arm_cycles[arm_idx]
                            .iter()
                            .any(|&idx| preflight.cycles[idx as usize].minor == *minor)
                            .then_some(*minor)
                    })
                    .collect();
                let kernels = WITGEN_MISC2_EXTRA_KERNELS.with(|cell| {
                    let cached = cell.borrow();
                    needed
                        .iter()
                        .map(|minor| {
                            cached
                                .get(minor)
                                .cloned()
                                .map(|kernel| (Some(*minor), kernel))
                        })
                        .collect::<Option<Vec<_>>>()
                });
                let Some(kernels) = kernels else {
                    skipped += 1;
                    continue;
                };
                kernels
            } else if arm_idx == 5 {
                let needed: Vec<u8> = MEM0_EXTRA_CHUNK_DELTAS
                    .iter()
                    .filter_map(|(minor, _, _, _)| {
                        per_arm_cycles[arm_idx]
                            .iter()
                            .any(|&idx| preflight.cycles[idx as usize].minor == *minor)
                            .then_some(*minor)
                    })
                    .collect();
                let kernels = WITGEN_MEM0_EXTRA_KERNELS.with(|cell| {
                    let cached = cell.borrow();
                    needed
                        .iter()
                        .map(|minor| {
                            cached
                                .get(minor)
                                .cloned()
                                .map(|kernel| (Some(*minor), kernel))
                        })
                        .collect::<Option<Vec<_>>>()
                });
                let Some(kernels) = kernels else {
                    skipped += 1;
                    continue;
                };
                kernels
            } else if arm_idx == 6 {
                let needed: Vec<u8> = MEM1_EXTRA_CHUNK_DELTAS
                    .iter()
                    .filter_map(|(minor, _, _, _)| {
                        per_arm_cycles[arm_idx]
                            .iter()
                            .any(|&idx| preflight.cycles[idx as usize].minor == *minor)
                            .then_some(*minor)
                    })
                    .collect();
                let kernels = WITGEN_MEM1_EXTRA_KERNELS.with(|cell| {
                    let cached = cell.borrow();
                    needed
                        .iter()
                        .map(|minor| {
                            cached
                                .get(minor)
                                .cloned()
                                .map(|kernel| (Some(*minor), kernel))
                        })
                        .collect::<Option<Vec<_>>>()
                });
                let Some(kernels) = kernels else {
                    skipped += 1;
                    continue;
                };
                kernels
            } else {
                Vec::new()
            };

            // Per-arm cycle list upload (storage buffer + queue write).
            let cycle_bytes: &[u8] = bytemuck::cast_slice(per_arm_cycles[arm_idx].as_slice());
            let cycle_buf = self
                .hal
                .create_storage_buffer("witgen_arm_arm_cycle_list", cycle_bytes.len() as u64)?;
            self.hal
                .write_buffer_named(&cycle_buf, "witgen_arm_arm_cycle_list", 0, cycle_bytes)?;
            let bind_group = self.hal.create_bind_group(
                "witgen_arm_arm_bg",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, data_gpu),
                    WebGpuBufferBinding::new(1, global_gpu),
                    WebGpuBufferBinding::new(2, &accum_buf),
                    WebGpuBufferBinding::new(3, &mix_buf),
                    WebGpuBufferBinding::new(4, &params_buf),
                    WebGpuBufferBinding::new(5, &cycle_buf),
                    WebGpuBufferBinding::new(6, &preflight_buf),
                    WebGpuBufferBinding::new(7, &diff_count_buf),
                    WebGpuBufferBinding::new(8, &txn_start_buf),
                    WebGpuBufferBinding::new(9, &txns_buf),
                ],
            )?;
            let workgroups = (cycle_count as u32).div_ceil(64);
            self.hal
                .dispatch_compute_1d(&kernel, &bind_group, workgroups);
            // Dispatch chunk1 too if we have it
            // (shares the same bind group -- bindings are identical).
            if let Some(k1) = chunk1_kernel {
                self.hal.dispatch_compute_1d(&k1, &bind_group, workgroups);
            }
            for (minor_filter, kernel) in &extra_kernels {
                let Some(minor) = minor_filter else {
                    self.hal
                        .dispatch_compute_1d(kernel, &bind_group, workgroups);
                    continue;
                };

                let filtered_cycles: Vec<u32> = per_arm_cycles[arm_idx]
                    .iter()
                    .copied()
                    .filter(|&idx| preflight.cycles[idx as usize].minor == *minor)
                    .collect();
                if filtered_cycles.is_empty() {
                    continue;
                }
                let filtered_cycle_bytes: &[u8] = bytemuck::cast_slice(filtered_cycles.as_slice());
                let filtered_cycle_buf = self.hal.create_storage_buffer(
                    "witgen_arm_arm_minor_cycle_list",
                    filtered_cycle_bytes.len() as u64,
                )?;
                self.hal.write_buffer_named(
                    &filtered_cycle_buf,
                    "witgen_arm_arm_minor_cycle_list",
                    0,
                    filtered_cycle_bytes,
                )?;
                let filtered_bind_group = self.hal.create_bind_group(
                    "witgen_arm_arm_minor_bg",
                    &layout,
                    &[
                        WebGpuBufferBinding::new(0, data_gpu),
                        WebGpuBufferBinding::new(1, global_gpu),
                        WebGpuBufferBinding::new(2, &accum_buf),
                        WebGpuBufferBinding::new(3, &mix_buf),
                        WebGpuBufferBinding::new(4, &params_buf),
                        WebGpuBufferBinding::new(5, &filtered_cycle_buf),
                        WebGpuBufferBinding::new(6, &preflight_buf),
                        WebGpuBufferBinding::new(7, &diff_count_buf),
                        WebGpuBufferBinding::new(8, &txn_start_buf),
                        WebGpuBufferBinding::new(9, &txns_buf),
                    ],
                )?;
                let filtered_workgroups = (filtered_cycles.len() as u32).div_ceil(64);
                self.hal
                    .dispatch_compute_1d(kernel, &filtered_bind_group, filtered_workgroups);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_minor_dispatch arm={} minor={} cycles={}",
                    arm_idx,
                    minor,
                    filtered_cycles.len()
                ));
                cycle_list_buffers.push(filtered_cycle_buf);
                bind_groups.push(filtered_bind_group);
            }
            cycle_list_buffers.push(cycle_buf);
            bind_groups.push(bind_group);
            dispatched += 1;
            dispatched_arms.push(arm_idx);
        }
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "witgen_arm_per_arm_dispatch dispatched={} skipped={}",
            dispatched, skipped,
        ));
        // Keep buffers + bind groups alive until dispatch completes.
        drop(bind_groups);
        drop(cycle_list_buffers);
        drop(preflight_buf);
        drop(diff_count_buf);
        drop(txn_start_buf);
        drop(txns_buf);
        Ok(dispatched_arms)
    }

    pub(crate) fn lookup_topaccum_arm5_split_inv_probe_kernels(
        &self,
    ) -> Result<(WebGpuKernel, WebGpuKernel, WebGpuKernel, WebGpuKernel, u32)> {
        let cached = TOPACCUM_ARM5_INV_CAPTURE_KERNEL.with(|capture_cell| {
            TOPACCUM_ARM5_INV_BUFFER_KERNEL.with(|buffer_cell| {
                TOPACCUM_ARM5_INV_CONSUME_KERNEL.with(|consume_cell| {
                    TOPACCUM_ARM5_INV_CONSUME_RAW_KERNEL.with(|raw_cell| {
                        capture_cell
                            .borrow()
                            .clone()
                            .zip(buffer_cell.borrow().clone())
                            .zip(consume_cell.borrow().clone())
                            .zip(raw_cell.borrow().clone())
                            .map(|(((capture, buffer), consume), raw)| {
                                (capture, buffer, consume, raw)
                            })
                    })
                })
            })
        });
        if let Some((capture, buffer, consume, raw)) = cached {
            return Ok((capture, buffer, consume, raw, TOPACCUM_ARM5_INV_CALLS));
        }

        use crate::prove::wgsl_pruner::{TOPACCUM_ARM5_WGSL, WITGEN_BASELINE_WGSL};
        let layout = self.hal.create_bind_group_layout(
            "rv32im_accum_topaccum_arm5_split_inv_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::storage(6, 0),
            ],
        )?;
        let (capture_body, capture_inv_count) =
            topaccum_arm5_replace_ext_inv_calls(TOPACCUM_ARM5_WGSL, "topaccum_arm5_capture_inv")?;
        let (consume_body, consume_inv_count) =
            topaccum_arm5_replace_ext_inv_calls(TOPACCUM_ARM5_WGSL, "topaccum_arm5_consume_inv")?;
        anyhow::ensure!(
            capture_inv_count == consume_inv_count,
            "TopAccum arm5 capture/consume inverse call counts differ"
        );

        let mut capture_module = String::with_capacity(
            WITGEN_BASELINE_WGSL.len()
                + capture_body.len()
                + TOPACCUM_ARM5_INV_CAPTURE_ENTRY_TEMPLATE.len()
                + 2,
        );
        capture_module.push_str(WITGEN_BASELINE_WGSL);
        if !capture_module.ends_with('\n') {
            capture_module.push('\n');
        }
        capture_module.push_str(&capture_body);
        if !capture_module.ends_with('\n') {
            capture_module.push('\n');
        }
        capture_module.push_str(&topaccum_arm5_inv_entry(
            TOPACCUM_ARM5_INV_CAPTURE_ENTRY_TEMPLATE,
            capture_inv_count,
        ));
        let capture_kernel = self.hal.create_compute_kernel(
            "rv32im_accum_topaccum_arm5_capture_inv_probe",
            &capture_module,
            "topaccum_arm5_capture_inv_main",
            &[layout.clone()],
        )?;

        let inv_layout = self.hal.create_bind_group_layout(
            "rv32im_accum_topaccum_arm5_inv_buffer_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::uniform(1, 16),
            ],
        )?;
        let inv_kernel = self.hal.create_compute_kernel(
            "rv32im_accum_topaccum_arm5_inv_buffer",
            TOPACCUM_ARM5_INV_BUFFER_WGSL,
            "main",
            &[inv_layout],
        )?;

        let mut consume_module = String::with_capacity(
            WITGEN_BASELINE_WGSL.len()
                + consume_body.len()
                + TOPACCUM_ARM5_INV_CONSUME_ENTRY_TEMPLATE.len()
                + 2,
        );
        consume_module.push_str(WITGEN_BASELINE_WGSL);
        if !consume_module.ends_with('\n') {
            consume_module.push('\n');
        }
        consume_module.push_str(&consume_body);
        if !consume_module.ends_with('\n') {
            consume_module.push('\n');
        }
        consume_module.push_str(&topaccum_arm5_inv_entry(
            TOPACCUM_ARM5_INV_CONSUME_ENTRY_TEMPLATE,
            consume_inv_count,
        ));
        let consume_kernel = self.hal.create_compute_kernel(
            "rv32im_accum_topaccum_arm5_consume_inv_probe",
            &consume_module,
            "topaccum_arm5_consume_inv_prefixed_main",
            &[layout.clone()],
        )?;
        let consume_raw_kernel = self.hal.create_compute_kernel(
            "rv32im_accum_topaccum_arm5_consume_inv_raw",
            &consume_module,
            "topaccum_arm5_consume_inv_raw_main",
            &[layout],
        )?;

        TOPACCUM_ARM5_INV_CAPTURE_KERNEL
            .with(|cell| *cell.borrow_mut() = Some(capture_kernel.clone()));
        TOPACCUM_ARM5_INV_BUFFER_KERNEL.with(|cell| *cell.borrow_mut() = Some(inv_kernel.clone()));
        TOPACCUM_ARM5_INV_CONSUME_KERNEL
            .with(|cell| *cell.borrow_mut() = Some(consume_kernel.clone()));
        TOPACCUM_ARM5_INV_CONSUME_RAW_KERNEL
            .with(|cell| *cell.borrow_mut() = Some(consume_raw_kernel.clone()));

        Ok((
            capture_kernel,
            inv_kernel,
            consume_kernel,
            consume_raw_kernel,
            capture_inv_count,
        ))
    }

    pub(crate) fn lookup_topaccum_arm5_compare_kernel(&self) -> Result<WebGpuKernel> {
        if let Some(kernel) = TOPACCUM_ARM5_COMPARE_KERNEL.with(|cell| cell.borrow().clone()) {
            return Ok(kernel);
        }
        let layout = self.hal.create_bind_group_layout(
            "rv32im_accum_topaccum_arm5_compare_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::storage(3, 0),
                WebGpuBindingLayout::uniform(4, 16),
            ],
        )?;
        let kernel = self.hal.create_compute_kernel(
            "rv32im_accum_topaccum_arm5_compare",
            TOPACCUM_ARM5_COMPARE_ROW_WGSL,
            "main",
            &[layout],
        )?;
        TOPACCUM_ARM5_COMPARE_KERNEL.with(|cell| *cell.borrow_mut() = Some(kernel.clone()));
        Ok(kernel)
    }

    pub(crate) fn dispatch_topaccum_arm5_real_buffer_probe(
        &self,
        preflight: &PreflightTrace,
        data: &MetaBuffer<WebGpuHal>,
        accum: &MetaBuffer<WebGpuHal>,
        global: &MetaBuffer<WebGpuHal>,
        mix: &MetaBuffer<WebGpuHal>,
    ) -> Result<bool> {
        if !ACCUM_GPU_ARM5_PROBE_ENABLED.load(Ordering::SeqCst) {
            return Ok(false);
        }
        let Some((sample_cycle_idx, sample)) = preflight
            .cycles
            .iter()
            .enumerate()
            .find(|(_, cycle)| cycle.major == 5)
        else {
            risc0_zkp::hal::webgpu::log_webgpu_metric(
                "rv32im_accumulate topaccum_arm5_probe SKIP no_arm5_cycles",
            );
            return Ok(false);
        };
        let sample_cycle =
            u32::try_from(sample_cycle_idx).context("TopAccum arm5 sample cycle exceeds u32")?;
        let available_cycles_usize = preflight
            .cycles
            .iter()
            .filter(|cycle| cycle.major == 5)
            .count();
        let available_cycles = u32::try_from(available_cycles_usize)
            .context("TopAccum arm5 available cycle count exceeds u32")?;
        let selector_col = LAYOUT_TOP.inst_result._selector[5]._super.offset;
        let data_major_col = LAYOUT_TOP.major._super.offset;
        let selector_value = read_data_cell_u32(data, sample_cycle_idx, selector_col)?;
        let data_major_value = read_data_cell_u32(data, sample_cycle_idx, data_major_col)?;
        let summary = TopAccumArm5ProbeSummary {
            sample_cycle,
            available_cycles,
            preflight_major: sample.major as u32,
            data_major_value,
            selector_value,
            mismatch_count: 0,
            first_mismatch_col: u32::MAX,
            first_mismatch_expected: u32::MAX,
            first_mismatch_actual: u32::MAX,
        };
        let _timer = WebGpuStageTimer::new_active_for(
            format!(
                "rv32im_accumulate topaccum_arm5_probe sample_cycle={sample_cycle} available_cycles={available_cycles}"
            ),
            self.hal.as_ref(),
        );
        let (capture_kernel, inv_kernel, consume_kernel, _consume_raw_kernel, inv_count) =
            self.lookup_topaccum_arm5_split_inv_probe_kernels()?;
        data.buf.sync_cpu_to_gpu(self.hal.as_ref())?;
        accum.buf.sync_cpu_to_gpu(self.hal.as_ref())?;
        global.buf.sync_cpu_to_gpu(self.hal.as_ref())?;
        mix.buf.sync_cpu_to_gpu(self.hal.as_ref())?;
        let data_gpu = data
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("TopAccum arm5 probe: data missing GPU storage"))?;
        let accum_gpu = accum
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("TopAccum arm5 probe: accum missing GPU storage"))?;
        let global_gpu = global
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("TopAccum arm5 probe: global missing GPU storage"))?;
        let mix_gpu = mix
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("TopAccum arm5 probe: mix missing GPU storage"))?;
        let accum_scratch_bytes = u64::try_from(
            accum
                .rows
                .checked_mul(accum.cols)
                .context("TopAccum arm5 probe accum size overflow")?
                .checked_mul(std::mem::size_of::<Val>())
                .context("TopAccum arm5 probe accum byte size overflow")?,
        )
        .context("TopAccum arm5 probe accum byte size exceeds u64")?;
        let accum_scratch = self.hal.create_storage_buffer(
            "rv32im_accum_topaccum_arm5_probe_accum_scratch",
            accum_scratch_bytes,
        )?;
        self.hal.copy_gpu_buffer_named(
            "rv32im_accum_topaccum_arm5_probe_accum_scratch",
            accum_gpu,
            accum.buf.byte_offset(),
            &accum_scratch,
            0,
            accum_scratch_bytes,
        )?;
        let layout = self.hal.create_bind_group_layout(
            "rv32im_accum_topaccum_arm5_probe_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::storage(6, 0),
            ],
        )?;
        let split = LAYOUT_TOP_ACCUM.columns[0].offset;
        let params = [
            u32::try_from(data.rows).context("TopAccum arm5 probe data rows exceed u32")?,
            u32::try_from(global.rows).context("TopAccum arm5 probe global rows exceed u32")?,
            u32::try_from(accum.rows).context("TopAccum arm5 probe accum rows exceed u32")?,
            u32::try_from(mix.rows).context("TopAccum arm5 probe mix rows exceed u32")?,
            u32::try_from(split).context("TopAccum arm5 probe zero-back split exceeds u32")?,
            0,
            0,
            0,
        ];
        let params_buf = self.hal.create_uniform_buffer(
            "rv32im_accum_topaccum_arm5_probe_params",
            bytemuck::cast_slice(&params),
        )?;
        let cycle_list = [sample_cycle];
        let cycle_buf = self.hal.create_storage_buffer(
            "rv32im_accum_topaccum_arm5_probe_cycles",
            std::mem::size_of_val(&cycle_list) as u64,
        )?;
        self.hal.write_buffer_named(
            &cycle_buf,
            "rv32im_accum_topaccum_arm5_probe_cycles",
            0,
            bytemuck::cast_slice(&cycle_list),
        )?;
        let inv_items = cycle_list
            .len()
            .checked_mul(inv_count as usize)
            .context("TopAccum arm5 inverse side-buffer item count overflow")?;
        let inv_buf_bytes = u64::try_from(
            inv_items
                .checked_mul(4)
                .and_then(|words| words.checked_mul(std::mem::size_of::<Val>()))
                .context("TopAccum arm5 inverse side-buffer byte size overflow")?,
        )
        .context("TopAccum arm5 inverse side-buffer byte size exceeds u64")?;
        let inv_buf = self.hal.create_storage_buffer(
            "rv32im_accum_topaccum_arm5_probe_inv_side_buffer",
            inv_buf_bytes,
        )?;
        let bind_group = self.hal.create_bind_group(
            "rv32im_accum_topaccum_arm5_probe_bg",
            &layout,
            &[
                WebGpuBufferBinding::new(0, data_gpu),
                WebGpuBufferBinding::new(1, global_gpu),
                WebGpuBufferBinding::new(2, &accum_scratch),
                WebGpuBufferBinding::new(3, mix_gpu),
                WebGpuBufferBinding::new(4, &params_buf),
                WebGpuBufferBinding::new(5, &cycle_buf),
                WebGpuBufferBinding::new(6, &inv_buf),
            ],
        )?;
        self.hal
            .dispatch_compute_1d(&capture_kernel, &bind_group, 1);
        let inv_layout = self.hal.create_bind_group_layout(
            "rv32im_accum_topaccum_arm5_inv_buffer_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::uniform(1, 16),
            ],
        )?;
        let inv_items_u32 = u32::try_from(inv_items)
            .context("TopAccum arm5 inverse side-buffer item count exceeds u32")?;
        let inv_params = [inv_items_u32, 0, 0, 0];
        let inv_params_buf = self.hal.create_uniform_buffer(
            "rv32im_accum_topaccum_arm5_inv_buffer_params",
            bytemuck::cast_slice(&inv_params),
        )?;
        let inv_bind_group = self.hal.create_bind_group(
            "rv32im_accum_topaccum_arm5_inv_buffer_bg",
            &inv_layout,
            &[
                WebGpuBufferBinding::new(0, &inv_buf),
                WebGpuBufferBinding::new(1, &inv_params_buf),
            ],
        )?;
        self.hal
            .dispatch_compute_1d(&inv_kernel, &inv_bind_group, inv_items_u32.div_ceil(64));
        self.hal
            .dispatch_compute_1d(&consume_kernel, &bind_group, 1);
        let compare_kernel = self.lookup_topaccum_arm5_compare_kernel()?;
        let compare_layout = self.hal.create_bind_group_layout(
            "rv32im_accum_topaccum_arm5_compare_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::storage(3, 0),
                WebGpuBindingLayout::uniform(4, 16),
            ],
        )?;
        let compare_flags = self
            .hal
            .alloc_elem("rv32im_accum_topaccum_arm5_compare_flags", accum.cols);
        let compare_details = self
            .hal
            .alloc_u32("rv32im_accum_topaccum_arm5_compare_details", accum.cols * 2);
        let compare_flags_gpu = compare_flags
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("TopAccum arm5 compare: flags missing GPU storage"))?;
        let compare_details_gpu = compare_details
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("TopAccum arm5 compare: details missing GPU storage"))?;
        let compare_params = [
            u32::try_from(accum.rows).context("TopAccum arm5 compare rows exceed u32")?,
            u32::try_from(accum.cols).context("TopAccum arm5 compare cols exceed u32")?,
            sample_cycle,
            0,
        ];
        let compare_params_buf = self.hal.create_uniform_buffer(
            "rv32im_accum_topaccum_arm5_compare_params",
            bytemuck::cast_slice(&compare_params),
        )?;
        let compare_bind_group = self.hal.create_bind_group(
            "rv32im_accum_topaccum_arm5_compare_bg",
            &compare_layout,
            &[
                WebGpuBufferBinding::new(0, accum_gpu),
                WebGpuBufferBinding::new(1, &accum_scratch),
                WebGpuBufferBinding::new(2, compare_flags_gpu),
                WebGpuBufferBinding::new(3, compare_details_gpu),
                WebGpuBufferBinding::new(4, &compare_params_buf),
            ],
        )?;
        self.hal.dispatch_compute_1d(
            &compare_kernel,
            &compare_bind_group,
            u32::try_from(accum.cols)
                .context("TopAccum arm5 compare cols exceed u32")?
                .div_ceil(64),
        );
        compare_flags.mark_gpu_dirty();
        compare_details.mark_gpu_dirty();
        TOPACCUM_ARM5_PROBE_COMPARE.with(|cell| {
            *cell.borrow_mut() = Some(TopAccumArm5ProbeCompare {
                hal: self.hal.clone(),
                flags: compare_flags,
                details: compare_details,
                summary,
            });
        });
        ACCUM_GPU_ARM5_PROBE_DISPATCHES.fetch_add(1, Ordering::SeqCst);
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "rv32im_accumulate topaccum_arm5_probe dispatched sample_cycle={sample_cycle} available_cycles={available_cycles} preflight_major={} data_major_value={} selector_value={}",
            summary.preflight_major, summary.data_major_value, summary.selector_value,
        ));
        Ok(true)
    }

    pub(crate) fn dispatch_topaccum_arm5_authoritative(
        &self,
        cycle_list: &[u32],
        data: &MetaBuffer<WebGpuHal>,
        accum: &MetaBuffer<WebGpuHal>,
        global: &MetaBuffer<WebGpuHal>,
        mix: &MetaBuffer<WebGpuHal>,
    ) -> Result<bool> {
        if cycle_list.is_empty() {
            return Ok(false);
        }

        let cycles = u32::try_from(cycle_list.len())
            .context("TopAccum arm5 authoritative cycle count exceeds u32")?;
        let _timer = WebGpuStageTimer::new_active_for(
            format!("rv32im_accumulate topaccum_arm5_authoritative cycles={cycles}"),
            self.hal.as_ref(),
        );
        let (capture_kernel, inv_kernel, _consume_kernel, consume_raw_kernel, inv_count) =
            self.lookup_topaccum_arm5_split_inv_probe_kernels()?;
        data.buf.sync_cpu_to_gpu(self.hal.as_ref())?;
        accum.buf.sync_cpu_to_gpu(self.hal.as_ref())?;
        global.buf.sync_cpu_to_gpu(self.hal.as_ref())?;
        mix.buf.sync_cpu_to_gpu(self.hal.as_ref())?;
        let data_gpu = data.buf.raw_buffer().ok_or_else(|| {
            anyhow::anyhow!("TopAccum arm5 authoritative: data missing GPU storage")
        })?;
        let accum_gpu = accum.buf.raw_buffer().ok_or_else(|| {
            anyhow::anyhow!("TopAccum arm5 authoritative: accum missing GPU storage")
        })?;
        let global_gpu = global.buf.raw_buffer().ok_or_else(|| {
            anyhow::anyhow!("TopAccum arm5 authoritative: global missing GPU storage")
        })?;
        let mix_gpu = mix.buf.raw_buffer().ok_or_else(|| {
            anyhow::anyhow!("TopAccum arm5 authoritative: mix missing GPU storage")
        })?;
        let layout = self.hal.create_bind_group_layout(
            "rv32im_accum_topaccum_arm5_authoritative_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::storage(6, 0),
            ],
        )?;
        let split = LAYOUT_TOP_ACCUM.columns[0].offset;
        let params = [
            u32::try_from(data.rows).context("TopAccum arm5 authoritative data rows exceed u32")?,
            u32::try_from(global.rows)
                .context("TopAccum arm5 authoritative global rows exceed u32")?,
            u32::try_from(accum.rows)
                .context("TopAccum arm5 authoritative accum rows exceed u32")?,
            u32::try_from(mix.rows).context("TopAccum arm5 authoritative mix rows exceed u32")?,
            u32::try_from(split)
                .context("TopAccum arm5 authoritative zero-back split exceeds u32")?,
            0,
            0,
            0,
        ];
        let params_buf = self.hal.create_uniform_buffer(
            "rv32im_accum_topaccum_arm5_authoritative_params",
            bytemuck::cast_slice(&params),
        )?;
        let cycle_buf_bytes = u64::try_from(
            cycle_list
                .len()
                .checked_mul(std::mem::size_of::<u32>())
                .context("TopAccum arm5 authoritative cycle-list byte size overflow")?,
        )
        .context("TopAccum arm5 authoritative cycle-list byte size exceeds u64")?;
        let cycle_buf = self.hal.create_storage_buffer(
            "rv32im_accum_topaccum_arm5_authoritative_cycles",
            cycle_buf_bytes,
        )?;
        self.hal.write_buffer_named(
            &cycle_buf,
            "rv32im_accum_topaccum_arm5_authoritative_cycles",
            0,
            bytemuck::cast_slice(cycle_list),
        )?;
        let inv_items = cycle_list
            .len()
            .checked_mul(inv_count as usize)
            .context("TopAccum arm5 authoritative inverse side-buffer item count overflow")?;
        let inv_buf_bytes = u64::try_from(
            inv_items
                .checked_mul(4)
                .and_then(|words| words.checked_mul(std::mem::size_of::<Val>()))
                .context("TopAccum arm5 authoritative inverse side-buffer byte size overflow")?,
        )
        .context("TopAccum arm5 authoritative inverse side-buffer byte size exceeds u64")?;
        let inv_buf = self.hal.create_storage_buffer(
            "rv32im_accum_topaccum_arm5_authoritative_inv_side_buffer",
            inv_buf_bytes,
        )?;
        let bind_group = self.hal.create_bind_group(
            "rv32im_accum_topaccum_arm5_authoritative_bg",
            &layout,
            &[
                WebGpuBufferBinding::new(0, data_gpu),
                WebGpuBufferBinding::new(1, global_gpu),
                WebGpuBufferBinding::new(2, accum_gpu),
                WebGpuBufferBinding::new(3, mix_gpu),
                WebGpuBufferBinding::new(4, &params_buf),
                WebGpuBufferBinding::new(5, &cycle_buf),
                WebGpuBufferBinding::new(6, &inv_buf),
            ],
        )?;
        self.hal
            .dispatch_compute_1d(&capture_kernel, &bind_group, cycles);
        let inv_layout = self.hal.create_bind_group_layout(
            "rv32im_accum_topaccum_arm5_inv_buffer_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::uniform(1, 16),
            ],
        )?;
        let inv_items_u32 = u32::try_from(inv_items)
            .context("TopAccum arm5 authoritative inverse side-buffer item count exceeds u32")?;
        let inv_params = [inv_items_u32, 0, 0, 0];
        let inv_params_buf = self.hal.create_uniform_buffer(
            "rv32im_accum_topaccum_arm5_authoritative_inv_buffer_params",
            bytemuck::cast_slice(&inv_params),
        )?;
        let inv_bind_group = self.hal.create_bind_group(
            "rv32im_accum_topaccum_arm5_authoritative_inv_buffer_bg",
            &inv_layout,
            &[
                WebGpuBufferBinding::new(0, &inv_buf),
                WebGpuBufferBinding::new(1, &inv_params_buf),
            ],
        )?;
        self.hal
            .dispatch_compute_1d(&inv_kernel, &inv_bind_group, inv_items_u32.div_ceil(64));
        self.hal
            .dispatch_compute_1d(&consume_raw_kernel, &bind_group, cycles);
        accum.buf.mark_gpu_dirty();
        ACCUM_GPU_ARM5_AUTHORITATIVE_DISPATCHES.fetch_add(1, Ordering::SeqCst);
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "rv32im_accumulate topaccum_arm5_authoritative dispatched cycles={cycles} inv_items={inv_items}"
        ));
        Ok(true)
    }

    pub(crate) fn lookup_accum_misc0_direct_kernel(&self) -> Result<WebGpuKernel> {
        if let Some(kernel) = ACCUM_MISC0_DIRECT_KERNEL.with(|cell| cell.borrow().clone()) {
            return Ok(kernel);
        }
        let layout = self.hal.create_bind_group_layout(
            "rv32im_accum_misc0_direct_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let wgsl = accum_misc0_direct_wgsl();
        let kernel = self.hal.create_compute_kernel(
            "rv32im_accum_misc0_direct",
            &wgsl,
            "main",
            &[layout],
        )?;
        ACCUM_MISC0_DIRECT_KERNEL.with(|cell| *cell.borrow_mut() = Some(kernel.clone()));
        Ok(kernel)
    }

    pub(crate) fn lookup_accum_misc2_direct_kernel(&self) -> Result<WebGpuKernel> {
        if let Some(kernel) = ACCUM_MISC2_DIRECT_KERNEL.with(|cell| cell.borrow().clone()) {
            return Ok(kernel);
        }
        let layout = self.hal.create_bind_group_layout(
            "rv32im_accum_misc2_direct_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let wgsl = accum_misc2_direct_wgsl();
        let kernel = self.hal.create_compute_kernel(
            "rv32im_accum_misc2_direct",
            &wgsl,
            "main",
            &[layout],
        )?;
        ACCUM_MISC2_DIRECT_KERNEL.with(|cell| *cell.borrow_mut() = Some(kernel.clone()));
        Ok(kernel)
    }

    pub(crate) fn lookup_accum_misc1_direct_kernel(&self) -> Result<WebGpuKernel> {
        if let Some(kernel) = ACCUM_MISC1_DIRECT_KERNEL.with(|cell| cell.borrow().clone()) {
            return Ok(kernel);
        }
        let layout = self.hal.create_bind_group_layout(
            "rv32im_accum_misc1_direct_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let wgsl = accum_misc1_direct_wgsl();
        let kernel = self.hal.create_compute_kernel(
            "rv32im_accum_misc1_direct",
            &wgsl,
            "main",
            &[layout],
        )?;
        ACCUM_MISC1_DIRECT_KERNEL.with(|cell| *cell.borrow_mut() = Some(kernel.clone()));
        Ok(kernel)
    }

    pub(crate) fn lookup_accum_mem0_direct_kernel(&self) -> Result<WebGpuKernel> {
        if let Some(kernel) = ACCUM_MEM0_DIRECT_KERNEL.with(|cell| cell.borrow().clone()) {
            return Ok(kernel);
        }
        let layout = self.hal.create_bind_group_layout(
            "rv32im_accum_mem0_direct_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let wgsl = accum_mem0_direct_wgsl();
        let kernel =
            self.hal
                .create_compute_kernel("rv32im_accum_mem0_direct", &wgsl, "main", &[layout])?;
        ACCUM_MEM0_DIRECT_KERNEL.with(|cell| *cell.borrow_mut() = Some(kernel.clone()));
        Ok(kernel)
    }

    pub(crate) fn lookup_accum_mem1_direct_kernel(&self) -> Result<WebGpuKernel> {
        if let Some(kernel) = ACCUM_MEM1_DIRECT_KERNEL.with(|cell| cell.borrow().clone()) {
            return Ok(kernel);
        }
        let layout = self.hal.create_bind_group_layout(
            "rv32im_accum_mem1_direct_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let wgsl = accum_mem1_direct_wgsl();
        let kernel =
            self.hal
                .create_compute_kernel("rv32im_accum_mem1_direct", &wgsl, "main", &[layout])?;
        ACCUM_MEM1_DIRECT_KERNEL.with(|cell| *cell.borrow_mut() = Some(kernel.clone()));
        Ok(kernel)
    }

    pub(crate) fn lookup_accum_control0_direct_kernel(&self) -> Result<WebGpuKernel> {
        if let Some(kernel) = ACCUM_CONTROL0_DIRECT_KERNEL.with(|cell| cell.borrow().clone()) {
            return Ok(kernel);
        }
        let layout = self.hal.create_bind_group_layout(
            "rv32im_accum_control0_direct_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let wgsl = accum_control0_direct_wgsl();
        let kernel = self.hal.create_compute_kernel(
            "rv32im_accum_control0_direct",
            &wgsl,
            "main",
            &[layout],
        )?;
        ACCUM_CONTROL0_DIRECT_KERNEL.with(|cell| *cell.borrow_mut() = Some(kernel.clone()));
        Ok(kernel)
    }

    pub(crate) fn lookup_accum_poseidon1_direct_kernel(&self) -> Result<WebGpuKernel> {
        if let Some(kernel) = ACCUM_POSEIDON1_DIRECT_KERNEL.with(|cell| cell.borrow().clone()) {
            return Ok(kernel);
        }
        let layout = self.hal.create_bind_group_layout(
            "rv32im_accum_poseidon1_direct_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let wgsl = accum_poseidon1_direct_wgsl();
        let kernel = self.hal.create_compute_kernel(
            "rv32im_accum_poseidon1_direct",
            &wgsl,
            "main",
            &[layout],
        )?;
        ACCUM_POSEIDON1_DIRECT_KERNEL.with(|cell| *cell.borrow_mut() = Some(kernel.clone()));
        Ok(kernel)
    }

    pub(crate) fn lookup_accum_misc_direct_kernel(
        &self,
        kind: AccumMiscDirectKind,
    ) -> Result<WebGpuKernel> {
        match kind {
            AccumMiscDirectKind::Misc0 => self.lookup_accum_misc0_direct_kernel(),
            AccumMiscDirectKind::Misc1 => self.lookup_accum_misc1_direct_kernel(),
            AccumMiscDirectKind::Misc2 => self.lookup_accum_misc2_direct_kernel(),
            AccumMiscDirectKind::Mem0 => self.lookup_accum_mem0_direct_kernel(),
            AccumMiscDirectKind::Mem1 => self.lookup_accum_mem1_direct_kernel(),
            AccumMiscDirectKind::Control0 => self.lookup_accum_control0_direct_kernel(),
            AccumMiscDirectKind::Poseidon1 => self.lookup_accum_poseidon1_direct_kernel(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_accum_misc_direct_grouped(
        &self,
        misc0_rows: &[u32],
        misc1_rows: &[u32],
        misc2_rows: &[u32],
        mem0_rows: &[u32],
        mem1_rows: &[u32],
        control0_rows: &[u32],
        poseidon1_rows: &[u32],
        data: &MetaBuffer<WebGpuHal>,
        accum: &MetaBuffer<WebGpuHal>,
        mix: &MetaBuffer<WebGpuHal>,
    ) -> Result<bool> {
        if misc0_rows.is_empty()
            && misc1_rows.is_empty()
            && misc2_rows.is_empty()
            && mem0_rows.is_empty()
            && mem1_rows.is_empty()
            && control0_rows.is_empty()
            && poseidon1_rows.is_empty()
        {
            return Ok(true);
        }

        let _timer = WebGpuStageTimer::new_active_for(
            "rv32im_accumulate misc_direct_gpu_grouped",
            self.hal.as_ref(),
        );
        data.buf.sync_cpu_to_gpu(self.hal.as_ref())?;
        mix.buf.sync_cpu_to_gpu(self.hal.as_ref())?;
        let data_gpu = data
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("MISC direct accum: data missing GPU storage"))?;
        let accum_gpu = accum
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("MISC direct accum: accum missing GPU storage"))?;
        let mix_gpu = mix
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("MISC direct accum: mix missing GPU storage"))?;

        let mut kernels = Vec::new();
        let mut bind_groups = Vec::new();
        let mut row_bufs = Vec::new();
        let mut params_bufs = Vec::new();
        let mut records = Vec::new();

        for (kind, rows) in [
            (AccumMiscDirectKind::Misc0, misc0_rows),
            (AccumMiscDirectKind::Misc1, misc1_rows),
            (AccumMiscDirectKind::Misc2, misc2_rows),
            (AccumMiscDirectKind::Mem0, mem0_rows),
            (AccumMiscDirectKind::Mem1, mem1_rows),
            (AccumMiscDirectKind::Control0, control0_rows),
            (AccumMiscDirectKind::Poseidon1, poseidon1_rows),
        ] {
            if rows.is_empty() {
                continue;
            }
            let label = kind.label();
            let row_count = u32::try_from(rows.len())
                .with_context(|| format!("{label} direct accum row count exceeds u32"))?;
            let kernel = self.lookup_accum_misc_direct_kernel(kind)?;
            let row_buf_bytes = u64::try_from(
                rows.len()
                    .checked_mul(std::mem::size_of::<u32>())
                    .with_context(|| format!("{label} direct accum row-list byte size overflow"))?,
            )
            .with_context(|| format!("{label} direct accum row-list byte size exceeds u64"))?;
            let row_buf = self
                .hal
                .create_storage_buffer(kind.row_source(), row_buf_bytes)?;
            self.hal.write_buffer_named(
                &row_buf,
                kind.row_source(),
                0,
                bytemuck::cast_slice(rows),
            )?;

            let data_base =
                u32::try_from(data.buf.byte_offset() / std::mem::size_of::<Val>() as u64)
                    .with_context(|| {
                        format!("{label} direct accum data buffer offset exceeds u32")
                    })?;
            let accum_base =
                u32::try_from(accum.buf.byte_offset() / std::mem::size_of::<Val>() as u64)
                    .with_context(|| {
                        format!("{label} direct accum accum buffer offset exceeds u32")
                    })?;
            let mix_base = u32::try_from(mix.buf.byte_offset() / std::mem::size_of::<Val>() as u64)
                .with_context(|| format!("{label} direct accum mix buffer offset exceeds u32"))?;
            let params = [
                u32::try_from(data.rows)
                    .with_context(|| format!("{label} direct accum data rows exceed u32"))?,
                u32::try_from(accum.rows)
                    .with_context(|| format!("{label} direct accum accum rows exceed u32"))?,
                row_count,
                data_base,
                accum_base,
                mix_base,
                0,
                0,
            ];
            let params_buf = self
                .hal
                .create_uniform_buffer(kind.params_source(), bytemuck::cast_slice(&params))?;
            let layout = self.hal.create_bind_group_layout(
                kind.layout_label(),
                &[
                    WebGpuBindingLayout::read_only_storage(0, 0),
                    WebGpuBindingLayout::storage(1, 0),
                    WebGpuBindingLayout::read_only_storage(2, 0),
                    WebGpuBindingLayout::read_only_storage(3, 0),
                    WebGpuBindingLayout::uniform(4, 32),
                ],
            )?;
            let bind_group = self.hal.create_bind_group(
                kind.bind_group_label(),
                &layout,
                &[
                    WebGpuBufferBinding::new(0, data_gpu),
                    WebGpuBufferBinding::new(1, accum_gpu),
                    WebGpuBufferBinding::new(2, mix_gpu),
                    WebGpuBufferBinding::new(3, &row_buf),
                    WebGpuBufferBinding::new(4, &params_buf),
                ],
            )?;

            records.push((kind, rows.len(), row_count, row_count.div_ceil(64)));
            kernels.push(kernel);
            bind_groups.push(bind_group);
            row_bufs.push(row_buf);
            params_bufs.push(params_buf);
        }

        let dispatches = kernels
            .iter()
            .zip(bind_groups.iter())
            .zip(records.iter())
            .map(|((kernel, bind_group), (_, _, _, workgroups))| (kernel, bind_group, *workgroups))
            .collect::<Vec<_>>();
        self.hal
            .dispatch_compute_1d_bind_group_sequence(&dispatches);
        for (kind, rows, row_count, _) in records {
            kind.record_rows(rows);
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "rv32im_accumulate {} direct_gpu dispatched rows={}",
                kind.label(),
                row_count
            ));
        }
        Ok(true)
    }
}
