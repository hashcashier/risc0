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

// Per-@compute-entry pruned WGSL module emitter.
//
// gen_zirgen (zirgen branch wgsl-gpu-backend, b956e80) emits one giant
// `steps.wgsl` containing 211 originals + 174 chunks (the MuxChunk
// pass splits every wide `zstruct.switch` into per-arm chunks). Both
// Chrome's whole-module ceiling (~2 MB) and its
// reachable-closure ceiling (~0.4 MB) sit below this 9.16 MB blob, so
// each `@compute` entry must ship in its OWN pruned module containing
// only the call-graph closure reachable from that entry.
//
// A per-leaf module-size probe showed:
// - exec_TopChunk0/1: 281 KB reach / 1.03 MB module -- sub-cliff,
//   wireable now.
// - exec_TopAccum/Extract chunks: 1.7 MB reach / 2.5 MB module --
//   over the ceilings (the validity/poly_ext glue isn't a wide mux,
//   so MuxChunk doesn't shrink it).
//
// This module ports that probe's algorithm to Rust so it can run at
// risc0 build time / call time:
//   1. Parse `fn <name>(` blocks out of `steps.wgsl` + `types.wgsl.inc`.
//   2. For each callsite to a chunked-base name, rewrite it to call a
//      specific chunk (chunk0 by default; the per-arm delta assembly
//      below selects other chunks).
//   3. Walk the transitive call-graph closure from `entry`.
//   4. Emit `prelude + types + layout + (steps fns in closure)`.
//
// The result is a self-contained WGSL module that naga validates and
// (for `exec_Top` chunks at least) clears both Tint capacity ceilings.
//
// The pruned exec_TopChunk0 module is vendored at
// `risc0/circuit/rv32im/src/zirgen/exec_top_chunk0.wgsl` (~1 MB) and
// exposed via [`EXEC_TOP_CHUNK0_WGSL`]. To get a runnable compute
// pipeline, append [`EXEC_TOP_CHUNK0_COMPUTE_ENTRY`] which adds a thin
// `@compute @workgroup_size(64) fn main` that sets `cycle = gid.x` and
// calls `exec_TopChunk0(kLayout_Top-bound, 0u)`. The real dispatch wiring
// selects the correct chunk per major opcode and feeds preflight data
// through the bound buffers.

/// Pruned `exec_TopChunk0` WGSL module (prelude + types + layout +
/// reachable-closure of `exec_TopChunk0`). ~1 MB, sub-cliff for both
/// Chrome's whole-module and reachable-closure capacity ceilings.
pub const EXEC_TOP_CHUNK0_WGSL: &str = include_str!("../zirgen/exec_top_chunk0.wgsl");

/// Pruned `exec_TopChunk1` WGSL module
/// (chunk1 of the top-level mux) -- 1.09 MB, sub-cliff. Generated via
/// `pruned_module_at_chunk(..., "exec_TopChunk1", 1)`. Together with
/// chunk0 these cover the full top-level mux; sub-chunk bases (e.g.,
/// `exec_Sha0` with only Chunk0) clamp to their max chunk index in
/// each module.
pub const EXEC_TOP_CHUNK1_WGSL: &str = include_str!("../zirgen/exec_top_chunk1.wgsl");

/// Thin `@compute` wrapper to make [`EXEC_TOP_CHUNK0_WGSL`] runnable on
/// a WebGPU compute pipeline. Concatenated at use sites; depends on the
/// names declared in the vendored module (`cycle`, `params`, `kLayout_Top`,
/// `BoundLayout_TopLayout`, `exec_TopChunk0`).
pub const EXEC_TOP_CHUNK0_COMPUTE_ENTRY: &str = r#"
@compute @workgroup_size(64)
fn exec_top_chunk0_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  cycle = gid.x;
  if (cycle >= params.data_rows) {
    return;
  }
  let bound = BoundLayout_TopLayout(kLayout_Top, buf_data);
  let _result = exec_TopChunk0(bound, buf_global);
}
"#;

/// `@compute` wrapper for [`EXEC_TOP_CHUNK1_WGSL`].
/// Symmetric with the chunk0 wrapper -- only the dispatched function
/// name changes (`exec_TopChunk1` vs `exec_TopChunk0`).
pub const EXEC_TOP_CHUNK1_COMPUTE_ENTRY: &str = r#"
@compute @workgroup_size(64)
fn exec_top_chunk1_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  cycle = gid.x;
  if (cycle >= params.data_rows) {
    return;
  }
  let bound = BoundLayout_TopLayout(kLayout_Top, buf_data);
  let _result = exec_TopChunk1(bound, buf_global);
}
"#;

/// Shared witgen baseline (prelude +
/// types.wgsl.inc + layout.wgsl.inc) used by every per-arm kernel.
/// ~791 KB. Concatenated with a per-arm delta + @compute wrapper at
/// HAL init to produce the final module passed to
/// `create_compute_kernel_async`.
pub const WITGEN_BASELINE_WGSL: &str = include_str!("../zirgen/witgen_baseline.wgsl");

/// TopAccum arm-5 kernel body. This is the pruned
/// reachable closure for `step_TopAccumArm5`, generated from
/// `steps_step_TopAccum.pruned.wgsl`; the HAL's accum arm5 probe
/// concatenates it with [`WITGEN_BASELINE_WGSL`] plus its
/// split-inverse compute entries.
pub const TOPACCUM_ARM5_WGSL: &str = include_str!("../zirgen/topaccum_arm5.wgsl");

/// Per-major-arm deltas. Each contains ONLY the
/// steps fns reachable from one major opcode arm's sub-fn (Chunk0
/// variant). Total ~38 KB average × 13 arms = ~488 KB vendored.
/// All deltas paired with [`WITGEN_BASELINE_WGSL`] at runtime via
/// [`assemble_arm_kernel`].
pub const EXEC_SHA0_CHUNK0_DELTA_WGSL: &str = include_str!("../zirgen/exec_sha0_chunk0_delta.wgsl");
pub const EXEC_CONTROL0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_control0_chunk0_delta.wgsl");
pub const EXEC_MEM0_CHUNK0_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem0_chunk0_delta.wgsl");
pub const EXEC_MEM0_CHUNK2_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem0_chunk2_delta.wgsl");
pub const EXEC_MEM0_CHUNK3_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem0_chunk3_delta.wgsl");
pub const EXEC_MEM0_CHUNK4_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem0_chunk4_delta.wgsl");
pub const EXEC_MEM0_CHUNK5_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem0_chunk5_delta.wgsl");
pub const EXEC_MEM0_CHUNK6_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem0_chunk6_delta.wgsl");
pub const EXEC_MEM0_CHUNK7_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem0_chunk7_delta.wgsl");
pub const EXEC_MEM1_CHUNK0_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem1_chunk0_delta.wgsl");
pub const EXEC_MEM1_CHUNK2_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem1_chunk2_delta.wgsl");
pub const EXEC_MEM1_CHUNK3_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem1_chunk3_delta.wgsl");
pub const EXEC_MEM1_CHUNK4_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem1_chunk4_delta.wgsl");
pub const EXEC_MEM1_CHUNK5_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem1_chunk5_delta.wgsl");
pub const EXEC_MEM1_CHUNK6_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem1_chunk6_delta.wgsl");
pub const EXEC_MEM1_CHUNK7_DELTA_WGSL: &str = include_str!("../zirgen/exec_mem1_chunk7_delta.wgsl");
pub const EXEC_MISC0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc0_chunk0_delta.wgsl");
pub const EXEC_MISC0_CHUNK2_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc0_chunk2_delta.wgsl");
pub const EXEC_MISC0_CHUNK3_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc0_chunk3_delta.wgsl");
pub const EXEC_MISC0_CHUNK4_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc0_chunk4_delta.wgsl");
pub const EXEC_MISC0_CHUNK7_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc0_chunk7_delta.wgsl");
pub const EXEC_MISC1_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc1_chunk0_delta.wgsl");
pub const EXEC_MISC2_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc2_chunk0_delta.wgsl");
pub const EXEC_MISC2_COMBINED_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc2_combined_delta.wgsl");
pub const EXEC_MISC2_CHUNK2_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc2_chunk2_delta.wgsl");
pub const EXEC_MISC2_CHUNK3_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc2_chunk3_delta.wgsl");
pub const EXEC_MISC2_CHUNK4_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc2_chunk4_delta.wgsl");
pub const EXEC_MISC2_CHUNK5_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc2_chunk5_delta.wgsl");
pub const EXEC_MISC2_CHUNK6_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc2_chunk6_delta.wgsl");
pub const EXEC_MISC2_CHUNK7_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc2_chunk7_delta.wgsl");
pub const EXEC_MUL0_CHUNK0_DELTA_WGSL: &str = include_str!("../zirgen/exec_mul0_chunk0_delta.wgsl");
pub const EXEC_DIV0_CHUNK0_DELTA_WGSL: &str = include_str!("../zirgen/exec_div0_chunk0_delta.wgsl");
pub const EXEC_BIGINT0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_bigint0_chunk0_delta.wgsl");
pub const EXEC_ECALL0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_ecall0_chunk0_delta.wgsl");
pub const EXEC_POSEIDON0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_poseidon0_chunk0_delta.wgsl");
pub const EXEC_POSEIDON1_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_poseidon1_chunk0_delta.wgsl");

/// Table of (label, delta WGSL, sub-fn name) tuples
/// for all 13 TopChunk0 major opcode arms. Used by the HAL prewarm
/// to fire `N` async create_compute_pipeline_async calls in
/// parallel and by the dispatch path to look up the right kernel
/// per cycle's major opcode.
///
/// **Index in this array = `crate::execute::platform::major` value**
/// (MISC0=0, MISC1=1, MISC2=2, MUL0=3, DIV0=4, MEM0=5, MEM1=6,
/// CONTROL0=7, ECALL0=8, POSEIDON0=9, POSEIDON1=10, SHA0=11,
/// BIGINT0=12). So `TOP_CHUNK0_ARM_DELTAS[cycle.major as usize]`
/// gives the kernel data for that cycle.
pub const TOP_CHUNK0_ARM_DELTAS: &[(&str, &str, &str)] = &[
    // 0: MISC0
    (
        "misc0_chunk0",
        EXEC_MISC0_CHUNK0_DELTA_WGSL,
        "exec_Misc0Chunk0",
    ),
    // 1: MISC1
    (
        "misc1_chunk0",
        EXEC_MISC1_CHUNK0_DELTA_WGSL,
        "exec_Misc1Chunk0",
    ),
    // 2: MISC2
    (
        "misc2_chunk0",
        EXEC_MISC2_CHUNK0_DELTA_WGSL,
        "exec_Misc2Chunk0",
    ),
    // 3: MUL0
    (
        "mul0_chunk0",
        EXEC_MUL0_CHUNK0_DELTA_WGSL,
        "exec_Mul0Chunk0",
    ),
    // 4: DIV0
    (
        "div0_chunk0",
        EXEC_DIV0_CHUNK0_DELTA_WGSL,
        "exec_Div0Chunk0",
    ),
    // 5: MEM0
    (
        "mem0_chunk0",
        EXEC_MEM0_CHUNK0_DELTA_WGSL,
        "exec_Mem0Chunk0",
    ),
    // 6: MEM1
    (
        "mem1_chunk0",
        EXEC_MEM1_CHUNK0_DELTA_WGSL,
        "exec_Mem1Chunk0",
    ),
    // 7: CONTROL0
    (
        "control0_chunk0",
        EXEC_CONTROL0_CHUNK0_DELTA_WGSL,
        "exec_Control0Chunk0",
    ),
    // 8: ECALL0
    (
        "ecall0_chunk0",
        EXEC_ECALL0_CHUNK0_DELTA_WGSL,
        "exec_ECall0Chunk0",
    ),
    // 9: POSEIDON0
    (
        "poseidon0_chunk0",
        EXEC_POSEIDON0_CHUNK0_DELTA_WGSL,
        "exec_Poseidon0Chunk0",
    ),
    // 10: POSEIDON1
    (
        "poseidon1_chunk0",
        EXEC_POSEIDON1_CHUNK0_DELTA_WGSL,
        "exec_Poseidon1Chunk0",
    ),
    // 11: SHA0
    (
        "sha0_chunk0",
        EXEC_SHA0_CHUNK0_DELTA_WGSL,
        "exec_Sha0Chunk0",
    ),
    // 12: BIGINT0
    (
        "bigint0_chunk0",
        EXEC_BIGINT0_CHUNK0_DELTA_WGSL,
        "exec_BigInt0Chunk0",
    ),
];

/// Extra per-minor MISC0 chunks used by the bounded authoritative
/// GPU-witgen replacement path. Chunk0 and chunk1 are handled by the
/// common per-arm kernels; these entries cover additional MISC0 minor
/// opcodes whose witness writes are safe to short-circuit once their
/// CPU lookup side effects are replayed.
pub const MISC0_EXTRA_CHUNK_DELTAS: &[(u8, &str, &str, &str)] = &[
    (
        2,
        "misc0_chunk2",
        EXEC_MISC0_CHUNK2_DELTA_WGSL,
        "exec_Misc0Chunk2",
    ),
    (
        3,
        "misc0_chunk3",
        EXEC_MISC0_CHUNK3_DELTA_WGSL,
        "exec_Misc0Chunk3",
    ),
    (
        4,
        "misc0_chunk4",
        EXEC_MISC0_CHUNK4_DELTA_WGSL,
        "exec_Misc0Chunk4",
    ),
    (
        7,
        "misc0_chunk7",
        EXEC_MISC0_CHUNK7_DELTA_WGSL,
        "exec_Misc0Chunk7",
    ),
];

/// Extra per-minor MISC2 chunks for diff-only correctness screening.
/// Unlike the rejected single-kernel `exec_Misc2_combined` path, these
/// run as separate dispatches so each kernel invocation starts
/// `extern_getMemoryTxn` at the correct per-cycle transaction offset.
pub const MISC2_EXTRA_CHUNK_DELTAS: &[(u8, &str, &str, &str)] = &[
    (
        2,
        "misc2_chunk2",
        EXEC_MISC2_CHUNK2_DELTA_WGSL,
        "exec_Misc2Chunk2",
    ),
    (
        3,
        "misc2_chunk3",
        EXEC_MISC2_CHUNK3_DELTA_WGSL,
        "exec_Misc2Chunk3",
    ),
    (
        4,
        "misc2_chunk4",
        EXEC_MISC2_CHUNK4_DELTA_WGSL,
        "exec_Misc2Chunk4",
    ),
    (
        5,
        "misc2_chunk5",
        EXEC_MISC2_CHUNK5_DELTA_WGSL,
        "exec_Misc2Chunk5",
    ),
    (
        6,
        "misc2_chunk6",
        EXEC_MISC2_CHUNK6_DELTA_WGSL,
        "exec_Misc2Chunk6",
    ),
    (
        7,
        "misc2_chunk7",
        EXEC_MISC2_CHUNK7_DELTA_WGSL,
        "exec_Misc2Chunk7",
    ),
];

/// Extra per-minor MEM0 chunks for diff-only correctness screening.
/// Chunk0 and chunk1 are covered by the common per-arm chunk0/chunk1
/// kernels; these complete the remaining load-op minor arms without the
/// combined-dispatch txn-order bug seen on generated multi-chunk kernels.
pub const MEM0_EXTRA_CHUNK_DELTAS: &[(u8, &str, &str, &str)] = &[
    (
        2,
        "mem0_chunk2",
        EXEC_MEM0_CHUNK2_DELTA_WGSL,
        "exec_Mem0Chunk2",
    ),
    (
        3,
        "mem0_chunk3",
        EXEC_MEM0_CHUNK3_DELTA_WGSL,
        "exec_Mem0Chunk3",
    ),
    (
        4,
        "mem0_chunk4",
        EXEC_MEM0_CHUNK4_DELTA_WGSL,
        "exec_Mem0Chunk4",
    ),
    (
        5,
        "mem0_chunk5",
        EXEC_MEM0_CHUNK5_DELTA_WGSL,
        "exec_Mem0Chunk5",
    ),
    (
        6,
        "mem0_chunk6",
        EXEC_MEM0_CHUNK6_DELTA_WGSL,
        "exec_Mem0Chunk6",
    ),
    (
        7,
        "mem0_chunk7",
        EXEC_MEM0_CHUNK7_DELTA_WGSL,
        "exec_Mem0Chunk7",
    ),
];

/// Extra per-minor MEM1 chunks for diff-only correctness screening.
/// Chunk0 and chunk1 are covered by the common per-arm chunk0/chunk1
/// kernels; these cover the remaining store-op minor arms so MEM1 can
/// be tested as a chunk-complete GPU-witgen replacement candidate.
pub const MEM1_EXTRA_CHUNK_DELTAS: &[(u8, &str, &str, &str)] = &[
    (
        2,
        "mem1_chunk2",
        EXEC_MEM1_CHUNK2_DELTA_WGSL,
        "exec_Mem1Chunk2",
    ),
    (
        3,
        "mem1_chunk3",
        EXEC_MEM1_CHUNK3_DELTA_WGSL,
        "exec_Mem1Chunk3",
    ),
    (
        4,
        "mem1_chunk4",
        EXEC_MEM1_CHUNK4_DELTA_WGSL,
        "exec_Mem1Chunk4",
    ),
    (
        5,
        "mem1_chunk5",
        EXEC_MEM1_CHUNK5_DELTA_WGSL,
        "exec_Mem1Chunk5",
    ),
    (
        6,
        "mem1_chunk6",
        EXEC_MEM1_CHUNK6_DELTA_WGSL,
        "exec_Mem1Chunk6",
    ),
    (
        7,
        "mem1_chunk7",
        EXEC_MEM1_CHUNK7_DELTA_WGSL,
        "exec_Mem1Chunk7",
    ),
];

/// Assemble a per-arm full kernel by concatenating
/// baseline + delta + the supplied @compute wrapper. The result is
/// passed to `WebGpuHal::create_compute_kernel_async` for Tint
/// compilation. Total module size = baseline + delta + wrapper ~=
/// 838 KB (matches the measured kernel size).
pub fn assemble_arm_kernel(delta: &str, compute_entry: &str) -> String {
    let mut out =
        String::with_capacity(WITGEN_BASELINE_WGSL.len() + delta.len() + compute_entry.len() + 2);
    let patched = patch_extern_get_diff_count(WITGEN_BASELINE_WGSL);
    let patched = patch_extern_get_memory_txn(&patched);
    out.push_str(&patched);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(delta);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(compute_entry);
    out
}

/// Replace the stub
/// `extern_getDiffCount` body with a buffer-backed lookup so per-arm
/// short-circuit cycles get the correct diff_count cell writes from
/// `DoCycleTable`. Stub returns 0 in WGSL but rust expects
/// `preflight.cycles[cycle].diff_count[i]` -- short-circuiting
/// without this fix produces wrong cells that break verify with
/// "Reached unreachable mux arm" downstream.
///
/// Caller is responsible for declaring `preflight_diff_count_buf` at
/// binding 7 in the @compute wrapper and uploading the buffer to it.
pub fn patch_extern_get_diff_count(wgsl: &str) -> String {
    const STUB: &str = "fn extern_getDiffCount(txn_cycle: Val) -> Val {\n  return 0u;\n}";
    const REPLACEMENT: &str = "@group(0) @binding(7) var<storage, read> preflight_diff_count_buf: array<u32>;\nfn extern_getDiffCount(txn_cycle: Val) -> Val {\n  let idx = decode(txn_cycle);\n  if (idx >= arrayLength(&preflight_diff_count_buf)) { return 0u; }\n  return encode(preflight_diff_count_buf[idx]);\n}";
    wgsl.replacen(STUB, REPLACEMENT, 1)
}

/// Replace the stub
/// `extern_getMemoryTxn` body with a per-cycle, stateful preflight
/// lookup matching rust `get_memory_txn`. Stub returns `[0,0,0,0,0]`
/// in WGSL but rust returns the next sequential preflight txn record
/// (advancing `txn_idx` within the cycle), encoded into (prev_cycle,
/// prev_word_low, prev_word_high, word_low, word_high).
///
/// Caller is responsible for declaring `preflight_txn_start` and
/// `preflight_txns_buf` at bindings 8 and 9 and uploading them.
/// `txn_call_idx` is a per-invocation `var<private>` that resets to 0
/// at the start of each lane and advances with each call.
pub fn patch_extern_get_memory_txn(wgsl: &str) -> String {
    const STUB: &str = "fn extern_getMemoryTxn(addr: Val) -> array<Val, 5> {\n  return array<Val, 5>(0u, 0u, 0u, 0u, 0u);\n}";
    const REPLACEMENT: &str = "@group(0) @binding(8) var<storage, read> preflight_txn_start: array<u32>;\n@group(0) @binding(9) var<storage, read> preflight_txns_buf: array<u32>;\nvar<private> txn_call_idx: u32 = 0u;\nfn extern_getMemoryTxn(addr: Val) -> array<Val, 5> {\n  let txn_idx = preflight_txn_start[cycle] + txn_call_idx;\n  txn_call_idx = txn_call_idx + 1u;\n  let base = txn_idx * 5u;\n  if (base + 4u >= arrayLength(&preflight_txns_buf)) {\n    return array<Val, 5>(0u, 0u, 0u, 0u, 0u);\n  }\n  return array<Val, 5>(\n    encode(preflight_txns_buf[base]),\n    encode(preflight_txns_buf[base + 1u]),\n    encode(preflight_txns_buf[base + 2u]),\n    encode(preflight_txns_buf[base + 3u]),\n    encode(preflight_txns_buf[base + 4u]),\n  );\n}";
    wgsl.replacen(STUB, REPLACEMENT, 1)
}

/// Self-contained shadow-init
/// kernel that pre-populates the 5 outer Top layout cells in
/// `data_buf` from a per-cycle preflight metadata buffer. Used as
/// pre-pass to the per-arm dispatch so that `back_Reg(1, ...)` reads
/// inside the arm sub-fns return correct values WITHOUT requiring
/// `rust_steps` to have run first.
///
/// Layout column offsets (from `kLayout_Top` const in
/// `witgen_baseline.wgsl:13529`):
/// - cycle (cycle_redef.this_cycle): col 0
/// - nextPcLow: col 14
/// - nextPcHigh: col 15
/// - nextState_0: col 16
/// - nextMachineMode: col 17
/// - isFirstCycle: col 18
///
/// preflight_meta layout: 4 u32 per cycle: [pc, state, machine_mode, packed_minor_major].
/// Built CPU-side from `preflight.cycles[i]` -- see
/// `prove::hal::webgpu::build_preflight_meta`.
///
/// Self-contained (does NOT use witgen_baseline.wgsl) so the bind
/// group only needs (data_buf, params, preflight_meta) -- 3 entries
/// instead of 6.
pub const SHADOW_INIT_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const R2: u32 = 1172168163u;
const MONT_ONE: u32 = 268435454u;

struct ShadowParams {
  data_rows: u32,
  data_cols: u32,
  _pad0: u32,
  _pad1: u32,
}

@group(0) @binding(0) var<storage, read_write> data_buf: array<u32>;
@group(0) @binding(1) var<uniform> params: ShadowParams;
@group(0) @binding(2) var<storage, read> preflight_meta: array<u32>;

fn mul_wide(a: u32, b: u32) -> vec2<u32> {
  let a_lo = a & 0xffffu;
  let a_hi = a >> 16u;
  let b_lo = b & 0xffffu;
  let b_hi = b >> 16u;
  let lo_lo = a_lo * b_lo;
  let lo_hi = a_lo * b_hi;
  let hi_lo = a_hi * b_lo;
  let hi_hi = a_hi * b_hi;
  let mid = (lo_lo >> 16u) + (lo_hi & 0xffffu) + (hi_lo & 0xffffu);
  let lo = (lo_lo & 0xffffu) | (mid << 16u);
  let hi = hi_hi + (lo_hi >> 16u) + (hi_lo >> 16u) + (mid >> 16u);
  return vec2<u32>(lo, hi);
}

fn mul(a: u32, b: u32) -> u32 {
  let product = mul_wide(a, b);
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

fn encode(a: u32) -> u32 {
  return mul(R2, a);
}

@compute @workgroup_size(64)
fn shadow_init_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let cycle = gid.x;
  if (cycle >= params.data_rows) {
    return;
  }
  // Match scatter convention from
  // `build_injector::set_cycle` -- the nextPcLow/High/state/mode cells
  // store the CURRENT cycle's pc/state/mode (NOT the next cycle's).
  // Per-cycle the arm sub-fn then OVERWRITES these with its computed
  // output (the "next" semantics is conceptual, not the cell value at
  // init time). Earlier shadow_init wrote next cycle's values which
  // broke cycle 0's wrap-around eqz (expected state=7 for padding
  // ControlDone but got state=0 for LoadRootAndNonce cycle 0).
  let base = cycle * 4u;
  let cur_pc = preflight_meta[base + 0u];
  let cur_state = preflight_meta[base + 1u];
  let cur_mode = preflight_meta[base + 2u];
  let next_pc_low = cur_pc & 0xFFFFu;
  let next_pc_high = (cur_pc >> 16u) & 0xFFFFu;
  let next_state = cur_state;
  let next_mode = cur_mode;
  let packed = preflight_meta[base + 3u];
  let major_u = packed >> 16u;
  let minor_u = packed & 0xFFFFu;

  // Column-major: data_buf[col * rows + row]
  let rows = params.data_rows;
  data_buf[ 0u * rows + cycle] = encode(cycle);           // cycle reg (col 0)
  // majorOnehot: cols 1-13 (Reg(1u)..Reg(13u))
  data_buf[ 1u * rows + cycle] = select(0u, MONT_ONE, major_u == 0u);
  data_buf[ 2u * rows + cycle] = select(0u, MONT_ONE, major_u == 1u);
  data_buf[ 3u * rows + cycle] = select(0u, MONT_ONE, major_u == 2u);
  data_buf[ 4u * rows + cycle] = select(0u, MONT_ONE, major_u == 3u);
  data_buf[ 5u * rows + cycle] = select(0u, MONT_ONE, major_u == 4u);
  data_buf[ 6u * rows + cycle] = select(0u, MONT_ONE, major_u == 5u);
  data_buf[ 7u * rows + cycle] = select(0u, MONT_ONE, major_u == 6u);
  data_buf[ 8u * rows + cycle] = select(0u, MONT_ONE, major_u == 7u);
  data_buf[ 9u * rows + cycle] = select(0u, MONT_ONE, major_u == 8u);
  data_buf[10u * rows + cycle] = select(0u, MONT_ONE, major_u == 9u);
  data_buf[11u * rows + cycle] = select(0u, MONT_ONE, major_u == 10u);
  data_buf[12u * rows + cycle] = select(0u, MONT_ONE, major_u == 11u);
  data_buf[13u * rows + cycle] = select(0u, MONT_ONE, major_u == 12u);
  data_buf[14u * rows + cycle] = encode(next_pc_low);     // nextPcLow
  data_buf[15u * rows + cycle] = encode(next_pc_high);    // nextPcHigh
  data_buf[16u * rows + cycle] = encode(next_state);      // nextState_0
  data_buf[17u * rows + cycle] = encode(next_mode);       // nextMachineMode
  let is_first = select(0u, 1u, cycle == 0u);
  data_buf[18u * rows + cycle] = encode(is_first);
  data_buf[19u * rows + cycle] = encode(major_u);         // major
  data_buf[20u * rows + cycle] = encode(minor_u);         // minor
  // minorOnehot: cols 21-28 (Reg(21u)..Reg(28u))
  data_buf[21u * rows + cycle] = select(0u, MONT_ONE, minor_u == 0u);
  data_buf[22u * rows + cycle] = select(0u, MONT_ONE, minor_u == 1u);
  data_buf[23u * rows + cycle] = select(0u, MONT_ONE, minor_u == 2u);
  data_buf[24u * rows + cycle] = select(0u, MONT_ONE, minor_u == 3u);
  data_buf[25u * rows + cycle] = select(0u, MONT_ONE, minor_u == 4u);
  data_buf[26u * rows + cycle] = select(0u, MONT_ONE, minor_u == 5u);
  data_buf[27u * rows + cycle] = select(0u, MONT_ONE, minor_u == 6u);
  data_buf[28u * rows + cycle] = select(0u, MONT_ONE, minor_u == 7u);
}
"#;

/// Per-major-arm pruned module.
/// Each major opcode arm of exec_TopChunk0 gets its own kernel whose
/// closure is restricted to the sub-fn path for that arm only.
/// Module size: ~820 KB (well under Chrome's 2 MB whole-module
/// cliff). Used in dispatch-per-arm where each kernel runs over the
/// subset of cycles with that major opcode (per preflight major
/// opcode lookup).
pub const EXEC_SHA0_CHUNK0_ONLY_WGSL: &str = include_str!("../zirgen/exec_sha0_chunk0_only.wgsl");

/// `@compute` wrapper for the Sha0 per-arm
/// kernel. Calls only exec_Sha0Chunk0; the delta modules multi-call
/// all Sha0ChunkN with OR-merge to cover all minor opcodes within
/// the Sha major arm.
pub const EXEC_SHA0_CHUNK0_ONLY_COMPUTE_ENTRY: &str = r#"
@compute @workgroup_size(64)
fn exec_sha0_chunk0_only_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  cycle = gid.x;
  if (cycle >= params.data_rows) {
    return;
  }
  let bound = BoundLayout_Sha0Layout(kLayout_Top.instResult.arm11._super, buf_data);
  let nondet = NondetRegStruct(0u);
  let inst_input = InstInputStruct(0u, 0u, ValU32Struct(0u, 0u), 0u, OneHot_5Struct(Val5Array(0u, 0u, 0u, 0u, 0u)), 0u);
  let _result = exec_Sha0Chunk0(nondet, inst_input, bound);
}
"#;

/// "All chunks" pruned module --
/// `exec_TopChunk0_all_chunks` reachable closure with each
/// `exec_BASE(...)` callsite replaced by a synthesized
/// `exec_BASE_combined(...)` helper that calls every ChunkN
/// sequentially and OR-merges the returns (assumes Tint zero-inits
/// the unreachable-arm return vars). 2.35 MB -- on the boundary of
/// Chrome's whole-module ceiling (measured 1.99-3.27 MB band).
/// Generated via `scripts/gen_all_chunks.py` (vendored in this crate).
pub const EXEC_TOP_CHUNK0_ALL_WGSL: &str = include_str!("../zirgen/exec_top_chunk0_all.wgsl");

/// `@compute` wrapper for [`EXEC_TOP_CHUNK0_ALL_WGSL`].
/// Identical body to the chunk0 wrapper -- only differs in the entry
/// point name to disambiguate the kernel cache key.
pub const EXEC_TOP_CHUNK0_ALL_COMPUTE_ENTRY: &str = r#"
@compute @workgroup_size(64)
fn exec_top_chunk0_all_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  cycle = gid.x;
  if (cycle >= params.data_rows) {
    return;
  }
  let bound = BoundLayout_TopLayout(kLayout_Top, buf_data);
  let _result = exec_TopChunk0(bound, buf_global);
}
"#;

use std::collections::{BTreeMap, BTreeSet};

/// One parsed WGSL function block: `fn <name>(...) ... { ... }` followed
/// by everything up to the next `fn` declaration.
#[derive(Debug)]
struct WgslFn<'a> {
    name: &'a str,
    body: &'a str,
}

#[derive(Debug)]
struct OwnedWgslFn {
    name: String,
    body: String,
}

fn parse_fns(text: &str) -> Vec<WgslFn<'_>> {
    let mut starts: Vec<(&str, usize)> = Vec::new();
    for (offset, _) in text.match_indices("fn ") {
        // Match only top-level `fn ` declarations: must be at column 0
        // (start of file or preceded by '\n').
        if offset != 0 && text.as_bytes()[offset - 1] != b'\n' {
            continue;
        }
        let after_fn = offset + 3;
        let rest = &text[after_fn..];
        // Name = identifier chars up to '('.
        let name_end = rest.find('(').unwrap_or(rest.len());
        let name = rest[..name_end].trim();
        if name.is_empty() || !name.chars().next().is_some_and(is_ident_start) {
            continue;
        }
        if !name.chars().all(is_ident_char) {
            continue;
        }
        starts.push((name, offset));
    }

    let mut fns = Vec::with_capacity(starts.len());
    for (i, &(name, start)) in starts.iter().enumerate() {
        let end = if i + 1 < starts.len() {
            starts[i + 1].1
        } else {
            text.len()
        };
        fns.push(WgslFn {
            name,
            body: &text[start..end],
        });
    }
    fns
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}
fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Find every `<ident>(` occurrence in `body` whose ident is in `valid`.
fn callees_in<'a>(body: &str, valid: &BTreeSet<&'a str>) -> BTreeSet<&'a str> {
    let mut out = BTreeSet::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !is_ident_start(bytes[i] as char) {
            i += 1;
            continue;
        }
        // Reject prefix-of-larger-identifier cases by confirming the
        // PRECEDING byte (if any) isn't an identifier char.
        if i > 0 && is_ident_char(bytes[i - 1] as char) {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < bytes.len() && is_ident_char(bytes[j] as char) {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b'(' {
            let ident = &body[i..j];
            if let Some(matched) = valid.get(ident) {
                out.insert(*matched);
            }
        }
        i = j.max(i + 1);
    }
    out
}

/// Replace every `<base>(` callsite in `body` (where `<base>` is in
/// `chunked_bases`) with `<base>Chunk0(`. Superseded in production by
/// the per-chunk-mux flow; kept under cfg(test) for its unit test and
/// future chunk-regeneration work.
#[cfg(test)]
fn rewrite_chunk0(body: &str, chunked_bases: &BTreeSet<&str>) -> String {
    rewrite_to_chunk(body, &chunked_bases.iter().map(|&b| (b, 0u32)).collect(), 0)
}

/// Same as [`pruned_module_at_chunk`]
/// but emits ONLY the rewritten steps fns reachable from `entry`,
/// without the prelude / types / layout prefix. The caller is
/// expected to concatenate a shared `prelude + types + layout`
/// baseline at runtime so vendoring 26+ per-arm kernels costs
/// `~baseline + N × delta_size` instead of `N × full_module_size`.
///
/// Empirically: full module ~830 KB, delta ~50-100 KB.
/// Vendoring 26 deltas + one baseline = ~3 MB vs naive 22-26 MB.
pub fn pruned_delta_at_chunk(
    types_inc: &str,
    steps: &str,
    entry: &str,
    target_chunk: u32,
) -> Result<String, PrunerError> {
    let steps_fns = parse_fns(steps);
    let types_fns = parse_fns(types_inc);
    let mut all_names: BTreeSet<&str> = BTreeSet::new();
    let mut steps_names: BTreeSet<&str> = BTreeSet::new();
    for f in &steps_fns {
        all_names.insert(f.name);
        steps_names.insert(f.name);
    }
    for f in &types_fns {
        all_names.insert(f.name);
    }
    if !steps_names.contains(entry) {
        return Err(PrunerError::EntryNotFound(entry.to_string()));
    }
    let mut chunked_max_idx: BTreeMap<&str, u32> = BTreeMap::new();
    for &n in &steps_names {
        if let Some(idx) = n.rfind("Chunk") {
            let base = &n[..idx];
            let suffix = &n[idx + "Chunk".len()..];
            if !suffix.is_empty()
                && suffix.chars().all(|c| c.is_ascii_digit())
                && steps_names.contains(base)
            {
                if let Ok(k) = suffix.parse::<u32>() {
                    chunked_max_idx
                        .entry(base)
                        .and_modify(|cur| {
                            if k > *cur {
                                *cur = k;
                            }
                        })
                        .or_insert(k);
                }
            }
        }
    }
    let rewritten_steps: BTreeMap<&str, String> = steps_fns
        .iter()
        .map(|f| {
            (
                f.name,
                rewrite_to_chunk(f.body, &chunked_max_idx, target_chunk),
            )
        })
        .collect();
    let mut calls: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for f in &steps_fns {
        let body = rewritten_steps.get(f.name).unwrap();
        calls.insert(f.name, callees_in(body, &all_names));
    }
    // Note: types_fns aren't rewritten here -- delta emits ONLY
    // steps fns. Types stay in the shared baseline.
    let mut closure: BTreeSet<&str> = BTreeSet::new();
    let mut stack: Vec<&str> = vec![entry];
    while let Some(n) = stack.pop() {
        if !closure.insert(n) {
            continue;
        }
        if let Some(cs) = calls.get(n) {
            for &c in cs {
                stack.push(c);
            }
        }
    }
    let mut out = String::with_capacity(steps.len() / 16);
    for f in &steps_fns {
        if closure.contains(f.name) {
            let body = rewritten_steps.get(f.name).unwrap();
            out.push_str(body);
            if !out.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    Ok(out)
}

/// Extract a delta-only reachable closure from an already assembled
/// all-chunks WGSL module. Functions already present in `baseline`
/// are treated as terminals because callers concatenate the returned
/// delta with [`WITGEN_BASELINE_WGSL`] before compiling the kernel.
pub fn pruned_delta_from_combined_module(
    baseline: &str,
    module: &str,
    entry: &str,
) -> Result<String, PrunerError> {
    let module_fns = parse_fns(module);
    let baseline_fns = parse_fns(baseline);

    let module_names: BTreeSet<&str> = module_fns.iter().map(|f| f.name).collect();
    let baseline_names: BTreeSet<&str> = baseline_fns.iter().map(|f| f.name).collect();
    if !module_names.contains(entry) {
        return Err(PrunerError::EntryNotFound(entry.to_string()));
    }

    let mut calls: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for f in &module_fns {
        calls.insert(f.name, callees_in(f.body, &module_names));
    }

    let mut closure: BTreeSet<&str> = BTreeSet::new();
    let mut stack: Vec<&str> = vec![entry];
    while let Some(name) = stack.pop() {
        if baseline_names.contains(name) || !closure.insert(name) {
            continue;
        }
        if let Some(callees) = calls.get(name) {
            for &callee in callees {
                stack.push(callee);
            }
        }
    }

    let mut out = String::with_capacity(module.len() / 8);
    for f in &module_fns {
        if closure.contains(f.name) && !baseline_names.contains(f.name) {
            out.push_str(f.body);
            if !out.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    Ok(out)
}

/// Generalized chunk rewrite. For each
/// callsite to a base in `chunked_max_idx`, append `ChunkK` where
/// `K = min(target_chunk, max_idx)`. The `max_idx` cap lets bases
/// with fewer chunks than `target_chunk` clamp to their last
/// available chunk (e.g., `exec_Sha0` only has Chunk0; rewriting at
/// `target_chunk=1` still produces `exec_Sha0Chunk0`).
fn rewrite_to_chunk(
    body: &str,
    chunked_max_idx: &BTreeMap<&str, u32>,
    target_chunk: u32,
) -> String {
    let bytes = body.as_bytes();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    while i < bytes.len() {
        if !is_ident_start(bytes[i] as char) || (i > 0 && is_ident_char(bytes[i - 1] as char)) {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let mut j = i;
        while j < bytes.len() && is_ident_char(bytes[j] as char) {
            j += 1;
        }
        let ident = &body[i..j];
        if j < bytes.len() && bytes[j] == b'(' {
            if let Some(&max_idx) = chunked_max_idx.get(ident) {
                let k = target_chunk.min(max_idx);
                out.push_str(ident);
                out.push_str("Chunk");
                out.push_str(&k.to_string());
            } else {
                out.push_str(ident);
            }
        } else {
            out.push_str(ident);
        }
        i = j;
    }
    out
}

/// Generate a pruned, single-major TopAccum probe from the steps-only
/// TopAccum WGSL artifact. This is the reproducible version of the
/// checked-in `topaccum_arm5.wgsl` slice, and is used to prevent
/// additional TopAccum arms from becoming hand-sliced one-offs.
pub fn topaccum_arm_probe_wgsl(steps: &str, arm: usize) -> Result<String, PrunerError> {
    const TOPACCUM_ARM_COUNT: usize = 13;
    if arm >= TOPACCUM_ARM_COUNT {
        return Err(PrunerError::InvalidArmIndex(arm));
    }

    let arm_suffix = format!("Arm{arm}");
    let mappings = [
        ("exec_TopExtract", format!("exec_TopExtract{arm_suffix}")),
        ("execUser_Accum", format!("execUser_Accum{arm_suffix}")),
        ("exec_TopAccum", format!("exec_TopAccum{arm_suffix}")),
        ("step_TopAccum", format!("step_TopAccum{arm_suffix}")),
    ];

    let parsed = parse_fns(steps);
    let mut transformed = Vec::with_capacity(parsed.len());
    for f in &parsed {
        let mut name = f.name.to_string();
        let mut body = f.body.to_string();
        match f.name {
            "exec_TopExtract" => {
                name = mappings[0].1.clone();
                body = rename_fn(&body, mappings[0].0, &mappings[0].1)?;
                body = slice_topaccum_mux_arm(
                    &body,
                    &name,
                    "lookup_TopLayout_majorOnehot(layout0)",
                    arm,
                )?;
            }
            "execUser_Accum" => {
                name = mappings[1].1.clone();
                body = rename_fn(&body, mappings[1].0, &mappings[1].1)?;
            }
            "exec_TopAccum" => {
                name = mappings[2].1.clone();
                body = rename_fn(&body, mappings[2].0, &mappings[2].1)?;
                body = slice_topaccum_mux_arm(
                    &body,
                    &name,
                    "lookup_TopInstResultLayout__selector(lookup_TopLayout_instResult(arg0))",
                    arm,
                )?;
            }
            "step_TopAccum" => {
                name = mappings[3].1.clone();
                body = rename_fn(&body, mappings[3].0, &mappings[3].1)?;
                body = body.replacen("{\n// zirgen/dsl/passes/GenerateAccum.cpp:524\n", "{\n", 1);
            }
            _ => {}
        }

        for (old, new) in &mappings {
            body = rewrite_call_name(&body, old, new);
        }

        transformed.push(OwnedWgslFn { name, body });
    }

    let entry = format!("step_TopAccum{arm_suffix}");
    let all_names: BTreeSet<&str> = transformed.iter().map(|f| f.name.as_str()).collect();
    if !all_names.contains(entry.as_str()) {
        return Err(PrunerError::EntryNotFound(entry));
    }

    let mut calls: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for f in &transformed {
        calls.insert(f.name.as_str(), callees_in(&f.body, &all_names));
    }

    let mut closure: BTreeSet<&str> = BTreeSet::new();
    let mut stack: Vec<&str> = vec![entry.as_str()];
    while let Some(n) = stack.pop() {
        if !closure.insert(n) {
            continue;
        }
        if let Some(cs) = calls.get(n) {
            for &c in cs {
                stack.push(c);
            }
        }
    }

    let compute_entry = format!(
        "\n@compute @workgroup_size(64)\nfn topaccum_arm{arm}_main(@builtin(global_invocation_id) gid: vec3<u32>) {{\n  cycle = gid.x;\n  if (cycle >= params.data_rows) {{ return; }}\n  step_TopAccum{arm_suffix}(buf_accum, buf_data, buf_global, buf_mix);\n}}\n"
    );
    let mut out = String::with_capacity(steps.len() / 12 + compute_entry.len());
    for f in &transformed {
        if closure.contains(f.name.as_str()) {
            out.push_str(&f.body);
            if !out.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    out.push_str(&compute_entry);
    Ok(out)
}

fn rename_fn(body: &str, old: &str, new: &str) -> Result<String, PrunerError> {
    let needle = format!("fn {old}(");
    let replacement = format!("fn {new}(");
    if !body.contains(&needle) {
        return Err(PrunerError::SymbolNotFound(old.to_string()));
    }
    Ok(body.replacen(&needle, &replacement, 1))
}

fn rewrite_call_name(body: &str, old: &str, new: &str) -> String {
    let bytes = body.as_bytes();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    while i < bytes.len() {
        if !is_ident_start(bytes[i] as char) || (i > 0 && is_ident_char(bytes[i - 1] as char)) {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let mut j = i;
        while j < bytes.len() && is_ident_char(bytes[j] as char) {
            j += 1;
        }
        let ident = &body[i..j];
        if ident == old && j < bytes.len() && bytes[j] == b'(' {
            out.push_str(new);
        } else {
            out.push_str(ident);
        }
        i = j;
    }
    out
}

fn slice_topaccum_mux_arm(
    body: &str,
    function_name: &str,
    marker: &str,
    arm: usize,
) -> Result<String, PrunerError> {
    let open = body
        .find('{')
        .ok_or_else(|| PrunerError::MalformedFunction(function_name.to_string()))?;
    let close = body
        .rfind('}')
        .ok_or_else(|| PrunerError::MalformedFunction(function_name.to_string()))?;
    if close <= open {
        return Err(PrunerError::MalformedFunction(function_name.to_string()));
    }

    let head = &body[..=open];
    let inner = &body[open + 1..close];
    let tail = &body[close..];
    let branch_starts = find_mux_branch_starts(inner, marker);
    if branch_starts.len() != 13 || arm >= branch_starts.len() {
        return Err(PrunerError::ArmMuxNotFound {
            function: function_name.to_string(),
            arm,
            found: branch_starts.len(),
        });
    }

    let branch_start = branch_starts[arm];
    let branch_content_start = inner[branch_start..]
        .find('\n')
        .map(|pos| branch_start + pos + 1)
        .unwrap_or(inner.len());
    let branch_end = if arm + 1 < branch_starts.len() {
        branch_starts[arm + 1]
    } else {
        find_outer_unreachable_else(inner, branch_content_start).ok_or_else(|| {
            PrunerError::ArmMuxNotFound {
                function: function_name.to_string(),
                arm,
                found: branch_starts.len(),
            }
        })?
    };
    let suffix_start = inner
        .rfind("\nreturn ")
        .map(|pos| pos + 1)
        .or_else(|| inner.find("return "))
        .ok_or_else(|| PrunerError::MalformedFunction(function_name.to_string()))?;
    if suffix_start <= branch_end {
        return Err(PrunerError::MalformedFunction(function_name.to_string()));
    }

    let mut out = String::with_capacity(body.len() / 4);
    out.push_str(head);
    out.push('\n');
    out.push_str(&inner[..branch_starts[0]]);
    out.push('\n');
    out.push_str(&inner[branch_content_start..branch_end]);
    out.push_str(&inner[suffix_start..]);
    out.push_str(tail);
    Ok(out)
}

fn find_mux_branch_starts(inner: &str, marker: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    for (idx, _) in inner.match_indices(marker) {
        let line_start = inner[..idx].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
        if starts.last().copied() != Some(line_start) {
            starts.push(line_start);
        }
    }
    starts
}

fn find_outer_unreachable_else(inner: &str, from: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut offset = 0usize;
    for line in inner.split_inclusive('\n') {
        if offset >= from && depth == 1 && line.starts_with("} else {") {
            return Some(offset);
        }
        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }
        offset += line.len();
    }
    None
}

/// Per-leaf pruned WGSL module for one `@compute` entry.
///
/// Inputs are the raw artifact strings emitted by gen_zirgen. The
/// returned module is `prelude + types + layout + (rewritten steps fns
/// in closure of `entry`)`. Defaults to chunk-0-everywhere rewrite;
/// use `pruned_module_at_chunk` to select a different chunk index.
pub fn pruned_module(
    prelude: &str,
    types_inc: &str,
    layout_inc: &str,
    steps: &str,
    entry: &str,
) -> Result<String, PrunerError> {
    pruned_module_at_chunk(prelude, types_inc, layout_inc, steps, entry, 0)
}

/// Like [`pruned_module`] but rewrites
/// chunked-base callsites to `<base>Chunk{target_chunk}` (clamped to
/// the highest available chunk index for each base). Used to emit a
/// per-chunk pruned module so a multi-kernel dispatch can cover all
/// major-opcode arms.
pub fn pruned_module_at_chunk(
    prelude: &str,
    types_inc: &str,
    layout_inc: &str,
    steps: &str,
    entry: &str,
    target_chunk: u32,
) -> Result<String, PrunerError> {
    let steps_fns = parse_fns(steps);
    let types_fns = parse_fns(types_inc);

    let mut all_names: BTreeSet<&str> = BTreeSet::new();
    let mut steps_names: BTreeSet<&str> = BTreeSet::new();
    for f in &steps_fns {
        all_names.insert(f.name);
        steps_names.insert(f.name);
    }
    for f in &types_fns {
        all_names.insert(f.name);
    }

    if !steps_names.contains(entry) {
        return Err(PrunerError::EntryNotFound(entry.to_string()));
    }

    // For each chunked base, track the highest chunk index available
    // so `rewrite_to_chunk` can clamp target_chunk when the base has
    // fewer chunks than requested.
    let mut chunked_max_idx: BTreeMap<&str, u32> = BTreeMap::new();
    for &n in &steps_names {
        if let Some(idx) = n.rfind("Chunk") {
            let base = &n[..idx];
            let suffix = &n[idx + "Chunk".len()..];
            if !suffix.is_empty()
                && suffix.chars().all(|c| c.is_ascii_digit())
                && steps_names.contains(base)
            {
                if let Ok(k) = suffix.parse::<u32>() {
                    chunked_max_idx
                        .entry(base)
                        .and_modify(|cur| {
                            if k > *cur {
                                *cur = k;
                            }
                        })
                        .or_insert(k);
                }
            }
        }
    }

    // Rewrite every fn body's callsites to chunked-bases -> chunk{target_chunk}.
    let rewritten_steps: BTreeMap<&str, String> = steps_fns
        .iter()
        .map(|f| {
            (
                f.name,
                rewrite_to_chunk(f.body, &chunked_max_idx, target_chunk),
            )
        })
        .collect();
    let rewritten_types: BTreeMap<&str, String> = types_fns
        .iter()
        .map(|f| {
            (
                f.name,
                rewrite_to_chunk(f.body, &chunked_max_idx, target_chunk),
            )
        })
        .collect();

    // Build call graph from the rewritten bodies.
    let mut calls: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for f in &steps_fns {
        let body = rewritten_steps.get(f.name).unwrap();
        calls.insert(f.name, callees_in(body, &all_names));
    }
    for f in &types_fns {
        let body = rewritten_types.get(f.name).unwrap();
        calls.insert(f.name, callees_in(body, &all_names));
    }

    // BFS the closure from `entry`.
    let mut closure: BTreeSet<&str> = BTreeSet::new();
    let mut stack: Vec<&str> = vec![entry];
    while let Some(n) = stack.pop() {
        if !closure.insert(n) {
            continue;
        }
        if let Some(cs) = calls.get(n) {
            for &c in cs {
                stack.push(c);
            }
        }
    }

    // Emit prelude + types + layout + (rewritten steps fns in closure,
    // preserving original file order for stability).
    let mut out = String::with_capacity(steps.len() / 4);
    out.push_str(prelude);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(types_inc);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(layout_inc);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    for f in &steps_fns {
        if closure.contains(f.name) {
            let body = rewritten_steps.get(f.name).unwrap();
            out.push_str(body);
            if !out.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    Ok(out)
}

#[derive(Debug)]
pub enum PrunerError {
    EntryNotFound(String),
    SymbolNotFound(String),
    InvalidArmIndex(usize),
    ArmMuxNotFound {
        function: String,
        arm: usize,
        found: usize,
    },
    MalformedFunction(String),
}

impl std::fmt::Display for PrunerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EntryNotFound(name) => {
                write!(f, "entry symbol not found in steps.wgsl: {name}")
            }
            Self::SymbolNotFound(name) => write!(f, "symbol not found in WGSL: {name}"),
            Self::InvalidArmIndex(arm) => {
                write!(f, "TopAccum arm index out of range: {arm}")
            }
            Self::ArmMuxNotFound {
                function,
                arm,
                found,
            } => write!(
                f,
                "could not isolate TopAccum arm {arm} in {function}: found {found} mux arms"
            ),
            Self::MalformedFunction(name) => write!(f, "malformed WGSL function: {name}"),
        }
    }
}

impl std::error::Error for PrunerError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Path to the gen_zirgen WGSL output -- not vendored into the
    /// crate (9 MB), but regenerable via `scripts/gen_all_chunks.py`
    /// from a zirgen checkout. Set `ZIRGEN_WGSL_OUT`
    /// to the gen_zirgen output directory (default `/tmp/zirgen-out8`)
    /// and `ZIRGEN_SRC` to a zirgen checkout; tests skip cleanly if
    /// either artifact is absent.
    fn try_load_generated_chunks() -> Option<(String, String, String, String)> {
        let dir = std::path::PathBuf::from(
            std::env::var("ZIRGEN_WGSL_OUT").unwrap_or_else(|_| "/tmp/zirgen-out8".into()),
        );
        let prelude = std::path::PathBuf::from(std::env::var("ZIRGEN_SRC").ok()?)
            .join("zirgen/compiler/codegen/gpu/witgen_prelude.wgsl");
        if !dir.join("steps.wgsl").exists() || !prelude.exists() {
            return None;
        }
        let steps = std::fs::read_to_string(dir.join("steps.wgsl")).ok()?;
        let types = std::fs::read_to_string(dir.join("types.wgsl.inc")).ok()?;
        let layout = std::fs::read_to_string(dir.join("layout.wgsl.inc")).ok()?;
        let prelude = std::fs::read_to_string(&prelude).ok()?;
        Some((prelude, types, layout, steps))
    }

    #[test]
    fn parses_a_handful_of_fns_from_a_synthetic_blob() {
        let src = "\
fn alpha() -> u32 { return 1u; }
fn beta(x: u32) -> u32 { return alpha() + x; }
fn gamma() -> u32 { return beta(2u); }
";
        let fns = parse_fns(src);
        assert_eq!(fns.len(), 3);
        assert_eq!(fns[0].name, "alpha");
        assert_eq!(fns[1].name, "beta");
        assert_eq!(fns[2].name, "gamma");
    }

    #[test]
    fn callees_picks_only_known_idents_with_open_paren() {
        let mut valid = BTreeSet::new();
        valid.insert("alpha");
        valid.insert("beta");
        let body = "fn x() { let a = alpha(); let b = beta_x(1); /* alpha is mentioned */ }";
        let cs = callees_in(body, &valid);
        // `alpha(` matches; `beta_x(` is a different ident; the comment
        // mention is not followed by `(`.
        assert!(cs.contains("alpha"));
        assert!(!cs.contains("beta"));
    }

    #[test]
    fn rewrite_chunk0_only_touches_chunked_bases() {
        let mut bases = BTreeSet::new();
        bases.insert("foo");
        let body = "let x = foo(1u); let y = bar(foo(2u));";
        let out = rewrite_chunk0(body, &bases);
        assert_eq!(out, "let x = fooChunk0(1u); let y = bar(fooChunk0(2u));");
    }

    #[test]
    fn misc2_combined_delta_extractor_covers_all_nested_chunks() {
        let delta = pruned_delta_from_combined_module(
            WITGEN_BASELINE_WGSL,
            EXEC_TOP_CHUNK0_ALL_WGSL,
            "exec_Misc2_combined",
        )
        .expect("MISC2 combined delta should extract from all-chunks module");

        assert!(delta.contains("fn exec_Misc2Chunk0("));
        assert!(delta.contains("fn exec_Misc2Chunk7("));
        assert!(delta.contains("fn exec_Misc2_combined("));
        assert!(delta.contains("fn exec_ReadSourceRegs_combined("));
        assert!(delta.contains("fn merge_InstOutputBaseStruct("));
        assert!(!delta.contains("const P: u32"));
        assert!(
            delta.len() < 512 * 1024,
            "MISC2 combined delta should stay below the reachable-closure cliff: {} bytes",
            delta.len()
        );
    }

    #[test]
    fn misc2_chunk4_delta_uses_checked_in_jalr_shape() {
        assert!(EXEC_MISC2_CHUNK4_DELTA_WGSL.contains("stale OpJALRLayout side writes"));
        assert!(
            !EXEC_MISC2_CHUNK4_DELTA_WGSL.contains("let x6: MiscOutputStruct = exec_OpJALR(x4"),
            "MISC2 chunk4 must mirror checked-in steps.rs.inc and avoid stale OpJALRLayout writes"
        );
    }

    #[test]
    fn mem0_extra_chunk_deltas_are_pruned_single_minor_entries() {
        for (minor, _label, delta, sub_fn) in MEM0_EXTRA_CHUNK_DELTAS {
            assert!(
                delta.contains(&format!("fn {sub_fn}(")),
                "MEM0 minor {minor} delta should contain its entry function"
            );
            assert!(
                !delta.contains("fn exec_Mem0_combined("),
                "MEM0 minor {minor} delta must not pull in the combined dispatcher"
            );
            assert!(
                delta.len() < 128 * 1024,
                "MEM0 minor {minor} delta is unexpectedly large: {} bytes",
                delta.len()
            );
        }
    }

    #[test]
    fn mem1_extra_chunk_deltas_are_pruned_single_minor_entries() {
        for (minor, _label, delta, sub_fn) in MEM1_EXTRA_CHUNK_DELTAS {
            assert!(
                delta.contains(&format!("fn {sub_fn}(")),
                "MEM1 minor {minor} delta should contain its entry function"
            );
            assert!(
                !delta.contains("fn exec_Mem1_combined("),
                "MEM1 minor {minor} delta must not pull in the combined dispatcher"
            );
            assert!(
                delta.len() < 128 * 1024,
                "MEM1 minor {minor} delta is unexpectedly large: {} bytes",
                delta.len()
            );
        }
    }

    #[test]
    fn mem1_chunk0_delta_uses_combined_source_registers() {
        assert!(
            EXEC_MEM1_CHUNK0_DELTA_WGSL.contains("fn exec_ReadSourceRegs_combined("),
            "MEM1 chunk0 must include the combined source-register helper"
        );
        assert!(
            EXEC_MEM1_CHUNK0_DELTA_WGSL
                .contains("let x4: ReadSourceRegsStruct = exec_ReadSourceRegs_combined("),
            "MEM1 store-byte input must not use only ReadSourceRegsChunk0"
        );
    }

    #[test]
    fn exec_top_chunk0_is_under_capacity_cliffs() {
        let Some((prelude, types, layout, steps)) = try_load_generated_chunks() else {
            eprintln!(
                "skipping: gen_zirgen output not present at /tmp/zirgen-out8 \
                 (regenerate via scripts/gen_all_chunks.py)"
            );
            return;
        };
        let module = pruned_module(&prelude, &types, &layout, &steps, "exec_TopChunk0")
            .expect("pruned module emits");
        let bytes = module.len();
        eprintln!("exec_TopChunk0 pruned module: {} bytes", bytes);
        // Measured Chrome cliffs:
        //   - whole-module ceiling: 1.99 MB OK, 3.27 MB FAIL -- be safe < 2 MB
        //   - reachable-closure ceiling: 0.39 MB FAIL -- the steps slice
        //     of this module should be < 400 KB
        assert!(
            bytes < 2 * 1024 * 1024,
            "exec_TopChunk0 pruned module {} bytes -- over the 2 MB whole-module cliff",
            bytes
        );
        // Verify the steps slice is also under the closure cliff. We
        // approximate by subtracting the prelude+types+layout baseline.
        let baseline = prelude.len() + types.len() + layout.len();
        let steps_bytes = bytes.saturating_sub(baseline);
        eprintln!("exec_TopChunk0 steps reachable: {} bytes", steps_bytes);
        assert!(
            steps_bytes < 400 * 1024,
            "exec_TopChunk0 reachable closure {} bytes -- over the 400 KB cliff",
            steps_bytes
        );

        // Also confirm naga can parse the emitted module -- proof that
        // chunk0-rewriting produces VALID WGSL, not just small WGSL. Skip cleanly if naga isn't on $PATH.
        let out_path = std::path::PathBuf::from("/tmp/pruned_exec_TopChunk0.wgsl");
        std::fs::write(&out_path, &module).expect("write probe module");
        let rc = std::process::Command::new("naga").arg(&out_path).output();
        match rc {
            Ok(r) if r.status.success() => {
                eprintln!("naga: validation successful for {}", out_path.display());
            }
            Ok(r) => {
                let stderr = String::from_utf8_lossy(&r.stderr);
                panic!(
                    "naga validation failed for exec_TopChunk0 pruned module:\n{}",
                    &stderr.chars().take(2000).collect::<String>()
                );
            }
            Err(e) => {
                eprintln!(
                    "skipping naga validation: {} (install naga-cli to enable)",
                    e
                );
            }
        }
    }

    #[test]
    fn top_accum_chunk0_is_documented_as_over_cliff() {
        let Some((prelude, types, layout, steps)) = try_load_generated_chunks() else {
            eprintln!("skipping: gen_zirgen output not present at /tmp/zirgen-out8");
            return;
        };
        let module = pruned_module(&prelude, &types, &layout, &steps, "exec_TopAccumChunk0")
            .expect("pruned module emits");
        let bytes = module.len();
        eprintln!("exec_TopAccumChunk0 pruned module: {} bytes", bytes);
        // TopAccum chunks measure 2.4-2.5 MB
        // module / 1.7 MB reachable -- this test pins the regression
        // direction so we notice if they ever shrink under 2 MB.
        assert!(
            bytes >= 2 * 1024 * 1024,
            "exec_TopAccumChunk0 module {} bytes -- the chunks shrank below the pinned size; \
             update test thresholds and ungate TopAccum from CPU fallback",
            bytes
        );
    }

    /// Validate the synth_arm_wrapper output
    /// concatenated with baseline + delta for MISC0 against naga. This
    /// is the offline equivalent of the Tint compile that happens in
    /// the browser; if naga accepts it, the wrapper is well-formed.
    /// (Tint may still reject for browser-specific reasons but naga
    /// catches type/lookup errors before any wasm rebuild.)
    #[test]
    fn misc0_synth_wrapper_validates_with_naga() {
        let wrapper = "@group(0) @binding(5) var<storage, read> cycle_list: array<u32>;\n\
            @group(0) @binding(6) var<storage, read> preflight_meta: array<u32>;\n\
            \n@compute @workgroup_size(64)\n\
            fn witgen_arm_misc0_chunk0_main(@builtin(global_invocation_id) gid: vec3<u32>) {\n\
              let lane = gid.x;\n\
              if (lane >= arrayLength(&cycle_list)) { return; }\n\
              cycle = cycle_list[lane];\n\
              if (cycle >= params.data_rows) { return; }\n\
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
              let _result = exec_Misc0Chunk0(x20, inst_input, lookup_TopInstResultLayout_arm0(lookup_TopLayout_instResult(bound_top)));\n\
            }\n";
        let module = assemble_arm_kernel(EXEC_MISC0_CHUNK0_DELTA_WGSL, wrapper);
        let out_path = std::path::PathBuf::from("/tmp/witgen_arm_misc0_wrapper.wgsl");
        std::fs::write(&out_path, &module).expect("write probe module");
        let rc = std::process::Command::new("naga").arg(&out_path).output();
        match rc {
            Ok(r) if r.status.success() => {
                eprintln!("naga validated the MISC0 synth wrapper");
            }
            Ok(r) => {
                let stderr = String::from_utf8_lossy(&r.stderr);
                panic!(
                    "MISC0 synth wrapper naga validation FAILED:\n{}",
                    &stderr.chars().take(4000).collect::<String>()
                );
            }
            Err(e) => {
                eprintln!("skipping naga validation: {} (install naga-cli)", e);
            }
        }
    }

    #[test]
    fn topaccum_arm_generator_reproduces_vendored_arm5() {
        // Repo-relative dev fixture shared with the browser-prove example;
        // exercised by in-repo tests only (not part of the packaged crate).
        let generated = topaccum_arm_probe_wgsl(
            include_str!(
                "../../../../../examples/browser-prove/src/witgen_wgsl/steps_step_TopAccum.pruned.wgsl"
            ),
            5,
        )
        .expect("TopAccum arm5 slice should generate");
        let expected = TOPACCUM_ARM5_WGSL
            .strip_prefix("// Generated TopAccum arm-5 slice for browser capacity checks.\n\n")
            .unwrap_or(TOPACCUM_ARM5_WGSL);
        assert_same_nonblank_wgsl(&generated, expected);
    }

    #[test]
    fn topaccum_arm_generator_profiles_all_arms() {
        let xgboost_major_cycles = [
            768_461usize,
            173_854,
            399_587,
            65_151,
            5_203,
            430_888,
            351_857,
            388_742,
            74_670,
            30_308,
            194_571,
            292,
            0,
        ];
        eprintln!(
            "arm generated_bytes nonblank_lines ext_inv_calls xgboost_cycles xgboost_inv_items"
        );
        for (arm, cycles) in xgboost_major_cycles.iter().copied().enumerate() {
            let generated = topaccum_arm_probe_wgsl(
                include_str!(
                    "../../../../../examples/browser-prove/src/witgen_wgsl/steps_step_TopAccum.pruned.wgsl"
                ),
                arm,
            )
            .unwrap_or_else(|err| panic!("TopAccum arm {arm} should generate: {err}"));
            let nonblank_lines = generated
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();
            let ext_inv_calls = generated.matches("ext_inv(").count();
            eprintln!(
                "{arm} {} {nonblank_lines} {ext_inv_calls} {cycles} {}",
                generated.len(),
                cycles.saturating_mul(ext_inv_calls),
            );
            if arm == 5 {
                assert_eq!(
                    ext_inv_calls, 26,
                    "arm5 profile must match the authoritative split-inverse path"
                );
            }
        }
    }

    fn assert_same_nonblank_wgsl(generated: &str, expected: &str) {
        let generated_lines: Vec<_> = generated
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        let expected_lines: Vec<_> = expected
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        for (idx, (generated, expected)) in generated_lines
            .iter()
            .zip(expected_lines.iter())
            .enumerate()
        {
            if generated != expected {
                let start = idx.saturating_sub(3);
                let end = (idx + 4).min(generated_lines.len().min(expected_lines.len()));
                panic!(
                    "generated TopAccum arm5 differs from vendored slice at nonblank line {}\n\
                     generated context: {:?}\nexpected context: {:?}",
                    idx + 1,
                    &generated_lines[start..end],
                    &expected_lines[start..end]
                );
            }
        }
        assert_eq!(
            generated_lines.len(),
            expected_lines.len(),
            "generated TopAccum arm5 line count differs after blank-line normalization"
        );
    }

    /// The vendored exec_TopChunk0 module with the
    /// thin `@compute` entry wrapper appended must remain naga-valid. This
    /// pins the wrapper against the names declared in the vendored WGSL --
    /// any future MuxChunk regeneration that renames `kLayout_Top`,
    /// `BoundLayout_TopLayout`, `cycle`, or `params` will fail here.
    #[test]
    fn compute_entry_concat_validates_with_naga() {
        let module = format!("{}{}", EXEC_TOP_CHUNK0_WGSL, EXEC_TOP_CHUNK0_COMPUTE_ENTRY,);
        let out_path = std::path::PathBuf::from("/tmp/exec_top_chunk0_with_entry.wgsl");
        std::fs::write(&out_path, &module).expect("write probe module");
        let rc = std::process::Command::new("naga").arg(&out_path).output();
        match rc {
            Ok(r) if r.status.success() => {
                eprintln!(
                    "naga validation successful for {} bytes module",
                    module.len()
                );
            }
            Ok(r) => {
                let stderr = String::from_utf8_lossy(&r.stderr);
                panic!(
                    "naga validation failed for compute-entry concat:\n{}",
                    &stderr.chars().take(4000).collect::<String>()
                );
            }
            Err(e) => {
                eprintln!("skipping naga validation: {} (install naga-cli)", e);
            }
        }
    }
}
