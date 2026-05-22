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

use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    slice,
    sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering},
};

use anyhow::{ensure, Context as _, Result};
use risc0_circuit_recursion_sys::{RawPreflightTrace, StepMode};
use risc0_zkp::{
    adapter::{CircuitInfo as _, TapsProvider as _, PROOF_SYSTEM_INFO},
    field::{
        baby_bear::{BabyBearElem, BabyBearExtElem},
        Elem as _, ExtElem as _,
    },
    hal::{
        webgpu::{
            WebGpuBindingLayout, WebGpuBuffer, WebGpuBufferBinding, WebGpuCircuitEvalCheck,
            WebGpuHal, WebGpuStageTimer,
        },
        AccumPreflight, Buffer, CircuitHal, Hal,
    },
    prove::Prover,
};

use crate::{
    prove::{preflight::Preflight, RecursionProver, RecursionProverImpl, RecursionReceipt},
    taps::TAPSET,
    CircuitImpl, CIRCUIT, REGISTER_GROUP_ACCUM, REGISTER_GROUP_CTRL, REGISTER_GROUP_DATA,
};

use super::{CircuitAccumulationMode, CircuitAccumulator, CircuitWitnessGenerator};

const RECURSION_ACCUM_PRELUDE_WGSL: &str = include_str!("webgpu_witgen_prelude.wgsl");
const RECURSION_ACCUM_LAYOUT_WGSL: &str = include_str!("webgpu_layout.wgsl.inc");
const RECURSION_STEP_COMPUTE_ACCUM_WGSL: &str = include_str!("webgpu_step_compute_accum.wgsl");
const RECURSION_STEP_VERIFY_ACCUM_WGSL: &str = include_str!("webgpu_step_verify_accum.wgsl");
const RECURSION_STEP_VERIFY_MEM_WGSL: &str = include_str!("webgpu_step_verify_mem.wgsl");
const RECURSION_STEP_EXEC_POSEIDON2_CHAIN_WGSL: &str =
    include_str!("webgpu_step_exec_poseidon2_chain.wgsl");
const RECURSION_STEP_EXEC_MICRO_OPS_WGSL: &str =
    include_str!("webgpu_step_exec_micro_ops.wgsl");
const RECURSION_STEP_EXEC_MACRO_OPS_WGSL: &str =
    include_str!("webgpu_step_exec_macro_ops.wgsl");
const RECURSION_RUST_KERNELS_GENERATED: &str = include_str!("rust_kernels_generated.rs.inc");

const RECURSION_ACCUM_COMPUTE_ENTRY: &str = r#"
@compute @workgroup_size(64)
fn recursion_step_compute_accum_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let logical_cycle = gid.x + gid.y * 65535u * 64u;
  if (logical_cycle >= params.work_cycles) {
    return;
  }
  cycle = logical_cycle;
  let _result = recursion(buf_ctrl, buf_global, buf_data, buf_mix, buf_wom);
}
"#;

const RECURSION_ACCUM_VERIFY_ENTRY: &str = r#"
@compute @workgroup_size(64)
fn recursion_step_verify_accum_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let logical_cycle = gid.x + gid.y * 65535u * 64u;
  if (logical_cycle >= params.work_cycles) {
    return;
  }
  cycle = logical_cycle;
  let _result = recursion(buf_ctrl, buf_global, buf_data, buf_mix, buf_accum);
}
"#;

static RECURSION_ACCUM_GPU_ENABLED: AtomicBool = AtomicBool::new(true);
static RECURSION_ACCUM_GPU_DISPATCHES: AtomicU64 = AtomicU64::new(0);
static RECURSION_WITGEN_GPU_VERIFY_MEM_CANDIDATE_ENABLED: AtomicBool = AtomicBool::new(true);
static RECURSION_WITGEN_GPU_VERIFY_MEM_CANDIDATE_DISPATCHES: AtomicU64 = AtomicU64::new(0);
static RECURSION_WITGEN_POST_ZEROIZE_HOOK_PROBE_ENABLED: AtomicBool = AtomicBool::new(false);
static RECURSION_WITGEN_POST_ZEROIZE_HOOK_CALLS: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static RECURSION_WITGEN_GPU_VERIFY_MEM_PLAN: RefCell<Option<super::rust_kernels::WomGpuVerifyPlan>> =
        RefCell::new(None);
}

pub fn set_recursion_accum_gpu_enabled(enabled: bool) {
    RECURSION_ACCUM_GPU_ENABLED.store(enabled, AtomicOrdering::SeqCst);
}

pub fn recursion_accum_gpu_dispatches() -> u64 {
    RECURSION_ACCUM_GPU_DISPATCHES.load(AtomicOrdering::SeqCst)
}

pub fn set_recursion_witgen_gpu_verify_mem_candidate_enabled(enabled: bool) {
    if enabled {
        RECURSION_WITGEN_GPU_VERIFY_MEM_CANDIDATE_DISPATCHES.store(0, AtomicOrdering::SeqCst);
    }
    RECURSION_WITGEN_GPU_VERIFY_MEM_CANDIDATE_ENABLED.store(enabled, AtomicOrdering::SeqCst);
    RECURSION_WITGEN_GPU_VERIFY_MEM_PLAN.with(|plan| {
        *plan.borrow_mut() = None;
    });
}

pub fn recursion_witgen_gpu_verify_mem_candidate_dispatches() -> u64 {
    RECURSION_WITGEN_GPU_VERIFY_MEM_CANDIDATE_DISPATCHES.load(AtomicOrdering::SeqCst)
}

pub fn set_recursion_witgen_post_zeroize_hook_probe_enabled(enabled: bool) {
    if enabled {
        RECURSION_WITGEN_POST_ZEROIZE_HOOK_CALLS.store(0, AtomicOrdering::SeqCst);
    }
    RECURSION_WITGEN_POST_ZEROIZE_HOOK_PROBE_ENABLED.store(enabled, AtomicOrdering::SeqCst);
}

pub fn recursion_witgen_post_zeroize_hook_calls() -> u64 {
    RECURSION_WITGEN_POST_ZEROIZE_HOOK_CALLS.load(AtomicOrdering::SeqCst)
}

pub fn set_recursion_wom_sort_profile_enabled(enabled: bool) {
    super::rust_kernels::set_wom_sort_profile_enabled(enabled);
}

pub fn recursion_wom_sort_profile_snapshot() -> [u64; 7] {
    super::rust_kernels::wom_sort_profile_snapshot()
}

pub fn recursion_accum_wgsl_modules_for_test() -> (String, String) {
    (
        recursion_accum_wgsl(
            RECURSION_STEP_COMPUTE_ACCUM_WGSL,
            RECURSION_ACCUM_COMPUTE_ENTRY,
        ),
        recursion_accum_wgsl(
            RECURSION_STEP_VERIFY_ACCUM_WGSL,
            RECURSION_ACCUM_VERIFY_ENTRY,
        ),
    )
}

pub fn recursion_exec_poseidon2_chain_wgsl_module_for_test() -> String {
    recursion_accum_wgsl(
        RECURSION_STEP_EXEC_POSEIDON2_CHAIN_WGSL,
        RECURSION_EXEC_TEST_EXTRA_WGSL,
    )
}

pub fn recursion_exec_poseidon2_chain_wom_probe_wgsl_module_for_test() -> String {
    recursion_accum_wgsl(
        RECURSION_STEP_EXEC_POSEIDON2_CHAIN_WGSL,
        RECURSION_EXEC_WOM_PROBE_EXTRA_WGSL,
    )
}

pub fn recursion_exec_poseidon2_chain_wom_scatter_probe_wgsl_module_for_test() -> String {
    recursion_accum_wgsl(
        RECURSION_STEP_EXEC_POSEIDON2_CHAIN_WGSL,
        RECURSION_EXEC_WOM_SCATTER_PROBE_EXTRA_WGSL,
    )
}

pub fn recursion_exec_micro_ops_wom_scatter_probe_wgsl_module_for_test() -> String {
    let entry = recursion_exec_wom_scatter_probe_extra_wgsl(
        "recursion_step_exec_micro_ops_main",
        "recursion_micro_ops_wom_scatter_main",
        "recursion_micro_ops_wom_backfill_main",
    );
    recursion_accum_wgsl(RECURSION_STEP_EXEC_MICRO_OPS_WGSL, &entry)
}

pub fn recursion_exec_macro_ops_wom_scatter_probe_wgsl_module_for_test() -> String {
    let entry = recursion_exec_wom_scatter_probe_extra_wgsl(
        "recursion_step_exec_macro_ops_main",
        "recursion_macro_ops_wom_scatter_main",
        "recursion_macro_ops_wom_backfill_main",
    );
    recursion_accum_wgsl(RECURSION_STEP_EXEC_MACRO_OPS_WGSL, &entry)
}

pub fn recursion_verify_mem_wom_probe_wgsl_module_for_test() -> String {
    recursion_accum_wgsl(
        RECURSION_STEP_VERIFY_MEM_WGSL,
        RECURSION_VERIFY_MEM_WOM_PROBE_EXTRA_WGSL,
    )
}

pub fn recursion_checked_bytes_wom_scatter_probe_wgsl_module_for_test() -> String {
    recursion_accum_wgsl("", RECURSION_CHECKED_BYTES_WOM_SCATTER_PROBE_EXTRA_WGSL)
}

pub fn recursion_wom_generated_row_coverage_for_test() -> Vec<(String, usize, usize)> {
    let writes = generated_wom_rows_by_family("ctx.plonk_write_wom", WomRowScan::Backward);
    let reads = generated_wom_rows_by_family("ctx.plonk_read_wom", WomRowScan::Forward);
    assert_eq!(
        writes.keys().collect::<Vec<_>>(),
        reads.keys().collect::<Vec<_>>(),
        "generated step_exec WOM write families differ from verify_mem read families"
    );

    writes
        .into_iter()
        .map(|(family, writes)| {
            let reads = reads[&family];
            (family, writes, reads)
        })
        .collect()
}

fn recursion_accum_wgsl(step: &str, entry: &str) -> String {
    let mut module = String::with_capacity(
        RECURSION_ACCUM_PRELUDE_WGSL.len()
            + RECURSION_ACCUM_LAYOUT_WGSL.len()
            + step.len()
            + entry.len()
            + 4,
    );
    module.push_str(RECURSION_ACCUM_PRELUDE_WGSL);
    module.push('\n');
    module.push_str(RECURSION_ACCUM_LAYOUT_WGSL);
    module.push('\n');
    module.push_str(step);
    module.push('\n');
    module.push_str(entry);
    module
}

fn recursion_exec_wom_scatter_probe_extra_wgsl(
    exec_entry: &str,
    scatter_entry: &str,
    backfill_entry: &str,
) -> String {
    RECURSION_EXEC_WOM_SCATTER_PROBE_EXTRA_WGSL
        .replace("recursion_step_exec_poseidon2_chain_main", exec_entry)
        .replace("recursion_poseidon2_wom_scatter_main", scatter_entry)
        .replace("recursion_poseidon2_wom_backfill_main", backfill_entry)
}

fn recursion_exec_wom_candidate_extra_wgsl(exec_entry: &str) -> String {
    RECURSION_EXEC_WOM_CANDIDATE_EXTRA_WGSL
        .replace("recursion_step_exec_poseidon2_chain_main", exec_entry)
}

enum WomRowScan {
    Backward,
    Forward,
}

fn generated_wom_rows_by_family(needle: &str, scan: WomRowScan) -> BTreeMap<String, usize> {
    let lines: Vec<_> = RECURSION_RUST_KERNELS_GENERATED.lines().collect();
    let mut out = BTreeMap::new();
    for (line_idx, line) in lines.iter().enumerate() {
        if !line.contains(needle) {
            continue;
        }
        let family = match scan {
            WomRowScan::Backward => (line_idx.saturating_sub(96)..line_idx)
                .rev()
                .find_map(|idx| wom_row_family_from_comment(lines[idx])),
            WomRowScan::Forward => (line_idx + 1..lines.len().min(line_idx + 250))
                .find_map(|idx| wom_row_family_from_comment(lines[idx])),
        }
        .unwrap_or_else(|| panic!("missing generated WOM row family near line {}", line_idx + 1));
        *out.entry(family).or_insert(0) += 1;
    }
    out
}

fn wom_row_family_from_comment(line: &str) -> Option<String> {
    let line = line.trim_start().strip_prefix("// ")?;
    let tail = line.split_once("top(recursion::Top)/mux(Mux)/")?.1;
    for marker in ["/wom_body", "/recursion::WomBody", "/PlonkFini"] {
        if let Some((family, _)) = tail.split_once(marker) {
            return Some(family.to_string());
        }
    }
    None
}

const RECURSION_EXEC_TEST_EXTRA_WGSL: &str = r#"
fn extern_womRead(addr: Val) -> array<Val, 4> {
  return array<Val, 4>(0u, 0u, 0u, 0u);
}

fn extern_womWrite(addr: Val, a: Val, b: Val, c: Val, d: Val) {
}

fn extern_readIOPHeader(a: Val, b: Val) {
}

fn extern_readIOPBody(a: Val, b: Val, c: Val) -> array<Val, 4> {
  return array<Val, 4>(0u, 0u, 0u, 0u);
}

fn extern_readCoefficients() -> array<Val, 16> {
  return array<Val, 16>(
    0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u);
}

fn extern_plonkWrite(a: Val, b: Val, c: Val, d: Val, e: Val) {
}

@compute @workgroup_size(64)
fn recursion_step_exec_poseidon2_chain_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  cycle = gid.x;
  if (cycle >= params.work_cycles) {
    return;
  }
  let _result = recursion(buf_ctrl, buf_global, buf_data, buf_mix, buf_accum);
}
"#;

const RECURSION_EXEC_WOM_PROBE_EXTRA_WGSL: &str = r#"
const MAX_WOM_ROWS_PER_CYCLE: u32 = 9u;

@group(0) @binding(7) var<storage, read_write> preflight_wom_probe_buf: array<u32>;
@group(0) @binding(8) var<storage, read_write> wom_write_probe_rows_buf: array<u32>;
@group(0) @binding(9) var<storage, read_write> plonk_probe_rows_buf: array<u32>;
@group(0) @binding(10) var<storage, read_write> wom_probe_cursors_buf: array<u32>;

fn wom_probe_row_base(idx: u32) -> u32 {
  return ((cycle * MAX_WOM_ROWS_PER_CYCLE) + idx) * 5u;
}

fn extern_womRead(addr: Val) -> array<Val, 4> {
  let base = decode(addr) * 4u;
  return array<Val, 4>(
    preflight_wom_probe_buf[base],
    preflight_wom_probe_buf[base + 1u],
    preflight_wom_probe_buf[base + 2u],
    preflight_wom_probe_buf[base + 3u]);
}

fn extern_womWrite(addr: Val, a: Val, b: Val, c: Val, d: Val) {
  let cursor_idx = cycle * 2u;
  let idx = wom_probe_cursors_buf[cursor_idx];
  wom_probe_cursors_buf[cursor_idx] = idx + 1u;
  let base = wom_probe_row_base(idx);
  wom_write_probe_rows_buf[base] = addr;
  wom_write_probe_rows_buf[base + 1u] = a;
  wom_write_probe_rows_buf[base + 2u] = b;
  wom_write_probe_rows_buf[base + 3u] = c;
  wom_write_probe_rows_buf[base + 4u] = d;
}

fn extern_readIOPHeader(a: Val, b: Val) {
}

fn extern_readIOPBody(a: Val, b: Val, c: Val) -> array<Val, 4> {
  return array<Val, 4>(0u, 0u, 0u, 0u);
}

fn extern_readCoefficients() -> array<Val, 16> {
  return array<Val, 16>(
    0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u);
}

fn extern_plonkWrite(a: Val, b: Val, c: Val, d: Val, e: Val) {
  let cursor_idx = cycle * 2u + 1u;
  let idx = wom_probe_cursors_buf[cursor_idx];
  wom_probe_cursors_buf[cursor_idx] = idx + 1u;
  let base = wom_probe_row_base(idx);
  plonk_probe_rows_buf[base] = a;
  plonk_probe_rows_buf[base + 1u] = b;
  plonk_probe_rows_buf[base + 2u] = c;
  plonk_probe_rows_buf[base + 3u] = d;
  plonk_probe_rows_buf[base + 4u] = e;
}

@compute @workgroup_size(64)
fn recursion_step_exec_poseidon2_chain_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  cycle = gid.x;
  if (cycle >= params.work_cycles) {
    return;
  }
  let _result = recursion(buf_ctrl, buf_global, buf_data, buf_mix, buf_accum);
}
"#;

const RECURSION_EXEC_WOM_SCATTER_PROBE_EXTRA_WGSL: &str = r#"
const MAX_WOM_ROWS_PER_CYCLE: u32 = 9u;

@group(0) @binding(7) var<storage, read_write> preflight_wom_probe_buf: array<u32>;
@group(0) @binding(8) var<storage, read_write> plonk_unsorted_rows_buf: array<u32>;
@group(0) @binding(9) var<storage, read_write> plonk_row_cursors_buf: array<u32>;
@group(0) @binding(10) var<storage, read_write> plonk_sorted_rows_buf: array<u32>;
@group(0) @binding(11) var<storage, read_write> plonk_sorted_counters_buf: array<atomic<u32>>;
@group(0) @binding(12) var<storage, read_write> plonk_bucket_bases_buf: array<u32>;
@group(0) @binding(13) var<storage, read_write> plonk_cycle_prefix_buf: array<u32>;

fn wom_scatter_row_base(row_cycle: u32, idx: u32) -> u32 {
  return ((row_cycle * MAX_WOM_ROWS_PER_CYCLE) + idx) * 5u;
}

fn extern_womRead(addr: Val) -> array<Val, 4> {
  let base = decode(addr) * 4u;
  return array<Val, 4>(
    preflight_wom_probe_buf[base],
    preflight_wom_probe_buf[base + 1u],
    preflight_wom_probe_buf[base + 2u],
    preflight_wom_probe_buf[base + 3u]);
}

fn extern_womWrite(addr: Val, a: Val, b: Val, c: Val, d: Val) {
}

fn extern_readIOPHeader(a: Val, b: Val) {
}

fn extern_readIOPBody(a: Val, b: Val, c: Val) -> array<Val, 4> {
  return array<Val, 4>(0u, 0u, 0u, 0u);
}

fn extern_readCoefficients() -> array<Val, 16> {
  return array<Val, 16>(
    0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u);
}

fn extern_plonkWrite(a: Val, b: Val, c: Val, d: Val, e: Val) {
  let idx = plonk_row_cursors_buf[cycle];
  plonk_row_cursors_buf[cycle] = idx + 1u;
  let base = wom_scatter_row_base(cycle, idx);
  plonk_unsorted_rows_buf[base] = a;
  plonk_unsorted_rows_buf[base + 1u] = b;
  plonk_unsorted_rows_buf[base + 2u] = c;
  plonk_unsorted_rows_buf[base + 3u] = d;
  plonk_unsorted_rows_buf[base + 4u] = e;
}

fn wom_scatter_bucket(base: u32) -> u32 {
  let addr = plonk_unsorted_rows_buf[base];
  let a = plonk_unsorted_rows_buf[base + 1u];
  let b = plonk_unsorted_rows_buf[base + 2u];
  let c = plonk_unsorted_rows_buf[base + 3u];
  let d = plonk_unsorted_rows_buf[base + 4u];
  if (addr == 0u && a == 0u && b == 0u && c == 0u && d == 0u) {
    return 0u;
  }
  return decode(addr) + 1u;
}

@compute @workgroup_size(64)
fn recursion_step_exec_poseidon2_chain_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  cycle = gid.x;
  if (cycle >= params.work_cycles) {
    return;
  }
  let _result = recursion(buf_ctrl, buf_global, buf_data, buf_mix, buf_accum);
}

@compute @workgroup_size(64)
fn recursion_poseidon2_wom_scatter_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let flat = gid.x;
  let row_cycle = flat / MAX_WOM_ROWS_PER_CYCLE;
  if (row_cycle >= params.work_cycles) {
    return;
  }
  let row_idx = flat - row_cycle * MAX_WOM_ROWS_PER_CYCLE;
  if (row_idx >= plonk_row_cursors_buf[row_cycle]) {
    return;
  }

  let src_base = wom_scatter_row_base(row_cycle, row_idx);
  let bucket = wom_scatter_bucket(src_base);
  let dst_idx = plonk_bucket_bases_buf[bucket] + atomicAdd(&plonk_sorted_counters_buf[bucket], 1u);
  let dst_base = dst_idx * 5u;
  plonk_sorted_rows_buf[dst_base] = plonk_unsorted_rows_buf[src_base];
  plonk_sorted_rows_buf[dst_base + 1u] = plonk_unsorted_rows_buf[src_base + 1u];
  plonk_sorted_rows_buf[dst_base + 2u] = plonk_unsorted_rows_buf[src_base + 2u];
  plonk_sorted_rows_buf[dst_base + 3u] = plonk_unsorted_rows_buf[src_base + 3u];
  plonk_sorted_rows_buf[dst_base + 4u] = plonk_unsorted_rows_buf[src_base + 4u];
}

fn wom_backfill_store(dst_cycle: u32, a: Val, b: Val, c: Val, d: Val, e: Val) {
  data_buf[dst_cycle] = a;
  data_buf[params.data_rows + dst_cycle] = b;
  data_buf[2u * params.data_rows + dst_cycle] = c;
  data_buf[3u * params.data_rows + dst_cycle] = d;
  data_buf[4u * params.data_rows + dst_cycle] = e;
}

@compute @workgroup_size(64)
fn recursion_poseidon2_wom_backfill_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let back_cycle = gid.x + 1u;
  if (back_cycle >= params.work_cycles) {
    return;
  }
  let dst_cycle = back_cycle - 1u;
  let sorted_idx = plonk_cycle_prefix_buf[back_cycle];
  if (sorted_idx == 0u) {
    wom_backfill_store(dst_cycle, 0u, 0u, 0u, 0u, 0u);
    return;
  }
  let src_base = (sorted_idx - 1u) * 5u;
  wom_backfill_store(
    dst_cycle,
    plonk_sorted_rows_buf[src_base],
    plonk_sorted_rows_buf[src_base + 1u],
    plonk_sorted_rows_buf[src_base + 2u],
    plonk_sorted_rows_buf[src_base + 3u],
    plonk_sorted_rows_buf[src_base + 4u]);
}
"#;

const RECURSION_VERIFY_MEM_WOM_PROBE_EXTRA_WGSL: &str = r#"
@group(0) @binding(7) var<storage, read> plonk_sorted_rows_buf: array<u32>;
@group(0) @binding(8) var<storage, read_write> plonk_cycle_cursors_buf: array<atomic<u32>>;
@group(0) @binding(9) var<storage, read> plonk_cycle_prefix_buf: array<u32>;

fn extern_plonkRead() -> array<Val, 5> {
  let row_idx = plonk_cycle_prefix_buf[cycle] + atomicAdd(&plonk_cycle_cursors_buf[cycle], 1u);
  let base = row_idx * 5u;
  return array<Val, 5>(
    plonk_sorted_rows_buf[base],
    plonk_sorted_rows_buf[base + 1u],
    plonk_sorted_rows_buf[base + 2u],
    plonk_sorted_rows_buf[base + 3u],
    plonk_sorted_rows_buf[base + 4u]);
}

@compute @workgroup_size(64)
fn recursion_step_verify_mem_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  cycle = gid.x;
  if (cycle >= params.work_cycles) {
    return;
  }
  let _result = recursion(buf_ctrl, buf_global, buf_data, buf_mix, buf_accum);
}
"#;

const RECURSION_CHECKED_BYTES_WOM_SCATTER_PROBE_EXTRA_WGSL: &str = r#"
const MAX_CHECKED_BYTES_WOM_ROWS_PER_CYCLE: u32 = 2u;

@group(0) @binding(7) var<storage, read_write> checked_bytes_unsorted_rows_buf: array<u32>;
@group(0) @binding(8) var<storage, read_write> checked_bytes_row_cursors_buf: array<u32>;
@group(0) @binding(9) var<storage, read_write> checked_bytes_sorted_rows_buf: array<u32>;
@group(0) @binding(10) var<storage, read_write> checked_bytes_sorted_counters_buf: array<atomic<u32>>;
@group(0) @binding(11) var<storage, read_write> checked_bytes_bucket_bases_buf: array<u32>;
@group(0) @binding(12) var<storage, read_write> checked_bytes_cycle_prefix_buf: array<u32>;

fn checked_bytes_wom_row_base(row_cycle: u32, idx: u32) -> u32 {
  return ((row_cycle * MAX_CHECKED_BYTES_WOM_ROWS_PER_CYCLE) + idx) * 5u;
}

fn checked_bytes_store_unsorted_row(a: Val, b: Val, c: Val, d: Val, e: Val) {
  let idx = checked_bytes_row_cursors_buf[cycle];
  checked_bytes_row_cursors_buf[cycle] = idx + 1u;
  let base = checked_bytes_wom_row_base(cycle, idx);
  checked_bytes_unsorted_rows_buf[base] = a;
  checked_bytes_unsorted_rows_buf[base + 1u] = b;
  checked_bytes_unsorted_rows_buf[base + 2u] = c;
  checked_bytes_unsorted_rows_buf[base + 3u] = d;
  checked_bytes_unsorted_rows_buf[base + 4u] = e;
}

fn checked_bytes_wom_bucket(base: u32) -> u32 {
  let addr = checked_bytes_unsorted_rows_buf[base];
  let a = checked_bytes_unsorted_rows_buf[base + 1u];
  let b = checked_bytes_unsorted_rows_buf[base + 2u];
  let c = checked_bytes_unsorted_rows_buf[base + 3u];
  let d = checked_bytes_unsorted_rows_buf[base + 4u];
  if (addr == 0u && a == 0u && b == 0u && c == 0u && d == 0u) {
    return 0u;
  }
  return decode(addr) + 1u;
}

@compute @workgroup_size(64)
fn recursion_checked_bytes_wom_rows_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  cycle = gid.x;
  if (cycle >= params.work_cycles) {
    return;
  }
  if (load_at(7u, buf_ctrl, 0u) == 0u) {
    return;
  }

  checked_bytes_store_unsorted_row(
    load_at(5u, buf_data, 0u),
    load_at(6u, buf_data, 0u),
    load_at(7u, buf_data, 0u),
    load_at(8u, buf_data, 0u),
    load_at(9u, buf_data, 0u));
  checked_bytes_store_unsorted_row(
    load_at(10u, buf_data, 0u),
    load_at(11u, buf_data, 0u),
    load_at(12u, buf_data, 0u),
    load_at(13u, buf_data, 0u),
    load_at(14u, buf_data, 0u));
}

@compute @workgroup_size(64)
fn recursion_checked_bytes_wom_scatter_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let flat = gid.x;
  let row_cycle = flat / MAX_CHECKED_BYTES_WOM_ROWS_PER_CYCLE;
  if (row_cycle >= params.work_cycles) {
    return;
  }
  let row_idx = flat - row_cycle * MAX_CHECKED_BYTES_WOM_ROWS_PER_CYCLE;
  if (row_idx >= checked_bytes_row_cursors_buf[row_cycle]) {
    return;
  }

  let src_base = checked_bytes_wom_row_base(row_cycle, row_idx);
  let bucket = checked_bytes_wom_bucket(src_base);
  let dst_idx = checked_bytes_bucket_bases_buf[bucket] + atomicAdd(&checked_bytes_sorted_counters_buf[bucket], 1u);
  let dst_base = dst_idx * 5u;
  checked_bytes_sorted_rows_buf[dst_base] = checked_bytes_unsorted_rows_buf[src_base];
  checked_bytes_sorted_rows_buf[dst_base + 1u] = checked_bytes_unsorted_rows_buf[src_base + 1u];
  checked_bytes_sorted_rows_buf[dst_base + 2u] = checked_bytes_unsorted_rows_buf[src_base + 2u];
  checked_bytes_sorted_rows_buf[dst_base + 3u] = checked_bytes_unsorted_rows_buf[src_base + 3u];
  checked_bytes_sorted_rows_buf[dst_base + 4u] = checked_bytes_unsorted_rows_buf[src_base + 4u];
}

fn checked_bytes_wom_backfill_store(dst_cycle: u32, a: Val, b: Val, c: Val, d: Val, e: Val) {
  data_buf[dst_cycle] = a;
  data_buf[params.data_rows + dst_cycle] = b;
  data_buf[2u * params.data_rows + dst_cycle] = c;
  data_buf[3u * params.data_rows + dst_cycle] = d;
  data_buf[4u * params.data_rows + dst_cycle] = e;
}

@compute @workgroup_size(64)
fn recursion_checked_bytes_wom_backfill_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let back_cycle = gid.x + 1u;
  if (back_cycle >= params.work_cycles) {
    return;
  }
  let dst_cycle = back_cycle - 1u;
  let sorted_idx = checked_bytes_cycle_prefix_buf[back_cycle];
  if (sorted_idx == 0u) {
    checked_bytes_wom_backfill_store(dst_cycle, 0u, 0u, 0u, 0u, 0u);
    return;
  }
  let src_base = (sorted_idx - 1u) * 5u;
  checked_bytes_wom_backfill_store(
    dst_cycle,
    checked_bytes_sorted_rows_buf[src_base],
    checked_bytes_sorted_rows_buf[src_base + 1u],
    checked_bytes_sorted_rows_buf[src_base + 2u],
    checked_bytes_sorted_rows_buf[src_base + 3u],
    checked_bytes_sorted_rows_buf[src_base + 4u]);
}
"#;

const RECURSION_EXEC_WOM_CANDIDATE_EXTRA_WGSL: &str = r#"
const MAX_WOM_ROWS_PER_CYCLE: u32 = 9u;

@group(0) @binding(7) var<storage, read> preflight_wom_buf: array<u32>;
@group(0) @binding(8) var<storage, read> iop_body_buf: array<u32>;
@group(0) @binding(9) var<storage, read_write> iop_cursor_buf: array<u32>;
@group(0) @binding(10) var<storage, read_write> plonk_unsorted_rows_buf: array<u32>;
@group(0) @binding(11) var<storage, read_write> plonk_row_cursors_buf: array<u32>;
@group(0) @binding(12) var<storage, read_write> plonk_sorted_rows_buf: array<u32>;
@group(0) @binding(13) var<storage, read_write> plonk_sorted_counters_buf: array<atomic<u32>>;
@group(0) @binding(14) var<storage, read> plonk_bucket_bases_buf: array<u32>;
@group(0) @binding(15) var<storage, read> plonk_cycle_prefix_buf: array<u32>;

fn wom_candidate_row_base(row_cycle: u32, idx: u32) -> u32 {
  return ((row_cycle * MAX_WOM_ROWS_PER_CYCLE) + idx) * 5u;
}

fn extern_womRead(addr: Val) -> array<Val, 4> {
  let base = decode(addr) * 4u;
  return array<Val, 4>(
    preflight_wom_buf[base],
    preflight_wom_buf[base + 1u],
    preflight_wom_buf[base + 2u],
    preflight_wom_buf[base + 3u]);
}

fn extern_womWrite(addr: Val, a: Val, b: Val, c: Val, d: Val) {
}

fn extern_readIOPHeader(a: Val, b: Val) {
}

fn extern_readIOPBody(a: Val, b: Val, c: Val) -> array<Val, 4> {
  let idx = iop_cursor_buf[cycle];
  iop_cursor_buf[cycle] = idx + 1u;
  let base = idx * 4u;
  return array<Val, 4>(
    iop_body_buf[base],
    iop_body_buf[base + 1u],
    iop_body_buf[base + 2u],
    iop_body_buf[base + 3u]);
}

fn extern_readCoefficients() -> array<Val, 16> {
  return array<Val, 16>(
    0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u,
    0u, 0u, 0u, 0u);
}

fn extern_plonkWrite(a: Val, b: Val, c: Val, d: Val, e: Val) {
  let idx = plonk_row_cursors_buf[cycle];
  plonk_row_cursors_buf[cycle] = idx + 1u;
  let base = wom_candidate_row_base(cycle, idx);
  plonk_unsorted_rows_buf[base] = a;
  plonk_unsorted_rows_buf[base + 1u] = b;
  plonk_unsorted_rows_buf[base + 2u] = c;
  plonk_unsorted_rows_buf[base + 3u] = d;
  plonk_unsorted_rows_buf[base + 4u] = e;
}

fn wom_candidate_bucket(base: u32) -> u32 {
  let addr = plonk_unsorted_rows_buf[base];
  let a = plonk_unsorted_rows_buf[base + 1u];
  let b = plonk_unsorted_rows_buf[base + 2u];
  let c = plonk_unsorted_rows_buf[base + 3u];
  let d = plonk_unsorted_rows_buf[base + 4u];
  if (addr == 0u && a == 0u && b == 0u && c == 0u && d == 0u) {
    return 0u;
  }
  return decode(addr) + 1u;
}

@compute @workgroup_size(64)
fn recursion_step_exec_poseidon2_chain_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  cycle = gid.x + gid.y * 65535u * 64u;
  if (cycle >= params.work_cycles) {
    return;
  }
  let _result = recursion(buf_ctrl, buf_global, buf_data, buf_mix, buf_accum);
}

@compute @workgroup_size(64)
fn recursion_witgen_wom_scatter_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let flat = gid.x + gid.y * 65535u * 64u;
  let row_cycle = flat / MAX_WOM_ROWS_PER_CYCLE;
  if (row_cycle >= params.work_cycles) {
    return;
  }
  let row_idx = flat - row_cycle * MAX_WOM_ROWS_PER_CYCLE;
  if (row_idx >= plonk_row_cursors_buf[row_cycle]) {
    return;
  }

  let src_base = wom_candidate_row_base(row_cycle, row_idx);
  let bucket = wom_candidate_bucket(src_base);
  let dst_idx = plonk_bucket_bases_buf[bucket] + atomicAdd(&plonk_sorted_counters_buf[bucket], 1u);
  let dst_base = dst_idx * 5u;
  plonk_sorted_rows_buf[dst_base] = plonk_unsorted_rows_buf[src_base];
  plonk_sorted_rows_buf[dst_base + 1u] = plonk_unsorted_rows_buf[src_base + 1u];
  plonk_sorted_rows_buf[dst_base + 2u] = plonk_unsorted_rows_buf[src_base + 2u];
  plonk_sorted_rows_buf[dst_base + 3u] = plonk_unsorted_rows_buf[src_base + 3u];
  plonk_sorted_rows_buf[dst_base + 4u] = plonk_unsorted_rows_buf[src_base + 4u];
}

fn wom_candidate_backfill_store(dst_cycle: u32, a: Val, b: Val, c: Val, d: Val, e: Val) {
  data_buf[dst_cycle] = a;
  data_buf[params.data_rows + dst_cycle] = b;
  data_buf[2u * params.data_rows + dst_cycle] = c;
  data_buf[3u * params.data_rows + dst_cycle] = d;
  data_buf[4u * params.data_rows + dst_cycle] = e;
}

@compute @workgroup_size(64)
fn recursion_witgen_wom_backfill_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let back_cycle = gid.x + gid.y * 65535u * 64u + 1u;
  if (back_cycle >= params.work_cycles) {
    return;
  }
  let dst_cycle = back_cycle - 1u;
  let sorted_idx = plonk_cycle_prefix_buf[back_cycle];
  if (sorted_idx == 0u) {
    wom_candidate_backfill_store(dst_cycle, 0u, 0u, 0u, 0u, 0u);
    return;
  }
  let src_base = (sorted_idx - 1u) * 5u;
  wom_candidate_backfill_store(
    dst_cycle,
    plonk_sorted_rows_buf[src_base],
    plonk_sorted_rows_buf[src_base + 1u],
    plonk_sorted_rows_buf[src_base + 2u],
    plonk_sorted_rows_buf[src_base + 3u],
    plonk_sorted_rows_buf[src_base + 4u]);
}
"#;

const RECURSION_CHECKED_BYTES_WOM_CANDIDATE_EXTRA_WGSL: &str = r#"
const MAX_WOM_ROWS_PER_CYCLE: u32 = 9u;

@group(0) @binding(10) var<storage, read_write> plonk_unsorted_rows_buf: array<u32>;
@group(0) @binding(11) var<storage, read_write> plonk_row_cursors_buf: array<u32>;

fn checked_bytes_candidate_row_base(row_cycle: u32, idx: u32) -> u32 {
  return ((row_cycle * MAX_WOM_ROWS_PER_CYCLE) + idx) * 5u;
}

fn checked_bytes_candidate_store_row(a: Val, b: Val, c: Val, d: Val, e: Val) {
  let idx = plonk_row_cursors_buf[cycle];
  plonk_row_cursors_buf[cycle] = idx + 1u;
  let base = checked_bytes_candidate_row_base(cycle, idx);
  plonk_unsorted_rows_buf[base] = a;
  plonk_unsorted_rows_buf[base + 1u] = b;
  plonk_unsorted_rows_buf[base + 2u] = c;
  plonk_unsorted_rows_buf[base + 3u] = d;
  plonk_unsorted_rows_buf[base + 4u] = e;
}

@compute @workgroup_size(64)
fn recursion_checked_bytes_wom_rows_combined_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  cycle = gid.x + gid.y * 65535u * 64u;
  if (cycle >= params.work_cycles) {
    return;
  }
  if (load_at(7u, buf_ctrl, 0u) == 0u) {
    return;
  }

  checked_bytes_candidate_store_row(
    load_at(5u, buf_data, 0u),
    load_at(6u, buf_data, 0u),
    load_at(7u, buf_data, 0u),
    load_at(8u, buf_data, 0u),
    load_at(9u, buf_data, 0u));
  checked_bytes_candidate_store_row(
    load_at(10u, buf_data, 0u),
    load_at(11u, buf_data, 0u),
    load_at(12u, buf_data, 0u),
    load_at(13u, buf_data, 0u),
    load_at(14u, buf_data, 0u));
}
"#;

fn nonempty_u32_byte_len(words: usize) -> Result<u64> {
    let words = words.max(1);
    words
        .checked_mul(std::mem::size_of::<u32>())
        .and_then(|bytes| u64::try_from(bytes).ok())
        .context("recursion WOM candidate buffer byte length overflow")
}

fn pack_ext_words(ptr: *const BabyBearExtElem, len: u32) -> Vec<BabyBearElem> {
    let ext = unsafe { slice::from_raw_parts(ptr, len as usize) };
    let mut words = Vec::with_capacity(ext.len() * BabyBearExtElem::EXT_SIZE);
    for value in ext {
        words.extend_from_slice(&value.elems());
    }
    words
}

fn pack_iop_cursors(preflight: &RawPreflightTrace) -> Vec<u32> {
    let cycles = unsafe { slice::from_raw_parts(preflight.cycles, preflight.num_cycles as usize) };
    cycles.iter().map(|cycle| cycle.iop_idx).collect()
}

#[allow(clippy::too_many_arguments)]
fn dispatch_recursion_witgen_gpu_verify_mem_candidate(
    hal: &WebGpuHal,
    plan: &super::rust_kernels::WomGpuVerifyPlan,
    preflight: &RawPreflightTrace,
    ctrl: &WebGpuBuffer<BabyBearElem>,
    data: &WebGpuBuffer<BabyBearElem>,
    global: &WebGpuBuffer<BabyBearElem>,
) -> Result<()> {
    ensure!(
        plan.work_cycles == preflight.num_cycles,
        "recursion WOM candidate plan cycles {} disagree with preflight cycles {}",
        plan.work_cycles,
        preflight.num_cycles
    );
    ensure!(
        plan.work_cycles <= plan.total_cycles,
        "recursion WOM candidate work_cycles {} exceeds total_cycles {}",
        plan.work_cycles,
        plan.total_cycles
    );
    ensure!(
        plan.cycle_prefixes.len() == plan.work_cycles as usize,
        "recursion WOM candidate cycle-prefix length {} disagrees with work_cycles {}",
        plan.cycle_prefixes.len(),
        plan.work_cycles
    );
    ensure!(
        !plan.bucket_bases.is_empty(),
        "recursion WOM candidate requires at least one WOM bucket"
    );
    ensure!(
        plan.valid_rows > 0,
        "recursion WOM candidate produced no rows"
    );

    ensure_storage_binding_fits(hal, ctrl, "ctrl")?;
    ensure_storage_binding_fits(hal, data, "data")?;
    ensure_storage_binding_fits(hal, global, "global")?;
    let (_, max_storage_buffer_binding_size, _) = hal.performance_limits();
    let accum_byte_len = (plan.total_cycles as u64)
        .checked_mul(CIRCUIT.accum_size() as u64)
        .and_then(|elems| elems.checked_mul(std::mem::size_of::<BabyBearElem>() as u64))
        .context("recursion WOM candidate accum byte length overflow")?;
    ensure!(
        accum_byte_len <= max_storage_buffer_binding_size,
        "recursion WOM candidate requires GPU accumulation, but accum would exceed one storage binding: {accum_byte_len} > {max_storage_buffer_binding_size}"
    );

    ctrl.sync_cpu_to_gpu(hal)
        .context("recursion WOM candidate ctrl upload")?;
    data.sync_cpu_to_gpu(hal)
        .context("recursion WOM candidate data upload")?;
    global
        .sync_cpu_to_gpu(hal)
        .context("recursion WOM candidate global upload")?;

    let ctrl_gpu = ctrl
        .raw_buffer()
        .context("recursion WOM candidate ctrl GPU buffer missing")?;
    let data_gpu = data
        .raw_buffer()
        .context("recursion WOM candidate data GPU buffer missing")?;
    let global_gpu = global
        .raw_buffer()
        .context("recursion WOM candidate global GPU buffer missing")?;

    let preflight_wom_words = pack_ext_words(preflight.wom, preflight.num_woms);
    let iop_words = pack_ext_words(preflight.iops, preflight.num_iops);
    let iop_cursors = pack_iop_cursors(preflight);
    let work_cycles = plan.work_cycles as usize;
    let unsorted_row_words = work_cycles
        .checked_mul(9)
        .and_then(|rows| rows.checked_mul(5))
        .context("recursion WOM candidate unsorted row word count overflow")?;
    let sorted_row_words = (plan.valid_rows as usize)
        .checked_mul(5)
        .context("recursion WOM candidate sorted row word count overflow")?;

    macro_rules! create_words_buffer {
        ($label:literal, $words:expr) => {
            hal.create_storage_buffer($label, nonempty_u32_byte_len($words)?)?
        };
    }
    macro_rules! upload_words {
        ($buffer:expr, $label:literal, $words:expr) => {{
            let words = $words;
            if !words.is_empty() {
                hal.write_buffer_named($buffer, $label, 0, bytemuck::cast_slice(words))?;
            }
        }};
    }

    let mix_buf = create_words_buffer!("recursion_witgen_candidate_mix", 1);
    let wom_buf = create_words_buffer!("recursion_witgen_candidate_wom", work_cycles * 4);
    let accum_buf = create_words_buffer!("recursion_witgen_candidate_accum", 1);
    let preflight_wom_buf = create_words_buffer!(
        "recursion_witgen_candidate_preflight_wom",
        preflight_wom_words.len()
    );
    let iop_buf = create_words_buffer!("recursion_witgen_candidate_iop_body", iop_words.len());
    let iop_cursor_buf = create_words_buffer!(
        "recursion_witgen_candidate_iop_cursors",
        iop_cursors.len()
    );
    let unsorted_rows_buf = create_words_buffer!(
        "recursion_witgen_candidate_unsorted_rows",
        unsorted_row_words
    );
    let row_cursors_buf =
        create_words_buffer!("recursion_witgen_candidate_row_cursors", work_cycles);
    let sorted_rows_buf =
        create_words_buffer!("recursion_witgen_candidate_sorted_rows", sorted_row_words);
    let counters_buf = create_words_buffer!(
        "recursion_witgen_candidate_bucket_counters",
        plan.bucket_bases.len()
    );
    let bucket_bases_buf = create_words_buffer!(
        "recursion_witgen_candidate_bucket_bases",
        plan.bucket_bases.len()
    );
    let cycle_prefixes_buf = create_words_buffer!(
        "recursion_witgen_candidate_cycle_prefixes",
        plan.cycle_prefixes.len()
    );
    let verify_cursors_buf =
        create_words_buffer!("recursion_witgen_candidate_verify_cursors", work_cycles);

    upload_words!(
        &preflight_wom_buf,
        "recursion_witgen_candidate_preflight_wom",
        preflight_wom_words.as_slice()
    );
    upload_words!(
        &iop_buf,
        "recursion_witgen_candidate_iop_body",
        iop_words.as_slice()
    );
    upload_words!(
        &iop_cursor_buf,
        "recursion_witgen_candidate_iop_cursors",
        iop_cursors.as_slice()
    );
    upload_words!(
        &bucket_bases_buf,
        "recursion_witgen_candidate_bucket_bases",
        plan.bucket_bases.as_slice()
    );
    upload_words!(
        &cycle_prefixes_buf,
        "recursion_witgen_candidate_cycle_prefixes",
        plan.cycle_prefixes.as_slice()
    );

    let params = [
        plan.total_cycles,
        1,
        plan.total_cycles,
        1,
        plan.work_cycles,
        plan.total_cycles,
        0,
        plan.work_cycles,
    ];
    let params_bytes: &[u8] = bytemuck::cast_slice(&params);
    let params_buf = hal.create_uniform_buffer("recursion_witgen_candidate_params", params_bytes)?;

    let exec_layout = hal.create_bind_group_layout(
        "recursion_witgen_candidate_exec_layout",
        &[
            WebGpuBindingLayout::storage(0, 0),
            WebGpuBindingLayout::storage(1, 0),
            WebGpuBindingLayout::storage(2, 0),
            WebGpuBindingLayout::storage(3, 0),
            WebGpuBindingLayout::storage(4, 0),
            WebGpuBindingLayout::storage(5, 0),
            WebGpuBindingLayout::uniform(6, params_bytes.len() as u64),
            WebGpuBindingLayout::read_only_storage(7, 0),
            WebGpuBindingLayout::read_only_storage(8, 0),
            WebGpuBindingLayout::storage(9, 0),
            WebGpuBindingLayout::storage(10, 0),
            WebGpuBindingLayout::storage(11, 0),
            WebGpuBindingLayout::storage(12, 0),
            WebGpuBindingLayout::storage(13, 0),
            WebGpuBindingLayout::read_only_storage(14, 0),
            WebGpuBindingLayout::read_only_storage(15, 0),
        ],
    )?;
    let verify_layout = hal.create_bind_group_layout(
        "recursion_witgen_candidate_verify_layout",
        &[
            WebGpuBindingLayout::storage(0, 0),
            WebGpuBindingLayout::storage(1, 0),
            WebGpuBindingLayout::storage(2, 0),
            WebGpuBindingLayout::storage(3, 0),
            WebGpuBindingLayout::storage(4, 0),
            WebGpuBindingLayout::storage(5, 0),
            WebGpuBindingLayout::uniform(6, params_bytes.len() as u64),
            WebGpuBindingLayout::read_only_storage(7, 0),
            WebGpuBindingLayout::storage(8, 0),
            WebGpuBindingLayout::read_only_storage(9, 0),
        ],
    )?;

    let poseidon_module = recursion_accum_wgsl(
        RECURSION_STEP_EXEC_POSEIDON2_CHAIN_WGSL,
        &recursion_exec_wom_candidate_extra_wgsl("recursion_step_exec_poseidon2_chain_main"),
    );
    let micro_module = recursion_accum_wgsl(
        RECURSION_STEP_EXEC_MICRO_OPS_WGSL,
        &recursion_exec_wom_candidate_extra_wgsl("recursion_step_exec_micro_ops_main"),
    );
    let macro_module = recursion_accum_wgsl(
        RECURSION_STEP_EXEC_MACRO_OPS_WGSL,
        &recursion_exec_wom_candidate_extra_wgsl("recursion_step_exec_macro_ops_main"),
    );
    let checked_bytes_module =
        recursion_accum_wgsl("", RECURSION_CHECKED_BYTES_WOM_CANDIDATE_EXTRA_WGSL);
    let verify_module = recursion_accum_wgsl(
        RECURSION_STEP_VERIFY_MEM_WGSL,
        RECURSION_VERIFY_MEM_WOM_PROBE_EXTRA_WGSL,
    );

    let poseidon_kernel = hal.create_compute_kernel(
        "recursion_witgen_candidate_poseidon2_rows",
        &poseidon_module,
        "recursion_step_exec_poseidon2_chain_main",
        &[exec_layout.clone()],
    )?;
    let micro_kernel = hal.create_compute_kernel(
        "recursion_witgen_candidate_micro_rows",
        &micro_module,
        "recursion_step_exec_micro_ops_main",
        &[exec_layout.clone()],
    )?;
    let macro_kernel = hal.create_compute_kernel(
        "recursion_witgen_candidate_macro_rows",
        &macro_module,
        "recursion_step_exec_macro_ops_main",
        &[exec_layout.clone()],
    )?;
    let checked_bytes_kernel = hal.create_compute_kernel(
        "recursion_witgen_candidate_checked_bytes_rows",
        &checked_bytes_module,
        "recursion_checked_bytes_wom_rows_combined_main",
        &[exec_layout.clone()],
    )?;
    let scatter_kernel = hal.create_compute_kernel(
        "recursion_witgen_candidate_scatter",
        &poseidon_module,
        "recursion_witgen_wom_scatter_main",
        &[exec_layout.clone()],
    )?;
    let backfill_kernel = hal.create_compute_kernel(
        "recursion_witgen_candidate_backfill",
        &poseidon_module,
        "recursion_witgen_wom_backfill_main",
        &[exec_layout.clone()],
    )?;
    let verify_kernel = hal.create_compute_kernel(
        "recursion_witgen_candidate_verify_mem",
        &verify_module,
        "recursion_step_verify_mem_main",
        &[verify_layout.clone()],
    )?;

    let exec_bind_group = hal.create_bind_group(
        "recursion_witgen_candidate_exec_bg",
        &exec_layout,
        &[
            WebGpuBufferBinding::new(0, ctrl_gpu),
            WebGpuBufferBinding::new(1, global_gpu),
            WebGpuBufferBinding::new(2, data_gpu),
            WebGpuBufferBinding::new(3, &mix_buf),
            WebGpuBufferBinding::new(4, &wom_buf),
            WebGpuBufferBinding::new(5, &accum_buf),
            WebGpuBufferBinding::new(6, &params_buf),
            WebGpuBufferBinding::new(7, &preflight_wom_buf),
            WebGpuBufferBinding::new(8, &iop_buf),
            WebGpuBufferBinding::new(9, &iop_cursor_buf),
            WebGpuBufferBinding::new(10, &unsorted_rows_buf),
            WebGpuBufferBinding::new(11, &row_cursors_buf),
            WebGpuBufferBinding::new(12, &sorted_rows_buf),
            WebGpuBufferBinding::new(13, &counters_buf),
            WebGpuBufferBinding::new(14, &bucket_bases_buf),
            WebGpuBufferBinding::new(15, &cycle_prefixes_buf),
        ],
    )?;
    let verify_bind_group = hal.create_bind_group(
        "recursion_witgen_candidate_verify_bg",
        &verify_layout,
        &[
            WebGpuBufferBinding::new(0, ctrl_gpu),
            WebGpuBufferBinding::new(1, global_gpu),
            WebGpuBufferBinding::new(2, data_gpu),
            WebGpuBufferBinding::new(3, &mix_buf),
            WebGpuBufferBinding::new(4, &wom_buf),
            WebGpuBufferBinding::new(5, &accum_buf),
            WebGpuBufferBinding::new(6, &params_buf),
            WebGpuBufferBinding::new(7, &sorted_rows_buf),
            WebGpuBufferBinding::new(8, &verify_cursors_buf),
            WebGpuBufferBinding::new(9, &cycle_prefixes_buf),
        ],
    )?;

    let row_workgroups = plan.work_cycles.div_ceil(64);
    let scatter_workgroups = plan
        .work_cycles
        .checked_mul(9)
        .context("recursion WOM candidate scatter workgroup overflow")?
        .div_ceil(64);
    let mut dispatches = 0u64;

    let _timer =
        WebGpuStageTimer::new_active_for("recursion_witgen gpu_verify_mem_candidate_sequence", hal);
    let mut gpu_verify_mem_dispatches = vec![
        (&poseidon_kernel, &exec_bind_group, row_workgroups),
        (&micro_kernel, &exec_bind_group, row_workgroups),
        (&macro_kernel, &exec_bind_group, row_workgroups),
        (&checked_bytes_kernel, &exec_bind_group, row_workgroups),
        (&scatter_kernel, &exec_bind_group, scatter_workgroups),
    ];
    dispatches += 5;
    if plan.work_cycles > 1 {
        gpu_verify_mem_dispatches.push((&backfill_kernel, &exec_bind_group, row_workgroups));
        dispatches += 1;
    }
    gpu_verify_mem_dispatches.push((&verify_kernel, &verify_bind_group, row_workgroups));
    dispatches += 1;
    hal.dispatch_compute_1d_bind_group_sequence(gpu_verify_mem_dispatches.as_slice());

    data.mark_gpu_dirty();
    RECURSION_WITGEN_GPU_VERIFY_MEM_CANDIDATE_DISPATCHES
        .fetch_add(dispatches, AtomicOrdering::SeqCst);
    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
        "recursion_witgen_gpu_verify_mem_candidate work_cycles={} valid_rows={} buckets={} dispatches={}",
        plan.work_cycles,
        plan.valid_rows,
        plan.bucket_bases.len(),
        dispatches
    ));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn dispatch_recursion_accum_gpu(
    hal: &WebGpuHal,
    work_cycles: u32,
    total_cycles: u32,
    ctrl: &WebGpuBuffer<BabyBearElem>,
    global: &WebGpuBuffer<BabyBearElem>,
    data: &WebGpuBuffer<BabyBearElem>,
    mix: &WebGpuBuffer<BabyBearElem>,
    accum: &WebGpuBuffer<BabyBearElem>,
) -> Result<()> {
    ensure!(
        work_cycles <= total_cycles,
        "recursion accumulator work_cycles {work_cycles} exceeds total_cycles {total_cycles}"
    );
    if work_cycles == 0 {
        return Ok(());
    }

    ensure_storage_binding_fits(hal, ctrl, "ctrl")?;
    ensure_storage_binding_fits(hal, global, "global")?;
    ensure_storage_binding_fits(hal, data, "data")?;
    ensure_storage_binding_fits(hal, mix, "mix")?;
    ensure_storage_binding_fits(hal, accum, "accum")?;

    ctrl.sync_cpu_to_gpu(hal)
        .context("recursion accumulator ctrl upload")?;
    global
        .sync_cpu_to_gpu(hal)
        .context("recursion accumulator global upload")?;
    data.sync_cpu_to_gpu(hal)
        .context("recursion accumulator data upload")?;
    mix.sync_cpu_to_gpu(hal)
        .context("recursion accumulator mix upload")?;
    accum
        .sync_cpu_to_gpu(hal)
        .context("recursion accumulator accum upload")?;

    let wom_init = vec![BabyBearExtElem::ONE; work_cycles as usize];
    let wom = hal.copy_from_extelem("recursion_accum_wom", &wom_init);
    ensure_storage_binding_fits(hal, &wom, "wom")?;

    let ctrl_gpu = ctrl
        .raw_buffer()
        .context("recursion accumulator ctrl GPU buffer missing")?;
    let global_gpu = global
        .raw_buffer()
        .context("recursion accumulator global GPU buffer missing")?;
    let data_gpu = data
        .raw_buffer()
        .context("recursion accumulator data GPU buffer missing")?;
    let mix_gpu = mix
        .raw_buffer()
        .context("recursion accumulator mix GPU buffer missing")?;
    let wom_gpu = wom
        .raw_buffer()
        .context("recursion accumulator WOM GPU buffer missing")?;
    let accum_gpu = accum
        .raw_buffer()
        .context("recursion accumulator accum GPU buffer missing")?;

    let (compute_module, verify_module) = recursion_accum_wgsl_modules_for_test();
    let params = [
        total_cycles,
        1,
        total_cycles,
        1,
        work_cycles,
        total_cycles,
        0,
        work_cycles,
    ];
    let params_bytes: &[u8] = bytemuck::cast_slice(&params);
    let params_buf = hal.create_uniform_buffer("recursion_accum_params", params_bytes)?;
    let layout = hal.create_bind_group_layout(
        "recursion_accum_layout",
        &[
            WebGpuBindingLayout::storage(0, 0),
            WebGpuBindingLayout::storage(1, 0),
            WebGpuBindingLayout::storage(2, 0),
            WebGpuBindingLayout::storage(3, 0),
            WebGpuBindingLayout::storage(4, 0),
            WebGpuBindingLayout::storage(5, 0),
            WebGpuBindingLayout::uniform(6, params_bytes.len() as u64),
        ],
    )?;
    let compute_kernel = hal.create_compute_kernel(
        "recursion_step_compute_accum",
        &compute_module,
        "recursion_step_compute_accum_main",
        &[layout.clone()],
    )?;
    let verify_kernel = hal.create_compute_kernel(
        "recursion_step_verify_accum",
        &verify_module,
        "recursion_step_verify_accum_main",
        &[layout.clone()],
    )?;
    let bind_group = hal.create_bind_group(
        "recursion_accum_bind_group",
        &layout,
        &[
            WebGpuBufferBinding::new(0, ctrl_gpu),
            WebGpuBufferBinding::new(1, global_gpu),
            WebGpuBufferBinding::new(2, data_gpu),
            WebGpuBufferBinding::new(3, mix_gpu),
            WebGpuBufferBinding::new(4, wom_gpu),
            WebGpuBufferBinding::new(5, accum_gpu),
            WebGpuBufferBinding::new(6, &params_buf),
        ],
    )?;
    let workgroups = work_cycles.div_ceil(64);

    {
        let _timer =
            WebGpuStageTimer::new_active_for("recursion_accumulate gpu_compute_accum", hal);
        hal.dispatch_compute_1d(&compute_kernel, &bind_group, workgroups);
    }
    wom.mark_gpu_dirty();

    {
        let _gpu_scope = hal.gpu_authoritative_scope(true);
        let _timer = WebGpuStageTimer::new_active_for("recursion_accumulate gpu_prefix", hal);
        hal.prefix_products(&wom);
    }

    {
        let _timer = WebGpuStageTimer::new_active_for("recursion_accumulate gpu_verify_accum", hal);
        hal.dispatch_compute_1d(&verify_kernel, &bind_group, workgroups);
    }
    accum.mark_gpu_dirty();
    RECURSION_ACCUM_GPU_DISPATCHES.fetch_add(2, AtomicOrdering::SeqCst);
    Ok(())
}

fn ensure_storage_binding_fits<T>(
    hal: &WebGpuHal,
    buffer: &WebGpuBuffer<T>,
    label: &'static str,
) -> Result<()>
where
    T: Clone + std::fmt::Debug + PartialEq,
{
    let (_, max_storage_buffer_binding_size, _) = hal.performance_limits();
    let byte_len = (buffer.size() as u64)
        .checked_mul(std::mem::size_of::<T>() as u64)
        .context("recursion accumulator storage binding byte length overflow")?;
    ensure!(
        byte_len <= max_storage_buffer_binding_size,
        "recursion accumulator {label} buffer too large for one storage binding: {byte_len} > {max_storage_buffer_binding_size}"
    );
    Ok(())
}

fn storage_binding_fits<T>(hal: &WebGpuHal, buffer: &WebGpuBuffer<T>) -> Result<bool>
where
    T: Clone + std::fmt::Debug + PartialEq,
{
    let (_, max_storage_buffer_binding_size, _) = hal.performance_limits();
    let byte_len = (buffer.size() as u64)
        .checked_mul(std::mem::size_of::<T>() as u64)
        .context("recursion accumulator storage binding byte length overflow")?;
    Ok(byte_len <= max_storage_buffer_binding_size)
}

#[allow(clippy::too_many_arguments)]
fn recursion_accum_storage_bindings_fit(
    hal: &WebGpuHal,
    ctrl: &WebGpuBuffer<BabyBearElem>,
    global: &WebGpuBuffer<BabyBearElem>,
    data: &WebGpuBuffer<BabyBearElem>,
    mix: &WebGpuBuffer<BabyBearElem>,
    accum: &WebGpuBuffer<BabyBearElem>,
) -> Result<bool> {
    Ok(storage_binding_fits(hal, ctrl)?
        && storage_binding_fits(hal, global)?
        && storage_binding_fits(hal, data)?
        && storage_binding_fits(hal, mix)?
        && storage_binding_fits(hal, accum)?)
}

#[derive(Default)]
#[allow(dead_code)]
pub(crate) struct WebGpuCircuitHal;

impl WebGpuCircuitEvalCheck for WebGpuCircuitHal {
    fn eval_check_webgpu(
        &self,
        hal: &WebGpuHal,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        globals: &[&WebGpuBuffer<BabyBearElem>],
        poly_mix: BabyBearExtElem,
        po2: usize,
        steps: usize,
    ) -> Result<bool> {
        hal.dispatch_eval_check_poly_ext(
            check,
            groups,
            globals,
            TAPSET,
            &crate::poly_ext::DEF,
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
        total_cycles: u32,
        preflight: &RawPreflightTrace,
        byte_reads: &BTreeMap<usize, Vec<u32>>,
        ctrl: &WebGpuBuffer<BabyBearElem>,
        data: &WebGpuBuffer<BabyBearElem>,
        global: &WebGpuBuffer<BabyBearElem>,
    ) -> Result<()> {
        let _timer = WebGpuStageTimer::new(format!(
            "recursion_witgen mode={} total_cycles={} preflight_cycles={} wom={} iops={}",
            step_mode_label(mode),
            total_cycles,
            preflight.num_cycles,
            preflight.num_woms,
            preflight.num_iops
        ));
        if RECURSION_WITGEN_GPU_VERIFY_MEM_CANDIDATE_ENABLED.load(AtomicOrdering::SeqCst) {
            let plan = super::rust_kernels::generate_witness_exec_plan(
                mode,
                total_cycles,
                preflight,
                byte_reads,
                ctrl,
                data,
                global,
            )?;
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "recursion_witgen_gpu_verify_mem_candidate_plan work_cycles={} total_cycles={} valid_rows={} buckets={}",
                plan.work_cycles,
                plan.total_cycles,
                plan.valid_rows,
                plan.bucket_bases.len()
            ));
            RECURSION_WITGEN_GPU_VERIFY_MEM_PLAN.with(|slot| {
                *slot.borrow_mut() = Some(plan);
            });
            return Ok(());
        }
        super::rust_kernels::generate_witness(
            mode,
            total_cycles,
            preflight,
            byte_reads,
            ctrl,
            data,
            global,
        )
    }

    fn post_witness_zeroize(
        &self,
        hal: &WebGpuHal,
        mode: StepMode,
        total_cycles: u32,
        preflight: &RawPreflightTrace,
        _byte_reads: &BTreeMap<usize, Vec<u32>>,
        ctrl: &WebGpuBuffer<BabyBearElem>,
        data: &WebGpuBuffer<BabyBearElem>,
        global: &WebGpuBuffer<BabyBearElem>,
    ) -> Result<()> {
        if RECURSION_WITGEN_POST_ZEROIZE_HOOK_PROBE_ENABLED.load(AtomicOrdering::SeqCst) {
            RECURSION_WITGEN_POST_ZEROIZE_HOOK_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "recursion_witgen_post_zeroize_hook mode={} total_cycles={} preflight_cycles={} wom={} iops={}",
                step_mode_label(mode),
                total_cycles,
                preflight.num_cycles,
                preflight.num_woms,
                preflight.num_iops
            ));
        }
        if RECURSION_WITGEN_GPU_VERIFY_MEM_CANDIDATE_ENABLED.load(AtomicOrdering::SeqCst) {
            let plan = RECURSION_WITGEN_GPU_VERIFY_MEM_PLAN
                .with(|slot| slot.borrow_mut().take())
                .context("recursion WOM candidate missing exec plan")?;
            ensure!(
                plan.total_cycles == total_cycles,
                "recursion WOM candidate plan total_cycles {} disagree with post-zeroize total_cycles {}",
                plan.total_cycles,
                total_cycles
            );
            dispatch_recursion_witgen_gpu_verify_mem_candidate(
                hal, &plan, preflight, ctrl, data, global,
            )?;
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "recursion_witgen_gpu_verify_mem_candidate_done mode={} total_cycles={} work_cycles={} valid_rows={}",
                step_mode_label(mode),
                total_cycles,
                plan.work_cycles,
                plan.valid_rows
            ));
        }
        Ok(())
    }
}

impl CircuitAccumulator<WebGpuHal> for WebGpuCircuitHal {
    fn accumulate(
        &self,
        hal: &WebGpuHal,
        work_cycles: u32,
        total_cycles: u32,
        ctrl: &WebGpuBuffer<BabyBearElem>,
        global: &WebGpuBuffer<BabyBearElem>,
        data: &WebGpuBuffer<BabyBearElem>,
        mix: &WebGpuBuffer<BabyBearElem>,
        accum: &WebGpuBuffer<BabyBearElem>,
    ) -> Result<CircuitAccumulationMode> {
        let _timer = WebGpuStageTimer::new(format!(
            "recursion_accumulate work_cycles={} total_cycles={}",
            work_cycles, total_cycles
        ));
        if RECURSION_ACCUM_GPU_ENABLED.load(AtomicOrdering::SeqCst)
            && recursion_accum_storage_bindings_fit(hal, ctrl, global, data, mix, accum)?
        {
            dispatch_recursion_accum_gpu(
                hal,
                work_cycles,
                total_cycles,
                ctrl,
                global,
                data,
                mix,
                accum,
            )?;
            return Ok(CircuitAccumulationMode::GpuAuthoritative);
        }
        super::rust_kernels::accumulate(work_cycles, total_cycles, ctrl, global, data, mix, accum)
            .map(|_| CircuitAccumulationMode::Default)
    }

    fn zeroize_after_accumulate(
        &self,
        hal: &WebGpuHal,
        mode: CircuitAccumulationMode,
        accum: &WebGpuBuffer<BabyBearElem>,
        global: &WebGpuBuffer<BabyBearElem>,
    ) {
        if mode == CircuitAccumulationMode::GpuAuthoritative {
            let _gpu_scope = hal.gpu_authoritative_scope(true);
            hal.eltwise_zeroize_elem(accum);
            hal.eltwise_zeroize_elem(global);
            return;
        }
        hal.eltwise_zeroize_elem(accum);
        hal.eltwise_zeroize_elem(global);
    }
}

impl CircuitHal<WebGpuHal> for WebGpuCircuitHal {
    fn eval_check(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        globals: &[&WebGpuBuffer<BabyBearElem>],
        poly_mix: BabyBearExtElem,
        po2: usize,
        steps: usize,
    ) {
        let _timer = WebGpuStageTimer::new(format!(
            "recursion_eval_check po2={} steps={} domain={}",
            po2,
            steps,
            steps * risc0_zkp::INV_RATE
        ));
        risc0_zkp::hal::portable::eval_check::<WebGpuHal, CircuitImpl>(
            &CircuitImpl::new(),
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
        _ctrl: &WebGpuBuffer<BabyBearElem>,
        _io: &WebGpuBuffer<BabyBearElem>,
        _data: &WebGpuBuffer<BabyBearElem>,
        _mix: &WebGpuBuffer<BabyBearElem>,
        _accum: &WebGpuBuffer<BabyBearElem>,
        _steps: usize,
    ) {
        unimplemented!("browser WebGPU recursion accumulation kernel is not wired yet")
    }
}

struct WebGpuRecursionProver {
    hal: Rc<WebGpuHal>,
    circuit_hal: Rc<WebGpuCircuitHal>,
}

impl RecursionProver for WebGpuRecursionProver {
    fn prove(
        &self,
        program: crate::prove::Program,
        input: std::collections::VecDeque<u32>,
    ) -> Result<RecursionReceipt> {
        let delegate = RecursionProverImpl::new(self.hal.clone(), self.circuit_hal.clone());
        delegate.prove(program, input)
    }

    fn prove_with_control_id(
        &self,
        program: crate::prove::Program,
        input: std::collections::VecDeque<u32>,
        control_id: Option<risc0_zkp::core::digest::Digest>,
    ) -> Result<RecursionReceipt> {
        let delegate = RecursionProverImpl::new(self.hal.clone(), self.circuit_hal.clone());
        delegate.prove_with_control_id(program, input, control_id)
    }

    fn prove_async<'a>(
        &'a self,
        program: crate::prove::Program,
        input: std::collections::VecDeque<u32>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<RecursionReceipt>> + 'a>> {
        self.prove_async_with_control_id(program, input, None)
    }

    fn prove_async_with_control_id<'a>(
        &'a self,
        program: crate::prove::Program,
        input: std::collections::VecDeque<u32>,
        control_id: Option<risc0_zkp::core::digest::Digest>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<RecursionReceipt>> + 'a>> {
        Box::pin(async move {
            risc0_core::scope!("prove");

            let mut preflight = Preflight::new(input);
            for (cycle, row) in program.code_by_row().enumerate() {
                preflight.step(cycle, row)?;
            }

            let witgen = crate::prove::witgen::WitnessGenerator::new(
                self.hal.as_ref(),
                self.circuit_hal.as_ref(),
                &program,
                &preflight,
                control_id,
            )?;

            let global = &witgen.global;
            let hashfn = &self.hal.get_hash_suite().hashfn;
            let mut prover = Prover::new(self.hal.as_ref(), TAPSET);

            prover
                .iop()
                .commit(&hashfn.hash_elem_slice(&PROOF_SYSTEM_INFO.encode()));
            prover
                .iop()
                .commit(&hashfn.hash_elem_slice(&CircuitImpl::CIRCUIT_INFO.encode()));

            let global_len = global.size();
            let mut header = vec![BabyBearElem::ZERO; global_len + 1];
            global.view_mut(|view| {
                for (i, elem) in view.iter_mut().enumerate() {
                    *elem = elem.valid_or_zero();
                    header[i] = *elem;
                }
                header[global_len] = BabyBearElem::new_raw(program.po2 as u32);
            });

            let header_digest = hashfn.hash_elem_slice(&header);
            prover.iop().commit(&header_digest);
            prover.iop().write_field_elem_slice(header.as_slice());
            prover.set_po2(program.po2);

            {
                let _gpu_scope = self.hal.gpu_authoritative_scope(true);
                {
                    let _t = WebGpuStageTimer::new_active_for(
                        "commit_group_async recursion_ctrl",
                        self.hal.as_ref(),
                    );
                    prover
                        .commit_group_async(REGISTER_GROUP_CTRL, &witgen.ctrl)
                        .await?;
                }
                {
                    let _t = WebGpuStageTimer::new_active_for(
                        "commit_group_async recursion_data",
                        self.hal.as_ref(),
                    );
                    prover
                        .commit_group_async(REGISTER_GROUP_DATA, &witgen.data)
                        .await?;
                }
            }

            let mix: [BabyBearElem; CircuitImpl::MIX_SIZE] =
                std::array::from_fn(|_| prover.iop().random_elem());
            let mix = {
                let _t = WebGpuStageTimer::new("recursion_witgen_accum");
                witgen.accum(self.hal.as_ref(), self.circuit_hal.as_ref(), &mix)?
            };

            let seal = {
                let _gpu_scope = self.hal.gpu_authoritative_scope(true);
                {
                    let _t = WebGpuStageTimer::new_active_for(
                        "commit_group_async recursion_accum",
                        self.hal.as_ref(),
                    );
                    prover
                        .commit_group_async_in_place(REGISTER_GROUP_ACCUM, witgen.accum.clone())
                        .await?;
                }
                prover
                    .finalize_async(&[&mix, global], self.circuit_hal.as_ref())
                    .await?
            };

            Ok(RecursionReceipt {
                seal,
                output: preflight.output,
            })
        })
    }
}

pub fn recursion_prover(hal: Rc<WebGpuHal>) -> Result<Box<dyn RecursionProver>> {
    Ok(Box::new(WebGpuRecursionProver {
        hal,
        circuit_hal: Rc::new(WebGpuCircuitHal),
    }))
}

fn step_mode_label(mode: StepMode) -> &'static str {
    match mode {
        StepMode::Parallel => "parallel",
        StepMode::SeqForward => "seq_forward",
        StepMode::SeqReverse => "seq_reverse",
    }
}
