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

//! Witgen module assembly: preflight metadata buffers, per-arm
//! `@compute` wrapper synthesis, and the replace-mode cycle/arm
//! predicates.

use super::*;
#[allow(unused_imports)]
use super::{accum_wgsl::*, kernels::*, phases::*, session::*, traits::*};

/// Build the per-cycle preflight
/// metadata buffer consumed by `SHADOW_INIT_WGSL`. Layout: 4 u32 per
/// cycle = `[pc, state, machine_mode, packed_minor_major]`. The packed
/// field is `(major as u32) << 16 | (minor as u32)` so per-arm wrappers
/// can extract both with one buffer read.
pub(crate) fn build_preflight_meta(preflight: &PreflightTrace) -> Vec<u32> {
    let mut out = Vec::with_capacity(preflight.cycles.len() * 4);
    for cycle in &preflight.cycles {
        out.push(cycle.pc);
        out.push(cycle.state);
        out.push(cycle.machine_mode as u32);
        out.push(((cycle.major as u32) << 16) | (cycle.minor as u32));
    }
    out
}

/// Per-cycle diff counts
/// (`preflight.cycles[i].diff_count[0..1]`) packed as `[a0, a1, b0, b1, ...]`
/// for the patched `extern_getDiffCount` to index. The patched stub
/// reads `preflight_diff_count_buf[idx]` where `idx = decode(txn_cycle)
/// = cycle*2 + i`, matching the rust `get_diff_count` lookup.
pub(crate) fn build_preflight_diff_count(preflight: &PreflightTrace) -> Vec<u32> {
    let mut out = Vec::with_capacity(preflight.cycles.len() * 2);
    for cycle in &preflight.cycles {
        out.push(cycle.diff_count[0]);
        out.push(cycle.diff_count[1]);
    }
    out
}

pub(crate) fn build_preflight_txn_start(preflight: &PreflightTrace) -> Vec<u32> {
    let mut out = Vec::with_capacity(preflight.cycles.len());
    for cycle in &preflight.cycles {
        out.push(cycle.txn_idx);
    }
    out
}

pub(crate) fn build_preflight_txns(preflight: &PreflightTrace) -> Vec<u32> {
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

/// Synthesize a per-arm
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
/// Only safe for the 8 audited zero-back_Reg arms
/// (MISC0/1/2, MUL0, DIV0, MEM0/1, ECALL0). Arms
/// with internal back_Reg deps (CONTROL0, POSEIDON0/1, SHA0, BIGINT0)
/// would read uninitialized cells and produce garbage; keep them
/// no-op until a deeper materialization scheme lands.
pub(crate) fn synth_arm_wrapper(label: &str, sub_fn: &str, arm_idx: usize) -> String {
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
         fn witgen_arm_{label}_main(@builtin(global_invocation_id) gid: vec3<u32>) {{\n\
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

/// Arms with zero internal back_Reg calls (audited). Safe to
/// GPU-witgen-replace given outer shadow-init. Indexed by major opcode.
pub(crate) const ZERO_BACK_REG_ARMS: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 8];
// MISC2 and MEM0 remain diff/probe-capable, but representative e2e evidence
// showed their sparse CPU-shadow repair is currently wall-negative. Keep
// production replacement mode MISC0-only until that repair cost is removed.
pub(crate) const WITGEN_REPLACE_BASE_SUPPORTED_ARM_MASK: u16 = 1u16 << 0;

pub(crate) fn is_zero_back_reg_arm(arm_idx: usize) -> bool {
    ZERO_BACK_REG_ARMS.contains(&arm_idx)
}

pub(crate) fn witgen_mem0_candidate_enabled() -> bool {
    WITGEN_GPU_MEM0_REPLACE_CANDIDATE_ENABLED.load(Ordering::SeqCst)
        && ACCUM_GPU_MEM0_DIRECT_ENABLED.load(Ordering::SeqCst)
}

pub(crate) fn witgen_mem1_candidate_enabled() -> bool {
    WITGEN_GPU_MEM1_REPLACE_CANDIDATE_ENABLED.load(Ordering::SeqCst)
        && ACCUM_GPU_MEM1_DIRECT_ENABLED.load(Ordering::SeqCst)
}

pub(crate) fn witgen_mem0_replace_minor_enabled(minor: u8) -> bool {
    matches!(minor, 0 | 1 | 2 | 3 | 4)
        && (WITGEN_GPU_MEM0_REPLACE_MINOR_MASK.load(Ordering::SeqCst) & (1u16 << minor)) != 0
}

pub(crate) fn witgen_mem1_replace_minor_enabled(minor: u8) -> bool {
    matches!(minor, 0 | 1 | 2)
        && (WITGEN_GPU_MEM1_REPLACE_MINOR_MASK.load(Ordering::SeqCst) & (1u16 << minor)) != 0
}

pub(crate) fn witgen_replace_supported_arm_mask() -> u16 {
    let mut mask = WITGEN_REPLACE_BASE_SUPPORTED_ARM_MASK;
    if witgen_mem0_candidate_enabled() {
        mask |= 1u16 << 5;
    }
    if witgen_mem1_candidate_enabled() {
        mask |= 1u16 << 6;
    }
    mask
}

pub(crate) fn is_witgen_replace_supported_arm(arm_idx: usize) -> bool {
    arm_idx < 16 && (witgen_replace_supported_arm_mask() & (1u16 << arm_idx)) != 0
}

pub(crate) fn is_witgen_replace_cycle(major: u8, minor: u8) -> bool {
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

pub(crate) fn is_witgen_diff_cycle(major: u8, minor: u8) -> bool {
    let selected = WITGEN_GPU_DIFF_MAJOR.load(Ordering::SeqCst);
    if selected == WITGEN_GPU_DIFF_MAJOR_NONE {
        return is_witgen_replace_cycle(major, minor);
    }
    major as usize == selected
}

pub(crate) fn witgen_cycle_selected_for_mode(major: u8, minor: u8, diff_only_mode: bool) -> bool {
    if diff_only_mode {
        is_witgen_diff_cycle(major, minor)
    } else {
        is_witgen_replace_cycle(major, minor)
    }
}

pub(crate) fn needed_extra_minors(
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

/// Synthesize chunk1 wrapper. Uses
/// EXEC_TOP_CHUNK1_WGSL as the standalone module (already contains
/// exec_NondetReg, exec_NondetBitReg, exec_InstInput, exec_OneHot_13_,
/// back_Reg, back_NondetReg, lookup helpers, kLayout_Top, etc.). The
/// wrapper just appends a @compute entry that calls exec_<arm>Chunk1.
pub(crate) fn synth_arm_chunk1_wrapper(label: &str, arm_idx: usize) -> String {
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
         fn witgen_arm_{label}_c1_main(@builtin(global_invocation_id) gid: vec3<u32>) {{\n\
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
