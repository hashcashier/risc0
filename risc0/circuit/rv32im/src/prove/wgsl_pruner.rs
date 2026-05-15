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

// SP7 iter 6c -- per-@compute-entry pruned WGSL module emitter.
//
// gen_zirgen (zirgen branch wgsl-gpu-backend, b956e80) emits one giant
// `steps.wgsl` containing 211 originals + 174 chunks (the MuxChunk
// pass splits every wide `zstruct.switch` into per-arm chunks). Per
// iter-5b/d, both Chrome's whole-module ceiling (~2 MB) and its
// reachable-closure ceiling (~0.4 MB) sit below this 9.16 MB blob, so
// each `@compute` entry must ship in its OWN pruned module containing
// only the call-graph closure reachable from that entry.
//
// iter-6b probe (Python `perleaf_module.py`) showed:
// - exec_TopChunk0/1: 281 KB reach / 1.03 MB module -- sub-cliff,
//   wireable now.
// - exec_TopAccum/Extract chunks: 1.7 MB reach / 2.5 MB module --
//   blocked on iter-6d (the validity/poly_ext glue isn't a wide mux,
//   so MuxChunk doesn't shrink it).
//
// This module ports the iter-6b algorithm to Rust so it can run at
// risc0 build time / call time:
//   1. Parse `fn <name>(` blocks out of `steps.wgsl` + `types.wgsl.inc`.
//   2. For each callsite to a chunked-base name, rewrite it to call a
//      specific chunk (currently chunk0 -- the SAME heuristic the iter-6b
//      probe used; correct chunk-per-arm dispatch is iter-6d work).
//   3. Walk the transitive call-graph closure from `entry`.
//   4. Emit `prelude + types + layout + (steps fns in closure)`.
//
// The result is a self-contained WGSL module that naga validates and
// (for `exec_Top` chunks at least) clears both Tint capacity ceilings.
//
// Iter 6d-a (2026-05-15): the pruned exec_TopChunk0 module is vendored at
// `risc0/circuit/rv32im/src/zirgen/exec_top_chunk0.wgsl` (~1 MB) and
// exposed via [`EXEC_TOP_CHUNK0_WGSL`]. To get a runnable compute
// pipeline, append [`EXEC_TOP_CHUNK0_COMPUTE_ENTRY`] which adds a thin
// `@compute @workgroup_size(64) fn main` that sets `cycle = gid.x` and
// calls `exec_TopChunk0(kLayout_Top-bound, 0u)`. Real dispatch wiring
// (iter 6d-b) selects the correct chunk per major opcode and feeds
// preflight data through the bound buffers.

/// Pruned `exec_TopChunk0` WGSL module (prelude + types + layout +
/// reachable-closure of `exec_TopChunk0`). ~1 MB, sub-cliff for both
/// Chrome's whole-module and reachable-closure capacity ceilings.
pub const EXEC_TOP_CHUNK0_WGSL: &str =
    include_str!("../zirgen/exec_top_chunk0.wgsl");

/// SP7 iter 6d-e (2026-05-15): pruned `exec_TopChunk1` WGSL module
/// (chunk1 of the top-level mux) -- 1.09 MB, sub-cliff. Generated via
/// `pruned_module_at_chunk(..., "exec_TopChunk1", 1)`. Together with
/// chunk0 these cover the full top-level mux; sub-chunk bases (e.g.,
/// `exec_Sha0` with only Chunk0) clamp to their max chunk index in
/// each module.
pub const EXEC_TOP_CHUNK1_WGSL: &str =
    include_str!("../zirgen/exec_top_chunk1.wgsl");

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

/// SP7 iter 6d-e: `@compute` wrapper for [`EXEC_TOP_CHUNK1_WGSL`].
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

/// SP7 iter 6d-g (2026-05-15): shared witgen baseline (prelude +
/// types.wgsl.inc + layout.wgsl.inc) used by every per-arm kernel.
/// ~791 KB. Concatenated with a per-arm delta + @compute wrapper at
/// HAL init to produce the final module passed to
/// `create_compute_kernel_async`.
pub const WITGEN_BASELINE_WGSL: &str =
    include_str!("../zirgen/witgen_baseline.wgsl");

/// SP7 iter 6d-g: per-major-arm deltas. Each contains ONLY the
/// steps fns reachable from one major opcode arm's sub-fn (Chunk0
/// variant). Total ~38 KB average × 13 arms = ~488 KB vendored.
/// All deltas paired with [`WITGEN_BASELINE_WGSL`] at runtime via
/// [`assemble_arm_kernel`].
pub const EXEC_SHA0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_sha0_chunk0_delta.wgsl");
pub const EXEC_CONTROL0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_control0_chunk0_delta.wgsl");
pub const EXEC_MEM0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_mem0_chunk0_delta.wgsl");
pub const EXEC_MEM1_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_mem1_chunk0_delta.wgsl");
pub const EXEC_MISC0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc0_chunk0_delta.wgsl");
pub const EXEC_MISC1_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc1_chunk0_delta.wgsl");
pub const EXEC_MISC2_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_misc2_chunk0_delta.wgsl");
pub const EXEC_MUL0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_mul0_chunk0_delta.wgsl");
pub const EXEC_DIV0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_div0_chunk0_delta.wgsl");
pub const EXEC_BIGINT0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_bigint0_chunk0_delta.wgsl");
pub const EXEC_ECALL0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_ecall0_chunk0_delta.wgsl");
pub const EXEC_POSEIDON0_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_poseidon0_chunk0_delta.wgsl");
pub const EXEC_POSEIDON1_CHUNK0_DELTA_WGSL: &str =
    include_str!("../zirgen/exec_poseidon1_chunk0_delta.wgsl");

/// SP7 iter 6d-g: table of (label, delta WGSL, sub-fn name) tuples
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
    ("misc0_chunk0", EXEC_MISC0_CHUNK0_DELTA_WGSL, "exec_Misc0Chunk0"),
    // 1: MISC1
    ("misc1_chunk0", EXEC_MISC1_CHUNK0_DELTA_WGSL, "exec_Misc1Chunk0"),
    // 2: MISC2
    ("misc2_chunk0", EXEC_MISC2_CHUNK0_DELTA_WGSL, "exec_Misc2Chunk0"),
    // 3: MUL0
    ("mul0_chunk0", EXEC_MUL0_CHUNK0_DELTA_WGSL, "exec_Mul0Chunk0"),
    // 4: DIV0
    ("div0_chunk0", EXEC_DIV0_CHUNK0_DELTA_WGSL, "exec_Div0Chunk0"),
    // 5: MEM0
    ("mem0_chunk0", EXEC_MEM0_CHUNK0_DELTA_WGSL, "exec_Mem0Chunk0"),
    // 6: MEM1
    ("mem1_chunk0", EXEC_MEM1_CHUNK0_DELTA_WGSL, "exec_Mem1Chunk0"),
    // 7: CONTROL0
    ("control0_chunk0", EXEC_CONTROL0_CHUNK0_DELTA_WGSL, "exec_Control0Chunk0"),
    // 8: ECALL0
    ("ecall0_chunk0", EXEC_ECALL0_CHUNK0_DELTA_WGSL, "exec_ECall0Chunk0"),
    // 9: POSEIDON0
    ("poseidon0_chunk0", EXEC_POSEIDON0_CHUNK0_DELTA_WGSL, "exec_Poseidon0Chunk0"),
    // 10: POSEIDON1
    ("poseidon1_chunk0", EXEC_POSEIDON1_CHUNK0_DELTA_WGSL, "exec_Poseidon1Chunk0"),
    // 11: SHA0
    ("sha0_chunk0", EXEC_SHA0_CHUNK0_DELTA_WGSL, "exec_Sha0Chunk0"),
    // 12: BIGINT0
    ("bigint0_chunk0", EXEC_BIGINT0_CHUNK0_DELTA_WGSL, "exec_BigInt0Chunk0"),
];

/// SP7 iter 6d-g: assemble a per-arm full kernel by concatenating
/// baseline + delta + the supplied @compute wrapper. The result is
/// passed to `WebGpuHal::create_compute_kernel_async` for Tint
/// compilation. Total module size = baseline + delta + wrapper ~=
/// 838 KB (matches the iter-6d-f-take-2 measured kernel size).
pub fn assemble_arm_kernel(delta: &str, compute_entry: &str) -> String {
    let mut out =
        String::with_capacity(WITGEN_BASELINE_WGSL.len() + delta.len() + compute_entry.len() + 2);
    out.push_str(WITGEN_BASELINE_WGSL);
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

/// SP7 iter 6d-g step 6.2.0 (2026-05-16): self-contained shadow-init
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
  // Wrap to cycle 0 for the last cycle: back_Reg(1, ...) at cycle 0
  // reads cycle (last)'s stored values via (cycle - 1 + rows) % rows.
  let next_cycle = (cycle + 1u) % params.data_rows;
  let next_base = next_cycle * 4u;
  let next_pc = preflight_meta[next_base + 0u];
  let next_state = preflight_meta[next_base + 1u];
  let next_mode = preflight_meta[next_base + 2u];
  let next_pc_low = next_pc & 0xFFFFu;
  let next_pc_high = (next_pc >> 16u) & 0xFFFFu;

  // Column-major: data_buf[col * rows + row]
  let rows = params.data_rows;
  data_buf[14u * rows + cycle] = encode(next_pc_low);     // nextPcLow
  data_buf[15u * rows + cycle] = encode(next_pc_high);    // nextPcHigh
  data_buf[16u * rows + cycle] = encode(next_state);      // nextState_0
  data_buf[17u * rows + cycle] = encode(next_mode);       // nextMachineMode
  // isFirstCycle: 1 at cycle 0, 0 elsewhere
  let is_first = select(0u, 1u, cycle == 0u);
  data_buf[18u * rows + cycle] = encode(is_first);
}
"#;

/// SP7 iter 6d-f-take-2 (2026-05-15): per-major-arm pruned module.
/// Each major opcode arm of exec_TopChunk0 gets its own kernel whose
/// closure is restricted to the sub-fn path for that arm only.
/// Module size: ~820 KB (well under Chrome's 2 MB whole-module
/// cliff). Used in dispatch-per-arm where each kernel runs over the
/// subset of cycles with that major opcode (per preflight major
/// opcode lookup -- iter-6d-g work).
pub const EXEC_SHA0_CHUNK0_ONLY_WGSL: &str =
    include_str!("../zirgen/exec_sha0_chunk0_only.wgsl");

/// SP7 iter 6d-f-take-2: `@compute` wrapper for the Sha0 per-arm
/// kernel. Calls only exec_Sha0Chunk0 -- iter-6d-g will multi-call
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

/// SP7 iter 6d-f attempt 1 (2026-05-15): "all chunks" pruned module --
/// `exec_TopChunk0_all_chunks` reachable closure with each
/// `exec_BASE(...)` callsite replaced by a synthesized
/// `exec_BASE_combined(...)` helper that calls every ChunkN
/// sequentially and OR-merges the returns (assumes Tint zero-inits
/// the unreachable-arm return vars). 2.35 MB -- on the boundary of
/// Chrome's whole-module ceiling (1.99-3.27 MB band per iter-5b).
/// Generated via `.recursive/.../sp9/gen_all_chunks.py`.
pub const EXEC_TOP_CHUNK0_ALL_WGSL: &str =
    include_str!("../zirgen/exec_top_chunk0_all.wgsl");

/// SP7 iter 6d-f `@compute` wrapper for [`EXEC_TOP_CHUNK0_ALL_WGSL`].
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
/// `chunked_bases`) with `<base>Chunk0(`.
fn rewrite_chunk0(body: &str, chunked_bases: &BTreeSet<&str>) -> String {
    rewrite_to_chunk(body, &chunked_bases.iter().map(|&b| (b, 0u32)).collect(), 0)
}

/// SP7 iter 6d-g (2026-05-15): same as [`pruned_module_at_chunk`]
/// but emits ONLY the rewritten steps fns reachable from `entry`,
/// without the prelude / types / layout prefix. The caller is
/// expected to concatenate a shared `prelude + types + layout`
/// baseline at runtime so vendoring 26+ per-arm kernels costs
/// `~baseline + N × delta_size` instead of `N × full_module_size`.
///
/// Empirically (2026-05-15): full module ~830 KB, delta ~50-100 KB.
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
        .map(|f| (f.name, rewrite_to_chunk(f.body, &chunked_max_idx, target_chunk)))
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

/// SP7 iter 6d-e (2026-05-15): generalized chunk rewrite. For each
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

/// SP7 iter 6d-e (2026-05-15): like [`pruned_module`] but rewrites
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
        .map(|f| (f.name, rewrite_to_chunk(f.body, &chunked_max_idx, target_chunk)))
        .collect();
    let rewritten_types: BTreeMap<&str, String> = types_fns
        .iter()
        .map(|f| (f.name, rewrite_to_chunk(f.body, &chunked_max_idx, target_chunk)))
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
}

impl std::fmt::Display for PrunerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EntryNotFound(name) => {
                write!(f, "entry symbol not found in steps.wgsl: {name}")
            }
        }
    }
}

impl std::error::Error for PrunerError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Path to the gen_zirgen iter-6a output -- not vendored into the
    /// crate (9 MB), but expected to be regeneratable via the gen_zirgen
    /// invocation documented in the SP7 evidence. Tests skip cleanly if
    /// the artifact is absent.
    fn try_load_iter6a() -> Option<(String, String, String, String)> {
        let dir = std::path::PathBuf::from("/tmp/zirgen-out8");
        let prelude = std::path::PathBuf::from(
            "/home/rami/repos/zirgen/zirgen/compiler/codegen/gpu/witgen_prelude.wgsl",
        );
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
    fn iter6a_exec_top_chunk0_is_under_capacity_cliffs() {
        let Some((prelude, types, layout, steps)) = try_load_iter6a() else {
            eprintln!(
                "skipping: gen_zirgen iter-6a output not present at /tmp/zirgen-out8 \
                 (regenerate via the SP7 evidence doc command)"
            );
            return;
        };
        let module = pruned_module(&prelude, &types, &layout, &steps, "exec_TopChunk0")
            .expect("pruned module emits");
        let bytes = module.len();
        eprintln!("exec_TopChunk0 pruned module: {} bytes", bytes);
        // Per iter-5b/d cliffs:
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

        // Also confirm naga can parse the emitted module -- the proof
        // that iter-6c's chunk0-rewriting produces VALID WGSL, not just
        // small WGSL. Skip cleanly if naga isn't on $PATH.
        let out_path = std::path::PathBuf::from("/tmp/iter6c_exec_TopChunk0.wgsl");
        std::fs::write(&out_path, &module).expect("write probe module");
        let rc = std::process::Command::new("naga")
            .arg(&out_path)
            .output();
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
                eprintln!("skipping naga validation: {} (install naga-cli to enable)", e);
            }
        }
    }

    #[test]
    fn iter6a_top_accum_chunk0_is_correctly_documented_as_over_cliff() {
        let Some((prelude, types, layout, steps)) = try_load_iter6a() else {
            eprintln!(
                "skipping: gen_zirgen iter-6a output not present at /tmp/zirgen-out8"
            );
            return;
        };
        let module = pruned_module(&prelude, &types, &layout, &steps, "exec_TopAccumChunk0")
            .expect("pruned module emits");
        let bytes = module.len();
        eprintln!("exec_TopAccumChunk0 pruned module: {} bytes", bytes);
        // Per iter-6b results doc: TopAccum chunks are still 2.4-2.5 MB
        // module / 1.7 MB reachable -- this test pins the regression
        // direction so we notice when iter-6d shrinks them under 2 MB.
        assert!(
            bytes >= 2 * 1024 * 1024,
            "exec_TopAccumChunk0 module {} bytes -- if this passes, iter-6d landed; \
             update test thresholds and ungate TopAccum from CPU fallback",
            bytes
        );
    }

    /// iter-6d-a (2026-05-15): the vendored exec_TopChunk0 module with the
    /// thin `@compute` entry wrapper appended must remain naga-valid. This
    /// pins the wrapper against the names declared in the vendored WGSL --
    /// any future MuxChunk regeneration that renames `kLayout_Top`,
    /// `BoundLayout_TopLayout`, `cycle`, or `params` will fail here.
    #[test]
    fn iter6d_a_compute_entry_concat_validates_with_naga() {
        let module = format!(
            "{}{}",
            EXEC_TOP_CHUNK0_WGSL, EXEC_TOP_CHUNK0_COMPUTE_ENTRY,
        );
        let out_path = std::path::PathBuf::from("/tmp/iter6d_a_exec_top_chunk0_with_entry.wgsl");
        std::fs::write(&out_path, &module).expect("write probe module");
        let rc = std::process::Command::new("naga").arg(&out_path).output();
        match rc {
            Ok(r) if r.status.success() => {
                eprintln!(
                    "iter6d-a: naga validation successful for {} bytes module",
                    module.len()
                );
            }
            Ok(r) => {
                let stderr = String::from_utf8_lossy(&r.stderr);
                panic!(
                    "iter6d-a: naga validation failed for compute-entry concat:\n{}",
                    &stderr.chars().take(4000).collect::<String>()
                );
            }
            Err(e) => {
                eprintln!("skipping naga validation: {} (install naga-cli)", e);
            }
        }
    }
}
