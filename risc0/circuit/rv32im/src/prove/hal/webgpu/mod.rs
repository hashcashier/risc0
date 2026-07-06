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

pub(crate) use std::cell::{Cell, RefCell};
pub(crate) use std::fmt::Write as _;
pub(crate) use std::rc::Rc;
pub(crate) use std::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};

pub(crate) use anyhow::{Context as _, Result};
pub(crate) use risc0_core::scope;
pub(crate) use risc0_zkp::{
    adapter::{CircuitInfo as _, PROOF_SYSTEM_INFO},
    field::{Elem as _, ExtElem as _},
    hal::{
        webgpu::{
            WebGpuBindingLayout, WebGpuBuffer, WebGpuBufferBinding, WebGpuCircuitEvalCheck,
            WebGpuHal, WebGpuKernel, WebGpuStageTimer, WebGpuStartedComputeKernel,
        },
        AccumPreflight, Buffer, CircuitHal, Hal,
    },
    prove::Prover,
};

pub(crate) use super::{
    CircuitAccumulator, CircuitWitnessGenerator, MetaBuffer, SegmentProverImpl, StepMode,
};
pub(crate) use crate::prove::{witgen::PreflightResults, SegmentProver};
pub(crate) use crate::{
    prove::witgen::preflight::PreflightTrace,
    zirgen::{
        circuit::{
            ArgU16Layout, ArgU8Layout, CycleArgLayout, DoCycleTableLayout, ExtVal,
            FinalizeMiscLayout, Mem0Layout, Mem1Layout, MemoryArgLayout, MiscInputLayout, Val,
            LAYOUT_MIX, LAYOUT_TOP, LAYOUT_TOP_ACCUM, REGCOUNT_ACCUM, REGCOUNT_MIX,
            REGISTER_GROUP_ACCUM, REGISTER_GROUP_CODE, REGISTER_GROUP_DATA,
        },
        taps::TAPSET,
        CircuitImpl,
    },
    RV32IM_SEAL_VERSION,
};

// GPU witness generation sits behind two opt-in process-global
// flags. Probe mode starts the background per-arm kernel prewarm and
// logs compile/readiness metrics while rust_steps stays
// authoritative. Replace mode additionally dispatches the ready arms
// and short-circuits rust_steps for the covered cycles. The browser
// prover's acceleration init enables both by default; kernel
// compiles are multi-second Tint operations, so a run that never
// dispatches them should leave the flags off.
/// Process-global flag that turns on the background GPU witgen
/// kernel prewarm (probe mode). The browser prover's acceleration
/// init enables it together with replace mode. Atomic so it can be
/// read from sync paths without RefCell borrow churn.
pub static WITGEN_GPU_PROBE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Public setter for the witgen GPU probe flag.
pub fn set_witgen_gpu_probe_enabled(enabled: bool) {
    WITGEN_GPU_PROBE_ENABLED.store(enabled, Ordering::SeqCst);
}

/// Process-global flag that, when set,
/// causes rust_steps to skip its `step_Top` call for cycles whose major
/// opcode is in the 8 zero-back_Reg arms (MISC0/1/2, MUL0, DIV0,
/// MEM0/1, ECALL0). The GPU prewarm + per-arm dispatch must have
/// populated data_buf for those arms first. Default off. Tests opt in.
pub static WITGEN_GPU_REPLACE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Public setter for the witgen GPU replace flag.
pub fn set_witgen_gpu_replace_enabled(enabled: bool) {
    WITGEN_GPU_REPLACE_ENABLED.store(enabled, Ordering::SeqCst);
    WITGEN_GPU_REPLACE_ON_DEMAND_KERNEL_COMPILES.store(0, Ordering::SeqCst);
    super::rust_steps::set_witgen_gpu_replace_enabled(enabled);
}

pub fn witgen_gpu_short_circuit_cycles() -> usize {
    super::rust_steps::witgen_gpu_short_circuit_cycles()
}

pub fn witgen_gpu_replace_arm_mask() -> u16 {
    super::rust_steps::witgen_gpu_replace_arm_mask()
}

pub fn witgen_accum_shadow_replay_rows() -> usize {
    super::rust_steps::witgen_accum_shadow_replay_rows()
}

pub fn set_witgen_gpu_direct_misc0_accum_enabled(enabled: bool) {
    super::rust_steps::set_witgen_gpu_direct_misc0_accum_enabled(enabled);
}

pub fn witgen_gpu_direct_misc0_accum_rows() -> usize {
    super::rust_steps::witgen_gpu_direct_misc0_accum_rows()
}

pub(crate) static WITGEN_GPU_REPLACE_ON_DEMAND_KERNEL_COMPILES: AtomicUsize = AtomicUsize::new(0);
pub(crate) static WITGEN_GPU_REPLACE_NONBLOCKING_PENDING_ENABLED: AtomicBool =
    AtomicBool::new(false);
pub(crate) static WITGEN_GPU_REPLACE_NONBLOCKING_PENDING_SKIPS: AtomicUsize = AtomicUsize::new(0);

pub fn witgen_gpu_replace_on_demand_kernel_compiles() -> usize {
    WITGEN_GPU_REPLACE_ON_DEMAND_KERNEL_COMPILES.load(Ordering::SeqCst)
}

pub fn set_witgen_gpu_replace_nonblocking_pending_enabled(enabled: bool) {
    WITGEN_GPU_REPLACE_NONBLOCKING_PENDING_SKIPS.store(0, Ordering::SeqCst);
    WITGEN_GPU_REPLACE_NONBLOCKING_PENDING_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn witgen_gpu_replace_nonblocking_pending_skips() -> usize {
    WITGEN_GPU_REPLACE_NONBLOCKING_PENDING_SKIPS.load(Ordering::SeqCst)
}

pub(crate) fn record_witgen_replace_on_demand_kernel_compile(label: &str) {
    let count = WITGEN_GPU_REPLACE_ON_DEMAND_KERNEL_COMPILES.fetch_add(1, Ordering::SeqCst) + 1;
    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
        "witgen_arm_on_demand_compile_count label={label} count={count}"
    ));
}

/// Cell-level diff diagnostic. When enabled,
/// `prove_core_async` runs the GPU pre-dispatch normally, snapshots the
/// CPU shadow data buffer, then resets it to INVALID + re-scatters the
/// injector + forces mask=0, runs rust_steps to fill everything via the
/// pure-CPU path, snapshots again, and emits the first N cells where
/// (gpu_snap != INVALID && cpu_snap != INVALID && gpu_snap != cpu_snap).
/// Bails out at the end so the test fails fast with the diagnostic
/// output. Requires both PROBE and REPLACE off (otherwise the early
/// returns in `pre_witgen_dispatch_async` make the diagnostic a no-op).
pub static WITGEN_GPU_DIFF_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) const WITGEN_GPU_DIFF_MAJOR_NONE: usize = usize::MAX;
pub(crate) static WITGEN_GPU_DIFF_MAJOR: AtomicUsize = AtomicUsize::new(WITGEN_GPU_DIFF_MAJOR_NONE);
pub(crate) static WITGEN_GPU_REPLACE_DIFF_TARGET_SEGMENT: AtomicUsize = AtomicUsize::new(0);
pub(crate) static WITGEN_GPU_REPLACE_DIFF_SEEN_SEGMENTS: AtomicUsize = AtomicUsize::new(0);

/// Public setter for the witgen GPU diff flag.
pub fn set_witgen_gpu_diff_enabled(enabled: bool) {
    WITGEN_GPU_DIFF_ENABLED.store(enabled, Ordering::SeqCst);
    if enabled {
        WITGEN_GPU_REPLACE_DIFF_SEEN_SEGMENTS.store(0, Ordering::SeqCst);
    }
}

/// Select a single major opcode arm for diff-only GPU-witgen screening.
///
/// This intentionally does not affect authoritative replacement mode. It lets
/// the diagnostic path compile and dispatch one candidate arm against real
/// traces before we add any CPU short-circuiting or proof-path behavior.
pub fn set_witgen_gpu_diff_major(major: Option<u8>) {
    if let Some(major) = major {
        assert!(major < 13, "witgen diff major must be in 0..13");
        WITGEN_GPU_DIFF_MAJOR.store(major as usize, Ordering::SeqCst);
    } else {
        WITGEN_GPU_DIFF_MAJOR.store(WITGEN_GPU_DIFF_MAJOR_NONE, Ordering::SeqCst);
    }
}

/// Select which replacement-mode segment should stop for final matrix diff.
///
/// Replacement-diff mode still proves earlier segments using the replacement
/// path so multi-segment fixtures can reach the segment that failed receipt
/// verification. `None` restores the historical first-segment diagnostic.
pub fn set_witgen_gpu_replace_diff_target_segment(segment: Option<usize>) {
    WITGEN_GPU_REPLACE_DIFF_TARGET_SEGMENT.store(segment.unwrap_or(0), Ordering::SeqCst);
    WITGEN_GPU_REPLACE_DIFF_SEEN_SEGMENTS.store(0, Ordering::SeqCst);
}

pub(crate) fn finish_witgen_from_populated_parts(
    hal: &WebGpuHal,
    trace: PreflightTrace,
    cycles: usize,
    global: MetaBuffer<WebGpuHal>,
    code: MetaBuffer<WebGpuHal>,
    data: MetaBuffer<WebGpuHal>,
) -> super::super::witgen::WitnessGenerator<WebGpuHal> {
    hal.eltwise_zeroize_elem(&global.buf);
    hal.eltwise_zeroize_elem(&data.buf);
    let accum = MetaBuffer::new("accum", hal, cycles, REGCOUNT_ACCUM, true);
    super::super::witgen::WitnessGenerator::<WebGpuHal> {
        cycles,
        global,
        code,
        data,
        accum,
        trace,
    }
}

/// TopAccum arm-5 real-buffer probe. This is intentionally opt-in:
/// the first version validates one real proof row after CPU TopAccum has
/// populated authoritative buffers, then leaves the normal proof flow to
/// verify the receipt end to end.
pub static ACCUM_GPU_MAJOR_HISTOGRAM_ENABLED: AtomicBool = AtomicBool::new(false);
pub static ACCUM_GPU_ARM5_PROBE_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) static ACCUM_GPU_ARM5_PROBE_DISPATCHES: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_ARM5_AUTHORITATIVE_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) static ACCUM_GPU_ARM5_AUTHORITATIVE_DISPATCHES: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_CANDIDATE_SYNC_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) static ACCUM_GPU_CANDIDATE_SYNC_WAITS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_MISC0_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) static ACCUM_GPU_MISC0_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_MISC1_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) static ACCUM_GPU_MISC1_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_MISC2_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) static ACCUM_GPU_MISC2_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_MEM0_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) static ACCUM_GPU_MEM0_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_MEM1_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) static ACCUM_GPU_MEM1_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_CONTROL0_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) static ACCUM_GPU_CONTROL0_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_POSEIDON1_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) static ACCUM_GPU_POSEIDON1_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
pub(crate) static WITGEN_GPU_MEM0_REPLACE_CANDIDATE_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) const WITGEN_GPU_MEM0_REPLACE_MINOR_MASK_ALL: u16 = 0x001f;
pub(crate) static WITGEN_GPU_MEM0_REPLACE_MINOR_MASK: AtomicU16 =
    AtomicU16::new(WITGEN_GPU_MEM0_REPLACE_MINOR_MASK_ALL);
pub(crate) static WITGEN_GPU_MEM0_EXTRA_PREWARM_REQUESTS: AtomicUsize = AtomicUsize::new(0);
pub(crate) static WITGEN_GPU_MEM1_REPLACE_CANDIDATE_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) const WITGEN_GPU_MEM1_REPLACE_MINOR_MASK_ALL: u16 = 0x0007;
pub(crate) static WITGEN_GPU_MEM1_REPLACE_MINOR_MASK: AtomicU16 =
    AtomicU16::new(WITGEN_GPU_MEM1_REPLACE_MINOR_MASK_ALL);
pub(crate) static WITGEN_GPU_MEM1_EXTRA_PREWARM_REQUESTS: AtomicUsize = AtomicUsize::new(0);

pub fn set_accum_gpu_major_histogram_enabled(enabled: bool) {
    ACCUM_GPU_MAJOR_HISTOGRAM_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn set_accum_gpu_arm5_probe_enabled(enabled: bool) {
    if enabled {
        ACCUM_GPU_ARM5_PROBE_DISPATCHES.store(0, Ordering::SeqCst);
        TOPACCUM_ARM5_PROBE_COMPARE.with(|cell| *cell.borrow_mut() = None);
    }
    ACCUM_GPU_ARM5_PROBE_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn accum_gpu_arm5_probe_dispatches() -> usize {
    ACCUM_GPU_ARM5_PROBE_DISPATCHES.load(Ordering::SeqCst)
}

pub fn set_accum_gpu_arm5_authoritative_enabled(enabled: bool) {
    if enabled {
        ACCUM_GPU_ARM5_AUTHORITATIVE_DISPATCHES.store(0, Ordering::SeqCst);
    }
    ACCUM_GPU_ARM5_AUTHORITATIVE_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn accum_gpu_arm5_authoritative_dispatches() -> usize {
    ACCUM_GPU_ARM5_AUTHORITATIVE_DISPATCHES.load(Ordering::SeqCst)
}

pub fn set_accum_gpu_candidate_sync_enabled(enabled: bool) {
    ACCUM_GPU_CANDIDATE_SYNC_WAITS.store(0, Ordering::SeqCst);
    ACCUM_GPU_CANDIDATE_SYNC_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn accum_gpu_candidate_sync_waits() -> usize {
    ACCUM_GPU_CANDIDATE_SYNC_WAITS.load(Ordering::SeqCst)
}

pub fn set_accum_gpu_misc0_direct_enabled(enabled: bool) {
    ACCUM_GPU_MISC0_DIRECT_ROWS.store(0, Ordering::SeqCst);
    ACCUM_GPU_MISC0_DIRECT_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn accum_gpu_misc0_direct_rows() -> usize {
    ACCUM_GPU_MISC0_DIRECT_ROWS.load(Ordering::SeqCst)
}

pub fn set_accum_gpu_misc1_direct_enabled(enabled: bool) {
    ACCUM_GPU_MISC1_DIRECT_ROWS.store(0, Ordering::SeqCst);
    ACCUM_GPU_MISC1_DIRECT_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn accum_gpu_misc1_direct_rows() -> usize {
    ACCUM_GPU_MISC1_DIRECT_ROWS.load(Ordering::SeqCst)
}

pub fn set_accum_gpu_misc2_direct_enabled(enabled: bool) {
    ACCUM_GPU_MISC2_DIRECT_ROWS.store(0, Ordering::SeqCst);
    ACCUM_GPU_MISC2_DIRECT_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn accum_gpu_misc2_direct_rows() -> usize {
    ACCUM_GPU_MISC2_DIRECT_ROWS.load(Ordering::SeqCst)
}

pub fn set_accum_gpu_mem0_direct_enabled(enabled: bool) {
    ACCUM_GPU_MEM0_DIRECT_ROWS.store(0, Ordering::SeqCst);
    ACCUM_GPU_MEM0_DIRECT_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn accum_gpu_mem0_direct_rows() -> usize {
    ACCUM_GPU_MEM0_DIRECT_ROWS.load(Ordering::SeqCst)
}

pub fn set_accum_gpu_mem1_direct_enabled(enabled: bool) {
    ACCUM_GPU_MEM1_DIRECT_ROWS.store(0, Ordering::SeqCst);
    ACCUM_GPU_MEM1_DIRECT_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn accum_gpu_mem1_direct_rows() -> usize {
    ACCUM_GPU_MEM1_DIRECT_ROWS.load(Ordering::SeqCst)
}

pub fn set_accum_gpu_control0_direct_enabled(enabled: bool) {
    ACCUM_GPU_CONTROL0_DIRECT_ROWS.store(0, Ordering::SeqCst);
    ACCUM_GPU_CONTROL0_DIRECT_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn accum_gpu_control0_direct_rows() -> usize {
    ACCUM_GPU_CONTROL0_DIRECT_ROWS.load(Ordering::SeqCst)
}

pub fn set_accum_gpu_poseidon1_direct_enabled(enabled: bool) {
    ACCUM_GPU_POSEIDON1_DIRECT_ROWS.store(0, Ordering::SeqCst);
    ACCUM_GPU_POSEIDON1_DIRECT_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn accum_gpu_poseidon1_direct_rows() -> usize {
    ACCUM_GPU_POSEIDON1_DIRECT_ROWS.load(Ordering::SeqCst)
}

pub fn set_witgen_gpu_mem0_replace_candidate_enabled(enabled: bool) {
    WITGEN_GPU_MEM0_REPLACE_CANDIDATE_ENABLED.store(enabled, Ordering::SeqCst);
    if enabled {
        super::rust_steps::set_witgen_gpu_mem0_replace_minor_mask(
            witgen_gpu_mem0_replace_minor_mask(),
        );
    } else {
        set_witgen_gpu_mem0_replace_minor_mask(WITGEN_GPU_MEM0_REPLACE_MINOR_MASK_ALL);
    }
}

pub fn set_witgen_gpu_mem0_replace_minor_mask(mask: u16) {
    let mask = mask & WITGEN_GPU_MEM0_REPLACE_MINOR_MASK_ALL;
    WITGEN_GPU_MEM0_EXTRA_PREWARM_REQUESTS.store(0, Ordering::SeqCst);
    WITGEN_GPU_MEM0_REPLACE_MINOR_MASK.store(mask, Ordering::SeqCst);
    super::rust_steps::set_witgen_gpu_mem0_replace_minor_mask(mask);
}

pub fn witgen_gpu_mem0_replace_minor_mask() -> u16 {
    WITGEN_GPU_MEM0_REPLACE_MINOR_MASK.load(Ordering::SeqCst)
}

pub fn witgen_gpu_mem0_extra_prewarm_requests() -> usize {
    WITGEN_GPU_MEM0_EXTRA_PREWARM_REQUESTS.load(Ordering::SeqCst)
}

pub fn set_witgen_gpu_mem1_replace_candidate_enabled(enabled: bool) {
    WITGEN_GPU_MEM1_REPLACE_CANDIDATE_ENABLED.store(enabled, Ordering::SeqCst);
    if enabled {
        super::rust_steps::set_witgen_gpu_mem1_replace_minor_mask(
            witgen_gpu_mem1_replace_minor_mask(),
        );
    } else {
        set_witgen_gpu_mem1_replace_minor_mask(WITGEN_GPU_MEM1_REPLACE_MINOR_MASK_ALL);
    }
}

pub fn set_witgen_gpu_mem1_replace_minor_mask(mask: u16) {
    let mask = mask & WITGEN_GPU_MEM1_REPLACE_MINOR_MASK_ALL;
    WITGEN_GPU_MEM1_EXTRA_PREWARM_REQUESTS.store(0, Ordering::SeqCst);
    WITGEN_GPU_MEM1_REPLACE_MINOR_MASK.store(mask, Ordering::SeqCst);
    super::rust_steps::set_witgen_gpu_mem1_replace_minor_mask(mask);
}

pub fn witgen_gpu_mem1_replace_minor_mask() -> u16 {
    WITGEN_GPU_MEM1_REPLACE_MINOR_MASK.load(Ordering::SeqCst)
}

pub fn witgen_gpu_mem1_extra_prewarm_requests() -> usize {
    WITGEN_GPU_MEM1_EXTRA_PREWARM_REQUESTS.load(Ordering::SeqCst)
}

#[derive(Clone, Copy, Debug)]
pub struct TopAccumArm5ProbeSummary {
    pub sample_cycle: u32,
    pub available_cycles: u32,
    pub preflight_major: u32,
    pub data_major_value: u32,
    pub selector_value: u32,
    pub mismatch_count: u32,
    pub first_mismatch_col: u32,
    pub first_mismatch_expected: u32,
    pub first_mismatch_actual: u32,
}

pub(crate) fn probe_u32(value: Val) -> u32 {
    if value == Val::INVALID {
        u32::MAX
    } else if value == Val::ZERO {
        0
    } else if value == Val::ONE {
        1
    } else {
        u32::from(value)
    }
}

pub(crate) fn read_data_cell_u32(
    data: &MetaBuffer<WebGpuHal>,
    row: usize,
    col: usize,
) -> Result<u32> {
    let idx = col
        .checked_mul(data.rows)
        .and_then(|base| base.checked_add(row))
        .context("TopAccum arm5 probe data cell index overflow")?;
    let mut value = Val::INVALID;
    data.buf.view(|cells| {
        value = cells[idx];
    });
    Ok(probe_u32(value))
}

pub(crate) fn log_accum_major_histogram(preflight: &PreflightTrace) {
    if !ACCUM_GPU_MAJOR_HISTOGRAM_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    let mut counts = [0usize; 13];
    let mut other = 0usize;
    for cycle in &preflight.cycles {
        if let Some(count) = counts.get_mut(cycle.major as usize) {
            *count += 1;
        } else {
            other += 1;
        }
    }
    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
        "rv32im_accumulate topaccum_major_histogram cycles={} major0={} major1={} major2={} major3={} major4={} major5={} major6={} major7={} major8={} major9={} major10={} major11={} major12={} other={}",
        preflight.cycles.len(),
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        counts[4],
        counts[5],
        counts[6],
        counts[7],
        counts[8],
        counts[9],
        counts[10],
        counts[11],
        counts[12],
        other,
    ));
}

pub async fn accum_gpu_arm5_probe_summary() -> Result<Option<TopAccumArm5ProbeSummary>> {
    let Some((hal, flags, details, mut summary)) = TOPACCUM_ARM5_PROBE_COMPARE.with(|cell| {
        cell.borrow().as_ref().map(|summary| {
            (
                summary.hal.clone(),
                summary.flags.clone(),
                summary.details.clone(),
                summary.summary,
            )
        })
    }) else {
        return Ok(None);
    };
    flags.sync_gpu_to_cpu(hal.as_ref()).await?;
    let flags = flags.to_vec();
    for (col, flag) in flags.iter().enumerate() {
        if *flag != Val::ZERO {
            summary.mismatch_count = summary.mismatch_count.saturating_add(1);
            if summary.first_mismatch_col == u32::MAX {
                summary.first_mismatch_col =
                    u32::try_from(col).context("TopAccum arm5 mismatch column exceeds u32")?;
            }
        }
    }
    details.sync_gpu_to_cpu(hal.as_ref()).await?;
    let details = details.to_vec();
    if summary.first_mismatch_col != u32::MAX {
        let detail_idx = usize::try_from(summary.first_mismatch_col)
            .context("TopAccum arm5 first mismatch column exceeds usize")?
            .checked_mul(2)
            .context("TopAccum arm5 mismatch detail index overflow")?;
        summary.first_mismatch_expected = details[detail_idx];
        summary.first_mismatch_actual = details[detail_idx + 1];
    }
    Ok(Some(summary))
}

pub async fn accum_gpu_arm5_probe_mismatch_summary() -> Result<Option<(u32, u32)>> {
    Ok(accum_gpu_arm5_probe_summary()
        .await?
        .map(|summary| (summary.mismatch_count, summary.first_mismatch_col)))
}

pub(crate) struct TopAccumArm5ProbeCompare {
    hal: Rc<WebGpuHal>,
    flags: WebGpuBuffer<Val>,
    details: WebGpuBuffer<u32>,
    summary: TopAccumArm5ProbeSummary,
}

thread_local! {
    /// Per-arm kernel cache for TOP_CHUNK0_ARM_DELTAS.
    /// Keyed by arm label (the first element of each tuple in
    /// TOP_CHUNK0_ARM_DELTAS), one entry per major opcode arm.
    /// Populated by the spawn_local prewarm task; consumed by the
    /// per-arm replace dispatch path. Thread-local (not a struct
    /// field) because ProverImpl's segment_prover constructs a fresh
    /// WebGpuCircuitHal per segment, which would defeat the cache.
    pub(crate) static WITGEN_ARM_KERNELS: RefCell<std::collections::BTreeMap<&'static str, WebGpuKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Replacement-arm chunk0 compiles that have been started via
    /// createComputePipelineAsync but have not resolved into cached kernels.
    pub(crate) static WITGEN_ARM_PENDING_KERNELS: RefCell<std::collections::BTreeMap<&'static str, WebGpuStartedComputeKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Per-arm chunk1 kernel cache. Mirrors
    /// WITGEN_ARM_KERNELS but for the chunk1 sub-fns (exec_Misc0Chunk1
    /// etc.). Together with chunk0 these cover all minor opcodes of each
    /// arm; rust_steps short-circuit needs BOTH chunks dispatched per
    /// segment per arm to be safe.
    pub(crate) static WITGEN_ARM_KERNELS_CHUNK1: RefCell<std::collections::BTreeMap<&'static str, WebGpuKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Pending chunk1 compiles for replacement arms.
    pub(crate) static WITGEN_ARM_PENDING_KERNELS_CHUNK1: RefCell<std::collections::BTreeMap<&'static str, WebGpuStartedComputeKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Extra per-minor MISC0 witgen slices beyond the globally prewarmed
    /// chunk0/chunk1 pair. Keyed by MISC0 minor opcode.
    pub(crate) static WITGEN_MISC0_EXTRA_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Pending MISC0 extra per-minor compiles.
    pub(crate) static WITGEN_MISC0_EXTRA_PENDING_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuStartedComputeKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Extra per-minor MISC2 witgen slices beyond chunk0/chunk1.
    /// Keyed by MISC2 minor opcode.
    pub(crate) static WITGEN_MISC2_EXTRA_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Extra per-minor MEM0 witgen slices beyond chunk0/chunk1.
    /// Production replacement uses the legal load minors after the diff gate
    /// proves cell-complete GPU output.
    pub(crate) static WITGEN_MEM0_EXTRA_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Pending MEM0 extra per-minor compiles.
    pub(crate) static WITGEN_MEM0_EXTRA_PENDING_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuStartedComputeKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Extra per-minor MEM1 witgen slices beyond chunk0/chunk1.
    /// Diff-only screening uses these to test store rows as a chunk-complete
    /// GPU-witgen replacement candidate without changing production masks.
    pub(crate) static WITGEN_MEM1_EXTRA_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Pending MEM1 extra per-minor compiles.
    pub(crate) static WITGEN_MEM1_EXTRA_PENDING_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuStartedComputeKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Tracks whether the async prewarm task has been spawned this
    /// session, so multiple `prewarm_witgen_kernel()` calls (one per
    /// segment_prover) only fire the compile once.
    pub(crate) static WITGEN_PREWARM_SPAWNED: Cell<bool> = const { Cell::new(false) };
    /// Cached shadow_init pipeline.
    /// Compiled once per session and reused across all segments.
    pub(crate) static SHADOW_INIT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// TopAccum arm-5 split-inverse scratch probe pipelines. The large
    /// arm5 module stays free of the real ext_inv body; it captures inverse
    /// inputs, a tiny kernel inverts them, then the arm5 module consumes the
    /// side-buffered inverses.
    pub(crate) static TOPACCUM_ARM5_INV_CAPTURE_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    pub(crate) static TOPACCUM_ARM5_INV_BUFFER_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    pub(crate) static TOPACCUM_ARM5_INV_CONSUME_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    pub(crate) static TOPACCUM_ARM5_INV_CONSUME_RAW_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// TopAccum arm-5 scratch-vs-CPU row compare pipeline.
    pub(crate) static TOPACCUM_ARM5_COMPARE_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow MISC0 accumulator replacement. Unlike generated TopAccum arms,
    /// this kernel only mirrors the direct Rust helper for the GPU-witgen-owned
    /// MISC0 minors, keeping module size and inverse pressure bounded.
    pub(crate) static ACCUM_MISC0_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow MISC1 accumulator replacement for CPU-witgen-owned major-1 rows.
    pub(crate) static ACCUM_MISC1_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow MISC2 accumulator replacement for CPU-witgen-owned major-2 rows.
    /// Kept separate from MISC2 GPU-witgen replacement so it avoids shadow
    /// repair readbacks while testing the larger TopAccum bucket.
    pub(crate) static ACCUM_MISC2_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow MEM0 accumulator replacement for GPU-witgen-owned load rows.
    /// This is the opt-in companion to MEM0 witgen replacement and avoids
    /// reading the whole MEM0 witness row back to CPU for step_TopAccum.
    pub(crate) static ACCUM_MEM0_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow MEM1 accumulator replacement for store rows. MEM1 rows stay
    /// CPU-witgen-owned; only their lookup accumulator contributions move to
    /// WebGPU.
    pub(crate) static ACCUM_MEM1_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow CONTROL0 accumulator replacement. CONTROL0 witness rows stay
    /// CPU-owned; only their lookup accumulator contributions move to WebGPU.
    pub(crate) static ACCUM_CONTROL0_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow POSEIDON1 accumulator replacement. Paging-hash witness
    /// rows stay CPU-owned; only their two cycle-table accumulator terms
    /// move to WebGPU.
    pub(crate) static ACCUM_POSEIDON1_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Latest scratch-vs-CPU row compare result buffer, readable by the
    /// async browser test after proof generation finishes.
    pub(crate) static TOPACCUM_ARM5_PROBE_COMPARE: RefCell<Option<TopAccumArm5ProbeCompare>> =
        const { RefCell::new(None) };
}

#[derive(Default)]
pub(crate) struct WitgenReplacementPrewarm {
    arm_chunk0: Vec<(&'static str, WebGpuStartedComputeKernel)>,
    arm_chunk1: Vec<(&'static str, WebGpuStartedComputeKernel)>,
    misc0_extra: Vec<(u8, &'static str, WebGpuStartedComputeKernel)>,
    mem0_extra: Vec<(u8, &'static str, WebGpuStartedComputeKernel)>,
    mem1_extra: Vec<(u8, &'static str, WebGpuStartedComputeKernel)>,
}
#[allow(dead_code)]
pub(crate) struct WebGpuCircuitHal {
    hal: Rc<WebGpuHal>,
    /// Per-prove GPU-witgen replacement arm mask. Each segment prove
    /// owns a fresh `WebGpuCircuitHal`, so storing the mask here (instead
    /// of the legacy process-wide static in `rust_steps`) lets the segment
    /// pipeline overlap segment N+1's witgen phase — which computes its
    /// own mask — with segment N's accum phase, which still consumes N's.
    /// The static remains as a write-only diagnostics mirror for tests.
    witgen_replace_arm_mask: Cell<u16>,
}

// The circuit HAL is split by layer; the `pub use` globs keep every
// public item at its original `prove::hal::webgpu::*` path. Promoted
// `pub(crate)` items stay crate-internal.
mod accum_wgsl;
mod kernels;
mod phases;
mod session;
mod traits;
mod witgen_wgsl;

pub use kernels::*;
pub use session::*;
