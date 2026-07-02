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
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};

use anyhow::{Context as _, Result};
use risc0_core::scope;
use risc0_zkp::{
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

use super::{
    CircuitAccumulator, CircuitWitnessGenerator, MetaBuffer, PreflightResults, SegmentProver,
    SegmentProverImpl, StepMode,
};
use crate::{
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

/// SP7 iter 6d-g step 6.2.3: process-global flag that, when set,
/// causes rust_steps to skip its `step_Top` call for cycles whose major
/// opcode is in the 8 zero-back_Reg arms (MISC0/1/2, MUL0, DIV0,
/// MEM0/1, ECALL0). The GPU prewarm + per-arm dispatch must have
/// populated data_buf for those arms first. Default off. Tests opt in.
pub static WITGEN_GPU_REPLACE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Public setter for the iter-6d-g step 6.2.3 replace flag.
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

static WITGEN_GPU_REPLACE_ON_DEMAND_KERNEL_COMPILES: AtomicUsize = AtomicUsize::new(0);
static WITGEN_GPU_REPLACE_NONBLOCKING_PENDING_ENABLED: AtomicBool = AtomicBool::new(false);
static WITGEN_GPU_REPLACE_NONBLOCKING_PENDING_SKIPS: AtomicUsize = AtomicUsize::new(0);

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

fn record_witgen_replace_on_demand_kernel_compile(label: &str) {
    let count = WITGEN_GPU_REPLACE_ON_DEMAND_KERNEL_COMPILES.fetch_add(1, Ordering::SeqCst) + 1;
    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
        "iter6d_g_on_demand_compile_count label={label} count={count}"
    ));
}

/// SP7 iter 6d-g step 6.2.13: cell-level diff diagnostic. When enabled,
/// `prove_core_async` runs the GPU pre-dispatch normally, snapshots the
/// CPU shadow data buffer, then resets it to INVALID + re-scatters the
/// injector + forces mask=0, runs rust_steps to fill everything via the
/// pure-CPU path, snapshots again, and emits the first N cells where
/// (gpu_snap != INVALID && cpu_snap != INVALID && gpu_snap != cpu_snap).
/// Bails out at the end so the test fails fast with the diagnostic
/// output. Requires both PROBE and REPLACE off (otherwise the early
/// returns in `pre_witgen_dispatch_async` make the diagnostic a no-op).
pub static WITGEN_GPU_DIFF_ENABLED: AtomicBool = AtomicBool::new(false);
const WITGEN_GPU_DIFF_MAJOR_NONE: usize = usize::MAX;
static WITGEN_GPU_DIFF_MAJOR: AtomicUsize = AtomicUsize::new(WITGEN_GPU_DIFF_MAJOR_NONE);
static WITGEN_GPU_REPLACE_DIFF_TARGET_SEGMENT: AtomicUsize = AtomicUsize::new(0);
static WITGEN_GPU_REPLACE_DIFF_SEEN_SEGMENTS: AtomicUsize = AtomicUsize::new(0);

/// Public setter for the iter-6d-g step 6.2.13 diff flag.
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

fn finish_witgen_from_populated_parts(
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

/// SP7 TopAccum arm5 real-buffer probe. This is intentionally opt-in:
/// the first version validates one real proof row after CPU TopAccum has
/// populated authoritative buffers, then leaves the normal proof flow to
/// verify the receipt end to end.
pub static ACCUM_GPU_MAJOR_HISTOGRAM_ENABLED: AtomicBool = AtomicBool::new(false);
pub static ACCUM_GPU_ARM5_PROBE_ENABLED: AtomicBool = AtomicBool::new(false);
static ACCUM_GPU_ARM5_PROBE_DISPATCHES: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_ARM5_AUTHORITATIVE_ENABLED: AtomicBool = AtomicBool::new(false);
static ACCUM_GPU_ARM5_AUTHORITATIVE_DISPATCHES: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_CANDIDATE_SYNC_ENABLED: AtomicBool = AtomicBool::new(false);
static ACCUM_GPU_CANDIDATE_SYNC_WAITS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_MISC0_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
static ACCUM_GPU_MISC0_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_MISC1_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
static ACCUM_GPU_MISC1_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_MISC2_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
static ACCUM_GPU_MISC2_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_MEM0_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
static ACCUM_GPU_MEM0_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_MEM1_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
static ACCUM_GPU_MEM1_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
pub static ACCUM_GPU_CONTROL0_DIRECT_ENABLED: AtomicBool = AtomicBool::new(false);
static ACCUM_GPU_CONTROL0_DIRECT_ROWS: AtomicUsize = AtomicUsize::new(0);
static WITGEN_GPU_MEM0_REPLACE_CANDIDATE_ENABLED: AtomicBool = AtomicBool::new(false);
const WITGEN_GPU_MEM0_REPLACE_MINOR_MASK_ALL: u16 = 0x001f;
static WITGEN_GPU_MEM0_REPLACE_MINOR_MASK: AtomicU16 =
    AtomicU16::new(WITGEN_GPU_MEM0_REPLACE_MINOR_MASK_ALL);
static WITGEN_GPU_MEM0_EXTRA_PREWARM_REQUESTS: AtomicUsize = AtomicUsize::new(0);
static WITGEN_GPU_MEM1_REPLACE_CANDIDATE_ENABLED: AtomicBool = AtomicBool::new(false);
const WITGEN_GPU_MEM1_REPLACE_MINOR_MASK_ALL: u16 = 0x0007;
static WITGEN_GPU_MEM1_REPLACE_MINOR_MASK: AtomicU16 =
    AtomicU16::new(WITGEN_GPU_MEM1_REPLACE_MINOR_MASK_ALL);
static WITGEN_GPU_MEM1_EXTRA_PREWARM_REQUESTS: AtomicUsize = AtomicUsize::new(0);

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

fn probe_u32(value: Val) -> u32 {
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

fn read_data_cell_u32(data: &MetaBuffer<WebGpuHal>, row: usize, col: usize) -> Result<u32> {
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

fn log_accum_major_histogram(preflight: &PreflightTrace) {
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

struct TopAccumArm5ProbeCompare {
    hal: Rc<WebGpuHal>,
    flags: WebGpuBuffer<Val>,
    details: WebGpuBuffer<u32>,
    summary: TopAccumArm5ProbeSummary,
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
    /// Replacement-arm chunk0 compiles that have been started via
    /// createComputePipelineAsync but have not resolved into cached kernels.
    static WITGEN_ARM_PENDING_KERNELS: RefCell<std::collections::BTreeMap<&'static str, WebGpuStartedComputeKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// SP7 iter 6d-g step 6.2.4: per-arm chunk1 kernel cache. Mirrors
    /// WITGEN_ARM_KERNELS but for the chunk1 sub-fns (exec_Misc0Chunk1
    /// etc.). Together with chunk0 these cover all minor opcodes of each
    /// arm; rust_steps short-circuit needs BOTH chunks dispatched per
    /// segment per arm to be safe.
    static WITGEN_ARM_KERNELS_CHUNK1: RefCell<std::collections::BTreeMap<&'static str, WebGpuKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Pending chunk1 compiles for replacement arms.
    static WITGEN_ARM_PENDING_KERNELS_CHUNK1: RefCell<std::collections::BTreeMap<&'static str, WebGpuStartedComputeKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Extra per-minor MISC0 witgen slices beyond the globally prewarmed
    /// chunk0/chunk1 pair. Keyed by MISC0 minor opcode.
    static WITGEN_MISC0_EXTRA_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Pending MISC0 extra per-minor compiles.
    static WITGEN_MISC0_EXTRA_PENDING_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuStartedComputeKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Extra per-minor MISC2 witgen slices beyond chunk0/chunk1.
    /// Keyed by MISC2 minor opcode.
    static WITGEN_MISC2_EXTRA_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Extra per-minor MEM0 witgen slices beyond chunk0/chunk1.
    /// Production replacement uses the legal load minors after the diff gate
    /// proves cell-complete GPU output.
    static WITGEN_MEM0_EXTRA_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Pending MEM0 extra per-minor compiles.
    static WITGEN_MEM0_EXTRA_PENDING_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuStartedComputeKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Extra per-minor MEM1 witgen slices beyond chunk0/chunk1.
    /// Diff-only screening uses these to test store rows as a chunk-complete
    /// GPU-witgen replacement candidate without changing production masks.
    static WITGEN_MEM1_EXTRA_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Pending MEM1 extra per-minor compiles.
    static WITGEN_MEM1_EXTRA_PENDING_KERNELS: RefCell<std::collections::BTreeMap<u8, WebGpuStartedComputeKernel>> =
        RefCell::new(std::collections::BTreeMap::new());
    /// Tracks whether the async prewarm task has been spawned this
    /// session, so multiple `prewarm_witgen_kernel()` calls (one per
    /// segment_prover) only fire the compile once.
    static WITGEN_PREWARM_SPAWNED: Cell<bool> = const { Cell::new(false) };
    /// SP7 iter 6d-g step 6.2.0: cached shadow_init pipeline.
    /// Compiled once per session and reused across all segments.
    static SHADOW_INIT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// SP7 TopAccum arm5 real-buffer probe pipeline. Compiled lazily
    /// on first opt-in proof and reused for later segments.
    static TOPACCUM_ARM5_PROBE_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// SP7 TopAccum arm5 split-inverse scratch probe pipelines. The large
    /// arm5 module stays free of the real ext_inv body; it captures inverse
    /// inputs, a tiny kernel inverts them, then the arm5 module consumes the
    /// side-buffered inverses.
    static TOPACCUM_ARM5_INV_CAPTURE_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    static TOPACCUM_ARM5_INV_BUFFER_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    static TOPACCUM_ARM5_INV_CONSUME_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    static TOPACCUM_ARM5_INV_CONSUME_RAW_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// SP7 TopAccum arm5 scratch-vs-CPU row compare pipeline.
    static TOPACCUM_ARM5_COMPARE_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow MISC0 accumulator replacement. Unlike generated TopAccum arms,
    /// this kernel only mirrors the direct Rust helper for the GPU-witgen-owned
    /// MISC0 minors, keeping module size and inverse pressure bounded.
    static ACCUM_MISC0_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow MISC1 accumulator replacement for CPU-witgen-owned major-1 rows.
    static ACCUM_MISC1_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow MISC2 accumulator replacement for CPU-witgen-owned major-2 rows.
    /// Kept separate from MISC2 GPU-witgen replacement so it avoids shadow
    /// repair readbacks while testing the larger TopAccum bucket.
    static ACCUM_MISC2_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow MEM0 accumulator replacement for GPU-witgen-owned load rows.
    /// This is the opt-in companion to MEM0 witgen replacement and avoids
    /// reading the whole MEM0 witness row back to CPU for step_TopAccum.
    static ACCUM_MEM0_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow MEM1 accumulator replacement for store rows. MEM1 rows stay
    /// CPU-witgen-owned; only their lookup accumulator contributions move to
    /// WebGPU.
    static ACCUM_MEM1_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Narrow CONTROL0 accumulator replacement. CONTROL0 witness rows stay
    /// CPU-owned; only their lookup accumulator contributions move to WebGPU.
    static ACCUM_CONTROL0_DIRECT_KERNEL: RefCell<Option<WebGpuKernel>> =
        const { RefCell::new(None) };
    /// Latest scratch-vs-CPU row compare result buffer, readable by the
    /// async browser test after proof generation finishes.
    static TOPACCUM_ARM5_PROBE_COMPARE: RefCell<Option<TopAccumArm5ProbeCompare>> =
        const { RefCell::new(None) };
}

#[derive(Default)]
struct WitgenReplacementPrewarm {
    arm_chunk0: Vec<(&'static str, WebGpuStartedComputeKernel)>,
    arm_chunk1: Vec<(&'static str, WebGpuStartedComputeKernel)>,
    misc0_extra: Vec<(u8, &'static str, WebGpuStartedComputeKernel)>,
    mem0_extra: Vec<(u8, &'static str, WebGpuStartedComputeKernel)>,
    mem1_extra: Vec<(u8, &'static str, WebGpuStartedComputeKernel)>,
}

const TOPACCUM_ARM5_COMPARE_ROW_WGSL: &str = r#"
struct CompareParams {
  rows: u32,
  cols: u32,
  row: u32,
  _pad: u32,
}

@group(0) @binding(0) var<storage, read> expected_accum: array<u32>;
@group(0) @binding(1) var<storage, read> actual_accum: array<u32>;
@group(0) @binding(2) var<storage, read_write> mismatch_flags: array<u32>;
@group(0) @binding(3) var<storage, read_write> mismatch_details: array<u32>;
@group(0) @binding(4) var<uniform> params: CompareParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let col = gid.x;
  if (col >= params.cols) {
    return;
  }
  let idx = col * params.rows + params.row;
  let expected = expected_accum[idx];
  let actual = actual_accum[idx];
  mismatch_flags[col] = select(0u, 1u, expected != actual);
  mismatch_details[col * 2u] = expected;
  mismatch_details[col * 2u + 1u] = actual;
}
"#;

const TOPACCUM_ARM5_INV_CALLS: u32 = 26;
const TOPACCUM_ARM5_INV_CAPTURE_ENTRY_TEMPLATE: &str = r#"
@group(0) @binding(5) var<storage, read> topaccum_arm5_cycle_list: array<u32>;
@group(0) @binding(6) var<storage, read_write> topaccum_arm5_inv_buf: array<u32>;

const TOPACCUM_ARM5_INV_COUNT: u32 = __INV_COUNT__;
var<private> topaccum_arm5_inv_item: u32;

fn topaccum_arm5_inv_base(slot: u32) -> u32 {
  return (topaccum_arm5_inv_item * TOPACCUM_ARM5_INV_COUNT + slot) * 4u;
}

fn topaccum_arm5_capture_inv(slot: u32, x: ExtVal) -> ExtVal {
  let base = topaccum_arm5_inv_base(slot);
  topaccum_arm5_inv_buf[base] = x.x;
  topaccum_arm5_inv_buf[base + 1u] = x.y;
  topaccum_arm5_inv_buf[base + 2u] = x.z;
  topaccum_arm5_inv_buf[base + 3u] = x.w;
  return x;
}

@compute @workgroup_size(1)
fn topaccum_arm5_capture_inv_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= arrayLength(&topaccum_arm5_cycle_list)) {
    return;
  }
  topaccum_arm5_inv_item = gid.x;
  cycle = topaccum_arm5_cycle_list[gid.x];
  if (cycle >= params.data_rows) {
    return;
  }
  step_TopAccumArm5(buf_accum, buf_data, buf_global, buf_mix);
}
"#;

const TOPACCUM_ARM5_INV_CONSUME_ENTRY_TEMPLATE: &str = r#"
@group(0) @binding(5) var<storage, read> topaccum_arm5_cycle_list: array<u32>;
@group(0) @binding(6) var<storage, read_write> topaccum_arm5_inv_buf: array<u32>;

const TOPACCUM_ARM5_INV_COUNT: u32 = __INV_COUNT__;
var<private> topaccum_arm5_inv_item: u32;

fn topaccum_arm5_inv_base(slot: u32) -> u32 {
  return (topaccum_arm5_inv_item * TOPACCUM_ARM5_INV_COUNT + slot) * 4u;
}

fn topaccum_arm5_consume_inv(slot: u32, x: ExtVal) -> ExtVal {
  let base = topaccum_arm5_inv_base(slot);
  _ = x;
  return ExtVal(
    topaccum_arm5_inv_buf[base],
    topaccum_arm5_inv_buf[base + 1u],
    topaccum_arm5_inv_buf[base + 2u],
    topaccum_arm5_inv_buf[base + 3u],
  );
}

fn topaccum_arm5_apply_terminal_prefix() {
  if (cycle == 0u) {
    return;
  }

  let rows = params.accum_rows;
  let prev = cycle - 1u;
  let cur = cycle;
  let terminal_col = 99u;
  accum_buf[(terminal_col + 0u) * rows + cur] = add(
    accum_buf[(terminal_col + 0u) * rows + cur],
    accum_buf[(terminal_col + 0u) * rows + prev],
  );
  accum_buf[(terminal_col + 1u) * rows + cur] = add(
    accum_buf[(terminal_col + 1u) * rows + cur],
    accum_buf[(terminal_col + 1u) * rows + prev],
  );
  accum_buf[(terminal_col + 2u) * rows + cur] = add(
    accum_buf[(terminal_col + 2u) * rows + cur],
    accum_buf[(terminal_col + 2u) * rows + prev],
  );
  accum_buf[(terminal_col + 3u) * rows + cur] = add(
    accum_buf[(terminal_col + 3u) * rows + cur],
    accum_buf[(terminal_col + 3u) * rows + prev],
  );
}

fn topaccum_arm5_consume_inv_row(item: u32) {
  topaccum_arm5_inv_item = item;
  cycle = topaccum_arm5_cycle_list[item];
  if (cycle >= params.data_rows) {
    return;
  }
  step_TopAccumArm5(buf_accum, buf_data, buf_global, buf_mix);
}

@compute @workgroup_size(1)
fn topaccum_arm5_consume_inv_raw_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= arrayLength(&topaccum_arm5_cycle_list)) {
    return;
  }
  topaccum_arm5_consume_inv_row(gid.x);
}

@compute @workgroup_size(1)
fn topaccum_arm5_consume_inv_prefixed_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= arrayLength(&topaccum_arm5_cycle_list)) {
    return;
  }
  topaccum_arm5_consume_inv_row(gid.x);
  topaccum_arm5_apply_terminal_prefix();
}
"#;

const TOPACCUM_ARM5_INV_BUFFER_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const MONT_ONE: u32 = 268435454u;

alias Val = u32;
alias ExtVal = vec4<u32>;

struct InvParams {
  items: u32,
  _pad0: u32,
  _pad1: u32,
  _pad2: u32,
}

@group(0) @binding(0) var<storage, read_write> inv_buf: array<u32>;
@group(0) @binding(1) var<uniform> params: InvParams;

fn add(lhs: Val, rhs: Val) -> Val {
  let sum = lhs + rhs;
  if (sum >= P) {
    return sum - P;
  }
  return sum;
}

fn sub(lhs: Val, rhs: Val) -> Val {
  if (lhs >= rhs) {
    return lhs - rhs;
  }
  return lhs + P - rhs;
}

fn mul_wide(lhs: u32, rhs: u32) -> vec2<u32> {
  let lhs_lo = lhs & 0xffffu;
  let lhs_hi = lhs >> 16u;
  let rhs_lo = rhs & 0xffffu;
  let rhs_hi = rhs >> 16u;
  let p0 = lhs_lo * rhs_lo;
  let p1 = lhs_hi * rhs_lo;
  let p2 = lhs_lo * rhs_hi;
  let p3 = lhs_hi * rhs_hi;
  let carry = (p0 >> 16u) + (p1 & 0xffffu) + (p2 & 0xffffu);
  let lo = (p0 & 0xffffu) | ((carry & 0xffffu) << 16u);
  let hi = p3 + (p1 >> 16u) + (p2 >> 16u) + (carry >> 16u);
  return vec2<u32>(lo, hi);
}

fn mul(lhs: Val, rhs: Val) -> Val {
  let product = mul_wide(lhs, rhs);
  let low = 0u - product.x;
  let red = M * low;
  let red_product = mul_wide(red, P);
  var ret = product.y + red_product.y;
  if (product.x + red_product.x < product.x) {
    ret = ret + 1u;
  }
  if (ret >= P) {
    return ret - P;
  }
  return ret;
}

fn pow(base: Val, exp: u32) -> Val {
  var result: Val = MONT_ONE;
  var b: Val = base;
  var e: u32 = exp;
  while (e != 0u) {
    if ((e & 1u) != 0u) {
      result = mul(result, b);
    }
    b = mul(b, b);
    e = e >> 1u;
  }
  return result;
}

fn inv(x: Val) -> Val {
  return pow(x, P - 2u);
}

fn ext_inv(x: ExtVal) -> ExtVal {
  let beta = sub(0u, NBETA);
  var b0 = add(mul(x.x, x.x), mul(beta, sub(mul(x.y, add(x.w, x.w)), mul(x.z, x.z))));
  var b2 = add(sub(mul(x.x, add(x.z, x.z)), mul(x.y, x.y)), mul(beta, mul(x.w, x.w)));
  let c = add(mul(b0, b0), mul(beta, mul(b2, b2)));
  let ic = inv(c);
  b0 = mul(b0, ic);
  b2 = mul(b2, ic);
  return ExtVal(
    add(mul(x.x, b0), mul(beta, mul(x.z, b2))),
    add(sub(0u, mul(x.y, b0)), mul(NBETA, mul(x.w, b2))),
    add(sub(0u, mul(x.x, b2)), mul(x.z, b0)),
    sub(mul(x.y, b2), mul(x.w, b0)),
  );
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= params.items) {
    return;
  }
  let base = gid.x * 4u;
  let x = ExtVal(inv_buf[base], inv_buf[base + 1u], inv_buf[base + 2u], inv_buf[base + 3u]);
  let y = ext_inv(x);
  inv_buf[base] = y.x;
  inv_buf[base + 1u] = y.y;
  inv_buf[base + 2u] = y.z;
  inv_buf[base + 3u] = y.w;
}
"#;

fn write_arg_u16_const(wgsl: &mut String, name: &str, arg: &'static ArgU16Layout) {
    writeln!(
        wgsl,
        "const {name}_COUNT: u32 = {}u;",
        arg.count._super.offset
    )
    .unwrap();
    writeln!(wgsl, "const {name}_VAL: u32 = {}u;", arg.val._super.offset).unwrap();
}

fn write_arg_u8_const(wgsl: &mut String, name: &str, arg: &'static ArgU8Layout) {
    writeln!(
        wgsl,
        "const {name}_COUNT: u32 = {}u;",
        arg.count._super.offset
    )
    .unwrap();
    writeln!(wgsl, "const {name}_VAL: u32 = {}u;", arg.val._super.offset).unwrap();
}

fn write_memory_arg_const(wgsl: &mut String, name: &str, arg: &'static MemoryArgLayout) {
    writeln!(
        wgsl,
        "const {name}_COUNT: u32 = {}u;",
        arg.count._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const {name}_ADDR: u32 = {}u;",
        arg.addr._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const {name}_CYCLE: u32 = {}u;",
        arg.cycle._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const {name}_DATA_LOW: u32 = {}u;",
        arg.data_low._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const {name}_DATA_HIGH: u32 = {}u;",
        arg.data_high._super.offset
    )
    .unwrap();
}

fn write_cycle_arg_const(wgsl: &mut String, name: &str, arg: &'static CycleArgLayout) {
    writeln!(
        wgsl,
        "const {name}_COUNT: u32 = {}u;",
        arg.count._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const {name}_CYCLE: u32 = {}u;",
        arg.cycle._super.offset
    )
    .unwrap();
}

fn accum_misc0_direct_wgsl() -> String {
    let misc0 = LAYOUT_TOP.inst_result.arm0;
    accum_misc_direct_wgsl(
        misc0._super,
        misc0._0,
        misc0.input,
        misc0._arguments_misc0_misc_output.arg_u16,
    )
}

fn accum_misc1_direct_wgsl() -> String {
    let misc1 = LAYOUT_TOP.inst_result.arm1;
    accum_misc_direct_wgsl(
        misc1._super,
        misc1._0,
        misc1.input,
        misc1._arguments_misc1_misc_output.arg_u16,
    )
}

fn accum_misc2_direct_wgsl() -> String {
    let misc2 = LAYOUT_TOP.inst_result.arm2;
    accum_misc_direct_wgsl(
        misc2._super,
        misc2._0,
        misc2.input,
        misc2._arguments_misc2_misc_output.arg_u16,
    )
}

fn accum_mem0_direct_wgsl() -> String {
    let mem0 = LAYOUT_TOP.inst_result.arm5;
    accum_mem0_direct_wgsl_for_layout(mem0)
}

fn accum_mem1_direct_wgsl() -> String {
    let mem1 = LAYOUT_TOP.inst_result.arm6;
    accum_mem1_direct_wgsl_for_layout(mem1)
}

fn accum_mem0_direct_wgsl_for_layout(mem0: &'static Mem0Layout) -> String {
    let input = mem0.input;
    let decoded = input.decoded;
    let output_args = mem0._arguments_mem0_output;
    let write_rd = mem0._1;
    let user = LAYOUT_TOP_ACCUM.user._0;
    let randomness = LAYOUT_MIX.randomness;

    let mut wgsl = String::with_capacity(24_000);
    writeln!(
        wgsl,
        "const MIX_ARG_U8_VAL: u32 = {}u;",
        randomness.arg_u8.val.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_ARG_U16_VAL: u32 = {}u;",
        randomness.arg_u16.val.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_OFFSET: u32 = {}u;",
        randomness._offset.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_ADDR: u32 = {}u;",
        randomness.memory_arg.addr.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_CYCLE: u32 = {}u;",
        randomness.memory_arg.cycle.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_DATA_LOW: u32 = {}u;",
        randomness.memory_arg.data_low.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_DATA_HIGH: u32 = {}u;",
        randomness.memory_arg.data_high.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_CYCLE: u32 = {}u;",
        randomness.cycle_arg.cycle.offset
    )
    .unwrap();

    write_cycle_arg_const(&mut wgsl, "MEM0_ARG1", mem0._0.arg1);
    write_cycle_arg_const(&mut wgsl, "MEM0_ARG2", mem0._0.arg2);
    write_arg_u16_const(
        &mut wgsl,
        "DECODED_PC_UPPER_DIFF",
        decoded.pc_addr.upper_diff.arg,
    );
    write_arg_u16_const(&mut wgsl, "DECODED_PC_MED14", decoded.pc_addr.med14.arg);
    write_memory_arg_const(&mut wgsl, "DECODED_LOAD_OLD", decoded.load_inst.io.old_txn);
    write_memory_arg_const(&mut wgsl, "DECODED_LOAD_NEW", decoded.load_inst.io.new_txn);
    write_cycle_arg_const(&mut wgsl, "DECODED_LOAD_CYCLE", decoded.load_inst._0._0.arg);
    write_memory_arg_const(&mut wgsl, "RS1_OLD", input.rs1._super.io.old_txn);
    write_memory_arg_const(&mut wgsl, "RS1_NEW", input.rs1._super.io.new_txn);
    write_cycle_arg_const(&mut wgsl, "RS1_CYCLE", input.rs1._super._0._0.arg);
    write_arg_u16_const(&mut wgsl, "ADDR_U32_LOW", input.addr_u32.low16.arg);
    write_arg_u16_const(&mut wgsl, "ADDR_U32_HIGH", input.addr_u32.high16.arg);
    write_arg_u16_const(&mut wgsl, "ADDR_BITS_UPPER_DIFF", input.addr.upper_diff.arg);
    write_arg_u16_const(&mut wgsl, "ADDR_BITS_MED14", input.addr.med14.arg);
    write_memory_arg_const(&mut wgsl, "DATA0_OLD", input.data_0.io.old_txn);
    write_memory_arg_const(&mut wgsl, "DATA0_NEW", input.data_0.io.new_txn);
    write_cycle_arg_const(&mut wgsl, "DATA0_CYCLE", input.data_0._0._0.arg);
    for (idx, arg) in output_args.arg_u8.iter().enumerate() {
        write_arg_u8_const(&mut wgsl, &format!("OUT_U8_{idx}"), arg);
    }
    write_arg_u16_const(&mut wgsl, "OUT_U16_0", output_args.arg_u16[0]);
    write_memory_arg_const(&mut wgsl, "WRITE_OLD", write_rd._0.io.old_txn);
    write_memory_arg_const(&mut wgsl, "WRITE_NEW", write_rd._0.io.new_txn);
    write_cycle_arg_const(&mut wgsl, "WRITE_CYCLE", write_rd._0._0._0.arg);
    write_arg_u16_const(&mut wgsl, "PC_ADD_LOW", mem0.pc_add.low16.arg);
    write_arg_u16_const(&mut wgsl, "PC_ADD_HIGH", mem0.pc_add.high16.arg);

    writeln!(
        wgsl,
        "const ACC_USER_POLY: u32 = {}u;",
        user.state.poly._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const ACC_USER_TERM: u32 = {}u;",
        user.state.term._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const ACC_USER_TOTAL: u32 = {}u;",
        user.state.total._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const ACC_USER_TMP: u32 = {}u;",
        user.state_redef.arm3.tmp._super.offset
    )
    .unwrap();
    for (idx, bit) in user.poly_op._super.iter().enumerate() {
        writeln!(
            wgsl,
            "const ACC_USER_POLY_OP{idx}: u32 = {}u;",
            bit._super.offset
        )
        .unwrap();
    }
    for col in 0..9 {
        writeln!(
            wgsl,
            "const ACC_COL{col}: u32 = {}u;",
            LAYOUT_TOP_ACCUM.columns[col].offset
        )
        .unwrap();
    }
    writeln!(
        wgsl,
        "const ACC_COL19: u32 = {}u;",
        LAYOUT_TOP_ACCUM.columns[19].offset
    )
    .unwrap();

    wgsl.push_str(
        r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const MONT_ONE: u32 = 268435454u;

alias Val = u32;
alias ExtVal = vec4<u32>;

struct Params {
  data_rows: u32,
  accum_rows: u32,
  row_count: u32,
  data_base: u32,
  accum_base: u32,
  mix_base: u32,
  _pad0: u32,
  _pad1: u32,
}

@group(0) @binding(0) var<storage, read> data: array<u32>;
@group(0) @binding(1) var<storage, read_write> accum: array<u32>;
@group(0) @binding(2) var<storage, read> mix: array<u32>;
@group(0) @binding(3) var<storage, read> rows: array<u32>;
@group(0) @binding(4) var<uniform> params: Params;

fn add(lhs: Val, rhs: Val) -> Val {
  let sum = lhs + rhs;
  if (sum >= P) {
    return sum - P;
  }
  return sum;
}

fn sub(lhs: Val, rhs: Val) -> Val {
  if (lhs >= rhs) {
    return lhs - rhs;
  }
  return lhs + P - rhs;
}

fn mul_wide(lhs: u32, rhs: u32) -> vec2<u32> {
  let lhs_lo = lhs & 0xffffu;
  let lhs_hi = lhs >> 16u;
  let rhs_lo = rhs & 0xffffu;
  let rhs_hi = rhs >> 16u;
  let p0 = lhs_lo * rhs_lo;
  let p1 = lhs_hi * rhs_lo;
  let p2 = lhs_lo * rhs_hi;
  let p3 = lhs_hi * rhs_hi;
  let carry = (p0 >> 16u) + (p1 & 0xffffu) + (p2 & 0xffffu);
  let lo = (p0 & 0xffffu) | ((carry & 0xffffu) << 16u);
  let hi = p3 + (p1 >> 16u) + (p2 >> 16u) + (carry >> 16u);
  return vec2<u32>(lo, hi);
}

fn mul(lhs: Val, rhs: Val) -> Val {
  let product = mul_wide(lhs, rhs);
  let low = 0u - product.x;
  let red = M * low;
  let red_product = mul_wide(red, P);
  var ret = product.y + red_product.y;
  if (product.x + red_product.x < product.x) {
    ret = ret + 1u;
  }
  if (ret >= P) {
    return ret - P;
  }
  return ret;
}

fn pow(base: Val, exp: u32) -> Val {
  var result: Val = MONT_ONE;
  var b: Val = base;
  var e: u32 = exp;
  while (e != 0u) {
    if ((e & 1u) != 0u) {
      result = mul(result, b);
    }
    b = mul(b, b);
    e = e >> 1u;
  }
  return result;
}

fn inv(x: Val) -> Val {
  return pow(x, P - 2u);
}

fn ext_add(lhs: ExtVal, rhs: ExtVal) -> ExtVal {
  return ExtVal(
    add(lhs.x, rhs.x),
    add(lhs.y, rhs.y),
    add(lhs.z, rhs.z),
    add(lhs.w, rhs.w),
  );
}

fn ext_scale(lhs: ExtVal, rhs: Val) -> ExtVal {
  return ExtVal(
    mul(lhs.x, rhs),
    mul(lhs.y, rhs),
    mul(lhs.z, rhs),
    mul(lhs.w, rhs),
  );
}

fn ext_inv(x: ExtVal) -> ExtVal {
  let beta = sub(0u, NBETA);
  var b0 = add(mul(x.x, x.x), mul(beta, sub(mul(x.y, add(x.w, x.w)), mul(x.z, x.z))));
  var b2 = add(sub(mul(x.x, add(x.z, x.z)), mul(x.y, x.y)), mul(beta, mul(x.w, x.w)));
  let c = add(mul(b0, b0), mul(beta, mul(b2, b2)));
  let ic = inv(c);
  b0 = mul(b0, ic);
  b2 = mul(b2, ic);
  return ExtVal(
    add(mul(x.x, b0), mul(beta, mul(x.z, b2))),
    add(sub(0u, mul(x.y, b0)), mul(NBETA, mul(x.w, b2))),
    add(sub(0u, mul(x.x, b2)), mul(x.z, b0)),
    sub(mul(x.y, b2), mul(x.w, b0)),
  );
}

fn data_at(row: u32, col: u32) -> Val {
  return data[params.data_base + col * params.data_rows + row];
}

fn mix_ext(offset: u32) -> ExtVal {
  let base = params.mix_base + offset;
  return ExtVal(mix[base], mix[base + 1u], mix[base + 2u], mix[base + 3u]);
}

fn store_val(row: u32, col: u32, value: Val) {
  accum[params.accum_base + col * params.accum_rows + row] = value;
}

fn store_ext(row: u32, col: u32, value: ExtVal) {
  store_val(row, col, value.x);
  store_val(row, col + 1u, value.y);
  store_val(row, col + 2u, value.z);
  store_val(row, col + 3u, value.w);
}

fn arg_u8_term(row: u32, count_col: u32, val_col: u32) -> ExtVal {
  let count = data_at(row, count_col);
  let value = data_at(row, val_col);
  let denom = ext_add(ext_scale(mix_ext(MIX_ARG_U8_VAL), value), mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), count);
}

fn arg_u16_term(row: u32, count_col: u32, val_col: u32) -> ExtVal {
  let count = data_at(row, count_col);
  let value = data_at(row, val_col);
  let denom = ext_add(ext_scale(mix_ext(MIX_ARG_U16_VAL), value), mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), count);
}

fn memory_term(
  row: u32,
  count_col: u32,
  addr_col: u32,
  cycle_col: u32,
  data_low_col: u32,
  data_high_col: u32,
) -> ExtVal {
  var denom = ext_scale(mix_ext(MIX_MEMORY_ADDR), data_at(row, addr_col));
  denom = ext_add(denom, ext_scale(mix_ext(MIX_MEMORY_CYCLE), data_at(row, cycle_col)));
  denom = ext_add(denom, ext_scale(mix_ext(MIX_MEMORY_DATA_LOW), data_at(row, data_low_col)));
  denom = ext_add(denom, ext_scale(mix_ext(MIX_MEMORY_DATA_HIGH), data_at(row, data_high_col)));
  denom = ext_add(denom, mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), data_at(row, count_col));
}

fn cycle_term(row: u32, count_col: u32, cycle_col: u32) -> ExtVal {
  var denom = ext_scale(mix_ext(MIX_CYCLE), data_at(row, cycle_col));
  denom = ext_add(denom, mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), data_at(row, count_col));
}

fn store_user_accum(row: u32) {
  store_ext(row, ACC_USER_POLY, ExtVal(0u, 0u, 0u, 0u));
  store_ext(row, ACC_USER_TERM, ExtVal(MONT_ONE, 0u, 0u, 0u));
  store_ext(row, ACC_USER_TOTAL, ExtVal(0u, 0u, 0u, 0u));
  store_val(row, ACC_USER_POLY_OP0, MONT_ONE);
  store_val(row, ACC_USER_POLY_OP1, 0u);
  store_val(row, ACC_USER_POLY_OP2, 0u);
  store_val(row, ACC_USER_POLY_OP3, 0u);
  store_val(row, ACC_USER_POLY_OP4, 0u);
  store_val(row, ACC_USER_POLY_OP5, 0u);
  store_val(row, ACC_USER_POLY_OP6, 0u);
  store_ext(row, ACC_USER_TMP, ExtVal(0u, 0u, 0u, 0u));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= params.row_count) {
    return;
  }
  let row = rows[gid.x];
  store_user_accum(row);

  var cur = ExtVal(0u, 0u, 0u, 0u);

  cur = ext_add(cur, cycle_term(row, MEM0_ARG1_COUNT, MEM0_ARG1_CYCLE));
  cur = ext_add(cur, cycle_term(row, MEM0_ARG2_COUNT, MEM0_ARG2_CYCLE));
  cur = ext_add(cur, arg_u16_term(row, DECODED_PC_UPPER_DIFF_COUNT, DECODED_PC_UPPER_DIFF_VAL));
  store_ext(row, ACC_COL0, cur);

  cur = ext_add(cur, arg_u16_term(row, DECODED_PC_MED14_COUNT, DECODED_PC_MED14_VAL));
  cur = ext_add(cur, memory_term(row, DECODED_LOAD_OLD_COUNT, DECODED_LOAD_OLD_ADDR, DECODED_LOAD_OLD_CYCLE, DECODED_LOAD_OLD_DATA_LOW, DECODED_LOAD_OLD_DATA_HIGH));
  cur = ext_add(cur, memory_term(row, DECODED_LOAD_NEW_COUNT, DECODED_LOAD_NEW_ADDR, DECODED_LOAD_NEW_CYCLE, DECODED_LOAD_NEW_DATA_LOW, DECODED_LOAD_NEW_DATA_HIGH));
  store_ext(row, ACC_COL1, cur);

  cur = ext_add(cur, cycle_term(row, DECODED_LOAD_CYCLE_COUNT, DECODED_LOAD_CYCLE_CYCLE));
  cur = ext_add(cur, memory_term(row, RS1_OLD_COUNT, RS1_OLD_ADDR, RS1_OLD_CYCLE, RS1_OLD_DATA_LOW, RS1_OLD_DATA_HIGH));
  cur = ext_add(cur, memory_term(row, RS1_NEW_COUNT, RS1_NEW_ADDR, RS1_NEW_CYCLE, RS1_NEW_DATA_LOW, RS1_NEW_DATA_HIGH));
  store_ext(row, ACC_COL2, cur);

  cur = ext_add(cur, cycle_term(row, RS1_CYCLE_COUNT, RS1_CYCLE_CYCLE));
  cur = ext_add(cur, arg_u16_term(row, ADDR_U32_LOW_COUNT, ADDR_U32_LOW_VAL));
  cur = ext_add(cur, arg_u16_term(row, ADDR_U32_HIGH_COUNT, ADDR_U32_HIGH_VAL));
  store_ext(row, ACC_COL3, cur);

  cur = ext_add(cur, arg_u16_term(row, ADDR_BITS_UPPER_DIFF_COUNT, ADDR_BITS_UPPER_DIFF_VAL));
  cur = ext_add(cur, arg_u16_term(row, ADDR_BITS_MED14_COUNT, ADDR_BITS_MED14_VAL));
  cur = ext_add(cur, memory_term(row, DATA0_OLD_COUNT, DATA0_OLD_ADDR, DATA0_OLD_CYCLE, DATA0_OLD_DATA_LOW, DATA0_OLD_DATA_HIGH));
  store_ext(row, ACC_COL4, cur);

  cur = ext_add(cur, memory_term(row, DATA0_NEW_COUNT, DATA0_NEW_ADDR, DATA0_NEW_CYCLE, DATA0_NEW_DATA_LOW, DATA0_NEW_DATA_HIGH));
  cur = ext_add(cur, cycle_term(row, DATA0_CYCLE_COUNT, DATA0_CYCLE_CYCLE));
  cur = ext_add(cur, arg_u8_term(row, OUT_U8_0_COUNT, OUT_U8_0_VAL));
  store_ext(row, ACC_COL5, cur);

  cur = ext_add(cur, arg_u8_term(row, OUT_U8_1_COUNT, OUT_U8_1_VAL));
  cur = ext_add(cur, arg_u8_term(row, OUT_U8_2_COUNT, OUT_U8_2_VAL));
  cur = ext_add(cur, arg_u16_term(row, OUT_U16_0_COUNT, OUT_U16_0_VAL));
  store_ext(row, ACC_COL6, cur);

  cur = ext_add(cur, memory_term(row, WRITE_OLD_COUNT, WRITE_OLD_ADDR, WRITE_OLD_CYCLE, WRITE_OLD_DATA_LOW, WRITE_OLD_DATA_HIGH));
  cur = ext_add(cur, memory_term(row, WRITE_NEW_COUNT, WRITE_NEW_ADDR, WRITE_NEW_CYCLE, WRITE_NEW_DATA_LOW, WRITE_NEW_DATA_HIGH));
  cur = ext_add(cur, cycle_term(row, WRITE_CYCLE_COUNT, WRITE_CYCLE_CYCLE));
  store_ext(row, ACC_COL7, cur);

  cur = ext_add(cur, arg_u16_term(row, PC_ADD_LOW_COUNT, PC_ADD_LOW_VAL));
  cur = ext_add(cur, arg_u16_term(row, PC_ADD_HIGH_COUNT, PC_ADD_HIGH_VAL));
  store_ext(row, ACC_COL8, cur);
  store_ext(row, ACC_COL19, cur);
}
"#,
    );
    wgsl
}

fn accum_mem1_direct_wgsl_for_layout(mem1: &'static Mem1Layout) -> String {
    let input = mem1.input;
    let decoded = input.decoded;
    let source_args = input.source_regs._arguments_read_source_regs_source_regs;
    let output_args = mem1._arguments_mem1_output;
    let write_mem = mem1._1;
    let user = LAYOUT_TOP_ACCUM.user._0;
    let randomness = LAYOUT_MIX.randomness;

    let mut wgsl = String::with_capacity(26_000);
    writeln!(
        wgsl,
        "const MIX_ARG_U8_VAL: u32 = {}u;",
        randomness.arg_u8.val.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_ARG_U16_VAL: u32 = {}u;",
        randomness.arg_u16.val.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_OFFSET: u32 = {}u;",
        randomness._offset.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_ADDR: u32 = {}u;",
        randomness.memory_arg.addr.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_CYCLE: u32 = {}u;",
        randomness.memory_arg.cycle.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_DATA_LOW: u32 = {}u;",
        randomness.memory_arg.data_low.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_DATA_HIGH: u32 = {}u;",
        randomness.memory_arg.data_high.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_CYCLE: u32 = {}u;",
        randomness.cycle_arg.cycle.offset
    )
    .unwrap();

    write_cycle_arg_const(&mut wgsl, "MEM1_ARG1", mem1._0.arg1);
    write_cycle_arg_const(&mut wgsl, "MEM1_ARG2", mem1._0.arg2);
    write_arg_u16_const(
        &mut wgsl,
        "DECODED_PC_UPPER_DIFF",
        decoded.pc_addr.upper_diff.arg,
    );
    write_arg_u16_const(&mut wgsl, "DECODED_PC_MED14", decoded.pc_addr.med14.arg);
    write_memory_arg_const(&mut wgsl, "DECODED_LOAD_OLD", decoded.load_inst.io.old_txn);
    write_memory_arg_const(&mut wgsl, "DECODED_LOAD_NEW", decoded.load_inst.io.new_txn);
    write_cycle_arg_const(&mut wgsl, "DECODED_LOAD_CYCLE", decoded.load_inst._0._0.arg);
    for (idx, arg) in source_args.memory_arg.iter().enumerate() {
        write_memory_arg_const(&mut wgsl, &format!("SRC_MEM{idx}"), arg);
    }
    for (idx, arg) in source_args.cycle_arg.iter().enumerate() {
        write_cycle_arg_const(&mut wgsl, &format!("SRC_CYCLE{idx}"), arg);
    }
    write_arg_u16_const(&mut wgsl, "ADDR_U32_LOW", input.addr_u32.low16.arg);
    write_arg_u16_const(&mut wgsl, "ADDR_U32_HIGH", input.addr_u32.high16.arg);
    write_arg_u16_const(&mut wgsl, "ADDR_BITS_UPPER_DIFF", input.addr.upper_diff.arg);
    write_arg_u16_const(&mut wgsl, "ADDR_BITS_MED14", input.addr.med14.arg);
    write_memory_arg_const(&mut wgsl, "DATA0_OLD", input.data_0.io.old_txn);
    write_memory_arg_const(&mut wgsl, "DATA0_NEW", input.data_0.io.new_txn);
    write_cycle_arg_const(&mut wgsl, "DATA0_CYCLE", input.data_0._0._0.arg);
    for (idx, arg) in output_args.arg_u8.iter().enumerate() {
        write_arg_u8_const(&mut wgsl, &format!("OUT_U8_{idx}"), arg);
    }
    write_memory_arg_const(&mut wgsl, "WRITE_OLD", write_mem._0.io.old_txn);
    write_memory_arg_const(&mut wgsl, "WRITE_NEW", write_mem._0.io.new_txn);
    write_cycle_arg_const(&mut wgsl, "WRITE_CYCLE", write_mem._0._0._0.arg);
    write_arg_u16_const(&mut wgsl, "PC_ADD_LOW", mem1.pc_add.low16.arg);
    write_arg_u16_const(&mut wgsl, "PC_ADD_HIGH", mem1.pc_add.high16.arg);

    writeln!(
        wgsl,
        "const ACC_USER_POLY: u32 = {}u;",
        user.state.poly._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const ACC_USER_TERM: u32 = {}u;",
        user.state.term._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const ACC_USER_TOTAL: u32 = {}u;",
        user.state.total._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const ACC_USER_TMP: u32 = {}u;",
        user.state_redef.arm3.tmp._super.offset
    )
    .unwrap();
    for (idx, bit) in user.poly_op._super.iter().enumerate() {
        writeln!(
            wgsl,
            "const ACC_USER_POLY_OP{idx}: u32 = {}u;",
            bit._super.offset
        )
        .unwrap();
    }
    for col in 0..10 {
        writeln!(
            wgsl,
            "const ACC_COL{col}: u32 = {}u;",
            LAYOUT_TOP_ACCUM.columns[col].offset
        )
        .unwrap();
    }
    writeln!(
        wgsl,
        "const ACC_COL19: u32 = {}u;",
        LAYOUT_TOP_ACCUM.columns[19].offset
    )
    .unwrap();

    wgsl.push_str(
        r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const MONT_ONE: u32 = 268435454u;

alias Val = u32;
alias ExtVal = vec4<u32>;

struct Params {
  data_rows: u32,
  accum_rows: u32,
  row_count: u32,
  data_base: u32,
  accum_base: u32,
  mix_base: u32,
  _pad0: u32,
  _pad1: u32,
}

@group(0) @binding(0) var<storage, read> data: array<u32>;
@group(0) @binding(1) var<storage, read_write> accum: array<u32>;
@group(0) @binding(2) var<storage, read> mix: array<u32>;
@group(0) @binding(3) var<storage, read> rows: array<u32>;
@group(0) @binding(4) var<uniform> params: Params;

fn add(lhs: Val, rhs: Val) -> Val {
  let sum = lhs + rhs;
  if (sum >= P) {
    return sum - P;
  }
  return sum;
}

fn sub(lhs: Val, rhs: Val) -> Val {
  if (lhs >= rhs) {
    return lhs - rhs;
  }
  return lhs + P - rhs;
}

fn mul_wide(lhs: u32, rhs: u32) -> vec2<u32> {
  let lhs_lo = lhs & 0xffffu;
  let lhs_hi = lhs >> 16u;
  let rhs_lo = rhs & 0xffffu;
  let rhs_hi = rhs >> 16u;
  let p0 = lhs_lo * rhs_lo;
  let p1 = lhs_hi * rhs_lo;
  let p2 = lhs_lo * rhs_hi;
  let p3 = lhs_hi * rhs_hi;
  let carry = (p0 >> 16u) + (p1 & 0xffffu) + (p2 & 0xffffu);
  let lo = (p0 & 0xffffu) | ((carry & 0xffffu) << 16u);
  let hi = p3 + (p1 >> 16u) + (p2 >> 16u) + (carry >> 16u);
  return vec2<u32>(lo, hi);
}

fn mul(lhs: Val, rhs: Val) -> Val {
  let product = mul_wide(lhs, rhs);
  let low = 0u - product.x;
  let red = M * low;
  let red_product = mul_wide(red, P);
  var ret = product.y + red_product.y;
  if (product.x + red_product.x < product.x) {
    ret = ret + 1u;
  }
  if (ret >= P) {
    return ret - P;
  }
  return ret;
}

fn pow(base: Val, exp: u32) -> Val {
  var result: Val = MONT_ONE;
  var b: Val = base;
  var e: u32 = exp;
  while (e != 0u) {
    if ((e & 1u) != 0u) {
      result = mul(result, b);
    }
    b = mul(b, b);
    e = e >> 1u;
  }
  return result;
}

fn inv(x: Val) -> Val {
  return pow(x, P - 2u);
}

fn ext_add(lhs: ExtVal, rhs: ExtVal) -> ExtVal {
  return ExtVal(
    add(lhs.x, rhs.x),
    add(lhs.y, rhs.y),
    add(lhs.z, rhs.z),
    add(lhs.w, rhs.w),
  );
}

fn ext_scale(lhs: ExtVal, rhs: Val) -> ExtVal {
  return ExtVal(
    mul(lhs.x, rhs),
    mul(lhs.y, rhs),
    mul(lhs.z, rhs),
    mul(lhs.w, rhs),
  );
}

fn ext_inv(x: ExtVal) -> ExtVal {
  let beta = sub(0u, NBETA);
  var b0 = add(mul(x.x, x.x), mul(beta, sub(mul(x.y, add(x.w, x.w)), mul(x.z, x.z))));
  var b2 = add(sub(mul(x.x, add(x.z, x.z)), mul(x.y, x.y)), mul(beta, mul(x.w, x.w)));
  let c = add(mul(b0, b0), mul(beta, mul(b2, b2)));
  let ic = inv(c);
  b0 = mul(b0, ic);
  b2 = mul(b2, ic);
  return ExtVal(
    add(mul(x.x, b0), mul(beta, mul(x.z, b2))),
    add(sub(0u, mul(x.y, b0)), mul(NBETA, mul(x.w, b2))),
    add(sub(0u, mul(x.x, b2)), mul(x.z, b0)),
    sub(mul(x.y, b2), mul(x.w, b0)),
  );
}

fn data_at(row: u32, col: u32) -> Val {
  return data[params.data_base + col * params.data_rows + row];
}

fn mix_ext(offset: u32) -> ExtVal {
  let base = params.mix_base + offset;
  return ExtVal(mix[base], mix[base + 1u], mix[base + 2u], mix[base + 3u]);
}

fn store_val(row: u32, col: u32, value: Val) {
  accum[params.accum_base + col * params.accum_rows + row] = value;
}

fn store_ext(row: u32, col: u32, value: ExtVal) {
  store_val(row, col, value.x);
  store_val(row, col + 1u, value.y);
  store_val(row, col + 2u, value.z);
  store_val(row, col + 3u, value.w);
}

fn arg_u8_term(row: u32, count_col: u32, val_col: u32) -> ExtVal {
  let count = data_at(row, count_col);
  let value = data_at(row, val_col);
  let denom = ext_add(ext_scale(mix_ext(MIX_ARG_U8_VAL), value), mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), count);
}

fn arg_u16_term(row: u32, count_col: u32, val_col: u32) -> ExtVal {
  let count = data_at(row, count_col);
  let value = data_at(row, val_col);
  let denom = ext_add(ext_scale(mix_ext(MIX_ARG_U16_VAL), value), mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), count);
}

fn memory_term(
  row: u32,
  count_col: u32,
  addr_col: u32,
  cycle_col: u32,
  data_low_col: u32,
  data_high_col: u32,
) -> ExtVal {
  var denom = ext_scale(mix_ext(MIX_MEMORY_ADDR), data_at(row, addr_col));
  denom = ext_add(denom, ext_scale(mix_ext(MIX_MEMORY_CYCLE), data_at(row, cycle_col)));
  denom = ext_add(denom, ext_scale(mix_ext(MIX_MEMORY_DATA_LOW), data_at(row, data_low_col)));
  denom = ext_add(denom, ext_scale(mix_ext(MIX_MEMORY_DATA_HIGH), data_at(row, data_high_col)));
  denom = ext_add(denom, mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), data_at(row, count_col));
}

fn cycle_term(row: u32, count_col: u32, cycle_col: u32) -> ExtVal {
  var denom = ext_scale(mix_ext(MIX_CYCLE), data_at(row, cycle_col));
  denom = ext_add(denom, mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), data_at(row, count_col));
}

fn store_user_accum(row: u32) {
  store_ext(row, ACC_USER_POLY, ExtVal(0u, 0u, 0u, 0u));
  store_ext(row, ACC_USER_TERM, ExtVal(MONT_ONE, 0u, 0u, 0u));
  store_ext(row, ACC_USER_TOTAL, ExtVal(0u, 0u, 0u, 0u));
  store_val(row, ACC_USER_POLY_OP0, MONT_ONE);
  store_val(row, ACC_USER_POLY_OP1, 0u);
  store_val(row, ACC_USER_POLY_OP2, 0u);
  store_val(row, ACC_USER_POLY_OP3, 0u);
  store_val(row, ACC_USER_POLY_OP4, 0u);
  store_val(row, ACC_USER_POLY_OP5, 0u);
  store_val(row, ACC_USER_POLY_OP6, 0u);
  store_ext(row, ACC_USER_TMP, ExtVal(0u, 0u, 0u, 0u));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= params.row_count) {
    return;
  }
  let row = rows[gid.x];
  store_user_accum(row);

  var cur = ExtVal(0u, 0u, 0u, 0u);

  cur = ext_add(cur, cycle_term(row, MEM1_ARG1_COUNT, MEM1_ARG1_CYCLE));
  cur = ext_add(cur, cycle_term(row, MEM1_ARG2_COUNT, MEM1_ARG2_CYCLE));
  cur = ext_add(cur, arg_u16_term(row, DECODED_PC_UPPER_DIFF_COUNT, DECODED_PC_UPPER_DIFF_VAL));
  store_ext(row, ACC_COL0, cur);

  cur = ext_add(cur, arg_u16_term(row, DECODED_PC_MED14_COUNT, DECODED_PC_MED14_VAL));
  cur = ext_add(cur, memory_term(row, DECODED_LOAD_OLD_COUNT, DECODED_LOAD_OLD_ADDR, DECODED_LOAD_OLD_CYCLE, DECODED_LOAD_OLD_DATA_LOW, DECODED_LOAD_OLD_DATA_HIGH));
  cur = ext_add(cur, memory_term(row, DECODED_LOAD_NEW_COUNT, DECODED_LOAD_NEW_ADDR, DECODED_LOAD_NEW_CYCLE, DECODED_LOAD_NEW_DATA_LOW, DECODED_LOAD_NEW_DATA_HIGH));
  store_ext(row, ACC_COL1, cur);

  cur = ext_add(cur, cycle_term(row, DECODED_LOAD_CYCLE_COUNT, DECODED_LOAD_CYCLE_CYCLE));
  cur = ext_add(cur, memory_term(row, SRC_MEM0_COUNT, SRC_MEM0_ADDR, SRC_MEM0_CYCLE, SRC_MEM0_DATA_LOW, SRC_MEM0_DATA_HIGH));
  cur = ext_add(cur, memory_term(row, SRC_MEM1_COUNT, SRC_MEM1_ADDR, SRC_MEM1_CYCLE, SRC_MEM1_DATA_LOW, SRC_MEM1_DATA_HIGH));
  store_ext(row, ACC_COL2, cur);

  cur = ext_add(cur, memory_term(row, SRC_MEM2_COUNT, SRC_MEM2_ADDR, SRC_MEM2_CYCLE, SRC_MEM2_DATA_LOW, SRC_MEM2_DATA_HIGH));
  cur = ext_add(cur, memory_term(row, SRC_MEM3_COUNT, SRC_MEM3_ADDR, SRC_MEM3_CYCLE, SRC_MEM3_DATA_LOW, SRC_MEM3_DATA_HIGH));
  cur = ext_add(cur, cycle_term(row, SRC_CYCLE0_COUNT, SRC_CYCLE0_CYCLE));
  store_ext(row, ACC_COL3, cur);

  cur = ext_add(cur, cycle_term(row, SRC_CYCLE1_COUNT, SRC_CYCLE1_CYCLE));
  cur = ext_add(cur, arg_u16_term(row, ADDR_U32_LOW_COUNT, ADDR_U32_LOW_VAL));
  cur = ext_add(cur, arg_u16_term(row, ADDR_U32_HIGH_COUNT, ADDR_U32_HIGH_VAL));
  store_ext(row, ACC_COL4, cur);

  cur = ext_add(cur, arg_u16_term(row, ADDR_BITS_UPPER_DIFF_COUNT, ADDR_BITS_UPPER_DIFF_VAL));
  cur = ext_add(cur, arg_u16_term(row, ADDR_BITS_MED14_COUNT, ADDR_BITS_MED14_VAL));
  cur = ext_add(cur, memory_term(row, DATA0_OLD_COUNT, DATA0_OLD_ADDR, DATA0_OLD_CYCLE, DATA0_OLD_DATA_LOW, DATA0_OLD_DATA_HIGH));
  store_ext(row, ACC_COL5, cur);

  cur = ext_add(cur, memory_term(row, DATA0_NEW_COUNT, DATA0_NEW_ADDR, DATA0_NEW_CYCLE, DATA0_NEW_DATA_LOW, DATA0_NEW_DATA_HIGH));
  cur = ext_add(cur, cycle_term(row, DATA0_CYCLE_COUNT, DATA0_CYCLE_CYCLE));
  cur = ext_add(cur, arg_u8_term(row, OUT_U8_0_COUNT, OUT_U8_0_VAL));
  store_ext(row, ACC_COL6, cur);

  cur = ext_add(cur, arg_u8_term(row, OUT_U8_1_COUNT, OUT_U8_1_VAL));
  cur = ext_add(cur, arg_u8_term(row, OUT_U8_2_COUNT, OUT_U8_2_VAL));
  cur = ext_add(cur, arg_u8_term(row, OUT_U8_3_COUNT, OUT_U8_3_VAL));
  store_ext(row, ACC_COL7, cur);

  cur = ext_add(cur, memory_term(row, WRITE_OLD_COUNT, WRITE_OLD_ADDR, WRITE_OLD_CYCLE, WRITE_OLD_DATA_LOW, WRITE_OLD_DATA_HIGH));
  cur = ext_add(cur, memory_term(row, WRITE_NEW_COUNT, WRITE_NEW_ADDR, WRITE_NEW_CYCLE, WRITE_NEW_DATA_LOW, WRITE_NEW_DATA_HIGH));
  cur = ext_add(cur, cycle_term(row, WRITE_CYCLE_COUNT, WRITE_CYCLE_CYCLE));
  store_ext(row, ACC_COL8, cur);

  cur = ext_add(cur, arg_u16_term(row, PC_ADD_LOW_COUNT, PC_ADD_LOW_VAL));
  cur = ext_add(cur, arg_u16_term(row, PC_ADD_HIGH_COUNT, PC_ADD_HIGH_VAL));
  store_ext(row, ACC_COL9, cur);
  store_ext(row, ACC_COL19, cur);
}
"#,
    );
    wgsl
}

fn accum_control0_direct_wgsl() -> String {
    let control0 = LAYOUT_TOP.inst_result.arm7;
    let args = control0._arguments_control0__super;
    let user = LAYOUT_TOP_ACCUM.user._0;
    let randomness = LAYOUT_MIX.randomness;

    let mut wgsl = String::with_capacity(34_000);
    writeln!(
        wgsl,
        "const MIX_ARG_U8_VAL: u32 = {}u;",
        randomness.arg_u8.val.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_ARG_U16_VAL: u32 = {}u;",
        randomness.arg_u16.val.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_OFFSET: u32 = {}u;",
        randomness._offset.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_ADDR: u32 = {}u;",
        randomness.memory_arg.addr.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_CYCLE: u32 = {}u;",
        randomness.memory_arg.cycle.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_DATA_LOW: u32 = {}u;",
        randomness.memory_arg.data_low.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_DATA_HIGH: u32 = {}u;",
        randomness.memory_arg.data_high.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_CYCLE: u32 = {}u;",
        randomness.cycle_arg.cycle.offset
    )
    .unwrap();

    write_cycle_arg_const(&mut wgsl, "CONTROL0_ARG1", control0._0.arg1);
    write_cycle_arg_const(&mut wgsl, "CONTROL0_ARG2", control0._0.arg2);
    for (idx, arg) in args.memory_arg.iter().enumerate() {
        write_memory_arg_const(&mut wgsl, &format!("CONTROL0_MEM{idx}"), arg);
    }
    for (idx, arg) in args.cycle_arg.iter().enumerate() {
        write_cycle_arg_const(&mut wgsl, &format!("CONTROL0_CYCLE{idx}"), arg);
    }
    for (idx, arg) in args.arg_u16.iter().enumerate() {
        write_arg_u16_const(&mut wgsl, &format!("CONTROL0_U16_{idx}"), arg);
    }
    for (idx, arg) in args.arg_u8.iter().enumerate() {
        write_arg_u8_const(&mut wgsl, &format!("CONTROL0_U8_{idx}"), arg);
    }

    writeln!(
        wgsl,
        "const ACC_USER_POLY: u32 = {}u;",
        user.state.poly._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const ACC_USER_TERM: u32 = {}u;",
        user.state.term._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const ACC_USER_TOTAL: u32 = {}u;",
        user.state.total._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const ACC_USER_TMP: u32 = {}u;",
        user.state_redef.arm3.tmp._super.offset
    )
    .unwrap();
    for (idx, bit) in user.poly_op._super.iter().enumerate() {
        writeln!(
            wgsl,
            "const ACC_USER_POLY_OP{idx}: u32 = {}u;",
            bit._super.offset
        )
        .unwrap();
    }
    for col in 0..20 {
        writeln!(
            wgsl,
            "const ACC_COL{col}: u32 = {}u;",
            LAYOUT_TOP_ACCUM.columns[col].offset
        )
        .unwrap();
    }

    wgsl.push_str(
        r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const MONT_ONE: u32 = 268435454u;

alias Val = u32;
alias ExtVal = vec4<u32>;

struct Params {
  data_rows: u32,
  accum_rows: u32,
  row_count: u32,
  data_base: u32,
  accum_base: u32,
  mix_base: u32,
  _pad0: u32,
  _pad1: u32,
}

@group(0) @binding(0) var<storage, read> data: array<u32>;
@group(0) @binding(1) var<storage, read_write> accum: array<u32>;
@group(0) @binding(2) var<storage, read> mix: array<u32>;
@group(0) @binding(3) var<storage, read> rows: array<u32>;
@group(0) @binding(4) var<uniform> params: Params;

fn add(lhs: Val, rhs: Val) -> Val {
  let sum = lhs + rhs;
  if (sum >= P) {
    return sum - P;
  }
  return sum;
}

fn sub(lhs: Val, rhs: Val) -> Val {
  if (lhs >= rhs) {
    return lhs - rhs;
  }
  return lhs + P - rhs;
}

fn mul_wide(lhs: u32, rhs: u32) -> vec2<u32> {
  let lhs_lo = lhs & 0xffffu;
  let lhs_hi = lhs >> 16u;
  let rhs_lo = rhs & 0xffffu;
  let rhs_hi = rhs >> 16u;
  let p0 = lhs_lo * rhs_lo;
  let p1 = lhs_hi * rhs_lo;
  let p2 = lhs_lo * rhs_hi;
  let p3 = lhs_hi * rhs_hi;
  let carry = (p0 >> 16u) + (p1 & 0xffffu) + (p2 & 0xffffu);
  let lo = (p0 & 0xffffu) | ((carry & 0xffffu) << 16u);
  let hi = p3 + (p1 >> 16u) + (p2 >> 16u) + (carry >> 16u);
  return vec2<u32>(lo, hi);
}

fn mul(lhs: Val, rhs: Val) -> Val {
  let product = mul_wide(lhs, rhs);
  let low = 0u - product.x;
  let red = M * low;
  let red_product = mul_wide(red, P);
  var ret = product.y + red_product.y;
  if (product.x + red_product.x < product.x) {
    ret = ret + 1u;
  }
  if (ret >= P) {
    return ret - P;
  }
  return ret;
}

fn pow(base: Val, exp: u32) -> Val {
  var result: Val = MONT_ONE;
  var b: Val = base;
  var e: u32 = exp;
  while (e != 0u) {
    if ((e & 1u) != 0u) {
      result = mul(result, b);
    }
    b = mul(b, b);
    e = e >> 1u;
  }
  return result;
}

fn inv(x: Val) -> Val {
  return pow(x, P - 2u);
}

fn ext_add(lhs: ExtVal, rhs: ExtVal) -> ExtVal {
  return ExtVal(
    add(lhs.x, rhs.x),
    add(lhs.y, rhs.y),
    add(lhs.z, rhs.z),
    add(lhs.w, rhs.w),
  );
}

fn ext_scale(lhs: ExtVal, rhs: Val) -> ExtVal {
  return ExtVal(
    mul(lhs.x, rhs),
    mul(lhs.y, rhs),
    mul(lhs.z, rhs),
    mul(lhs.w, rhs),
  );
}

fn ext_inv(x: ExtVal) -> ExtVal {
  let beta = sub(0u, NBETA);
  var b0 = add(mul(x.x, x.x), mul(beta, sub(mul(x.y, add(x.w, x.w)), mul(x.z, x.z))));
  var b2 = add(sub(mul(x.x, add(x.z, x.z)), mul(x.y, x.y)), mul(beta, mul(x.w, x.w)));
  let c = add(mul(b0, b0), mul(beta, mul(b2, b2)));
  let ic = inv(c);
  b0 = mul(b0, ic);
  b2 = mul(b2, ic);
  return ExtVal(
    add(mul(x.x, b0), mul(beta, mul(x.z, b2))),
    add(sub(0u, mul(x.y, b0)), mul(NBETA, mul(x.w, b2))),
    add(sub(0u, mul(x.x, b2)), mul(x.z, b0)),
    sub(mul(x.y, b2), mul(x.w, b0)),
  );
}

fn data_at(row: u32, col: u32) -> Val {
  return data[params.data_base + col * params.data_rows + row];
}

fn mix_ext(offset: u32) -> ExtVal {
  let base = params.mix_base + offset;
  return ExtVal(mix[base], mix[base + 1u], mix[base + 2u], mix[base + 3u]);
}

fn store_val(row: u32, col: u32, value: Val) {
  accum[params.accum_base + col * params.accum_rows + row] = value;
}

fn store_ext(row: u32, col: u32, value: ExtVal) {
  store_val(row, col, value.x);
  store_val(row, col + 1u, value.y);
  store_val(row, col + 2u, value.z);
  store_val(row, col + 3u, value.w);
}

fn arg_u8_term(row: u32, count_col: u32, val_col: u32) -> ExtVal {
  let count = data_at(row, count_col);
  let value = data_at(row, val_col);
  let denom = ext_add(ext_scale(mix_ext(MIX_ARG_U8_VAL), value), mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), count);
}

fn arg_u16_term(row: u32, count_col: u32, val_col: u32) -> ExtVal {
  let count = data_at(row, count_col);
  let value = data_at(row, val_col);
  let denom = ext_add(ext_scale(mix_ext(MIX_ARG_U16_VAL), value), mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), count);
}

fn memory_term(
  row: u32,
  count_col: u32,
  addr_col: u32,
  cycle_col: u32,
  data_low_col: u32,
  data_high_col: u32,
) -> ExtVal {
  var denom = ext_scale(mix_ext(MIX_MEMORY_ADDR), data_at(row, addr_col));
  denom = ext_add(denom, ext_scale(mix_ext(MIX_MEMORY_CYCLE), data_at(row, cycle_col)));
  denom = ext_add(denom, ext_scale(mix_ext(MIX_MEMORY_DATA_LOW), data_at(row, data_low_col)));
  denom = ext_add(denom, ext_scale(mix_ext(MIX_MEMORY_DATA_HIGH), data_at(row, data_high_col)));
  denom = ext_add(denom, mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), data_at(row, count_col));
}

fn cycle_term(row: u32, count_col: u32, cycle_col: u32) -> ExtVal {
  var denom = ext_scale(mix_ext(MIX_CYCLE), data_at(row, cycle_col));
  denom = ext_add(denom, mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), data_at(row, count_col));
}

fn store_user_accum(row: u32) {
  store_ext(row, ACC_USER_POLY, ExtVal(0u, 0u, 0u, 0u));
  store_ext(row, ACC_USER_TERM, ExtVal(MONT_ONE, 0u, 0u, 0u));
  store_ext(row, ACC_USER_TOTAL, ExtVal(0u, 0u, 0u, 0u));
  store_val(row, ACC_USER_POLY_OP0, MONT_ONE);
  store_val(row, ACC_USER_POLY_OP1, 0u);
  store_val(row, ACC_USER_POLY_OP2, 0u);
  store_val(row, ACC_USER_POLY_OP3, 0u);
  store_val(row, ACC_USER_POLY_OP4, 0u);
  store_val(row, ACC_USER_POLY_OP5, 0u);
  store_val(row, ACC_USER_POLY_OP6, 0u);
  store_ext(row, ACC_USER_TMP, ExtVal(0u, 0u, 0u, 0u));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= params.row_count) {
    return;
  }
  let row = rows[gid.x];
  store_user_accum(row);

  var cur = ExtVal(0u, 0u, 0u, 0u);

"#,
    );

    let mut term_idx = 0usize;
    let mut push_term = |wgsl: &mut String, term: String| {
        writeln!(wgsl, "  cur = ext_add(cur, {term});").unwrap();
        if term_idx % 3 == 2 {
            writeln!(wgsl, "  store_ext(row, ACC_COL{}, cur);", term_idx / 3).unwrap();
            writeln!(wgsl).unwrap();
        }
        term_idx += 1;
    };

    push_term(
        &mut wgsl,
        "cycle_term(row, CONTROL0_ARG1_COUNT, CONTROL0_ARG1_CYCLE)".to_string(),
    );
    push_term(
        &mut wgsl,
        "cycle_term(row, CONTROL0_ARG2_COUNT, CONTROL0_ARG2_CYCLE)".to_string(),
    );
    for idx in 0..args.memory_arg.len() {
        push_term(
            &mut wgsl,
            format!("memory_term(row, CONTROL0_MEM{idx}_COUNT, CONTROL0_MEM{idx}_ADDR, CONTROL0_MEM{idx}_CYCLE, CONTROL0_MEM{idx}_DATA_LOW, CONTROL0_MEM{idx}_DATA_HIGH)"),
        );
    }
    for idx in 0..args.cycle_arg.len() {
        push_term(
            &mut wgsl,
            format!("cycle_term(row, CONTROL0_CYCLE{idx}_COUNT, CONTROL0_CYCLE{idx}_CYCLE)"),
        );
    }
    for idx in 0..args.arg_u16.len() {
        push_term(
            &mut wgsl,
            format!("arg_u16_term(row, CONTROL0_U16_{idx}_COUNT, CONTROL0_U16_{idx}_VAL)"),
        );
    }
    for idx in 0..args.arg_u8.len() {
        push_term(
            &mut wgsl,
            format!("arg_u8_term(row, CONTROL0_U8_{idx}_COUNT, CONTROL0_U8_{idx}_VAL)"),
        );
    }
    assert_eq!(
        term_idx, 58,
        "CONTROL0 direct accumulator term count changed"
    );

    wgsl.push_str(
        r#"  store_ext(row, ACC_COL19, cur);
}
"#,
    );
    wgsl
}

fn accum_misc_direct_wgsl(
    misc: &'static FinalizeMiscLayout,
    cycle_table: &'static DoCycleTableLayout,
    input: &'static MiscInputLayout,
    output_args: &'static [&'static ArgU16Layout; 5],
) -> String {
    let write_rd = misc._0;
    let decoded = input.decoded;
    let source_args = input.source_regs._arguments_read_source_regs_source_regs;
    let user = LAYOUT_TOP_ACCUM.user._0;
    let randomness = LAYOUT_MIX.randomness;

    let mut wgsl = String::with_capacity(24_000);
    writeln!(
        wgsl,
        "const MIX_ARG_U16_VAL: u32 = {}u;",
        randomness.arg_u16.val.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_OFFSET: u32 = {}u;",
        randomness._offset.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_ADDR: u32 = {}u;",
        randomness.memory_arg.addr.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_CYCLE: u32 = {}u;",
        randomness.memory_arg.cycle.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_DATA_LOW: u32 = {}u;",
        randomness.memory_arg.data_low.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_MEMORY_DATA_HIGH: u32 = {}u;",
        randomness.memory_arg.data_high.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_CYCLE: u32 = {}u;",
        randomness.cycle_arg.cycle.offset
    )
    .unwrap();

    write_arg_u16_const(&mut wgsl, "WRITE_DATA_LOW", misc.write_data.low16.arg);
    write_arg_u16_const(&mut wgsl, "WRITE_DATA_HIGH", misc.write_data.high16.arg);
    write_arg_u16_const(&mut wgsl, "PC_NORM_LOW", misc.pc_norm.low16.arg);
    write_arg_u16_const(&mut wgsl, "PC_NORM_HIGH", misc.pc_norm.high16.arg);
    write_arg_u16_const(
        &mut wgsl,
        "DECODED_UPPER_DIFF",
        decoded.pc_addr.upper_diff.arg,
    );
    write_arg_u16_const(&mut wgsl, "DECODED_MED14", decoded.pc_addr.med14.arg);
    for (idx, arg) in output_args.iter().enumerate() {
        write_arg_u16_const(&mut wgsl, &format!("OUT{idx}"), arg);
    }

    write_memory_arg_const(&mut wgsl, "WRITE_OLD", write_rd._0.io.old_txn);
    write_memory_arg_const(&mut wgsl, "WRITE_NEW", write_rd._0.io.new_txn);
    write_memory_arg_const(&mut wgsl, "LOAD_OLD", decoded.load_inst.io.old_txn);
    write_memory_arg_const(&mut wgsl, "LOAD_NEW", decoded.load_inst.io.new_txn);
    for (idx, arg) in source_args.memory_arg.iter().enumerate() {
        write_memory_arg_const(&mut wgsl, &format!("SRC_MEM{idx}"), arg);
    }

    write_cycle_arg_const(&mut wgsl, "WRITE_CYCLE", write_rd._0._0._0.arg);
    write_cycle_arg_const(&mut wgsl, "MISC_ARG1", cycle_table.arg1);
    write_cycle_arg_const(&mut wgsl, "MISC_ARG2", cycle_table.arg2);
    write_cycle_arg_const(&mut wgsl, "LOAD_CYCLE", decoded.load_inst._0._0.arg);
    for (idx, arg) in source_args.cycle_arg.iter().enumerate() {
        write_cycle_arg_const(&mut wgsl, &format!("SRC_CYCLE{idx}"), arg);
    }

    writeln!(
        wgsl,
        "const ACC_USER_POLY: u32 = {}u;",
        user.state.poly._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const ACC_USER_TERM: u32 = {}u;",
        user.state.term._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const ACC_USER_TOTAL: u32 = {}u;",
        user.state.total._super.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const ACC_USER_TMP: u32 = {}u;",
        user.state_redef.arm3.tmp._super.offset
    )
    .unwrap();
    for (idx, bit) in user.poly_op._super.iter().enumerate() {
        writeln!(
            wgsl,
            "const ACC_USER_POLY_OP{idx}: u32 = {}u;",
            bit._super.offset
        )
        .unwrap();
    }
    for col in 0..9 {
        writeln!(
            wgsl,
            "const ACC_COL{col}: u32 = {}u;",
            LAYOUT_TOP_ACCUM.columns[col].offset
        )
        .unwrap();
    }
    writeln!(
        wgsl,
        "const ACC_COL19: u32 = {}u;",
        LAYOUT_TOP_ACCUM.columns[19].offset
    )
    .unwrap();

    wgsl.push_str(
        r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const MONT_ONE: u32 = 268435454u;

alias Val = u32;
alias ExtVal = vec4<u32>;

struct Params {
  data_rows: u32,
  accum_rows: u32,
  row_count: u32,
  data_base: u32,
  accum_base: u32,
  mix_base: u32,
  _pad0: u32,
  _pad1: u32,
}

@group(0) @binding(0) var<storage, read> data: array<u32>;
@group(0) @binding(1) var<storage, read_write> accum: array<u32>;
@group(0) @binding(2) var<storage, read> mix: array<u32>;
@group(0) @binding(3) var<storage, read> rows: array<u32>;
@group(0) @binding(4) var<uniform> params: Params;

fn add(lhs: Val, rhs: Val) -> Val {
  let sum = lhs + rhs;
  if (sum >= P) {
    return sum - P;
  }
  return sum;
}

fn sub(lhs: Val, rhs: Val) -> Val {
  if (lhs >= rhs) {
    return lhs - rhs;
  }
  return lhs + P - rhs;
}

fn mul_wide(lhs: u32, rhs: u32) -> vec2<u32> {
  let lhs_lo = lhs & 0xffffu;
  let lhs_hi = lhs >> 16u;
  let rhs_lo = rhs & 0xffffu;
  let rhs_hi = rhs >> 16u;
  let p0 = lhs_lo * rhs_lo;
  let p1 = lhs_hi * rhs_lo;
  let p2 = lhs_lo * rhs_hi;
  let p3 = lhs_hi * rhs_hi;
  let carry = (p0 >> 16u) + (p1 & 0xffffu) + (p2 & 0xffffu);
  let lo = (p0 & 0xffffu) | ((carry & 0xffffu) << 16u);
  let hi = p3 + (p1 >> 16u) + (p2 >> 16u) + (carry >> 16u);
  return vec2<u32>(lo, hi);
}

fn mul(lhs: Val, rhs: Val) -> Val {
  let product = mul_wide(lhs, rhs);
  let low = 0u - product.x;
  let red = M * low;
  let red_product = mul_wide(red, P);
  var ret = product.y + red_product.y;
  if (product.x + red_product.x < product.x) {
    ret = ret + 1u;
  }
  if (ret >= P) {
    return ret - P;
  }
  return ret;
}

fn pow(base: Val, exp: u32) -> Val {
  var result: Val = MONT_ONE;
  var b: Val = base;
  var e: u32 = exp;
  while (e != 0u) {
    if ((e & 1u) != 0u) {
      result = mul(result, b);
    }
    b = mul(b, b);
    e = e >> 1u;
  }
  return result;
}

fn inv(x: Val) -> Val {
  return pow(x, P - 2u);
}

fn ext_add(lhs: ExtVal, rhs: ExtVal) -> ExtVal {
  return ExtVal(
    add(lhs.x, rhs.x),
    add(lhs.y, rhs.y),
    add(lhs.z, rhs.z),
    add(lhs.w, rhs.w),
  );
}

fn ext_scale(lhs: ExtVal, rhs: Val) -> ExtVal {
  return ExtVal(
    mul(lhs.x, rhs),
    mul(lhs.y, rhs),
    mul(lhs.z, rhs),
    mul(lhs.w, rhs),
  );
}

fn ext_inv(x: ExtVal) -> ExtVal {
  let beta = sub(0u, NBETA);
  var b0 = add(mul(x.x, x.x), mul(beta, sub(mul(x.y, add(x.w, x.w)), mul(x.z, x.z))));
  var b2 = add(sub(mul(x.x, add(x.z, x.z)), mul(x.y, x.y)), mul(beta, mul(x.w, x.w)));
  let c = add(mul(b0, b0), mul(beta, mul(b2, b2)));
  let ic = inv(c);
  b0 = mul(b0, ic);
  b2 = mul(b2, ic);
  return ExtVal(
    add(mul(x.x, b0), mul(beta, mul(x.z, b2))),
    add(sub(0u, mul(x.y, b0)), mul(NBETA, mul(x.w, b2))),
    add(sub(0u, mul(x.x, b2)), mul(x.z, b0)),
    sub(mul(x.y, b2), mul(x.w, b0)),
  );
}

fn data_at(row: u32, col: u32) -> Val {
  return data[params.data_base + col * params.data_rows + row];
}

fn mix_ext(offset: u32) -> ExtVal {
  let base = params.mix_base + offset;
  return ExtVal(mix[base], mix[base + 1u], mix[base + 2u], mix[base + 3u]);
}

fn store_val(row: u32, col: u32, value: Val) {
  accum[params.accum_base + col * params.accum_rows + row] = value;
}

fn store_ext(row: u32, col: u32, value: ExtVal) {
  store_val(row, col, value.x);
  store_val(row, col + 1u, value.y);
  store_val(row, col + 2u, value.z);
  store_val(row, col + 3u, value.w);
}

fn arg_u16_term(row: u32, count_col: u32, val_col: u32) -> ExtVal {
  let count = data_at(row, count_col);
  let value = data_at(row, val_col);
  let denom = ext_add(ext_scale(mix_ext(MIX_ARG_U16_VAL), value), mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), count);
}

fn memory_term(
  row: u32,
  count_col: u32,
  addr_col: u32,
  cycle_col: u32,
  data_low_col: u32,
  data_high_col: u32,
) -> ExtVal {
  var denom = ext_scale(mix_ext(MIX_MEMORY_ADDR), data_at(row, addr_col));
  denom = ext_add(denom, ext_scale(mix_ext(MIX_MEMORY_CYCLE), data_at(row, cycle_col)));
  denom = ext_add(denom, ext_scale(mix_ext(MIX_MEMORY_DATA_LOW), data_at(row, data_low_col)));
  denom = ext_add(denom, ext_scale(mix_ext(MIX_MEMORY_DATA_HIGH), data_at(row, data_high_col)));
  denom = ext_add(denom, mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), data_at(row, count_col));
}

fn cycle_term(row: u32, count_col: u32, cycle_col: u32) -> ExtVal {
  var denom = ext_scale(mix_ext(MIX_CYCLE), data_at(row, cycle_col));
  denom = ext_add(denom, mix_ext(MIX_OFFSET));
  return ext_scale(ext_inv(denom), data_at(row, count_col));
}

fn store_user_accum(row: u32) {
  store_ext(row, ACC_USER_POLY, ExtVal(0u, 0u, 0u, 0u));
  store_ext(row, ACC_USER_TERM, ExtVal(MONT_ONE, 0u, 0u, 0u));
  store_ext(row, ACC_USER_TOTAL, ExtVal(0u, 0u, 0u, 0u));
  store_val(row, ACC_USER_POLY_OP0, MONT_ONE);
  store_val(row, ACC_USER_POLY_OP1, 0u);
  store_val(row, ACC_USER_POLY_OP2, 0u);
  store_val(row, ACC_USER_POLY_OP3, 0u);
  store_val(row, ACC_USER_POLY_OP4, 0u);
  store_val(row, ACC_USER_POLY_OP5, 0u);
  store_val(row, ACC_USER_POLY_OP6, 0u);
  store_ext(row, ACC_USER_TMP, ExtVal(0u, 0u, 0u, 0u));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= params.row_count) {
    return;
  }
  let row = rows[gid.x];
  store_user_accum(row);

  var cur = ExtVal(0u, 0u, 0u, 0u);

  cur = ext_add(cur, arg_u16_term(row, WRITE_DATA_LOW_COUNT, WRITE_DATA_LOW_VAL));
  cur = ext_add(cur, arg_u16_term(row, WRITE_DATA_HIGH_COUNT, WRITE_DATA_HIGH_VAL));
  cur = ext_add(cur, arg_u16_term(row, PC_NORM_LOW_COUNT, PC_NORM_LOW_VAL));
  store_ext(row, ACC_COL0, cur);

  cur = ext_add(cur, arg_u16_term(row, PC_NORM_HIGH_COUNT, PC_NORM_HIGH_VAL));
  cur = ext_add(cur, memory_term(row, WRITE_OLD_COUNT, WRITE_OLD_ADDR, WRITE_OLD_CYCLE, WRITE_OLD_DATA_LOW, WRITE_OLD_DATA_HIGH));
  cur = ext_add(cur, memory_term(row, WRITE_NEW_COUNT, WRITE_NEW_ADDR, WRITE_NEW_CYCLE, WRITE_NEW_DATA_LOW, WRITE_NEW_DATA_HIGH));
  store_ext(row, ACC_COL1, cur);

  cur = ext_add(cur, cycle_term(row, WRITE_CYCLE_COUNT, WRITE_CYCLE_CYCLE));
  cur = ext_add(cur, cycle_term(row, MISC_ARG1_COUNT, MISC_ARG1_CYCLE));
  cur = ext_add(cur, cycle_term(row, MISC_ARG2_COUNT, MISC_ARG2_CYCLE));
  store_ext(row, ACC_COL2, cur);

  cur = ext_add(cur, arg_u16_term(row, DECODED_UPPER_DIFF_COUNT, DECODED_UPPER_DIFF_VAL));
  cur = ext_add(cur, arg_u16_term(row, DECODED_MED14_COUNT, DECODED_MED14_VAL));
  cur = ext_add(cur, memory_term(row, LOAD_OLD_COUNT, LOAD_OLD_ADDR, LOAD_OLD_CYCLE, LOAD_OLD_DATA_LOW, LOAD_OLD_DATA_HIGH));
  store_ext(row, ACC_COL3, cur);

  cur = ext_add(cur, memory_term(row, LOAD_NEW_COUNT, LOAD_NEW_ADDR, LOAD_NEW_CYCLE, LOAD_NEW_DATA_LOW, LOAD_NEW_DATA_HIGH));
  cur = ext_add(cur, cycle_term(row, LOAD_CYCLE_COUNT, LOAD_CYCLE_CYCLE));
  cur = ext_add(cur, memory_term(row, SRC_MEM0_COUNT, SRC_MEM0_ADDR, SRC_MEM0_CYCLE, SRC_MEM0_DATA_LOW, SRC_MEM0_DATA_HIGH));
  store_ext(row, ACC_COL4, cur);

  cur = ext_add(cur, memory_term(row, SRC_MEM1_COUNT, SRC_MEM1_ADDR, SRC_MEM1_CYCLE, SRC_MEM1_DATA_LOW, SRC_MEM1_DATA_HIGH));
  cur = ext_add(cur, memory_term(row, SRC_MEM2_COUNT, SRC_MEM2_ADDR, SRC_MEM2_CYCLE, SRC_MEM2_DATA_LOW, SRC_MEM2_DATA_HIGH));
  cur = ext_add(cur, memory_term(row, SRC_MEM3_COUNT, SRC_MEM3_ADDR, SRC_MEM3_CYCLE, SRC_MEM3_DATA_LOW, SRC_MEM3_DATA_HIGH));
  store_ext(row, ACC_COL5, cur);

  cur = ext_add(cur, cycle_term(row, SRC_CYCLE0_COUNT, SRC_CYCLE0_CYCLE));
  cur = ext_add(cur, cycle_term(row, SRC_CYCLE1_COUNT, SRC_CYCLE1_CYCLE));
  cur = ext_add(cur, arg_u16_term(row, OUT0_COUNT, OUT0_VAL));
  store_ext(row, ACC_COL6, cur);

  cur = ext_add(cur, arg_u16_term(row, OUT1_COUNT, OUT1_VAL));
  cur = ext_add(cur, arg_u16_term(row, OUT2_COUNT, OUT2_VAL));
  cur = ext_add(cur, arg_u16_term(row, OUT3_COUNT, OUT3_VAL));
  store_ext(row, ACC_COL7, cur);

  cur = ext_add(cur, arg_u16_term(row, OUT4_COUNT, OUT4_VAL));
  store_ext(row, ACC_COL8, cur);
  store_ext(row, ACC_COL19, cur);
}
"#,
    );
    wgsl
}

#[derive(Clone, Copy)]
enum AccumMiscDirectKind {
    Misc0,
    Misc1,
    Misc2,
    Mem0,
    Mem1,
    Control0,
}

impl AccumMiscDirectKind {
    fn label(self) -> &'static str {
        match self {
            Self::Misc0 => "MISC0",
            Self::Misc1 => "MISC1",
            Self::Misc2 => "MISC2",
            Self::Mem0 => "MEM0",
            Self::Mem1 => "MEM1",
            Self::Control0 => "CONTROL0",
        }
    }

    fn row_source(self) -> &'static str {
        match self {
            Self::Misc0 => "rv32im_accum_misc0_direct_rows",
            Self::Misc1 => "rv32im_accum_misc1_direct_rows",
            Self::Misc2 => "rv32im_accum_misc2_direct_rows",
            Self::Mem0 => "rv32im_accum_mem0_direct_rows",
            Self::Mem1 => "rv32im_accum_mem1_direct_rows",
            Self::Control0 => "rv32im_accum_control0_direct_rows",
        }
    }

    fn params_source(self) -> &'static str {
        match self {
            Self::Misc0 => "rv32im_accum_misc0_direct_params",
            Self::Misc1 => "rv32im_accum_misc1_direct_params",
            Self::Misc2 => "rv32im_accum_misc2_direct_params",
            Self::Mem0 => "rv32im_accum_mem0_direct_params",
            Self::Mem1 => "rv32im_accum_mem1_direct_params",
            Self::Control0 => "rv32im_accum_control0_direct_params",
        }
    }

    fn layout_label(self) -> &'static str {
        match self {
            Self::Misc0 => "rv32im_accum_misc0_direct_layout",
            Self::Misc1 => "rv32im_accum_misc1_direct_layout",
            Self::Misc2 => "rv32im_accum_misc2_direct_layout",
            Self::Mem0 => "rv32im_accum_mem0_direct_layout",
            Self::Mem1 => "rv32im_accum_mem1_direct_layout",
            Self::Control0 => "rv32im_accum_control0_direct_layout",
        }
    }

    fn bind_group_label(self) -> &'static str {
        match self {
            Self::Misc0 => "rv32im_accum_misc0_direct_bind_group",
            Self::Misc1 => "rv32im_accum_misc1_direct_bind_group",
            Self::Misc2 => "rv32im_accum_misc2_direct_bind_group",
            Self::Mem0 => "rv32im_accum_mem0_direct_bind_group",
            Self::Mem1 => "rv32im_accum_mem1_direct_bind_group",
            Self::Control0 => "rv32im_accum_control0_direct_bind_group",
        }
    }

    fn record_rows(self, rows: usize) {
        match self {
            Self::Misc0 => {
                ACCUM_GPU_MISC0_DIRECT_ROWS.fetch_add(rows, Ordering::SeqCst);
            }
            Self::Misc1 => {
                ACCUM_GPU_MISC1_DIRECT_ROWS.fetch_add(rows, Ordering::SeqCst);
            }
            Self::Misc2 => {
                ACCUM_GPU_MISC2_DIRECT_ROWS.fetch_add(rows, Ordering::SeqCst);
            }
            Self::Mem0 => {
                ACCUM_GPU_MEM0_DIRECT_ROWS.fetch_add(rows, Ordering::SeqCst);
            }
            Self::Mem1 => {
                ACCUM_GPU_MEM1_DIRECT_ROWS.fetch_add(rows, Ordering::SeqCst);
            }
            Self::Control0 => {
                ACCUM_GPU_CONTROL0_DIRECT_ROWS.fetch_add(rows, Ordering::SeqCst);
            }
        }
    }
}

fn topaccum_replace_ext_inv_calls(src: &str, adapter: &str) -> Result<(String, u32)> {
    let needle = "ext_inv(";
    let mut rest = src;
    let mut out = String::with_capacity(src.len() + 512);
    let mut slot = 0u32;
    while let Some(pos) = rest.find(needle) {
        out.push_str(&rest[..pos]);
        out.push_str(adapter);
        out.push('(');
        out.push_str(&slot.to_string());
        out.push_str("u, ");
        rest = &rest[pos + needle.len()..];
        slot = slot
            .checked_add(1)
            .context("TopAccum arm5 ext_inv call count overflow")?;
    }
    out.push_str(rest);
    Ok((out, slot))
}

fn topaccum_arm5_replace_ext_inv_calls(src: &str, adapter: &str) -> Result<(String, u32)> {
    let (out, slot) = topaccum_replace_ext_inv_calls(src, adapter)?;
    anyhow::ensure!(
        slot == TOPACCUM_ARM5_INV_CALLS,
        "TopAccum ext_inv call count changed: expected {}, found {}",
        TOPACCUM_ARM5_INV_CALLS,
        slot
    );
    Ok((out, slot))
}

fn topaccum_arm5_inv_entry(template: &str, inv_count: u32) -> String {
    template.replace("__INV_COUNT__", &format!("{inv_count}u"))
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

/// SP7 iter 6d-g step 6.2.5 (2026-05-16): per-cycle diff counts
/// (`preflight.cycles[i].diff_count[0..1]`) packed as `[a0, a1, b0, b1, ...]`
/// for the patched `extern_getDiffCount` to index. The patched stub
/// reads `preflight_diff_count_buf[idx]` where `idx = decode(txn_cycle)
/// = cycle*2 + i`, matching the rust `get_diff_count` lookup.
fn build_preflight_diff_count(preflight: &PreflightTrace) -> Vec<u32> {
    let mut out = Vec::with_capacity(preflight.cycles.len() * 2);
    for cycle in &preflight.cycles {
        out.push(cycle.diff_count[0]);
        out.push(cycle.diff_count[1]);
    }
    out
}

fn build_preflight_txn_start(preflight: &PreflightTrace) -> Vec<u32> {
    let mut out = Vec::with_capacity(preflight.cycles.len());
    for cycle in &preflight.cycles {
        out.push(cycle.txn_idx);
    }
    out
}

fn build_preflight_txns(preflight: &PreflightTrace) -> Vec<u32> {
    let mut out = Vec::with_capacity(preflight.txns.len() * 5);
    for txn in &preflight.txns {
        out.push(txn.prev_cycle);
        out.push(txn.prev_word & 0xFFFF);
        out.push(txn.prev_word >> 16);
        out.push(txn.word & 0xFFFF);
        out.push(txn.word >> 16);
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
    // shadow_init has already written: cols 0 (cycle), 1-13 (majorOnehot),
    // 14-18 (next* + isFirstCycle), 19-20 (major/minor), 21-28 (minorOnehot).
    // Wrapper only needs to: (1) read back_Reg(1, ...) for previous cycle's
    // next* cells, (2) construct InstInputStruct inline (no helper-fn calls
    // since exec_InstInput / exec_OneHot_8_ aren't in baseline+delta),
    // (3) call the arm sub-fn.
    // Binding 7 (preflight_diff_count_buf) is declared by
    // patch_extern_get_diff_count when it rewrites the stub; do not
    // redeclare here.
    format!(
        "@group(0) @binding(5) var<storage, read> cycle_list: array<u32>;\n\
         @group(0) @binding(6) var<storage, read> preflight_meta: array<u32>;\n\
         \n\
         // back_NondetReg / back_Reg: the 8 zero-back-reg arm deltas don't\n\
         // emit these because their sub-fns don't call them. Wrapper needs\n\
         // them to read previous-cycle outer Top cells (nextPc/state/mode).\n\
         fn back_NondetReg(distance0: Index, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {{\n\
           let x2: NondetRegStruct = NondetRegStruct(load(lookup_NondetRegLayout__super(layout1), distance0));\n\
           return x2;\n\
         }}\n\
         fn back_Reg(distance0: Index, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {{\n\
           return back_NondetReg(distance0, layout1);\n\
         }}\n\
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
           let minor_u = packed & 0xFFFFu;\n\
           let is_first_super = select(0u, MONT_ONE, cycle == 0u);\n\
           let x4 = sub(MONT_ONE, is_first_super);\n\
           let x9 = back_Reg(1, lookup_TopLayout_nextPcLow(bound_top));\n\
           let x10 = back_Reg(1, lookup_TopLayout_nextPcHigh(bound_top));\n\
           let x11 = back_Reg(1, lookup_TopLayout_nextState_0(bound_top));\n\
           let x12 = back_Reg(1, lookup_TopLayout_nextMachineMode(bound_top));\n\
           let m0 = NondetRegStruct(select(0u, MONT_ONE, minor_u == 0u));\n\
           let m1 = NondetRegStruct(select(0u, MONT_ONE, minor_u == 1u));\n\
           let m2 = NondetRegStruct(select(0u, MONT_ONE, minor_u == 2u));\n\
           let m3 = NondetRegStruct(select(0u, MONT_ONE, minor_u == 3u));\n\
           let m4 = NondetRegStruct(select(0u, MONT_ONE, minor_u == 4u));\n\
           let m5 = NondetRegStruct(select(0u, MONT_ONE, minor_u == 5u));\n\
           let m6 = NondetRegStruct(select(0u, MONT_ONE, minor_u == 6u));\n\
           let m7 = NondetRegStruct(select(0u, MONT_ONE, minor_u == 7u));\n\
           let onehot = OneHot_8_Struct(NondetRegStruct8Array(m0, m1, m2, m3, m4, m5, m6, m7));\n\
           let inst_input = InstInputStruct(\n\
             encode(minor_u),\n\
             ValU32Struct(mul(x4, x9._super), mul(x4, x10._super)),\n\
             mul(x4, x11._super),\n\
             add(mul(x4, x12._super), is_first_super),\n\
             onehot,\n\
           );\n\
           let x20 = back_Reg(0, lookup_TopCycleLayout__super(lookup_TopLayout_cycleRedef(bound_top)));\n\
           let _result = {sub_fn}(x20, inst_input, lookup_TopInstResultLayout_arm{arm_idx}(lookup_TopLayout_instResult(bound_top)){extra_arg});\n\
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
// MISC2 and MEM0 remain diff/probe-capable, but representative e2e evidence
// showed their sparse CPU-shadow repair is currently wall-negative. Keep
// production replacement mode MISC0-only until that repair cost is removed.
const WITGEN_REPLACE_BASE_SUPPORTED_ARM_MASK: u16 = 1u16 << 0;

fn is_zero_back_reg_arm(arm_idx: usize) -> bool {
    ZERO_BACK_REG_ARMS.contains(&arm_idx)
}

fn witgen_mem0_candidate_enabled() -> bool {
    WITGEN_GPU_MEM0_REPLACE_CANDIDATE_ENABLED.load(Ordering::SeqCst)
        && ACCUM_GPU_MEM0_DIRECT_ENABLED.load(Ordering::SeqCst)
}

fn witgen_mem1_candidate_enabled() -> bool {
    WITGEN_GPU_MEM1_REPLACE_CANDIDATE_ENABLED.load(Ordering::SeqCst)
        && ACCUM_GPU_MEM1_DIRECT_ENABLED.load(Ordering::SeqCst)
}

fn witgen_mem0_replace_minor_enabled(minor: u8) -> bool {
    matches!(minor, 0 | 1 | 2 | 3 | 4)
        && (WITGEN_GPU_MEM0_REPLACE_MINOR_MASK.load(Ordering::SeqCst) & (1u16 << minor)) != 0
}

fn witgen_mem1_replace_minor_enabled(minor: u8) -> bool {
    matches!(minor, 0 | 1 | 2)
        && (WITGEN_GPU_MEM1_REPLACE_MINOR_MASK.load(Ordering::SeqCst) & (1u16 << minor)) != 0
}

fn witgen_replace_supported_arm_mask() -> u16 {
    let mut mask = WITGEN_REPLACE_BASE_SUPPORTED_ARM_MASK;
    if witgen_mem0_candidate_enabled() {
        mask |= 1u16 << 5;
    }
    if witgen_mem1_candidate_enabled() {
        mask |= 1u16 << 6;
    }
    mask
}

fn is_witgen_replace_supported_arm(arm_idx: usize) -> bool {
    arm_idx < 16 && (witgen_replace_supported_arm_mask() & (1u16 << arm_idx)) != 0
}

fn is_witgen_replace_cycle(major: u8, minor: u8) -> bool {
    // Keep this in sync with rust_steps::cycle_short_circuited. Only skip CPU
    // rows whose lookup-table side effects are replayed explicitly.
    (major == 0 && matches!(minor, 0 | 1 | 2 | 3 | 4 | 7))
        || (major == 2 && matches!(minor, 0 | 2 | 3 | 4 | 5 | 6 | 7))
        || (major == 5
            && witgen_mem0_candidate_enabled()
            && witgen_mem0_replace_minor_enabled(minor))
        || (major == 6
            && witgen_mem1_candidate_enabled()
            && witgen_mem1_replace_minor_enabled(minor))
}

fn is_witgen_diff_cycle(major: u8, minor: u8) -> bool {
    let selected = WITGEN_GPU_DIFF_MAJOR.load(Ordering::SeqCst);
    if selected == WITGEN_GPU_DIFF_MAJOR_NONE {
        return is_witgen_replace_cycle(major, minor);
    }
    major as usize == selected
}

fn witgen_cycle_selected_for_mode(major: u8, minor: u8, diff_only_mode: bool) -> bool {
    if diff_only_mode {
        is_witgen_diff_cycle(major, minor)
    } else {
        is_witgen_replace_cycle(major, minor)
    }
}

fn needed_extra_minors(
    preflight: &PreflightTrace,
    arm_idx: usize,
    deltas: &[(u8, &str, &str, &str)],
    diff_only_mode: bool,
) -> std::collections::BTreeSet<u8> {
    deltas
        .iter()
        .filter_map(|(minor, _, _, _)| {
            preflight
                .cycles
                .iter()
                .any(|cycle| {
                    cycle.major as usize == arm_idx
                        && cycle.minor == *minor
                        && witgen_cycle_selected_for_mode(cycle.major, cycle.minor, diff_only_mode)
                })
                .then_some(*minor)
        })
        .collect()
}

/// SP7 iter 6d-g step 6.2.4: synthesize chunk1 wrapper. Uses
/// EXEC_TOP_CHUNK1_WGSL as the standalone module (already contains
/// exec_NondetReg, exec_NondetBitReg, exec_InstInput, exec_OneHot_13_,
/// back_Reg, back_NondetReg, lookup helpers, kLayout_Top, etc.). The
/// wrapper just appends a @compute entry that calls exec_<arm>Chunk1.
fn synth_arm_chunk1_wrapper(label: &str, arm_idx: usize) -> String {
    let sub_fn = format!(
        "exec_{}",
        match arm_idx {
            0 => "Misc0",
            1 => "Misc1",
            2 => "Misc2",
            3 => "Mul0",
            4 => "Div0",
            5 => "Mem0",
            6 => "Mem1",
            8 => "ECall0",
            _ => "UNUSED",
        }
    );
    let extra_arg = if arm_idx == 8 { ", buf_global" } else { "" };
    // Binding 7 (preflight_diff_count_buf) is declared by
    // patch_extern_get_diff_count when it rewrites the stub; do not
    // redeclare here.
    format!(
        "@group(0) @binding(5) var<storage, read> cycle_list: array<u32>;\n\
         @group(0) @binding(6) var<storage, read> preflight_meta: array<u32>;\n\
         \n\
         @compute @workgroup_size(64)\n\
         fn iter6d_g_{label}_c1_main(@builtin(global_invocation_id) gid: vec3<u32>) {{\n\
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
           let _x18 = exec_OneHot_13_(x15._super, lookup_TopLayout_majorOnehot(bound_top));\n\
           let x20 = back_Reg(0, lookup_TopCycleLayout__super(lookup_TopLayout_cycleRedef(bound_top)));\n\
           let _result = {sub_fn}Chunk1(x20, x17, lookup_TopInstResultLayout_arm{arm_idx}(lookup_TopLayout_instResult(bound_top)){extra_arg});\n\
         }}\n",
        label = label,
        sub_fn = sub_fn,
        arm_idx = arm_idx,
        extra_arg = extra_arg,
    )
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

const ACCUM_MACHINE_COLUMN_CARRY_WGSL: &str = r#"
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

const ACCUM_TERMINAL_EXT_PREFIX_WGSL: &str = r#"
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
        Self { hal }
    }

    fn start_witgen_replacement_prewarm(&self) -> Result<WitgenReplacementPrewarm> {
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
            "iter6d_g_arm_layout",
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
                let entry = format!("iter6d_g_{}_main", label);
                let started = self.hal.start_compute_kernel_async(
                    "iter6d_g_arm_kernel",
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
            let entry = format!("iter6d_g_{extra_label}_main");
            let started = self.hal.start_compute_kernel_async(
                "iter6d_g_misc0_extra_kernel",
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
                let entry = format!("iter6d_g_{extra_label}_main");
                let started = self.hal.start_compute_kernel_async(
                    "iter6d_g_mem0_extra_kernel",
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
                let entry = format!("iter6d_g_{extra_label}_main");
                let started = self.hal.start_compute_kernel_async(
                    "iter6d_g_mem1_extra_kernel",
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
            let entry = format!("iter6d_g_{}_c1_main", label);
            let started = self.hal.start_compute_kernel_async(
                "iter6d_g_arm_chunk1_kernel",
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

    async fn finish_witgen_replacement_prewarm(
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
                        "iter6d_g_prewarm arm={} DONE",
                        label
                    ));
                }
                Err(err) => {
                    WITGEN_ARM_PENDING_KERNELS.with(|cell| cell.borrow_mut().remove(label));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "iter6d_g_prewarm arm={} FAILED err={err:?}",
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
                        "iter6d_g_prewarm arm={} DONE",
                        label
                    ));
                }
                Err(err) => {
                    WITGEN_MISC0_EXTRA_PENDING_KERNELS
                        .with(|cell| cell.borrow_mut().remove(&minor));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "iter6d_g_prewarm arm={} FAILED err={err:?}",
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
                        "iter6d_g_prewarm arm={} DONE",
                        label
                    ));
                }
                Err(err) => {
                    WITGEN_MEM0_EXTRA_PENDING_KERNELS.with(|cell| cell.borrow_mut().remove(&minor));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "iter6d_g_prewarm arm={} FAILED err={err:?}",
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
                        "iter6d_g_prewarm arm={} DONE",
                        label
                    ));
                }
                Err(err) => {
                    WITGEN_MEM1_EXTRA_PENDING_KERNELS.with(|cell| cell.borrow_mut().remove(&minor));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "iter6d_g_prewarm arm={} FAILED err={err:?}",
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
                        "iter6d_g_prewarm arm={}_chunk1 DONE",
                        label
                    ));
                }
                Err(err) => {
                    WITGEN_ARM_PENDING_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().remove(label));
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "iter6d_g_prewarm arm={}_chunk1 FAILED err={err:?}",
                        label
                    ));
                }
            }
        }

        if misc0_extra_requested != 0 {
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "iter6d_g_prewarm misc0_extra requested={misc0_extra_requested}",
            ));
        }
        if mem0_extra_requested != 0 {
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "iter6d_g_prewarm mem0_extra requested={mem0_extra_requested}",
            ));
        }
        if mem1_extra_requested != 0 {
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "iter6d_g_prewarm mem1_extra requested={mem1_extra_requested}",
            ));
        }
        if arm_chunk0_requested != 0 {
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "iter6d_g_prewarm ALL arms requested={arm_chunk0_requested}",
            ));
        }
        if arm_chunk1_requested != 0 {
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "iter6d_g_prewarm chunk1 ALL requested={arm_chunk1_requested}",
            ));
        }
    }

    async fn finish_pending_witgen_replace_arm_async(
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
                "iter6d_g_prewarm_pending arm={} chunk0 DONE",
                label,
            ));
        }

        if let Some(started) =
            WITGEN_ARM_PENDING_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().remove(label))
        {
            let kernel = self.hal.finish_compute_kernel_async(started).await?;
            WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(*label, kernel));
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "iter6d_g_prewarm_pending arm={} chunk1 DONE",
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
                    "iter6d_g_prewarm_pending misc0_minor={} DONE",
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
                    "iter6d_g_prewarm_pending mem0_minor={} DONE",
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
                    "iter6d_g_prewarm_pending mem1_minor={} DONE",
                    minor,
                ));
            }
        }

        Ok(())
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
        let replace_prewarm_requested = WITGEN_GPU_REPLACE_ENABLED.load(Ordering::SeqCst);
        let replacement_prewarm = match self.start_witgen_replacement_prewarm() {
            Ok(prewarm) => prewarm,
            Err(err) => {
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_prewarm start_FAILED err={err:?}"
                ));
                WitgenReplacementPrewarm::default()
            }
        };
        let hal = self.hal.clone();
        wasm_bindgen_futures::spawn_local(async move {
            // SP7 iter 6d-e: compile both top-mux chunks. Chrome
            // pipelines createComputePipelineAsync internally so the
            // two compiles can overlap with each other and with
            // session execution. Measured wall on xgboost: chunk0
            // compile ~2.65 s, chunk1 ~similar.
            let _t = WebGpuStageTimer::new("iter6d_d_witgen_prewarm_async");
            Self::finish_witgen_replacement_prewarm(hal.clone(), replacement_prewarm).await;
            if replace_prewarm_requested {
                return;
            }
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
                    WITGEN_TOP_CHUNK0_KERNEL.with(|cell| *cell.borrow_mut() = Some(kernel));
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
                    WITGEN_TOP_CHUNK1_KERNEL.with(|cell| *cell.borrow_mut() = Some(kernel));
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

    fn ready_witgen_replace_mask(&self, preflight: &PreflightTrace) -> u16 {
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

    async fn ensure_witgen_replace_arm_ready_async(
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
            "iter6d_g_arm_layout",
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
            let chunk0_entry = format!("iter6d_g_{}_main", label);
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
            let chunk1_entry = format!("iter6d_g_{}_c1_main", label);
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
                        let entry = format!("iter6d_g_{extra_label}_main");
                        Some((*minor, module, entry, *extra_label))
                    })
                    .collect();
            let misc0_extra_futures: Vec<_> = misc0_extra_modules
                .iter()
                .map(|(_, module, entry, _)| {
                    self.hal.create_compute_kernel_async(
                        "iter6d_g_misc0_extra_kernel_on_demand",
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
                                "iter6d_g_arm_kernel_on_demand",
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
                                "iter6d_g_arm_chunk1_kernel_on_demand",
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
                    "iter6d_g_on_demand arm={} chunk0 DONE",
                    label,
                ));
            }
            if let Some(result) = chunk1_result {
                let kernel = result?;
                WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(*label, kernel));
                record_witgen_replace_on_demand_kernel_compile(label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_on_demand arm={} chunk1 DONE",
                    label,
                ));
            }
            for (i, fut) in misc0_extra_futures.into_iter().enumerate() {
                let kernel = fut.await?;
                let (minor, _, _, extra_label) = &misc0_extra_modules[i];
                WITGEN_MISC0_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(*minor, kernel));
                record_witgen_replace_on_demand_kernel_compile(extra_label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_on_demand arm={} DONE",
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
            let chunk0_entry = format!("iter6d_g_{}_main", label);
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
            let chunk1_entry = format!("iter6d_g_{}_c1_main", label);
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
                        let entry = format!("iter6d_g_{extra_label}_main");
                        Some((*minor, module, entry, *extra_label))
                    })
                    .collect();
            let misc2_extra_futures: Vec<_> = misc2_extra_modules
                .iter()
                .map(|(_, module, entry, _)| {
                    self.hal.create_compute_kernel_async(
                        "iter6d_g_misc2_extra_kernel_on_demand",
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
                                "iter6d_g_arm_kernel_on_demand",
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
                                "iter6d_g_arm_chunk1_kernel_on_demand",
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
                    "iter6d_g_on_demand arm={} chunk0 DONE",
                    label,
                ));
            }
            if let Some(result) = chunk1_result {
                let kernel = result?;
                WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(*label, kernel));
                record_witgen_replace_on_demand_kernel_compile(label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_on_demand arm={} chunk1 DONE",
                    label,
                ));
            }
            for (i, fut) in misc2_extra_futures.into_iter().enumerate() {
                let kernel = fut.await?;
                let (minor, _, _, extra_label) = &misc2_extra_modules[i];
                WITGEN_MISC2_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(*minor, kernel));
                record_witgen_replace_on_demand_kernel_compile(extra_label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_on_demand arm={} DONE",
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
            let chunk0_entry = format!("iter6d_g_{}_main", label);
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
            let chunk1_entry = format!("iter6d_g_{}_c1_main", label);
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
                        let entry = format!("iter6d_g_{extra_label}_main");
                        Some((*minor, module, entry, *extra_label))
                    })
                    .collect();
            let mem0_extra_futures: Vec<_> = mem0_extra_modules
                .iter()
                .map(|(_, module, entry, _)| {
                    self.hal.create_compute_kernel_async(
                        "iter6d_g_mem0_extra_kernel_on_demand",
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
                                "iter6d_g_arm_kernel_on_demand",
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
                                "iter6d_g_arm_chunk1_kernel_on_demand",
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
                    "iter6d_g_on_demand arm={} chunk0 DONE",
                    label,
                ));
            }
            if let Some(result) = chunk1_result {
                let kernel = result?;
                WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(*label, kernel));
                record_witgen_replace_on_demand_kernel_compile(label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_on_demand arm={} chunk1 DONE",
                    label,
                ));
            }
            for (i, fut) in mem0_extra_futures.into_iter().enumerate() {
                let kernel = fut.await?;
                let (minor, _, _, extra_label) = &mem0_extra_modules[i];
                WITGEN_MEM0_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(*minor, kernel));
                record_witgen_replace_on_demand_kernel_compile(extra_label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_on_demand arm={} DONE",
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
            let chunk0_entry = format!("iter6d_g_{}_main", label);
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
            let chunk1_entry = format!("iter6d_g_{}_c1_main", label);
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
                        let entry = format!("iter6d_g_{extra_label}_main");
                        Some((*minor, module, entry, *extra_label))
                    })
                    .collect();
            let mem1_extra_futures: Vec<_> = mem1_extra_modules
                .iter()
                .map(|(_, module, entry, _)| {
                    self.hal.create_compute_kernel_async(
                        "iter6d_g_mem1_extra_kernel_on_demand",
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
                                "iter6d_g_arm_kernel_on_demand",
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
                                "iter6d_g_arm_chunk1_kernel_on_demand",
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
                    "iter6d_g_on_demand arm={} chunk0 DONE",
                    label,
                ));
            }
            if let Some(result) = chunk1_result {
                let kernel = result?;
                WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(*label, kernel));
                record_witgen_replace_on_demand_kernel_compile(label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_on_demand arm={} chunk1 DONE",
                    label,
                ));
            }
            for (i, fut) in mem1_extra_futures.into_iter().enumerate() {
                let kernel = fut.await?;
                let (minor, _, _, extra_label) = &mem1_extra_modules[i];
                WITGEN_MEM1_EXTRA_KERNELS.with(|cell| cell.borrow_mut().insert(*minor, kernel));
                record_witgen_replace_on_demand_kernel_compile(extra_label);
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_on_demand arm={} DONE",
                    extra_label,
                ));
            }
            return Ok(());
        }

        if !chunk0_ready {
            let wrapper = synth_arm_wrapper(label, sub_fn, arm_idx);
            let module = assemble_arm_kernel(delta, &wrapper);
            let entry = format!("iter6d_g_{}_main", label);
            let kernel = self
                .hal
                .create_compute_kernel_async(
                    "iter6d_g_arm_kernel_on_demand",
                    &module,
                    &entry,
                    &layouts,
                )
                .await?;
            WITGEN_ARM_KERNELS.with(|cell| cell.borrow_mut().insert(*label, kernel));
            record_witgen_replace_on_demand_kernel_compile(label);
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "iter6d_g_on_demand arm={} chunk0 DONE",
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
            let entry = format!("iter6d_g_{}_c1_main", label);
            let kernel = self
                .hal
                .create_compute_kernel_async(
                    "iter6d_g_arm_chunk1_kernel_on_demand",
                    &module,
                    &entry,
                    &layouts,
                )
                .await?;
            WITGEN_ARM_KERNELS_CHUNK1.with(|cell| cell.borrow_mut().insert(*label, kernel));
            record_witgen_replace_on_demand_kernel_compile(label);
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "iter6d_g_on_demand arm={} chunk1 DONE",
                label,
            ));
        }

        Ok(())
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
    /// Returns the set of arm_idx values that were actually dispatched on
    /// this call (kernel cached + cycles > 0). Caller can use this to
    /// decide whether `rust_steps` may short-circuit those arms.
    fn dispatch_witgen_per_arm_probe(
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
            "iter6d_g_per_arm_dispatch arms={} total_cycles={}",
            TOP_CHUNK0_ARM_DELTAS.len(),
            preflight.cycles.len(),
        ));
        // iter-6d-g step 6.2.1a: layout now has binding 5 (cycle_list)
        // and binding 6 (preflight_meta). Per-arm wrappers read major/
        // minor from preflight_meta via packed_minor_major at index 3.
        // step 6.2.5: binding 7 is preflight_diff_count_buf (patched
        // extern_getDiffCount reads it). step 6.2.6: bindings 8 and 9
        // are preflight_txn_start and preflight_txns_buf (patched
        // extern_getMemoryTxn reads them with a per-invocation counter).
        let layout = self.hal.create_bind_group_layout(
            "iter6d_g_arm_layout",
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
            .create_storage_buffer("iter6d_g_arm_accum_ph", placeholder_bytes)?;
        let mix_buf = self
            .hal
            .create_storage_buffer("iter6d_g_arm_mix_ph", placeholder_bytes)?;
        let params: [u32; 8] = [total_cycles, 1, total_cycles, 1, 0, 0, 0, 0];
        let params_buf = self
            .hal
            .create_uniform_buffer("iter6d_g_arm_params_ph", bytemuck::cast_slice(&params))?;
        // Upload preflight_meta for per-arm wrappers (separate buffer
        // from the shadow_init kernel's upload; could be shared in a
        // future tightening, but keeping separate avoids cross-pass
        // ownership coupling).
        let meta = build_preflight_meta(preflight);
        let meta_bytes: &[u8] = bytemuck::cast_slice(meta.as_slice());
        let preflight_buf = self
            .hal
            .create_storage_buffer("iter6d_g_arm_preflight", meta_bytes.len() as u64)?;
        self.hal
            .write_buffer_named(&preflight_buf, "iter6d_g_arm_preflight", 0, meta_bytes)?;
        let data_gpu = data
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("iter-6d-g: data missing GPU storage"))?;
        let global_gpu = global
            .buf
            .raw_buffer()
            .ok_or_else(|| anyhow::anyhow!("iter-6d-g: global missing GPU storage"))?;
        {
            let _t = WebGpuStageTimer::new(format!(
                "iter6d_g_shadow_init cycles={}",
                preflight.cycles.len()
            ));
            let kernel = SHADOW_INIT_KERNEL.with(|cell| cell.borrow().clone());
            let kernel = match kernel {
                Some(k) => k,
                None => {
                    let layout = self.hal.create_bind_group_layout(
                        "iter6d_g_shadow_init_layout",
                        &[
                            WebGpuBindingLayout::storage(0, 0),
                            WebGpuBindingLayout::uniform(1, 16),
                            WebGpuBindingLayout::read_only_storage(2, 0),
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
            let shadow_params: [u32; 4] = [total_cycles, data.cols as u32, 0, 0];
            let shadow_params_buf = self.hal.create_uniform_buffer(
                "iter6d_g_shadow_params",
                bytemuck::cast_slice(&shadow_params),
            )?;
            let shadow_layout = self.hal.create_bind_group_layout(
                "iter6d_g_shadow_init_layout",
                &[
                    WebGpuBindingLayout::storage(0, 0),
                    WebGpuBindingLayout::uniform(1, 16),
                    WebGpuBindingLayout::read_only_storage(2, 0),
                ],
            )?;
            let shadow_bind_group = self.hal.create_bind_group(
                "iter6d_g_shadow_bg",
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
                "iter6d_g_shadow_init cycles={} meta_bytes={} reused_arm_preflight=true",
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
            .create_storage_buffer("iter6d_g_arm_diff_count", diff_count_bytes.len() as u64)?;
        self.hal.write_buffer_named(
            &diff_count_buf,
            "iter6d_g_arm_diff_count",
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
            .create_storage_buffer("iter6d_g_arm_txn_start", txn_start_bytes.len() as u64)?;
        self.hal.write_buffer_named(
            &txn_start_buf,
            "iter6d_g_arm_txn_start",
            0,
            txn_start_bytes,
        )?;
        let txns_bytes: &[u8] = bytemuck::cast_slice(txns.as_slice());
        let txns_buf = self
            .hal
            .create_storage_buffer("iter6d_g_arm_txns", txns_bytes.len() as u64)?;
        self.hal
            .write_buffer_named(&txns_buf, "iter6d_g_arm_txns", 0, txns_bytes)?;
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
            // iter-6d-g step 6.2.4: for short-circuit safety the arm's
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
                .create_storage_buffer("iter6d_g_arm_cycle_list", cycle_bytes.len() as u64)?;
            self.hal
                .write_buffer_named(&cycle_buf, "iter6d_g_arm_cycle_list", 0, cycle_bytes)?;
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
                    WebGpuBufferBinding::new(7, &diff_count_buf),
                    WebGpuBufferBinding::new(8, &txn_start_buf),
                    WebGpuBufferBinding::new(9, &txns_buf),
                ],
            )?;
            let workgroups = (cycle_count as u32).div_ceil(64);
            self.hal
                .dispatch_compute_1d(&kernel, &bind_group, workgroups);
            // iter-6d-g step 6.2.4: dispatch chunk1 too if we have it
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
                    "iter6d_g_arm_minor_cycle_list",
                    filtered_cycle_bytes.len() as u64,
                )?;
                self.hal.write_buffer_named(
                    &filtered_cycle_buf,
                    "iter6d_g_arm_minor_cycle_list",
                    0,
                    filtered_cycle_bytes,
                )?;
                let filtered_bind_group = self.hal.create_bind_group(
                    "iter6d_g_arm_minor_bg",
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
                    "iter6d_g_minor_dispatch arm={} minor={} cycles={}",
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
            "iter6d_g_per_arm_dispatch dispatched={} skipped={}",
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

    /// Returns the number of TopChunk{0,1} kernels actually dispatched
    /// (0 = neither ready, 1 = only one ready, 2 = both ready). step_witgen
    /// gates the replace short-circuit on this returning 2 -- only then do
    /// we trust that GPU has written cells for both first-cycle (chunk0)
    /// and non-first-cycle (chunk1) arm dispatches.
    fn dispatch_witgen_top_chunk0_probe(
        &self,
        data: &MetaBuffer<WebGpuHal>,
        global: &MetaBuffer<WebGpuHal>,
        total_cycles: u32,
    ) -> Result<usize> {
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
            return Ok(0);
        }
        let chunks_ready = kernels.len();
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
            self.hal
                .dispatch_compute_1d(kernel, &bind_group, workgroups);
        }
        Ok(chunks_ready)
    }

    fn lookup_topaccum_arm5_probe_kernel(&self) -> Result<WebGpuKernel> {
        if let Some(kernel) = TOPACCUM_ARM5_PROBE_KERNEL.with(|cell| cell.borrow().clone()) {
            return Ok(kernel);
        }
        use crate::prove::wgsl_pruner::{
            TOPACCUM_ARM5_CYCLE_LIST_ENTRY, TOPACCUM_ARM5_PROBE_WGSL, WITGEN_BASELINE_WGSL,
        };
        let layout = self.hal.create_bind_group_layout(
            "rv32im_accum_topaccum_arm5_probe_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
                WebGpuBindingLayout::read_only_storage(5, 0),
            ],
        )?;
        let mut module = String::with_capacity(
            WITGEN_BASELINE_WGSL.len()
                + TOPACCUM_ARM5_PROBE_WGSL.len()
                + TOPACCUM_ARM5_CYCLE_LIST_ENTRY.len()
                + 2,
        );
        module.push_str(WITGEN_BASELINE_WGSL);
        if !module.ends_with('\n') {
            module.push('\n');
        }
        module.push_str(TOPACCUM_ARM5_PROBE_WGSL);
        if !module.ends_with('\n') {
            module.push('\n');
        }
        module.push_str(TOPACCUM_ARM5_CYCLE_LIST_ENTRY);
        let kernel = self.hal.create_compute_kernel(
            "rv32im_accum_topaccum_arm5_probe",
            &module,
            "topaccum_arm5_cycle_list_main",
            &[layout],
        )?;
        TOPACCUM_ARM5_PROBE_KERNEL.with(|cell| *cell.borrow_mut() = Some(kernel.clone()));
        Ok(kernel)
    }

    fn lookup_topaccum_arm5_split_inv_probe_kernels(
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

        use crate::prove::wgsl_pruner::{TOPACCUM_ARM5_PROBE_WGSL, WITGEN_BASELINE_WGSL};
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
        let (capture_body, capture_inv_count) = topaccum_arm5_replace_ext_inv_calls(
            TOPACCUM_ARM5_PROBE_WGSL,
            "topaccum_arm5_capture_inv",
        )?;
        let (consume_body, consume_inv_count) = topaccum_arm5_replace_ext_inv_calls(
            TOPACCUM_ARM5_PROBE_WGSL,
            "topaccum_arm5_consume_inv",
        )?;
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

    fn lookup_topaccum_arm5_compare_kernel(&self) -> Result<WebGpuKernel> {
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

    fn dispatch_topaccum_arm5_real_buffer_probe(
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

    fn dispatch_topaccum_arm5_authoritative(
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

    fn lookup_accum_misc0_direct_kernel(&self) -> Result<WebGpuKernel> {
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

    fn lookup_accum_misc2_direct_kernel(&self) -> Result<WebGpuKernel> {
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

    fn lookup_accum_misc1_direct_kernel(&self) -> Result<WebGpuKernel> {
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

    fn lookup_accum_mem0_direct_kernel(&self) -> Result<WebGpuKernel> {
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

    fn lookup_accum_mem1_direct_kernel(&self) -> Result<WebGpuKernel> {
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

    fn lookup_accum_control0_direct_kernel(&self) -> Result<WebGpuKernel> {
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

    fn lookup_accum_misc_direct_kernel(&self, kind: AccumMiscDirectKind) -> Result<WebGpuKernel> {
        match kind {
            AccumMiscDirectKind::Misc0 => self.lookup_accum_misc0_direct_kernel(),
            AccumMiscDirectKind::Misc1 => self.lookup_accum_misc1_direct_kernel(),
            AccumMiscDirectKind::Misc2 => self.lookup_accum_misc2_direct_kernel(),
            AccumMiscDirectKind::Mem0 => self.lookup_accum_mem0_direct_kernel(),
            AccumMiscDirectKind::Mem1 => self.lookup_accum_mem1_direct_kernel(),
            AccumMiscDirectKind::Control0 => self.lookup_accum_control0_direct_kernel(),
        }
    }

    fn dispatch_accum_misc_direct_grouped(
        &self,
        misc0_rows: &[u32],
        misc1_rows: &[u32],
        misc2_rows: &[u32],
        mem0_rows: &[u32],
        mem1_rows: &[u32],
        control0_rows: &[u32],
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

impl WebGpuCircuitHal {
    fn witgen_replace_shadow_rows(
        preflight: &PreflightTrace,
        mask: u16,
    ) -> (Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>) {
        let arithmetic_rows = Vec::new();
        let bitwise_rows = Vec::new();
        let mut misc2_compare_rows = Vec::new();
        let mut misc2_branch_rows = Vec::new();
        for (cycle_idx, cycle) in preflight.cycles.iter().enumerate() {
            match cycle.major {
                // MISC0 lookup-table side effects are replayed directly from
                // preflight in rust_steps, so the CPU shadow does not need the
                // GPU-authored arithmetic/bitwise rows before generate_witness.
                0 if (mask & 0x0001) != 0 => {}
                2 if (mask & (1u16 << 2)) != 0 => match cycle.minor {
                    0 | 2 => misc2_compare_rows.push(cycle_idx as u32),
                    3 | 4 | 5 | 6 | 7 => misc2_branch_rows.push(cycle_idx as u32),
                    _ => {}
                },
                _ => {}
            }
        }
        (
            arithmetic_rows,
            bitwise_rows,
            misc2_compare_rows,
            misc2_branch_rows,
        )
    }

    fn witgen_replace_accum_shadow_rows_by_minor(
        preflight: &PreflightTrace,
        mask: u16,
    ) -> ([Vec<u32>; 8], Vec<u32>, Vec<u32>, Vec<u32>) {
        let mut misc0_rows: [Vec<u32>; 8] = std::array::from_fn(|_| Vec::new());
        let mut misc2_compare_rows = Vec::new();
        let mut misc2_branch_rows = Vec::new();
        let mut mem0_rows = Vec::new();
        for (cycle_idx, cycle) in preflight.cycles.iter().enumerate() {
            match cycle.major {
                0 if (mask & 0x0001) != 0 => match cycle.minor {
                    0 | 1 | 2 | 3 | 4 | 7 => {
                        misc0_rows[cycle.minor as usize].push(cycle_idx as u32)
                    }
                    _ => {}
                },
                2 if (mask & (1u16 << 2)) != 0 => match cycle.minor {
                    0 | 2 => misc2_compare_rows.push(cycle_idx as u32),
                    3 | 4 | 5 | 6 | 7 => misc2_branch_rows.push(cycle_idx as u32),
                    _ => {}
                },
                5 if (mask & (1u16 << 5)) != 0
                    && witgen_mem0_replace_minor_enabled(cycle.minor) =>
                {
                    mem0_rows.push(cycle_idx as u32);
                }
                _ => {}
            }
        }
        (misc0_rows, misc2_compare_rows, misc2_branch_rows, mem0_rows)
    }

    fn top_accum_major0_shadow_columns(prefix: usize, data_cols: usize) -> Vec<u32> {
        let capped = prefix.min(data_cols);
        (0..capped)
            .filter(|&col| col <= 1 || col >= 14)
            .map(|col| col as u32)
            .collect()
    }

    fn prefix_shadow_columns(prefix: usize, data_cols: usize) -> Vec<u32> {
        (0..prefix.min(data_cols)).map(|col| col as u32).collect()
    }

    fn push_shadow_column(columns: &mut Vec<u32>, data_cols: usize, col: u32) {
        if (col as usize) < data_cols {
            columns.push(col);
        }
    }

    fn push_shadow_column_range(
        columns: &mut Vec<u32>,
        data_cols: usize,
        start: u32,
        end_inclusive: u32,
    ) {
        for col in start..=end_inclusive {
            Self::push_shadow_column(columns, data_cols, col);
        }
    }

    fn misc0_accum_shadow_columns(minor: u8, data_cols: usize) -> Vec<u32> {
        let mut columns = Vec::with_capacity(160);
        for col in [0, 1, 14, 15, 17, 18, 21] {
            Self::push_shadow_column(&mut columns, data_cols, col);
        }
        match minor {
            1 => Self::push_shadow_column(&mut columns, data_cols, 22),
            2 => Self::push_shadow_column_range(&mut columns, data_cols, 22, 23),
            3 => Self::push_shadow_column_range(&mut columns, data_cols, 22, 24),
            4 => Self::push_shadow_column_range(&mut columns, data_cols, 22, 25),
            7 => Self::push_shadow_column_range(&mut columns, data_cols, 22, 28),
            _ => {}
        }
        Self::push_shadow_column_range(&mut columns, data_cols, 29, 40);
        Self::push_shadow_column_range(&mut columns, data_cols, 42, 43);
        Self::push_shadow_column_range(&mut columns, data_cols, 45, 46);
        Self::push_shadow_column_range(&mut columns, data_cols, 48, 49);
        Self::push_shadow_column_range(&mut columns, data_cols, 54, 75);
        Self::push_shadow_column_range(&mut columns, data_cols, 86, 87);
        Self::push_shadow_column_range(&mut columns, data_cols, 90, 125);
        Self::push_shadow_column_range(&mut columns, data_cols, 128, 131);
        if matches!(minor, 2 | 3 | 4) {
            Self::push_shadow_column_range(&mut columns, data_cols, 132, 195);
        }
        columns
    }

    async fn sync_witgen_replace_shadow_rows(
        &self,
        data: &MetaBuffer<WebGpuHal>,
        arithmetic_rows: &[u32],
        bitwise_rows: &[u32],
        misc2_compare_rows: &[u32],
        misc2_branch_rows: &[u32],
        source: &'static str,
    ) -> Result<()> {
        // The authoritative slices are MISC0 arithmetic/bitwise ops and
        // MISC2 rows where rust_steps short-circuits CPU writes. Keep the
        // row classes separate so branch-like MISC2 rows do not pay for the
        // signed-compare aux columns, and minor 1 remains CPU-owned.
        // Diff evidence shows `replace_cpu_only_nonzero=0` for this slice:
        // all nonzero CPU cells are already written by the GPU. The arithmetic
        // subset lives within [0, 132); bitwise rows additionally write their
        // shared ToBits_16 decomposition columns through [132, 196). Keep the
        // row groups separate so common ADD/SUB/ADDI rows do not pay for the
        // wider bitwise shadow readback. For MISC0 accum shadow repair, column
        // 1 is the active major-0 selector; columns 2..13 are inactive
        // top-level selectors and are not read once the major-0 branch is
        // selected. Omitting them avoids transferring 12 dead cells per
        // GPU-owned row while preserving the current CPU TopAccum path.
        const MISC0_ARITH_SHADOW_COLS: usize = 132;
        const MISC0_BITWISE_SHADOW_COLS: usize = 196;
        // MISC2 branch-like rows (minors 3..=7) need MiscInput + source-reg
        // fields through column 131. Compare rows (minors 0 and 2) also need
        // NormalizeU32 carries plus signed compare aux cells through column
        // 138.
        const MISC2_BRANCH_SHADOW_COLS: usize = 132;
        const MISC2_COMPARE_SHADOW_COLS: usize = 139;
        let misc0_arith_columns =
            Self::top_accum_major0_shadow_columns(MISC0_ARITH_SHADOW_COLS, data.cols);
        let misc0_bitwise_columns =
            Self::top_accum_major0_shadow_columns(MISC0_BITWISE_SHADOW_COLS, data.cols);
        let misc2_compare_columns =
            Self::prefix_shadow_columns(MISC2_COMPARE_SHADOW_COLS, data.cols);
        let misc2_branch_columns = Self::prefix_shadow_columns(MISC2_BRANCH_SHADOW_COLS, data.cols);
        let groups = [
            (misc0_arith_columns.as_slice(), arithmetic_rows),
            (misc0_bitwise_columns.as_slice(), bitwise_rows),
            (misc2_compare_columns.as_slice(), misc2_compare_rows),
            (misc2_branch_columns.as_slice(), misc2_branch_rows),
        ];
        if groups
            .iter()
            .all(|(columns, rows)| columns.is_empty() || rows.is_empty())
        {
            // The subsequent synchronous rust_steps pass only needs the CPU
            // shadow for non-replaced rows. Keep the mixed CPU/GPU ownership
            // intentional: MISC0 cells remain GPU-owned, CPU-written cells are
            // uploaded sparsely by eltwise_zeroize_elem before commitment.
            data.buf
                .sync_gpu_ranges_to_cpu_unchecked(self.hal.as_ref(), &[], source)
                .await?;
        } else {
            data.buf
                .sync_gpu_column_set_row_groups_to_cpu_unchecked(
                    self.hal.as_ref(),
                    data.rows,
                    groups.as_slice(),
                    source,
                )
                .await?;
        }
        Ok(())
    }

    async fn sync_witgen_replace_accum_shadow_rows(
        &self,
        preflight: &PreflightTrace,
        data: &MetaBuffer<WebGpuHal>,
    ) -> Result<usize> {
        let mask = super::rust_steps::witgen_gpu_replace_arm_mask();
        if mask == 0 {
            return Ok(0);
        }
        let (misc0_rows, misc2_compare_rows, misc2_branch_rows, mem0_rows) =
            Self::witgen_replace_accum_shadow_rows_by_minor(preflight, mask);
        let gpu_direct_misc0 =
            ACCUM_GPU_MISC0_DIRECT_ENABLED.load(Ordering::SeqCst) && (mask & 0x0001) != 0;
        let gpu_direct_mem0 =
            ACCUM_GPU_MEM0_DIRECT_ENABLED.load(Ordering::SeqCst) && (mask & (1u16 << 5)) != 0;
        let misc0_read_rows = if gpu_direct_misc0 {
            0
        } else {
            [0usize, 1, 2, 3, 4, 7]
                .into_iter()
                .try_fold(0usize, |total, minor| {
                    total.checked_add(misc0_rows[minor].len())
                })
                .ok_or_else(|| anyhow::anyhow!("witgen accum shadow row count overflow"))?
        };
        let mem0_read_rows = if gpu_direct_mem0 { 0 } else { mem0_rows.len() };
        let rows = misc0_read_rows
            .checked_add(misc2_compare_rows.len())
            .and_then(|n| n.checked_add(misc2_branch_rows.len()))
            .and_then(|n| n.checked_add(mem0_read_rows))
            .ok_or_else(|| anyhow::anyhow!("witgen accum shadow row count overflow"))?;
        if rows == 0 {
            return Ok(0);
        }
        // Accumulation only reads a sparse subset of each MISC0 minor's GPU
        // witness cells. Keep these row classes exact instead of using the old
        // broad arithmetic/bitwise prefixes; xgboost evidence shows the common
        // rows need 90-97 columns and bitwise rows need 153-155 columns, not
        // the previous 120/184 column transfer.
        let misc0_columns: [Vec<u32>; 8] =
            std::array::from_fn(|minor| Self::misc0_accum_shadow_columns(minor as u8, data.cols));
        let misc2_compare_columns = Self::prefix_shadow_columns(139, data.cols);
        let misc2_branch_columns = Self::prefix_shadow_columns(132, data.cols);
        let mem0_columns = Self::prefix_shadow_columns(data.cols, data.cols);
        let empty_rows: &[u32] = &[];
        let groups = [
            (
                misc0_columns[0].as_slice(),
                if gpu_direct_misc0 {
                    empty_rows
                } else {
                    misc0_rows[0].as_slice()
                },
            ),
            (
                misc0_columns[1].as_slice(),
                if gpu_direct_misc0 {
                    empty_rows
                } else {
                    misc0_rows[1].as_slice()
                },
            ),
            (
                misc0_columns[2].as_slice(),
                if gpu_direct_misc0 {
                    empty_rows
                } else {
                    misc0_rows[2].as_slice()
                },
            ),
            (
                misc0_columns[3].as_slice(),
                if gpu_direct_misc0 {
                    empty_rows
                } else {
                    misc0_rows[3].as_slice()
                },
            ),
            (
                misc0_columns[4].as_slice(),
                if gpu_direct_misc0 {
                    empty_rows
                } else {
                    misc0_rows[4].as_slice()
                },
            ),
            (
                misc0_columns[7].as_slice(),
                if gpu_direct_misc0 {
                    empty_rows
                } else {
                    misc0_rows[7].as_slice()
                },
            ),
            (
                misc2_compare_columns.as_slice(),
                misc2_compare_rows.as_slice(),
            ),
            (
                misc2_branch_columns.as_slice(),
                misc2_branch_rows.as_slice(),
            ),
            (
                mem0_columns.as_slice(),
                if gpu_direct_mem0 {
                    empty_rows
                } else {
                    mem0_rows.as_slice()
                },
            ),
        ];
        data.buf
            .sync_gpu_column_set_row_groups_to_cpu_unchecked(
                self.hal.as_ref(),
                data.rows,
                groups.as_slice(),
                "witgen_accum_shadow_rows",
            )
            .await?;
        Ok(rows)
    }

    /// SP7 iter 6d-g step 6.2.8 (2026-05-16): async pre-dispatch hook.
    /// Does the iter-6d-g GPU work (shadow_init + per-arm chunks) and then
    /// `sync_gpu_to_cpu` on the data buffer so rust_steps' subsequent
    /// view_mut sees GPU writes in the CPU shadow. Sets the short-circuit
    /// mask so rust_steps skips arms covered by GPU dispatch.
    ///
    /// Called from `prove_core_async` BEFORE `WitnessGenerator::populate_from_parts`
    /// (which runs the sync `generate_witness` / rust_steps path). No-op when
    /// the probe flag is off (legacy sync path still works).
    pub async fn pre_witgen_dispatch_async(
        &self,
        preflight: &PreflightTrace,
        data: &MetaBuffer<WebGpuHal>,
        global: &MetaBuffer<WebGpuHal>,
    ) -> Result<()> {
        let diff_mode = WITGEN_GPU_DIFF_ENABLED.load(Ordering::SeqCst);
        if !WITGEN_GPU_PROBE_ENABLED.load(Ordering::SeqCst) && !diff_mode {
            super::rust_steps::set_witgen_gpu_replace_arm_mask(0);
            return Ok(());
        }
        // SP7 iter 6d-g step 6.2.11 (2026-05-16): only run the GPU
        // dispatches + sync if we're actually going to short-circuit
        // (replace flag on). For probe-only mode, the dispatches'
        // partial cell writes would conflict with rust_steps' full
        // writes via set_at's "inconsistent set" check. Probe-only
        // exists to measure dispatch cost; with the async refactor it's
        // moot since we'd be syncing back values that rust_steps
        // overwrites anyway.
        //
        // SP7 iter 6d-g step 6.2.13 (2026-05-16): diff mode bypasses
        // the replace-off gate so we can run GPU writes for snapshot
        // purposes even when not short-circuiting. Caller in
        // `prove_core_async` will reset the CPU shadow + re-scatter
        // injector + force mask=0 after snapshot to keep rust_steps'
        // set_at consistency check satisfied.
        let replace_enabled = WITGEN_GPU_REPLACE_ENABLED.load(Ordering::SeqCst);
        if !replace_enabled && !diff_mode {
            super::rust_steps::set_witgen_gpu_replace_arm_mask(0);
            return Ok(());
        }
        let replace_mode = replace_enabled;
        let _timer = WebGpuStageTimer::new(format!(
            "iter6d_g_pre_witgen_dispatch_async cycles={}",
            preflight.cycles.len()
        ));

        let diff_only_mode = diff_mode && !replace_mode;
        let nonblocking_pending = WITGEN_GPU_REPLACE_NONBLOCKING_PENDING_ENABLED
            .load(Ordering::SeqCst)
            && replace_mode
            && !diff_only_mode;
        let mut needed_replacement_arm = false;
        if replace_mode || diff_mode {
            let mut needed_arms = [false; 13];
            for cycle in &preflight.cycles {
                let arm = cycle.major as usize;
                let needed = if diff_only_mode {
                    is_witgen_diff_cycle(cycle.major, cycle.minor)
                } else {
                    is_witgen_replace_cycle(cycle.major, cycle.minor)
                        && is_witgen_replace_supported_arm(arm)
                };
                if arm < needed_arms.len() && needed {
                    needed_arms[arm] = true;
                }
            }
            for (arm_idx, needed) in needed_arms.iter().enumerate() {
                if *needed {
                    if replace_mode {
                        needed_replacement_arm = true;
                    }
                    if nonblocking_pending {
                        continue;
                    }
                    if let Err(err) = self
                        .ensure_witgen_replace_arm_ready_async(arm_idx, preflight, diff_only_mode)
                        .await
                    {
                        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                            "iter6d_g_on_demand arm{arm_idx} FAILED err={err:?}"
                        ));
                    }
                }
            }
        }

        let desired_mask = if replace_mode {
            self.ready_witgen_replace_mask(preflight) & witgen_replace_supported_arm_mask()
        } else {
            0
        };
        if replace_mode && desired_mask == 0 {
            super::rust_steps::set_witgen_gpu_replace_arm_mask(0);
            if nonblocking_pending && needed_replacement_arm {
                WITGEN_GPU_REPLACE_NONBLOCKING_PENDING_SKIPS.fetch_add(1, Ordering::SeqCst);
                risc0_zkp::hal::webgpu::log_webgpu_metric(
                    "iter6d_g_pre_witgen_dispatch_async nonblocking_pending_skip",
                );
            }
            risc0_zkp::hal::webgpu::log_webgpu_metric(
                "iter6d_g_pre_witgen_dispatch_async mask=0x0000 no_ready_replacement no_sync",
            );
            return Ok(());
        }

        // Keep unwritten GPU witness cells at INVALID. In replacement mode
        // seed the browser buffer directly with INVALID; uploading the full
        // CPU shadow here costs one full data matrix per segment before any
        // GPU witgen work can run.
        let seeded_on_gpu = if replace_mode && !diff_mode {
            self.hal.init_invalid_elem(&data.buf).unwrap_or_else(|err| {
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_gpu_seed FAILED err={err:?}"
                ));
                false
            })
        } else {
            false
        };
        if seeded_on_gpu {
            risc0_zkp::hal::webgpu::log_webgpu_metric("iter6d_g_gpu_seed invalid_fill");
        } else {
            data.buf.sync_cpu_to_gpu(self.hal.as_ref())?;
        }

        // 1. Per-arm chunk0+chunk1 dispatches. Returns the arm_idx values
        //    actually dispatched (kernel ready + cycles > 0).
        //    The dispatch path also runs shadow_init using the same uploaded
        //    preflight metadata buffer.
        let dispatched_arms = match self.dispatch_witgen_per_arm_probe(
            data,
            global,
            preflight,
            replace_mode.then_some(desired_mask),
            diff_only_mode,
        ) {
            Ok(arms) => arms,
            Err(err) => {
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "iter6d_g_per_arm_dispatch FAILED err={err:?}"
                ));
                Vec::new()
            }
        };
        // 2. Compute the short-circuit mask (MISC0-only for the initial
        //    bring-up; expand once verify passes).
        let mut mask: u16 = 0;
        if replace_mode {
            for arm_idx in &dispatched_arms {
                if is_zero_back_reg_arm(*arm_idx) && *arm_idx < 13 {
                    mask |= 1u16 << arm_idx;
                }
            }
            mask &= desired_mask;
        }
        super::rust_steps::set_witgen_gpu_replace_arm_mask(mask);
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "iter6d_g_pre_witgen_dispatch_async mask=0x{:04x} dispatched_arms={:?}",
            mask, dispatched_arms,
        ));
        if replace_mode && mask == 0 {
            return Ok(());
        }
        let (arithmetic_rows, bitwise_rows, misc2_compare_rows, misc2_branch_rows) =
            if replace_mode && !diff_mode {
                Self::witgen_replace_shadow_rows(preflight, mask)
            } else {
                (Vec::new(), Vec::new(), Vec::new(), Vec::new())
            };
        // 3. Mark GPU writes as authoritative + sync GPU -> CPU shadow.
        //    In normal replacement mode, repair only the columns needed by
        //    the current MISC0/ADD slice. Explicit diff modes keep the full
        //    unchecked readback so diagnostics still compare every cell.
        data.buf.mark_gpu_dirty();
        if replace_mode && !diff_mode {
            self.sync_witgen_replace_shadow_rows(
                data,
                arithmetic_rows.as_slice(),
                bitwise_rows.as_slice(),
                misc2_compare_rows.as_slice(),
                misc2_branch_rows.as_slice(),
                "witgen_data_shadow_rows",
            )
            .await?;
        } else {
            data.buf
                .sync_gpu_to_cpu_unchecked(self.hal.as_ref())
                .await?;
        }
        Ok(())
    }

    pub async fn post_accum_candidate_sync_async(&self) -> Result<()> {
        if !ACCUM_GPU_CANDIDATE_SYNC_ENABLED.load(Ordering::SeqCst) {
            return Ok(());
        }
        let _timer = WebGpuStageTimer::new_active_for(
            "rv32im_accumulate candidate_sync_wait",
            self.hal.as_ref(),
        );
        self.hal
            .wait_idle()
            .await
            .context("TopAccum candidate sync wait failed")?;
        ACCUM_GPU_CANDIDATE_SYNC_WAITS.fetch_add(1, Ordering::SeqCst);
        risc0_zkp::hal::webgpu::log_webgpu_metric(
            "rv32im_accumulate candidate_sync_wait completed",
        );
        Ok(())
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
        // SP7 iter 6d-g step 6.2.8 (2026-05-16): GPU dispatches now happen
        // in `pre_witgen_dispatch_async` (called from the async prove path
        // BEFORE this sync `generate_witness`). The mask and CPU shadow
        // are already in sync by the time we get here, so rust_steps
        // reads correct values for short-circuited cycles.
        //
        // For the sync prove path (no pre_dispatch hook), mask was reset
        // to 0 by pre_witgen_dispatch_async's no-op branch, OR if that
        // wasn't called, the legacy state stands (still 0 by default).
        super::rust_steps::with_eqz_elided(|| {
            super::rust_steps::generate_witness(mode, preflight, global, data)
        })
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
        log_accum_major_histogram(preflight);
        let scopes = crate::prove::webgpu_async_authoritative_scopes();
        if scopes.accum_make_coeffs && scopes.accum_poly_group && scopes.accum_merkle {
            let mask = super::rust_steps::witgen_gpu_replace_arm_mask();
            let misc0_direct_enabled = ACCUM_GPU_MISC0_DIRECT_ENABLED.load(Ordering::SeqCst);
            let mem0_direct_enabled = ACCUM_GPU_MEM0_DIRECT_ENABLED.load(Ordering::SeqCst);
            let direct_row_mask = mask
                | if misc0_direct_enabled { 0x0001 } else { 0 }
                | if mem0_direct_enabled { 1u16 << 5 } else { 0 };
            let (misc0_rows_by_minor, _, _, mem0_rows_by_minor) =
                Self::witgen_replace_accum_shadow_rows_by_minor(preflight, direct_row_mask);
            let misc0_rows = if misc0_direct_enabled {
                [0usize, 1, 2, 3, 4, 7]
                    .into_iter()
                    .flat_map(|minor| misc0_rows_by_minor[minor].iter().copied())
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let misc1_rows = if ACCUM_GPU_MISC1_DIRECT_ENABLED.load(Ordering::SeqCst) {
                preflight
                    .cycles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, cycle)| {
                        (cycle.major == 1).then(|| {
                            u32::try_from(idx)
                                .context("MISC1 direct accumulator cycle index exceeds u32")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            let misc2_rows = if ACCUM_GPU_MISC2_DIRECT_ENABLED.load(Ordering::SeqCst) {
                preflight
                    .cycles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, cycle)| {
                        (cycle.major == 2).then(|| {
                            u32::try_from(idx)
                                .context("MISC2 direct accumulator cycle index exceeds u32")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            let mem0_rows = if mem0_direct_enabled {
                mem0_rows_by_minor
            } else {
                Vec::new()
            };
            let mem1_rows = if ACCUM_GPU_MEM1_DIRECT_ENABLED.load(Ordering::SeqCst) {
                preflight
                    .cycles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, cycle)| {
                        (cycle.major == 6).then(|| {
                            u32::try_from(idx)
                                .context("MEM1 direct accumulator cycle index exceeds u32")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            let control0_rows = if ACCUM_GPU_CONTROL0_DIRECT_ENABLED.load(Ordering::SeqCst) {
                preflight
                    .cycles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, cycle)| {
                        (cycle.major == 7).then(|| {
                            u32::try_from(idx)
                                .context("CONTROL0 direct accumulator cycle index exceeds u32")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            if !misc0_rows.is_empty()
                || !misc1_rows.is_empty()
                || !misc2_rows.is_empty()
                || !mem0_rows.is_empty()
                || !mem1_rows.is_empty()
                || !control0_rows.is_empty()
            {
                let direct_major_mask = if misc1_rows.is_empty() { 0 } else { 1u16 << 1 }
                    | if misc2_rows.is_empty() { 0 } else { 1u16 << 2 }
                    | if mem1_rows.is_empty() { 0 } else { 1u16 << 6 }
                    | if control0_rows.is_empty() {
                        0
                    } else {
                        1u16 << 7
                    };
                super::rust_steps::with_accum_eqz_elided(|| {
                    super::rust_steps::step_accum_without_selected_majors_or_postprocess(
                        preflight,
                        data,
                        accum,
                        global,
                        mix,
                        !misc0_rows.is_empty(),
                        !mem0_rows.is_empty(),
                        direct_major_mask,
                    )
                })?;
                anyhow::ensure!(
                    self.dispatch_accum_misc_direct_grouped(
                        &misc0_rows,
                        &misc1_rows,
                        &misc2_rows,
                        &mem0_rows,
                        &mem1_rows,
                        &control0_rows,
                        data,
                        accum,
                        mix
                    )?,
                    "RV32IM grouped MISC direct accumulator GPU dispatch unavailable"
                );
                {
                    let _timer = WebGpuStageTimer::new_active_for(
                        "rv32im_accumulate terminal_ext_prefix_gpu",
                        self.hal.as_ref(),
                    );
                    anyhow::ensure!(
                        dispatch_accum_terminal_ext_prefix(
                            self.hal.as_ref(),
                            &accum.buf,
                            accum.rows,
                            accum.cols,
                        )?,
                        "RV32IM accum terminal prefix GPU dispatch unavailable in MISC0 direct mode"
                    );
                }
                let split = LAYOUT_TOP_ACCUM.columns[0].offset;
                {
                    let _timer = WebGpuStageTimer::new_active_for(
                        "rv32im_accumulate machine_column_carry_gpu",
                        self.hal.as_ref(),
                    );
                    anyhow::ensure!(
                        dispatch_accum_machine_column_carry(
                            self.hal.as_ref(),
                            &accum.buf,
                            accum.rows,
                            accum.cols,
                            split,
                        )?,
                        "RV32IM accum machine-column carry GPU dispatch unavailable in MISC0 direct mode"
                    );
                }
                return Ok(());
            }
            if ACCUM_GPU_ARM5_AUTHORITATIVE_ENABLED.load(Ordering::SeqCst) {
                let arm5_cycles = preflight
                    .cycles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, cycle)| {
                        (cycle.major == 5).then(|| {
                            u32::try_from(idx)
                                .context("TopAccum arm5 authoritative cycle index exceeds u32")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                if !arm5_cycles.is_empty() {
                    super::rust_steps::with_accum_eqz_elided(|| {
                        super::rust_steps::step_accum_without_major_or_postprocess(
                            preflight, data, accum, global, mix, 5,
                        )
                    })?;
                    self.dispatch_topaccum_arm5_authoritative(
                        &arm5_cycles,
                        data,
                        accum,
                        global,
                        mix,
                    )?;
                    {
                        let _timer = WebGpuStageTimer::new_active_for(
                            "rv32im_accumulate terminal_ext_prefix_gpu",
                            self.hal.as_ref(),
                        );
                        anyhow::ensure!(
                            dispatch_accum_terminal_ext_prefix(
                                self.hal.as_ref(),
                                &accum.buf,
                                accum.rows,
                                accum.cols,
                            )?,
                            "RV32IM accum terminal prefix GPU dispatch unavailable in authoritative arm5 mode"
                        );
                    }
                    let split = LAYOUT_TOP_ACCUM.columns[0].offset;
                    {
                        let _timer = WebGpuStageTimer::new_active_for(
                            "rv32im_accumulate machine_column_carry_gpu",
                            self.hal.as_ref(),
                        );
                        anyhow::ensure!(
                            dispatch_accum_machine_column_carry(
                                self.hal.as_ref(),
                                &accum.buf,
                                accum.rows,
                                accum.cols,
                                split,
                            )?,
                            "RV32IM accum machine-column carry GPU dispatch unavailable in authoritative arm5 mode"
                        );
                    }
                    return Ok(());
                }
            }
            super::rust_steps::step_accum_without_machine_column_carry(
                preflight, data, accum, global, mix,
            )?;
            self.dispatch_topaccum_arm5_real_buffer_probe(preflight, data, accum, global, mix)?;
            let split = LAYOUT_TOP_ACCUM.columns[0].offset;
            {
                let _timer = WebGpuStageTimer::new_active_for(
                    "rv32im_accumulate machine_column_carry_gpu",
                    self.hal.as_ref(),
                );
                if dispatch_accum_machine_column_carry(
                    self.hal.as_ref(),
                    &accum.buf,
                    accum.rows,
                    accum.cols,
                    split,
                )? {
                    return Ok(());
                }
            }
            super::rust_steps::finish_accum_machine_column_carry(accum);
            return Ok(());
        }
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
            // SP7 iter 6d-g step 6.2.8 (2026-05-16): split WitnessGenerator
            // construction so we can insert an ASYNC pre-dispatch hook
            // (GPU shadow_init + per-arm chunks + sync_gpu_to_cpu) BEFORE
            // sync `generate_witness` runs. This is the fix for the
            // CPU/GPU shadow desync blocker that step 6.2.7 identified.
            let (global_vec, injector, cycles, trace, _po2_inner) =
                super::super::witgen::WitnessGenerator::<WebGpuHal>::preflight_components(
                    preflight_results,
                );
            let (global_buf, code_buf, data_buf) = super::super::witgen::WitnessGenerator::<
                WebGpuHal,
            >::allocate_buffers(
                hal, &global_vec, cycles, &injector
            );
            circuit_hal
                .pre_witgen_dispatch_async(&trace, &data_buf, &global_buf)
                .await?;
            // SP7 iter 6d-g step 6.2.13 (2026-05-16): if diff mode is on,
            // snapshot GPU result from CPU shadow, reset shadow to
            // INVALID, re-scatter injector, force mask=0 so rust_steps
            // can write cleanly, then diff BEFORE zeroize.
            //
            // SP7 iter 6d-g step 6.2.14 (2026-05-16): snapshot AFTER
            // generate_witness but BEFORE eltwise_zeroize_elem so we can
            // distinguish "INVALID (not written)" from "0 (actually
            // written zero)". The 6.2.13 filter conflated both, missing
            // mismatches where GPU wrote V != 0 but rust_steps wrote 0.
            let diff_mode = WITGEN_GPU_DIFF_ENABLED.load(Ordering::SeqCst);
            let replace_diff_mode = diff_mode && WITGEN_GPU_REPLACE_ENABLED.load(Ordering::SeqCst);
            let witgen = if replace_diff_mode {
                let replace_diff_segment =
                    WITGEN_GPU_REPLACE_DIFF_SEEN_SEGMENTS.fetch_add(1, Ordering::SeqCst);
                let replace_diff_target =
                    WITGEN_GPU_REPLACE_DIFF_TARGET_SEGMENT.load(Ordering::SeqCst);
                circuit_hal
                    .generate_witness(mode, &trace, &global_buf, &data_buf)
                    .context("witness generation failure (REPLACE DIFF mode)")?;
                if replace_diff_segment != replace_diff_target {
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "REPLACE_FINAL_SKIP segment_ord={} target={}",
                        replace_diff_segment, replace_diff_target
                    ));
                    finish_witgen_from_populated_parts(
                        hal, trace, cycles, global_buf, code_buf, data_buf,
                    )
                } else {
                    hal.eltwise_zeroize_elem(&global_buf.buf);
                    hal.eltwise_zeroize_elem(&data_buf.buf);
                    let mut replace_snap = vec![0u32; data_buf.buf.size()];
                    data_buf.buf.view(|slice: &[Val]| {
                        let u32s: &[u32] = unsafe {
                            std::slice::from_raw_parts(slice.as_ptr() as *const u32, slice.len())
                        };
                        replace_snap.copy_from_slice(u32s);
                    });

                    data_buf.buf.view_mut(|slice: &mut [Val]| {
                        for v in slice.iter_mut() {
                            *v = Val::INVALID;
                        }
                    });
                    global_buf.buf.view_mut(|slice: &mut [Val]| {
                        slice.copy_from_slice(&global_vec);
                    });
                    hal.scatter(
                        &data_buf.buf,
                        &injector.index,
                        &injector.offsets,
                        &injector.values,
                    );
                    super::rust_steps::set_witgen_gpu_replace_arm_mask(0);
                    circuit_hal
                        .generate_witness(mode, &trace, &global_buf, &data_buf)
                        .context("witness generation failure (REPLACE DIFF CPU mode)")?;
                    hal.eltwise_zeroize_elem(&global_buf.buf);
                    hal.eltwise_zeroize_elem(&data_buf.buf);
                    let mut cpu_snap = vec![0u32; data_buf.buf.size()];
                    data_buf.buf.view(|slice: &[Val]| {
                        let u32s: &[u32] = unsafe {
                            std::slice::from_raw_parts(slice.as_ptr() as *const u32, slice.len())
                        };
                        cpu_snap.copy_from_slice(u32s);
                    });
                    let rows = data_buf.rows;
                    let cols = data_buf.cols;
                    let mut mismatches = 0usize;
                    for i in 0..replace_snap.len() {
                        let g = replace_snap[i];
                        let c = cpu_snap[i];
                        if g != c {
                            if mismatches < 20 {
                                let row = i % rows;
                                let col = i / rows;
                                let (major, minor) = trace
                                    .cycles
                                    .get(row)
                                    .map(|cycle| (cycle.major, cycle.minor))
                                    .unwrap_or((u8::MAX, u8::MAX));
                                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                                    "REPLACE_FINAL_MISMATCH idx={} row={} col={} major={} minor={} replace=0x{:08x} cpu=0x{:08x}",
                                    i, row, col, major, minor, g, c,
                                ));
                            }
                            mismatches += 1;
                        }
                    }
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "REPLACE_FINAL_SUMMARY segment_ord={} total_cells={} mismatches={} rows={} cols={}",
                        replace_diff_segment,
                        replace_snap.len(),
                        mismatches,
                        rows,
                        cols,
                    ));
                    anyhow::bail!(
                        "REPLACE DIFF mode: {} final data mismatches; see REPLACE_FINAL_MISMATCH + REPLACE_FINAL_SUMMARY",
                        mismatches
                    );
                }
            } else if diff_mode {
                let mut snap = vec![0u32; data_buf.buf.size()];
                data_buf.buf.view(|slice: &[Val]| {
                    let u32s: &[u32] = unsafe {
                        std::slice::from_raw_parts(slice.as_ptr() as *const u32, slice.len())
                    };
                    snap.copy_from_slice(u32s);
                });
                data_buf.buf.view_mut(|slice: &mut [Val]| {
                    for v in slice.iter_mut() {
                        *v = Val::INVALID;
                    }
                });
                hal.scatter(
                    &data_buf.buf,
                    &injector.index,
                    &injector.offsets,
                    &injector.values,
                );
                super::rust_steps::set_witgen_gpu_replace_arm_mask(0);
                // Run generate_witness only (no zeroize) so the CPU shadow
                // still has INVALID markers for unwritten cells.
                circuit_hal
                    .generate_witness(mode, &trace, &global_buf, &data_buf)
                    .context("witness generation failure (DIFF mode)")?;
                let cols = data_buf.cols;
                let rows = data_buf.rows;
                let mut cpu_snap = vec![0u32; data_buf.buf.size()];
                data_buf.buf.view(|slice: &[Val]| {
                    let u32s: &[u32] = unsafe {
                        std::slice::from_raw_parts(slice.as_ptr() as *const u32, slice.len())
                    };
                    cpu_snap.copy_from_slice(u32s);
                });
                const INVALID_U32: u32 = 0xffffffffu32;
                let mut mismatches = 0usize;
                let mut gpu_wrote = 0usize;
                let mut cpu_wrote = 0usize;
                let mut both_wrote_match = 0usize;
                let mut gpu_only = 0usize;
                let mut cpu_only = 0usize;
                let mut candidate_cpu_only_nonzero = 0usize;
                for i in 0..snap.len() {
                    let g = snap[i];
                    let c = cpu_snap[i];
                    let row = i % rows;
                    let col = i / rows;
                    let cycle = trace.cycles.get(row);
                    let candidate_row = cycle
                        .map(|cycle| is_witgen_diff_cycle(cycle.major, cycle.minor))
                        .unwrap_or(false);
                    let g_wrote = g != INVALID_U32;
                    let c_wrote = c != INVALID_U32;
                    if g_wrote {
                        gpu_wrote += 1;
                    }
                    if c_wrote {
                        cpu_wrote += 1;
                    }
                    if g_wrote && !c_wrote {
                        gpu_only += 1;
                    }
                    if !g_wrote && c_wrote {
                        cpu_only += 1;
                        if candidate_row && c != 0 {
                            if candidate_cpu_only_nonzero < 20 {
                                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                                    "DIFF_CANDIDATE_MISSING idx={} row={} col={} cpu=0x{:08x}",
                                    i, row, col, c,
                                ));
                            }
                            candidate_cpu_only_nonzero += 1;
                        }
                    }
                    if g_wrote && c_wrote && g != c {
                        if mismatches < 20 {
                            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                                "DIFF_MISMATCH idx={} row={} col={} gpu=0x{:08x} cpu=0x{:08x}",
                                i, row, col, g, c,
                            ));
                        }
                        mismatches += 1;
                    } else if g_wrote && c_wrote && g == c {
                        both_wrote_match += 1;
                    }
                }
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "DIFF_SUMMARY total_cells={} gpu_wrote={} cpu_wrote={} both_match={} mismatches={} gpu_only={} cpu_only={} candidate_cpu_only_nonzero={} rows={} cols={}",
                    snap.len(),
                    gpu_wrote,
                    cpu_wrote,
                    both_wrote_match,
                    mismatches,
                    gpu_only,
                    cpu_only,
                    candidate_cpu_only_nonzero,
                    rows,
                    cols,
                ));
                if mismatches != 0 || gpu_only != 0 || candidate_cpu_only_nonzero != 0 {
                    anyhow::bail!(
                        "DIFF mode: {} mismatches (gpu_only={} cpu_only={} candidate_cpu_only_nonzero={} both_match={}); see DIFF_MISMATCH + DIFF_CANDIDATE_MISSING + DIFF_SUMMARY",
                        mismatches,
                        gpu_only,
                        cpu_only,
                        candidate_cpu_only_nonzero,
                        both_wrote_match
                    );
                }

                hal.eltwise_zeroize_elem(&global_buf.buf);
                hal.eltwise_zeroize_elem(&data_buf.buf);
                let accum = MetaBuffer::new("accum", hal, cycles, REGCOUNT_ACCUM, true);
                super::super::witgen::WitnessGenerator::<WebGpuHal> {
                    cycles,
                    global: global_buf,
                    code: code_buf,
                    data: data_buf,
                    accum,
                    trace,
                }
            } else {
                super::super::witgen::WitnessGenerator::<WebGpuHal>::populate_from_parts(
                    hal,
                    circuit_hal,
                    mode,
                    trace,
                    cycles,
                    global_buf,
                    code_buf,
                    data_buf,
                )?
            };

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
                    let _t =
                        WebGpuStageTimer::new_active_for("commit_group_async rv32im_code", hal);
                    prover
                        .commit_group_async_in_place(REGISTER_GROUP_CODE, code.clone())
                        .await?;
                }
                {
                    let _t =
                        WebGpuStageTimer::new_active_for("commit_group_async rv32im_data", hal);
                    prover.commit_group_async(REGISTER_GROUP_DATA, data).await?;
                }
            }

            let accum_shadow_rows = circuit_hal
                .sync_witgen_replace_accum_shadow_rows(&witgen.trace, &witgen.data)
                .await?;
            if accum_shadow_rows != 0 {
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "rv32im_witgen_accum_shadow_gpu_sync rows={accum_shadow_rows}"
                ));
            }

            let mix: [Val; REGCOUNT_MIX] = std::array::from_fn(|_| prover.iop().random_elem());
            let mix = {
                let _t = WebGpuStageTimer::new("rv32im_witgen_accum");
                witgen.accum(hal, circuit_hal, &mix)?
            };
            circuit_hal.post_accum_candidate_sync_async().await?;

            let async_scopes = crate::prove::webgpu_async_authoritative_scopes();
            {
                let _t = WebGpuStageTimer::new_active_for("commit_group_async rv32im_accum", hal);
                prover
                    .commit_group_async_in_place_scoped(
                        REGISTER_GROUP_ACCUM,
                        witgen.accum.buf.clone(),
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

pub fn prewarm_witgen_kernel_for_hal(hal: Rc<WebGpuHal>) {
    WebGpuCircuitHal::new(hal).prewarm_witgen_kernel();
}

pub fn enable_webgpu_witgen_accum_acceleration_for_hal(hal: Rc<WebGpuHal>) {
    set_witgen_gpu_probe_enabled(true);
    set_witgen_gpu_replace_enabled(true);
    set_witgen_gpu_replace_nonblocking_pending_enabled(true);
    set_accum_gpu_misc0_direct_enabled(true);
    set_accum_gpu_misc1_direct_enabled(true);
    set_accum_gpu_misc2_direct_enabled(true);
    set_witgen_gpu_mem0_replace_candidate_enabled(true);
    set_witgen_gpu_mem0_replace_minor_mask(1u16 << 2);
    set_accum_gpu_mem0_direct_enabled(true);
    set_accum_gpu_mem1_direct_enabled(true);
    set_accum_gpu_control0_direct_enabled(true);
    prewarm_witgen_kernel_for_hal(hal);
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
