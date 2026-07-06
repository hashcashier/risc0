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

//! WGSL builders for the direct accum kernels: per-arm accumulation
//! bodies generated from the circuit layouts, plus the TopAccum arm-5
//! probe/compare templates and ext_inv split-inverse rewriting.

use super::*;
#[allow(unused_imports)]
use super::{kernels::*, phases::*, session::*, traits::*, witgen_wgsl::*};

pub(crate) const TOPACCUM_ARM5_COMPARE_ROW_WGSL: &str = r#"
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

pub(crate) const TOPACCUM_ARM5_INV_CALLS: u32 = 26;
pub(crate) const TOPACCUM_ARM5_INV_CAPTURE_ENTRY_TEMPLATE: &str = r#"
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

pub(crate) const TOPACCUM_ARM5_INV_CONSUME_ENTRY_TEMPLATE: &str = r#"
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

pub(crate) const TOPACCUM_ARM5_INV_BUFFER_WGSL: &str = r#"
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

pub(crate) fn write_arg_u16_const(wgsl: &mut String, name: &str, arg: &'static ArgU16Layout) {
    writeln!(
        wgsl,
        "const {name}_COUNT: u32 = {}u;",
        arg.count._super.offset
    )
    .unwrap();
    writeln!(wgsl, "const {name}_VAL: u32 = {}u;", arg.val._super.offset).unwrap();
}

pub(crate) fn write_arg_u8_const(wgsl: &mut String, name: &str, arg: &'static ArgU8Layout) {
    writeln!(
        wgsl,
        "const {name}_COUNT: u32 = {}u;",
        arg.count._super.offset
    )
    .unwrap();
    writeln!(wgsl, "const {name}_VAL: u32 = {}u;", arg.val._super.offset).unwrap();
}

pub(crate) fn write_memory_arg_const(wgsl: &mut String, name: &str, arg: &'static MemoryArgLayout) {
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

pub(crate) fn write_cycle_arg_const(wgsl: &mut String, name: &str, arg: &'static CycleArgLayout) {
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

pub(crate) fn accum_misc0_direct_wgsl() -> String {
    let misc0 = LAYOUT_TOP.inst_result.arm0;
    accum_misc_direct_wgsl(
        misc0._super,
        misc0._0,
        misc0.input,
        misc0._arguments_misc0_misc_output.arg_u16,
    )
}

pub(crate) fn accum_misc1_direct_wgsl() -> String {
    let misc1 = LAYOUT_TOP.inst_result.arm1;
    accum_misc_direct_wgsl(
        misc1._super,
        misc1._0,
        misc1.input,
        misc1._arguments_misc1_misc_output.arg_u16,
    )
}

pub(crate) fn accum_misc2_direct_wgsl() -> String {
    let misc2 = LAYOUT_TOP.inst_result.arm2;
    accum_misc_direct_wgsl(
        misc2._super,
        misc2._0,
        misc2.input,
        misc2._arguments_misc2_misc_output.arg_u16,
    )
}

pub(crate) fn accum_mem0_direct_wgsl() -> String {
    let mem0 = LAYOUT_TOP.inst_result.arm5;
    accum_mem0_direct_wgsl_for_layout(mem0)
}

pub(crate) fn accum_mem1_direct_wgsl() -> String {
    let mem1 = LAYOUT_TOP.inst_result.arm6;
    accum_mem1_direct_wgsl_for_layout(mem1)
}

pub(crate) fn accum_mem0_direct_wgsl_for_layout(mem0: &'static Mem0Layout) -> String {
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

pub(crate) fn accum_mem1_direct_wgsl_for_layout(mem1: &'static Mem1Layout) -> String {
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

pub(crate) fn accum_control0_direct_wgsl() -> String {
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

/// Direct WebGPU accumulator for POSEIDON1 (major 10) rows — the
/// paged-memory hashing cycles that dominate the CPU accum stepper's
/// remaining cycle count (173 K of 416 K stepped cycles per xgboost run;
/// the four uncovered majors 3/8/9/10 each cost ~25% of the pass). A
/// POSEIDON1 row's only argument contributions are the two cycle-table
/// terms; its user-accum state is the BigInt nop constants, and the
/// cross-row recurrences stay in the terminal-prefix + machine-column-
/// carry passes, exactly like the six production direct arms. This is
/// the hand-written scalar form those arms use — NOT the
/// zirgen-generated arm10 kernel, which was rejected after its
/// generated body queued 45 s of hidden GPU work on BusyLoop.
pub(crate) fn accum_poseidon1_direct_wgsl() -> String {
    let poseidon1 = LAYOUT_TOP.inst_result.arm10;
    let user = LAYOUT_TOP_ACCUM.user._0;
    let randomness = LAYOUT_MIX.randomness;

    let mut wgsl = String::with_capacity(8_000);
    writeln!(
        wgsl,
        "const MIX_OFFSET: u32 = {}u;",
        randomness._offset.offset
    )
    .unwrap();
    writeln!(
        wgsl,
        "const MIX_CYCLE: u32 = {}u;",
        randomness.cycle_arg.cycle.offset
    )
    .unwrap();

    write_cycle_arg_const(&mut wgsl, "POSEIDON1_ARG1", poseidon1._0.arg1);
    write_cycle_arg_const(&mut wgsl, "POSEIDON1_ARG2", poseidon1._0.arg2);

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
    writeln!(
        wgsl,
        "const ACC_COL0: u32 = {}u;",
        LAYOUT_TOP_ACCUM.columns[0].offset
    )
    .unwrap();
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

  cur = ext_add(cur, cycle_term(row, POSEIDON1_ARG1_COUNT, POSEIDON1_ARG1_CYCLE));
  cur = ext_add(cur, cycle_term(row, POSEIDON1_ARG2_COUNT, POSEIDON1_ARG2_CYCLE));
  store_ext(row, ACC_COL0, cur);
  store_ext(row, ACC_COL19, cur);
}
"#,
    );
    wgsl
}

pub(crate) fn accum_misc_direct_wgsl(
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
pub(crate) enum AccumMiscDirectKind {
    Misc0,
    Misc1,
    Misc2,
    Mem0,
    Mem1,
    Control0,
    Poseidon1,
}

impl AccumMiscDirectKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Misc0 => "MISC0",
            Self::Misc1 => "MISC1",
            Self::Misc2 => "MISC2",
            Self::Mem0 => "MEM0",
            Self::Mem1 => "MEM1",
            Self::Control0 => "CONTROL0",
            Self::Poseidon1 => "POSEIDON1",
        }
    }

    pub(crate) fn row_source(self) -> &'static str {
        match self {
            Self::Misc0 => "rv32im_accum_misc0_direct_rows",
            Self::Misc1 => "rv32im_accum_misc1_direct_rows",
            Self::Misc2 => "rv32im_accum_misc2_direct_rows",
            Self::Mem0 => "rv32im_accum_mem0_direct_rows",
            Self::Mem1 => "rv32im_accum_mem1_direct_rows",
            Self::Control0 => "rv32im_accum_control0_direct_rows",
            Self::Poseidon1 => "rv32im_accum_poseidon1_direct_rows",
        }
    }

    pub(crate) fn params_source(self) -> &'static str {
        match self {
            Self::Misc0 => "rv32im_accum_misc0_direct_params",
            Self::Misc1 => "rv32im_accum_misc1_direct_params",
            Self::Misc2 => "rv32im_accum_misc2_direct_params",
            Self::Mem0 => "rv32im_accum_mem0_direct_params",
            Self::Mem1 => "rv32im_accum_mem1_direct_params",
            Self::Control0 => "rv32im_accum_control0_direct_params",
            Self::Poseidon1 => "rv32im_accum_poseidon1_direct_params",
        }
    }

    pub(crate) fn layout_label(self) -> &'static str {
        match self {
            Self::Misc0 => "rv32im_accum_misc0_direct_layout",
            Self::Misc1 => "rv32im_accum_misc1_direct_layout",
            Self::Misc2 => "rv32im_accum_misc2_direct_layout",
            Self::Mem0 => "rv32im_accum_mem0_direct_layout",
            Self::Mem1 => "rv32im_accum_mem1_direct_layout",
            Self::Control0 => "rv32im_accum_control0_direct_layout",
            Self::Poseidon1 => "rv32im_accum_poseidon1_direct_layout",
        }
    }

    pub(crate) fn bind_group_label(self) -> &'static str {
        match self {
            Self::Misc0 => "rv32im_accum_misc0_direct_bind_group",
            Self::Misc1 => "rv32im_accum_misc1_direct_bind_group",
            Self::Misc2 => "rv32im_accum_misc2_direct_bind_group",
            Self::Mem0 => "rv32im_accum_mem0_direct_bind_group",
            Self::Mem1 => "rv32im_accum_mem1_direct_bind_group",
            Self::Control0 => "rv32im_accum_control0_direct_bind_group",
            Self::Poseidon1 => "rv32im_accum_poseidon1_direct_bind_group",
        }
    }

    pub(crate) fn record_rows(self, rows: usize) {
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
            Self::Poseidon1 => {
                ACCUM_GPU_POSEIDON1_DIRECT_ROWS.fetch_add(rows, Ordering::SeqCst);
            }
        }
    }
}

pub(crate) fn topaccum_replace_ext_inv_calls(src: &str, adapter: &str) -> Result<(String, u32)> {
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

pub(crate) fn topaccum_arm5_replace_ext_inv_calls(
    src: &str,
    adapter: &str,
) -> Result<(String, u32)> {
    let (out, slot) = topaccum_replace_ext_inv_calls(src, adapter)?;
    anyhow::ensure!(
        slot == TOPACCUM_ARM5_INV_CALLS,
        "TopAccum ext_inv call count changed: expected {}, found {}",
        TOPACCUM_ARM5_INV_CALLS,
        slot
    );
    Ok((out, slot))
}

pub(crate) fn topaccum_arm5_inv_entry(template: &str, inv_count: u32) -> String {
    template.replace("__INV_COUNT__", &format!("{inv_count}u"))
}
