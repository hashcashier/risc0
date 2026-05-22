// Copyright 2025 RISC Zero, Inc.
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

// ============================================================================
// WGSL witgen prelude for the zirgen WGSL backend.
//
// The zirgen WGSL backend emits `steps.wgsl` (the witness-generation step
// functions) referencing the field types, field arithmetic, and witness-buffer
// access helpers defined here. WGSL has no #include, so the consumed shader
// module is the concatenation:
//
//     witgen_prelude.wgsl  +  types.wgsl.inc  +  layout.wgsl.inc  +  steps.wgsl
//
// plus a circuit-specific `@compute` entry point (risc0-side) that sets `cycle`
// from the dispatch id and calls `step_Top` / `step_TopAccum`.
//
// This prelude mirrors `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs` (the
// CPU witgen reference) and `risc0_core/src/field/baby_bear.rs` (the field).
// The field-arithmetic helpers (add/sub/mul/mul_wide/ext_*) are the verified
// implementations from `risc0/zkp/src/hal/webgpu_codegen/prelude.wgsl`.
// ============================================================================

// ----- BabyBear field constants ---------------------------------------------
const P: u32 = 2013265921u;       // 15 * 2^27 + 1
const M: u32 = 2281701377u;       // -P^-1 mod 2^32  (0x88000001)
const R2: u32 = 1172168163u;      // R^2 mod P,  R = 2^32
const NBETA: u32 = 1073741848u;   // extension-field BETA constant
const MONT_ONE: u32 = 268435454u; // encode(1) = R mod P
const INVALID: u32 = 0xffffffffu; // unset-cell sentinel

// ----- Field types ----------------------------------------------------------
// WGSL has no generics; the zirgen WGSL backend lowers field elements to plain
// u32 (Montgomery form) and the degree-4 extension to vec4<u32>.
alias Val = u32;
alias ExtVal = vec4<u32>;
alias Index = u32;

// A column reference. The backend lowers `RefAttr` to `Reg(Nu)`.
struct Reg {
  col: u32,
}
// The leaf monomorphized BoundLayout wrapper. The per-circuit composite
// BoundLayout_<T> structs are emitted into types.wgsl.inc by emitLayoutDef;
// BoundLayout_Reg is the universal leaf and lives here in the prelude.
// The layout field is named `lyt` because `layout` is a WGSL reserved keyword.
struct BoundLayout_Reg {
  lyt: Reg,
  buf: u32,
}

// ----- Witness buffers ------------------------------------------------------
// Buffer ids (the backend lowers `get_buffer(name)` to `buf_<name>`, and the
// @compute entry passes these into step_Top / step_TopAccum).
const buf_data: u32 = 0u;
const buf_global: u32 = 1u;
const buf_accum: u32 = 2u;
const buf_mix: u32 = 3u;

// `cycle` is the witness row currently being generated. The @compute entry
// point sets it from @builtin(global_invocation_id) before calling a step fn.
var<private> cycle: u32;

struct WitgenParams {
  data_rows: u32,
  global_rows: u32,
  accum_rows: u32,
  mix_rows: u32,
  // accum's BufferRow::with_zero_back_after column (0 = no zero-back rule).
  accum_zero_back: u32,
}

@group(0) @binding(0) var<storage, read_write> data_buf: array<u32>;
@group(0) @binding(1) var<storage, read_write> global_buf: array<u32>;
@group(0) @binding(2) var<storage, read_write> accum_buf: array<u32>;
@group(0) @binding(3) var<storage, read_write> mix_buf: array<u32>;
@group(0) @binding(4) var<uniform> params: WitgenParams;

// ----- BabyBear scalar arithmetic (Montgomery form) -------------------------
// Verbatim from risc0/zkp/src/hal/webgpu_codegen/prelude.wgsl, which is
// verified to match baby_bear.rs.

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

// Montgomery encode (direct -> Montgomery) and decode (Montgomery -> direct).
fn encode(a: u32) -> Val {
  return mul(R2, a);
}
fn decode(a: Val) -> u32 {
  return mul(1u, a);
}

// Square-and-multiply exponentiation in the Montgomery domain.
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

// Multiplicative inverse via Fermat's little theorem: x^(P-2). inv(0) = 0,
// matching baby_bear.rs.
fn inv(x: Val) -> Val {
  return pow(x, P - 2u);
}

// ----- BabyBearExt arithmetic (vec4<u32>, Montgomery components) -------------

fn ext_add(lhs: ExtVal, rhs: ExtVal) -> ExtVal {
  return ExtVal(add(lhs.x, rhs.x), add(lhs.y, rhs.y), add(lhs.z, rhs.z), add(lhs.w, rhs.w));
}

fn ext_sub(lhs: ExtVal, rhs: ExtVal) -> ExtVal {
  return ExtVal(sub(lhs.x, rhs.x), sub(lhs.y, rhs.y), sub(lhs.z, rhs.z), sub(lhs.w, rhs.w));
}

fn ext_mul(lhs: ExtVal, rhs: ExtVal) -> ExtVal {
  return ExtVal(
    add(mul(lhs.x, rhs.x),
        mul(NBETA, add(add(mul(lhs.y, rhs.w), mul(lhs.z, rhs.z)), mul(lhs.w, rhs.y)))),
    add(add(mul(lhs.x, rhs.y), mul(lhs.y, rhs.x)),
        mul(NBETA, add(mul(lhs.z, rhs.w), mul(lhs.w, rhs.z)))),
    add(add(add(mul(lhs.x, rhs.z), mul(lhs.y, rhs.y)), mul(lhs.z, rhs.x)),
        mul(NBETA, mul(lhs.w, rhs.w))),
    add(add(add(mul(lhs.x, rhs.w), mul(lhs.y, rhs.z)), mul(lhs.z, rhs.y)), mul(lhs.w, rhs.x)),
  );
}

fn ext_scale(lhs: ExtVal, rhs: Val) -> ExtVal {
  return ExtVal(mul(lhs.x, rhs), mul(lhs.y, rhs), mul(lhs.z, rhs), mul(lhs.w, rhs));
}

// TODO(wgsl): the extension-field inverse. The base-field path (inv/inv_0) is
// exact; ext_inv is a placeholder until verified against ExtElem::inv. iter 6's
// byte-identical SP-CR check will flag any rv32im path that actually needs it.
fn ext_inv(x: ExtVal) -> ExtVal {
  return x;
}

// ----- DSL builtin field helpers --------------------------------------------
// These names are emitted verbatim by the witgen step functions; they mirror
// the helpers in risc0's CUDA witgen.h / rust_steps.rs.

fn isz(x: Val) -> Val {
  if (x == 0u) {
    return MONT_ONE;
  }
  return 0u;
}

fn neg_0(x: Val) -> Val {
  return sub(0u, x);
}

fn inv_0(x: Val) -> Val {
  return inv(x);
}

// `mod` is a WGSL reserved keyword, so canonIdent escapes the DSL builtin to
// `mod_`; the prelude definition matches.
fn mod_(lhs: Val, rhs: Val) -> Val {
  return encode(decode(lhs) % decode(rhs));
}

// bitAnd / inRange keep the camelCase DSL builtin names (canonIdent leaves
// already-camelCase identifiers unchanged).
fn bitAnd(lhs: Val, rhs: Val) -> Val {
  return encode(decode(lhs) & decode(rhs));
}

fn inRange(low: Val, mid: Val, high: Val) -> Val {
  let l = decode(low);
  let m = decode(mid);
  let h = decode(high);
  if (l <= m && m < h) {
    return MONT_ONE;
  }
  return 0u;
}

// Montgomery Val -> plain u32, used for array/index conversions.
fn to_size_t(v: Val) -> u32 {
  return decode(v);
}

// eqz is a witness consistency assertion (it never writes witness state). For
// now it is a no-op so the witness is still produced.
// TODO(wgsl): route a failure to a debug error-flag buffer instead of dropping.
fn eqz(v: Val) {
}
fn eqz_ext(v: ExtVal) {
}

// ----- Witness buffer access (column-major; mirrors rust_steps.rs BufferRow) -
// A buffer cell (row, col) lives at `col * rows + row`. `data` and `accum` are
// mutable per-cycle buffers; `global` and `mix` are global (single-row).

fn buf_rows(buf_id: u32) -> u32 {
  switch buf_id {
    case 0u: { return params.data_rows; }
    case 1u: { return params.global_rows; }
    case 2u: { return params.accum_rows; }
    default: { return params.mix_rows; }
  }
}

fn buf_is_global(buf_id: u32) -> bool {
  return buf_id == buf_global || buf_id == buf_mix;
}

fn buf_get(buf_id: u32, idx: u32) -> u32 {
  switch buf_id {
    case 0u: { return data_buf[idx]; }
    case 1u: { return global_buf[idx]; }
    case 2u: { return accum_buf[idx]; }
    default: { return mix_buf[idx]; }
  }
}

fn buf_set(buf_id: u32, idx: u32, v: u32) {
  switch buf_id {
    case 0u: { data_buf[idx] = v; }
    case 1u: { global_buf[idx] = v; }
    case 2u: { accum_buf[idx] = v; }
    default: { mix_buf[idx] = v; }
  }
}

// load_at / store_at take a flat (col, buf) pair. The public load / store /
// load_ext / ... take a BoundLayout_Reg by value: the LoadOp/StoreOp op
// handlers then emit the ref expression once (`load(reg, back)`) instead of
// twice (`load(reg.lyt.col, reg.buf, back)`), which roughly halves the size of
// the generated step functions and keeps the WGSL validator/compiler tractable.

fn load_at(col: u32, buf_id: u32, back: u32) -> Val {
  // accum's zero-back rule (BufferRow::with_zero_back_after).
  if (buf_id == buf_accum && params.accum_zero_back != 0u
      && col > params.accum_zero_back && back > 0u) {
    return 0u;
  }
  let rows = buf_rows(buf_id);
  var row: u32;
  if (buf_is_global(buf_id)) {
    row = 0u;
  } else {
    row = (rows + cycle - back) % rows;
  }
  // TODO(wgsl): mirror BufferRow's is_valid()/valid_or_zero() checked-read
  // behavior; for now an unset (INVALID) cell is returned as-is.
  return buf_get(buf_id, col * rows + row);
}

fn store_at(col: u32, buf_id: u32, v: Val) {
  let rows = buf_rows(buf_id);
  var row: u32;
  if (buf_is_global(buf_id)) {
    row = 0u;
  } else {
    row = cycle;
  }
  buf_set(buf_id, col * rows + row, v);
}

fn load(reg: BoundLayout_Reg, back: u32) -> Val {
  return load_at(reg.lyt.col, reg.buf, back);
}

fn load_ext(reg: BoundLayout_Reg, back: u32) -> ExtVal {
  return ExtVal(load_at(reg.lyt.col, reg.buf, back),
                load_at(reg.lyt.col + 1u, reg.buf, back),
                load_at(reg.lyt.col + 2u, reg.buf, back),
                load_at(reg.lyt.col + 3u, reg.buf, back));
}

// Base-field value promoted to the canonical (x, 0, 0, 0) ExtVal embedding,
// matching F::ExtElem::from_subfield(F::Elem).
fn load_as_ext(reg: BoundLayout_Reg, back: u32) -> ExtVal {
  return ExtVal(load_at(reg.lyt.col, reg.buf, back), 0u, 0u, 0u);
}

fn store(reg: BoundLayout_Reg, v: Val) {
  store_at(reg.lyt.col, reg.buf, v);
}

fn store_ext(reg: BoundLayout_Reg, v: ExtVal) {
  store_at(reg.lyt.col, reg.buf, v.x);
  store_at(reg.lyt.col + 1u, reg.buf, v.y);
  store_at(reg.lyt.col + 2u, reg.buf, v.z);
  store_at(reg.lyt.col + 3u, reg.buf, v.w);
}

// ----- Externs --------------------------------------------------------------
// `invoke_extern` lowers here. assert/log/print are no-ops in witgen (matching
// CUDA witgen.h) and collapse to extern_noop(). The remaining externs read from
// the preflight trace -- in CUDA, ExecContext::preflight. The bodies below are
// TODO(wgsl) stubs with the correct signatures and zeroed results; uploading
// the PreflightTrace as GPU buffers and implementing the reads is a later
// iteration (the dispatch/buffer wiring is risc0-side). Array-returning
// externs return a WGSL array<Val,N>, which emitSaveResults projects per result.

fn extern_noop() {
}

fn extern_lookupDelta(table: Val, index: Val, count: Val) {
}

fn extern_lookupCurrent(table: Val, index: Val) -> Val {
  return 0u;
}

fn extern_memoryDelta(addr: Val, txn_cycle: Val, data_low: Val, data_high: Val, count: Val) {
}

fn extern_getDiffCount(txn_cycle: Val) -> Val {
  return 0u;
}

fn extern_isFirstCycle_0() -> Val {
  return 0u;
}

fn extern_hostReadPrepare(fp: Val, len: Val) -> Val {
  return 0u;
}

fn extern_hostWrite(fd: Val, addr_low: Val, addr_high: Val, len: Val) -> Val {
  return 0u;
}

fn extern_getMemoryTxn(addr: Val) -> array<Val, 5> {
  return array<Val, 5>(0u, 0u, 0u, 0u, 0u);
}

fn extern_divide(numer_low: Val, numer_high: Val, denom_low: Val, denom_high: Val,
                 sign_type: Val) -> array<Val, 4> {
  return array<Val, 4>(0u, 0u, 0u, 0u);
}

fn extern_getMajorMinor() -> array<Val, 2> {
  return array<Val, 2>(0u, 0u);
}

fn extern_nextPagingIdx() -> array<Val, 2> {
  return array<Val, 2>(0u, 0u);
}

fn extern_bigIntExtern() -> array<Val, 16> {
  return array<Val, 16>(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u);
}
struct NondetRegLayout {
  _super: Reg,
}
struct BoundLayout_NondetRegLayout {
  lyt: NondetRegLayout,
  buf: u32,
}
fn lookup_NondetRegLayout__super(b: BoundLayout_NondetRegLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt._super, b.buf);
}
alias NondetRegLayout7LayoutArray = array<NondetRegLayout, 7>;
struct BoundLayout_NondetRegLayout7LayoutArray {
  lyt: NondetRegLayout7LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout7LayoutArray(b: BoundLayout_NondetRegLayout7LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct OneHot_7_Layout {
  _super: NondetRegLayout7LayoutArray,
}
struct BoundLayout_OneHot_7_Layout {
  lyt: OneHot_7_Layout,
  buf: u32,
}
fn lookup_OneHot_7_Layout__super(b: BoundLayout_OneHot_7_Layout) -> BoundLayout_NondetRegLayout7LayoutArray {
  return BoundLayout_NondetRegLayout7LayoutArray(b.lyt._super, b.buf);
}
struct NondetExtRegLayout {
  _super: Reg,
}
struct BoundLayout_NondetExtRegLayout {
  lyt: NondetExtRegLayout,
  buf: u32,
}
fn lookup_NondetExtRegLayout__super(b: BoundLayout_NondetExtRegLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt._super, b.buf);
}
struct BigIntAccumStateLayout {
  poly: NondetExtRegLayout,
  term: NondetExtRegLayout,
  total: NondetExtRegLayout,
}
struct BoundLayout_BigIntAccumStateLayout {
  lyt: BigIntAccumStateLayout,
  buf: u32,
}
fn lookup_BigIntAccumStateLayout_poly(b: BoundLayout_BigIntAccumStateLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.poly, b.buf);
}
fn lookup_BigIntAccumStateLayout_term(b: BoundLayout_BigIntAccumStateLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.term, b.buf);
}
fn lookup_BigIntAccumStateLayout_total(b: BoundLayout_BigIntAccumStateLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.total, b.buf);
}
struct BigIntPolyOpAddTotalLayout {
  _super: BigIntAccumStateLayout,
  tmp: NondetExtRegLayout,
}
struct BoundLayout_BigIntPolyOpAddTotalLayout {
  lyt: BigIntPolyOpAddTotalLayout,
  buf: u32,
}
fn lookup_BigIntPolyOpAddTotalLayout__super(b: BoundLayout_BigIntPolyOpAddTotalLayout) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntPolyOpAddTotalLayout_tmp(b: BoundLayout_BigIntPolyOpAddTotalLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.tmp, b.buf);
}
struct BigIntAccumStateLayout_0 {
  _super: BigIntAccumStateLayout,
  arm0: BigIntAccumStateLayout,
  arm1: BigIntAccumStateLayout,
  arm2: BigIntAccumStateLayout,
  arm3: BigIntPolyOpAddTotalLayout,
  arm4: BigIntAccumStateLayout,
  arm5: BigIntAccumStateLayout,
  arm6: BigIntAccumStateLayout,
}
struct BoundLayout_BigIntAccumStateLayout_0 {
  lyt: BigIntAccumStateLayout_0,
  buf: u32,
}
fn lookup_BigIntAccumStateLayout_0__super(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm0(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.arm0, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm1(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.arm1, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm2(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.arm2, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm3(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntPolyOpAddTotalLayout {
  return BoundLayout_BigIntPolyOpAddTotalLayout(b.lyt.arm3, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm4(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.arm4, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm5(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.arm5, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm6(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.arm6, b.buf);
}
struct BigIntAccumLayout {
  polyOp: OneHot_7_Layout,
  state: BigIntAccumStateLayout,
  stateRedef: BigIntAccumStateLayout_0,
}
struct BoundLayout_BigIntAccumLayout {
  lyt: BigIntAccumLayout,
  buf: u32,
}
fn lookup_BigIntAccumLayout_polyOp(b: BoundLayout_BigIntAccumLayout) -> BoundLayout_OneHot_7_Layout {
  return BoundLayout_OneHot_7_Layout(b.lyt.polyOp, b.buf);
}
fn lookup_BigIntAccumLayout_state(b: BoundLayout_BigIntAccumLayout) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.state, b.buf);
}
fn lookup_BigIntAccumLayout_stateRedef(b: BoundLayout_BigIntAccumLayout) -> BoundLayout_BigIntAccumStateLayout_0 {
  return BoundLayout_BigIntAccumStateLayout_0(b.lyt.stateRedef, b.buf);
}
struct AccumLayout {
  _0: BigIntAccumLayout,
}
struct BoundLayout_AccumLayout {
  lyt: AccumLayout,
  buf: u32,
}
fn lookup_AccumLayout__0(b: BoundLayout_AccumLayout) -> BoundLayout_BigIntAccumLayout {
  return BoundLayout_BigIntAccumLayout(b.lyt._0, b.buf);
}
alias NondetRegLayout8LayoutArray = array<NondetRegLayout, 8>;
struct BoundLayout_NondetRegLayout8LayoutArray {
  lyt: NondetRegLayout8LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout8LayoutArray(b: BoundLayout_NondetRegLayout8LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct OneHot_8_Layout {
  _super: NondetRegLayout8LayoutArray,
}
struct BoundLayout_OneHot_8_Layout {
  lyt: OneHot_8_Layout,
  buf: u32,
}
fn lookup_OneHot_8_Layout__super(b: BoundLayout_OneHot_8_Layout) -> BoundLayout_NondetRegLayout8LayoutArray {
  return BoundLayout_NondetRegLayout8LayoutArray(b.lyt._super, b.buf);
}
struct InstInputLayout {
  minorOnehot: OneHot_8_Layout,
}
struct BoundLayout_InstInputLayout {
  lyt: InstInputLayout,
  buf: u32,
}
fn lookup_InstInputLayout_minorOnehot(b: BoundLayout_InstInputLayout) -> BoundLayout_OneHot_8_Layout {
  return BoundLayout_OneHot_8_Layout(b.lyt.minorOnehot, b.buf);
}
alias NondetRegLayout13LayoutArray = array<NondetRegLayout, 13>;
struct BoundLayout_NondetRegLayout13LayoutArray {
  lyt: NondetRegLayout13LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout13LayoutArray(b: BoundLayout_NondetRegLayout13LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct OneHot_13_Layout {
  _super: NondetRegLayout13LayoutArray,
}
struct BoundLayout_OneHot_13_Layout {
  lyt: OneHot_13_Layout,
  buf: u32,
}
fn lookup_OneHot_13_Layout__super(b: BoundLayout_OneHot_13_Layout) -> BoundLayout_NondetRegLayout13LayoutArray {
  return BoundLayout_NondetRegLayout13LayoutArray(b.lyt._super, b.buf);
}
struct ArgU16Layout {
  count: NondetRegLayout,
  val: NondetRegLayout,
}
struct BoundLayout_ArgU16Layout {
  lyt: ArgU16Layout,
  buf: u32,
}
fn lookup_ArgU16Layout_count(b: BoundLayout_ArgU16Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.count, b.buf);
}
fn lookup_ArgU16Layout_val(b: BoundLayout_ArgU16Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.val, b.buf);
}
struct NondetU16RegLayout {
  arg: ArgU16Layout,
}
struct BoundLayout_NondetU16RegLayout {
  lyt: NondetU16RegLayout,
  buf: u32,
}
fn lookup_NondetU16RegLayout_arg(b: BoundLayout_NondetU16RegLayout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt.arg, b.buf);
}
struct NormalizeU32Layout {
  low16: NondetU16RegLayout,
  lowCarry: NondetRegLayout,
  high16: NondetU16RegLayout,
  highCarry: NondetRegLayout,
}
struct BoundLayout_NormalizeU32Layout {
  lyt: NormalizeU32Layout,
  buf: u32,
}
fn lookup_NormalizeU32Layout_low16(b: BoundLayout_NormalizeU32Layout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.low16, b.buf);
}
fn lookup_NormalizeU32Layout_lowCarry(b: BoundLayout_NormalizeU32Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.lowCarry, b.buf);
}
fn lookup_NormalizeU32Layout_high16(b: BoundLayout_NormalizeU32Layout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.high16, b.buf);
}
fn lookup_NormalizeU32Layout_highCarry(b: BoundLayout_NormalizeU32Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.highCarry, b.buf);
}
struct MemoryArgLayout {
  count: NondetRegLayout,
  addr: NondetRegLayout,
  cycle: NondetRegLayout,
  dataLow: NondetRegLayout,
  dataHigh: NondetRegLayout,
}
struct BoundLayout_MemoryArgLayout {
  lyt: MemoryArgLayout,
  buf: u32,
}
fn lookup_MemoryArgLayout_count(b: BoundLayout_MemoryArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.count, b.buf);
}
fn lookup_MemoryArgLayout_addr(b: BoundLayout_MemoryArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.addr, b.buf);
}
fn lookup_MemoryArgLayout_cycle(b: BoundLayout_MemoryArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.cycle, b.buf);
}
fn lookup_MemoryArgLayout_dataLow(b: BoundLayout_MemoryArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.dataLow, b.buf);
}
fn lookup_MemoryArgLayout_dataHigh(b: BoundLayout_MemoryArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.dataHigh, b.buf);
}
struct MemoryIOLayout {
  oldTxn: MemoryArgLayout,
  newTxn: MemoryArgLayout,
}
struct BoundLayout_MemoryIOLayout {
  lyt: MemoryIOLayout,
  buf: u32,
}
fn lookup_MemoryIOLayout_oldTxn(b: BoundLayout_MemoryIOLayout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt.oldTxn, b.buf);
}
fn lookup_MemoryIOLayout_newTxn(b: BoundLayout_MemoryIOLayout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt.newTxn, b.buf);
}
struct CycleArgLayout {
  count: NondetRegLayout,
  cycle: NondetRegLayout,
}
struct BoundLayout_CycleArgLayout {
  lyt: CycleArgLayout,
  buf: u32,
}
fn lookup_CycleArgLayout_count(b: BoundLayout_CycleArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.count, b.buf);
}
fn lookup_CycleArgLayout_cycle(b: BoundLayout_CycleArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.cycle, b.buf);
}
struct IsCycleLayout {
  arg: CycleArgLayout,
}
struct BoundLayout_IsCycleLayout {
  lyt: IsCycleLayout,
  buf: u32,
}
fn lookup_IsCycleLayout_arg(b: BoundLayout_IsCycleLayout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt.arg, b.buf);
}
struct IsForwardLayout {
  _0: IsCycleLayout,
}
struct BoundLayout_IsForwardLayout {
  lyt: IsForwardLayout,
  buf: u32,
}
fn lookup_IsForwardLayout__0(b: BoundLayout_IsForwardLayout) -> BoundLayout_IsCycleLayout {
  return BoundLayout_IsCycleLayout(b.lyt._0, b.buf);
}
struct MemoryWriteLayout {
  io: MemoryIOLayout,
  _0: IsForwardLayout,
}
struct BoundLayout_MemoryWriteLayout {
  lyt: MemoryWriteLayout,
  buf: u32,
}
fn lookup_MemoryWriteLayout_io(b: BoundLayout_MemoryWriteLayout) -> BoundLayout_MemoryIOLayout {
  return BoundLayout_MemoryIOLayout(b.lyt.io, b.buf);
}
fn lookup_MemoryWriteLayout__0(b: BoundLayout_MemoryWriteLayout) -> BoundLayout_IsForwardLayout {
  return BoundLayout_IsForwardLayout(b.lyt._0, b.buf);
}
struct IsZeroLayout {
  _super: NondetRegLayout,
  inv: NondetRegLayout,
}
struct BoundLayout_IsZeroLayout {
  lyt: IsZeroLayout,
  buf: u32,
}
fn lookup_IsZeroLayout__super(b: BoundLayout_IsZeroLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._super, b.buf);
}
fn lookup_IsZeroLayout_inv(b: BoundLayout_IsZeroLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.inv, b.buf);
}
struct WriteRdLayout {
  isRd0: IsZeroLayout,
  writeAddr: NondetRegLayout,
  _0: MemoryWriteLayout,
}
struct BoundLayout_WriteRdLayout {
  lyt: WriteRdLayout,
  buf: u32,
}
fn lookup_WriteRdLayout_isRd0(b: BoundLayout_WriteRdLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isRd0, b.buf);
}
fn lookup_WriteRdLayout_writeAddr(b: BoundLayout_WriteRdLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.writeAddr, b.buf);
}
fn lookup_WriteRdLayout__0(b: BoundLayout_WriteRdLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
struct FinalizeMiscLayout {
  writeData: NormalizeU32Layout,
  pcNorm: NormalizeU32Layout,
  _0: WriteRdLayout,
}
struct BoundLayout_FinalizeMiscLayout {
  lyt: FinalizeMiscLayout,
  buf: u32,
}
fn lookup_FinalizeMiscLayout_writeData(b: BoundLayout_FinalizeMiscLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.writeData, b.buf);
}
fn lookup_FinalizeMiscLayout_pcNorm(b: BoundLayout_FinalizeMiscLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.pcNorm, b.buf);
}
fn lookup_FinalizeMiscLayout__0(b: BoundLayout_FinalizeMiscLayout) -> BoundLayout_WriteRdLayout {
  return BoundLayout_WriteRdLayout(b.lyt._0, b.buf);
}
struct DoCycleTableLayout {
  arg1: CycleArgLayout,
  arg2: CycleArgLayout,
}
struct BoundLayout_DoCycleTableLayout {
  lyt: DoCycleTableLayout,
  buf: u32,
}
fn lookup_DoCycleTableLayout_arg1(b: BoundLayout_DoCycleTableLayout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt.arg1, b.buf);
}
fn lookup_DoCycleTableLayout_arg2(b: BoundLayout_DoCycleTableLayout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt.arg2, b.buf);
}
struct DecoderLayout {
  _f7_6: NondetRegLayout,
  _f7_45: NondetRegLayout,
  _f7_23: NondetRegLayout,
  _f7_01: NondetRegLayout,
  _rs2_34: NondetRegLayout,
  _rs2_12: NondetRegLayout,
  _rs2_0: NondetRegLayout,
  _rs1_34: NondetRegLayout,
  _rs1_12: NondetRegLayout,
  _rs1_0: NondetRegLayout,
  _f3_2: NondetRegLayout,
  _f3_01: NondetRegLayout,
  _rd_34: NondetRegLayout,
  _rd_12: NondetRegLayout,
  _rd_0: NondetRegLayout,
  opcode: NondetRegLayout,
}
struct BoundLayout_DecoderLayout {
  lyt: DecoderLayout,
  buf: u32,
}
fn lookup_DecoderLayout__f7_6(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._f7_6, b.buf);
}
fn lookup_DecoderLayout__f7_45(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._f7_45, b.buf);
}
fn lookup_DecoderLayout__f7_23(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._f7_23, b.buf);
}
fn lookup_DecoderLayout__f7_01(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._f7_01, b.buf);
}
fn lookup_DecoderLayout__rs2_34(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rs2_34, b.buf);
}
fn lookup_DecoderLayout__rs2_12(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rs2_12, b.buf);
}
fn lookup_DecoderLayout__rs2_0(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rs2_0, b.buf);
}
fn lookup_DecoderLayout__rs1_34(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rs1_34, b.buf);
}
fn lookup_DecoderLayout__rs1_12(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rs1_12, b.buf);
}
fn lookup_DecoderLayout__rs1_0(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rs1_0, b.buf);
}
fn lookup_DecoderLayout__f3_2(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._f3_2, b.buf);
}
fn lookup_DecoderLayout__f3_01(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._f3_01, b.buf);
}
fn lookup_DecoderLayout__rd_34(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rd_34, b.buf);
}
fn lookup_DecoderLayout__rd_12(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rd_12, b.buf);
}
fn lookup_DecoderLayout__rd_0(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rd_0, b.buf);
}
fn lookup_DecoderLayout_opcode(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.opcode, b.buf);
}
struct AddrDecomposeLayout {
  low2: NondetRegLayout,
  upperDiff: NondetU16RegLayout,
  _0: IsZeroLayout,
  med14: NondetU16RegLayout,
}
struct BoundLayout_AddrDecomposeLayout {
  lyt: AddrDecomposeLayout,
  buf: u32,
}
fn lookup_AddrDecomposeLayout_low2(b: BoundLayout_AddrDecomposeLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low2, b.buf);
}
fn lookup_AddrDecomposeLayout_upperDiff(b: BoundLayout_AddrDecomposeLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.upperDiff, b.buf);
}
fn lookup_AddrDecomposeLayout__0(b: BoundLayout_AddrDecomposeLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt._0, b.buf);
}
fn lookup_AddrDecomposeLayout_med14(b: BoundLayout_AddrDecomposeLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.med14, b.buf);
}
struct MemoryReadLayout {
  io: MemoryIOLayout,
  _0: IsForwardLayout,
}
struct BoundLayout_MemoryReadLayout {
  lyt: MemoryReadLayout,
  buf: u32,
}
fn lookup_MemoryReadLayout_io(b: BoundLayout_MemoryReadLayout) -> BoundLayout_MemoryIOLayout {
  return BoundLayout_MemoryIOLayout(b.lyt.io, b.buf);
}
fn lookup_MemoryReadLayout__0(b: BoundLayout_MemoryReadLayout) -> BoundLayout_IsForwardLayout {
  return BoundLayout_IsForwardLayout(b.lyt._0, b.buf);
}
struct DecodeInstLayout {
  _super: DecoderLayout,
  pcAddr: AddrDecomposeLayout,
  loadInst: MemoryReadLayout,
}
struct BoundLayout_DecodeInstLayout {
  lyt: DecodeInstLayout,
  buf: u32,
}
fn lookup_DecodeInstLayout__super(b: BoundLayout_DecodeInstLayout) -> BoundLayout_DecoderLayout {
  return BoundLayout_DecoderLayout(b.lyt._super, b.buf);
}
fn lookup_DecodeInstLayout_pcAddr(b: BoundLayout_DecodeInstLayout) -> BoundLayout_AddrDecomposeLayout {
  return BoundLayout_AddrDecomposeLayout(b.lyt.pcAddr, b.buf);
}
fn lookup_DecodeInstLayout_loadInst(b: BoundLayout_DecodeInstLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.loadInst, b.buf);
}
alias MemoryArgLayout4LayoutArray = array<MemoryArgLayout, 4>;
struct BoundLayout_MemoryArgLayout4LayoutArray {
  lyt: MemoryArgLayout4LayoutArray,
  buf: u32,
}
fn subscript_MemoryArgLayout4LayoutArray(b: BoundLayout_MemoryArgLayout4LayoutArray, i: u32) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt[i], b.buf);
}
alias CycleArgLayout2LayoutArray = array<CycleArgLayout, 2>;
struct BoundLayout_CycleArgLayout2LayoutArray {
  lyt: CycleArgLayout2LayoutArray,
  buf: u32,
}
fn subscript_CycleArgLayout2LayoutArray(b: BoundLayout_CycleArgLayout2LayoutArray, i: u32) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt[i], b.buf);
}
struct _Arguments_ReadSourceRegsSourceRegsLayout {
  memoryArg: MemoryArgLayout4LayoutArray,
  cycleArg: CycleArgLayout2LayoutArray,
}
struct BoundLayout__Arguments_ReadSourceRegsSourceRegsLayout {
  lyt: _Arguments_ReadSourceRegsSourceRegsLayout,
  buf: u32,
}
fn lookup__Arguments_ReadSourceRegsSourceRegsLayout_memoryArg(b: BoundLayout__Arguments_ReadSourceRegsSourceRegsLayout) -> BoundLayout_MemoryArgLayout4LayoutArray {
  return BoundLayout_MemoryArgLayout4LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_ReadSourceRegsSourceRegsLayout_cycleArg(b: BoundLayout__Arguments_ReadSourceRegsSourceRegsLayout) -> BoundLayout_CycleArgLayout2LayoutArray {
  return BoundLayout_CycleArgLayout2LayoutArray(b.lyt.cycleArg, b.buf);
}
struct ReadRegLayout {
  _super: MemoryReadLayout,
  addr: NondetRegLayout,
}
struct BoundLayout_ReadRegLayout {
  lyt: ReadRegLayout,
  buf: u32,
}
fn lookup_ReadRegLayout__super(b: BoundLayout_ReadRegLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt._super, b.buf);
}
fn lookup_ReadRegLayout_addr(b: BoundLayout_ReadRegLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.addr, b.buf);
}
struct ReadSourceRegsSourceRegsArm0_SuperLayout {
  rboth: ReadRegLayout,
}
struct BoundLayout_ReadSourceRegsSourceRegsArm0_SuperLayout {
  lyt: ReadSourceRegsSourceRegsArm0_SuperLayout,
  buf: u32,
}
fn lookup_ReadSourceRegsSourceRegsArm0_SuperLayout_rboth(b: BoundLayout_ReadSourceRegsSourceRegsArm0_SuperLayout) -> BoundLayout_ReadRegLayout {
  return BoundLayout_ReadRegLayout(b.lyt.rboth, b.buf);
}
struct ReadSourceRegsSourceRegsArm0Layout {
  _super: ReadSourceRegsSourceRegsArm0_SuperLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: CycleArgLayout,
}
struct BoundLayout_ReadSourceRegsSourceRegsArm0Layout {
  lyt: ReadSourceRegsSourceRegsArm0Layout,
  buf: u32,
}
fn lookup_ReadSourceRegsSourceRegsArm0Layout__super(b: BoundLayout_ReadSourceRegsSourceRegsArm0Layout) -> BoundLayout_ReadSourceRegsSourceRegsArm0_SuperLayout {
  return BoundLayout_ReadSourceRegsSourceRegsArm0_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ReadSourceRegsSourceRegsArm0Layout__extra0(b: BoundLayout_ReadSourceRegsSourceRegsArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ReadSourceRegsSourceRegsArm0Layout__extra1(b: BoundLayout_ReadSourceRegsSourceRegsArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ReadSourceRegsSourceRegsArm0Layout__extra2(b: BoundLayout_ReadSourceRegsSourceRegsArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra2, b.buf);
}
struct ReadSourceRegsSourceRegsArm1_SuperLayout {
  _0: ReadRegLayout,
  _1: ReadRegLayout,
}
struct BoundLayout_ReadSourceRegsSourceRegsArm1_SuperLayout {
  lyt: ReadSourceRegsSourceRegsArm1_SuperLayout,
  buf: u32,
}
fn lookup_ReadSourceRegsSourceRegsArm1_SuperLayout__0(b: BoundLayout_ReadSourceRegsSourceRegsArm1_SuperLayout) -> BoundLayout_ReadRegLayout {
  return BoundLayout_ReadRegLayout(b.lyt._0, b.buf);
}
fn lookup_ReadSourceRegsSourceRegsArm1_SuperLayout__1(b: BoundLayout_ReadSourceRegsSourceRegsArm1_SuperLayout) -> BoundLayout_ReadRegLayout {
  return BoundLayout_ReadRegLayout(b.lyt._1, b.buf);
}
struct ReadSourceRegsSourceRegsLayout {
  arm0: ReadSourceRegsSourceRegsArm0Layout,
  arm1: ReadSourceRegsSourceRegsArm1_SuperLayout,
}
struct BoundLayout_ReadSourceRegsSourceRegsLayout {
  lyt: ReadSourceRegsSourceRegsLayout,
  buf: u32,
}
fn lookup_ReadSourceRegsSourceRegsLayout_arm0(b: BoundLayout_ReadSourceRegsSourceRegsLayout) -> BoundLayout_ReadSourceRegsSourceRegsArm0Layout {
  return BoundLayout_ReadSourceRegsSourceRegsArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_ReadSourceRegsSourceRegsLayout_arm1(b: BoundLayout_ReadSourceRegsSourceRegsLayout) -> BoundLayout_ReadSourceRegsSourceRegsArm1_SuperLayout {
  return BoundLayout_ReadSourceRegsSourceRegsArm1_SuperLayout(b.lyt.arm1, b.buf);
}
struct ReadSourceRegsLayout {
  isSameReg: NondetRegLayout,
  _arguments_ReadSourceRegsSourceRegs: _Arguments_ReadSourceRegsSourceRegsLayout,
  sourceRegs: ReadSourceRegsSourceRegsLayout,
  rs1Low: NondetRegLayout,
  rs1High: NondetRegLayout,
  rs2Low: NondetRegLayout,
  rs2High: NondetRegLayout,
}
struct BoundLayout_ReadSourceRegsLayout {
  lyt: ReadSourceRegsLayout,
  buf: u32,
}
fn lookup_ReadSourceRegsLayout_isSameReg(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isSameReg, b.buf);
}
fn lookup_ReadSourceRegsLayout__arguments_ReadSourceRegsSourceRegs(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout__Arguments_ReadSourceRegsSourceRegsLayout {
  return BoundLayout__Arguments_ReadSourceRegsSourceRegsLayout(b.lyt._arguments_ReadSourceRegsSourceRegs, b.buf);
}
fn lookup_ReadSourceRegsLayout_sourceRegs(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout_ReadSourceRegsSourceRegsLayout {
  return BoundLayout_ReadSourceRegsSourceRegsLayout(b.lyt.sourceRegs, b.buf);
}
fn lookup_ReadSourceRegsLayout_rs1Low(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.rs1Low, b.buf);
}
fn lookup_ReadSourceRegsLayout_rs1High(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.rs1High, b.buf);
}
fn lookup_ReadSourceRegsLayout_rs2Low(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.rs2Low, b.buf);
}
fn lookup_ReadSourceRegsLayout_rs2High(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.rs2High, b.buf);
}
struct MiscInputLayout {
  decoded: DecodeInstLayout,
  sourceRegs: ReadSourceRegsLayout,
}
struct BoundLayout_MiscInputLayout {
  lyt: MiscInputLayout,
  buf: u32,
}
fn lookup_MiscInputLayout_decoded(b: BoundLayout_MiscInputLayout) -> BoundLayout_DecodeInstLayout {
  return BoundLayout_DecodeInstLayout(b.lyt.decoded, b.buf);
}
fn lookup_MiscInputLayout_sourceRegs(b: BoundLayout_MiscInputLayout) -> BoundLayout_ReadSourceRegsLayout {
  return BoundLayout_ReadSourceRegsLayout(b.lyt.sourceRegs, b.buf);
}
alias ArgU16Layout5LayoutArray = array<ArgU16Layout, 5>;
struct BoundLayout_ArgU16Layout5LayoutArray {
  lyt: ArgU16Layout5LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout5LayoutArray(b: BoundLayout_ArgU16Layout5LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
struct _Arguments_Misc0MiscOutputLayout {
  argU16: ArgU16Layout5LayoutArray,
}
struct BoundLayout__Arguments_Misc0MiscOutputLayout {
  lyt: _Arguments_Misc0MiscOutputLayout,
  buf: u32,
}
fn lookup__Arguments_Misc0MiscOutputLayout_argU16(b: BoundLayout__Arguments_Misc0MiscOutputLayout) -> BoundLayout_ArgU16Layout5LayoutArray {
  return BoundLayout_ArgU16Layout5LayoutArray(b.lyt.argU16, b.buf);
}
struct Misc0MiscOutputArm0Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm0Layout {
  lyt: Misc0MiscOutputArm0Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm0Layout__extra0(b: BoundLayout_Misc0MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm0Layout__extra1(b: BoundLayout_Misc0MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm0Layout__extra2(b: BoundLayout_Misc0MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc0MiscOutputArm0Layout__extra3(b: BoundLayout_Misc0MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc0MiscOutputArm0Layout__extra4(b: BoundLayout_Misc0MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct Misc0MiscOutputArm1Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm1Layout {
  lyt: Misc0MiscOutputArm1Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm1Layout__extra0(b: BoundLayout_Misc0MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm1Layout__extra1(b: BoundLayout_Misc0MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm1Layout__extra2(b: BoundLayout_Misc0MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc0MiscOutputArm1Layout__extra3(b: BoundLayout_Misc0MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc0MiscOutputArm1Layout__extra4(b: BoundLayout_Misc0MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
alias NondetRegLayout16LayoutArray = array<NondetRegLayout, 16>;
struct BoundLayout_NondetRegLayout16LayoutArray {
  lyt: NondetRegLayout16LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout16LayoutArray(b: BoundLayout_NondetRegLayout16LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct ToBits_16_Layout {
  _super: NondetRegLayout16LayoutArray,
}
struct BoundLayout_ToBits_16_Layout {
  lyt: ToBits_16_Layout,
  buf: u32,
}
fn lookup_ToBits_16_Layout__super(b: BoundLayout_ToBits_16_Layout) -> BoundLayout_NondetRegLayout16LayoutArray {
  return BoundLayout_NondetRegLayout16LayoutArray(b.lyt._super, b.buf);
}
struct BitwiseAndU16Layout {
  bitsX: ToBits_16_Layout,
  bitsY: ToBits_16_Layout,
}
struct BoundLayout_BitwiseAndU16Layout {
  lyt: BitwiseAndU16Layout,
  buf: u32,
}
fn lookup_BitwiseAndU16Layout_bitsX(b: BoundLayout_BitwiseAndU16Layout) -> BoundLayout_ToBits_16_Layout {
  return BoundLayout_ToBits_16_Layout(b.lyt.bitsX, b.buf);
}
fn lookup_BitwiseAndU16Layout_bitsY(b: BoundLayout_BitwiseAndU16Layout) -> BoundLayout_ToBits_16_Layout {
  return BoundLayout_ToBits_16_Layout(b.lyt.bitsY, b.buf);
}
struct BitwiseAndLayout {
  _0: BitwiseAndU16Layout,
  _1: BitwiseAndU16Layout,
}
struct BoundLayout_BitwiseAndLayout {
  lyt: BitwiseAndLayout,
  buf: u32,
}
fn lookup_BitwiseAndLayout__0(b: BoundLayout_BitwiseAndLayout) -> BoundLayout_BitwiseAndU16Layout {
  return BoundLayout_BitwiseAndU16Layout(b.lyt._0, b.buf);
}
fn lookup_BitwiseAndLayout__1(b: BoundLayout_BitwiseAndLayout) -> BoundLayout_BitwiseAndU16Layout {
  return BoundLayout_BitwiseAndU16Layout(b.lyt._1, b.buf);
}
struct BitwiseXorLayout {
  andXy: BitwiseAndLayout,
}
struct BoundLayout_BitwiseXorLayout {
  lyt: BitwiseXorLayout,
  buf: u32,
}
fn lookup_BitwiseXorLayout_andXy(b: BoundLayout_BitwiseXorLayout) -> BoundLayout_BitwiseAndLayout {
  return BoundLayout_BitwiseAndLayout(b.lyt.andXy, b.buf);
}
struct OpXORLayout {
  _0: BitwiseXorLayout,
}
struct BoundLayout_OpXORLayout {
  lyt: OpXORLayout,
  buf: u32,
}
fn lookup_OpXORLayout__0(b: BoundLayout_OpXORLayout) -> BoundLayout_BitwiseXorLayout {
  return BoundLayout_BitwiseXorLayout(b.lyt._0, b.buf);
}
struct Misc0MiscOutputArm2Layout {
  _super: OpXORLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm2Layout {
  lyt: Misc0MiscOutputArm2Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm2Layout__super(b: BoundLayout_Misc0MiscOutputArm2Layout) -> BoundLayout_OpXORLayout {
  return BoundLayout_OpXORLayout(b.lyt._super, b.buf);
}
fn lookup_Misc0MiscOutputArm2Layout__extra0(b: BoundLayout_Misc0MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm2Layout__extra1(b: BoundLayout_Misc0MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm2Layout__extra2(b: BoundLayout_Misc0MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc0MiscOutputArm2Layout__extra3(b: BoundLayout_Misc0MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc0MiscOutputArm2Layout__extra4(b: BoundLayout_Misc0MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct BitwiseOrLayout {
  andXy: BitwiseAndLayout,
}
struct BoundLayout_BitwiseOrLayout {
  lyt: BitwiseOrLayout,
  buf: u32,
}
fn lookup_BitwiseOrLayout_andXy(b: BoundLayout_BitwiseOrLayout) -> BoundLayout_BitwiseAndLayout {
  return BoundLayout_BitwiseAndLayout(b.lyt.andXy, b.buf);
}
struct OpORLayout {
  _0: BitwiseOrLayout,
}
struct BoundLayout_OpORLayout {
  lyt: OpORLayout,
  buf: u32,
}
fn lookup_OpORLayout__0(b: BoundLayout_OpORLayout) -> BoundLayout_BitwiseOrLayout {
  return BoundLayout_BitwiseOrLayout(b.lyt._0, b.buf);
}
struct Misc0MiscOutputArm3Layout {
  _super: OpORLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm3Layout {
  lyt: Misc0MiscOutputArm3Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm3Layout__super(b: BoundLayout_Misc0MiscOutputArm3Layout) -> BoundLayout_OpORLayout {
  return BoundLayout_OpORLayout(b.lyt._super, b.buf);
}
fn lookup_Misc0MiscOutputArm3Layout__extra0(b: BoundLayout_Misc0MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm3Layout__extra1(b: BoundLayout_Misc0MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm3Layout__extra2(b: BoundLayout_Misc0MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc0MiscOutputArm3Layout__extra3(b: BoundLayout_Misc0MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc0MiscOutputArm3Layout__extra4(b: BoundLayout_Misc0MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpANDLayout {
  _0: BitwiseAndLayout,
}
struct BoundLayout_OpANDLayout {
  lyt: OpANDLayout,
  buf: u32,
}
fn lookup_OpANDLayout__0(b: BoundLayout_OpANDLayout) -> BoundLayout_BitwiseAndLayout {
  return BoundLayout_BitwiseAndLayout(b.lyt._0, b.buf);
}
struct Misc0MiscOutputArm4Layout {
  _super: OpANDLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm4Layout {
  lyt: Misc0MiscOutputArm4Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm4Layout__super(b: BoundLayout_Misc0MiscOutputArm4Layout) -> BoundLayout_OpANDLayout {
  return BoundLayout_OpANDLayout(b.lyt._super, b.buf);
}
fn lookup_Misc0MiscOutputArm4Layout__extra0(b: BoundLayout_Misc0MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm4Layout__extra1(b: BoundLayout_Misc0MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm4Layout__extra2(b: BoundLayout_Misc0MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc0MiscOutputArm4Layout__extra3(b: BoundLayout_Misc0MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc0MiscOutputArm4Layout__extra4(b: BoundLayout_Misc0MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct GetSignU32Layout {
  _super: NondetRegLayout,
  restTimesTwo: NondetU16RegLayout,
}
struct BoundLayout_GetSignU32Layout {
  lyt: GetSignU32Layout,
  buf: u32,
}
fn lookup_GetSignU32Layout__super(b: BoundLayout_GetSignU32Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._super, b.buf);
}
fn lookup_GetSignU32Layout_restTimesTwo(b: BoundLayout_GetSignU32Layout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.restTimesTwo, b.buf);
}
struct CmpLessThanLayout {
  diff: NormalizeU32Layout,
  s1: GetSignU32Layout,
  s2: GetSignU32Layout,
  s3: GetSignU32Layout,
  overflow: NondetRegLayout,
  isLessThan: NondetRegLayout,
}
struct BoundLayout_CmpLessThanLayout {
  lyt: CmpLessThanLayout,
  buf: u32,
}
fn lookup_CmpLessThanLayout_diff(b: BoundLayout_CmpLessThanLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.diff, b.buf);
}
fn lookup_CmpLessThanLayout_s1(b: BoundLayout_CmpLessThanLayout) -> BoundLayout_GetSignU32Layout {
  return BoundLayout_GetSignU32Layout(b.lyt.s1, b.buf);
}
fn lookup_CmpLessThanLayout_s2(b: BoundLayout_CmpLessThanLayout) -> BoundLayout_GetSignU32Layout {
  return BoundLayout_GetSignU32Layout(b.lyt.s2, b.buf);
}
fn lookup_CmpLessThanLayout_s3(b: BoundLayout_CmpLessThanLayout) -> BoundLayout_GetSignU32Layout {
  return BoundLayout_GetSignU32Layout(b.lyt.s3, b.buf);
}
fn lookup_CmpLessThanLayout_overflow(b: BoundLayout_CmpLessThanLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.overflow, b.buf);
}
fn lookup_CmpLessThanLayout_isLessThan(b: BoundLayout_CmpLessThanLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isLessThan, b.buf);
}
struct OpSLTLayout {
  cmp: CmpLessThanLayout,
}
struct BoundLayout_OpSLTLayout {
  lyt: OpSLTLayout,
  buf: u32,
}
fn lookup_OpSLTLayout_cmp(b: BoundLayout_OpSLTLayout) -> BoundLayout_CmpLessThanLayout {
  return BoundLayout_CmpLessThanLayout(b.lyt.cmp, b.buf);
}
struct CmpLessThanUnsignedLayout {
  diff: NormalizeU32Layout,
}
struct BoundLayout_CmpLessThanUnsignedLayout {
  lyt: CmpLessThanUnsignedLayout,
  buf: u32,
}
fn lookup_CmpLessThanUnsignedLayout_diff(b: BoundLayout_CmpLessThanUnsignedLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.diff, b.buf);
}
struct OpSLTULayout {
  cmp: CmpLessThanUnsignedLayout,
}
struct BoundLayout_OpSLTULayout {
  lyt: OpSLTULayout,
  buf: u32,
}
fn lookup_OpSLTULayout_cmp(b: BoundLayout_OpSLTULayout) -> BoundLayout_CmpLessThanUnsignedLayout {
  return BoundLayout_CmpLessThanUnsignedLayout(b.lyt.cmp, b.buf);
}
struct Misc0MiscOutputArm6Layout {
  _super: OpSLTULayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm6Layout {
  lyt: Misc0MiscOutputArm6Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm6Layout__super(b: BoundLayout_Misc0MiscOutputArm6Layout) -> BoundLayout_OpSLTULayout {
  return BoundLayout_OpSLTULayout(b.lyt._super, b.buf);
}
fn lookup_Misc0MiscOutputArm6Layout__extra0(b: BoundLayout_Misc0MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm6Layout__extra1(b: BoundLayout_Misc0MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm6Layout__extra2(b: BoundLayout_Misc0MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
struct Misc0MiscOutputArm7Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm7Layout {
  lyt: Misc0MiscOutputArm7Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm7Layout__extra0(b: BoundLayout_Misc0MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm7Layout__extra1(b: BoundLayout_Misc0MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm7Layout__extra2(b: BoundLayout_Misc0MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc0MiscOutputArm7Layout__extra3(b: BoundLayout_Misc0MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc0MiscOutputArm7Layout__extra4(b: BoundLayout_Misc0MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct Misc0MiscOutputLayout {
  arm0: Misc0MiscOutputArm0Layout,
  arm1: Misc0MiscOutputArm1Layout,
  arm2: Misc0MiscOutputArm2Layout,
  arm3: Misc0MiscOutputArm3Layout,
  arm4: Misc0MiscOutputArm4Layout,
  arm5: OpSLTLayout,
  arm6: Misc0MiscOutputArm6Layout,
  arm7: Misc0MiscOutputArm7Layout,
}
struct BoundLayout_Misc0MiscOutputLayout {
  lyt: Misc0MiscOutputLayout,
  buf: u32,
}
fn lookup_Misc0MiscOutputLayout_arm0(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm0Layout {
  return BoundLayout_Misc0MiscOutputArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm1(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm1Layout {
  return BoundLayout_Misc0MiscOutputArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm2(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm2Layout {
  return BoundLayout_Misc0MiscOutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm3(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm3Layout {
  return BoundLayout_Misc0MiscOutputArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm4(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm4Layout {
  return BoundLayout_Misc0MiscOutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm5(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_OpSLTLayout {
  return BoundLayout_OpSLTLayout(b.lyt.arm5, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm6(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm6Layout {
  return BoundLayout_Misc0MiscOutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm7(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm7Layout {
  return BoundLayout_Misc0MiscOutputArm7Layout(b.lyt.arm7, b.buf);
}
struct Misc0Layout {
  _super: FinalizeMiscLayout,
  _0: DoCycleTableLayout,
  input: MiscInputLayout,
  _arguments_Misc0MiscOutput: _Arguments_Misc0MiscOutputLayout,
  miscOutput: Misc0MiscOutputLayout,
}
struct BoundLayout_Misc0Layout {
  lyt: Misc0Layout,
  buf: u32,
}
fn lookup_Misc0Layout__super(b: BoundLayout_Misc0Layout) -> BoundLayout_FinalizeMiscLayout {
  return BoundLayout_FinalizeMiscLayout(b.lyt._super, b.buf);
}
fn lookup_Misc0Layout__0(b: BoundLayout_Misc0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Misc0Layout_input(b: BoundLayout_Misc0Layout) -> BoundLayout_MiscInputLayout {
  return BoundLayout_MiscInputLayout(b.lyt.input, b.buf);
}
fn lookup_Misc0Layout__arguments_Misc0MiscOutput(b: BoundLayout_Misc0Layout) -> BoundLayout__Arguments_Misc0MiscOutputLayout {
  return BoundLayout__Arguments_Misc0MiscOutputLayout(b.lyt._arguments_Misc0MiscOutput, b.buf);
}
fn lookup_Misc0Layout_miscOutput(b: BoundLayout_Misc0Layout) -> BoundLayout_Misc0MiscOutputLayout {
  return BoundLayout_Misc0MiscOutputLayout(b.lyt.miscOutput, b.buf);
}
struct _Arguments_Misc1MiscOutputLayout {
  argU16: ArgU16Layout5LayoutArray,
}
struct BoundLayout__Arguments_Misc1MiscOutputLayout {
  lyt: _Arguments_Misc1MiscOutputLayout,
  buf: u32,
}
fn lookup__Arguments_Misc1MiscOutputLayout_argU16(b: BoundLayout__Arguments_Misc1MiscOutputLayout) -> BoundLayout_ArgU16Layout5LayoutArray {
  return BoundLayout_ArgU16Layout5LayoutArray(b.lyt.argU16, b.buf);
}
struct OpXORILayout {
  _0: BitwiseXorLayout,
}
struct BoundLayout_OpXORILayout {
  lyt: OpXORILayout,
  buf: u32,
}
fn lookup_OpXORILayout__0(b: BoundLayout_OpXORILayout) -> BoundLayout_BitwiseXorLayout {
  return BoundLayout_BitwiseXorLayout(b.lyt._0, b.buf);
}
struct Misc1MiscOutputArm0Layout {
  _super: OpXORILayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc1MiscOutputArm0Layout {
  lyt: Misc1MiscOutputArm0Layout,
  buf: u32,
}
fn lookup_Misc1MiscOutputArm0Layout__super(b: BoundLayout_Misc1MiscOutputArm0Layout) -> BoundLayout_OpXORILayout {
  return BoundLayout_OpXORILayout(b.lyt._super, b.buf);
}
fn lookup_Misc1MiscOutputArm0Layout__extra0(b: BoundLayout_Misc1MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc1MiscOutputArm0Layout__extra1(b: BoundLayout_Misc1MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc1MiscOutputArm0Layout__extra2(b: BoundLayout_Misc1MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc1MiscOutputArm0Layout__extra3(b: BoundLayout_Misc1MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc1MiscOutputArm0Layout__extra4(b: BoundLayout_Misc1MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpORILayout {
  _0: BitwiseOrLayout,
}
struct BoundLayout_OpORILayout {
  lyt: OpORILayout,
  buf: u32,
}
fn lookup_OpORILayout__0(b: BoundLayout_OpORILayout) -> BoundLayout_BitwiseOrLayout {
  return BoundLayout_BitwiseOrLayout(b.lyt._0, b.buf);
}
struct Misc1MiscOutputArm1Layout {
  _super: OpORILayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc1MiscOutputArm1Layout {
  lyt: Misc1MiscOutputArm1Layout,
  buf: u32,
}
fn lookup_Misc1MiscOutputArm1Layout__super(b: BoundLayout_Misc1MiscOutputArm1Layout) -> BoundLayout_OpORILayout {
  return BoundLayout_OpORILayout(b.lyt._super, b.buf);
}
fn lookup_Misc1MiscOutputArm1Layout__extra0(b: BoundLayout_Misc1MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc1MiscOutputArm1Layout__extra1(b: BoundLayout_Misc1MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc1MiscOutputArm1Layout__extra2(b: BoundLayout_Misc1MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc1MiscOutputArm1Layout__extra3(b: BoundLayout_Misc1MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc1MiscOutputArm1Layout__extra4(b: BoundLayout_Misc1MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpANDILayout {
  _0: BitwiseAndLayout,
}
struct BoundLayout_OpANDILayout {
  lyt: OpANDILayout,
  buf: u32,
}
fn lookup_OpANDILayout__0(b: BoundLayout_OpANDILayout) -> BoundLayout_BitwiseAndLayout {
  return BoundLayout_BitwiseAndLayout(b.lyt._0, b.buf);
}
struct Misc1MiscOutputArm2Layout {
  _super: OpANDILayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc1MiscOutputArm2Layout {
  lyt: Misc1MiscOutputArm2Layout,
  buf: u32,
}
fn lookup_Misc1MiscOutputArm2Layout__super(b: BoundLayout_Misc1MiscOutputArm2Layout) -> BoundLayout_OpANDILayout {
  return BoundLayout_OpANDILayout(b.lyt._super, b.buf);
}
fn lookup_Misc1MiscOutputArm2Layout__extra0(b: BoundLayout_Misc1MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc1MiscOutputArm2Layout__extra1(b: BoundLayout_Misc1MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc1MiscOutputArm2Layout__extra2(b: BoundLayout_Misc1MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc1MiscOutputArm2Layout__extra3(b: BoundLayout_Misc1MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc1MiscOutputArm2Layout__extra4(b: BoundLayout_Misc1MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpSLTILayout {
  cmp: CmpLessThanLayout,
}
struct BoundLayout_OpSLTILayout {
  lyt: OpSLTILayout,
  buf: u32,
}
fn lookup_OpSLTILayout_cmp(b: BoundLayout_OpSLTILayout) -> BoundLayout_CmpLessThanLayout {
  return BoundLayout_CmpLessThanLayout(b.lyt.cmp, b.buf);
}
struct OpSLTIULayout {
  cmp: CmpLessThanUnsignedLayout,
}
struct BoundLayout_OpSLTIULayout {
  lyt: OpSLTIULayout,
  buf: u32,
}
fn lookup_OpSLTIULayout_cmp(b: BoundLayout_OpSLTIULayout) -> BoundLayout_CmpLessThanUnsignedLayout {
  return BoundLayout_CmpLessThanUnsignedLayout(b.lyt.cmp, b.buf);
}
struct Misc1MiscOutputArm4Layout {
  _super: OpSLTIULayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
}
struct BoundLayout_Misc1MiscOutputArm4Layout {
  lyt: Misc1MiscOutputArm4Layout,
  buf: u32,
}
fn lookup_Misc1MiscOutputArm4Layout__super(b: BoundLayout_Misc1MiscOutputArm4Layout) -> BoundLayout_OpSLTIULayout {
  return BoundLayout_OpSLTIULayout(b.lyt._super, b.buf);
}
fn lookup_Misc1MiscOutputArm4Layout__extra0(b: BoundLayout_Misc1MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc1MiscOutputArm4Layout__extra1(b: BoundLayout_Misc1MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc1MiscOutputArm4Layout__extra2(b: BoundLayout_Misc1MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
struct CmpEqualLayout {
  lowSame: IsZeroLayout,
  highSame: IsZeroLayout,
  isEqual: NondetRegLayout,
}
struct BoundLayout_CmpEqualLayout {
  lyt: CmpEqualLayout,
  buf: u32,
}
fn lookup_CmpEqualLayout_lowSame(b: BoundLayout_CmpEqualLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lowSame, b.buf);
}
fn lookup_CmpEqualLayout_highSame(b: BoundLayout_CmpEqualLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.highSame, b.buf);
}
fn lookup_CmpEqualLayout_isEqual(b: BoundLayout_CmpEqualLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isEqual, b.buf);
}
struct OpBEQLayout {
  cmp: CmpEqualLayout,
}
struct BoundLayout_OpBEQLayout {
  lyt: OpBEQLayout,
  buf: u32,
}
fn lookup_OpBEQLayout_cmp(b: BoundLayout_OpBEQLayout) -> BoundLayout_CmpEqualLayout {
  return BoundLayout_CmpEqualLayout(b.lyt.cmp, b.buf);
}
struct Misc1MiscOutputArm5Layout {
  _super: OpBEQLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc1MiscOutputArm5Layout {
  lyt: Misc1MiscOutputArm5Layout,
  buf: u32,
}
fn lookup_Misc1MiscOutputArm5Layout__super(b: BoundLayout_Misc1MiscOutputArm5Layout) -> BoundLayout_OpBEQLayout {
  return BoundLayout_OpBEQLayout(b.lyt._super, b.buf);
}
fn lookup_Misc1MiscOutputArm5Layout__extra0(b: BoundLayout_Misc1MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc1MiscOutputArm5Layout__extra1(b: BoundLayout_Misc1MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc1MiscOutputArm5Layout__extra2(b: BoundLayout_Misc1MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc1MiscOutputArm5Layout__extra3(b: BoundLayout_Misc1MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc1MiscOutputArm5Layout__extra4(b: BoundLayout_Misc1MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpBNELayout {
  cmp: CmpEqualLayout,
}
struct BoundLayout_OpBNELayout {
  lyt: OpBNELayout,
  buf: u32,
}
fn lookup_OpBNELayout_cmp(b: BoundLayout_OpBNELayout) -> BoundLayout_CmpEqualLayout {
  return BoundLayout_CmpEqualLayout(b.lyt.cmp, b.buf);
}
struct Misc1MiscOutputArm6Layout {
  _super: OpBNELayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc1MiscOutputArm6Layout {
  lyt: Misc1MiscOutputArm6Layout,
  buf: u32,
}
fn lookup_Misc1MiscOutputArm6Layout__super(b: BoundLayout_Misc1MiscOutputArm6Layout) -> BoundLayout_OpBNELayout {
  return BoundLayout_OpBNELayout(b.lyt._super, b.buf);
}
fn lookup_Misc1MiscOutputArm6Layout__extra0(b: BoundLayout_Misc1MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc1MiscOutputArm6Layout__extra1(b: BoundLayout_Misc1MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc1MiscOutputArm6Layout__extra2(b: BoundLayout_Misc1MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc1MiscOutputArm6Layout__extra3(b: BoundLayout_Misc1MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc1MiscOutputArm6Layout__extra4(b: BoundLayout_Misc1MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpBLTLayout {
  cmp: CmpLessThanLayout,
}
struct BoundLayout_OpBLTLayout {
  lyt: OpBLTLayout,
  buf: u32,
}
fn lookup_OpBLTLayout_cmp(b: BoundLayout_OpBLTLayout) -> BoundLayout_CmpLessThanLayout {
  return BoundLayout_CmpLessThanLayout(b.lyt.cmp, b.buf);
}
struct Misc1MiscOutputLayout {
  arm0: Misc1MiscOutputArm0Layout,
  arm1: Misc1MiscOutputArm1Layout,
  arm2: Misc1MiscOutputArm2Layout,
  arm3: OpSLTILayout,
  arm4: Misc1MiscOutputArm4Layout,
  arm5: Misc1MiscOutputArm5Layout,
  arm6: Misc1MiscOutputArm6Layout,
  arm7: OpBLTLayout,
}
struct BoundLayout_Misc1MiscOutputLayout {
  lyt: Misc1MiscOutputLayout,
  buf: u32,
}
fn lookup_Misc1MiscOutputLayout_arm0(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_Misc1MiscOutputArm0Layout {
  return BoundLayout_Misc1MiscOutputArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm1(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_Misc1MiscOutputArm1Layout {
  return BoundLayout_Misc1MiscOutputArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm2(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_Misc1MiscOutputArm2Layout {
  return BoundLayout_Misc1MiscOutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm3(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_OpSLTILayout {
  return BoundLayout_OpSLTILayout(b.lyt.arm3, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm4(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_Misc1MiscOutputArm4Layout {
  return BoundLayout_Misc1MiscOutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm5(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_Misc1MiscOutputArm5Layout {
  return BoundLayout_Misc1MiscOutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm6(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_Misc1MiscOutputArm6Layout {
  return BoundLayout_Misc1MiscOutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm7(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_OpBLTLayout {
  return BoundLayout_OpBLTLayout(b.lyt.arm7, b.buf);
}
struct Misc1Layout {
  _super: FinalizeMiscLayout,
  _0: DoCycleTableLayout,
  input: MiscInputLayout,
  _arguments_Misc1MiscOutput: _Arguments_Misc1MiscOutputLayout,
  miscOutput: Misc1MiscOutputLayout,
}
struct BoundLayout_Misc1Layout {
  lyt: Misc1Layout,
  buf: u32,
}
fn lookup_Misc1Layout__super(b: BoundLayout_Misc1Layout) -> BoundLayout_FinalizeMiscLayout {
  return BoundLayout_FinalizeMiscLayout(b.lyt._super, b.buf);
}
fn lookup_Misc1Layout__0(b: BoundLayout_Misc1Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Misc1Layout_input(b: BoundLayout_Misc1Layout) -> BoundLayout_MiscInputLayout {
  return BoundLayout_MiscInputLayout(b.lyt.input, b.buf);
}
fn lookup_Misc1Layout__arguments_Misc1MiscOutput(b: BoundLayout_Misc1Layout) -> BoundLayout__Arguments_Misc1MiscOutputLayout {
  return BoundLayout__Arguments_Misc1MiscOutputLayout(b.lyt._arguments_Misc1MiscOutput, b.buf);
}
fn lookup_Misc1Layout_miscOutput(b: BoundLayout_Misc1Layout) -> BoundLayout_Misc1MiscOutputLayout {
  return BoundLayout_Misc1MiscOutputLayout(b.lyt.miscOutput, b.buf);
}
struct _Arguments_Misc2MiscOutputLayout {
  argU16: ArgU16Layout5LayoutArray,
}
struct BoundLayout__Arguments_Misc2MiscOutputLayout {
  lyt: _Arguments_Misc2MiscOutputLayout,
  buf: u32,
}
fn lookup__Arguments_Misc2MiscOutputLayout_argU16(b: BoundLayout__Arguments_Misc2MiscOutputLayout) -> BoundLayout_ArgU16Layout5LayoutArray {
  return BoundLayout_ArgU16Layout5LayoutArray(b.lyt.argU16, b.buf);
}
struct OpBGELayout {
  cmp: CmpLessThanLayout,
}
struct BoundLayout_OpBGELayout {
  lyt: OpBGELayout,
  buf: u32,
}
fn lookup_OpBGELayout_cmp(b: BoundLayout_OpBGELayout) -> BoundLayout_CmpLessThanLayout {
  return BoundLayout_CmpLessThanLayout(b.lyt.cmp, b.buf);
}
struct OpBLTULayout {
  cmp: CmpLessThanUnsignedLayout,
}
struct BoundLayout_OpBLTULayout {
  lyt: OpBLTULayout,
  buf: u32,
}
fn lookup_OpBLTULayout_cmp(b: BoundLayout_OpBLTULayout) -> BoundLayout_CmpLessThanUnsignedLayout {
  return BoundLayout_CmpLessThanUnsignedLayout(b.lyt.cmp, b.buf);
}
struct Misc2MiscOutputArm1Layout {
  _super: OpBLTULayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm1Layout {
  lyt: Misc2MiscOutputArm1Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm1Layout__super(b: BoundLayout_Misc2MiscOutputArm1Layout) -> BoundLayout_OpBLTULayout {
  return BoundLayout_OpBLTULayout(b.lyt._super, b.buf);
}
fn lookup_Misc2MiscOutputArm1Layout__extra0(b: BoundLayout_Misc2MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm1Layout__extra1(b: BoundLayout_Misc2MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm1Layout__extra2(b: BoundLayout_Misc2MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
struct OpBGEULayout {
  cmp: CmpLessThanUnsignedLayout,
}
struct BoundLayout_OpBGEULayout {
  lyt: OpBGEULayout,
  buf: u32,
}
fn lookup_OpBGEULayout_cmp(b: BoundLayout_OpBGEULayout) -> BoundLayout_CmpLessThanUnsignedLayout {
  return BoundLayout_CmpLessThanUnsignedLayout(b.lyt.cmp, b.buf);
}
struct Misc2MiscOutputArm2Layout {
  _super: OpBGEULayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm2Layout {
  lyt: Misc2MiscOutputArm2Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm2Layout__super(b: BoundLayout_Misc2MiscOutputArm2Layout) -> BoundLayout_OpBGEULayout {
  return BoundLayout_OpBGEULayout(b.lyt._super, b.buf);
}
fn lookup_Misc2MiscOutputArm2Layout__extra0(b: BoundLayout_Misc2MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm2Layout__extra1(b: BoundLayout_Misc2MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm2Layout__extra2(b: BoundLayout_Misc2MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
struct Misc2MiscOutputArm3Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm3Layout {
  lyt: Misc2MiscOutputArm3Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm3Layout__extra0(b: BoundLayout_Misc2MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm3Layout__extra1(b: BoundLayout_Misc2MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm3Layout__extra2(b: BoundLayout_Misc2MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc2MiscOutputArm3Layout__extra3(b: BoundLayout_Misc2MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc2MiscOutputArm3Layout__extra4(b: BoundLayout_Misc2MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpJALRLayout {
  lsb: NondetRegLayout,
  half: NondetU16RegLayout,
}
struct BoundLayout_OpJALRLayout {
  lyt: OpJALRLayout,
  buf: u32,
}
fn lookup_OpJALRLayout_lsb(b: BoundLayout_OpJALRLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.lsb, b.buf);
}
fn lookup_OpJALRLayout_half(b: BoundLayout_OpJALRLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.half, b.buf);
}
struct Misc2MiscOutputArm4Layout {
  _super: OpJALRLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm4Layout {
  lyt: Misc2MiscOutputArm4Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm4Layout__super(b: BoundLayout_Misc2MiscOutputArm4Layout) -> BoundLayout_OpJALRLayout {
  return BoundLayout_OpJALRLayout(b.lyt._super, b.buf);
}
fn lookup_Misc2MiscOutputArm4Layout__extra0(b: BoundLayout_Misc2MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm4Layout__extra1(b: BoundLayout_Misc2MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm4Layout__extra2(b: BoundLayout_Misc2MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc2MiscOutputArm4Layout__extra3(b: BoundLayout_Misc2MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
struct Misc2MiscOutputArm5Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm5Layout {
  lyt: Misc2MiscOutputArm5Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm5Layout__extra0(b: BoundLayout_Misc2MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm5Layout__extra1(b: BoundLayout_Misc2MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm5Layout__extra2(b: BoundLayout_Misc2MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc2MiscOutputArm5Layout__extra3(b: BoundLayout_Misc2MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc2MiscOutputArm5Layout__extra4(b: BoundLayout_Misc2MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct Misc2MiscOutputArm6Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm6Layout {
  lyt: Misc2MiscOutputArm6Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm6Layout__extra0(b: BoundLayout_Misc2MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm6Layout__extra1(b: BoundLayout_Misc2MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm6Layout__extra2(b: BoundLayout_Misc2MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc2MiscOutputArm6Layout__extra3(b: BoundLayout_Misc2MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc2MiscOutputArm6Layout__extra4(b: BoundLayout_Misc2MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct Misc2MiscOutputArm7Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm7Layout {
  lyt: Misc2MiscOutputArm7Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm7Layout__extra0(b: BoundLayout_Misc2MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm7Layout__extra1(b: BoundLayout_Misc2MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm7Layout__extra2(b: BoundLayout_Misc2MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc2MiscOutputArm7Layout__extra3(b: BoundLayout_Misc2MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc2MiscOutputArm7Layout__extra4(b: BoundLayout_Misc2MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct Misc2MiscOutputLayout {
  arm0: OpBGELayout,
  arm1: Misc2MiscOutputArm1Layout,
  arm2: Misc2MiscOutputArm2Layout,
  arm3: Misc2MiscOutputArm3Layout,
  arm4: Misc2MiscOutputArm4Layout,
  arm5: Misc2MiscOutputArm5Layout,
  arm6: Misc2MiscOutputArm6Layout,
  arm7: Misc2MiscOutputArm7Layout,
}
struct BoundLayout_Misc2MiscOutputLayout {
  lyt: Misc2MiscOutputLayout,
  buf: u32,
}
fn lookup_Misc2MiscOutputLayout_arm0(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_OpBGELayout {
  return BoundLayout_OpBGELayout(b.lyt.arm0, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm1(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm1Layout {
  return BoundLayout_Misc2MiscOutputArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm2(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm2Layout {
  return BoundLayout_Misc2MiscOutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm3(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm3Layout {
  return BoundLayout_Misc2MiscOutputArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm4(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm4Layout {
  return BoundLayout_Misc2MiscOutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm5(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm5Layout {
  return BoundLayout_Misc2MiscOutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm6(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm6Layout {
  return BoundLayout_Misc2MiscOutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm7(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm7Layout {
  return BoundLayout_Misc2MiscOutputArm7Layout(b.lyt.arm7, b.buf);
}
struct Misc2Layout {
  _super: FinalizeMiscLayout,
  _0: DoCycleTableLayout,
  input: MiscInputLayout,
  _arguments_Misc2MiscOutput: _Arguments_Misc2MiscOutputLayout,
  miscOutput: Misc2MiscOutputLayout,
}
struct BoundLayout_Misc2Layout {
  lyt: Misc2Layout,
  buf: u32,
}
fn lookup_Misc2Layout__super(b: BoundLayout_Misc2Layout) -> BoundLayout_FinalizeMiscLayout {
  return BoundLayout_FinalizeMiscLayout(b.lyt._super, b.buf);
}
fn lookup_Misc2Layout__0(b: BoundLayout_Misc2Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Misc2Layout_input(b: BoundLayout_Misc2Layout) -> BoundLayout_MiscInputLayout {
  return BoundLayout_MiscInputLayout(b.lyt.input, b.buf);
}
fn lookup_Misc2Layout__arguments_Misc2MiscOutput(b: BoundLayout_Misc2Layout) -> BoundLayout__Arguments_Misc2MiscOutputLayout {
  return BoundLayout__Arguments_Misc2MiscOutputLayout(b.lyt._arguments_Misc2MiscOutput, b.buf);
}
fn lookup_Misc2Layout_miscOutput(b: BoundLayout_Misc2Layout) -> BoundLayout_Misc2MiscOutputLayout {
  return BoundLayout_Misc2MiscOutputLayout(b.lyt.miscOutput, b.buf);
}
struct MulInputLayout {
  decoded: DecodeInstLayout,
  sourceRegs: ReadSourceRegsLayout,
}
struct BoundLayout_MulInputLayout {
  lyt: MulInputLayout,
  buf: u32,
}
fn lookup_MulInputLayout_decoded(b: BoundLayout_MulInputLayout) -> BoundLayout_DecodeInstLayout {
  return BoundLayout_DecodeInstLayout(b.lyt.decoded, b.buf);
}
fn lookup_MulInputLayout_sourceRegs(b: BoundLayout_MulInputLayout) -> BoundLayout_ReadSourceRegsLayout {
  return BoundLayout_ReadSourceRegsLayout(b.lyt.sourceRegs, b.buf);
}
alias ArgU16Layout6LayoutArray = array<ArgU16Layout, 6>;
struct BoundLayout_ArgU16Layout6LayoutArray {
  lyt: ArgU16Layout6LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout6LayoutArray(b: BoundLayout_ArgU16Layout6LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
struct ArgU8Layout {
  count: NondetRegLayout,
  val: NondetRegLayout,
}
struct BoundLayout_ArgU8Layout {
  lyt: ArgU8Layout,
  buf: u32,
}
fn lookup_ArgU8Layout_count(b: BoundLayout_ArgU8Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.count, b.buf);
}
fn lookup_ArgU8Layout_val(b: BoundLayout_ArgU8Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.val, b.buf);
}
alias ArgU8Layout13LayoutArray = array<ArgU8Layout, 13>;
struct BoundLayout_ArgU8Layout13LayoutArray {
  lyt: ArgU8Layout13LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout13LayoutArray(b: BoundLayout_ArgU8Layout13LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
struct _Arguments_Mul0MulOutputLayout {
  argU16: ArgU16Layout6LayoutArray,
  argU8: ArgU8Layout13LayoutArray,
}
struct BoundLayout__Arguments_Mul0MulOutputLayout {
  lyt: _Arguments_Mul0MulOutputLayout,
  buf: u32,
}
fn lookup__Arguments_Mul0MulOutputLayout_argU16(b: BoundLayout__Arguments_Mul0MulOutputLayout) -> BoundLayout_ArgU16Layout6LayoutArray {
  return BoundLayout_ArgU16Layout6LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_Mul0MulOutputLayout_argU8(b: BoundLayout__Arguments_Mul0MulOutputLayout) -> BoundLayout_ArgU8Layout13LayoutArray {
  return BoundLayout_ArgU8Layout13LayoutArray(b.lyt.argU8, b.buf);
}
alias NondetRegLayout5LayoutArray = array<NondetRegLayout, 5>;
struct BoundLayout_NondetRegLayout5LayoutArray {
  lyt: NondetRegLayout5LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout5LayoutArray(b: BoundLayout_NondetRegLayout5LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct ToBits_5_Layout {
  _super: NondetRegLayout5LayoutArray,
}
struct BoundLayout_ToBits_5_Layout {
  lyt: ToBits_5_Layout,
  buf: u32,
}
fn lookup_ToBits_5_Layout__super(b: BoundLayout_ToBits_5_Layout) -> BoundLayout_NondetRegLayout5LayoutArray {
  return BoundLayout_NondetRegLayout5LayoutArray(b.lyt._super, b.buf);
}
struct DynPo2Layout {
  low5: ToBits_5_Layout,
  checkU16: NondetU16RegLayout,
  b3: NondetRegLayout,
  low: NondetRegLayout,
  high: NondetRegLayout,
}
struct BoundLayout_DynPo2Layout {
  lyt: DynPo2Layout,
  buf: u32,
}
fn lookup_DynPo2Layout_low5(b: BoundLayout_DynPo2Layout) -> BoundLayout_ToBits_5_Layout {
  return BoundLayout_ToBits_5_Layout(b.lyt.low5, b.buf);
}
fn lookup_DynPo2Layout_checkU16(b: BoundLayout_DynPo2Layout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.checkU16, b.buf);
}
fn lookup_DynPo2Layout_b3(b: BoundLayout_DynPo2Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.b3, b.buf);
}
fn lookup_DynPo2Layout_low(b: BoundLayout_DynPo2Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low, b.buf);
}
fn lookup_DynPo2Layout_high(b: BoundLayout_DynPo2Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.high, b.buf);
}
struct NondetU8RegLayout {
  arg: ArgU8Layout,
}
struct BoundLayout_NondetU8RegLayout {
  lyt: NondetU8RegLayout,
  buf: u32,
}
fn lookup_NondetU8RegLayout_arg(b: BoundLayout_NondetU8RegLayout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt.arg, b.buf);
}
struct ExpandU32Layout {
  b0: NondetU8RegLayout,
  b1: NondetU8RegLayout,
  b2: NondetU8RegLayout,
  b3: NondetU8RegLayout,
  b3Top7times2: NondetU8RegLayout,
  topBit: NondetRegLayout,
}
struct BoundLayout_ExpandU32Layout {
  lyt: ExpandU32Layout,
  buf: u32,
}
fn lookup_ExpandU32Layout_b0(b: BoundLayout_ExpandU32Layout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.b0, b.buf);
}
fn lookup_ExpandU32Layout_b1(b: BoundLayout_ExpandU32Layout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.b1, b.buf);
}
fn lookup_ExpandU32Layout_b2(b: BoundLayout_ExpandU32Layout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.b2, b.buf);
}
fn lookup_ExpandU32Layout_b3(b: BoundLayout_ExpandU32Layout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.b3, b.buf);
}
fn lookup_ExpandU32Layout_b3Top7times2(b: BoundLayout_ExpandU32Layout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.b3Top7times2, b.buf);
}
fn lookup_ExpandU32Layout_topBit(b: BoundLayout_ExpandU32Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.topBit, b.buf);
}
struct NondetFakeTwitRegLayout {
  reg0: NondetRegLayout,
  reg1: NondetRegLayout,
}
struct BoundLayout_NondetFakeTwitRegLayout {
  lyt: NondetFakeTwitRegLayout,
  buf: u32,
}
fn lookup_NondetFakeTwitRegLayout_reg0(b: BoundLayout_NondetFakeTwitRegLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.reg0, b.buf);
}
fn lookup_NondetFakeTwitRegLayout_reg1(b: BoundLayout_NondetFakeTwitRegLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.reg1, b.buf);
}
struct SplitTotalLayout {
  out_: NondetU16RegLayout,
  carryByte: NondetU8RegLayout,
  carryExtra: NondetFakeTwitRegLayout,
}
struct BoundLayout_SplitTotalLayout {
  lyt: SplitTotalLayout,
  buf: u32,
}
fn lookup_SplitTotalLayout_out_(b: BoundLayout_SplitTotalLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.out_, b.buf);
}
fn lookup_SplitTotalLayout_carryByte(b: BoundLayout_SplitTotalLayout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.carryByte, b.buf);
}
fn lookup_SplitTotalLayout_carryExtra(b: BoundLayout_SplitTotalLayout) -> BoundLayout_NondetFakeTwitRegLayout {
  return BoundLayout_NondetFakeTwitRegLayout(b.lyt.carryExtra, b.buf);
}
struct MultiplyAccumulateLayout {
  ax: ExpandU32Layout,
  bx: ExpandU32Layout,
  cSign: NondetRegLayout,
  cRestTimes2: NondetU16RegLayout,
  s0: SplitTotalLayout,
  s1: SplitTotalLayout,
  s2: SplitTotalLayout,
  s3Out: NondetU16RegLayout,
  s3Carry: NondetFakeTwitRegLayout,
}
struct BoundLayout_MultiplyAccumulateLayout {
  lyt: MultiplyAccumulateLayout,
  buf: u32,
}
fn lookup_MultiplyAccumulateLayout_ax(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_ExpandU32Layout {
  return BoundLayout_ExpandU32Layout(b.lyt.ax, b.buf);
}
fn lookup_MultiplyAccumulateLayout_bx(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_ExpandU32Layout {
  return BoundLayout_ExpandU32Layout(b.lyt.bx, b.buf);
}
fn lookup_MultiplyAccumulateLayout_cSign(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.cSign, b.buf);
}
fn lookup_MultiplyAccumulateLayout_cRestTimes2(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.cRestTimes2, b.buf);
}
fn lookup_MultiplyAccumulateLayout_s0(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_SplitTotalLayout {
  return BoundLayout_SplitTotalLayout(b.lyt.s0, b.buf);
}
fn lookup_MultiplyAccumulateLayout_s1(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_SplitTotalLayout {
  return BoundLayout_SplitTotalLayout(b.lyt.s1, b.buf);
}
fn lookup_MultiplyAccumulateLayout_s2(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_SplitTotalLayout {
  return BoundLayout_SplitTotalLayout(b.lyt.s2, b.buf);
}
fn lookup_MultiplyAccumulateLayout_s3Out(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.s3Out, b.buf);
}
fn lookup_MultiplyAccumulateLayout_s3Carry(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_NondetFakeTwitRegLayout {
  return BoundLayout_NondetFakeTwitRegLayout(b.lyt.s3Carry, b.buf);
}
struct DoMulLayout {
  mul: MultiplyAccumulateLayout,
}
struct BoundLayout_DoMulLayout {
  lyt: DoMulLayout,
  buf: u32,
}
fn lookup_DoMulLayout_mul(b: BoundLayout_DoMulLayout) -> BoundLayout_MultiplyAccumulateLayout {
  return BoundLayout_MultiplyAccumulateLayout(b.lyt.mul, b.buf);
}
struct OpSLLLayout {
  shiftMul: DynPo2Layout,
  _0: DoMulLayout,
}
struct BoundLayout_OpSLLLayout {
  lyt: OpSLLLayout,
  buf: u32,
}
fn lookup_OpSLLLayout_shiftMul(b: BoundLayout_OpSLLLayout) -> BoundLayout_DynPo2Layout {
  return BoundLayout_DynPo2Layout(b.lyt.shiftMul, b.buf);
}
fn lookup_OpSLLLayout__0(b: BoundLayout_OpSLLLayout) -> BoundLayout_DoMulLayout {
  return BoundLayout_DoMulLayout(b.lyt._0, b.buf);
}
struct OpSLLILayout {
  shiftMul: DynPo2Layout,
  _0: DoMulLayout,
}
struct BoundLayout_OpSLLILayout {
  lyt: OpSLLILayout,
  buf: u32,
}
fn lookup_OpSLLILayout_shiftMul(b: BoundLayout_OpSLLILayout) -> BoundLayout_DynPo2Layout {
  return BoundLayout_DynPo2Layout(b.lyt.shiftMul, b.buf);
}
fn lookup_OpSLLILayout__0(b: BoundLayout_OpSLLILayout) -> BoundLayout_DoMulLayout {
  return BoundLayout_DoMulLayout(b.lyt._0, b.buf);
}
struct OpMULLayout {
  _0: DoMulLayout,
}
struct BoundLayout_OpMULLayout {
  lyt: OpMULLayout,
  buf: u32,
}
fn lookup_OpMULLayout__0(b: BoundLayout_OpMULLayout) -> BoundLayout_DoMulLayout {
  return BoundLayout_DoMulLayout(b.lyt._0, b.buf);
}
struct Mul0MulOutputArm2Layout {
  _super: OpMULLayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Mul0MulOutputArm2Layout {
  lyt: Mul0MulOutputArm2Layout,
  buf: u32,
}
fn lookup_Mul0MulOutputArm2Layout__super(b: BoundLayout_Mul0MulOutputArm2Layout) -> BoundLayout_OpMULLayout {
  return BoundLayout_OpMULLayout(b.lyt._super, b.buf);
}
fn lookup_Mul0MulOutputArm2Layout__extra0(b: BoundLayout_Mul0MulOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct OpMULHLayout {
  _0: DoMulLayout,
}
struct BoundLayout_OpMULHLayout {
  lyt: OpMULHLayout,
  buf: u32,
}
fn lookup_OpMULHLayout__0(b: BoundLayout_OpMULHLayout) -> BoundLayout_DoMulLayout {
  return BoundLayout_DoMulLayout(b.lyt._0, b.buf);
}
struct Mul0MulOutputArm3Layout {
  _super: OpMULHLayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Mul0MulOutputArm3Layout {
  lyt: Mul0MulOutputArm3Layout,
  buf: u32,
}
fn lookup_Mul0MulOutputArm3Layout__super(b: BoundLayout_Mul0MulOutputArm3Layout) -> BoundLayout_OpMULHLayout {
  return BoundLayout_OpMULHLayout(b.lyt._super, b.buf);
}
fn lookup_Mul0MulOutputArm3Layout__extra0(b: BoundLayout_Mul0MulOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct OpMULHSULayout {
  _0: DoMulLayout,
}
struct BoundLayout_OpMULHSULayout {
  lyt: OpMULHSULayout,
  buf: u32,
}
fn lookup_OpMULHSULayout__0(b: BoundLayout_OpMULHSULayout) -> BoundLayout_DoMulLayout {
  return BoundLayout_DoMulLayout(b.lyt._0, b.buf);
}
struct Mul0MulOutputArm4Layout {
  _super: OpMULHSULayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Mul0MulOutputArm4Layout {
  lyt: Mul0MulOutputArm4Layout,
  buf: u32,
}
fn lookup_Mul0MulOutputArm4Layout__super(b: BoundLayout_Mul0MulOutputArm4Layout) -> BoundLayout_OpMULHSULayout {
  return BoundLayout_OpMULHSULayout(b.lyt._super, b.buf);
}
fn lookup_Mul0MulOutputArm4Layout__extra0(b: BoundLayout_Mul0MulOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct OpMULHULayout {
  _0: DoMulLayout,
}
struct BoundLayout_OpMULHULayout {
  lyt: OpMULHULayout,
  buf: u32,
}
fn lookup_OpMULHULayout__0(b: BoundLayout_OpMULHULayout) -> BoundLayout_DoMulLayout {
  return BoundLayout_DoMulLayout(b.lyt._0, b.buf);
}
struct Mul0MulOutputArm5Layout {
  _super: OpMULHULayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Mul0MulOutputArm5Layout {
  lyt: Mul0MulOutputArm5Layout,
  buf: u32,
}
fn lookup_Mul0MulOutputArm5Layout__super(b: BoundLayout_Mul0MulOutputArm5Layout) -> BoundLayout_OpMULHULayout {
  return BoundLayout_OpMULHULayout(b.lyt._super, b.buf);
}
fn lookup_Mul0MulOutputArm5Layout__extra0(b: BoundLayout_Mul0MulOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct Mul0MulOutputArm6Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU8Layout,
  _extra7: ArgU8Layout,
  _extra8: ArgU8Layout,
  _extra9: ArgU8Layout,
  _extra10: ArgU8Layout,
  _extra11: ArgU8Layout,
  _extra12: ArgU8Layout,
  _extra13: ArgU8Layout,
  _extra14: ArgU8Layout,
  _extra15: ArgU8Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
}
struct BoundLayout_Mul0MulOutputArm6Layout {
  lyt: Mul0MulOutputArm6Layout,
  buf: u32,
}
fn lookup_Mul0MulOutputArm6Layout__extra0(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra1(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra2(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra3(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra4(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra5(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra6(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra6, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra7(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra7, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra8(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra8, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra9(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra9, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra10(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra10, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra11(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra11, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra12(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra12, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra13(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra13, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra14(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra14, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra15(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra15, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra16(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra17(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra18(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
struct Mul0MulOutputArm7Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU8Layout,
  _extra7: ArgU8Layout,
  _extra8: ArgU8Layout,
  _extra9: ArgU8Layout,
  _extra10: ArgU8Layout,
  _extra11: ArgU8Layout,
  _extra12: ArgU8Layout,
  _extra13: ArgU8Layout,
  _extra14: ArgU8Layout,
  _extra15: ArgU8Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
}
struct BoundLayout_Mul0MulOutputArm7Layout {
  lyt: Mul0MulOutputArm7Layout,
  buf: u32,
}
fn lookup_Mul0MulOutputArm7Layout__extra0(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra1(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra2(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra3(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra4(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra5(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra6(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra6, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra7(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra7, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra8(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra8, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra9(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra9, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra10(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra10, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra11(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra11, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra12(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra12, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra13(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra13, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra14(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra14, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra15(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra15, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra16(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra17(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra18(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
struct Mul0MulOutputLayout {
  arm0: OpSLLLayout,
  arm1: OpSLLILayout,
  arm2: Mul0MulOutputArm2Layout,
  arm3: Mul0MulOutputArm3Layout,
  arm4: Mul0MulOutputArm4Layout,
  arm5: Mul0MulOutputArm5Layout,
  arm6: Mul0MulOutputArm6Layout,
  arm7: Mul0MulOutputArm7Layout,
}
struct BoundLayout_Mul0MulOutputLayout {
  lyt: Mul0MulOutputLayout,
  buf: u32,
}
fn lookup_Mul0MulOutputLayout_arm0(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_OpSLLLayout {
  return BoundLayout_OpSLLLayout(b.lyt.arm0, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm1(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_OpSLLILayout {
  return BoundLayout_OpSLLILayout(b.lyt.arm1, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm2(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_Mul0MulOutputArm2Layout {
  return BoundLayout_Mul0MulOutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm3(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_Mul0MulOutputArm3Layout {
  return BoundLayout_Mul0MulOutputArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm4(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_Mul0MulOutputArm4Layout {
  return BoundLayout_Mul0MulOutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm5(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_Mul0MulOutputArm5Layout {
  return BoundLayout_Mul0MulOutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm6(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_Mul0MulOutputArm6Layout {
  return BoundLayout_Mul0MulOutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm7(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_Mul0MulOutputArm7Layout {
  return BoundLayout_Mul0MulOutputArm7Layout(b.lyt.arm7, b.buf);
}
struct Mul0Layout {
  _0: DoCycleTableLayout,
  input: MulInputLayout,
  _arguments_Mul0MulOutput: _Arguments_Mul0MulOutputLayout,
  mulOutput: Mul0MulOutputLayout,
  _1: WriteRdLayout,
  pcAdd: NormalizeU32Layout,
}
struct BoundLayout_Mul0Layout {
  lyt: Mul0Layout,
  buf: u32,
}
fn lookup_Mul0Layout__0(b: BoundLayout_Mul0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Mul0Layout_input(b: BoundLayout_Mul0Layout) -> BoundLayout_MulInputLayout {
  return BoundLayout_MulInputLayout(b.lyt.input, b.buf);
}
fn lookup_Mul0Layout__arguments_Mul0MulOutput(b: BoundLayout_Mul0Layout) -> BoundLayout__Arguments_Mul0MulOutputLayout {
  return BoundLayout__Arguments_Mul0MulOutputLayout(b.lyt._arguments_Mul0MulOutput, b.buf);
}
fn lookup_Mul0Layout_mulOutput(b: BoundLayout_Mul0Layout) -> BoundLayout_Mul0MulOutputLayout {
  return BoundLayout_Mul0MulOutputLayout(b.lyt.mulOutput, b.buf);
}
fn lookup_Mul0Layout__1(b: BoundLayout_Mul0Layout) -> BoundLayout_WriteRdLayout {
  return BoundLayout_WriteRdLayout(b.lyt._1, b.buf);
}
fn lookup_Mul0Layout_pcAdd(b: BoundLayout_Mul0Layout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.pcAdd, b.buf);
}
struct DivInputLayout {
  decoded: DecodeInstLayout,
  sourceRegs: ReadSourceRegsLayout,
}
struct BoundLayout_DivInputLayout {
  lyt: DivInputLayout,
  buf: u32,
}
fn lookup_DivInputLayout_decoded(b: BoundLayout_DivInputLayout) -> BoundLayout_DecodeInstLayout {
  return BoundLayout_DecodeInstLayout(b.lyt.decoded, b.buf);
}
fn lookup_DivInputLayout_sourceRegs(b: BoundLayout_DivInputLayout) -> BoundLayout_ReadSourceRegsLayout {
  return BoundLayout_ReadSourceRegsLayout(b.lyt.sourceRegs, b.buf);
}
alias ArgU16Layout16LayoutArray = array<ArgU16Layout, 16>;
struct BoundLayout_ArgU16Layout16LayoutArray {
  lyt: ArgU16Layout16LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout16LayoutArray(b: BoundLayout_ArgU16Layout16LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
struct _Arguments_Div0MulOutputLayout {
  argU16: ArgU16Layout16LayoutArray,
  argU8: ArgU8Layout13LayoutArray,
}
struct BoundLayout__Arguments_Div0MulOutputLayout {
  lyt: _Arguments_Div0MulOutputLayout,
  buf: u32,
}
fn lookup__Arguments_Div0MulOutputLayout_argU16(b: BoundLayout__Arguments_Div0MulOutputLayout) -> BoundLayout_ArgU16Layout16LayoutArray {
  return BoundLayout_ArgU16Layout16LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_Div0MulOutputLayout_argU8(b: BoundLayout__Arguments_Div0MulOutputLayout) -> BoundLayout_ArgU8Layout13LayoutArray {
  return BoundLayout_ArgU8Layout13LayoutArray(b.lyt.argU8, b.buf);
}
struct DoDivLayout {
  quotLow: NondetRegLayout,
  quotHigh: NondetRegLayout,
  remLow: NondetU16RegLayout,
  remHigh: NondetU16RegLayout,
  mul: MultiplyAccumulateLayout,
  topBitType: NondetRegLayout,
  topNum: NondetRegLayout,
  _0: NondetU16RegLayout,
  denomAbs: NormalizeU32Layout,
  remNormal: NormalizeU32Layout,
  isZero: NondetRegLayout,
  signedOverflowCase: NondetRegLayout,
  lt: CmpLessThanUnsignedLayout,
}
struct BoundLayout_DoDivLayout {
  lyt: DoDivLayout,
  buf: u32,
}
fn lookup_DoDivLayout_quotLow(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.quotLow, b.buf);
}
fn lookup_DoDivLayout_quotHigh(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.quotHigh, b.buf);
}
fn lookup_DoDivLayout_remLow(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.remLow, b.buf);
}
fn lookup_DoDivLayout_remHigh(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.remHigh, b.buf);
}
fn lookup_DoDivLayout_mul(b: BoundLayout_DoDivLayout) -> BoundLayout_MultiplyAccumulateLayout {
  return BoundLayout_MultiplyAccumulateLayout(b.lyt.mul, b.buf);
}
fn lookup_DoDivLayout_topBitType(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.topBitType, b.buf);
}
fn lookup_DoDivLayout_topNum(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.topNum, b.buf);
}
fn lookup_DoDivLayout__0(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt._0, b.buf);
}
fn lookup_DoDivLayout_denomAbs(b: BoundLayout_DoDivLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.denomAbs, b.buf);
}
fn lookup_DoDivLayout_remNormal(b: BoundLayout_DoDivLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.remNormal, b.buf);
}
fn lookup_DoDivLayout_isZero(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isZero, b.buf);
}
fn lookup_DoDivLayout_signedOverflowCase(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.signedOverflowCase, b.buf);
}
fn lookup_DoDivLayout_lt(b: BoundLayout_DoDivLayout) -> BoundLayout_CmpLessThanUnsignedLayout {
  return BoundLayout_CmpLessThanUnsignedLayout(b.lyt.lt, b.buf);
}
struct OpSRLLayout {
  shiftMul: DynPo2Layout,
  _0: DoDivLayout,
}
struct BoundLayout_OpSRLLayout {
  lyt: OpSRLLayout,
  buf: u32,
}
fn lookup_OpSRLLayout_shiftMul(b: BoundLayout_OpSRLLayout) -> BoundLayout_DynPo2Layout {
  return BoundLayout_DynPo2Layout(b.lyt.shiftMul, b.buf);
}
fn lookup_OpSRLLayout__0(b: BoundLayout_OpSRLLayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct Div0MulOutputArm0Layout {
  _super: OpSRLLayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Div0MulOutputArm0Layout {
  lyt: Div0MulOutputArm0Layout,
  buf: u32,
}
fn lookup_Div0MulOutputArm0Layout__super(b: BoundLayout_Div0MulOutputArm0Layout) -> BoundLayout_OpSRLLayout {
  return BoundLayout_OpSRLLayout(b.lyt._super, b.buf);
}
fn lookup_Div0MulOutputArm0Layout__extra0(b: BoundLayout_Div0MulOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct TopBitLayout {
  _super: NondetRegLayout,
  rest: NondetU16RegLayout,
}
struct BoundLayout_TopBitLayout {
  lyt: TopBitLayout,
  buf: u32,
}
fn lookup_TopBitLayout__super(b: BoundLayout_TopBitLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._super, b.buf);
}
fn lookup_TopBitLayout_rest(b: BoundLayout_TopBitLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.rest, b.buf);
}
struct OpSRALayout {
  shiftMul: DynPo2Layout,
  flip: TopBitLayout,
  _0: DoDivLayout,
}
struct BoundLayout_OpSRALayout {
  lyt: OpSRALayout,
  buf: u32,
}
fn lookup_OpSRALayout_shiftMul(b: BoundLayout_OpSRALayout) -> BoundLayout_DynPo2Layout {
  return BoundLayout_DynPo2Layout(b.lyt.shiftMul, b.buf);
}
fn lookup_OpSRALayout_flip(b: BoundLayout_OpSRALayout) -> BoundLayout_TopBitLayout {
  return BoundLayout_TopBitLayout(b.lyt.flip, b.buf);
}
fn lookup_OpSRALayout__0(b: BoundLayout_OpSRALayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct OpSRLILayout {
  shiftMul: DynPo2Layout,
  _0: DoDivLayout,
}
struct BoundLayout_OpSRLILayout {
  lyt: OpSRLILayout,
  buf: u32,
}
fn lookup_OpSRLILayout_shiftMul(b: BoundLayout_OpSRLILayout) -> BoundLayout_DynPo2Layout {
  return BoundLayout_DynPo2Layout(b.lyt.shiftMul, b.buf);
}
fn lookup_OpSRLILayout__0(b: BoundLayout_OpSRLILayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct Div0MulOutputArm2Layout {
  _super: OpSRLILayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Div0MulOutputArm2Layout {
  lyt: Div0MulOutputArm2Layout,
  buf: u32,
}
fn lookup_Div0MulOutputArm2Layout__super(b: BoundLayout_Div0MulOutputArm2Layout) -> BoundLayout_OpSRLILayout {
  return BoundLayout_OpSRLILayout(b.lyt._super, b.buf);
}
fn lookup_Div0MulOutputArm2Layout__extra0(b: BoundLayout_Div0MulOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct OpSRAILayout {
  shiftMul: DynPo2Layout,
  flip: TopBitLayout,
  _0: DoDivLayout,
}
struct BoundLayout_OpSRAILayout {
  lyt: OpSRAILayout,
  buf: u32,
}
fn lookup_OpSRAILayout_shiftMul(b: BoundLayout_OpSRAILayout) -> BoundLayout_DynPo2Layout {
  return BoundLayout_DynPo2Layout(b.lyt.shiftMul, b.buf);
}
fn lookup_OpSRAILayout_flip(b: BoundLayout_OpSRAILayout) -> BoundLayout_TopBitLayout {
  return BoundLayout_TopBitLayout(b.lyt.flip, b.buf);
}
fn lookup_OpSRAILayout__0(b: BoundLayout_OpSRAILayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct OpDIVLayout {
  _0: DoDivLayout,
}
struct BoundLayout_OpDIVLayout {
  lyt: OpDIVLayout,
  buf: u32,
}
fn lookup_OpDIVLayout__0(b: BoundLayout_OpDIVLayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct Div0MulOutputArm4Layout {
  _super: OpDIVLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
}
struct BoundLayout_Div0MulOutputArm4Layout {
  lyt: Div0MulOutputArm4Layout,
  buf: u32,
}
fn lookup_Div0MulOutputArm4Layout__super(b: BoundLayout_Div0MulOutputArm4Layout) -> BoundLayout_OpDIVLayout {
  return BoundLayout_OpDIVLayout(b.lyt._super, b.buf);
}
fn lookup_Div0MulOutputArm4Layout__extra0(b: BoundLayout_Div0MulOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Div0MulOutputArm4Layout__extra1(b: BoundLayout_Div0MulOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
struct OpDIVULayout {
  _0: DoDivLayout,
}
struct BoundLayout_OpDIVULayout {
  lyt: OpDIVULayout,
  buf: u32,
}
fn lookup_OpDIVULayout__0(b: BoundLayout_OpDIVULayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct Div0MulOutputArm5Layout {
  _super: OpDIVULayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
}
struct BoundLayout_Div0MulOutputArm5Layout {
  lyt: Div0MulOutputArm5Layout,
  buf: u32,
}
fn lookup_Div0MulOutputArm5Layout__super(b: BoundLayout_Div0MulOutputArm5Layout) -> BoundLayout_OpDIVULayout {
  return BoundLayout_OpDIVULayout(b.lyt._super, b.buf);
}
fn lookup_Div0MulOutputArm5Layout__extra0(b: BoundLayout_Div0MulOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Div0MulOutputArm5Layout__extra1(b: BoundLayout_Div0MulOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
struct OpREMLayout {
  _0: DoDivLayout,
}
struct BoundLayout_OpREMLayout {
  lyt: OpREMLayout,
  buf: u32,
}
fn lookup_OpREMLayout__0(b: BoundLayout_OpREMLayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct Div0MulOutputArm6Layout {
  _super: OpREMLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
}
struct BoundLayout_Div0MulOutputArm6Layout {
  lyt: Div0MulOutputArm6Layout,
  buf: u32,
}
fn lookup_Div0MulOutputArm6Layout__super(b: BoundLayout_Div0MulOutputArm6Layout) -> BoundLayout_OpREMLayout {
  return BoundLayout_OpREMLayout(b.lyt._super, b.buf);
}
fn lookup_Div0MulOutputArm6Layout__extra0(b: BoundLayout_Div0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Div0MulOutputArm6Layout__extra1(b: BoundLayout_Div0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
struct OpREMULayout {
  _0: DoDivLayout,
}
struct BoundLayout_OpREMULayout {
  lyt: OpREMULayout,
  buf: u32,
}
fn lookup_OpREMULayout__0(b: BoundLayout_OpREMULayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct Div0MulOutputArm7Layout {
  _super: OpREMULayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
}
struct BoundLayout_Div0MulOutputArm7Layout {
  lyt: Div0MulOutputArm7Layout,
  buf: u32,
}
fn lookup_Div0MulOutputArm7Layout__super(b: BoundLayout_Div0MulOutputArm7Layout) -> BoundLayout_OpREMULayout {
  return BoundLayout_OpREMULayout(b.lyt._super, b.buf);
}
fn lookup_Div0MulOutputArm7Layout__extra0(b: BoundLayout_Div0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Div0MulOutputArm7Layout__extra1(b: BoundLayout_Div0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
struct Div0MulOutputLayout {
  arm0: Div0MulOutputArm0Layout,
  arm1: OpSRALayout,
  arm2: Div0MulOutputArm2Layout,
  arm3: OpSRAILayout,
  arm4: Div0MulOutputArm4Layout,
  arm5: Div0MulOutputArm5Layout,
  arm6: Div0MulOutputArm6Layout,
  arm7: Div0MulOutputArm7Layout,
}
struct BoundLayout_Div0MulOutputLayout {
  lyt: Div0MulOutputLayout,
  buf: u32,
}
fn lookup_Div0MulOutputLayout_arm0(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_Div0MulOutputArm0Layout {
  return BoundLayout_Div0MulOutputArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_Div0MulOutputLayout_arm1(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_OpSRALayout {
  return BoundLayout_OpSRALayout(b.lyt.arm1, b.buf);
}
fn lookup_Div0MulOutputLayout_arm2(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_Div0MulOutputArm2Layout {
  return BoundLayout_Div0MulOutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Div0MulOutputLayout_arm3(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_OpSRAILayout {
  return BoundLayout_OpSRAILayout(b.lyt.arm3, b.buf);
}
fn lookup_Div0MulOutputLayout_arm4(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_Div0MulOutputArm4Layout {
  return BoundLayout_Div0MulOutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Div0MulOutputLayout_arm5(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_Div0MulOutputArm5Layout {
  return BoundLayout_Div0MulOutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Div0MulOutputLayout_arm6(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_Div0MulOutputArm6Layout {
  return BoundLayout_Div0MulOutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Div0MulOutputLayout_arm7(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_Div0MulOutputArm7Layout {
  return BoundLayout_Div0MulOutputArm7Layout(b.lyt.arm7, b.buf);
}
struct Div0Layout {
  _0: DoCycleTableLayout,
  input: DivInputLayout,
  _arguments_Div0MulOutput: _Arguments_Div0MulOutputLayout,
  mulOutput: Div0MulOutputLayout,
  _1: WriteRdLayout,
  pcAdd: NormalizeU32Layout,
}
struct BoundLayout_Div0Layout {
  lyt: Div0Layout,
  buf: u32,
}
fn lookup_Div0Layout__0(b: BoundLayout_Div0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Div0Layout_input(b: BoundLayout_Div0Layout) -> BoundLayout_DivInputLayout {
  return BoundLayout_DivInputLayout(b.lyt.input, b.buf);
}
fn lookup_Div0Layout__arguments_Div0MulOutput(b: BoundLayout_Div0Layout) -> BoundLayout__Arguments_Div0MulOutputLayout {
  return BoundLayout__Arguments_Div0MulOutputLayout(b.lyt._arguments_Div0MulOutput, b.buf);
}
fn lookup_Div0Layout_mulOutput(b: BoundLayout_Div0Layout) -> BoundLayout_Div0MulOutputLayout {
  return BoundLayout_Div0MulOutputLayout(b.lyt.mulOutput, b.buf);
}
fn lookup_Div0Layout__1(b: BoundLayout_Div0Layout) -> BoundLayout_WriteRdLayout {
  return BoundLayout_WriteRdLayout(b.lyt._1, b.buf);
}
fn lookup_Div0Layout_pcAdd(b: BoundLayout_Div0Layout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.pcAdd, b.buf);
}
struct AddrDecomposeBitsLayout {
  low0: NondetRegLayout,
  low1: NondetRegLayout,
  upperDiff: NondetU16RegLayout,
  _0: IsZeroLayout,
  med14: NondetU16RegLayout,
}
struct BoundLayout_AddrDecomposeBitsLayout {
  lyt: AddrDecomposeBitsLayout,
  buf: u32,
}
fn lookup_AddrDecomposeBitsLayout_low0(b: BoundLayout_AddrDecomposeBitsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low0, b.buf);
}
fn lookup_AddrDecomposeBitsLayout_low1(b: BoundLayout_AddrDecomposeBitsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low1, b.buf);
}
fn lookup_AddrDecomposeBitsLayout_upperDiff(b: BoundLayout_AddrDecomposeBitsLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.upperDiff, b.buf);
}
fn lookup_AddrDecomposeBitsLayout__0(b: BoundLayout_AddrDecomposeBitsLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt._0, b.buf);
}
fn lookup_AddrDecomposeBitsLayout_med14(b: BoundLayout_AddrDecomposeBitsLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.med14, b.buf);
}
struct MemLoadInputLayout {
  decoded: DecodeInstLayout,
  rs1: ReadRegLayout,
  addrU32: NormalizeU32Layout,
  addr: AddrDecomposeBitsLayout,
  data: MemoryReadLayout,
}
struct BoundLayout_MemLoadInputLayout {
  lyt: MemLoadInputLayout,
  buf: u32,
}
fn lookup_MemLoadInputLayout_decoded(b: BoundLayout_MemLoadInputLayout) -> BoundLayout_DecodeInstLayout {
  return BoundLayout_DecodeInstLayout(b.lyt.decoded, b.buf);
}
fn lookup_MemLoadInputLayout_rs1(b: BoundLayout_MemLoadInputLayout) -> BoundLayout_ReadRegLayout {
  return BoundLayout_ReadRegLayout(b.lyt.rs1, b.buf);
}
fn lookup_MemLoadInputLayout_addrU32(b: BoundLayout_MemLoadInputLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.addrU32, b.buf);
}
fn lookup_MemLoadInputLayout_addr(b: BoundLayout_MemLoadInputLayout) -> BoundLayout_AddrDecomposeBitsLayout {
  return BoundLayout_AddrDecomposeBitsLayout(b.lyt.addr, b.buf);
}
fn lookup_MemLoadInputLayout_data(b: BoundLayout_MemLoadInputLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.data, b.buf);
}
alias ArgU8Layout3LayoutArray = array<ArgU8Layout, 3>;
struct BoundLayout_ArgU8Layout3LayoutArray {
  lyt: ArgU8Layout3LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout3LayoutArray(b: BoundLayout_ArgU8Layout3LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
alias ArgU16Layout1LayoutArray = array<ArgU16Layout, 1>;
struct BoundLayout_ArgU16Layout1LayoutArray {
  lyt: ArgU16Layout1LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout1LayoutArray(b: BoundLayout_ArgU16Layout1LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
struct _Arguments_Mem0OutputLayout {
  argU8: ArgU8Layout3LayoutArray,
  argU16: ArgU16Layout1LayoutArray,
}
struct BoundLayout__Arguments_Mem0OutputLayout {
  lyt: _Arguments_Mem0OutputLayout,
  buf: u32,
}
fn lookup__Arguments_Mem0OutputLayout_argU8(b: BoundLayout__Arguments_Mem0OutputLayout) -> BoundLayout_ArgU8Layout3LayoutArray {
  return BoundLayout_ArgU8Layout3LayoutArray(b.lyt.argU8, b.buf);
}
fn lookup__Arguments_Mem0OutputLayout_argU16(b: BoundLayout__Arguments_Mem0OutputLayout) -> BoundLayout_ArgU16Layout1LayoutArray {
  return BoundLayout_ArgU16Layout1LayoutArray(b.lyt.argU16, b.buf);
}
struct SplitWordLayout {
  byte0: NondetU8RegLayout,
  byte1: NondetU8RegLayout,
}
struct BoundLayout_SplitWordLayout {
  lyt: SplitWordLayout,
  buf: u32,
}
fn lookup_SplitWordLayout_byte0(b: BoundLayout_SplitWordLayout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.byte0, b.buf);
}
fn lookup_SplitWordLayout_byte1(b: BoundLayout_SplitWordLayout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.byte1, b.buf);
}
struct OpLBLayout {
  bytes: SplitWordLayout,
  highBit: NondetRegLayout,
  low7x2: NondetU8RegLayout,
}
struct BoundLayout_OpLBLayout {
  lyt: OpLBLayout,
  buf: u32,
}
fn lookup_OpLBLayout_bytes(b: BoundLayout_OpLBLayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.bytes, b.buf);
}
fn lookup_OpLBLayout_highBit(b: BoundLayout_OpLBLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.highBit, b.buf);
}
fn lookup_OpLBLayout_low7x2(b: BoundLayout_OpLBLayout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.low7x2, b.buf);
}
struct Mem0OutputArm0Layout {
  _super: OpLBLayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm0Layout {
  lyt: Mem0OutputArm0Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm0Layout__super(b: BoundLayout_Mem0OutputArm0Layout) -> BoundLayout_OpLBLayout {
  return BoundLayout_OpLBLayout(b.lyt._super, b.buf);
}
fn lookup_Mem0OutputArm0Layout__extra0(b: BoundLayout_Mem0OutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct OpLHLayout {
  highBit: NondetRegLayout,
  low15x2: NondetU16RegLayout,
}
struct BoundLayout_OpLHLayout {
  lyt: OpLHLayout,
  buf: u32,
}
fn lookup_OpLHLayout_highBit(b: BoundLayout_OpLHLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.highBit, b.buf);
}
fn lookup_OpLHLayout_low15x2(b: BoundLayout_OpLHLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.low15x2, b.buf);
}
struct Mem0OutputArm1Layout {
  _super: OpLHLayout,
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
}
struct BoundLayout_Mem0OutputArm1Layout {
  lyt: Mem0OutputArm1Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm1Layout__super(b: BoundLayout_Mem0OutputArm1Layout) -> BoundLayout_OpLHLayout {
  return BoundLayout_OpLHLayout(b.lyt._super, b.buf);
}
fn lookup_Mem0OutputArm1Layout__extra0(b: BoundLayout_Mem0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm1Layout__extra1(b: BoundLayout_Mem0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem0OutputArm1Layout__extra2(b: BoundLayout_Mem0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
struct Mem0OutputArm2Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm2Layout {
  lyt: Mem0OutputArm2Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm2Layout__extra0(b: BoundLayout_Mem0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm2Layout__extra1(b: BoundLayout_Mem0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem0OutputArm2Layout__extra2(b: BoundLayout_Mem0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem0OutputArm2Layout__extra3(b: BoundLayout_Mem0OutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
struct OpLBULayout {
  bytes: SplitWordLayout,
}
struct BoundLayout_OpLBULayout {
  lyt: OpLBULayout,
  buf: u32,
}
fn lookup_OpLBULayout_bytes(b: BoundLayout_OpLBULayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.bytes, b.buf);
}
struct Mem0OutputArm3Layout {
  _super: OpLBULayout,
  _extra0: ArgU8Layout,
  _extra1: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm3Layout {
  lyt: Mem0OutputArm3Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm3Layout__super(b: BoundLayout_Mem0OutputArm3Layout) -> BoundLayout_OpLBULayout {
  return BoundLayout_OpLBULayout(b.lyt._super, b.buf);
}
fn lookup_Mem0OutputArm3Layout__extra0(b: BoundLayout_Mem0OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm3Layout__extra1(b: BoundLayout_Mem0OutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
struct Mem0OutputArm4Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm4Layout {
  lyt: Mem0OutputArm4Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm4Layout__extra0(b: BoundLayout_Mem0OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm4Layout__extra1(b: BoundLayout_Mem0OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem0OutputArm4Layout__extra2(b: BoundLayout_Mem0OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem0OutputArm4Layout__extra3(b: BoundLayout_Mem0OutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
struct Mem0OutputArm5Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm5Layout {
  lyt: Mem0OutputArm5Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm5Layout__extra0(b: BoundLayout_Mem0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm5Layout__extra1(b: BoundLayout_Mem0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem0OutputArm5Layout__extra2(b: BoundLayout_Mem0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem0OutputArm5Layout__extra3(b: BoundLayout_Mem0OutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
struct Mem0OutputArm6Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm6Layout {
  lyt: Mem0OutputArm6Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm6Layout__extra0(b: BoundLayout_Mem0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm6Layout__extra1(b: BoundLayout_Mem0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem0OutputArm6Layout__extra2(b: BoundLayout_Mem0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem0OutputArm6Layout__extra3(b: BoundLayout_Mem0OutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
struct Mem0OutputArm7Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm7Layout {
  lyt: Mem0OutputArm7Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm7Layout__extra0(b: BoundLayout_Mem0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm7Layout__extra1(b: BoundLayout_Mem0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem0OutputArm7Layout__extra2(b: BoundLayout_Mem0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem0OutputArm7Layout__extra3(b: BoundLayout_Mem0OutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
struct Mem0OutputLayout {
  arm0: Mem0OutputArm0Layout,
  arm1: Mem0OutputArm1Layout,
  arm2: Mem0OutputArm2Layout,
  arm3: Mem0OutputArm3Layout,
  arm4: Mem0OutputArm4Layout,
  arm5: Mem0OutputArm5Layout,
  arm6: Mem0OutputArm6Layout,
  arm7: Mem0OutputArm7Layout,
}
struct BoundLayout_Mem0OutputLayout {
  lyt: Mem0OutputLayout,
  buf: u32,
}
fn lookup_Mem0OutputLayout_arm0(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm0Layout {
  return BoundLayout_Mem0OutputArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_Mem0OutputLayout_arm1(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm1Layout {
  return BoundLayout_Mem0OutputArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Mem0OutputLayout_arm2(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm2Layout {
  return BoundLayout_Mem0OutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Mem0OutputLayout_arm3(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm3Layout {
  return BoundLayout_Mem0OutputArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Mem0OutputLayout_arm4(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm4Layout {
  return BoundLayout_Mem0OutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Mem0OutputLayout_arm5(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm5Layout {
  return BoundLayout_Mem0OutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Mem0OutputLayout_arm6(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm6Layout {
  return BoundLayout_Mem0OutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Mem0OutputLayout_arm7(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm7Layout {
  return BoundLayout_Mem0OutputArm7Layout(b.lyt.arm7, b.buf);
}
struct Mem0Layout {
  _0: DoCycleTableLayout,
  input: MemLoadInputLayout,
  _arguments_Mem0Output: _Arguments_Mem0OutputLayout,
  output: Mem0OutputLayout,
  _1: WriteRdLayout,
  pcAdd: NormalizeU32Layout,
}
struct BoundLayout_Mem0Layout {
  lyt: Mem0Layout,
  buf: u32,
}
fn lookup_Mem0Layout__0(b: BoundLayout_Mem0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Mem0Layout_input(b: BoundLayout_Mem0Layout) -> BoundLayout_MemLoadInputLayout {
  return BoundLayout_MemLoadInputLayout(b.lyt.input, b.buf);
}
fn lookup_Mem0Layout__arguments_Mem0Output(b: BoundLayout_Mem0Layout) -> BoundLayout__Arguments_Mem0OutputLayout {
  return BoundLayout__Arguments_Mem0OutputLayout(b.lyt._arguments_Mem0Output, b.buf);
}
fn lookup_Mem0Layout_output(b: BoundLayout_Mem0Layout) -> BoundLayout_Mem0OutputLayout {
  return BoundLayout_Mem0OutputLayout(b.lyt.output, b.buf);
}
fn lookup_Mem0Layout__1(b: BoundLayout_Mem0Layout) -> BoundLayout_WriteRdLayout {
  return BoundLayout_WriteRdLayout(b.lyt._1, b.buf);
}
fn lookup_Mem0Layout_pcAdd(b: BoundLayout_Mem0Layout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.pcAdd, b.buf);
}
struct MemStoreInputLayout {
  decoded: DecodeInstLayout,
  sourceRegs: ReadSourceRegsLayout,
  addrU32: NormalizeU32Layout,
  addr: AddrDecomposeBitsLayout,
  data: MemoryReadLayout,
}
struct BoundLayout_MemStoreInputLayout {
  lyt: MemStoreInputLayout,
  buf: u32,
}
fn lookup_MemStoreInputLayout_decoded(b: BoundLayout_MemStoreInputLayout) -> BoundLayout_DecodeInstLayout {
  return BoundLayout_DecodeInstLayout(b.lyt.decoded, b.buf);
}
fn lookup_MemStoreInputLayout_sourceRegs(b: BoundLayout_MemStoreInputLayout) -> BoundLayout_ReadSourceRegsLayout {
  return BoundLayout_ReadSourceRegsLayout(b.lyt.sourceRegs, b.buf);
}
fn lookup_MemStoreInputLayout_addrU32(b: BoundLayout_MemStoreInputLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.addrU32, b.buf);
}
fn lookup_MemStoreInputLayout_addr(b: BoundLayout_MemStoreInputLayout) -> BoundLayout_AddrDecomposeBitsLayout {
  return BoundLayout_AddrDecomposeBitsLayout(b.lyt.addr, b.buf);
}
fn lookup_MemStoreInputLayout_data(b: BoundLayout_MemStoreInputLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.data, b.buf);
}
alias ArgU8Layout4LayoutArray = array<ArgU8Layout, 4>;
struct BoundLayout_ArgU8Layout4LayoutArray {
  lyt: ArgU8Layout4LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout4LayoutArray(b: BoundLayout_ArgU8Layout4LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
struct _Arguments_Mem1OutputLayout {
  argU8: ArgU8Layout4LayoutArray,
}
struct BoundLayout__Arguments_Mem1OutputLayout {
  lyt: _Arguments_Mem1OutputLayout,
  buf: u32,
}
fn lookup__Arguments_Mem1OutputLayout_argU8(b: BoundLayout__Arguments_Mem1OutputLayout) -> BoundLayout_ArgU8Layout4LayoutArray {
  return BoundLayout_ArgU8Layout4LayoutArray(b.lyt.argU8, b.buf);
}
struct OpSBLayout {
  origBytes: SplitWordLayout,
  newBytes: SplitWordLayout,
}
struct BoundLayout_OpSBLayout {
  lyt: OpSBLayout,
  buf: u32,
}
fn lookup_OpSBLayout_origBytes(b: BoundLayout_OpSBLayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.origBytes, b.buf);
}
fn lookup_OpSBLayout_newBytes(b: BoundLayout_OpSBLayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.newBytes, b.buf);
}
struct Mem1OutputArm1Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm1Layout {
  lyt: Mem1OutputArm1Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm1Layout__extra0(b: BoundLayout_Mem1OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm1Layout__extra1(b: BoundLayout_Mem1OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm1Layout__extra2(b: BoundLayout_Mem1OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm1Layout__extra3(b: BoundLayout_Mem1OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputArm2Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm2Layout {
  lyt: Mem1OutputArm2Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm2Layout__extra0(b: BoundLayout_Mem1OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm2Layout__extra1(b: BoundLayout_Mem1OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm2Layout__extra2(b: BoundLayout_Mem1OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm2Layout__extra3(b: BoundLayout_Mem1OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputArm3Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm3Layout {
  lyt: Mem1OutputArm3Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm3Layout__extra0(b: BoundLayout_Mem1OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm3Layout__extra1(b: BoundLayout_Mem1OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm3Layout__extra2(b: BoundLayout_Mem1OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm3Layout__extra3(b: BoundLayout_Mem1OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputArm4Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm4Layout {
  lyt: Mem1OutputArm4Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm4Layout__extra0(b: BoundLayout_Mem1OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm4Layout__extra1(b: BoundLayout_Mem1OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm4Layout__extra2(b: BoundLayout_Mem1OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm4Layout__extra3(b: BoundLayout_Mem1OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputArm5Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm5Layout {
  lyt: Mem1OutputArm5Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm5Layout__extra0(b: BoundLayout_Mem1OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm5Layout__extra1(b: BoundLayout_Mem1OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm5Layout__extra2(b: BoundLayout_Mem1OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm5Layout__extra3(b: BoundLayout_Mem1OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputArm6Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm6Layout {
  lyt: Mem1OutputArm6Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm6Layout__extra0(b: BoundLayout_Mem1OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm6Layout__extra1(b: BoundLayout_Mem1OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm6Layout__extra2(b: BoundLayout_Mem1OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm6Layout__extra3(b: BoundLayout_Mem1OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputArm7Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm7Layout {
  lyt: Mem1OutputArm7Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm7Layout__extra0(b: BoundLayout_Mem1OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm7Layout__extra1(b: BoundLayout_Mem1OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm7Layout__extra2(b: BoundLayout_Mem1OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm7Layout__extra3(b: BoundLayout_Mem1OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputLayout {
  arm0: OpSBLayout,
  arm1: Mem1OutputArm1Layout,
  arm2: Mem1OutputArm2Layout,
  arm3: Mem1OutputArm3Layout,
  arm4: Mem1OutputArm4Layout,
  arm5: Mem1OutputArm5Layout,
  arm6: Mem1OutputArm6Layout,
  arm7: Mem1OutputArm7Layout,
}
struct BoundLayout_Mem1OutputLayout {
  lyt: Mem1OutputLayout,
  buf: u32,
}
fn lookup_Mem1OutputLayout_arm0(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_OpSBLayout {
  return BoundLayout_OpSBLayout(b.lyt.arm0, b.buf);
}
fn lookup_Mem1OutputLayout_arm1(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm1Layout {
  return BoundLayout_Mem1OutputArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Mem1OutputLayout_arm2(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm2Layout {
  return BoundLayout_Mem1OutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Mem1OutputLayout_arm3(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm3Layout {
  return BoundLayout_Mem1OutputArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Mem1OutputLayout_arm4(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm4Layout {
  return BoundLayout_Mem1OutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Mem1OutputLayout_arm5(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm5Layout {
  return BoundLayout_Mem1OutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Mem1OutputLayout_arm6(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm6Layout {
  return BoundLayout_Mem1OutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Mem1OutputLayout_arm7(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm7Layout {
  return BoundLayout_Mem1OutputArm7Layout(b.lyt.arm7, b.buf);
}
struct MemStoreFinalizeLayout {
  _0: MemoryWriteLayout,
}
struct BoundLayout_MemStoreFinalizeLayout {
  lyt: MemStoreFinalizeLayout,
  buf: u32,
}
fn lookup_MemStoreFinalizeLayout__0(b: BoundLayout_MemStoreFinalizeLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
struct Mem1Layout {
  _0: DoCycleTableLayout,
  input: MemStoreInputLayout,
  _arguments_Mem1Output: _Arguments_Mem1OutputLayout,
  output: Mem1OutputLayout,
  _1: MemStoreFinalizeLayout,
  pcAdd: NormalizeU32Layout,
}
struct BoundLayout_Mem1Layout {
  lyt: Mem1Layout,
  buf: u32,
}
fn lookup_Mem1Layout__0(b: BoundLayout_Mem1Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Mem1Layout_input(b: BoundLayout_Mem1Layout) -> BoundLayout_MemStoreInputLayout {
  return BoundLayout_MemStoreInputLayout(b.lyt.input, b.buf);
}
fn lookup_Mem1Layout__arguments_Mem1Output(b: BoundLayout_Mem1Layout) -> BoundLayout__Arguments_Mem1OutputLayout {
  return BoundLayout__Arguments_Mem1OutputLayout(b.lyt._arguments_Mem1Output, b.buf);
}
fn lookup_Mem1Layout_output(b: BoundLayout_Mem1Layout) -> BoundLayout_Mem1OutputLayout {
  return BoundLayout_Mem1OutputLayout(b.lyt.output, b.buf);
}
fn lookup_Mem1Layout__1(b: BoundLayout_Mem1Layout) -> BoundLayout_MemStoreFinalizeLayout {
  return BoundLayout_MemStoreFinalizeLayout(b.lyt._1, b.buf);
}
fn lookup_Mem1Layout_pcAdd(b: BoundLayout_Mem1Layout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.pcAdd, b.buf);
}
struct MemoryPageInLayout {
  io: MemoryIOLayout,
}
struct BoundLayout_MemoryPageInLayout {
  lyt: MemoryPageInLayout,
  buf: u32,
}
fn lookup_MemoryPageInLayout_io(b: BoundLayout_MemoryPageInLayout) -> BoundLayout_MemoryIOLayout {
  return BoundLayout_MemoryIOLayout(b.lyt.io, b.buf);
}
alias MemoryPageInLayout8LayoutArray = array<MemoryPageInLayout, 8>;
struct BoundLayout_MemoryPageInLayout8LayoutArray {
  lyt: MemoryPageInLayout8LayoutArray,
  buf: u32,
}
fn subscript_MemoryPageInLayout8LayoutArray(b: BoundLayout_MemoryPageInLayout8LayoutArray, i: u32) -> BoundLayout_MemoryPageInLayout {
  return BoundLayout_MemoryPageInLayout(b.lyt[i], b.buf);
}
struct ControlLoadRootAndNonceLayout {
  mem: MemoryPageInLayout8LayoutArray,
}
struct BoundLayout_ControlLoadRootAndNonceLayout {
  lyt: ControlLoadRootAndNonceLayout,
  buf: u32,
}
fn lookup_ControlLoadRootAndNonceLayout_mem(b: BoundLayout_ControlLoadRootAndNonceLayout) -> BoundLayout_MemoryPageInLayout8LayoutArray {
  return BoundLayout_MemoryPageInLayout8LayoutArray(b.lyt.mem, b.buf);
}
struct Control0_SuperArm0Layout {
  _super: ControlLoadRootAndNonceLayout,
  _extra0: CycleArgLayout,
  _extra1: CycleArgLayout,
  _extra2: CycleArgLayout,
  _extra3: CycleArgLayout,
  _extra4: CycleArgLayout,
  _extra5: CycleArgLayout,
  _extra6: CycleArgLayout,
  _extra7: CycleArgLayout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU16Layout,
  _extra17: ArgU16Layout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU8Layout,
  _extra37: ArgU8Layout,
  _extra38: ArgU8Layout,
  _extra39: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm0Layout {
  lyt: Control0_SuperArm0Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm0Layout__super(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ControlLoadRootAndNonceLayout {
  return BoundLayout_ControlLoadRootAndNonceLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra0(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra1(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra2(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra3(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra4(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra5(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra6(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra7(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra8(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra9(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra10(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra11(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra12(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra13(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra14(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra15(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra16(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra17(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra18(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra19(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra20(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra21(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra22(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra23(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra24(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra25(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra26(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra27(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra28(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra29(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra30(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra31(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra32(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra33(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra34(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra35(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra36(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra36, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra37(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra37, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra38(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra38, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra39(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra39, b.buf);
}
struct ControlResume_SuperArm0_SuperLayout {
  pc: MemoryReadLayout,
  mode: MemoryReadLayout,
}
struct BoundLayout_ControlResume_SuperArm0_SuperLayout {
  lyt: ControlResume_SuperArm0_SuperLayout,
  buf: u32,
}
fn lookup_ControlResume_SuperArm0_SuperLayout_pc(b: BoundLayout_ControlResume_SuperArm0_SuperLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.pc, b.buf);
}
fn lookup_ControlResume_SuperArm0_SuperLayout_mode(b: BoundLayout_ControlResume_SuperArm0_SuperLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.mode, b.buf);
}
struct ControlResume_SuperArm0Layout {
  _super: ControlResume_SuperArm0_SuperLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
}
struct BoundLayout_ControlResume_SuperArm0Layout {
  lyt: ControlResume_SuperArm0Layout,
  buf: u32,
}
fn lookup_ControlResume_SuperArm0Layout__super(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_ControlResume_SuperArm0_SuperLayout {
  return BoundLayout_ControlResume_SuperArm0_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra0(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra1(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra2(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra3(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra4(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra5(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra6(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra7(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra8(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra9(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra10(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra11(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra12(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra13(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra14(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra15(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra16(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra17(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
struct ControlResume_SuperArm1_Super__0_SuperLayout {
  _0: MemoryWriteLayout,
}
struct BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout {
  lyt: ControlResume_SuperArm1_Super__0_SuperLayout,
  buf: u32,
}
fn lookup_ControlResume_SuperArm1_Super__0_SuperLayout__0(b: BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
alias ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray = array<ControlResume_SuperArm1_Super__0_SuperLayout, 8>;
struct BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray {
  lyt: ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray,
  buf: u32,
}
fn subscript_ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray(b: BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray, i: u32) -> BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout {
  return BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout(b.lyt[i], b.buf);
}
struct ControlResume_SuperArm1_SuperLayout {
  _1: ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray,
}
struct BoundLayout_ControlResume_SuperArm1_SuperLayout {
  lyt: ControlResume_SuperArm1_SuperLayout,
  buf: u32,
}
fn lookup_ControlResume_SuperArm1_SuperLayout__1(b: BoundLayout_ControlResume_SuperArm1_SuperLayout) -> BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray {
  return BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray(b.lyt._1, b.buf);
}
struct ControlResume_SuperLayout {
  arm0: ControlResume_SuperArm0Layout,
  arm1: ControlResume_SuperArm1_SuperLayout,
}
struct BoundLayout_ControlResume_SuperLayout {
  lyt: ControlResume_SuperLayout,
  buf: u32,
}
fn lookup_ControlResume_SuperLayout_arm0(b: BoundLayout_ControlResume_SuperLayout) -> BoundLayout_ControlResume_SuperArm0Layout {
  return BoundLayout_ControlResume_SuperArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_ControlResume_SuperLayout_arm1(b: BoundLayout_ControlResume_SuperLayout) -> BoundLayout_ControlResume_SuperArm1_SuperLayout {
  return BoundLayout_ControlResume_SuperArm1_SuperLayout(b.lyt.arm1, b.buf);
}
alias MemoryArgLayout16LayoutArray = array<MemoryArgLayout, 16>;
struct BoundLayout_MemoryArgLayout16LayoutArray {
  lyt: MemoryArgLayout16LayoutArray,
  buf: u32,
}
fn subscript_MemoryArgLayout16LayoutArray(b: BoundLayout_MemoryArgLayout16LayoutArray, i: u32) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt[i], b.buf);
}
alias CycleArgLayout8LayoutArray = array<CycleArgLayout, 8>;
struct BoundLayout_CycleArgLayout8LayoutArray {
  lyt: CycleArgLayout8LayoutArray,
  buf: u32,
}
fn subscript_CycleArgLayout8LayoutArray(b: BoundLayout_CycleArgLayout8LayoutArray, i: u32) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt[i], b.buf);
}
struct _Arguments_ControlResume_SuperLayout {
  memoryArg: MemoryArgLayout16LayoutArray,
  cycleArg: CycleArgLayout8LayoutArray,
}
struct BoundLayout__Arguments_ControlResume_SuperLayout {
  lyt: _Arguments_ControlResume_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_ControlResume_SuperLayout_memoryArg(b: BoundLayout__Arguments_ControlResume_SuperLayout) -> BoundLayout_MemoryArgLayout16LayoutArray {
  return BoundLayout_MemoryArgLayout16LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_ControlResume_SuperLayout_cycleArg(b: BoundLayout__Arguments_ControlResume_SuperLayout) -> BoundLayout_CycleArgLayout8LayoutArray {
  return BoundLayout_CycleArgLayout8LayoutArray(b.lyt.cycleArg, b.buf);
}
struct ControlResumeLayout {
  _super: ControlResume_SuperLayout,
  pcZero: IsZeroLayout,
  _arguments_ControlResume_Super: _Arguments_ControlResume_SuperLayout,
}
struct BoundLayout_ControlResumeLayout {
  lyt: ControlResumeLayout,
  buf: u32,
}
fn lookup_ControlResumeLayout__super(b: BoundLayout_ControlResumeLayout) -> BoundLayout_ControlResume_SuperLayout {
  return BoundLayout_ControlResume_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlResumeLayout_pcZero(b: BoundLayout_ControlResumeLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.pcZero, b.buf);
}
fn lookup_ControlResumeLayout__arguments_ControlResume_Super(b: BoundLayout_ControlResumeLayout) -> BoundLayout__Arguments_ControlResume_SuperLayout {
  return BoundLayout__Arguments_ControlResume_SuperLayout(b.lyt._arguments_ControlResume_Super, b.buf);
}
struct Control0_SuperArm1Layout {
  _super: ControlResumeLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm1Layout {
  lyt: Control0_SuperArm1Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm1Layout__super(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ControlResumeLayout {
  return BoundLayout_ControlResumeLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra0(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra1(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra2(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra3(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra4(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra5(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra6(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra7(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra8(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra9(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra10(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra11(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra12(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra13(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra14(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra15(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra16(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra17(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra18(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra19(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra20(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra21(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra22(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra23(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra24(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra25(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra26(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra27(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra28(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra29(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra30(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra31(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
struct ControlUserEcallOrFence_SuperArm0_SuperLayout {
  newPc: NormalizeU32Layout,
}
struct BoundLayout_ControlUserEcallOrFence_SuperArm0_SuperLayout {
  lyt: ControlUserEcallOrFence_SuperArm0_SuperLayout,
  buf: u32,
}
fn lookup_ControlUserEcallOrFence_SuperArm0_SuperLayout_newPc(b: BoundLayout_ControlUserEcallOrFence_SuperArm0_SuperLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.newPc, b.buf);
}
struct ControlUserEcallOrFence_SuperArm0Layout {
  _super: ControlUserEcallOrFence_SuperArm0_SuperLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: CycleArgLayout,
  _extra5: CycleArgLayout,
}
struct BoundLayout_ControlUserEcallOrFence_SuperArm0Layout {
  lyt: ControlUserEcallOrFence_SuperArm0Layout,
  buf: u32,
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__super(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_ControlUserEcallOrFence_SuperArm0_SuperLayout {
  return BoundLayout_ControlUserEcallOrFence_SuperArm0_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__extra0(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__extra1(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__extra2(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__extra3(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__extra4(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__extra5(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra5, b.buf);
}
struct ControlUserEcallOrFence_SuperArm1_SuperLayout {
  newPcAddr: MemoryReadLayout,
  _0: MemoryWriteLayout,
}
struct BoundLayout_ControlUserEcallOrFence_SuperArm1_SuperLayout {
  lyt: ControlUserEcallOrFence_SuperArm1_SuperLayout,
  buf: u32,
}
fn lookup_ControlUserEcallOrFence_SuperArm1_SuperLayout_newPcAddr(b: BoundLayout_ControlUserEcallOrFence_SuperArm1_SuperLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.newPcAddr, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm1_SuperLayout__0(b: BoundLayout_ControlUserEcallOrFence_SuperArm1_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
struct ControlUserEcallOrFence_SuperArm1Layout {
  _super: ControlUserEcallOrFence_SuperArm1_SuperLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
}
struct BoundLayout_ControlUserEcallOrFence_SuperArm1Layout {
  lyt: ControlUserEcallOrFence_SuperArm1Layout,
  buf: u32,
}
fn lookup_ControlUserEcallOrFence_SuperArm1Layout__super(b: BoundLayout_ControlUserEcallOrFence_SuperArm1Layout) -> BoundLayout_ControlUserEcallOrFence_SuperArm1_SuperLayout {
  return BoundLayout_ControlUserEcallOrFence_SuperArm1_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm1Layout__extra0(b: BoundLayout_ControlUserEcallOrFence_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm1Layout__extra1(b: BoundLayout_ControlUserEcallOrFence_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
struct ControlUserEcallOrFence_SuperLayout {
  arm0: ControlUserEcallOrFence_SuperArm0Layout,
  arm1: ControlUserEcallOrFence_SuperArm1Layout,
}
struct BoundLayout_ControlUserEcallOrFence_SuperLayout {
  lyt: ControlUserEcallOrFence_SuperLayout,
  buf: u32,
}
fn lookup_ControlUserEcallOrFence_SuperLayout_arm0(b: BoundLayout_ControlUserEcallOrFence_SuperLayout) -> BoundLayout_ControlUserEcallOrFence_SuperArm0Layout {
  return BoundLayout_ControlUserEcallOrFence_SuperArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperLayout_arm1(b: BoundLayout_ControlUserEcallOrFence_SuperLayout) -> BoundLayout_ControlUserEcallOrFence_SuperArm1Layout {
  return BoundLayout_ControlUserEcallOrFence_SuperArm1Layout(b.lyt.arm1, b.buf);
}
alias ArgU16Layout2LayoutArray = array<ArgU16Layout, 2>;
struct BoundLayout_ArgU16Layout2LayoutArray {
  lyt: ArgU16Layout2LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout2LayoutArray(b: BoundLayout_ArgU16Layout2LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
struct _Arguments_ControlUserEcallOrFence_SuperLayout {
  argU16: ArgU16Layout2LayoutArray,
  memoryArg: MemoryArgLayout4LayoutArray,
  cycleArg: CycleArgLayout2LayoutArray,
}
struct BoundLayout__Arguments_ControlUserEcallOrFence_SuperLayout {
  lyt: _Arguments_ControlUserEcallOrFence_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_ControlUserEcallOrFence_SuperLayout_argU16(b: BoundLayout__Arguments_ControlUserEcallOrFence_SuperLayout) -> BoundLayout_ArgU16Layout2LayoutArray {
  return BoundLayout_ArgU16Layout2LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_ControlUserEcallOrFence_SuperLayout_memoryArg(b: BoundLayout__Arguments_ControlUserEcallOrFence_SuperLayout) -> BoundLayout_MemoryArgLayout4LayoutArray {
  return BoundLayout_MemoryArgLayout4LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_ControlUserEcallOrFence_SuperLayout_cycleArg(b: BoundLayout__Arguments_ControlUserEcallOrFence_SuperLayout) -> BoundLayout_CycleArgLayout2LayoutArray {
  return BoundLayout_CycleArgLayout2LayoutArray(b.lyt.cycleArg, b.buf);
}
struct ControlUserEcallOrFenceLayout {
  _super: ControlUserEcallOrFence_SuperLayout,
  safeMode: NondetRegLayout,
  pcAddr: AddrDecomposeBitsLayout,
  loadInst: MemoryReadLayout,
  isFence: NondetRegLayout,
  _arguments_ControlUserEcallOrFence_Super: _Arguments_ControlUserEcallOrFence_SuperLayout,
}
struct BoundLayout_ControlUserEcallOrFenceLayout {
  lyt: ControlUserEcallOrFenceLayout,
  buf: u32,
}
fn lookup_ControlUserEcallOrFenceLayout__super(b: BoundLayout_ControlUserEcallOrFenceLayout) -> BoundLayout_ControlUserEcallOrFence_SuperLayout {
  return BoundLayout_ControlUserEcallOrFence_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlUserEcallOrFenceLayout_safeMode(b: BoundLayout_ControlUserEcallOrFenceLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.safeMode, b.buf);
}
fn lookup_ControlUserEcallOrFenceLayout_pcAddr(b: BoundLayout_ControlUserEcallOrFenceLayout) -> BoundLayout_AddrDecomposeBitsLayout {
  return BoundLayout_AddrDecomposeBitsLayout(b.lyt.pcAddr, b.buf);
}
fn lookup_ControlUserEcallOrFenceLayout_loadInst(b: BoundLayout_ControlUserEcallOrFenceLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.loadInst, b.buf);
}
fn lookup_ControlUserEcallOrFenceLayout_isFence(b: BoundLayout_ControlUserEcallOrFenceLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isFence, b.buf);
}
fn lookup_ControlUserEcallOrFenceLayout__arguments_ControlUserEcallOrFence_Super(b: BoundLayout_ControlUserEcallOrFenceLayout) -> BoundLayout__Arguments_ControlUserEcallOrFence_SuperLayout {
  return BoundLayout__Arguments_ControlUserEcallOrFence_SuperLayout(b.lyt._arguments_ControlUserEcallOrFence_Super, b.buf);
}
struct Control0_SuperArm2Layout {
  _super: ControlUserEcallOrFenceLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: ArgU16Layout,
  _extra16: ArgU16Layout,
  _extra17: ArgU16Layout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU8Layout,
  _extra37: ArgU8Layout,
  _extra38: ArgU8Layout,
  _extra39: ArgU8Layout,
  _extra40: ArgU8Layout,
  _extra41: ArgU8Layout,
  _extra42: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm2Layout {
  lyt: Control0_SuperArm2Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm2Layout__super(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ControlUserEcallOrFenceLayout {
  return BoundLayout_ControlUserEcallOrFenceLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra0(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra1(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra2(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra3(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra4(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra5(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra6(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra7(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra8(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra9(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra10(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra11(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra12(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra13(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra14(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra15(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra16(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra17(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra18(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra19(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra20(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra21(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra22(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra23(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra24(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra25(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra26(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra27(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra28(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra29(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra30(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra31(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra32(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra33(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra34(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra35(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra36(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra36, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra37(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra37, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra38(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra38, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra39(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra39, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra40(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra40, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra41(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra41, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra42(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra42, b.buf);
}
struct ControlMRETLayout {
  safeMode: NondetRegLayout,
  pcAddr: AddrDecomposeBitsLayout,
  loadInst: MemoryReadLayout,
  pc: MemoryReadLayout,
  pcAdd: NormalizeU32Layout,
}
struct BoundLayout_ControlMRETLayout {
  lyt: ControlMRETLayout,
  buf: u32,
}
fn lookup_ControlMRETLayout_safeMode(b: BoundLayout_ControlMRETLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.safeMode, b.buf);
}
fn lookup_ControlMRETLayout_pcAddr(b: BoundLayout_ControlMRETLayout) -> BoundLayout_AddrDecomposeBitsLayout {
  return BoundLayout_AddrDecomposeBitsLayout(b.lyt.pcAddr, b.buf);
}
fn lookup_ControlMRETLayout_loadInst(b: BoundLayout_ControlMRETLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.loadInst, b.buf);
}
fn lookup_ControlMRETLayout_pc(b: BoundLayout_ControlMRETLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.pc, b.buf);
}
fn lookup_ControlMRETLayout_pcAdd(b: BoundLayout_ControlMRETLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.pcAdd, b.buf);
}
struct Control0_SuperArm3Layout {
  _super: ControlMRETLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU16Layout,
  _extra28: ArgU16Layout,
  _extra29: ArgU16Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU8Layout,
  _extra37: ArgU8Layout,
  _extra38: ArgU8Layout,
  _extra39: ArgU8Layout,
  _extra40: ArgU8Layout,
  _extra41: ArgU8Layout,
  _extra42: ArgU8Layout,
  _extra43: ArgU8Layout,
  _extra44: ArgU8Layout,
  _extra45: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm3Layout {
  lyt: Control0_SuperArm3Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm3Layout__super(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ControlMRETLayout {
  return BoundLayout_ControlMRETLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra0(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra1(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra2(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra3(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra4(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra5(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra6(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra7(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra8(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra9(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra10(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra11(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra12(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra13(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra14(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra15(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra16(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra17(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra18(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra19(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra20(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra21(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra22(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra23(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra24(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra25(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra26(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra27(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra28(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra29(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra30(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra31(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra32(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra33(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra34(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra35(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra36(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra36, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra37(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra37, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra38(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra38, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra39(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra39, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra40(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra40, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra41(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra41, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra42(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra42, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra43(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra43, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra44(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra44, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra45(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra45, b.buf);
}
alias MemoryReadLayout8LayoutArray = array<MemoryReadLayout, 8>;
struct BoundLayout_MemoryReadLayout8LayoutArray {
  lyt: MemoryReadLayout8LayoutArray,
  buf: u32,
}
fn subscript_MemoryReadLayout8LayoutArray(b: BoundLayout_MemoryReadLayout8LayoutArray, i: u32) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt[i], b.buf);
}
struct ControlSuspend_SuperArm0_SuperLayout {
  _1: MemoryReadLayout8LayoutArray,
}
struct BoundLayout_ControlSuspend_SuperArm0_SuperLayout {
  lyt: ControlSuspend_SuperArm0_SuperLayout,
  buf: u32,
}
fn lookup_ControlSuspend_SuperArm0_SuperLayout__1(b: BoundLayout_ControlSuspend_SuperArm0_SuperLayout) -> BoundLayout_MemoryReadLayout8LayoutArray {
  return BoundLayout_MemoryReadLayout8LayoutArray(b.lyt._1, b.buf);
}
struct ControlSuspend_SuperArm1_SuperLayout {
  state: NondetRegLayout,
  _0: MemoryWriteLayout,
  _1: MemoryWriteLayout,
}
struct BoundLayout_ControlSuspend_SuperArm1_SuperLayout {
  lyt: ControlSuspend_SuperArm1_SuperLayout,
  buf: u32,
}
fn lookup_ControlSuspend_SuperArm1_SuperLayout_state(b: BoundLayout_ControlSuspend_SuperArm1_SuperLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.state, b.buf);
}
fn lookup_ControlSuspend_SuperArm1_SuperLayout__0(b: BoundLayout_ControlSuspend_SuperArm1_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
fn lookup_ControlSuspend_SuperArm1_SuperLayout__1(b: BoundLayout_ControlSuspend_SuperArm1_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._1, b.buf);
}
struct ControlSuspend_SuperArm1Layout {
  _super: ControlSuspend_SuperArm1_SuperLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
}
struct BoundLayout_ControlSuspend_SuperArm1Layout {
  lyt: ControlSuspend_SuperArm1Layout,
  buf: u32,
}
fn lookup_ControlSuspend_SuperArm1Layout__super(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_ControlSuspend_SuperArm1_SuperLayout {
  return BoundLayout_ControlSuspend_SuperArm1_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra0(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra1(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra2(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra3(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra4(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra5(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra6(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra7(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra8(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra9(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra10(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra11(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra12(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra13(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra14(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra15(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra16(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra17(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
struct ControlSuspend_SuperLayout {
  arm0: ControlSuspend_SuperArm0_SuperLayout,
  arm1: ControlSuspend_SuperArm1Layout,
}
struct BoundLayout_ControlSuspend_SuperLayout {
  lyt: ControlSuspend_SuperLayout,
  buf: u32,
}
fn lookup_ControlSuspend_SuperLayout_arm0(b: BoundLayout_ControlSuspend_SuperLayout) -> BoundLayout_ControlSuspend_SuperArm0_SuperLayout {
  return BoundLayout_ControlSuspend_SuperArm0_SuperLayout(b.lyt.arm0, b.buf);
}
fn lookup_ControlSuspend_SuperLayout_arm1(b: BoundLayout_ControlSuspend_SuperLayout) -> BoundLayout_ControlSuspend_SuperArm1Layout {
  return BoundLayout_ControlSuspend_SuperArm1Layout(b.lyt.arm1, b.buf);
}
struct _Arguments_ControlSuspend_SuperLayout {
  memoryArg: MemoryArgLayout16LayoutArray,
  cycleArg: CycleArgLayout8LayoutArray,
}
struct BoundLayout__Arguments_ControlSuspend_SuperLayout {
  lyt: _Arguments_ControlSuspend_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_ControlSuspend_SuperLayout_memoryArg(b: BoundLayout__Arguments_ControlSuspend_SuperLayout) -> BoundLayout_MemoryArgLayout16LayoutArray {
  return BoundLayout_MemoryArgLayout16LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_ControlSuspend_SuperLayout_cycleArg(b: BoundLayout__Arguments_ControlSuspend_SuperLayout) -> BoundLayout_CycleArgLayout8LayoutArray {
  return BoundLayout_CycleArgLayout8LayoutArray(b.lyt.cycleArg, b.buf);
}
struct ControlSuspendLayout {
  _super: ControlSuspend_SuperLayout,
  pcZero: IsZeroLayout,
  _arguments_ControlSuspend_Super: _Arguments_ControlSuspend_SuperLayout,
}
struct BoundLayout_ControlSuspendLayout {
  lyt: ControlSuspendLayout,
  buf: u32,
}
fn lookup_ControlSuspendLayout__super(b: BoundLayout_ControlSuspendLayout) -> BoundLayout_ControlSuspend_SuperLayout {
  return BoundLayout_ControlSuspend_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlSuspendLayout_pcZero(b: BoundLayout_ControlSuspendLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.pcZero, b.buf);
}
fn lookup_ControlSuspendLayout__arguments_ControlSuspend_Super(b: BoundLayout_ControlSuspendLayout) -> BoundLayout__Arguments_ControlSuspend_SuperLayout {
  return BoundLayout__Arguments_ControlSuspend_SuperLayout(b.lyt._arguments_ControlSuspend_Super, b.buf);
}
struct Control0_SuperArm4Layout {
  _super: ControlSuspendLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm4Layout {
  lyt: Control0_SuperArm4Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm4Layout__super(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ControlSuspendLayout {
  return BoundLayout_ControlSuspendLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra0(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra1(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra2(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra3(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra4(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra5(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra6(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra7(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra8(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra9(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra10(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra11(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra12(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra13(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra14(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra15(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra16(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra17(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra18(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra19(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra20(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra21(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra22(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra23(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra24(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra25(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra26(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra27(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra28(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra29(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra30(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra31(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
struct MemoryPageOutLayout {
  io: MemoryIOLayout,
  _0: IsForwardLayout,
}
struct BoundLayout_MemoryPageOutLayout {
  lyt: MemoryPageOutLayout,
  buf: u32,
}
fn lookup_MemoryPageOutLayout_io(b: BoundLayout_MemoryPageOutLayout) -> BoundLayout_MemoryIOLayout {
  return BoundLayout_MemoryIOLayout(b.lyt.io, b.buf);
}
fn lookup_MemoryPageOutLayout__0(b: BoundLayout_MemoryPageOutLayout) -> BoundLayout_IsForwardLayout {
  return BoundLayout_IsForwardLayout(b.lyt._0, b.buf);
}
alias MemoryPageOutLayout8LayoutArray = array<MemoryPageOutLayout, 8>;
struct BoundLayout_MemoryPageOutLayout8LayoutArray {
  lyt: MemoryPageOutLayout8LayoutArray,
  buf: u32,
}
fn subscript_MemoryPageOutLayout8LayoutArray(b: BoundLayout_MemoryPageOutLayout8LayoutArray, i: u32) -> BoundLayout_MemoryPageOutLayout {
  return BoundLayout_MemoryPageOutLayout(b.lyt[i], b.buf);
}
struct ControlStoreRootLayout {
  _1: MemoryPageOutLayout8LayoutArray,
}
struct BoundLayout_ControlStoreRootLayout {
  lyt: ControlStoreRootLayout,
  buf: u32,
}
fn lookup_ControlStoreRootLayout__1(b: BoundLayout_ControlStoreRootLayout) -> BoundLayout_MemoryPageOutLayout8LayoutArray {
  return BoundLayout_MemoryPageOutLayout8LayoutArray(b.lyt._1, b.buf);
}
struct Control0_SuperArm5Layout {
  _super: ControlStoreRootLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm5Layout {
  lyt: Control0_SuperArm5Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm5Layout__super(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ControlStoreRootLayout {
  return BoundLayout_ControlStoreRootLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra0(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra1(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra2(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra3(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra4(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra5(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra6(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra7(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra8(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra9(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra10(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra11(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra12(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra13(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra14(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra15(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra16(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra17(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra18(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra19(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra20(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra21(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra22(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra23(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra24(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra25(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra26(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra27(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra28(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra29(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra30(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra31(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
struct ControlTable_SuperArm0_Super__0_SuperLayout {
  arg: ArgU16Layout,
}
struct BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout {
  lyt: ControlTable_SuperArm0_Super__0_SuperLayout,
  buf: u32,
}
fn lookup_ControlTable_SuperArm0_Super__0_SuperLayout_arg(b: BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt.arg, b.buf);
}
alias ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray = array<ControlTable_SuperArm0_Super__0_SuperLayout, 16>;
struct BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray {
  lyt: ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray,
  buf: u32,
}
fn subscript_ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray(b: BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray, i: u32) -> BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout {
  return BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout(b.lyt[i], b.buf);
}
struct ControlTable_SuperArm0_SuperLayout {
  _1: ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray,
  done: IsZeroLayout,
}
struct BoundLayout_ControlTable_SuperArm0_SuperLayout {
  lyt: ControlTable_SuperArm0_SuperLayout,
  buf: u32,
}
fn lookup_ControlTable_SuperArm0_SuperLayout__1(b: BoundLayout_ControlTable_SuperArm0_SuperLayout) -> BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray {
  return BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray(b.lyt._1, b.buf);
}
fn lookup_ControlTable_SuperArm0_SuperLayout_done(b: BoundLayout_ControlTable_SuperArm0_SuperLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.done, b.buf);
}
struct ControlTable_SuperArm0Layout {
  _super: ControlTable_SuperArm0_SuperLayout,
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
  _extra4: ArgU8Layout,
  _extra5: ArgU8Layout,
  _extra6: ArgU8Layout,
  _extra7: ArgU8Layout,
  _extra8: ArgU8Layout,
  _extra9: ArgU8Layout,
  _extra10: ArgU8Layout,
  _extra11: ArgU8Layout,
  _extra12: ArgU8Layout,
  _extra13: ArgU8Layout,
  _extra14: ArgU8Layout,
  _extra15: ArgU8Layout,
}
struct BoundLayout_ControlTable_SuperArm0Layout {
  lyt: ControlTable_SuperArm0Layout,
  buf: u32,
}
fn lookup_ControlTable_SuperArm0Layout__super(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ControlTable_SuperArm0_SuperLayout {
  return BoundLayout_ControlTable_SuperArm0_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra0(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra1(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra2(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra3(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra4(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra4, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra5(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra5, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra6(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra6, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra7(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra7, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra8(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra8, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra9(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra9, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra10(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra10, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra11(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra11, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra12(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra12, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra13(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra13, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra14(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra14, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra15(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra15, b.buf);
}
struct ControlTable_SuperArm1_Super__0_SuperLayout {
  arg: ArgU8Layout,
}
struct BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout {
  lyt: ControlTable_SuperArm1_Super__0_SuperLayout,
  buf: u32,
}
fn lookup_ControlTable_SuperArm1_Super__0_SuperLayout_arg(b: BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt.arg, b.buf);
}
alias ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray = array<ControlTable_SuperArm1_Super__0_SuperLayout, 16>;
struct BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray {
  lyt: ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray,
  buf: u32,
}
fn subscript_ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray(b: BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray, i: u32) -> BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout {
  return BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout(b.lyt[i], b.buf);
}
struct ControlTable_SuperArm1_SuperLayout {
  _1: ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray,
  done: IsZeroLayout,
}
struct BoundLayout_ControlTable_SuperArm1_SuperLayout {
  lyt: ControlTable_SuperArm1_SuperLayout,
  buf: u32,
}
fn lookup_ControlTable_SuperArm1_SuperLayout__1(b: BoundLayout_ControlTable_SuperArm1_SuperLayout) -> BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray {
  return BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray(b.lyt._1, b.buf);
}
fn lookup_ControlTable_SuperArm1_SuperLayout_done(b: BoundLayout_ControlTable_SuperArm1_SuperLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.done, b.buf);
}
struct ControlTable_SuperArm1Layout {
  _super: ControlTable_SuperArm1_SuperLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
}
struct BoundLayout_ControlTable_SuperArm1Layout {
  lyt: ControlTable_SuperArm1Layout,
  buf: u32,
}
fn lookup_ControlTable_SuperArm1Layout__super(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ControlTable_SuperArm1_SuperLayout {
  return BoundLayout_ControlTable_SuperArm1_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra0(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra1(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra2(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra3(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra4(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra5(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra6(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra7(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra8(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra9(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra10(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra11(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra12(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra13(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra14(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra15(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
struct ControlTable_SuperLayout {
  arm0: ControlTable_SuperArm0Layout,
  arm1: ControlTable_SuperArm1Layout,
}
struct BoundLayout_ControlTable_SuperLayout {
  lyt: ControlTable_SuperLayout,
  buf: u32,
}
fn lookup_ControlTable_SuperLayout_arm0(b: BoundLayout_ControlTable_SuperLayout) -> BoundLayout_ControlTable_SuperArm0Layout {
  return BoundLayout_ControlTable_SuperArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_ControlTable_SuperLayout_arm1(b: BoundLayout_ControlTable_SuperLayout) -> BoundLayout_ControlTable_SuperArm1Layout {
  return BoundLayout_ControlTable_SuperArm1Layout(b.lyt.arm1, b.buf);
}
alias ArgU8Layout16LayoutArray = array<ArgU8Layout, 16>;
struct BoundLayout_ArgU8Layout16LayoutArray {
  lyt: ArgU8Layout16LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout16LayoutArray(b: BoundLayout_ArgU8Layout16LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
struct _Arguments_ControlTable_SuperLayout {
  argU16: ArgU16Layout16LayoutArray,
  argU8: ArgU8Layout16LayoutArray,
}
struct BoundLayout__Arguments_ControlTable_SuperLayout {
  lyt: _Arguments_ControlTable_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_ControlTable_SuperLayout_argU16(b: BoundLayout__Arguments_ControlTable_SuperLayout) -> BoundLayout_ArgU16Layout16LayoutArray {
  return BoundLayout_ArgU16Layout16LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_ControlTable_SuperLayout_argU8(b: BoundLayout__Arguments_ControlTable_SuperLayout) -> BoundLayout_ArgU8Layout16LayoutArray {
  return BoundLayout_ArgU8Layout16LayoutArray(b.lyt.argU8, b.buf);
}
struct ControlTableLayout {
  _super: ControlTable_SuperLayout,
  entry: NondetRegLayout,
  mode: NondetRegLayout,
  _arguments_ControlTable_Super: _Arguments_ControlTable_SuperLayout,
}
struct BoundLayout_ControlTableLayout {
  lyt: ControlTableLayout,
  buf: u32,
}
fn lookup_ControlTableLayout__super(b: BoundLayout_ControlTableLayout) -> BoundLayout_ControlTable_SuperLayout {
  return BoundLayout_ControlTable_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlTableLayout_entry(b: BoundLayout_ControlTableLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.entry, b.buf);
}
fn lookup_ControlTableLayout_mode(b: BoundLayout_ControlTableLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.mode, b.buf);
}
fn lookup_ControlTableLayout__arguments_ControlTable_Super(b: BoundLayout_ControlTableLayout) -> BoundLayout__Arguments_ControlTable_SuperLayout {
  return BoundLayout__Arguments_ControlTable_SuperLayout(b.lyt._arguments_ControlTable_Super, b.buf);
}
struct Control0_SuperArm6Layout {
  _super: ControlTableLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: MemoryArgLayout,
  _extra13: MemoryArgLayout,
  _extra14: MemoryArgLayout,
  _extra15: MemoryArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: CycleArgLayout,
  _extra19: CycleArgLayout,
  _extra20: CycleArgLayout,
  _extra21: CycleArgLayout,
  _extra22: CycleArgLayout,
  _extra23: CycleArgLayout,
}
struct BoundLayout_Control0_SuperArm6Layout {
  lyt: Control0_SuperArm6Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm6Layout__super(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_ControlTableLayout {
  return BoundLayout_ControlTableLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra0(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra1(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra2(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra3(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra4(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra5(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra6(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra7(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra8(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra9(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra10(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra11(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra12(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra13(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra14(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra15(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra16(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra17(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra18(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra19(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra20(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra21(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra22(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra23(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra23, b.buf);
}
alias CycleArgLayout1LayoutArray = array<CycleArgLayout, 1>;
struct BoundLayout_CycleArgLayout1LayoutArray {
  lyt: CycleArgLayout1LayoutArray,
  buf: u32,
}
fn subscript_CycleArgLayout1LayoutArray(b: BoundLayout_CycleArgLayout1LayoutArray, i: u32) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt[i], b.buf);
}
struct _Arguments_ControlDone__0Layout {
  cycleArg: CycleArgLayout1LayoutArray,
}
struct BoundLayout__Arguments_ControlDone__0Layout {
  lyt: _Arguments_ControlDone__0Layout,
  buf: u32,
}
fn lookup__Arguments_ControlDone__0Layout_cycleArg(b: BoundLayout__Arguments_ControlDone__0Layout) -> BoundLayout_CycleArgLayout1LayoutArray {
  return BoundLayout_CycleArgLayout1LayoutArray(b.lyt.cycleArg, b.buf);
}
struct ControlDone__0Arm0_SuperLayout {
  _0: IsCycleLayout,
}
struct BoundLayout_ControlDone__0Arm0_SuperLayout {
  lyt: ControlDone__0Arm0_SuperLayout,
  buf: u32,
}
fn lookup_ControlDone__0Arm0_SuperLayout__0(b: BoundLayout_ControlDone__0Arm0_SuperLayout) -> BoundLayout_IsCycleLayout {
  return BoundLayout_IsCycleLayout(b.lyt._0, b.buf);
}
struct ControlDone__0Arm1Layout {
  _extra0: CycleArgLayout,
}
struct BoundLayout_ControlDone__0Arm1Layout {
  lyt: ControlDone__0Arm1Layout,
  buf: u32,
}
fn lookup_ControlDone__0Arm1Layout__extra0(b: BoundLayout_ControlDone__0Arm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra0, b.buf);
}
struct ControlDone__0Layout {
  arm0: ControlDone__0Arm0_SuperLayout,
  arm1: ControlDone__0Arm1Layout,
}
struct BoundLayout_ControlDone__0Layout {
  lyt: ControlDone__0Layout,
  buf: u32,
}
fn lookup_ControlDone__0Layout_arm0(b: BoundLayout_ControlDone__0Layout) -> BoundLayout_ControlDone__0Arm0_SuperLayout {
  return BoundLayout_ControlDone__0Arm0_SuperLayout(b.lyt.arm0, b.buf);
}
fn lookup_ControlDone__0Layout_arm1(b: BoundLayout_ControlDone__0Layout) -> BoundLayout_ControlDone__0Arm1Layout {
  return BoundLayout_ControlDone__0Arm1Layout(b.lyt.arm1, b.buf);
}
struct ControlDoneLayout {
  _arguments_ControlDone__0: _Arguments_ControlDone__0Layout,
  _2: ControlDone__0Layout,
}
struct BoundLayout_ControlDoneLayout {
  lyt: ControlDoneLayout,
  buf: u32,
}
fn lookup_ControlDoneLayout__arguments_ControlDone__0(b: BoundLayout_ControlDoneLayout) -> BoundLayout__Arguments_ControlDone__0Layout {
  return BoundLayout__Arguments_ControlDone__0Layout(b.lyt._arguments_ControlDone__0, b.buf);
}
fn lookup_ControlDoneLayout__2(b: BoundLayout_ControlDoneLayout) -> BoundLayout_ControlDone__0Layout {
  return BoundLayout_ControlDone__0Layout(b.lyt._2, b.buf);
}
struct Control0_SuperArm7Layout {
  _super: ControlDoneLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: MemoryArgLayout,
  _extra13: MemoryArgLayout,
  _extra14: MemoryArgLayout,
  _extra15: MemoryArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: CycleArgLayout,
  _extra19: CycleArgLayout,
  _extra20: CycleArgLayout,
  _extra21: CycleArgLayout,
  _extra22: CycleArgLayout,
  _extra23: ArgU16Layout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU16Layout,
  _extra28: ArgU16Layout,
  _extra29: ArgU16Layout,
  _extra30: ArgU16Layout,
  _extra31: ArgU16Layout,
  _extra32: ArgU16Layout,
  _extra33: ArgU16Layout,
  _extra34: ArgU16Layout,
  _extra35: ArgU16Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU8Layout,
  _extra40: ArgU8Layout,
  _extra41: ArgU8Layout,
  _extra42: ArgU8Layout,
  _extra43: ArgU8Layout,
  _extra44: ArgU8Layout,
  _extra45: ArgU8Layout,
  _extra46: ArgU8Layout,
  _extra47: ArgU8Layout,
  _extra48: ArgU8Layout,
  _extra49: ArgU8Layout,
  _extra50: ArgU8Layout,
  _extra51: ArgU8Layout,
  _extra52: ArgU8Layout,
  _extra53: ArgU8Layout,
  _extra54: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm7Layout {
  lyt: Control0_SuperArm7Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm7Layout__super(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ControlDoneLayout {
  return BoundLayout_ControlDoneLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra0(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra1(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra2(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra3(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra4(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra5(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra6(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra7(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra8(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra9(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra10(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra11(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra12(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra13(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra14(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra15(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra16(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra17(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra18(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra19(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra20(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra21(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra22(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra23(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra24(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra25(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra26(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra27(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra28(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra29(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra30(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra31(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra31, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra32(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra32, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra33(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra33, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra34(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra34, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra35(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra35, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra36(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra37(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra38(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra39(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra39, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra40(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra40, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra41(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra41, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra42(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra42, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra43(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra43, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra44(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra44, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra45(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra45, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra46(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra46, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra47(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra47, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra48(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra48, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra49(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra49, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra50(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra50, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra51(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra51, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra52(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra52, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra53(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra53, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra54(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra54, b.buf);
}
struct Control0_SuperLayout {
  arm0: Control0_SuperArm0Layout,
  arm1: Control0_SuperArm1Layout,
  arm2: Control0_SuperArm2Layout,
  arm3: Control0_SuperArm3Layout,
  arm4: Control0_SuperArm4Layout,
  arm5: Control0_SuperArm5Layout,
  arm6: Control0_SuperArm6Layout,
  arm7: Control0_SuperArm7Layout,
}
struct BoundLayout_Control0_SuperLayout {
  lyt: Control0_SuperLayout,
  buf: u32,
}
fn lookup_Control0_SuperLayout_arm0(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm0Layout {
  return BoundLayout_Control0_SuperArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_Control0_SuperLayout_arm1(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm1Layout {
  return BoundLayout_Control0_SuperArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Control0_SuperLayout_arm2(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm2Layout {
  return BoundLayout_Control0_SuperArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Control0_SuperLayout_arm3(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm3Layout {
  return BoundLayout_Control0_SuperArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Control0_SuperLayout_arm4(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm4Layout {
  return BoundLayout_Control0_SuperArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Control0_SuperLayout_arm5(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm5Layout {
  return BoundLayout_Control0_SuperArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Control0_SuperLayout_arm6(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm6Layout {
  return BoundLayout_Control0_SuperArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Control0_SuperLayout_arm7(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm7Layout {
  return BoundLayout_Control0_SuperArm7Layout(b.lyt.arm7, b.buf);
}
struct _Arguments_Control0_SuperLayout {
  memoryArg: MemoryArgLayout16LayoutArray,
  cycleArg: CycleArgLayout8LayoutArray,
  argU16: ArgU16Layout16LayoutArray,
  argU8: ArgU8Layout16LayoutArray,
}
struct BoundLayout__Arguments_Control0_SuperLayout {
  lyt: _Arguments_Control0_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_Control0_SuperLayout_memoryArg(b: BoundLayout__Arguments_Control0_SuperLayout) -> BoundLayout_MemoryArgLayout16LayoutArray {
  return BoundLayout_MemoryArgLayout16LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_Control0_SuperLayout_cycleArg(b: BoundLayout__Arguments_Control0_SuperLayout) -> BoundLayout_CycleArgLayout8LayoutArray {
  return BoundLayout_CycleArgLayout8LayoutArray(b.lyt.cycleArg, b.buf);
}
fn lookup__Arguments_Control0_SuperLayout_argU16(b: BoundLayout__Arguments_Control0_SuperLayout) -> BoundLayout_ArgU16Layout16LayoutArray {
  return BoundLayout_ArgU16Layout16LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_Control0_SuperLayout_argU8(b: BoundLayout__Arguments_Control0_SuperLayout) -> BoundLayout_ArgU8Layout16LayoutArray {
  return BoundLayout_ArgU8Layout16LayoutArray(b.lyt.argU8, b.buf);
}
struct Control0Layout {
  _super: Control0_SuperLayout,
  _0: DoCycleTableLayout,
  _arguments_Control0_Super: _Arguments_Control0_SuperLayout,
}
struct BoundLayout_Control0Layout {
  lyt: Control0Layout,
  buf: u32,
}
fn lookup_Control0Layout__super(b: BoundLayout_Control0Layout) -> BoundLayout_Control0_SuperLayout {
  return BoundLayout_Control0_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_Control0Layout__0(b: BoundLayout_Control0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Control0Layout__arguments_Control0_Super(b: BoundLayout_Control0Layout) -> BoundLayout__Arguments_Control0_SuperLayout {
  return BoundLayout__Arguments_Control0_SuperLayout(b.lyt._arguments_Control0_Super, b.buf);
}
alias MemoryArgLayout8LayoutArray = array<MemoryArgLayout, 8>;
struct BoundLayout_MemoryArgLayout8LayoutArray {
  lyt: MemoryArgLayout8LayoutArray,
  buf: u32,
}
fn subscript_MemoryArgLayout8LayoutArray(b: BoundLayout_MemoryArgLayout8LayoutArray, i: u32) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt[i], b.buf);
}
alias CycleArgLayout4LayoutArray = array<CycleArgLayout, 4>;
struct BoundLayout_CycleArgLayout4LayoutArray {
  lyt: CycleArgLayout4LayoutArray,
  buf: u32,
}
fn subscript_CycleArgLayout4LayoutArray(b: BoundLayout_CycleArgLayout4LayoutArray, i: u32) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt[i], b.buf);
}
alias ArgU16Layout4LayoutArray = array<ArgU16Layout, 4>;
struct BoundLayout_ArgU16Layout4LayoutArray {
  lyt: ArgU16Layout4LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout4LayoutArray(b: BoundLayout_ArgU16Layout4LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
struct _Arguments_ECall0OutputLayout {
  memoryArg: MemoryArgLayout8LayoutArray,
  cycleArg: CycleArgLayout4LayoutArray,
  argU16: ArgU16Layout4LayoutArray,
  argU8: ArgU8Layout4LayoutArray,
}
struct BoundLayout__Arguments_ECall0OutputLayout {
  lyt: _Arguments_ECall0OutputLayout,
  buf: u32,
}
fn lookup__Arguments_ECall0OutputLayout_memoryArg(b: BoundLayout__Arguments_ECall0OutputLayout) -> BoundLayout_MemoryArgLayout8LayoutArray {
  return BoundLayout_MemoryArgLayout8LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_ECall0OutputLayout_cycleArg(b: BoundLayout__Arguments_ECall0OutputLayout) -> BoundLayout_CycleArgLayout4LayoutArray {
  return BoundLayout_CycleArgLayout4LayoutArray(b.lyt.cycleArg, b.buf);
}
fn lookup__Arguments_ECall0OutputLayout_argU16(b: BoundLayout__Arguments_ECall0OutputLayout) -> BoundLayout_ArgU16Layout4LayoutArray {
  return BoundLayout_ArgU16Layout4LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_ECall0OutputLayout_argU8(b: BoundLayout__Arguments_ECall0OutputLayout) -> BoundLayout_ArgU8Layout4LayoutArray {
  return BoundLayout_ArgU8Layout4LayoutArray(b.lyt.argU8, b.buf);
}
alias NondetRegLayout6LayoutArray = array<NondetRegLayout, 6>;
struct BoundLayout_NondetRegLayout6LayoutArray {
  lyt: NondetRegLayout6LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout6LayoutArray(b: BoundLayout_NondetRegLayout6LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct OneHot_6_Layout {
  _super: NondetRegLayout6LayoutArray,
}
struct BoundLayout_OneHot_6_Layout {
  lyt: OneHot_6_Layout,
  buf: u32,
}
fn lookup_OneHot_6_Layout__super(b: BoundLayout_OneHot_6_Layout) -> BoundLayout_NondetRegLayout6LayoutArray {
  return BoundLayout_NondetRegLayout6LayoutArray(b.lyt._super, b.buf);
}
struct MachineECallLayout {
  loadInst: MemoryReadLayout,
  dispatchIdx: MemoryReadLayout,
  dispatch: OneHot_6_Layout,
}
struct BoundLayout_MachineECallLayout {
  lyt: MachineECallLayout,
  buf: u32,
}
fn lookup_MachineECallLayout_loadInst(b: BoundLayout_MachineECallLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.loadInst, b.buf);
}
fn lookup_MachineECallLayout_dispatchIdx(b: BoundLayout_MachineECallLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.dispatchIdx, b.buf);
}
fn lookup_MachineECallLayout_dispatch(b: BoundLayout_MachineECallLayout) -> BoundLayout_OneHot_6_Layout {
  return BoundLayout_OneHot_6_Layout(b.lyt.dispatch, b.buf);
}
struct ECall0OutputArm0Layout {
  _super: MachineECallLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: CycleArgLayout,
  _extra5: CycleArgLayout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU8Layout,
  _extra11: ArgU8Layout,
  _extra12: ArgU8Layout,
  _extra13: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm0Layout {
  lyt: ECall0OutputArm0Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm0Layout__super(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_MachineECallLayout {
  return BoundLayout_MachineECallLayout(b.lyt._super, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra0(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra1(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra2(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra3(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra4(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra5(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra6(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra7(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra8(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra9(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra10(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra10, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra11(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra11, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra12(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra12, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra13(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra13, b.buf);
}
struct ECallTerminateLayout {
  a0: MemoryReadLayout,
  a1: MemoryReadLayout,
}
struct BoundLayout_ECallTerminateLayout {
  lyt: ECallTerminateLayout,
  buf: u32,
}
fn lookup_ECallTerminateLayout_a0(b: BoundLayout_ECallTerminateLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.a0, b.buf);
}
fn lookup_ECallTerminateLayout_a1(b: BoundLayout_ECallTerminateLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.a1, b.buf);
}
struct ECall0OutputArm1Layout {
  _super: ECallTerminateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: CycleArgLayout,
  _extra5: CycleArgLayout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU8Layout,
  _extra11: ArgU8Layout,
  _extra12: ArgU8Layout,
  _extra13: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm1Layout {
  lyt: ECall0OutputArm1Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm1Layout__super(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ECallTerminateLayout {
  return BoundLayout_ECallTerminateLayout(b.lyt._super, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra0(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra1(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra2(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra3(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra4(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra5(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra6(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra7(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra8(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra9(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra10(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra10, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra11(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra11, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra12(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra12, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra13(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra13, b.buf);
}
alias NondetRegLayout4LayoutArray = array<NondetRegLayout, 4>;
struct BoundLayout_NondetRegLayout4LayoutArray {
  lyt: NondetRegLayout4LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout4LayoutArray(b: BoundLayout_NondetRegLayout4LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct OneHot_4_Layout {
  _super: NondetRegLayout4LayoutArray,
}
struct BoundLayout_OneHot_4_Layout {
  lyt: OneHot_4_Layout,
  buf: u32,
}
fn lookup_OneHot_4_Layout__super(b: BoundLayout_OneHot_4_Layout) -> BoundLayout_NondetRegLayout4LayoutArray {
  return BoundLayout_NondetRegLayout4LayoutArray(b.lyt._super, b.buf);
}
struct DecomposeLow2Layout {
  high: NondetU16RegLayout,
  low2: NondetRegLayout,
  low2Hot: OneHot_4_Layout,
  highZero: IsZeroLayout,
  isZero: NondetRegLayout,
}
struct BoundLayout_DecomposeLow2Layout {
  lyt: DecomposeLow2Layout,
  buf: u32,
}
fn lookup_DecomposeLow2Layout_high(b: BoundLayout_DecomposeLow2Layout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.high, b.buf);
}
fn lookup_DecomposeLow2Layout_low2(b: BoundLayout_DecomposeLow2Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low2, b.buf);
}
fn lookup_DecomposeLow2Layout_low2Hot(b: BoundLayout_DecomposeLow2Layout) -> BoundLayout_OneHot_4_Layout {
  return BoundLayout_OneHot_4_Layout(b.lyt.low2Hot, b.buf);
}
fn lookup_DecomposeLow2Layout_highZero(b: BoundLayout_DecomposeLow2Layout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.highZero, b.buf);
}
fn lookup_DecomposeLow2Layout_isZero(b: BoundLayout_DecomposeLow2Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isZero, b.buf);
}
struct ECallHostReadSetupLayout {
  fd: MemoryReadLayout,
  ptr_: MemoryReadLayout,
  len: MemoryReadLayout,
  newLen: NondetU16RegLayout,
  diff: NondetU16RegLayout,
  _0: MemoryWriteLayout,
  ptrDecomp: DecomposeLow2Layout,
  lenDecomp: DecomposeLow2Layout,
  len123: NondetRegLayout,
  uneven: NondetRegLayout,
}
struct BoundLayout_ECallHostReadSetupLayout {
  lyt: ECallHostReadSetupLayout,
  buf: u32,
}
fn lookup_ECallHostReadSetupLayout_fd(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.fd, b.buf);
}
fn lookup_ECallHostReadSetupLayout_ptr_(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.ptr_, b.buf);
}
fn lookup_ECallHostReadSetupLayout_len(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.len, b.buf);
}
fn lookup_ECallHostReadSetupLayout_newLen(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.newLen, b.buf);
}
fn lookup_ECallHostReadSetupLayout_diff(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.diff, b.buf);
}
fn lookup_ECallHostReadSetupLayout__0(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
fn lookup_ECallHostReadSetupLayout_ptrDecomp(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_DecomposeLow2Layout {
  return BoundLayout_DecomposeLow2Layout(b.lyt.ptrDecomp, b.buf);
}
fn lookup_ECallHostReadSetupLayout_lenDecomp(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_DecomposeLow2Layout {
  return BoundLayout_DecomposeLow2Layout(b.lyt.lenDecomp, b.buf);
}
fn lookup_ECallHostReadSetupLayout_len123(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.len123, b.buf);
}
fn lookup_ECallHostReadSetupLayout_uneven(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.uneven, b.buf);
}
struct ECall0OutputArm2Layout {
  _super: ECallHostReadSetupLayout,
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm2Layout {
  lyt: ECall0OutputArm2Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm2Layout__super(b: BoundLayout_ECall0OutputArm2Layout) -> BoundLayout_ECallHostReadSetupLayout {
  return BoundLayout_ECallHostReadSetupLayout(b.lyt._super, b.buf);
}
fn lookup_ECall0OutputArm2Layout__extra0(b: BoundLayout_ECall0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm2Layout__extra1(b: BoundLayout_ECall0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm2Layout__extra2(b: BoundLayout_ECall0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm2Layout__extra3(b: BoundLayout_ECall0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct ECallHostWriteLayout {
  fd: MemoryReadLayout,
  ptr_: MemoryReadLayout,
  len: MemoryReadLayout,
  newLen: NondetU16RegLayout,
  diff: NondetU16RegLayout,
  _0: MemoryWriteLayout,
}
struct BoundLayout_ECallHostWriteLayout {
  lyt: ECallHostWriteLayout,
  buf: u32,
}
fn lookup_ECallHostWriteLayout_fd(b: BoundLayout_ECallHostWriteLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.fd, b.buf);
}
fn lookup_ECallHostWriteLayout_ptr_(b: BoundLayout_ECallHostWriteLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.ptr_, b.buf);
}
fn lookup_ECallHostWriteLayout_len(b: BoundLayout_ECallHostWriteLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.len, b.buf);
}
fn lookup_ECallHostWriteLayout_newLen(b: BoundLayout_ECallHostWriteLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.newLen, b.buf);
}
fn lookup_ECallHostWriteLayout_diff(b: BoundLayout_ECallHostWriteLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.diff, b.buf);
}
fn lookup_ECallHostWriteLayout__0(b: BoundLayout_ECallHostWriteLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
struct ECall0OutputArm3Layout {
  _super: ECallHostWriteLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
  _extra4: ArgU8Layout,
  _extra5: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm3Layout {
  lyt: ECall0OutputArm3Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm3Layout__super(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ECallHostWriteLayout {
  return BoundLayout_ECallHostWriteLayout(b.lyt._super, b.buf);
}
fn lookup_ECall0OutputArm3Layout__extra0(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm3Layout__extra1(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm3Layout__extra2(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm3Layout__extra3(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm3Layout__extra4(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm3Layout__extra5(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra5, b.buf);
}
struct MemoryWriteUnconstrainedLayout {
  io: MemoryIOLayout,
  _0: IsForwardLayout,
}
struct BoundLayout_MemoryWriteUnconstrainedLayout {
  lyt: MemoryWriteUnconstrainedLayout,
  buf: u32,
}
fn lookup_MemoryWriteUnconstrainedLayout_io(b: BoundLayout_MemoryWriteUnconstrainedLayout) -> BoundLayout_MemoryIOLayout {
  return BoundLayout_MemoryIOLayout(b.lyt.io, b.buf);
}
fn lookup_MemoryWriteUnconstrainedLayout__0(b: BoundLayout_MemoryWriteUnconstrainedLayout) -> BoundLayout_IsForwardLayout {
  return BoundLayout_IsForwardLayout(b.lyt._0, b.buf);
}
struct ECallHostReadBytesLayout {
  lenDecomp: DecomposeLow2Layout,
  len123: NondetRegLayout,
  nextPtrEven: IsZeroLayout,
  uneven: NondetRegLayout,
  lenZero: IsZeroLayout,
  low0: NondetRegLayout,
  low1: NondetRegLayout,
  origWord: MemoryReadLayout,
  _0: MemoryWriteUnconstrainedLayout,
  oldBytes: SplitWordLayout,
  newBytes_0: SplitWordLayout,
}
struct BoundLayout_ECallHostReadBytesLayout {
  lyt: ECallHostReadBytesLayout,
  buf: u32,
}
fn lookup_ECallHostReadBytesLayout_lenDecomp(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_DecomposeLow2Layout {
  return BoundLayout_DecomposeLow2Layout(b.lyt.lenDecomp, b.buf);
}
fn lookup_ECallHostReadBytesLayout_len123(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.len123, b.buf);
}
fn lookup_ECallHostReadBytesLayout_nextPtrEven(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.nextPtrEven, b.buf);
}
fn lookup_ECallHostReadBytesLayout_uneven(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.uneven, b.buf);
}
fn lookup_ECallHostReadBytesLayout_lenZero(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lenZero, b.buf);
}
fn lookup_ECallHostReadBytesLayout_low0(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low0, b.buf);
}
fn lookup_ECallHostReadBytesLayout_low1(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low1, b.buf);
}
fn lookup_ECallHostReadBytesLayout_origWord(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.origWord, b.buf);
}
fn lookup_ECallHostReadBytesLayout__0(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_MemoryWriteUnconstrainedLayout {
  return BoundLayout_MemoryWriteUnconstrainedLayout(b.lyt._0, b.buf);
}
fn lookup_ECallHostReadBytesLayout_oldBytes(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.oldBytes, b.buf);
}
fn lookup_ECallHostReadBytesLayout_newBytes_0(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.newBytes_0, b.buf);
}
struct ECall0OutputArm4Layout {
  _super: ECallHostReadBytesLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: CycleArgLayout,
  _extra5: CycleArgLayout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
}
struct BoundLayout_ECall0OutputArm4Layout {
  lyt: ECall0OutputArm4Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm4Layout__super(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_ECallHostReadBytesLayout {
  return BoundLayout_ECallHostReadBytesLayout(b.lyt._super, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra0(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra1(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra2(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra3(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra4(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra5(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra6(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra7(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra8(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
struct ECallHostReadWords__0_SuperLayout {
  addr: NondetRegLayout,
  _0: MemoryWriteUnconstrainedLayout,
}
struct BoundLayout_ECallHostReadWords__0_SuperLayout {
  lyt: ECallHostReadWords__0_SuperLayout,
  buf: u32,
}
fn lookup_ECallHostReadWords__0_SuperLayout_addr(b: BoundLayout_ECallHostReadWords__0_SuperLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.addr, b.buf);
}
fn lookup_ECallHostReadWords__0_SuperLayout__0(b: BoundLayout_ECallHostReadWords__0_SuperLayout) -> BoundLayout_MemoryWriteUnconstrainedLayout {
  return BoundLayout_MemoryWriteUnconstrainedLayout(b.lyt._0, b.buf);
}
alias ECallHostReadWords__0_SuperLayout4LayoutArray = array<ECallHostReadWords__0_SuperLayout, 4>;
struct BoundLayout_ECallHostReadWords__0_SuperLayout4LayoutArray {
  lyt: ECallHostReadWords__0_SuperLayout4LayoutArray,
  buf: u32,
}
fn subscript_ECallHostReadWords__0_SuperLayout4LayoutArray(b: BoundLayout_ECallHostReadWords__0_SuperLayout4LayoutArray, i: u32) -> BoundLayout_ECallHostReadWords__0_SuperLayout {
  return BoundLayout_ECallHostReadWords__0_SuperLayout(b.lyt[i], b.buf);
}
struct ECallHostReadWordsLayout {
  lenDecomp: DecomposeLow2Layout,
  wordsDecomp: DecomposeLow2Layout,
  _1: ECallHostReadWords__0_SuperLayout4LayoutArray,
  newLenHighZero: IsZeroLayout,
  lenZero: NondetRegLayout,
}
struct BoundLayout_ECallHostReadWordsLayout {
  lyt: ECallHostReadWordsLayout,
  buf: u32,
}
fn lookup_ECallHostReadWordsLayout_lenDecomp(b: BoundLayout_ECallHostReadWordsLayout) -> BoundLayout_DecomposeLow2Layout {
  return BoundLayout_DecomposeLow2Layout(b.lyt.lenDecomp, b.buf);
}
fn lookup_ECallHostReadWordsLayout_wordsDecomp(b: BoundLayout_ECallHostReadWordsLayout) -> BoundLayout_DecomposeLow2Layout {
  return BoundLayout_DecomposeLow2Layout(b.lyt.wordsDecomp, b.buf);
}
fn lookup_ECallHostReadWordsLayout__1(b: BoundLayout_ECallHostReadWordsLayout) -> BoundLayout_ECallHostReadWords__0_SuperLayout4LayoutArray {
  return BoundLayout_ECallHostReadWords__0_SuperLayout4LayoutArray(b.lyt._1, b.buf);
}
fn lookup_ECallHostReadWordsLayout_newLenHighZero(b: BoundLayout_ECallHostReadWordsLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.newLenHighZero, b.buf);
}
fn lookup_ECallHostReadWordsLayout_lenZero(b: BoundLayout_ECallHostReadWordsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.lenZero, b.buf);
}
struct ECall0OutputArm5Layout {
  _super: ECallHostReadWordsLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
  _extra4: ArgU8Layout,
  _extra5: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm5Layout {
  lyt: ECall0OutputArm5Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm5Layout__super(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ECallHostReadWordsLayout {
  return BoundLayout_ECallHostReadWordsLayout(b.lyt._super, b.buf);
}
fn lookup_ECall0OutputArm5Layout__extra0(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm5Layout__extra1(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm5Layout__extra2(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm5Layout__extra3(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm5Layout__extra4(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm5Layout__extra5(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra5, b.buf);
}
struct ECall0OutputArm6Layout {
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: CycleArgLayout,
  _extra9: CycleArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm6Layout {
  lyt: ECall0OutputArm6Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm6Layout__extra0(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra1(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra2(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra3(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra4(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra5(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra6(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra7(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra8(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra9(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra10(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra11(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra12(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra13(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra14(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra15(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra16(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra17(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra18(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra19(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
struct ECall0OutputArm7Layout {
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: CycleArgLayout,
  _extra9: CycleArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm7Layout {
  lyt: ECall0OutputArm7Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm7Layout__extra0(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra1(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra2(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra3(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra4(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra5(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra6(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra7(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra8(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra9(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra10(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra11(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra12(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra13(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra14(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra15(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra16(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra17(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra18(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra19(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
struct ECall0OutputLayout {
  arm0: ECall0OutputArm0Layout,
  arm1: ECall0OutputArm1Layout,
  arm2: ECall0OutputArm2Layout,
  arm3: ECall0OutputArm3Layout,
  arm4: ECall0OutputArm4Layout,
  arm5: ECall0OutputArm5Layout,
  arm6: ECall0OutputArm6Layout,
  arm7: ECall0OutputArm7Layout,
}
struct BoundLayout_ECall0OutputLayout {
  lyt: ECall0OutputLayout,
  buf: u32,
}
fn lookup_ECall0OutputLayout_arm0(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm0Layout {
  return BoundLayout_ECall0OutputArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_ECall0OutputLayout_arm1(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm1Layout {
  return BoundLayout_ECall0OutputArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_ECall0OutputLayout_arm2(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm2Layout {
  return BoundLayout_ECall0OutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_ECall0OutputLayout_arm3(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm3Layout {
  return BoundLayout_ECall0OutputArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_ECall0OutputLayout_arm4(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm4Layout {
  return BoundLayout_ECall0OutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_ECall0OutputLayout_arm5(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm5Layout {
  return BoundLayout_ECall0OutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_ECall0OutputLayout_arm6(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm6Layout {
  return BoundLayout_ECall0OutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_ECall0OutputLayout_arm7(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm7Layout {
  return BoundLayout_ECall0OutputArm7Layout(b.lyt.arm7, b.buf);
}
struct ECall0Layout {
  s0: NondetRegLayout,
  s1: NondetRegLayout,
  s2: NondetRegLayout,
  _0: DoCycleTableLayout,
  pcAddr: AddrDecomposeBitsLayout,
  _arguments_ECall0Output: _Arguments_ECall0OutputLayout,
  output: ECall0OutputLayout,
  isSuspend: IsZeroLayout,
  isDecode: IsZeroLayout,
  isP2Entry: IsZeroLayout,
  isShaEcall: IsZeroLayout,
  isBigIntEcall: IsZeroLayout,
  addPC: NormalizeU32Layout,
}
struct BoundLayout_ECall0Layout {
  lyt: ECall0Layout,
  buf: u32,
}
fn lookup_ECall0Layout_s0(b: BoundLayout_ECall0Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.s0, b.buf);
}
fn lookup_ECall0Layout_s1(b: BoundLayout_ECall0Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.s1, b.buf);
}
fn lookup_ECall0Layout_s2(b: BoundLayout_ECall0Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.s2, b.buf);
}
fn lookup_ECall0Layout__0(b: BoundLayout_ECall0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_ECall0Layout_pcAddr(b: BoundLayout_ECall0Layout) -> BoundLayout_AddrDecomposeBitsLayout {
  return BoundLayout_AddrDecomposeBitsLayout(b.lyt.pcAddr, b.buf);
}
fn lookup_ECall0Layout__arguments_ECall0Output(b: BoundLayout_ECall0Layout) -> BoundLayout__Arguments_ECall0OutputLayout {
  return BoundLayout__Arguments_ECall0OutputLayout(b.lyt._arguments_ECall0Output, b.buf);
}
fn lookup_ECall0Layout_output(b: BoundLayout_ECall0Layout) -> BoundLayout_ECall0OutputLayout {
  return BoundLayout_ECall0OutputLayout(b.lyt.output, b.buf);
}
fn lookup_ECall0Layout_isSuspend(b: BoundLayout_ECall0Layout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isSuspend, b.buf);
}
fn lookup_ECall0Layout_isDecode(b: BoundLayout_ECall0Layout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isDecode, b.buf);
}
fn lookup_ECall0Layout_isP2Entry(b: BoundLayout_ECall0Layout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isP2Entry, b.buf);
}
fn lookup_ECall0Layout_isShaEcall(b: BoundLayout_ECall0Layout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isShaEcall, b.buf);
}
fn lookup_ECall0Layout_isBigIntEcall(b: BoundLayout_ECall0Layout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isBigIntEcall, b.buf);
}
fn lookup_ECall0Layout_addPC(b: BoundLayout_ECall0Layout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.addPC, b.buf);
}
alias NondetRegLayout24LayoutArray = array<NondetRegLayout, 24>;
struct BoundLayout_NondetRegLayout24LayoutArray {
  lyt: NondetRegLayout24LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout24LayoutArray(b: BoundLayout_NondetRegLayout24LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct PoseidonStateLayout {
  hasState: NondetRegLayout,
  stateAddr: NondetRegLayout,
  bufOutAddr: NondetRegLayout,
  isElem: NondetRegLayout,
  checkOut: NondetRegLayout,
  loadTxType: NondetRegLayout,
  nextState: NondetRegLayout,
  subState: NondetRegLayout,
  bufInAddr: NondetRegLayout,
  count: NondetRegLayout,
  mode: NondetRegLayout,
  inner: NondetRegLayout24LayoutArray,
  zcheck: NondetExtRegLayout,
}
struct BoundLayout_PoseidonStateLayout {
  lyt: PoseidonStateLayout,
  buf: u32,
}
fn lookup_PoseidonStateLayout_hasState(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.hasState, b.buf);
}
fn lookup_PoseidonStateLayout_stateAddr(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.stateAddr, b.buf);
}
fn lookup_PoseidonStateLayout_bufOutAddr(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.bufOutAddr, b.buf);
}
fn lookup_PoseidonStateLayout_isElem(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isElem, b.buf);
}
fn lookup_PoseidonStateLayout_checkOut(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.checkOut, b.buf);
}
fn lookup_PoseidonStateLayout_loadTxType(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.loadTxType, b.buf);
}
fn lookup_PoseidonStateLayout_nextState(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextState, b.buf);
}
fn lookup_PoseidonStateLayout_subState(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.subState, b.buf);
}
fn lookup_PoseidonStateLayout_bufInAddr(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.bufInAddr, b.buf);
}
fn lookup_PoseidonStateLayout_count(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.count, b.buf);
}
fn lookup_PoseidonStateLayout_mode(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.mode, b.buf);
}
fn lookup_PoseidonStateLayout_inner(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout24LayoutArray {
  return BoundLayout_NondetRegLayout24LayoutArray(b.lyt.inner, b.buf);
}
fn lookup_PoseidonStateLayout_zcheck(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.zcheck, b.buf);
}
alias ArgU16Layout24LayoutArray = array<ArgU16Layout, 24>;
struct BoundLayout_ArgU16Layout24LayoutArray {
  lyt: ArgU16Layout24LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout24LayoutArray(b: BoundLayout_ArgU16Layout24LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
alias ArgU8Layout2LayoutArray = array<ArgU8Layout, 2>;
struct BoundLayout_ArgU8Layout2LayoutArray {
  lyt: ArgU8Layout2LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout2LayoutArray(b: BoundLayout_ArgU8Layout2LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
struct _Arguments_Poseidon0StateLayout {
  memoryArg: MemoryArgLayout16LayoutArray,
  cycleArg: CycleArgLayout8LayoutArray,
  argU16: ArgU16Layout24LayoutArray,
  argU8: ArgU8Layout2LayoutArray,
}
struct BoundLayout__Arguments_Poseidon0StateLayout {
  lyt: _Arguments_Poseidon0StateLayout,
  buf: u32,
}
fn lookup__Arguments_Poseidon0StateLayout_memoryArg(b: BoundLayout__Arguments_Poseidon0StateLayout) -> BoundLayout_MemoryArgLayout16LayoutArray {
  return BoundLayout_MemoryArgLayout16LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_Poseidon0StateLayout_cycleArg(b: BoundLayout__Arguments_Poseidon0StateLayout) -> BoundLayout_CycleArgLayout8LayoutArray {
  return BoundLayout_CycleArgLayout8LayoutArray(b.lyt.cycleArg, b.buf);
}
fn lookup__Arguments_Poseidon0StateLayout_argU16(b: BoundLayout__Arguments_Poseidon0StateLayout) -> BoundLayout_ArgU16Layout24LayoutArray {
  return BoundLayout_ArgU16Layout24LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_Poseidon0StateLayout_argU8(b: BoundLayout__Arguments_Poseidon0StateLayout) -> BoundLayout_ArgU8Layout2LayoutArray {
  return BoundLayout_ArgU8Layout2LayoutArray(b.lyt.argU8, b.buf);
}
struct PoseidonEntry_SuperArm0Layout {
  _super: PoseidonStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: CycleArgLayout,
  _extra9: CycleArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
}
struct BoundLayout_PoseidonEntry_SuperArm0Layout {
  lyt: PoseidonEntry_SuperArm0Layout,
  buf: u32,
}
fn lookup_PoseidonEntry_SuperArm0Layout__super(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra0(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra1(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra2(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra3(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra4(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra5(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra6(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra7(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra8(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra9(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra10(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra11(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
struct ReadAddrLayout {
  addr32: MemoryReadLayout,
}
struct BoundLayout_ReadAddrLayout {
  lyt: ReadAddrLayout,
  buf: u32,
}
fn lookup_ReadAddrLayout_addr32(b: BoundLayout_ReadAddrLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.addr32, b.buf);
}
struct PoseidonEcallLayout {
  _super: PoseidonStateLayout,
  stateAddr: ReadAddrLayout,
  bufInAddr: ReadAddrLayout,
  bufOutAddr: ReadAddrLayout,
  bitsAndCount: MemoryReadLayout,
  _0: IsZeroLayout,
  isElem: NondetRegLayout,
  checkOut: NondetRegLayout,
  countZero: IsZeroLayout,
}
struct BoundLayout_PoseidonEcallLayout {
  lyt: PoseidonEcallLayout,
  buf: u32,
}
fn lookup_PoseidonEcallLayout__super(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonEcallLayout_stateAddr(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.stateAddr, b.buf);
}
fn lookup_PoseidonEcallLayout_bufInAddr(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.bufInAddr, b.buf);
}
fn lookup_PoseidonEcallLayout_bufOutAddr(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.bufOutAddr, b.buf);
}
fn lookup_PoseidonEcallLayout_bitsAndCount(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.bitsAndCount, b.buf);
}
fn lookup_PoseidonEcallLayout__0(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt._0, b.buf);
}
fn lookup_PoseidonEcallLayout_isElem(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isElem, b.buf);
}
fn lookup_PoseidonEcallLayout_checkOut(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.checkOut, b.buf);
}
fn lookup_PoseidonEcallLayout_countZero(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.countZero, b.buf);
}
struct PoseidonEntry_SuperLayout {
  _super: PoseidonStateLayout,
  arm0: PoseidonEntry_SuperArm0Layout,
  arm1: PoseidonEcallLayout,
}
struct BoundLayout_PoseidonEntry_SuperLayout {
  lyt: PoseidonEntry_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonEntry_SuperLayout__super(b: BoundLayout_PoseidonEntry_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonEntry_SuperLayout_arm0(b: BoundLayout_PoseidonEntry_SuperLayout) -> BoundLayout_PoseidonEntry_SuperArm0Layout {
  return BoundLayout_PoseidonEntry_SuperArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_PoseidonEntry_SuperLayout_arm1(b: BoundLayout_PoseidonEntry_SuperLayout) -> BoundLayout_PoseidonEcallLayout {
  return BoundLayout_PoseidonEcallLayout(b.lyt.arm1, b.buf);
}
struct _Arguments_PoseidonEntry_SuperLayout {
  memoryArg: MemoryArgLayout8LayoutArray,
  cycleArg: CycleArgLayout4LayoutArray,
}
struct BoundLayout__Arguments_PoseidonEntry_SuperLayout {
  lyt: _Arguments_PoseidonEntry_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_PoseidonEntry_SuperLayout_memoryArg(b: BoundLayout__Arguments_PoseidonEntry_SuperLayout) -> BoundLayout_MemoryArgLayout8LayoutArray {
  return BoundLayout_MemoryArgLayout8LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_PoseidonEntry_SuperLayout_cycleArg(b: BoundLayout__Arguments_PoseidonEntry_SuperLayout) -> BoundLayout_CycleArgLayout4LayoutArray {
  return BoundLayout_CycleArgLayout4LayoutArray(b.lyt.cycleArg, b.buf);
}
struct PoseidonEntryLayout {
  _super: PoseidonEntry_SuperLayout,
  pcZero: IsZeroLayout,
  _arguments_PoseidonEntry_Super: _Arguments_PoseidonEntry_SuperLayout,
}
struct BoundLayout_PoseidonEntryLayout {
  lyt: PoseidonEntryLayout,
  buf: u32,
}
fn lookup_PoseidonEntryLayout__super(b: BoundLayout_PoseidonEntryLayout) -> BoundLayout_PoseidonEntry_SuperLayout {
  return BoundLayout_PoseidonEntry_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonEntryLayout_pcZero(b: BoundLayout_PoseidonEntryLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.pcZero, b.buf);
}
fn lookup_PoseidonEntryLayout__arguments_PoseidonEntry_Super(b: BoundLayout_PoseidonEntryLayout) -> BoundLayout__Arguments_PoseidonEntry_SuperLayout {
  return BoundLayout__Arguments_PoseidonEntry_SuperLayout(b.lyt._arguments_PoseidonEntry_Super, b.buf);
}
struct Poseidon0StateArm0Layout {
  _super: PoseidonEntryLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: CycleArgLayout,
  _extra9: CycleArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU16Layout,
  _extra17: ArgU16Layout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU16Layout,
  _extra28: ArgU16Layout,
  _extra29: ArgU16Layout,
  _extra30: ArgU16Layout,
  _extra31: ArgU16Layout,
  _extra32: ArgU16Layout,
  _extra33: ArgU16Layout,
  _extra34: ArgU16Layout,
  _extra35: ArgU16Layout,
  _extra36: ArgU8Layout,
  _extra37: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm0Layout {
  lyt: Poseidon0StateArm0Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm0Layout__super(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_PoseidonEntryLayout {
  return BoundLayout_PoseidonEntryLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra0(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra1(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra2(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra3(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra4(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra5(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra6(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra7(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra8(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra9(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra10(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra11(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra12(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra13(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra14(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra15(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra16(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra16, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra17(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra17, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra18(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra19(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra20(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra21(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra22(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra23(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra24(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra25(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra26(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra27(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra27, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra28(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra28, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra29(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra29, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra30(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra30, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra31(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra31, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra32(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra32, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra33(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra33, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra34(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra34, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra35(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra35, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra36(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra36, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra37(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra37, b.buf);
}
struct ReadElemLayout {
  elem32: MemoryReadLayout,
}
struct BoundLayout_ReadElemLayout {
  lyt: ReadElemLayout,
  buf: u32,
}
fn lookup_ReadElemLayout_elem32(b: BoundLayout_ReadElemLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.elem32, b.buf);
}
alias ReadElemLayout8LayoutArray = array<ReadElemLayout, 8>;
struct BoundLayout_ReadElemLayout8LayoutArray {
  lyt: ReadElemLayout8LayoutArray,
  buf: u32,
}
fn subscript_ReadElemLayout8LayoutArray(b: BoundLayout_ReadElemLayout8LayoutArray, i: u32) -> BoundLayout_ReadElemLayout {
  return BoundLayout_ReadElemLayout(b.lyt[i], b.buf);
}
struct PoseidonLoadStateLayout {
  _super: PoseidonStateLayout,
  loadList: ReadElemLayout8LayoutArray,
}
struct BoundLayout_PoseidonLoadStateLayout {
  lyt: PoseidonLoadStateLayout,
  buf: u32,
}
fn lookup_PoseidonLoadStateLayout__super(b: BoundLayout_PoseidonLoadStateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonLoadStateLayout_loadList(b: BoundLayout_PoseidonLoadStateLayout) -> BoundLayout_ReadElemLayout8LayoutArray {
  return BoundLayout_ReadElemLayout8LayoutArray(b.lyt.loadList, b.buf);
}
struct Poseidon0StateArm1Layout {
  _super: PoseidonLoadStateLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU16Layout,
  _extra17: ArgU16Layout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm1Layout {
  lyt: Poseidon0StateArm1Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm1Layout__super(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_PoseidonLoadStateLayout {
  return BoundLayout_PoseidonLoadStateLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra0(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra1(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra2(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra3(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra4(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra5(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra6(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra7(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra8(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra9(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra10(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra11(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra12(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra13(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra14(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra15(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra16(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra16, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra17(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra17, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra18(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra19(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra20(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra21(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra22(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra23(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra24(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra25(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
alias NondetRegLayout3LayoutArray = array<NondetRegLayout, 3>;
struct BoundLayout_NondetRegLayout3LayoutArray {
  lyt: NondetRegLayout3LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout3LayoutArray(b: BoundLayout_NondetRegLayout3LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct OneHot_3_Layout {
  _super: NondetRegLayout3LayoutArray,
}
struct BoundLayout_OneHot_3_Layout {
  lyt: OneHot_3_Layout,
  buf: u32,
}
fn lookup_OneHot_3_Layout__super(b: BoundLayout_OneHot_3_Layout) -> BoundLayout_NondetRegLayout3LayoutArray {
  return BoundLayout_NondetRegLayout3LayoutArray(b.lyt._super, b.buf);
}
struct MemoryGet_SuperArm1Layout {
  _super: MemoryPageInLayout,
  _extra0: CycleArgLayout,
}
struct BoundLayout_MemoryGet_SuperArm1Layout {
  lyt: MemoryGet_SuperArm1Layout,
  buf: u32,
}
fn lookup_MemoryGet_SuperArm1Layout__super(b: BoundLayout_MemoryGet_SuperArm1Layout) -> BoundLayout_MemoryPageInLayout {
  return BoundLayout_MemoryPageInLayout(b.lyt._super, b.buf);
}
fn lookup_MemoryGet_SuperArm1Layout__extra0(b: BoundLayout_MemoryGet_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra0, b.buf);
}
struct MemoryGet_SuperLayout {
  arm0: MemoryReadLayout,
  arm1: MemoryGet_SuperArm1Layout,
  arm2: MemoryPageOutLayout,
}
struct BoundLayout_MemoryGet_SuperLayout {
  lyt: MemoryGet_SuperLayout,
  buf: u32,
}
fn lookup_MemoryGet_SuperLayout_arm0(b: BoundLayout_MemoryGet_SuperLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.arm0, b.buf);
}
fn lookup_MemoryGet_SuperLayout_arm1(b: BoundLayout_MemoryGet_SuperLayout) -> BoundLayout_MemoryGet_SuperArm1Layout {
  return BoundLayout_MemoryGet_SuperArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_MemoryGet_SuperLayout_arm2(b: BoundLayout_MemoryGet_SuperLayout) -> BoundLayout_MemoryPageOutLayout {
  return BoundLayout_MemoryPageOutLayout(b.lyt.arm2, b.buf);
}
alias MemoryArgLayout2LayoutArray = array<MemoryArgLayout, 2>;
struct BoundLayout_MemoryArgLayout2LayoutArray {
  lyt: MemoryArgLayout2LayoutArray,
  buf: u32,
}
fn subscript_MemoryArgLayout2LayoutArray(b: BoundLayout_MemoryArgLayout2LayoutArray, i: u32) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt[i], b.buf);
}
struct _Arguments_MemoryGet_SuperLayout {
  memoryArg: MemoryArgLayout2LayoutArray,
  cycleArg: CycleArgLayout1LayoutArray,
}
struct BoundLayout__Arguments_MemoryGet_SuperLayout {
  lyt: _Arguments_MemoryGet_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_MemoryGet_SuperLayout_memoryArg(b: BoundLayout__Arguments_MemoryGet_SuperLayout) -> BoundLayout_MemoryArgLayout2LayoutArray {
  return BoundLayout_MemoryArgLayout2LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_MemoryGet_SuperLayout_cycleArg(b: BoundLayout__Arguments_MemoryGet_SuperLayout) -> BoundLayout_CycleArgLayout1LayoutArray {
  return BoundLayout_CycleArgLayout1LayoutArray(b.lyt.cycleArg, b.buf);
}
struct MemoryGetLayout {
  _super: MemoryGet_SuperLayout,
  _arguments_MemoryGet_Super: _Arguments_MemoryGet_SuperLayout,
}
struct BoundLayout_MemoryGetLayout {
  lyt: MemoryGetLayout,
  buf: u32,
}
fn lookup_MemoryGetLayout__super(b: BoundLayout_MemoryGetLayout) -> BoundLayout_MemoryGet_SuperLayout {
  return BoundLayout_MemoryGet_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_MemoryGetLayout__arguments_MemoryGet_Super(b: BoundLayout_MemoryGetLayout) -> BoundLayout__Arguments_MemoryGet_SuperLayout {
  return BoundLayout__Arguments_MemoryGet_SuperLayout(b.lyt._arguments_MemoryGet_Super, b.buf);
}
alias MemoryGetLayout8LayoutArray = array<MemoryGetLayout, 8>;
struct BoundLayout_MemoryGetLayout8LayoutArray {
  lyt: MemoryGetLayout8LayoutArray,
  buf: u32,
}
fn subscript_MemoryGetLayout8LayoutArray(b: BoundLayout_MemoryGetLayout8LayoutArray, i: u32) -> BoundLayout_MemoryGetLayout {
  return BoundLayout_MemoryGetLayout(b.lyt[i], b.buf);
}
struct PoseidonLoadInShortLayout {
  _super: PoseidonStateLayout,
  txType: OneHot_3_Layout,
  loadList: MemoryGetLayout8LayoutArray,
}
struct BoundLayout_PoseidonLoadInShortLayout {
  lyt: PoseidonLoadInShortLayout,
  buf: u32,
}
fn lookup_PoseidonLoadInShortLayout__super(b: BoundLayout_PoseidonLoadInShortLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonLoadInShortLayout_txType(b: BoundLayout_PoseidonLoadInShortLayout) -> BoundLayout_OneHot_3_Layout {
  return BoundLayout_OneHot_3_Layout(b.lyt.txType, b.buf);
}
fn lookup_PoseidonLoadInShortLayout_loadList(b: BoundLayout_PoseidonLoadInShortLayout) -> BoundLayout_MemoryGetLayout8LayoutArray {
  return BoundLayout_MemoryGetLayout8LayoutArray(b.lyt.loadList, b.buf);
}
struct PoseidonLoadInLowLayout {
  _super: PoseidonStateLayout,
  txType: OneHot_3_Layout,
  loadList: MemoryGetLayout8LayoutArray,
}
struct BoundLayout_PoseidonLoadInLowLayout {
  lyt: PoseidonLoadInLowLayout,
  buf: u32,
}
fn lookup_PoseidonLoadInLowLayout__super(b: BoundLayout_PoseidonLoadInLowLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonLoadInLowLayout_txType(b: BoundLayout_PoseidonLoadInLowLayout) -> BoundLayout_OneHot_3_Layout {
  return BoundLayout_OneHot_3_Layout(b.lyt.txType, b.buf);
}
fn lookup_PoseidonLoadInLowLayout_loadList(b: BoundLayout_PoseidonLoadInLowLayout) -> BoundLayout_MemoryGetLayout8LayoutArray {
  return BoundLayout_MemoryGetLayout8LayoutArray(b.lyt.loadList, b.buf);
}
struct PoseidonLoadInHighLayout {
  _super: PoseidonStateLayout,
  txType: OneHot_3_Layout,
  loadList: MemoryGetLayout8LayoutArray,
}
struct BoundLayout_PoseidonLoadInHighLayout {
  lyt: PoseidonLoadInHighLayout,
  buf: u32,
}
fn lookup_PoseidonLoadInHighLayout__super(b: BoundLayout_PoseidonLoadInHighLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonLoadInHighLayout_txType(b: BoundLayout_PoseidonLoadInHighLayout) -> BoundLayout_OneHot_3_Layout {
  return BoundLayout_OneHot_3_Layout(b.lyt.txType, b.buf);
}
fn lookup_PoseidonLoadInHighLayout_loadList(b: BoundLayout_PoseidonLoadInHighLayout) -> BoundLayout_MemoryGetLayout8LayoutArray {
  return BoundLayout_MemoryGetLayout8LayoutArray(b.lyt.loadList, b.buf);
}
struct PoseidonLoadIn_SuperLayout {
  _super: PoseidonStateLayout,
  arm0: PoseidonLoadInShortLayout,
  arm1: PoseidonLoadInLowLayout,
  arm2: PoseidonLoadInHighLayout,
}
struct BoundLayout_PoseidonLoadIn_SuperLayout {
  lyt: PoseidonLoadIn_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonLoadIn_SuperLayout__super(b: BoundLayout_PoseidonLoadIn_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonLoadIn_SuperLayout_arm0(b: BoundLayout_PoseidonLoadIn_SuperLayout) -> BoundLayout_PoseidonLoadInShortLayout {
  return BoundLayout_PoseidonLoadInShortLayout(b.lyt.arm0, b.buf);
}
fn lookup_PoseidonLoadIn_SuperLayout_arm1(b: BoundLayout_PoseidonLoadIn_SuperLayout) -> BoundLayout_PoseidonLoadInLowLayout {
  return BoundLayout_PoseidonLoadInLowLayout(b.lyt.arm1, b.buf);
}
fn lookup_PoseidonLoadIn_SuperLayout_arm2(b: BoundLayout_PoseidonLoadIn_SuperLayout) -> BoundLayout_PoseidonLoadInHighLayout {
  return BoundLayout_PoseidonLoadInHighLayout(b.lyt.arm2, b.buf);
}
struct _Arguments_PoseidonLoadIn_SuperLayout {
  memoryArg: MemoryArgLayout16LayoutArray,
  cycleArg: CycleArgLayout8LayoutArray,
}
struct BoundLayout__Arguments_PoseidonLoadIn_SuperLayout {
  lyt: _Arguments_PoseidonLoadIn_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_PoseidonLoadIn_SuperLayout_memoryArg(b: BoundLayout__Arguments_PoseidonLoadIn_SuperLayout) -> BoundLayout_MemoryArgLayout16LayoutArray {
  return BoundLayout_MemoryArgLayout16LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_PoseidonLoadIn_SuperLayout_cycleArg(b: BoundLayout__Arguments_PoseidonLoadIn_SuperLayout) -> BoundLayout_CycleArgLayout8LayoutArray {
  return BoundLayout_CycleArgLayout8LayoutArray(b.lyt.cycleArg, b.buf);
}
struct PoseidonLoadInLayout {
  _super: PoseidonLoadIn_SuperLayout,
  _0: OneHot_3_Layout,
  _arguments_PoseidonLoadIn_Super: _Arguments_PoseidonLoadIn_SuperLayout,
}
struct BoundLayout_PoseidonLoadInLayout {
  lyt: PoseidonLoadInLayout,
  buf: u32,
}
fn lookup_PoseidonLoadInLayout__super(b: BoundLayout_PoseidonLoadInLayout) -> BoundLayout_PoseidonLoadIn_SuperLayout {
  return BoundLayout_PoseidonLoadIn_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonLoadInLayout__0(b: BoundLayout_PoseidonLoadInLayout) -> BoundLayout_OneHot_3_Layout {
  return BoundLayout_OneHot_3_Layout(b.lyt._0, b.buf);
}
fn lookup_PoseidonLoadInLayout__arguments_PoseidonLoadIn_Super(b: BoundLayout_PoseidonLoadInLayout) -> BoundLayout__Arguments_PoseidonLoadIn_SuperLayout {
  return BoundLayout__Arguments_PoseidonLoadIn_SuperLayout(b.lyt._arguments_PoseidonLoadIn_Super, b.buf);
}
struct Poseidon0StateArm2Layout {
  _super: PoseidonLoadInLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU16Layout,
  _extra17: ArgU16Layout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm2Layout {
  lyt: Poseidon0StateArm2Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm2Layout__super(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_PoseidonLoadInLayout {
  return BoundLayout_PoseidonLoadInLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra0(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra1(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra2(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra3(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra4(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra5(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra6(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra7(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra8(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra9(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra10(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra11(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra12(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra13(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra14(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra15(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra16(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra16, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra17(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra17, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra18(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra19(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra20(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra21(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra22(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra23(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra24(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra25(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
struct Poseidon0StateArm3Layout {
  _super: PoseidonStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: MemoryArgLayout,
  _extra13: MemoryArgLayout,
  _extra14: MemoryArgLayout,
  _extra15: MemoryArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: CycleArgLayout,
  _extra19: CycleArgLayout,
  _extra20: CycleArgLayout,
  _extra21: CycleArgLayout,
  _extra22: CycleArgLayout,
  _extra23: CycleArgLayout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU16Layout,
  _extra28: ArgU16Layout,
  _extra29: ArgU16Layout,
  _extra30: ArgU16Layout,
  _extra31: ArgU16Layout,
  _extra32: ArgU16Layout,
  _extra33: ArgU16Layout,
  _extra34: ArgU16Layout,
  _extra35: ArgU16Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
  _extra40: ArgU16Layout,
  _extra41: ArgU16Layout,
  _extra42: ArgU16Layout,
  _extra43: ArgU16Layout,
  _extra44: ArgU16Layout,
  _extra45: ArgU16Layout,
  _extra46: ArgU16Layout,
  _extra47: ArgU16Layout,
  _extra48: ArgU8Layout,
  _extra49: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm3Layout {
  lyt: Poseidon0StateArm3Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm3Layout__super(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra0(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra1(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra2(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra3(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra4(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra5(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra6(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra7(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra8(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra9(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra10(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra11(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra12(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra13(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra14(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra15(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra16(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra17(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra18(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra18, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra19(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra19, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra20(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra20, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra21(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra21, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra22(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra22, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra23(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra23, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra24(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra25(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra26(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra27(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra27, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra28(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra28, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra29(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra29, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra30(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra30, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra31(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra31, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra32(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra32, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra33(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra33, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra34(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra34, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra35(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra35, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra36(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra37(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra38(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra39(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra40(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra40, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra41(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra41, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra42(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra42, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra43(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra43, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra44(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra44, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra45(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra45, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra46(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra46, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra47(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra47, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra48(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra48, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra49(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra49, b.buf);
}
struct Poseidon0StateArm4Layout {
  _super: PoseidonStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: MemoryArgLayout,
  _extra13: MemoryArgLayout,
  _extra14: MemoryArgLayout,
  _extra15: MemoryArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: CycleArgLayout,
  _extra19: CycleArgLayout,
  _extra20: CycleArgLayout,
  _extra21: CycleArgLayout,
  _extra22: CycleArgLayout,
  _extra23: CycleArgLayout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU16Layout,
  _extra28: ArgU16Layout,
  _extra29: ArgU16Layout,
  _extra30: ArgU16Layout,
  _extra31: ArgU16Layout,
  _extra32: ArgU16Layout,
  _extra33: ArgU16Layout,
  _extra34: ArgU16Layout,
  _extra35: ArgU16Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
  _extra40: ArgU16Layout,
  _extra41: ArgU16Layout,
  _extra42: ArgU16Layout,
  _extra43: ArgU16Layout,
  _extra44: ArgU16Layout,
  _extra45: ArgU16Layout,
  _extra46: ArgU16Layout,
  _extra47: ArgU16Layout,
  _extra48: ArgU8Layout,
  _extra49: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm4Layout {
  lyt: Poseidon0StateArm4Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm4Layout__super(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra0(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra1(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra2(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra3(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra4(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra5(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra6(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra7(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra8(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra9(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra10(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra11(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra12(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra13(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra14(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra15(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra16(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra17(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra18(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra18, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra19(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra19, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra20(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra20, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra21(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra21, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra22(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra22, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra23(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra23, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra24(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra25(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra26(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra27(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra27, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra28(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra28, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra29(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra29, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra30(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra30, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra31(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra31, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra32(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra32, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra33(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra33, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra34(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra34, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra35(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra35, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra36(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra37(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra38(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra39(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra40(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra40, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra41(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra41, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra42(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra42, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra43(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra43, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra44(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra44, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra45(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra45, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra46(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra46, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra47(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra47, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra48(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra48, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra49(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra49, b.buf);
}
struct PoseidonCheckOut__0_SuperLayout {
  goal: ReadElemLayout,
}
struct BoundLayout_PoseidonCheckOut__0_SuperLayout {
  lyt: PoseidonCheckOut__0_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonCheckOut__0_SuperLayout_goal(b: BoundLayout_PoseidonCheckOut__0_SuperLayout) -> BoundLayout_ReadElemLayout {
  return BoundLayout_ReadElemLayout(b.lyt.goal, b.buf);
}
alias PoseidonCheckOut__0_SuperLayout8LayoutArray = array<PoseidonCheckOut__0_SuperLayout, 8>;
struct BoundLayout_PoseidonCheckOut__0_SuperLayout8LayoutArray {
  lyt: PoseidonCheckOut__0_SuperLayout8LayoutArray,
  buf: u32,
}
fn subscript_PoseidonCheckOut__0_SuperLayout8LayoutArray(b: BoundLayout_PoseidonCheckOut__0_SuperLayout8LayoutArray, i: u32) -> BoundLayout_PoseidonCheckOut__0_SuperLayout {
  return BoundLayout_PoseidonCheckOut__0_SuperLayout(b.lyt[i], b.buf);
}
struct PoseidonCheckOutLayout {
  _super: PoseidonStateLayout,
  _1: PoseidonCheckOut__0_SuperLayout8LayoutArray,
  isNormal: IsZeroLayout,
}
struct BoundLayout_PoseidonCheckOutLayout {
  lyt: PoseidonCheckOutLayout,
  buf: u32,
}
fn lookup_PoseidonCheckOutLayout__super(b: BoundLayout_PoseidonCheckOutLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonCheckOutLayout__1(b: BoundLayout_PoseidonCheckOutLayout) -> BoundLayout_PoseidonCheckOut__0_SuperLayout8LayoutArray {
  return BoundLayout_PoseidonCheckOut__0_SuperLayout8LayoutArray(b.lyt._1, b.buf);
}
fn lookup_PoseidonCheckOutLayout_isNormal(b: BoundLayout_PoseidonCheckOutLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isNormal, b.buf);
}
struct PoseidonDoOut_SuperArm0Layout {
  _super: PoseidonCheckOutLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU16Layout,
  _extra17: ArgU16Layout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
}
struct BoundLayout_PoseidonDoOut_SuperArm0Layout {
  lyt: PoseidonDoOut_SuperArm0Layout,
  buf: u32,
}
fn lookup_PoseidonDoOut_SuperArm0Layout__super(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_PoseidonCheckOutLayout {
  return BoundLayout_PoseidonCheckOutLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra0(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra1(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra2(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra3(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra4(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra5(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra6(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra7(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra8(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra9(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra10(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra11(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra12(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra13(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra14(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra15(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra16(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra16, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra17(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra17, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra18(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra19(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra20(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra21(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra22(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra23(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
struct _Arguments_FieldToWord__0Layout {
  argU16: ArgU16Layout1LayoutArray,
}
struct BoundLayout__Arguments_FieldToWord__0Layout {
  lyt: _Arguments_FieldToWord__0Layout,
  buf: u32,
}
fn lookup__Arguments_FieldToWord__0Layout_argU16(b: BoundLayout__Arguments_FieldToWord__0Layout) -> BoundLayout_ArgU16Layout1LayoutArray {
  return BoundLayout_ArgU16Layout1LayoutArray(b.lyt.argU16, b.buf);
}
struct FieldToWord__0Arm0_SuperLayout {
  _0: NondetU16RegLayout,
}
struct BoundLayout_FieldToWord__0Arm0_SuperLayout {
  lyt: FieldToWord__0Arm0_SuperLayout,
  buf: u32,
}
fn lookup_FieldToWord__0Arm0_SuperLayout__0(b: BoundLayout_FieldToWord__0Arm0_SuperLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt._0, b.buf);
}
struct FieldToWord__0Arm1_SuperLayout {
  _0: NondetU16RegLayout,
}
struct BoundLayout_FieldToWord__0Arm1_SuperLayout {
  lyt: FieldToWord__0Arm1_SuperLayout,
  buf: u32,
}
fn lookup_FieldToWord__0Arm1_SuperLayout__0(b: BoundLayout_FieldToWord__0Arm1_SuperLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt._0, b.buf);
}
struct FieldToWord__0Layout {
  arm0: FieldToWord__0Arm0_SuperLayout,
  arm1: FieldToWord__0Arm1_SuperLayout,
}
struct BoundLayout_FieldToWord__0Layout {
  lyt: FieldToWord__0Layout,
  buf: u32,
}
fn lookup_FieldToWord__0Layout_arm0(b: BoundLayout_FieldToWord__0Layout) -> BoundLayout_FieldToWord__0Arm0_SuperLayout {
  return BoundLayout_FieldToWord__0Arm0_SuperLayout(b.lyt.arm0, b.buf);
}
fn lookup_FieldToWord__0Layout_arm1(b: BoundLayout_FieldToWord__0Layout) -> BoundLayout_FieldToWord__0Arm1_SuperLayout {
  return BoundLayout_FieldToWord__0Arm1_SuperLayout(b.lyt.arm1, b.buf);
}
struct FieldToWordLayout {
  low: NondetU16RegLayout,
  high: NondetU16RegLayout,
  lowIsZero: NondetRegLayout,
  _arguments_FieldToWord__0: _Arguments_FieldToWord__0Layout,
  _2: FieldToWord__0Layout,
}
struct BoundLayout_FieldToWordLayout {
  lyt: FieldToWordLayout,
  buf: u32,
}
fn lookup_FieldToWordLayout_low(b: BoundLayout_FieldToWordLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.low, b.buf);
}
fn lookup_FieldToWordLayout_high(b: BoundLayout_FieldToWordLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.high, b.buf);
}
fn lookup_FieldToWordLayout_lowIsZero(b: BoundLayout_FieldToWordLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.lowIsZero, b.buf);
}
fn lookup_FieldToWordLayout__arguments_FieldToWord__0(b: BoundLayout_FieldToWordLayout) -> BoundLayout__Arguments_FieldToWord__0Layout {
  return BoundLayout__Arguments_FieldToWord__0Layout(b.lyt._arguments_FieldToWord__0, b.buf);
}
fn lookup_FieldToWordLayout__2(b: BoundLayout_FieldToWordLayout) -> BoundLayout_FieldToWord__0Layout {
  return BoundLayout_FieldToWord__0Layout(b.lyt._2, b.buf);
}
struct PoseidonStoreOut__0_SuperLayout {
  ftw: FieldToWordLayout,
  mw: MemoryWriteLayout,
}
struct BoundLayout_PoseidonStoreOut__0_SuperLayout {
  lyt: PoseidonStoreOut__0_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonStoreOut__0_SuperLayout_ftw(b: BoundLayout_PoseidonStoreOut__0_SuperLayout) -> BoundLayout_FieldToWordLayout {
  return BoundLayout_FieldToWordLayout(b.lyt.ftw, b.buf);
}
fn lookup_PoseidonStoreOut__0_SuperLayout_mw(b: BoundLayout_PoseidonStoreOut__0_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt.mw, b.buf);
}
alias PoseidonStoreOut__0_SuperLayout8LayoutArray = array<PoseidonStoreOut__0_SuperLayout, 8>;
struct BoundLayout_PoseidonStoreOut__0_SuperLayout8LayoutArray {
  lyt: PoseidonStoreOut__0_SuperLayout8LayoutArray,
  buf: u32,
}
fn subscript_PoseidonStoreOut__0_SuperLayout8LayoutArray(b: BoundLayout_PoseidonStoreOut__0_SuperLayout8LayoutArray, i: u32) -> BoundLayout_PoseidonStoreOut__0_SuperLayout {
  return BoundLayout_PoseidonStoreOut__0_SuperLayout(b.lyt[i], b.buf);
}
struct PoseidonStoreOutLayout {
  _super: PoseidonStateLayout,
  _1: PoseidonStoreOut__0_SuperLayout8LayoutArray,
  isNormal: IsZeroLayout,
  extInv: NondetExtRegLayout,
}
struct BoundLayout_PoseidonStoreOutLayout {
  lyt: PoseidonStoreOutLayout,
  buf: u32,
}
fn lookup_PoseidonStoreOutLayout__super(b: BoundLayout_PoseidonStoreOutLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonStoreOutLayout__1(b: BoundLayout_PoseidonStoreOutLayout) -> BoundLayout_PoseidonStoreOut__0_SuperLayout8LayoutArray {
  return BoundLayout_PoseidonStoreOut__0_SuperLayout8LayoutArray(b.lyt._1, b.buf);
}
fn lookup_PoseidonStoreOutLayout_isNormal(b: BoundLayout_PoseidonStoreOutLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isNormal, b.buf);
}
fn lookup_PoseidonStoreOutLayout_extInv(b: BoundLayout_PoseidonStoreOutLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.extInv, b.buf);
}
struct PoseidonDoOut_SuperLayout {
  _super: PoseidonStateLayout,
  arm0: PoseidonDoOut_SuperArm0Layout,
  arm1: PoseidonStoreOutLayout,
}
struct BoundLayout_PoseidonDoOut_SuperLayout {
  lyt: PoseidonDoOut_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonDoOut_SuperLayout__super(b: BoundLayout_PoseidonDoOut_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonDoOut_SuperLayout_arm0(b: BoundLayout_PoseidonDoOut_SuperLayout) -> BoundLayout_PoseidonDoOut_SuperArm0Layout {
  return BoundLayout_PoseidonDoOut_SuperArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_PoseidonDoOut_SuperLayout_arm1(b: BoundLayout_PoseidonDoOut_SuperLayout) -> BoundLayout_PoseidonStoreOutLayout {
  return BoundLayout_PoseidonStoreOutLayout(b.lyt.arm1, b.buf);
}
struct _Arguments_PoseidonDoOut_SuperLayout {
  memoryArg: MemoryArgLayout16LayoutArray,
  cycleArg: CycleArgLayout8LayoutArray,
  argU16: ArgU16Layout24LayoutArray,
}
struct BoundLayout__Arguments_PoseidonDoOut_SuperLayout {
  lyt: _Arguments_PoseidonDoOut_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_PoseidonDoOut_SuperLayout_memoryArg(b: BoundLayout__Arguments_PoseidonDoOut_SuperLayout) -> BoundLayout_MemoryArgLayout16LayoutArray {
  return BoundLayout_MemoryArgLayout16LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_PoseidonDoOut_SuperLayout_cycleArg(b: BoundLayout__Arguments_PoseidonDoOut_SuperLayout) -> BoundLayout_CycleArgLayout8LayoutArray {
  return BoundLayout_CycleArgLayout8LayoutArray(b.lyt.cycleArg, b.buf);
}
fn lookup__Arguments_PoseidonDoOut_SuperLayout_argU16(b: BoundLayout__Arguments_PoseidonDoOut_SuperLayout) -> BoundLayout_ArgU16Layout24LayoutArray {
  return BoundLayout_ArgU16Layout24LayoutArray(b.lyt.argU16, b.buf);
}
struct PoseidonDoOutLayout {
  _super: PoseidonDoOut_SuperLayout,
  _arguments_PoseidonDoOut_Super: _Arguments_PoseidonDoOut_SuperLayout,
}
struct BoundLayout_PoseidonDoOutLayout {
  lyt: PoseidonDoOutLayout,
  buf: u32,
}
fn lookup_PoseidonDoOutLayout__super(b: BoundLayout_PoseidonDoOutLayout) -> BoundLayout_PoseidonDoOut_SuperLayout {
  return BoundLayout_PoseidonDoOut_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonDoOutLayout__arguments_PoseidonDoOut_Super(b: BoundLayout_PoseidonDoOutLayout) -> BoundLayout__Arguments_PoseidonDoOut_SuperLayout {
  return BoundLayout__Arguments_PoseidonDoOut_SuperLayout(b.lyt._arguments_PoseidonDoOut_Super, b.buf);
}
struct Poseidon0StateArm5Layout {
  _super: PoseidonDoOutLayout,
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm5Layout {
  lyt: Poseidon0StateArm5Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm5Layout__super(b: BoundLayout_Poseidon0StateArm5Layout) -> BoundLayout_PoseidonDoOutLayout {
  return BoundLayout_PoseidonDoOutLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm5Layout__extra0(b: BoundLayout_Poseidon0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm5Layout__extra1(b: BoundLayout_Poseidon0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
struct PoseidonPaging_SuperLayout {
  _super: PoseidonStateLayout,
  arm0: PoseidonStateLayout,
  arm1: PoseidonStateLayout,
  arm2: PoseidonStateLayout,
  arm3: PoseidonStateLayout,
  arm4: PoseidonStateLayout,
  arm5: PoseidonStateLayout,
}
struct BoundLayout_PoseidonPaging_SuperLayout {
  lyt: PoseidonPaging_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonPaging_SuperLayout__super(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonPaging_SuperLayout_arm0(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm0, b.buf);
}
fn lookup_PoseidonPaging_SuperLayout_arm1(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm1, b.buf);
}
fn lookup_PoseidonPaging_SuperLayout_arm2(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm2, b.buf);
}
fn lookup_PoseidonPaging_SuperLayout_arm3(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm3, b.buf);
}
fn lookup_PoseidonPaging_SuperLayout_arm4(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm4, b.buf);
}
fn lookup_PoseidonPaging_SuperLayout_arm5(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm5, b.buf);
}
struct IsU24Layout {
  low16: NondetU16RegLayout,
  _0: NondetU8RegLayout,
}
struct BoundLayout_IsU24Layout {
  lyt: IsU24Layout,
  buf: u32,
}
fn lookup_IsU24Layout_low16(b: BoundLayout_IsU24Layout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.low16, b.buf);
}
fn lookup_IsU24Layout__0(b: BoundLayout_IsU24Layout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt._0, b.buf);
}
alias ArgU8Layout1LayoutArray = array<ArgU8Layout, 1>;
struct BoundLayout_ArgU8Layout1LayoutArray {
  lyt: ArgU8Layout1LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout1LayoutArray(b: BoundLayout_ArgU8Layout1LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
struct _Arguments_PoseidonPaging__1Layout {
  argU16: ArgU16Layout1LayoutArray,
  argU8: ArgU8Layout1LayoutArray,
}
struct BoundLayout__Arguments_PoseidonPaging__1Layout {
  lyt: _Arguments_PoseidonPaging__1Layout,
  buf: u32,
}
fn lookup__Arguments_PoseidonPaging__1Layout_argU16(b: BoundLayout__Arguments_PoseidonPaging__1Layout) -> BoundLayout_ArgU16Layout1LayoutArray {
  return BoundLayout_ArgU16Layout1LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_PoseidonPaging__1Layout_argU8(b: BoundLayout__Arguments_PoseidonPaging__1Layout) -> BoundLayout_ArgU8Layout1LayoutArray {
  return BoundLayout_ArgU8Layout1LayoutArray(b.lyt.argU8, b.buf);
}
struct PoseidonPaging__1Arm0_SuperLayout {
  _0: IsU24Layout,
}
struct BoundLayout_PoseidonPaging__1Arm0_SuperLayout {
  lyt: PoseidonPaging__1Arm0_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonPaging__1Arm0_SuperLayout__0(b: BoundLayout_PoseidonPaging__1Arm0_SuperLayout) -> BoundLayout_IsU24Layout {
  return BoundLayout_IsU24Layout(b.lyt._0, b.buf);
}
struct PoseidonPaging__1Arm1_SuperLayout {
  _0: IsU24Layout,
}
struct BoundLayout_PoseidonPaging__1Arm1_SuperLayout {
  lyt: PoseidonPaging__1Arm1_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonPaging__1Arm1_SuperLayout__0(b: BoundLayout_PoseidonPaging__1Arm1_SuperLayout) -> BoundLayout_IsU24Layout {
  return BoundLayout_IsU24Layout(b.lyt._0, b.buf);
}
struct PoseidonPaging__1Layout {
  arm0: PoseidonPaging__1Arm0_SuperLayout,
  arm1: PoseidonPaging__1Arm1_SuperLayout,
}
struct BoundLayout_PoseidonPaging__1Layout {
  lyt: PoseidonPaging__1Layout,
  buf: u32,
}
fn lookup_PoseidonPaging__1Layout_arm0(b: BoundLayout_PoseidonPaging__1Layout) -> BoundLayout_PoseidonPaging__1Arm0_SuperLayout {
  return BoundLayout_PoseidonPaging__1Arm0_SuperLayout(b.lyt.arm0, b.buf);
}
fn lookup_PoseidonPaging__1Layout_arm1(b: BoundLayout_PoseidonPaging__1Layout) -> BoundLayout_PoseidonPaging__1Arm1_SuperLayout {
  return BoundLayout_PoseidonPaging__1Arm1_SuperLayout(b.lyt.arm1, b.buf);
}
struct PoseidonPagingLayout {
  _super: PoseidonPaging_SuperLayout,
  curIdx: NondetRegLayout,
  curMode: NondetRegLayout,
  modeSplit: OneHot_6_Layout,
  _0: IsU24Layout,
  _arguments_PoseidonPaging__1: _Arguments_PoseidonPaging__1Layout,
  _3: PoseidonPaging__1Layout,
  _4: NondetRegLayout,
}
struct BoundLayout_PoseidonPagingLayout {
  lyt: PoseidonPagingLayout,
  buf: u32,
}
fn lookup_PoseidonPagingLayout__super(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_PoseidonPaging_SuperLayout {
  return BoundLayout_PoseidonPaging_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonPagingLayout_curIdx(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.curIdx, b.buf);
}
fn lookup_PoseidonPagingLayout_curMode(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.curMode, b.buf);
}
fn lookup_PoseidonPagingLayout_modeSplit(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_OneHot_6_Layout {
  return BoundLayout_OneHot_6_Layout(b.lyt.modeSplit, b.buf);
}
fn lookup_PoseidonPagingLayout__0(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_IsU24Layout {
  return BoundLayout_IsU24Layout(b.lyt._0, b.buf);
}
fn lookup_PoseidonPagingLayout__arguments_PoseidonPaging__1(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout__Arguments_PoseidonPaging__1Layout {
  return BoundLayout__Arguments_PoseidonPaging__1Layout(b.lyt._arguments_PoseidonPaging__1, b.buf);
}
fn lookup_PoseidonPagingLayout__3(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_PoseidonPaging__1Layout {
  return BoundLayout_PoseidonPaging__1Layout(b.lyt._3, b.buf);
}
fn lookup_PoseidonPagingLayout__4(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._4, b.buf);
}
struct Poseidon0StateArm6Layout {
  _super: PoseidonPagingLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: MemoryArgLayout,
  _extra13: MemoryArgLayout,
  _extra14: MemoryArgLayout,
  _extra15: MemoryArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: CycleArgLayout,
  _extra19: CycleArgLayout,
  _extra20: CycleArgLayout,
  _extra21: CycleArgLayout,
  _extra22: CycleArgLayout,
  _extra23: CycleArgLayout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU16Layout,
  _extra28: ArgU16Layout,
  _extra29: ArgU16Layout,
  _extra30: ArgU16Layout,
  _extra31: ArgU16Layout,
  _extra32: ArgU16Layout,
  _extra33: ArgU16Layout,
  _extra34: ArgU16Layout,
  _extra35: ArgU16Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
  _extra40: ArgU16Layout,
  _extra41: ArgU16Layout,
  _extra42: ArgU16Layout,
  _extra43: ArgU16Layout,
  _extra44: ArgU16Layout,
  _extra45: ArgU16Layout,
}
struct BoundLayout_Poseidon0StateArm6Layout {
  lyt: Poseidon0StateArm6Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm6Layout__super(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_PoseidonPagingLayout {
  return BoundLayout_PoseidonPagingLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra0(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra1(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra2(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra3(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra4(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra5(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra6(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra7(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra8(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra9(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra10(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra11(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra12(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra13(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra14(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra15(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra16(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra17(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra18(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra18, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra19(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra19, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra20(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra20, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra21(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra21, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra22(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra22, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra23(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra23, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra24(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra25(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra26(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra27(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra27, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra28(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra28, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra29(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra29, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra30(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra30, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra31(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra31, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra32(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra32, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra33(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra33, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra34(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra34, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra35(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra35, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra36(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra37(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra38(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra39(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra40(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra40, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra41(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra41, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra42(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra42, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra43(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra43, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra44(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra44, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra45(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra45, b.buf);
}
struct PoseidonStoreState__0_SuperLayout {
  ftw: FieldToWordLayout,
  mw: MemoryWriteLayout,
}
struct BoundLayout_PoseidonStoreState__0_SuperLayout {
  lyt: PoseidonStoreState__0_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonStoreState__0_SuperLayout_ftw(b: BoundLayout_PoseidonStoreState__0_SuperLayout) -> BoundLayout_FieldToWordLayout {
  return BoundLayout_FieldToWordLayout(b.lyt.ftw, b.buf);
}
fn lookup_PoseidonStoreState__0_SuperLayout_mw(b: BoundLayout_PoseidonStoreState__0_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt.mw, b.buf);
}
alias PoseidonStoreState__0_SuperLayout8LayoutArray = array<PoseidonStoreState__0_SuperLayout, 8>;
struct BoundLayout_PoseidonStoreState__0_SuperLayout8LayoutArray {
  lyt: PoseidonStoreState__0_SuperLayout8LayoutArray,
  buf: u32,
}
fn subscript_PoseidonStoreState__0_SuperLayout8LayoutArray(b: BoundLayout_PoseidonStoreState__0_SuperLayout8LayoutArray, i: u32) -> BoundLayout_PoseidonStoreState__0_SuperLayout {
  return BoundLayout_PoseidonStoreState__0_SuperLayout(b.lyt[i], b.buf);
}
struct PoseidonStoreStateLayout {
  _super: PoseidonStateLayout,
  _1: PoseidonStoreState__0_SuperLayout8LayoutArray,
}
struct BoundLayout_PoseidonStoreStateLayout {
  lyt: PoseidonStoreStateLayout,
  buf: u32,
}
fn lookup_PoseidonStoreStateLayout__super(b: BoundLayout_PoseidonStoreStateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonStoreStateLayout__1(b: BoundLayout_PoseidonStoreStateLayout) -> BoundLayout_PoseidonStoreState__0_SuperLayout8LayoutArray {
  return BoundLayout_PoseidonStoreState__0_SuperLayout8LayoutArray(b.lyt._1, b.buf);
}
struct Poseidon0StateArm7Layout {
  _super: PoseidonStoreStateLayout,
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm7Layout {
  lyt: Poseidon0StateArm7Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm7Layout__super(b: BoundLayout_Poseidon0StateArm7Layout) -> BoundLayout_PoseidonStoreStateLayout {
  return BoundLayout_PoseidonStoreStateLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm7Layout__extra0(b: BoundLayout_Poseidon0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm7Layout__extra1(b: BoundLayout_Poseidon0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
struct Poseidon0StateLayout {
  _super: PoseidonStateLayout,
  arm0: Poseidon0StateArm0Layout,
  arm1: Poseidon0StateArm1Layout,
  arm2: Poseidon0StateArm2Layout,
  arm3: Poseidon0StateArm3Layout,
  arm4: Poseidon0StateArm4Layout,
  arm5: Poseidon0StateArm5Layout,
  arm6: Poseidon0StateArm6Layout,
  arm7: Poseidon0StateArm7Layout,
}
struct BoundLayout_Poseidon0StateLayout {
  lyt: Poseidon0StateLayout,
  buf: u32,
}
fn lookup_Poseidon0StateLayout__super(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateLayout_arm0(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm0Layout {
  return BoundLayout_Poseidon0StateArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_Poseidon0StateLayout_arm1(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm1Layout {
  return BoundLayout_Poseidon0StateArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Poseidon0StateLayout_arm2(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm2Layout {
  return BoundLayout_Poseidon0StateArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Poseidon0StateLayout_arm3(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm3Layout {
  return BoundLayout_Poseidon0StateArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Poseidon0StateLayout_arm4(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm4Layout {
  return BoundLayout_Poseidon0StateArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Poseidon0StateLayout_arm5(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm5Layout {
  return BoundLayout_Poseidon0StateArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Poseidon0StateLayout_arm6(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm6Layout {
  return BoundLayout_Poseidon0StateArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Poseidon0StateLayout_arm7(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm7Layout {
  return BoundLayout_Poseidon0StateArm7Layout(b.lyt.arm7, b.buf);
}
struct Poseidon0Layout {
  _0: DoCycleTableLayout,
  state: PoseidonStateLayout,
  _arguments_Poseidon0State: _Arguments_Poseidon0StateLayout,
  stateRedef: Poseidon0StateLayout,
}
struct BoundLayout_Poseidon0Layout {
  lyt: Poseidon0Layout,
  buf: u32,
}
fn lookup_Poseidon0Layout__0(b: BoundLayout_Poseidon0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Poseidon0Layout_state(b: BoundLayout_Poseidon0Layout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.state, b.buf);
}
fn lookup_Poseidon0Layout__arguments_Poseidon0State(b: BoundLayout_Poseidon0Layout) -> BoundLayout__Arguments_Poseidon0StateLayout {
  return BoundLayout__Arguments_Poseidon0StateLayout(b.lyt._arguments_Poseidon0State, b.buf);
}
fn lookup_Poseidon0Layout_stateRedef(b: BoundLayout_Poseidon0Layout) -> BoundLayout_Poseidon0StateLayout {
  return BoundLayout_Poseidon0StateLayout(b.lyt.stateRedef, b.buf);
}
struct SBoxLayout {
  _super: NondetRegLayout,
  cubed: NondetRegLayout,
}
struct BoundLayout_SBoxLayout {
  lyt: SBoxLayout,
  buf: u32,
}
fn lookup_SBoxLayout__super(b: BoundLayout_SBoxLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._super, b.buf);
}
fn lookup_SBoxLayout_cubed(b: BoundLayout_SBoxLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.cubed, b.buf);
}
alias SBoxLayout24LayoutArray = array<SBoxLayout, 24>;
struct BoundLayout_SBoxLayout24LayoutArray {
  lyt: SBoxLayout24LayoutArray,
  buf: u32,
}
fn subscript_SBoxLayout24LayoutArray(b: BoundLayout_SBoxLayout24LayoutArray, i: u32) -> BoundLayout_SBoxLayout {
  return BoundLayout_SBoxLayout(b.lyt[i], b.buf);
}
struct DoExtRoundLayout {
  _1: SBoxLayout24LayoutArray,
}
struct BoundLayout_DoExtRoundLayout {
  lyt: DoExtRoundLayout,
  buf: u32,
}
fn lookup_DoExtRoundLayout__1(b: BoundLayout_DoExtRoundLayout) -> BoundLayout_SBoxLayout24LayoutArray {
  return BoundLayout_SBoxLayout24LayoutArray(b.lyt._1, b.buf);
}
struct DoExtRoundByIdxLayout {
  _super: DoExtRoundLayout,
  idxHot: OneHot_8_Layout,
}
struct BoundLayout_DoExtRoundByIdxLayout {
  lyt: DoExtRoundByIdxLayout,
  buf: u32,
}
fn lookup_DoExtRoundByIdxLayout__super(b: BoundLayout_DoExtRoundByIdxLayout) -> BoundLayout_DoExtRoundLayout {
  return BoundLayout_DoExtRoundLayout(b.lyt._super, b.buf);
}
fn lookup_DoExtRoundByIdxLayout_idxHot(b: BoundLayout_DoExtRoundByIdxLayout) -> BoundLayout_OneHot_8_Layout {
  return BoundLayout_OneHot_8_Layout(b.lyt.idxHot, b.buf);
}
struct PoseidonExtRoundLayout {
  _super: PoseidonStateLayout,
  isRound3: IsZeroLayout,
  isRound7: IsZeroLayout,
  lastBlock: IsZeroLayout,
  nextInner: DoExtRoundByIdxLayout,
}
struct BoundLayout_PoseidonExtRoundLayout {
  lyt: PoseidonExtRoundLayout,
  buf: u32,
}
fn lookup_PoseidonExtRoundLayout__super(b: BoundLayout_PoseidonExtRoundLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonExtRoundLayout_isRound3(b: BoundLayout_PoseidonExtRoundLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isRound3, b.buf);
}
fn lookup_PoseidonExtRoundLayout_isRound7(b: BoundLayout_PoseidonExtRoundLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isRound7, b.buf);
}
fn lookup_PoseidonExtRoundLayout_lastBlock(b: BoundLayout_PoseidonExtRoundLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lastBlock, b.buf);
}
fn lookup_PoseidonExtRoundLayout_nextInner(b: BoundLayout_PoseidonExtRoundLayout) -> BoundLayout_DoExtRoundByIdxLayout {
  return BoundLayout_DoExtRoundByIdxLayout(b.lyt.nextInner, b.buf);
}
struct DoIntRoundLayout {
  sbox: SBoxLayout,
}
struct BoundLayout_DoIntRoundLayout {
  lyt: DoIntRoundLayout,
  buf: u32,
}
fn lookup_DoIntRoundLayout_sbox(b: BoundLayout_DoIntRoundLayout) -> BoundLayout_SBoxLayout {
  return BoundLayout_SBoxLayout(b.lyt.sbox, b.buf);
}
alias DoIntRoundLayout21LayoutArray = array<DoIntRoundLayout, 21>;
struct BoundLayout_DoIntRoundLayout21LayoutArray {
  lyt: DoIntRoundLayout21LayoutArray,
  buf: u32,
}
fn subscript_DoIntRoundLayout21LayoutArray(b: BoundLayout_DoIntRoundLayout21LayoutArray, i: u32) -> BoundLayout_DoIntRoundLayout {
  return BoundLayout_DoIntRoundLayout(b.lyt[i], b.buf);
}
struct DoIntRoundsLayout {
  _super: DoIntRoundLayout21LayoutArray,
}
struct BoundLayout_DoIntRoundsLayout {
  lyt: DoIntRoundsLayout,
  buf: u32,
}
fn lookup_DoIntRoundsLayout__super(b: BoundLayout_DoIntRoundsLayout) -> BoundLayout_DoIntRoundLayout21LayoutArray {
  return BoundLayout_DoIntRoundLayout21LayoutArray(b.lyt._super, b.buf);
}
struct PoseidonIntRoundsLayout {
  _super: PoseidonStateLayout,
  nextInner: DoIntRoundsLayout,
}
struct BoundLayout_PoseidonIntRoundsLayout {
  lyt: PoseidonIntRoundsLayout,
  buf: u32,
}
fn lookup_PoseidonIntRoundsLayout__super(b: BoundLayout_PoseidonIntRoundsLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonIntRoundsLayout_nextInner(b: BoundLayout_PoseidonIntRoundsLayout) -> BoundLayout_DoIntRoundsLayout {
  return BoundLayout_DoIntRoundsLayout(b.lyt.nextInner, b.buf);
}
struct Poseidon1StateLayout {
  _super: PoseidonStateLayout,
  arm0: PoseidonExtRoundLayout,
  arm1: PoseidonIntRoundsLayout,
  arm2: PoseidonStateLayout,
  arm3: PoseidonStateLayout,
  arm4: PoseidonStateLayout,
  arm5: PoseidonStateLayout,
  arm6: PoseidonStateLayout,
  arm7: PoseidonStateLayout,
}
struct BoundLayout_Poseidon1StateLayout {
  lyt: Poseidon1StateLayout,
  buf: u32,
}
fn lookup_Poseidon1StateLayout__super(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon1StateLayout_arm0(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonExtRoundLayout {
  return BoundLayout_PoseidonExtRoundLayout(b.lyt.arm0, b.buf);
}
fn lookup_Poseidon1StateLayout_arm1(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonIntRoundsLayout {
  return BoundLayout_PoseidonIntRoundsLayout(b.lyt.arm1, b.buf);
}
fn lookup_Poseidon1StateLayout_arm2(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm2, b.buf);
}
fn lookup_Poseidon1StateLayout_arm3(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm3, b.buf);
}
fn lookup_Poseidon1StateLayout_arm4(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm4, b.buf);
}
fn lookup_Poseidon1StateLayout_arm5(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm5, b.buf);
}
fn lookup_Poseidon1StateLayout_arm6(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm6, b.buf);
}
fn lookup_Poseidon1StateLayout_arm7(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm7, b.buf);
}
struct Poseidon1Layout {
  _0: DoCycleTableLayout,
  state: PoseidonStateLayout,
  stateRedef: Poseidon1StateLayout,
}
struct BoundLayout_Poseidon1Layout {
  lyt: Poseidon1Layout,
  buf: u32,
}
fn lookup_Poseidon1Layout__0(b: BoundLayout_Poseidon1Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Poseidon1Layout_state(b: BoundLayout_Poseidon1Layout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.state, b.buf);
}
fn lookup_Poseidon1Layout_stateRedef(b: BoundLayout_Poseidon1Layout) -> BoundLayout_Poseidon1StateLayout {
  return BoundLayout_Poseidon1StateLayout(b.lyt.stateRedef, b.buf);
}
alias NondetRegLayout32LayoutArray = array<NondetRegLayout, 32>;
struct BoundLayout_NondetRegLayout32LayoutArray {
  lyt: NondetRegLayout32LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout32LayoutArray(b: BoundLayout_NondetRegLayout32LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct ShaStateLayout {
  stateInAddr: NondetRegLayout,
  stateOutAddr: NondetRegLayout,
  dataAddr: NondetRegLayout,
  count: NondetRegLayout,
  kAddr: NondetRegLayout,
  round: NondetRegLayout,
  nextState: NondetRegLayout,
  a: NondetRegLayout32LayoutArray,
  e: NondetRegLayout32LayoutArray,
  w: NondetRegLayout32LayoutArray,
}
struct BoundLayout_ShaStateLayout {
  lyt: ShaStateLayout,
  buf: u32,
}
fn lookup_ShaStateLayout_stateInAddr(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.stateInAddr, b.buf);
}
fn lookup_ShaStateLayout_stateOutAddr(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.stateOutAddr, b.buf);
}
fn lookup_ShaStateLayout_dataAddr(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.dataAddr, b.buf);
}
fn lookup_ShaStateLayout_count(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.count, b.buf);
}
fn lookup_ShaStateLayout_kAddr(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.kAddr, b.buf);
}
fn lookup_ShaStateLayout_round(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.round, b.buf);
}
fn lookup_ShaStateLayout_nextState(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextState, b.buf);
}
fn lookup_ShaStateLayout_a(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout32LayoutArray {
  return BoundLayout_NondetRegLayout32LayoutArray(b.lyt.a, b.buf);
}
fn lookup_ShaStateLayout_e(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout32LayoutArray {
  return BoundLayout_NondetRegLayout32LayoutArray(b.lyt.e, b.buf);
}
fn lookup_ShaStateLayout_w(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout32LayoutArray {
  return BoundLayout_NondetRegLayout32LayoutArray(b.lyt.w, b.buf);
}
alias MemoryArgLayout10LayoutArray = array<MemoryArgLayout, 10>;
struct BoundLayout_MemoryArgLayout10LayoutArray {
  lyt: MemoryArgLayout10LayoutArray,
  buf: u32,
}
fn subscript_MemoryArgLayout10LayoutArray(b: BoundLayout_MemoryArgLayout10LayoutArray, i: u32) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt[i], b.buf);
}
alias CycleArgLayout5LayoutArray = array<CycleArgLayout, 5>;
struct BoundLayout_CycleArgLayout5LayoutArray {
  lyt: CycleArgLayout5LayoutArray,
  buf: u32,
}
fn subscript_CycleArgLayout5LayoutArray(b: BoundLayout_CycleArgLayout5LayoutArray, i: u32) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt[i], b.buf);
}
struct _Arguments_Sha0StateLayout {
  memoryArg: MemoryArgLayout10LayoutArray,
  cycleArg: CycleArgLayout5LayoutArray,
}
struct BoundLayout__Arguments_Sha0StateLayout {
  lyt: _Arguments_Sha0StateLayout,
  buf: u32,
}
fn lookup__Arguments_Sha0StateLayout_memoryArg(b: BoundLayout__Arguments_Sha0StateLayout) -> BoundLayout_MemoryArgLayout10LayoutArray {
  return BoundLayout_MemoryArgLayout10LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_Sha0StateLayout_cycleArg(b: BoundLayout__Arguments_Sha0StateLayout) -> BoundLayout_CycleArgLayout5LayoutArray {
  return BoundLayout_CycleArgLayout5LayoutArray(b.lyt.cycleArg, b.buf);
}
struct ShaEcallLayout {
  _super: ShaStateLayout,
  stateInAddr: ReadAddrLayout,
  stateOutAddr: ReadAddrLayout,
  dataAddr: ReadAddrLayout,
  _0: MemoryReadLayout,
  kAddr: ReadAddrLayout,
}
struct BoundLayout_ShaEcallLayout {
  lyt: ShaEcallLayout,
  buf: u32,
}
fn lookup_ShaEcallLayout__super(b: BoundLayout_ShaEcallLayout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_ShaEcallLayout_stateInAddr(b: BoundLayout_ShaEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.stateInAddr, b.buf);
}
fn lookup_ShaEcallLayout_stateOutAddr(b: BoundLayout_ShaEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.stateOutAddr, b.buf);
}
fn lookup_ShaEcallLayout_dataAddr(b: BoundLayout_ShaEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.dataAddr, b.buf);
}
fn lookup_ShaEcallLayout__0(b: BoundLayout_ShaEcallLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt._0, b.buf);
}
fn lookup_ShaEcallLayout_kAddr(b: BoundLayout_ShaEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.kAddr, b.buf);
}
struct ShaLoadStateLayout {
  _super: ShaStateLayout,
  lastRound: IsZeroLayout,
  countZero: IsZeroLayout,
  a32: MemoryReadLayout,
  e32: MemoryReadLayout,
  _0: MemoryWriteLayout,
  _1: MemoryWriteLayout,
}
struct BoundLayout_ShaLoadStateLayout {
  lyt: ShaLoadStateLayout,
  buf: u32,
}
fn lookup_ShaLoadStateLayout__super(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_ShaLoadStateLayout_lastRound(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lastRound, b.buf);
}
fn lookup_ShaLoadStateLayout_countZero(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.countZero, b.buf);
}
fn lookup_ShaLoadStateLayout_a32(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.a32, b.buf);
}
fn lookup_ShaLoadStateLayout_e32(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.e32, b.buf);
}
fn lookup_ShaLoadStateLayout__0(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
fn lookup_ShaLoadStateLayout__1(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._1, b.buf);
}
struct Sha0StateArm1Layout {
  _super: ShaLoadStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm1Layout {
  lyt: Sha0StateArm1Layout,
  buf: u32,
}
fn lookup_Sha0StateArm1Layout__super(b: BoundLayout_Sha0StateArm1Layout) -> BoundLayout_ShaLoadStateLayout {
  return BoundLayout_ShaLoadStateLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm1Layout__extra0(b: BoundLayout_Sha0StateArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm1Layout__extra1(b: BoundLayout_Sha0StateArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm1Layout__extra2(b: BoundLayout_Sha0StateArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra2, b.buf);
}
struct UnpackReg_32__16_Layout {
  _super: NondetRegLayout32LayoutArray,
}
struct BoundLayout_UnpackReg_32__16_Layout {
  lyt: UnpackReg_32__16_Layout,
  buf: u32,
}
fn lookup_UnpackReg_32__16_Layout__super(b: BoundLayout_UnpackReg_32__16_Layout) -> BoundLayout_NondetRegLayout32LayoutArray {
  return BoundLayout_NondetRegLayout32LayoutArray(b.lyt._super, b.buf);
}
struct CarryExtractLayout {
  bit0: NondetRegLayout,
  bit1: NondetRegLayout,
  bit2: NondetRegLayout,
}
struct BoundLayout_CarryExtractLayout {
  lyt: CarryExtractLayout,
  buf: u32,
}
fn lookup_CarryExtractLayout_bit0(b: BoundLayout_CarryExtractLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.bit0, b.buf);
}
fn lookup_CarryExtractLayout_bit1(b: BoundLayout_CarryExtractLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.bit1, b.buf);
}
fn lookup_CarryExtractLayout_bit2(b: BoundLayout_CarryExtractLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.bit2, b.buf);
}
struct CarryAndExpandLayout {
  _super: UnpackReg_32__16_Layout,
  lowCarry: CarryExtractLayout,
  highCarry: CarryExtractLayout,
}
struct BoundLayout_CarryAndExpandLayout {
  lyt: CarryAndExpandLayout,
  buf: u32,
}
fn lookup_CarryAndExpandLayout__super(b: BoundLayout_CarryAndExpandLayout) -> BoundLayout_UnpackReg_32__16_Layout {
  return BoundLayout_UnpackReg_32__16_Layout(b.lyt._super, b.buf);
}
fn lookup_CarryAndExpandLayout_lowCarry(b: BoundLayout_CarryAndExpandLayout) -> BoundLayout_CarryExtractLayout {
  return BoundLayout_CarryExtractLayout(b.lyt.lowCarry, b.buf);
}
fn lookup_CarryAndExpandLayout_highCarry(b: BoundLayout_CarryAndExpandLayout) -> BoundLayout_CarryExtractLayout {
  return BoundLayout_CarryExtractLayout(b.lyt.highCarry, b.buf);
}
struct ShaLoadDataLayout {
  _super: ShaStateLayout,
  lastRound: IsZeroLayout,
  k: MemoryReadLayout,
  wMem: MemoryReadLayout,
  wBits: NondetRegLayout32LayoutArray,
  a: CarryAndExpandLayout,
  e: CarryAndExpandLayout,
}
struct BoundLayout_ShaLoadDataLayout {
  lyt: ShaLoadDataLayout,
  buf: u32,
}
fn lookup_ShaLoadDataLayout__super(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_ShaLoadDataLayout_lastRound(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lastRound, b.buf);
}
fn lookup_ShaLoadDataLayout_k(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.k, b.buf);
}
fn lookup_ShaLoadDataLayout_wMem(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.wMem, b.buf);
}
fn lookup_ShaLoadDataLayout_wBits(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_NondetRegLayout32LayoutArray {
  return BoundLayout_NondetRegLayout32LayoutArray(b.lyt.wBits, b.buf);
}
fn lookup_ShaLoadDataLayout_a(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.a, b.buf);
}
fn lookup_ShaLoadDataLayout_e(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.e, b.buf);
}
struct Sha0StateArm2Layout {
  _super: ShaLoadDataLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: CycleArgLayout,
  _extra7: CycleArgLayout,
  _extra8: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm2Layout {
  lyt: Sha0StateArm2Layout,
  buf: u32,
}
fn lookup_Sha0StateArm2Layout__super(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_ShaLoadDataLayout {
  return BoundLayout_ShaLoadDataLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra0(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra1(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra2(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra3(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra4(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra5(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra6(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra7(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra8(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
struct ShaMixLayout {
  _super: ShaStateLayout,
  lastRound: IsZeroLayout,
  k: MemoryReadLayout,
  wBits: CarryAndExpandLayout,
  a: CarryAndExpandLayout,
  e: CarryAndExpandLayout,
}
struct BoundLayout_ShaMixLayout {
  lyt: ShaMixLayout,
  buf: u32,
}
fn lookup_ShaMixLayout__super(b: BoundLayout_ShaMixLayout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_ShaMixLayout_lastRound(b: BoundLayout_ShaMixLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lastRound, b.buf);
}
fn lookup_ShaMixLayout_k(b: BoundLayout_ShaMixLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.k, b.buf);
}
fn lookup_ShaMixLayout_wBits(b: BoundLayout_ShaMixLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.wBits, b.buf);
}
fn lookup_ShaMixLayout_a(b: BoundLayout_ShaMixLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.a, b.buf);
}
fn lookup_ShaMixLayout_e(b: BoundLayout_ShaMixLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.e, b.buf);
}
struct Sha0StateArm3Layout {
  _super: ShaMixLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: CycleArgLayout,
  _extra9: CycleArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm3Layout {
  lyt: Sha0StateArm3Layout,
  buf: u32,
}
fn lookup_Sha0StateArm3Layout__super(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_ShaMixLayout {
  return BoundLayout_ShaMixLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra0(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra1(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra2(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra3(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra4(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra5(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra6(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra7(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra8(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra9(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra10(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra11(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
struct ShaStoreStateLayout {
  _super: ShaStateLayout,
  lastRound: IsZeroLayout,
  countZero: IsZeroLayout,
  a: CarryAndExpandLayout,
  e: CarryAndExpandLayout,
  _1: MemoryWriteLayout,
  _2: MemoryWriteLayout,
}
struct BoundLayout_ShaStoreStateLayout {
  lyt: ShaStoreStateLayout,
  buf: u32,
}
fn lookup_ShaStoreStateLayout__super(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_ShaStoreStateLayout_lastRound(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lastRound, b.buf);
}
fn lookup_ShaStoreStateLayout_countZero(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.countZero, b.buf);
}
fn lookup_ShaStoreStateLayout_a(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.a, b.buf);
}
fn lookup_ShaStoreStateLayout_e(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.e, b.buf);
}
fn lookup_ShaStoreStateLayout__1(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._1, b.buf);
}
fn lookup_ShaStoreStateLayout__2(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._2, b.buf);
}
struct Sha0StateArm4Layout {
  _super: ShaStoreStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: CycleArgLayout,
  _extra7: CycleArgLayout,
  _extra8: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm4Layout {
  lyt: Sha0StateArm4Layout,
  buf: u32,
}
fn lookup_Sha0StateArm4Layout__super(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_ShaStoreStateLayout {
  return BoundLayout_ShaStoreStateLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra0(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra1(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra2(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra3(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra4(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra5(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra6(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra7(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra8(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
struct Sha0StateArm5Layout {
  _super: ShaStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm5Layout {
  lyt: Sha0StateArm5Layout,
  buf: u32,
}
fn lookup_Sha0StateArm5Layout__super(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra0(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra1(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra2(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra3(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra4(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra5(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra6(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra7(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra8(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra9(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra10(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra11(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra12(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra13(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra14(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
struct Sha0StateArm6Layout {
  _super: ShaStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm6Layout {
  lyt: Sha0StateArm6Layout,
  buf: u32,
}
fn lookup_Sha0StateArm6Layout__super(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra0(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra1(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra2(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra3(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra4(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra5(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra6(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra7(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra8(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra9(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra10(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra11(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra12(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra13(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra14(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
struct Sha0StateArm7Layout {
  _super: ShaStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm7Layout {
  lyt: Sha0StateArm7Layout,
  buf: u32,
}
fn lookup_Sha0StateArm7Layout__super(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra0(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra1(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra2(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra3(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra4(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra5(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra6(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra7(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra8(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra9(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra10(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra11(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra12(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra13(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra14(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
struct Sha0StateLayout {
  _super: ShaStateLayout,
  arm0: ShaEcallLayout,
  arm1: Sha0StateArm1Layout,
  arm2: Sha0StateArm2Layout,
  arm3: Sha0StateArm3Layout,
  arm4: Sha0StateArm4Layout,
  arm5: Sha0StateArm5Layout,
  arm6: Sha0StateArm6Layout,
  arm7: Sha0StateArm7Layout,
}
struct BoundLayout_Sha0StateLayout {
  lyt: Sha0StateLayout,
  buf: u32,
}
fn lookup_Sha0StateLayout__super(b: BoundLayout_Sha0StateLayout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateLayout_arm0(b: BoundLayout_Sha0StateLayout) -> BoundLayout_ShaEcallLayout {
  return BoundLayout_ShaEcallLayout(b.lyt.arm0, b.buf);
}
fn lookup_Sha0StateLayout_arm1(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm1Layout {
  return BoundLayout_Sha0StateArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Sha0StateLayout_arm2(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm2Layout {
  return BoundLayout_Sha0StateArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Sha0StateLayout_arm3(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm3Layout {
  return BoundLayout_Sha0StateArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Sha0StateLayout_arm4(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm4Layout {
  return BoundLayout_Sha0StateArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Sha0StateLayout_arm5(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm5Layout {
  return BoundLayout_Sha0StateArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Sha0StateLayout_arm6(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm6Layout {
  return BoundLayout_Sha0StateArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Sha0StateLayout_arm7(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm7Layout {
  return BoundLayout_Sha0StateArm7Layout(b.lyt.arm7, b.buf);
}
struct Sha0Layout {
  _0: DoCycleTableLayout,
  state: ShaStateLayout,
  _arguments_Sha0State: _Arguments_Sha0StateLayout,
  stateRedef: Sha0StateLayout,
}
struct BoundLayout_Sha0Layout {
  lyt: Sha0Layout,
  buf: u32,
}
fn lookup_Sha0Layout__0(b: BoundLayout_Sha0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Sha0Layout_state(b: BoundLayout_Sha0Layout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt.state, b.buf);
}
fn lookup_Sha0Layout__arguments_Sha0State(b: BoundLayout_Sha0Layout) -> BoundLayout__Arguments_Sha0StateLayout {
  return BoundLayout__Arguments_Sha0StateLayout(b.lyt._arguments_Sha0State, b.buf);
}
fn lookup_Sha0Layout_stateRedef(b: BoundLayout_Sha0Layout) -> BoundLayout_Sha0StateLayout {
  return BoundLayout_Sha0StateLayout(b.lyt.stateRedef, b.buf);
}
struct BigIntStateLayout {
  isEcall: NondetRegLayout,
  mode: NondetRegLayout,
  pc: NondetRegLayout,
  polyOp: NondetRegLayout,
  coeff: NondetRegLayout,
  bytes: NondetRegLayout16LayoutArray,
  nextState: NondetRegLayout,
}
struct BoundLayout_BigIntStateLayout {
  lyt: BigIntStateLayout,
  buf: u32,
}
fn lookup_BigIntStateLayout_isEcall(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isEcall, b.buf);
}
fn lookup_BigIntStateLayout_mode(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.mode, b.buf);
}
fn lookup_BigIntStateLayout_pc(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.pc, b.buf);
}
fn lookup_BigIntStateLayout_polyOp(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.polyOp, b.buf);
}
fn lookup_BigIntStateLayout_coeff(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.coeff, b.buf);
}
fn lookup_BigIntStateLayout_bytes(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout16LayoutArray {
  return BoundLayout_NondetRegLayout16LayoutArray(b.lyt.bytes, b.buf);
}
fn lookup_BigIntStateLayout_nextState(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextState, b.buf);
}
alias MemoryArgLayout12LayoutArray = array<MemoryArgLayout, 12>;
struct BoundLayout_MemoryArgLayout12LayoutArray {
  lyt: MemoryArgLayout12LayoutArray,
  buf: u32,
}
fn subscript_MemoryArgLayout12LayoutArray(b: BoundLayout_MemoryArgLayout12LayoutArray, i: u32) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt[i], b.buf);
}
alias CycleArgLayout6LayoutArray = array<CycleArgLayout, 6>;
struct BoundLayout_CycleArgLayout6LayoutArray {
  lyt: CycleArgLayout6LayoutArray,
  buf: u32,
}
fn subscript_CycleArgLayout6LayoutArray(b: BoundLayout_CycleArgLayout6LayoutArray, i: u32) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt[i], b.buf);
}
alias ArgU8Layout18LayoutArray = array<ArgU8Layout, 18>;
struct BoundLayout_ArgU8Layout18LayoutArray {
  lyt: ArgU8Layout18LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout18LayoutArray(b: BoundLayout_ArgU8Layout18LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
struct _Arguments_BigInt0StateLayout {
  memoryArg: MemoryArgLayout12LayoutArray,
  cycleArg: CycleArgLayout6LayoutArray,
  argU8: ArgU8Layout18LayoutArray,
  argU16: ArgU16Layout4LayoutArray,
}
struct BoundLayout__Arguments_BigInt0StateLayout {
  lyt: _Arguments_BigInt0StateLayout,
  buf: u32,
}
fn lookup__Arguments_BigInt0StateLayout_memoryArg(b: BoundLayout__Arguments_BigInt0StateLayout) -> BoundLayout_MemoryArgLayout12LayoutArray {
  return BoundLayout_MemoryArgLayout12LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_BigInt0StateLayout_cycleArg(b: BoundLayout__Arguments_BigInt0StateLayout) -> BoundLayout_CycleArgLayout6LayoutArray {
  return BoundLayout_CycleArgLayout6LayoutArray(b.lyt.cycleArg, b.buf);
}
fn lookup__Arguments_BigInt0StateLayout_argU8(b: BoundLayout__Arguments_BigInt0StateLayout) -> BoundLayout_ArgU8Layout18LayoutArray {
  return BoundLayout_ArgU8Layout18LayoutArray(b.lyt.argU8, b.buf);
}
fn lookup__Arguments_BigInt0StateLayout_argU16(b: BoundLayout__Arguments_BigInt0StateLayout) -> BoundLayout_ArgU16Layout4LayoutArray {
  return BoundLayout_ArgU16Layout4LayoutArray(b.lyt.argU16, b.buf);
}
struct BigIntEcallLayout {
  _super: BigIntStateLayout,
  mode: MemoryReadLayout,
  pc: ReadAddrLayout,
}
struct BoundLayout_BigIntEcallLayout {
  lyt: BigIntEcallLayout,
  buf: u32,
}
fn lookup_BigIntEcallLayout__super(b: BoundLayout_BigIntEcallLayout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntEcallLayout_mode(b: BoundLayout_BigIntEcallLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.mode, b.buf);
}
fn lookup_BigIntEcallLayout_pc(b: BoundLayout_BigIntEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.pc, b.buf);
}
struct BigInt0StateArm0Layout {
  _super: BigIntEcallLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: CycleArgLayout,
  _extra9: CycleArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: ArgU8Layout,
  _extra13: ArgU8Layout,
  _extra14: ArgU8Layout,
  _extra15: ArgU8Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU16Layout,
  _extra31: ArgU16Layout,
  _extra32: ArgU16Layout,
  _extra33: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm0Layout {
  lyt: BigInt0StateArm0Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm0Layout__super(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_BigIntEcallLayout {
  return BoundLayout_BigIntEcallLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra0(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra1(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra2(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra3(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra4(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra5(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra6(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra7(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra8(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra9(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra10(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra11(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra12(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra13(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra14(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra15(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra16(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra17(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra18(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra19(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra20(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra21(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra22(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra23(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra24(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra25(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra26(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra27(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra28(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra29(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra30(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra31(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra32(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra33(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra33, b.buf);
}
struct _Arguments_BigIntStepBytesLayout {
  argU16: ArgU16Layout2LayoutArray,
  argU8: ArgU8Layout16LayoutArray,
  memoryArg: MemoryArgLayout8LayoutArray,
  cycleArg: CycleArgLayout4LayoutArray,
}
struct BoundLayout__Arguments_BigIntStepBytesLayout {
  lyt: _Arguments_BigIntStepBytesLayout,
  buf: u32,
}
fn lookup__Arguments_BigIntStepBytesLayout_argU16(b: BoundLayout__Arguments_BigIntStepBytesLayout) -> BoundLayout_ArgU16Layout2LayoutArray {
  return BoundLayout_ArgU16Layout2LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_BigIntStepBytesLayout_argU8(b: BoundLayout__Arguments_BigIntStepBytesLayout) -> BoundLayout_ArgU8Layout16LayoutArray {
  return BoundLayout_ArgU8Layout16LayoutArray(b.lyt.argU8, b.buf);
}
fn lookup__Arguments_BigIntStepBytesLayout_memoryArg(b: BoundLayout__Arguments_BigIntStepBytesLayout) -> BoundLayout_MemoryArgLayout8LayoutArray {
  return BoundLayout_MemoryArgLayout8LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_BigIntStepBytesLayout_cycleArg(b: BoundLayout__Arguments_BigIntStepBytesLayout) -> BoundLayout_CycleArgLayout4LayoutArray {
  return BoundLayout_CycleArgLayout4LayoutArray(b.lyt.cycleArg, b.buf);
}
struct BigIntAddrLayout {
  _super: AddrDecomposeBitsLayout,
  _0: IsZeroLayout,
}
struct BoundLayout_BigIntAddrLayout {
  lyt: BigIntAddrLayout,
  buf: u32,
}
fn lookup_BigIntAddrLayout__super(b: BoundLayout_BigIntAddrLayout) -> BoundLayout_AddrDecomposeBitsLayout {
  return BoundLayout_AddrDecomposeBitsLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntAddrLayout__0(b: BoundLayout_BigIntAddrLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt._0, b.buf);
}
struct SplitU32Layout {
  low: SplitWordLayout,
  high: SplitWordLayout,
}
struct BoundLayout_SplitU32Layout {
  lyt: SplitU32Layout,
  buf: u32,
}
fn lookup_SplitU32Layout_low(b: BoundLayout_SplitU32Layout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.low, b.buf);
}
fn lookup_SplitU32Layout_high(b: BoundLayout_SplitU32Layout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.high, b.buf);
}
struct BigIntReadWords_SuperLayout {
  _super: SplitU32Layout,
  _0: MemoryReadLayout,
}
struct BoundLayout_BigIntReadWords_SuperLayout {
  lyt: BigIntReadWords_SuperLayout,
  buf: u32,
}
fn lookup_BigIntReadWords_SuperLayout__super(b: BoundLayout_BigIntReadWords_SuperLayout) -> BoundLayout_SplitU32Layout {
  return BoundLayout_SplitU32Layout(b.lyt._super, b.buf);
}
fn lookup_BigIntReadWords_SuperLayout__0(b: BoundLayout_BigIntReadWords_SuperLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt._0, b.buf);
}
alias BigIntReadWords_SuperLayout4LayoutArray = array<BigIntReadWords_SuperLayout, 4>;
struct BoundLayout_BigIntReadWords_SuperLayout4LayoutArray {
  lyt: BigIntReadWords_SuperLayout4LayoutArray,
  buf: u32,
}
fn subscript_BigIntReadWords_SuperLayout4LayoutArray(b: BoundLayout_BigIntReadWords_SuperLayout4LayoutArray, i: u32) -> BoundLayout_BigIntReadWords_SuperLayout {
  return BoundLayout_BigIntReadWords_SuperLayout(b.lyt[i], b.buf);
}
struct BigIntReadLayout {
  addr: BigIntAddrLayout,
  words: BigIntReadWords_SuperLayout4LayoutArray,
}
struct BoundLayout_BigIntReadLayout {
  lyt: BigIntReadLayout,
  buf: u32,
}
fn lookup_BigIntReadLayout_addr(b: BoundLayout_BigIntReadLayout) -> BoundLayout_BigIntAddrLayout {
  return BoundLayout_BigIntAddrLayout(b.lyt.addr, b.buf);
}
fn lookup_BigIntReadLayout_words(b: BoundLayout_BigIntReadLayout) -> BoundLayout_BigIntReadWords_SuperLayout4LayoutArray {
  return BoundLayout_BigIntReadWords_SuperLayout4LayoutArray(b.lyt.words, b.buf);
}
alias NondetU8RegLayout16LayoutArray = array<NondetU8RegLayout, 16>;
struct BoundLayout_NondetU8RegLayout16LayoutArray {
  lyt: NondetU8RegLayout16LayoutArray,
  buf: u32,
}
fn subscript_NondetU8RegLayout16LayoutArray(b: BoundLayout_NondetU8RegLayout16LayoutArray, i: u32) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt[i], b.buf);
}
struct BigIntWitnessLayout {
  _super: NondetU8RegLayout16LayoutArray,
}
struct BoundLayout_BigIntWitnessLayout {
  lyt: BigIntWitnessLayout,
  buf: u32,
}
fn lookup_BigIntWitnessLayout__super(b: BoundLayout_BigIntWitnessLayout) -> BoundLayout_NondetU8RegLayout16LayoutArray {
  return BoundLayout_NondetU8RegLayout16LayoutArray(b.lyt._super, b.buf);
}
struct BigIntWrite__0_SuperLayout {
  _0: MemoryWriteLayout,
}
struct BoundLayout_BigIntWrite__0_SuperLayout {
  lyt: BigIntWrite__0_SuperLayout,
  buf: u32,
}
fn lookup_BigIntWrite__0_SuperLayout__0(b: BoundLayout_BigIntWrite__0_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
alias BigIntWrite__0_SuperLayout4LayoutArray = array<BigIntWrite__0_SuperLayout, 4>;
struct BoundLayout_BigIntWrite__0_SuperLayout4LayoutArray {
  lyt: BigIntWrite__0_SuperLayout4LayoutArray,
  buf: u32,
}
fn subscript_BigIntWrite__0_SuperLayout4LayoutArray(b: BoundLayout_BigIntWrite__0_SuperLayout4LayoutArray, i: u32) -> BoundLayout_BigIntWrite__0_SuperLayout {
  return BoundLayout_BigIntWrite__0_SuperLayout(b.lyt[i], b.buf);
}
struct BigIntWriteLayout {
  _super: BigIntWitnessLayout,
  addr: BigIntAddrLayout,
  _1: BigIntWrite__0_SuperLayout4LayoutArray,
}
struct BoundLayout_BigIntWriteLayout {
  lyt: BigIntWriteLayout,
  buf: u32,
}
fn lookup_BigIntWriteLayout__super(b: BoundLayout_BigIntWriteLayout) -> BoundLayout_BigIntWitnessLayout {
  return BoundLayout_BigIntWitnessLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntWriteLayout_addr(b: BoundLayout_BigIntWriteLayout) -> BoundLayout_BigIntAddrLayout {
  return BoundLayout_BigIntAddrLayout(b.lyt.addr, b.buf);
}
fn lookup_BigIntWriteLayout__1(b: BoundLayout_BigIntWriteLayout) -> BoundLayout_BigIntWrite__0_SuperLayout4LayoutArray {
  return BoundLayout_BigIntWrite__0_SuperLayout4LayoutArray(b.lyt._1, b.buf);
}
struct BigIntStepBytesArm2Layout {
  _super: BigIntWitnessLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
}
struct BoundLayout_BigIntStepBytesArm2Layout {
  lyt: BigIntStepBytesArm2Layout,
  buf: u32,
}
fn lookup_BigIntStepBytesArm2Layout__super(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_BigIntWitnessLayout {
  return BoundLayout_BigIntWitnessLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra0(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra1(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra2(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra3(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra4(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra5(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra6(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra7(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra8(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra9(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra10(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra11(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra12(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra13(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
struct BigIntStepBytesLayout {
  arm0: BigIntReadLayout,
  arm1: BigIntWriteLayout,
  arm2: BigIntStepBytesArm2Layout,
}
struct BoundLayout_BigIntStepBytesLayout {
  lyt: BigIntStepBytesLayout,
  buf: u32,
}
fn lookup_BigIntStepBytesLayout_arm0(b: BoundLayout_BigIntStepBytesLayout) -> BoundLayout_BigIntReadLayout {
  return BoundLayout_BigIntReadLayout(b.lyt.arm0, b.buf);
}
fn lookup_BigIntStepBytesLayout_arm1(b: BoundLayout_BigIntStepBytesLayout) -> BoundLayout_BigIntWriteLayout {
  return BoundLayout_BigIntWriteLayout(b.lyt.arm1, b.buf);
}
fn lookup_BigIntStepBytesLayout_arm2(b: BoundLayout_BigIntStepBytesLayout) -> BoundLayout_BigIntStepBytesArm2Layout {
  return BoundLayout_BigIntStepBytesArm2Layout(b.lyt.arm2, b.buf);
}
struct BigIntStepLayout {
  _super: BigIntStateLayout,
  loadInst_0: MemoryReadLayout,
  instHigh: SplitWordLayout,
  polyOp: NondetRegLayout,
  memOp: NondetRegLayout,
  regBits: NondetRegLayout5LayoutArray,
  coeffBits: NondetRegLayout3LayoutArray,
  baseAddrU32: MemoryReadLayout,
  dataAddrU32: NormalizeU32Layout,
  memOpOneHot: OneHot_3_Layout,
  _arguments_BigIntStepBytes: _Arguments_BigIntStepBytesLayout,
  bytes: BigIntStepBytesLayout,
  _2: IsZeroLayout,
}
struct BoundLayout_BigIntStepLayout {
  lyt: BigIntStepLayout,
  buf: u32,
}
fn lookup_BigIntStepLayout__super(b: BoundLayout_BigIntStepLayout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntStepLayout_loadInst_0(b: BoundLayout_BigIntStepLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.loadInst_0, b.buf);
}
fn lookup_BigIntStepLayout_instHigh(b: BoundLayout_BigIntStepLayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.instHigh, b.buf);
}
fn lookup_BigIntStepLayout_polyOp(b: BoundLayout_BigIntStepLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.polyOp, b.buf);
}
fn lookup_BigIntStepLayout_memOp(b: BoundLayout_BigIntStepLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.memOp, b.buf);
}
fn lookup_BigIntStepLayout_regBits(b: BoundLayout_BigIntStepLayout) -> BoundLayout_NondetRegLayout5LayoutArray {
  return BoundLayout_NondetRegLayout5LayoutArray(b.lyt.regBits, b.buf);
}
fn lookup_BigIntStepLayout_coeffBits(b: BoundLayout_BigIntStepLayout) -> BoundLayout_NondetRegLayout3LayoutArray {
  return BoundLayout_NondetRegLayout3LayoutArray(b.lyt.coeffBits, b.buf);
}
fn lookup_BigIntStepLayout_baseAddrU32(b: BoundLayout_BigIntStepLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.baseAddrU32, b.buf);
}
fn lookup_BigIntStepLayout_dataAddrU32(b: BoundLayout_BigIntStepLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.dataAddrU32, b.buf);
}
fn lookup_BigIntStepLayout_memOpOneHot(b: BoundLayout_BigIntStepLayout) -> BoundLayout_OneHot_3_Layout {
  return BoundLayout_OneHot_3_Layout(b.lyt.memOpOneHot, b.buf);
}
fn lookup_BigIntStepLayout__arguments_BigIntStepBytes(b: BoundLayout_BigIntStepLayout) -> BoundLayout__Arguments_BigIntStepBytesLayout {
  return BoundLayout__Arguments_BigIntStepBytesLayout(b.lyt._arguments_BigIntStepBytes, b.buf);
}
fn lookup_BigIntStepLayout_bytes(b: BoundLayout_BigIntStepLayout) -> BoundLayout_BigIntStepBytesLayout {
  return BoundLayout_BigIntStepBytesLayout(b.lyt.bytes, b.buf);
}
fn lookup_BigIntStepLayout__2(b: BoundLayout_BigIntStepLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt._2, b.buf);
}
struct BigInt0StateArm2Layout {
  _super: BigIntStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm2Layout {
  lyt: BigInt0StateArm2Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm2Layout__super(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra0(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra1(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra2(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra3(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra4(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra5(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra6(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra7(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra8(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra9(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra10(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra11(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra12(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra13(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra14(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra15(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra16(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra17(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra18(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra19(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra20(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra21(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra22(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra23(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra24(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra25(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra26(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra27(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra28(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra29(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra30(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra31(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra32(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra33(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra34(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra35(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra36(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra37(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra38(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra39(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
struct BigInt0StateArm3Layout {
  _super: BigIntStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm3Layout {
  lyt: BigInt0StateArm3Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm3Layout__super(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra0(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra1(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra2(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra3(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra4(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra5(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra6(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra7(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra8(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra9(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra10(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra11(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra12(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra13(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra14(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra15(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra16(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra17(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra18(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra19(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra20(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra21(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra22(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra23(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra24(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra25(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra26(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra27(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra28(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra29(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra30(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra31(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra32(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra33(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra34(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra35(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra36(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra37(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra38(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra39(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
struct BigInt0StateArm4Layout {
  _super: BigIntStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm4Layout {
  lyt: BigInt0StateArm4Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm4Layout__super(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra0(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra1(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra2(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra3(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra4(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra5(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra6(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra7(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra8(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra9(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra10(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra11(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra12(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra13(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra14(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra15(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra16(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra17(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra18(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra19(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra20(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra21(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra22(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra23(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra24(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra25(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra26(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra27(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra28(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra29(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra30(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra31(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra32(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra33(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra34(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra35(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra36(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra37(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra38(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra39(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
struct BigInt0StateArm5Layout {
  _super: BigIntStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm5Layout {
  lyt: BigInt0StateArm5Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm5Layout__super(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra0(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra1(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra2(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra3(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra4(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra5(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra6(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra7(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra8(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra9(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra10(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra11(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra12(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra13(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra14(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra15(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra16(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra17(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra18(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra19(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra20(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra21(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra22(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra23(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra24(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra25(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra26(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra27(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra28(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra29(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra30(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra31(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra32(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra33(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra34(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra35(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra36(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra37(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra38(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra39(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
struct BigInt0StateArm6Layout {
  _super: BigIntStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm6Layout {
  lyt: BigInt0StateArm6Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm6Layout__super(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra0(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra1(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra2(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra3(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra4(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra5(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra6(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra7(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra8(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra9(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra10(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra11(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra12(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra13(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra14(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra15(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra16(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra17(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra18(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra19(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra20(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra21(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra22(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra23(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra24(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra25(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra26(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra27(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra28(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra29(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra30(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra31(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra32(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra33(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra34(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra35(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra36(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra37(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra38(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra39(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
struct BigInt0StateArm7Layout {
  _super: BigIntStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm7Layout {
  lyt: BigInt0StateArm7Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm7Layout__super(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra0(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra1(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra2(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra3(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra4(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra5(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra6(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra7(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra8(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra9(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra10(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra11(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra12(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra13(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra14(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra15(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra16(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra17(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra18(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra19(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra20(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra21(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra22(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra23(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra24(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra25(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra26(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra27(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra28(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra29(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra30(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra31(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra32(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra33(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra34(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra35(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra36(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra37(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra38(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra39(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
struct BigInt0StateLayout {
  _super: BigIntStateLayout,
  arm0: BigInt0StateArm0Layout,
  arm1: BigIntStepLayout,
  arm2: BigInt0StateArm2Layout,
  arm3: BigInt0StateArm3Layout,
  arm4: BigInt0StateArm4Layout,
  arm5: BigInt0StateArm5Layout,
  arm6: BigInt0StateArm6Layout,
  arm7: BigInt0StateArm7Layout,
}
struct BoundLayout_BigInt0StateLayout {
  lyt: BigInt0StateLayout,
  buf: u32,
}
fn lookup_BigInt0StateLayout__super(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateLayout_arm0(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm0Layout {
  return BoundLayout_BigInt0StateArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_BigInt0StateLayout_arm1(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigIntStepLayout {
  return BoundLayout_BigIntStepLayout(b.lyt.arm1, b.buf);
}
fn lookup_BigInt0StateLayout_arm2(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm2Layout {
  return BoundLayout_BigInt0StateArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_BigInt0StateLayout_arm3(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm3Layout {
  return BoundLayout_BigInt0StateArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_BigInt0StateLayout_arm4(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm4Layout {
  return BoundLayout_BigInt0StateArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_BigInt0StateLayout_arm5(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm5Layout {
  return BoundLayout_BigInt0StateArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_BigInt0StateLayout_arm6(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm6Layout {
  return BoundLayout_BigInt0StateArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_BigInt0StateLayout_arm7(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm7Layout {
  return BoundLayout_BigInt0StateArm7Layout(b.lyt.arm7, b.buf);
}
struct BigInt0Layout {
  _0: DoCycleTableLayout,
  state: BigIntStateLayout,
  _arguments_BigInt0State: _Arguments_BigInt0StateLayout,
  stateRedef: BigInt0StateLayout,
}
struct BoundLayout_BigInt0Layout {
  lyt: BigInt0Layout,
  buf: u32,
}
fn lookup_BigInt0Layout__0(b: BoundLayout_BigInt0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_BigInt0Layout_state(b: BoundLayout_BigInt0Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt.state, b.buf);
}
fn lookup_BigInt0Layout__arguments_BigInt0State(b: BoundLayout_BigInt0Layout) -> BoundLayout__Arguments_BigInt0StateLayout {
  return BoundLayout__Arguments_BigInt0StateLayout(b.lyt._arguments_BigInt0State, b.buf);
}
fn lookup_BigInt0Layout_stateRedef(b: BoundLayout_BigInt0Layout) -> BoundLayout_BigInt0StateLayout {
  return BoundLayout_BigInt0StateLayout(b.lyt.stateRedef, b.buf);
}
struct TopInstResultLayout {
  _selector: NondetRegLayout13LayoutArray,
  arm0: Misc0Layout,
  arm1: Misc1Layout,
  arm2: Misc2Layout,
  arm3: Mul0Layout,
  arm4: Div0Layout,
  arm5: Mem0Layout,
  arm6: Mem1Layout,
  arm7: Control0Layout,
  arm8: ECall0Layout,
  arm9: Poseidon0Layout,
  arm10: Poseidon1Layout,
  arm11: Sha0Layout,
  arm12: BigInt0Layout,
}
struct BoundLayout_TopInstResultLayout {
  lyt: TopInstResultLayout,
  buf: u32,
}
fn lookup_TopInstResultLayout__selector(b: BoundLayout_TopInstResultLayout) -> BoundLayout_NondetRegLayout13LayoutArray {
  return BoundLayout_NondetRegLayout13LayoutArray(b.lyt._selector, b.buf);
}
fn lookup_TopInstResultLayout_arm0(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Misc0Layout {
  return BoundLayout_Misc0Layout(b.lyt.arm0, b.buf);
}
fn lookup_TopInstResultLayout_arm1(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Misc1Layout {
  return BoundLayout_Misc1Layout(b.lyt.arm1, b.buf);
}
fn lookup_TopInstResultLayout_arm2(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Misc2Layout {
  return BoundLayout_Misc2Layout(b.lyt.arm2, b.buf);
}
fn lookup_TopInstResultLayout_arm3(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Mul0Layout {
  return BoundLayout_Mul0Layout(b.lyt.arm3, b.buf);
}
fn lookup_TopInstResultLayout_arm4(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Div0Layout {
  return BoundLayout_Div0Layout(b.lyt.arm4, b.buf);
}
fn lookup_TopInstResultLayout_arm5(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Mem0Layout {
  return BoundLayout_Mem0Layout(b.lyt.arm5, b.buf);
}
fn lookup_TopInstResultLayout_arm6(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Mem1Layout {
  return BoundLayout_Mem1Layout(b.lyt.arm6, b.buf);
}
fn lookup_TopInstResultLayout_arm7(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Control0Layout {
  return BoundLayout_Control0Layout(b.lyt.arm7, b.buf);
}
fn lookup_TopInstResultLayout_arm8(b: BoundLayout_TopInstResultLayout) -> BoundLayout_ECall0Layout {
  return BoundLayout_ECall0Layout(b.lyt.arm8, b.buf);
}
fn lookup_TopInstResultLayout_arm9(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Poseidon0Layout {
  return BoundLayout_Poseidon0Layout(b.lyt.arm9, b.buf);
}
fn lookup_TopInstResultLayout_arm10(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Poseidon1Layout {
  return BoundLayout_Poseidon1Layout(b.lyt.arm10, b.buf);
}
fn lookup_TopInstResultLayout_arm11(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Sha0Layout {
  return BoundLayout_Sha0Layout(b.lyt.arm11, b.buf);
}
fn lookup_TopInstResultLayout_arm12(b: BoundLayout_TopInstResultLayout) -> BoundLayout_BigInt0Layout {
  return BoundLayout_BigInt0Layout(b.lyt.arm12, b.buf);
}
struct TopCycleLayout {
  _super: NondetRegLayout,
  arm0: NondetRegLayout,
  arm1: NondetRegLayout,
}
struct BoundLayout_TopCycleLayout {
  lyt: TopCycleLayout,
  buf: u32,
}
fn lookup_TopCycleLayout__super(b: BoundLayout_TopCycleLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._super, b.buf);
}
fn lookup_TopCycleLayout_arm0(b: BoundLayout_TopCycleLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.arm0, b.buf);
}
fn lookup_TopCycleLayout_arm1(b: BoundLayout_TopCycleLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.arm1, b.buf);
}
struct TopLayout {
  cycle: NondetRegLayout,
  nextPcLow: NondetRegLayout,
  nextPcHigh: NondetRegLayout,
  nextState_0: NondetRegLayout,
  nextMachineMode: NondetRegLayout,
  isFirstCycle: NondetRegLayout,
  cycleRedef: TopCycleLayout,
  major: NondetRegLayout,
  minor: NondetRegLayout,
  instInput: InstInputLayout,
  majorOnehot: OneHot_13_Layout,
  instResult: TopInstResultLayout,
}
struct BoundLayout_TopLayout {
  lyt: TopLayout,
  buf: u32,
}
fn lookup_TopLayout_cycle(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.cycle, b.buf);
}
fn lookup_TopLayout_nextPcLow(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextPcLow, b.buf);
}
fn lookup_TopLayout_nextPcHigh(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextPcHigh, b.buf);
}
fn lookup_TopLayout_nextState_0(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextState_0, b.buf);
}
fn lookup_TopLayout_nextMachineMode(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextMachineMode, b.buf);
}
fn lookup_TopLayout_isFirstCycle(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isFirstCycle, b.buf);
}
fn lookup_TopLayout_cycleRedef(b: BoundLayout_TopLayout) -> BoundLayout_TopCycleLayout {
  return BoundLayout_TopCycleLayout(b.lyt.cycleRedef, b.buf);
}
fn lookup_TopLayout_major(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.major, b.buf);
}
fn lookup_TopLayout_minor(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.minor, b.buf);
}
fn lookup_TopLayout_instInput(b: BoundLayout_TopLayout) -> BoundLayout_InstInputLayout {
  return BoundLayout_InstInputLayout(b.lyt.instInput, b.buf);
}
fn lookup_TopLayout_majorOnehot(b: BoundLayout_TopLayout) -> BoundLayout_OneHot_13_Layout {
  return BoundLayout_OneHot_13_Layout(b.lyt.majorOnehot, b.buf);
}
fn lookup_TopLayout_instResult(b: BoundLayout_TopLayout) -> BoundLayout_TopInstResultLayout {
  return BoundLayout_TopInstResultLayout(b.lyt.instResult, b.buf);
}
struct DigestRegValues_SuperLayout {
  low: NondetRegLayout,
  high: NondetRegLayout,
}
struct BoundLayout_DigestRegValues_SuperLayout {
  lyt: DigestRegValues_SuperLayout,
  buf: u32,
}
fn lookup_DigestRegValues_SuperLayout_low(b: BoundLayout_DigestRegValues_SuperLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low, b.buf);
}
fn lookup_DigestRegValues_SuperLayout_high(b: BoundLayout_DigestRegValues_SuperLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.high, b.buf);
}
alias DigestRegValues_SuperLayout8LayoutArray = array<DigestRegValues_SuperLayout, 8>;
struct BoundLayout_DigestRegValues_SuperLayout8LayoutArray {
  lyt: DigestRegValues_SuperLayout8LayoutArray,
  buf: u32,
}
fn subscript_DigestRegValues_SuperLayout8LayoutArray(b: BoundLayout_DigestRegValues_SuperLayout8LayoutArray, i: u32) -> BoundLayout_DigestRegValues_SuperLayout {
  return BoundLayout_DigestRegValues_SuperLayout(b.lyt[i], b.buf);
}
struct DigestRegLayout {
  values: DigestRegValues_SuperLayout8LayoutArray,
}
struct BoundLayout_DigestRegLayout {
  lyt: DigestRegLayout,
  buf: u32,
}
fn lookup_DigestRegLayout_values(b: BoundLayout_DigestRegLayout) -> BoundLayout_DigestRegValues_SuperLayout8LayoutArray {
  return BoundLayout_DigestRegValues_SuperLayout8LayoutArray(b.lyt.values, b.buf);
}
struct Arg_ArgU8Layout {
  val: Reg,
}
struct BoundLayout_Arg_ArgU8Layout {
  lyt: Arg_ArgU8Layout,
  buf: u32,
}
fn lookup_Arg_ArgU8Layout_val(b: BoundLayout_Arg_ArgU8Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.val, b.buf);
}
struct Arg_ArgU16Layout {
  val: Reg,
}
struct BoundLayout_Arg_ArgU16Layout {
  lyt: Arg_ArgU16Layout,
  buf: u32,
}
fn lookup_Arg_ArgU16Layout_val(b: BoundLayout_Arg_ArgU16Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.val, b.buf);
}
struct Arg_MemoryArgLayout {
  addr: Reg,
  cycle: Reg,
  dataLow: Reg,
  dataHigh: Reg,
}
struct BoundLayout_Arg_MemoryArgLayout {
  lyt: Arg_MemoryArgLayout,
  buf: u32,
}
fn lookup_Arg_MemoryArgLayout_addr(b: BoundLayout_Arg_MemoryArgLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.addr, b.buf);
}
fn lookup_Arg_MemoryArgLayout_cycle(b: BoundLayout_Arg_MemoryArgLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.cycle, b.buf);
}
fn lookup_Arg_MemoryArgLayout_dataLow(b: BoundLayout_Arg_MemoryArgLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.dataLow, b.buf);
}
fn lookup_Arg_MemoryArgLayout_dataHigh(b: BoundLayout_Arg_MemoryArgLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.dataHigh, b.buf);
}
struct Arg_CycleArgLayout {
  cycle: Reg,
}
struct BoundLayout_Arg_CycleArgLayout {
  lyt: Arg_CycleArgLayout,
  buf: u32,
}
fn lookup_Arg_CycleArgLayout_cycle(b: BoundLayout_Arg_CycleArgLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.cycle, b.buf);
}
alias Reg1LayoutArray = array<Reg, 1>;
struct BoundLayout_Reg1LayoutArray {
  lyt: Reg1LayoutArray,
  buf: u32,
}
fn subscript_Reg1LayoutArray(b: BoundLayout_Reg1LayoutArray, i: u32) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt[i], b.buf);
}
struct _accumLayout {
  argU8: Arg_ArgU8Layout,
  argU16: Arg_ArgU16Layout,
  memoryArg: Arg_MemoryArgLayout,
  cycleArg: Arg_CycleArgLayout,
  _offset: Reg,
  _user: Reg1LayoutArray,
}
struct BoundLayout__accumLayout {
  lyt: _accumLayout,
  buf: u32,
}
fn lookup__accumLayout_argU8(b: BoundLayout__accumLayout) -> BoundLayout_Arg_ArgU8Layout {
  return BoundLayout_Arg_ArgU8Layout(b.lyt.argU8, b.buf);
}
fn lookup__accumLayout_argU16(b: BoundLayout__accumLayout) -> BoundLayout_Arg_ArgU16Layout {
  return BoundLayout_Arg_ArgU16Layout(b.lyt.argU16, b.buf);
}
fn lookup__accumLayout_memoryArg(b: BoundLayout__accumLayout) -> BoundLayout_Arg_MemoryArgLayout {
  return BoundLayout_Arg_MemoryArgLayout(b.lyt.memoryArg, b.buf);
}
fn lookup__accumLayout_cycleArg(b: BoundLayout__accumLayout) -> BoundLayout_Arg_CycleArgLayout {
  return BoundLayout_Arg_CycleArgLayout(b.lyt.cycleArg, b.buf);
}
fn lookup__accumLayout__offset(b: BoundLayout__accumLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt._offset, b.buf);
}
fn lookup__accumLayout__user(b: BoundLayout__accumLayout) -> BoundLayout_Reg1LayoutArray {
  return BoundLayout_Reg1LayoutArray(b.lyt._user, b.buf);
}
alias Reg20LayoutArray = array<Reg, 20>;
struct BoundLayout_Reg20LayoutArray {
  lyt: Reg20LayoutArray,
  buf: u32,
}
fn subscript_Reg20LayoutArray(b: BoundLayout_Reg20LayoutArray, i: u32) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt[i], b.buf);
}
struct LayoutAccumLayout {
  user: AccumLayout,
  columns: Reg20LayoutArray,
}
struct BoundLayout_LayoutAccumLayout {
  lyt: LayoutAccumLayout,
  buf: u32,
}
fn lookup_LayoutAccumLayout_user(b: BoundLayout_LayoutAccumLayout) -> BoundLayout_AccumLayout {
  return BoundLayout_AccumLayout(b.lyt.user, b.buf);
}
fn lookup_LayoutAccumLayout_columns(b: BoundLayout_LayoutAccumLayout) -> BoundLayout_Reg20LayoutArray {
  return BoundLayout_Reg20LayoutArray(b.lyt.columns, b.buf);
}
struct TestSuccRunLayout {
  _0: TopLayout,
}
struct BoundLayout_TestSuccRunLayout {
  lyt: TestSuccRunLayout,
  buf: u32,
}
fn lookup_TestSuccRunLayout__0(b: BoundLayout_TestSuccRunLayout) -> BoundLayout_TopLayout {
  return BoundLayout_TopLayout(b.lyt._0, b.buf);
}
struct _globalLayout {
  input: DigestRegLayout,
  isTerminate: NondetRegLayout,
  output: DigestRegLayout,
  povwNonce: DigestRegLayout,
  rng: NondetExtRegLayout,
  shutdownCycle: NondetRegLayout,
  stateIn: DigestRegLayout,
  stateOut: DigestRegLayout,
  termA0high: NondetRegLayout,
  termA0low: NondetRegLayout,
  termA1high: NondetRegLayout,
  termA1low: NondetRegLayout,
}
struct BoundLayout__globalLayout {
  lyt: _globalLayout,
  buf: u32,
}
fn lookup__globalLayout_input(b: BoundLayout__globalLayout) -> BoundLayout_DigestRegLayout {
  return BoundLayout_DigestRegLayout(b.lyt.input, b.buf);
}
fn lookup__globalLayout_isTerminate(b: BoundLayout__globalLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isTerminate, b.buf);
}
fn lookup__globalLayout_output(b: BoundLayout__globalLayout) -> BoundLayout_DigestRegLayout {
  return BoundLayout_DigestRegLayout(b.lyt.output, b.buf);
}
fn lookup__globalLayout_povwNonce(b: BoundLayout__globalLayout) -> BoundLayout_DigestRegLayout {
  return BoundLayout_DigestRegLayout(b.lyt.povwNonce, b.buf);
}
fn lookup__globalLayout_rng(b: BoundLayout__globalLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.rng, b.buf);
}
fn lookup__globalLayout_shutdownCycle(b: BoundLayout__globalLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.shutdownCycle, b.buf);
}
fn lookup__globalLayout_stateIn(b: BoundLayout__globalLayout) -> BoundLayout_DigestRegLayout {
  return BoundLayout_DigestRegLayout(b.lyt.stateIn, b.buf);
}
fn lookup__globalLayout_stateOut(b: BoundLayout__globalLayout) -> BoundLayout_DigestRegLayout {
  return BoundLayout_DigestRegLayout(b.lyt.stateOut, b.buf);
}
fn lookup__globalLayout_termA0high(b: BoundLayout__globalLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.termA0high, b.buf);
}
fn lookup__globalLayout_termA0low(b: BoundLayout__globalLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.termA0low, b.buf);
}
fn lookup__globalLayout_termA1high(b: BoundLayout__globalLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.termA1high, b.buf);
}
fn lookup__globalLayout_termA1low(b: BoundLayout__globalLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.termA1low, b.buf);
}
struct _mixLayout {
  randomness: _accumLayout,
}
struct BoundLayout__mixLayout {
  lyt: _mixLayout,
  buf: u32,
}
fn lookup__mixLayout_randomness(b: BoundLayout__mixLayout) -> BoundLayout__accumLayout {
  return BoundLayout__accumLayout(b.lyt.randomness, b.buf);
}
struct NondetRegStruct {
  _super: Val,
}
struct NondetExtRegStruct {
  _super: ExtVal,
}
struct NondetFakeTwitRegStruct {
  _super: Val,
}
struct FakeTwitRegStruct {
  _unused: u32,
}
struct ArgU8Struct {
  count: NondetRegStruct,
  val: NondetRegStruct,
}
struct U8RegStruct {
  _unused: u32,
}
struct ArgU16Struct {
  count: NondetRegStruct,
  val: NondetRegStruct,
}
struct NondetU16RegStruct {
  _super: NondetRegStruct,
}
alias Val5Array = array<Val, 5>;
alias Val30Array = array<Val, 30>;
alias NondetRegStruct5Array = array<NondetRegStruct, 5>;
struct ToBits_5_Struct {
  _super: NondetRegStruct5Array,
}
struct ValU32Struct {
  low: Val,
  high: Val,
}
struct DenormedValU32Struct {
  low: Val,
  high: Val,
}
struct NormalizeU32Struct {
  _super: ValU32Struct,
  highCarry: NondetRegStruct,
}
struct AddrDecomposeStruct {
  _super: Val,
  low2: NondetRegStruct,
}
struct AddrDecomposeBitsStruct {
  _super: Val,
  low0: NondetRegStruct,
  low1: NondetRegStruct,
  low2: Val,
}
struct CmpEqualStruct {
  isEqual: NondetRegStruct,
}
struct CmpLessThanUnsignedStruct {
  isLessThan: Val,
}
struct CmpLessThanStruct {
  isLessThan: NondetRegStruct,
}
alias Val16Array = array<Val, 16>;
alias NondetRegStruct16Array = array<NondetRegStruct, 16>;
struct ToBits_16_Struct {
  _super: NondetRegStruct16Array,
}
struct FromBits_16_Struct {
  _super: Val,
}
struct DecoderStruct {
  opcode: NondetRegStruct,
  rs1: Val,
  rs2: Val,
  rd: Val,
  func7: Val,
  func3: Val,
  immI: ValU32Struct,
  immS: ValU32Struct,
  immB: ValU32Struct,
  immU: ValU32Struct,
  immJ: ValU32Struct,
}
struct MemoryArgStruct {
  count: NondetRegStruct,
  addr: NondetRegStruct,
  cycle: NondetRegStruct,
  dataLow: NondetRegStruct,
  dataHigh: NondetRegStruct,
}
struct CycleArgStruct {
  count: NondetRegStruct,
  cycle: NondetRegStruct,
}
struct IsCycleStruct {
  _unused: u32,
}
struct MemoryIOStruct {
  oldTxn: MemoryArgStruct,
  newTxn: MemoryArgStruct,
}
struct IsForwardStruct {
  _unused: u32,
}
struct GetDataStruct {
  _super: ValU32Struct,
  diffLow: Val,
  diffHigh: Val,
}
struct MemoryWriteStruct {
  _unused: u32,
}
struct MemoryWriteUnconstrainedStruct {
  io: MemoryIOStruct,
}
alias Val3Array = array<Val, 3>;
alias NondetRegStruct3Array = array<NondetRegStruct, 3>;
struct OneHot_3_Struct {
  _super: NondetRegStruct3Array,
}
alias Val8Array = array<Val, 8>;
alias NondetRegStruct8Array = array<NondetRegStruct, 8>;
struct OneHot_8_Struct {
  _super: NondetRegStruct8Array,
}
struct InstInputStruct {
  minor: Val,
  pcU32: ValU32Struct,
  state: Val,
  mode: Val,
  minorOnehot: OneHot_8_Struct,
}
struct DoCycleTableStruct {
  _unused: u32,
}
struct SourceRegsStruct {
  rs1: ValU32Struct,
  rs2: ValU32Struct,
}
struct ReadSourceRegsStruct {
  rs1: ValU32Struct,
  rs2: ValU32Struct,
}
struct WriteRdStruct {
  _unused: u32,
}
struct ExpandU32Struct {
  b0: NondetRegStruct,
  b1: NondetRegStruct,
  b2: NondetRegStruct,
  b3: NondetRegStruct,
  neg: Val,
}
struct SplitTotalStruct {
  out_: NondetU16RegStruct,
  carry: Val,
}
struct MultiplySettingsStruct {
  aSigned: Val,
  bSigned: Val,
  cSigned: Val,
}
struct MultiplyAccumulateStruct {
  outLow: ValU32Struct,
  outHigh: ValU32Struct,
  bNeg: Val,
}
struct DivInputStruct {
  _super: InstInputStruct,
  decoded: DecoderStruct,
  rs1: ValU32Struct,
  rs2: ValU32Struct,
}
struct ComponentStruct {
  _unused: u32,
}
struct DivideReturnStruct {
  quot: ValU32Struct,
  rem: ValU32Struct,
}
struct BigIntTopStateStruct {
  polyOp: Val,
  coeff: Val,
  witness: Val16Array,
}
struct InstOutputBaseStruct {
  newPc: ValU32Struct,
  newState: Val,
  newMode: Val,
  topState: BigIntTopStateStruct,
}
struct MiscInputStruct {
  _super: InstInputStruct,
  decoded: DecoderStruct,
  rs1: ValU32Struct,
  rs2: ValU32Struct,
}
struct MiscOutputStruct {
  doWrite: Val,
  toWrite: DenormedValU32Struct,
  newPc: DenormedValU32Struct,
}
struct MulInputStruct {
  _super: InstInputStruct,
  decoded: DecoderStruct,
  rs1: ValU32Struct,
  rs2: ValU32Struct,
}
struct DoMulStruct {
  low: ValU32Struct,
  high: ValU32Struct,
}
struct MemLoadInputStruct {
  ii: InstInputStruct,
  decoded: DecoderStruct,
  addr: AddrDecomposeBitsStruct,
  data: GetDataStruct,
}
struct MemStoreInputStruct {
  decoded: DecoderStruct,
  rs2: ValU32Struct,
  addr: AddrDecomposeBitsStruct,
  data: GetDataStruct,
}
struct MemStoreFinalizeStruct {
  _unused: u32,
}
struct SplitWordStruct {
  byte0: NondetRegStruct,
  byte1: NondetRegStruct,
}
struct DigestRegValues_SuperStruct {
  low: NondetRegStruct,
  high: NondetRegStruct,
}
alias DigestRegValues_SuperStruct8Array = array<DigestRegValues_SuperStruct, 8>;
struct DigestRegStruct {
  values: DigestRegValues_SuperStruct8Array,
}
alias ValU32Struct8Array = array<ValU32Struct, 8>;
alias GetDataStruct8Array = array<GetDataStruct, 8>;
struct ControlResume_SuperArm1_Super__0Struct {
  _unused: u32,
}
alias ControlResume_SuperArm1_Super__0Struct8Array = array<ControlResume_SuperArm1_Super__0Struct, 8>;
struct ControlTable_SuperArm0_Super__0Struct {
  _unused: u32,
}
struct ControlTable_SuperArm1_Super__0Struct {
  _unused: u32,
}
alias ControlTable_SuperArm0_Super__0Struct16Array = array<ControlTable_SuperArm0_Super__0Struct, 16>;
alias ControlTable_SuperArm1_Super__0Struct16Array = array<ControlTable_SuperArm1_Super__0Struct, 16>;
alias Val6Array = array<Val, 6>;
alias NondetRegStruct6Array = array<NondetRegStruct, 6>;
struct OneHot_6_Struct {
  _super: NondetRegStruct6Array,
}
struct ECallOutputStruct {
  state: Val,
  s0: Val,
  s1: Val,
  s2: Val,
}
alias Val4Array = array<Val, 4>;
alias NondetRegStruct4Array = array<NondetRegStruct, 4>;
struct OneHot_4_Struct {
  _super: NondetRegStruct4Array,
}
struct DecomposeLow2Struct {
  high: NondetU16RegStruct,
  low2: NondetRegStruct,
  low2Hot: OneHot_4_Struct,
  highZero: NondetRegStruct,
  isZero: NondetRegStruct,
  low2Nonzero: Val,
}
struct ECallHostReadWords__0Struct {
  _unused: u32,
}
alias ECallHostReadWords__0Struct4Array = array<ECallHostReadWords__0Struct, 4>;
alias Val24Array = array<Val, 24>;
struct MultiplyByMInt_Super_SuperStruct {
  _super: Val,
}
alias MultiplyByMInt_Super_SuperStruct24Array = array<MultiplyByMInt_Super_SuperStruct, 24>;
struct MultiplyByMIntStruct {
  _super: MultiplyByMInt_Super_SuperStruct24Array,
}
struct DoIntRounds__0_SuperStruct {
  _super: Val,
}
alias DoIntRounds__0_SuperStruct21Array = array<DoIntRounds__0_SuperStruct, 21>;
struct DoIntRoundsStruct {
  _super: Val24Array,
}
alias NondetRegStruct24Array = array<NondetRegStruct, 24>;
struct MultiplyByMExt_Super_SuperStruct {
  _super: Val,
}
alias MultiplyByMExt_Super_SuperStruct24Array = array<MultiplyByMExt_Super_SuperStruct, 24>;
struct MultiplyByMExtStruct {
  _super: MultiplyByMExt_Super_SuperStruct24Array,
}
struct PoseidonStateStruct {
  hasState: NondetRegStruct,
  stateAddr: NondetRegStruct,
  bufOutAddr: NondetRegStruct,
  isElem: NondetRegStruct,
  checkOut: NondetRegStruct,
  loadTxType: NondetRegStruct,
  nextState: NondetRegStruct,
  subState: NondetRegStruct,
  bufInAddr: NondetRegStruct,
  count: NondetRegStruct,
  mode: NondetRegStruct,
  inner: NondetRegStruct24Array,
  zcheck: NondetExtRegStruct,
}
struct PoseidonOpDefStruct {
  hasState: Val,
  stateAddr: Val,
  bufOutAddr: Val,
  isElem: Val,
  checkOut: Val,
  loadTxType: Val,
}
struct ReadAddrStruct {
  _super: Val,
}
struct ReadElemStruct {
  _super: Val,
}
alias ReadElemStruct8Array = array<ReadElemStruct, 8>;
struct PoseidonCheckOut__0Struct {
  _unused: u32,
}
alias PoseidonCheckOut__0Struct8Array = array<PoseidonCheckOut__0Struct, 8>;
struct FieldToWordStruct {
  ret: ValU32Struct,
}
struct PoseidonStoreOut__0Struct {
  _unused: u32,
}
alias PoseidonStoreOut__0Struct8Array = array<PoseidonStoreOut__0Struct, 8>;
struct PoseidonStoreState__0Struct {
  _unused: u32,
}
alias PoseidonStoreState__0Struct8Array = array<PoseidonStoreState__0Struct, 8>;
struct IsU24Struct {
  _unused: u32,
}
struct CarryExtractStruct {
  carry: Val,
  out_: Val,
}
alias Val2Array = array<Val, 2>;
struct DivStruct {
  _super: Val,
}
alias DivStruct32Array = array<DivStruct, 32>;
alias Val32Array = array<Val, 32>;
alias NondetRegStruct32Array = array<NondetRegStruct, 32>;
struct UnpackReg_32__16_Struct {
  _super: NondetRegStruct32Array,
}
struct ShaStateAStruct {
  _super: NondetRegStruct,
}
alias ShaStateAStruct32Array = array<ShaStateAStruct, 32>;
struct ShaStateEStruct {
  _super: NondetRegStruct,
}
alias ShaStateEStruct32Array = array<ShaStateEStruct, 32>;
struct ShaStateWStruct {
  _super: NondetRegStruct,
}
alias ShaStateWStruct32Array = array<ShaStateWStruct, 32>;
struct ShaStateStruct {
  stateInAddr: NondetRegStruct,
  stateOutAddr: NondetRegStruct,
  dataAddr: NondetRegStruct,
  count: NondetRegStruct,
  kAddr: NondetRegStruct,
  round: NondetRegStruct,
  nextState: NondetRegStruct,
  a: ShaStateAStruct32Array,
  e: ShaStateEStruct32Array,
  w: ShaStateWStruct32Array,
}
struct BigIntStateStruct {
  isEcall: NondetRegStruct,
  mode: NondetRegStruct,
  pc: NondetRegStruct,
  polyOp: NondetRegStruct,
  coeff: NondetRegStruct,
  bytes: NondetRegStruct16Array,
  nextState: NondetRegStruct,
}
struct SplitU32Struct {
  bytes: NondetRegStruct4Array,
}
alias SplitU32Struct4Array = array<SplitU32Struct, 4>;
struct BigIntReadStruct {
  _super: NondetRegStruct16Array,
}
struct BigIntWitnessStruct {
  _super: NondetRegStruct16Array,
}
struct BigIntWrite__0Struct {
  _unused: u32,
}
alias BigIntWrite__0Struct4Array = array<BigIntWrite__0Struct, 4>;
struct BigIntAccumStateStruct {
  poly: NondetExtRegStruct,
  term: NondetExtRegStruct,
  total: NondetExtRegStruct,
}
alias Val7Array = array<Val, 7>;
alias NondetRegStruct7Array = array<NondetRegStruct, 7>;
struct OneHot_7_Struct {
  _super: NondetRegStruct7Array,
}
alias ExtVal1Array = array<ExtVal, 1>;
struct BigIntAccumStruct {
  _unused: u32,
}
alias Val13Array = array<Val, 13>;
alias NondetRegStruct13Array = array<NondetRegStruct, 13>;
struct OneHot_13_Struct {
  _super: NondetRegStruct13Array,
}
struct TopStruct {
  _unused: u32,
}
struct AccumStruct {
  _unused: u32,
}
const kLayout__3: NondetRegLayout7LayoutArray = NondetRegLayout7LayoutArray(NondetRegLayout(Reg(12u)), NondetRegLayout(Reg(13u)), NondetRegLayout(Reg(14u)), NondetRegLayout(Reg(15u)), NondetRegLayout(Reg(16u)), NondetRegLayout(Reg(17u)), NondetRegLayout(Reg(18u)));
const kLayout__2: OneHot_7_Layout = OneHot_7_Layout(kLayout__3);
const kLayout__4: BigIntAccumStateLayout = BigIntAccumStateLayout(NondetExtRegLayout(Reg(0u)), NondetExtRegLayout(Reg(4u)), NondetExtRegLayout(Reg(8u)));
const kLayout__6: BigIntPolyOpAddTotalLayout = BigIntPolyOpAddTotalLayout(kLayout__4, NondetExtRegLayout(Reg(19u)));
const kLayout__5: BigIntAccumStateLayout_0 = BigIntAccumStateLayout_0(kLayout__4, kLayout__4, kLayout__4, kLayout__4, kLayout__6, kLayout__4, kLayout__4, kLayout__4);
const kLayout__1: BigIntAccumLayout = BigIntAccumLayout(kLayout__2, kLayout__4, kLayout__5);
const kLayout__0: AccumLayout = AccumLayout(kLayout__1);
const kLayout__10: NondetRegLayout8LayoutArray = NondetRegLayout8LayoutArray(NondetRegLayout(Reg(21u)), NondetRegLayout(Reg(22u)), NondetRegLayout(Reg(23u)), NondetRegLayout(Reg(24u)), NondetRegLayout(Reg(25u)), NondetRegLayout(Reg(26u)), NondetRegLayout(Reg(27u)), NondetRegLayout(Reg(28u)));
const kLayout__9: OneHot_8_Layout = OneHot_8_Layout(kLayout__10);
const kLayout__8: InstInputLayout = InstInputLayout(kLayout__9);
const kLayout__12: NondetRegLayout13LayoutArray = NondetRegLayout13LayoutArray(NondetRegLayout(Reg(1u)), NondetRegLayout(Reg(2u)), NondetRegLayout(Reg(3u)), NondetRegLayout(Reg(4u)), NondetRegLayout(Reg(5u)), NondetRegLayout(Reg(6u)), NondetRegLayout(Reg(7u)), NondetRegLayout(Reg(8u)), NondetRegLayout(Reg(9u)), NondetRegLayout(Reg(10u)), NondetRegLayout(Reg(11u)), NondetRegLayout(Reg(12u)), NondetRegLayout(Reg(13u)));
const kLayout__11: OneHot_13_Layout = OneHot_13_Layout(kLayout__12);
const kLayout__17: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u))));
const kLayout__18: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(42u)), NondetRegLayout(Reg(43u))));
const kLayout__16: NormalizeU32Layout = NormalizeU32Layout(kLayout__17, NondetRegLayout(Reg(41u)), kLayout__18, NondetRegLayout(Reg(44u)));
const kLayout__20: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(45u)), NondetRegLayout(Reg(46u))));
const kLayout__21: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(48u)), NondetRegLayout(Reg(49u))));
const kLayout__19: NormalizeU32Layout = NormalizeU32Layout(kLayout__20, NondetRegLayout(Reg(47u)), kLayout__21, NondetRegLayout(Reg(50u)));
const kLayout__25: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(55u)), NondetRegLayout(Reg(54u)), NondetRegLayout(Reg(56u)), NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u)));
const kLayout__26: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(54u)), NondetRegLayout(Reg(60u)), NondetRegLayout(Reg(61u)), NondetRegLayout(Reg(62u)));
const kLayout__24: MemoryIOLayout = MemoryIOLayout(kLayout__25, kLayout__26);
const kLayout__28: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(63u)), NondetRegLayout(Reg(64u))));
const kLayout__27: IsForwardLayout = IsForwardLayout(kLayout__28);
const kLayout__23: MemoryWriteLayout = MemoryWriteLayout(kLayout__24, kLayout__27);
const kLayout__22: WriteRdLayout = WriteRdLayout(IsZeroLayout(NondetRegLayout(Reg(51u)), NondetRegLayout(Reg(52u))), NondetRegLayout(Reg(53u)), kLayout__23);
const kLayout__15: FinalizeMiscLayout = FinalizeMiscLayout(kLayout__16, kLayout__19, kLayout__22);
const kLayout__29: DoCycleTableLayout = DoCycleTableLayout(CycleArgLayout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(66u))), CycleArgLayout(NondetRegLayout(Reg(67u)), NondetRegLayout(Reg(68u))));
const kLayout__32: DecoderLayout = DecoderLayout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u)), NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u)), NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(74u)), NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u)), NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u)), NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u)), NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u)), NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u)));
const kLayout__34: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(86u)), NondetRegLayout(Reg(87u))));
const kLayout__35: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(90u)), NondetRegLayout(Reg(91u))));
const kLayout__33: AddrDecomposeLayout = AddrDecomposeLayout(NondetRegLayout(Reg(85u)), kLayout__34, IsZeroLayout(NondetRegLayout(Reg(88u)), NondetRegLayout(Reg(89u))), kLayout__35);
const kLayout__38: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(93u)), NondetRegLayout(Reg(92u)), NondetRegLayout(Reg(94u)), NondetRegLayout(Reg(95u)), NondetRegLayout(Reg(96u)));
const kLayout__39: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(97u)), NondetRegLayout(Reg(92u)), NondetRegLayout(Reg(98u)), NondetRegLayout(Reg(99u)), NondetRegLayout(Reg(100u)));
const kLayout__37: MemoryIOLayout = MemoryIOLayout(kLayout__38, kLayout__39);
const kLayout__41: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(102u))));
const kLayout__40: IsForwardLayout = IsForwardLayout(kLayout__41);
const kLayout__36: MemoryReadLayout = MemoryReadLayout(kLayout__37, kLayout__40);
const kLayout__31: DecodeInstLayout = DecodeInstLayout(kLayout__32, kLayout__33, kLayout__36);
const kLayout__45: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(103u)), NondetRegLayout(Reg(104u)), NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u)), NondetRegLayout(Reg(107u)));
const kLayout__46: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(108u)), NondetRegLayout(Reg(104u)), NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u)), NondetRegLayout(Reg(111u)));
const kLayout__47: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(112u)), NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u)), NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u)));
const kLayout__48: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(118u)), NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u)));
const kLayout__44: MemoryArgLayout4LayoutArray = MemoryArgLayout4LayoutArray(kLayout__45, kLayout__46, kLayout__47, kLayout__48);
const kLayout__49: CycleArgLayout2LayoutArray = CycleArgLayout2LayoutArray(CycleArgLayout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), CycleArgLayout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))));
const kLayout__43: _Arguments_ReadSourceRegsSourceRegsLayout = _Arguments_ReadSourceRegsSourceRegsLayout(kLayout__44, kLayout__49);
const kLayout__55: MemoryIOLayout = MemoryIOLayout(kLayout__45, kLayout__46);
const kLayout__57: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))));
const kLayout__56: IsForwardLayout = IsForwardLayout(kLayout__57);
const kLayout__54: MemoryReadLayout = MemoryReadLayout(kLayout__55, kLayout__56);
const kLayout__53: ReadRegLayout = ReadRegLayout(kLayout__54, NondetRegLayout(Reg(126u)));
const kLayout__52: ReadSourceRegsSourceRegsArm0_SuperLayout = ReadSourceRegsSourceRegsArm0_SuperLayout(kLayout__53);
const kLayout__51: ReadSourceRegsSourceRegsArm0Layout = ReadSourceRegsSourceRegsArm0Layout(kLayout__52, kLayout__47, kLayout__48, CycleArgLayout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))));
const kLayout__61: MemoryIOLayout = MemoryIOLayout(kLayout__47, kLayout__48);
const kLayout__63: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))));
const kLayout__62: IsForwardLayout = IsForwardLayout(kLayout__63);
const kLayout__60: MemoryReadLayout = MemoryReadLayout(kLayout__61, kLayout__62);
const kLayout__59: ReadRegLayout = ReadRegLayout(kLayout__60, NondetRegLayout(Reg(127u)));
const kLayout__58: ReadSourceRegsSourceRegsArm1_SuperLayout = ReadSourceRegsSourceRegsArm1_SuperLayout(kLayout__53, kLayout__59);
const kLayout__50: ReadSourceRegsSourceRegsLayout = ReadSourceRegsSourceRegsLayout(kLayout__51, kLayout__58);
const kLayout__42: ReadSourceRegsLayout = ReadSourceRegsLayout(NondetRegLayout(Reg(125u)), kLayout__43, kLayout__50, NondetRegLayout(Reg(128u)), NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u)), NondetRegLayout(Reg(131u)));
const kLayout__30: MiscInputLayout = MiscInputLayout(kLayout__31, kLayout__42);
const kLayout__65: ArgU16Layout5LayoutArray = ArgU16Layout5LayoutArray(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__64: _Arguments_Misc0MiscOutputLayout = _Arguments_Misc0MiscOutputLayout(kLayout__65);
const kLayout__67: Misc0MiscOutputArm0Layout = Misc0MiscOutputArm0Layout(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__68: Misc0MiscOutputArm1Layout = Misc0MiscOutputArm1Layout(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__75: NondetRegLayout16LayoutArray = NondetRegLayout16LayoutArray(NondetRegLayout(Reg(132u)), NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u)), NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u)), NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u)), NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u)), NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u)), NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u)), NondetRegLayout(Reg(147u)));
const kLayout__74: ToBits_16_Layout = ToBits_16_Layout(kLayout__75);
const kLayout__77: NondetRegLayout16LayoutArray = NondetRegLayout16LayoutArray(NondetRegLayout(Reg(148u)), NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u)), NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u)), NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u)), NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u)), NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u)));
const kLayout__76: ToBits_16_Layout = ToBits_16_Layout(kLayout__77);
const kLayout__73: BitwiseAndU16Layout = BitwiseAndU16Layout(kLayout__74, kLayout__76);
const kLayout__80: NondetRegLayout16LayoutArray = NondetRegLayout16LayoutArray(NondetRegLayout(Reg(164u)), NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u)), NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u)), NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u)), NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u)), NondetRegLayout(Reg(179u)));
const kLayout__79: ToBits_16_Layout = ToBits_16_Layout(kLayout__80);
const kLayout__82: NondetRegLayout16LayoutArray = NondetRegLayout16LayoutArray(NondetRegLayout(Reg(180u)), NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u)), NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u)), NondetRegLayout(Reg(188u)), NondetRegLayout(Reg(189u)), NondetRegLayout(Reg(190u)), NondetRegLayout(Reg(191u)), NondetRegLayout(Reg(192u)), NondetRegLayout(Reg(193u)), NondetRegLayout(Reg(194u)), NondetRegLayout(Reg(195u)));
const kLayout__81: ToBits_16_Layout = ToBits_16_Layout(kLayout__82);
const kLayout__78: BitwiseAndU16Layout = BitwiseAndU16Layout(kLayout__79, kLayout__81);
const kLayout__72: BitwiseAndLayout = BitwiseAndLayout(kLayout__73, kLayout__78);
const kLayout__71: BitwiseXorLayout = BitwiseXorLayout(kLayout__72);
const kLayout__70: OpXORLayout = OpXORLayout(kLayout__71);
const kLayout__69: Misc0MiscOutputArm2Layout = Misc0MiscOutputArm2Layout(kLayout__70, ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__85: BitwiseOrLayout = BitwiseOrLayout(kLayout__72);
const kLayout__84: OpORLayout = OpORLayout(kLayout__85);
const kLayout__83: Misc0MiscOutputArm3Layout = Misc0MiscOutputArm3Layout(kLayout__84, ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__87: OpANDLayout = OpANDLayout(kLayout__72);
const kLayout__86: Misc0MiscOutputArm4Layout = Misc0MiscOutputArm4Layout(kLayout__87, ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__91: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))));
const kLayout__92: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))));
const kLayout__90: NormalizeU32Layout = NormalizeU32Layout(kLayout__91, NondetRegLayout(Reg(132u)), kLayout__92, NondetRegLayout(Reg(133u)));
const kLayout__94: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))));
const kLayout__93: GetSignU32Layout = GetSignU32Layout(NondetRegLayout(Reg(134u)), kLayout__94);
const kLayout__96: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__95: GetSignU32Layout = GetSignU32Layout(NondetRegLayout(Reg(135u)), kLayout__96);
const kLayout__98: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__97: GetSignU32Layout = GetSignU32Layout(NondetRegLayout(Reg(136u)), kLayout__98);
const kLayout__89: CmpLessThanLayout = CmpLessThanLayout(kLayout__90, kLayout__93, kLayout__95, kLayout__97, NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u)));
const kLayout__88: OpSLTLayout = OpSLTLayout(kLayout__89);
const kLayout__101: CmpLessThanUnsignedLayout = CmpLessThanUnsignedLayout(kLayout__90);
const kLayout__100: OpSLTULayout = OpSLTULayout(kLayout__101);
const kLayout__99: Misc0MiscOutputArm6Layout = Misc0MiscOutputArm6Layout(kLayout__100, ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__102: Misc0MiscOutputArm7Layout = Misc0MiscOutputArm7Layout(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__66: Misc0MiscOutputLayout = Misc0MiscOutputLayout(kLayout__67, kLayout__68, kLayout__69, kLayout__83, kLayout__86, kLayout__88, kLayout__99, kLayout__102);
const kLayout__14: Misc0Layout = Misc0Layout(kLayout__15, kLayout__29, kLayout__30, kLayout__64, kLayout__66);
const kLayout__104: _Arguments_Misc1MiscOutputLayout = _Arguments_Misc1MiscOutputLayout(kLayout__65);
const kLayout__107: OpXORILayout = OpXORILayout(kLayout__71);
const kLayout__106: Misc1MiscOutputArm0Layout = Misc1MiscOutputArm0Layout(kLayout__107, ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__109: OpORILayout = OpORILayout(kLayout__85);
const kLayout__108: Misc1MiscOutputArm1Layout = Misc1MiscOutputArm1Layout(kLayout__109, ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__111: OpANDILayout = OpANDILayout(kLayout__72);
const kLayout__110: Misc1MiscOutputArm2Layout = Misc1MiscOutputArm2Layout(kLayout__111, ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__112: OpSLTILayout = OpSLTILayout(kLayout__89);
const kLayout__114: OpSLTIULayout = OpSLTIULayout(kLayout__101);
const kLayout__113: Misc1MiscOutputArm4Layout = Misc1MiscOutputArm4Layout(kLayout__114, ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__117: CmpEqualLayout = CmpEqualLayout(IsZeroLayout(NondetRegLayout(Reg(132u)), NondetRegLayout(Reg(133u))), IsZeroLayout(NondetRegLayout(Reg(134u)), NondetRegLayout(Reg(135u))), NondetRegLayout(Reg(136u)));
const kLayout__116: OpBEQLayout = OpBEQLayout(kLayout__117);
const kLayout__115: Misc1MiscOutputArm5Layout = Misc1MiscOutputArm5Layout(kLayout__116, ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__119: OpBNELayout = OpBNELayout(kLayout__117);
const kLayout__118: Misc1MiscOutputArm6Layout = Misc1MiscOutputArm6Layout(kLayout__119, ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__120: OpBLTLayout = OpBLTLayout(kLayout__89);
const kLayout__105: Misc1MiscOutputLayout = Misc1MiscOutputLayout(kLayout__106, kLayout__108, kLayout__110, kLayout__112, kLayout__113, kLayout__115, kLayout__118, kLayout__120);
const kLayout__103: Misc1Layout = Misc1Layout(kLayout__15, kLayout__29, kLayout__30, kLayout__104, kLayout__105);
const kLayout__122: _Arguments_Misc2MiscOutputLayout = _Arguments_Misc2MiscOutputLayout(kLayout__65);
const kLayout__124: OpBGELayout = OpBGELayout(kLayout__89);
const kLayout__126: OpBLTULayout = OpBLTULayout(kLayout__101);
const kLayout__125: Misc2MiscOutputArm1Layout = Misc2MiscOutputArm1Layout(kLayout__126, ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__128: OpBGEULayout = OpBGEULayout(kLayout__101);
const kLayout__127: Misc2MiscOutputArm2Layout = Misc2MiscOutputArm2Layout(kLayout__128, ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__129: Misc2MiscOutputArm3Layout = Misc2MiscOutputArm3Layout(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__131: OpJALRLayout = OpJALRLayout(NondetRegLayout(Reg(132u)), kLayout__91);
const kLayout__130: Misc2MiscOutputArm4Layout = Misc2MiscOutputArm4Layout(kLayout__131, ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__132: Misc2MiscOutputArm5Layout = Misc2MiscOutputArm5Layout(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__133: Misc2MiscOutputArm6Layout = Misc2MiscOutputArm6Layout(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__134: Misc2MiscOutputArm7Layout = Misc2MiscOutputArm7Layout(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))));
const kLayout__123: Misc2MiscOutputLayout = Misc2MiscOutputLayout(kLayout__124, kLayout__125, kLayout__127, kLayout__129, kLayout__130, kLayout__132, kLayout__133, kLayout__134);
const kLayout__121: Misc2Layout = Misc2Layout(kLayout__15, kLayout__29, kLayout__30, kLayout__122, kLayout__123);
const kLayout__136: DoCycleTableLayout = DoCycleTableLayout(CycleArgLayout(NondetRegLayout(Reg(67u)), NondetRegLayout(Reg(68u))), CycleArgLayout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u))));
const kLayout__139: DecoderLayout = DecoderLayout(NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u)), NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(74u)), NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u)), NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u)), NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u)), NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u)), NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u)), NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u)));
const kLayout__141: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(88u)), NondetRegLayout(Reg(89u))));
const kLayout__142: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(92u)), NondetRegLayout(Reg(93u))));
const kLayout__140: AddrDecomposeLayout = AddrDecomposeLayout(NondetRegLayout(Reg(87u)), kLayout__141, IsZeroLayout(NondetRegLayout(Reg(90u)), NondetRegLayout(Reg(91u))), kLayout__142);
const kLayout__145: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(95u)), NondetRegLayout(Reg(94u)), NondetRegLayout(Reg(96u)), NondetRegLayout(Reg(97u)), NondetRegLayout(Reg(98u)));
const kLayout__146: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(99u)), NondetRegLayout(Reg(94u)), NondetRegLayout(Reg(100u)), NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(102u)));
const kLayout__144: MemoryIOLayout = MemoryIOLayout(kLayout__145, kLayout__146);
const kLayout__148: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(103u)), NondetRegLayout(Reg(104u))));
const kLayout__147: IsForwardLayout = IsForwardLayout(kLayout__148);
const kLayout__143: MemoryReadLayout = MemoryReadLayout(kLayout__144, kLayout__147);
const kLayout__138: DecodeInstLayout = DecodeInstLayout(kLayout__139, kLayout__140, kLayout__143);
const kLayout__152: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u)), NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u)), NondetRegLayout(Reg(109u)));
const kLayout__153: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(110u)), NondetRegLayout(Reg(106u)), NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u)), NondetRegLayout(Reg(113u)));
const kLayout__154: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(114u)), NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u)), NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u)));
const kLayout__155: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(120u)), NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u)));
const kLayout__151: MemoryArgLayout4LayoutArray = MemoryArgLayout4LayoutArray(kLayout__152, kLayout__153, kLayout__154, kLayout__155);
const kLayout__156: CycleArgLayout2LayoutArray = CycleArgLayout2LayoutArray(CycleArgLayout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), CycleArgLayout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))));
const kLayout__150: _Arguments_ReadSourceRegsSourceRegsLayout = _Arguments_ReadSourceRegsSourceRegsLayout(kLayout__151, kLayout__156);
const kLayout__162: MemoryIOLayout = MemoryIOLayout(kLayout__152, kLayout__153);
const kLayout__161: MemoryReadLayout = MemoryReadLayout(kLayout__162, kLayout__62);
const kLayout__160: ReadRegLayout = ReadRegLayout(kLayout__161, NondetRegLayout(Reg(128u)));
const kLayout__159: ReadSourceRegsSourceRegsArm0_SuperLayout = ReadSourceRegsSourceRegsArm0_SuperLayout(kLayout__160);
const kLayout__158: ReadSourceRegsSourceRegsArm0Layout = ReadSourceRegsSourceRegsArm0Layout(kLayout__159, kLayout__154, kLayout__155, CycleArgLayout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))));
const kLayout__166: MemoryIOLayout = MemoryIOLayout(kLayout__154, kLayout__155);
const kLayout__168: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))));
const kLayout__167: IsForwardLayout = IsForwardLayout(kLayout__168);
const kLayout__165: MemoryReadLayout = MemoryReadLayout(kLayout__166, kLayout__167);
const kLayout__164: ReadRegLayout = ReadRegLayout(kLayout__165, NondetRegLayout(Reg(129u)));
const kLayout__163: ReadSourceRegsSourceRegsArm1_SuperLayout = ReadSourceRegsSourceRegsArm1_SuperLayout(kLayout__160, kLayout__164);
const kLayout__157: ReadSourceRegsSourceRegsLayout = ReadSourceRegsSourceRegsLayout(kLayout__158, kLayout__163);
const kLayout__149: ReadSourceRegsLayout = ReadSourceRegsLayout(NondetRegLayout(Reg(127u)), kLayout__150, kLayout__157, NondetRegLayout(Reg(130u)), NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u)), NondetRegLayout(Reg(133u)));
const kLayout__137: MulInputLayout = MulInputLayout(kLayout__138, kLayout__149);
const kLayout__170: ArgU16Layout6LayoutArray = ArgU16Layout6LayoutArray(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))), ArgU16Layout(NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u))));
const kLayout__171: ArgU8Layout13LayoutArray = ArgU8Layout13LayoutArray(ArgU8Layout(NondetRegLayout(Reg(41u)), NondetRegLayout(Reg(42u))), ArgU8Layout(NondetRegLayout(Reg(43u)), NondetRegLayout(Reg(44u))), ArgU8Layout(NondetRegLayout(Reg(45u)), NondetRegLayout(Reg(46u))), ArgU8Layout(NondetRegLayout(Reg(47u)), NondetRegLayout(Reg(48u))), ArgU8Layout(NondetRegLayout(Reg(49u)), NondetRegLayout(Reg(50u))), ArgU8Layout(NondetRegLayout(Reg(51u)), NondetRegLayout(Reg(52u))), ArgU8Layout(NondetRegLayout(Reg(53u)), NondetRegLayout(Reg(54u))), ArgU8Layout(NondetRegLayout(Reg(55u)), NondetRegLayout(Reg(56u))), ArgU8Layout(NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u))), ArgU8Layout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u))), ArgU8Layout(NondetRegLayout(Reg(61u)), NondetRegLayout(Reg(62u))), ArgU8Layout(NondetRegLayout(Reg(63u)), NondetRegLayout(Reg(64u))), ArgU8Layout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(66u))));
const kLayout__169: _Arguments_Mul0MulOutputLayout = _Arguments_Mul0MulOutputLayout(kLayout__170, kLayout__171);
const kLayout__176: NondetRegLayout5LayoutArray = NondetRegLayout5LayoutArray(NondetRegLayout(Reg(134u)), NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u)), NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u)));
const kLayout__175: ToBits_5_Layout = ToBits_5_Layout(kLayout__176);
const kLayout__174: DynPo2Layout = DynPo2Layout(kLayout__175, kLayout__91, NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u)));
const kLayout__180: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(41u)), NondetRegLayout(Reg(42u))));
const kLayout__181: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(43u)), NondetRegLayout(Reg(44u))));
const kLayout__182: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(45u)), NondetRegLayout(Reg(46u))));
const kLayout__183: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(47u)), NondetRegLayout(Reg(48u))));
const kLayout__184: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(49u)), NondetRegLayout(Reg(50u))));
const kLayout__179: ExpandU32Layout = ExpandU32Layout(kLayout__180, kLayout__181, kLayout__182, kLayout__183, kLayout__184, NondetRegLayout(Reg(142u)));
const kLayout__186: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(51u)), NondetRegLayout(Reg(52u))));
const kLayout__187: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(53u)), NondetRegLayout(Reg(54u))));
const kLayout__188: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(55u)), NondetRegLayout(Reg(56u))));
const kLayout__189: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u))));
const kLayout__190: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u))));
const kLayout__185: ExpandU32Layout = ExpandU32Layout(kLayout__186, kLayout__187, kLayout__188, kLayout__189, kLayout__190, NondetRegLayout(Reg(143u)));
const kLayout__192: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(61u)), NondetRegLayout(Reg(62u))));
const kLayout__191: SplitTotalLayout = SplitTotalLayout(kLayout__94, kLayout__192, NondetFakeTwitRegLayout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))));
const kLayout__194: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(63u)), NondetRegLayout(Reg(64u))));
const kLayout__193: SplitTotalLayout = SplitTotalLayout(kLayout__96, kLayout__194, NondetFakeTwitRegLayout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))));
const kLayout__196: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(66u))));
const kLayout__195: SplitTotalLayout = SplitTotalLayout(kLayout__98, kLayout__196, NondetFakeTwitRegLayout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))));
const kLayout__178: MultiplyAccumulateLayout = MultiplyAccumulateLayout(kLayout__179, kLayout__185, NondetRegLayout(Reg(144u)), kLayout__92, kLayout__191, kLayout__193, kLayout__195, kLayout__17, NondetFakeTwitRegLayout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))));
const kLayout__177: DoMulLayout = DoMulLayout(kLayout__178);
const kLayout__173: OpSLLLayout = OpSLLLayout(kLayout__174, kLayout__177);
const kLayout__197: OpSLLILayout = OpSLLILayout(kLayout__174, kLayout__177);
const kLayout__202: ExpandU32Layout = ExpandU32Layout(kLayout__180, kLayout__181, kLayout__182, kLayout__183, kLayout__184, NondetRegLayout(Reg(134u)));
const kLayout__203: ExpandU32Layout = ExpandU32Layout(kLayout__186, kLayout__187, kLayout__188, kLayout__189, kLayout__190, NondetRegLayout(Reg(135u)));
const kLayout__204: SplitTotalLayout = SplitTotalLayout(kLayout__92, kLayout__192, NondetFakeTwitRegLayout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))));
const kLayout__205: SplitTotalLayout = SplitTotalLayout(kLayout__94, kLayout__194, NondetFakeTwitRegLayout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))));
const kLayout__206: SplitTotalLayout = SplitTotalLayout(kLayout__96, kLayout__196, NondetFakeTwitRegLayout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))));
const kLayout__201: MultiplyAccumulateLayout = MultiplyAccumulateLayout(kLayout__202, kLayout__203, NondetRegLayout(Reg(136u)), kLayout__91, kLayout__204, kLayout__205, kLayout__206, kLayout__98, NondetFakeTwitRegLayout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))));
const kLayout__200: DoMulLayout = DoMulLayout(kLayout__201);
const kLayout__199: OpMULLayout = OpMULLayout(kLayout__200);
const kLayout__198: Mul0MulOutputArm2Layout = Mul0MulOutputArm2Layout(kLayout__199, ArgU16Layout(NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u))));
const kLayout__208: OpMULHLayout = OpMULHLayout(kLayout__200);
const kLayout__207: Mul0MulOutputArm3Layout = Mul0MulOutputArm3Layout(kLayout__208, ArgU16Layout(NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u))));
const kLayout__210: OpMULHSULayout = OpMULHSULayout(kLayout__200);
const kLayout__209: Mul0MulOutputArm4Layout = Mul0MulOutputArm4Layout(kLayout__210, ArgU16Layout(NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u))));
const kLayout__212: OpMULHULayout = OpMULHULayout(kLayout__200);
const kLayout__211: Mul0MulOutputArm5Layout = Mul0MulOutputArm5Layout(kLayout__212, ArgU16Layout(NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u))));
const kLayout__213: Mul0MulOutputArm6Layout = Mul0MulOutputArm6Layout(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))), ArgU16Layout(NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u))), ArgU8Layout(NondetRegLayout(Reg(41u)), NondetRegLayout(Reg(42u))), ArgU8Layout(NondetRegLayout(Reg(43u)), NondetRegLayout(Reg(44u))), ArgU8Layout(NondetRegLayout(Reg(45u)), NondetRegLayout(Reg(46u))), ArgU8Layout(NondetRegLayout(Reg(47u)), NondetRegLayout(Reg(48u))), ArgU8Layout(NondetRegLayout(Reg(49u)), NondetRegLayout(Reg(50u))), ArgU8Layout(NondetRegLayout(Reg(51u)), NondetRegLayout(Reg(52u))), ArgU8Layout(NondetRegLayout(Reg(53u)), NondetRegLayout(Reg(54u))), ArgU8Layout(NondetRegLayout(Reg(55u)), NondetRegLayout(Reg(56u))), ArgU8Layout(NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u))), ArgU8Layout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u))), ArgU8Layout(NondetRegLayout(Reg(61u)), NondetRegLayout(Reg(62u))), ArgU8Layout(NondetRegLayout(Reg(63u)), NondetRegLayout(Reg(64u))), ArgU8Layout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(66u))));
const kLayout__214: Mul0MulOutputArm7Layout = Mul0MulOutputArm7Layout(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))), ArgU16Layout(NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u))), ArgU8Layout(NondetRegLayout(Reg(41u)), NondetRegLayout(Reg(42u))), ArgU8Layout(NondetRegLayout(Reg(43u)), NondetRegLayout(Reg(44u))), ArgU8Layout(NondetRegLayout(Reg(45u)), NondetRegLayout(Reg(46u))), ArgU8Layout(NondetRegLayout(Reg(47u)), NondetRegLayout(Reg(48u))), ArgU8Layout(NondetRegLayout(Reg(49u)), NondetRegLayout(Reg(50u))), ArgU8Layout(NondetRegLayout(Reg(51u)), NondetRegLayout(Reg(52u))), ArgU8Layout(NondetRegLayout(Reg(53u)), NondetRegLayout(Reg(54u))), ArgU8Layout(NondetRegLayout(Reg(55u)), NondetRegLayout(Reg(56u))), ArgU8Layout(NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u))), ArgU8Layout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u))), ArgU8Layout(NondetRegLayout(Reg(61u)), NondetRegLayout(Reg(62u))), ArgU8Layout(NondetRegLayout(Reg(63u)), NondetRegLayout(Reg(64u))), ArgU8Layout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(66u))));
const kLayout__172: Mul0MulOutputLayout = Mul0MulOutputLayout(kLayout__173, kLayout__197, kLayout__198, kLayout__207, kLayout__209, kLayout__211, kLayout__213, kLayout__214);
const kLayout__218: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(158u)), NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u)));
const kLayout__219: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u)));
const kLayout__217: MemoryIOLayout = MemoryIOLayout(kLayout__218, kLayout__219);
const kLayout__221: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u))));
const kLayout__220: IsForwardLayout = IsForwardLayout(kLayout__221);
const kLayout__216: MemoryWriteLayout = MemoryWriteLayout(kLayout__217, kLayout__220);
const kLayout__215: WriteRdLayout = WriteRdLayout(IsZeroLayout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), NondetRegLayout(Reg(155u)), kLayout__216);
const kLayout__223: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u))));
const kLayout__224: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u))));
const kLayout__222: NormalizeU32Layout = NormalizeU32Layout(kLayout__223, NondetRegLayout(Reg(169u)), kLayout__224, NondetRegLayout(Reg(172u)));
const kLayout__135: Mul0Layout = Mul0Layout(kLayout__136, kLayout__137, kLayout__169, kLayout__172, kLayout__215, kLayout__222);
const kLayout__226: DoCycleTableLayout = DoCycleTableLayout(CycleArgLayout(NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u))), CycleArgLayout(NondetRegLayout(Reg(89u)), NondetRegLayout(Reg(90u))));
const kLayout__229: DecoderLayout = DecoderLayout(NondetRegLayout(Reg(91u)), NondetRegLayout(Reg(92u)), NondetRegLayout(Reg(93u)), NondetRegLayout(Reg(94u)), NondetRegLayout(Reg(95u)), NondetRegLayout(Reg(96u)), NondetRegLayout(Reg(97u)), NondetRegLayout(Reg(98u)), NondetRegLayout(Reg(99u)), NondetRegLayout(Reg(100u)), NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(102u)), NondetRegLayout(Reg(103u)), NondetRegLayout(Reg(104u)), NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u)));
const kLayout__231: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(108u)), NondetRegLayout(Reg(109u))));
const kLayout__232: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(112u)), NondetRegLayout(Reg(113u))));
const kLayout__230: AddrDecomposeLayout = AddrDecomposeLayout(NondetRegLayout(Reg(107u)), kLayout__231, IsZeroLayout(NondetRegLayout(Reg(110u)), NondetRegLayout(Reg(111u))), kLayout__232);
const kLayout__235: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(114u)), NondetRegLayout(Reg(116u)), NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u)));
const kLayout__236: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(114u)), NondetRegLayout(Reg(120u)), NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u)));
const kLayout__234: MemoryIOLayout = MemoryIOLayout(kLayout__235, kLayout__236);
const kLayout__233: MemoryReadLayout = MemoryReadLayout(kLayout__234, kLayout__62);
const kLayout__228: DecodeInstLayout = DecodeInstLayout(kLayout__229, kLayout__230, kLayout__233);
const kLayout__240: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u)), NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u)), NondetRegLayout(Reg(129u)));
const kLayout__241: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(130u)), NondetRegLayout(Reg(126u)), NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u)), NondetRegLayout(Reg(133u)));
const kLayout__242: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(134u)), NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u)), NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u)));
const kLayout__243: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u)));
const kLayout__239: MemoryArgLayout4LayoutArray = MemoryArgLayout4LayoutArray(kLayout__240, kLayout__241, kLayout__242, kLayout__243);
const kLayout__244: CycleArgLayout2LayoutArray = CycleArgLayout2LayoutArray(CycleArgLayout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), CycleArgLayout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))));
const kLayout__238: _Arguments_ReadSourceRegsSourceRegsLayout = _Arguments_ReadSourceRegsSourceRegsLayout(kLayout__239, kLayout__244);
const kLayout__250: MemoryIOLayout = MemoryIOLayout(kLayout__240, kLayout__241);
const kLayout__252: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))));
const kLayout__251: IsForwardLayout = IsForwardLayout(kLayout__252);
const kLayout__249: MemoryReadLayout = MemoryReadLayout(kLayout__250, kLayout__251);
const kLayout__248: ReadRegLayout = ReadRegLayout(kLayout__249, NondetRegLayout(Reg(148u)));
const kLayout__247: ReadSourceRegsSourceRegsArm0_SuperLayout = ReadSourceRegsSourceRegsArm0_SuperLayout(kLayout__248);
const kLayout__246: ReadSourceRegsSourceRegsArm0Layout = ReadSourceRegsSourceRegsArm0Layout(kLayout__247, kLayout__242, kLayout__243, CycleArgLayout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))));
const kLayout__256: MemoryIOLayout = MemoryIOLayout(kLayout__242, kLayout__243);
const kLayout__258: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))));
const kLayout__257: IsForwardLayout = IsForwardLayout(kLayout__258);
const kLayout__255: MemoryReadLayout = MemoryReadLayout(kLayout__256, kLayout__257);
const kLayout__254: ReadRegLayout = ReadRegLayout(kLayout__255, NondetRegLayout(Reg(149u)));
const kLayout__253: ReadSourceRegsSourceRegsArm1_SuperLayout = ReadSourceRegsSourceRegsArm1_SuperLayout(kLayout__248, kLayout__254);
const kLayout__245: ReadSourceRegsSourceRegsLayout = ReadSourceRegsSourceRegsLayout(kLayout__246, kLayout__253);
const kLayout__237: ReadSourceRegsLayout = ReadSourceRegsLayout(NondetRegLayout(Reg(147u)), kLayout__238, kLayout__245, NondetRegLayout(Reg(150u)), NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u)), NondetRegLayout(Reg(153u)));
const kLayout__227: DivInputLayout = DivInputLayout(kLayout__228, kLayout__237);
const kLayout__260: ArgU16Layout16LayoutArray = ArgU16Layout16LayoutArray(ArgU16Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU16Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU16Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), ArgU16Layout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))), ArgU16Layout(NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u))), ArgU16Layout(NondetRegLayout(Reg(41u)), NondetRegLayout(Reg(42u))), ArgU16Layout(NondetRegLayout(Reg(43u)), NondetRegLayout(Reg(44u))), ArgU16Layout(NondetRegLayout(Reg(45u)), NondetRegLayout(Reg(46u))), ArgU16Layout(NondetRegLayout(Reg(47u)), NondetRegLayout(Reg(48u))), ArgU16Layout(NondetRegLayout(Reg(49u)), NondetRegLayout(Reg(50u))), ArgU16Layout(NondetRegLayout(Reg(51u)), NondetRegLayout(Reg(52u))), ArgU16Layout(NondetRegLayout(Reg(53u)), NondetRegLayout(Reg(54u))), ArgU16Layout(NondetRegLayout(Reg(55u)), NondetRegLayout(Reg(56u))), ArgU16Layout(NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u))), ArgU16Layout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u))));
const kLayout__261: ArgU8Layout13LayoutArray = ArgU8Layout13LayoutArray(ArgU8Layout(NondetRegLayout(Reg(61u)), NondetRegLayout(Reg(62u))), ArgU8Layout(NondetRegLayout(Reg(63u)), NondetRegLayout(Reg(64u))), ArgU8Layout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(66u))), ArgU8Layout(NondetRegLayout(Reg(67u)), NondetRegLayout(Reg(68u))), ArgU8Layout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u))), ArgU8Layout(NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u))), ArgU8Layout(NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(74u))), ArgU8Layout(NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u))), ArgU8Layout(NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u))), ArgU8Layout(NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u))), ArgU8Layout(NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u))), ArgU8Layout(NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u))), ArgU8Layout(NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u))));
const kLayout__259: _Arguments_Div0MulOutputLayout = _Arguments_Div0MulOutputLayout(kLayout__260, kLayout__261);
const kLayout__267: NondetRegLayout5LayoutArray = NondetRegLayout5LayoutArray(NondetRegLayout(Reg(154u)), NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u)));
const kLayout__266: ToBits_5_Layout = ToBits_5_Layout(kLayout__267);
const kLayout__265: DynPo2Layout = DynPo2Layout(kLayout__266, kLayout__91, NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(161u)));
const kLayout__271: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(67u)), NondetRegLayout(Reg(68u))));
const kLayout__272: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u))));
const kLayout__270: ExpandU32Layout = ExpandU32Layout(kLayout__192, kLayout__194, kLayout__196, kLayout__271, kLayout__272, NondetRegLayout(Reg(164u)));
const kLayout__274: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u))));
const kLayout__275: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(74u))));
const kLayout__276: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u))));
const kLayout__277: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u))));
const kLayout__278: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u))));
const kLayout__273: ExpandU32Layout = ExpandU32Layout(kLayout__274, kLayout__275, kLayout__276, kLayout__277, kLayout__278, NondetRegLayout(Reg(165u)));
const kLayout__280: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u))));
const kLayout__279: SplitTotalLayout = SplitTotalLayout(kLayout__98, kLayout__280, NondetFakeTwitRegLayout(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u))));
const kLayout__282: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u))));
const kLayout__281: SplitTotalLayout = SplitTotalLayout(kLayout__17, kLayout__282, NondetFakeTwitRegLayout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u))));
const kLayout__284: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(41u)), NondetRegLayout(Reg(42u))));
const kLayout__285: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u))));
const kLayout__283: SplitTotalLayout = SplitTotalLayout(kLayout__284, kLayout__285, NondetFakeTwitRegLayout(NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u))));
const kLayout__286: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(43u)), NondetRegLayout(Reg(44u))));
const kLayout__269: MultiplyAccumulateLayout = MultiplyAccumulateLayout(kLayout__270, kLayout__273, NondetRegLayout(Reg(166u)), kLayout__96, kLayout__279, kLayout__281, kLayout__283, kLayout__286, NondetFakeTwitRegLayout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u))));
const kLayout__288: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(47u)), NondetRegLayout(Reg(48u))));
const kLayout__289: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(49u)), NondetRegLayout(Reg(50u))));
const kLayout__287: NormalizeU32Layout = NormalizeU32Layout(kLayout__288, NondetRegLayout(Reg(177u)), kLayout__289, NondetRegLayout(Reg(178u)));
const kLayout__291: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(51u)), NondetRegLayout(Reg(52u))));
const kLayout__292: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(53u)), NondetRegLayout(Reg(54u))));
const kLayout__290: NormalizeU32Layout = NormalizeU32Layout(kLayout__291, NondetRegLayout(Reg(179u)), kLayout__292, NondetRegLayout(Reg(180u)));
const kLayout__295: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(55u)), NondetRegLayout(Reg(56u))));
const kLayout__296: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u))));
const kLayout__294: NormalizeU32Layout = NormalizeU32Layout(kLayout__295, NondetRegLayout(Reg(183u)), kLayout__296, NondetRegLayout(Reg(184u)));
const kLayout__293: CmpLessThanUnsignedLayout = CmpLessThanUnsignedLayout(kLayout__294);
const kLayout__268: DoDivLayout = DoDivLayout(NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u)), kLayout__92, kLayout__94, kLayout__269, NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u)), kLayout__20, kLayout__287, kLayout__290, NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u)), kLayout__293);
const kLayout__264: OpSRLLayout = OpSRLLayout(kLayout__265, kLayout__268);
const kLayout__263: Div0MulOutputArm0Layout = Div0MulOutputArm0Layout(kLayout__264, ArgU16Layout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u))));
const kLayout__298: TopBitLayout = TopBitLayout(NondetRegLayout(Reg(162u)), kLayout__92);
const kLayout__301: ExpandU32Layout = ExpandU32Layout(kLayout__192, kLayout__194, kLayout__196, kLayout__271, kLayout__272, NondetRegLayout(Reg(165u)));
const kLayout__302: ExpandU32Layout = ExpandU32Layout(kLayout__274, kLayout__275, kLayout__276, kLayout__277, kLayout__278, NondetRegLayout(Reg(166u)));
const kLayout__303: SplitTotalLayout = SplitTotalLayout(kLayout__17, kLayout__280, NondetFakeTwitRegLayout(NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(169u))));
const kLayout__304: SplitTotalLayout = SplitTotalLayout(kLayout__284, kLayout__282, NondetFakeTwitRegLayout(NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u))));
const kLayout__305: SplitTotalLayout = SplitTotalLayout(kLayout__286, kLayout__285, NondetFakeTwitRegLayout(NondetRegLayout(Reg(172u)), NondetRegLayout(Reg(173u))));
const kLayout__300: MultiplyAccumulateLayout = MultiplyAccumulateLayout(kLayout__301, kLayout__302, NondetRegLayout(Reg(167u)), kLayout__98, kLayout__303, kLayout__304, kLayout__305, kLayout__20, NondetFakeTwitRegLayout(NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u))));
const kLayout__306: NormalizeU32Layout = NormalizeU32Layout(kLayout__289, NondetRegLayout(Reg(178u)), kLayout__291, NondetRegLayout(Reg(179u)));
const kLayout__307: NormalizeU32Layout = NormalizeU32Layout(kLayout__292, NondetRegLayout(Reg(180u)), kLayout__295, NondetRegLayout(Reg(181u)));
const kLayout__310: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u))));
const kLayout__309: NormalizeU32Layout = NormalizeU32Layout(kLayout__296, NondetRegLayout(Reg(184u)), kLayout__310, NondetRegLayout(Reg(185u)));
const kLayout__308: CmpLessThanUnsignedLayout = CmpLessThanUnsignedLayout(kLayout__309);
const kLayout__299: DoDivLayout = DoDivLayout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u)), kLayout__94, kLayout__96, kLayout__300, NondetRegLayout(Reg(176u)), NondetRegLayout(Reg(177u)), kLayout__288, kLayout__306, kLayout__307, NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u)), kLayout__308);
const kLayout__297: OpSRALayout = OpSRALayout(kLayout__265, kLayout__298, kLayout__299);
const kLayout__312: OpSRLILayout = OpSRLILayout(kLayout__265, kLayout__268);
const kLayout__311: Div0MulOutputArm2Layout = Div0MulOutputArm2Layout(kLayout__312, ArgU16Layout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u))));
const kLayout__313: OpSRAILayout = OpSRAILayout(kLayout__265, kLayout__298, kLayout__299);
const kLayout__318: ExpandU32Layout = ExpandU32Layout(kLayout__192, kLayout__194, kLayout__196, kLayout__271, kLayout__272, NondetRegLayout(Reg(156u)));
const kLayout__319: ExpandU32Layout = ExpandU32Layout(kLayout__274, kLayout__275, kLayout__276, kLayout__277, kLayout__278, NondetRegLayout(Reg(157u)));
const kLayout__320: SplitTotalLayout = SplitTotalLayout(kLayout__96, kLayout__280, NondetFakeTwitRegLayout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))));
const kLayout__321: SplitTotalLayout = SplitTotalLayout(kLayout__98, kLayout__282, NondetFakeTwitRegLayout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u))));
const kLayout__322: SplitTotalLayout = SplitTotalLayout(kLayout__17, kLayout__285, NondetFakeTwitRegLayout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u))));
const kLayout__317: MultiplyAccumulateLayout = MultiplyAccumulateLayout(kLayout__318, kLayout__319, NondetRegLayout(Reg(158u)), kLayout__94, kLayout__320, kLayout__321, kLayout__322, kLayout__284, NondetFakeTwitRegLayout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u))));
const kLayout__323: NormalizeU32Layout = NormalizeU32Layout(kLayout__20, NondetRegLayout(Reg(169u)), kLayout__288, NondetRegLayout(Reg(170u)));
const kLayout__324: NormalizeU32Layout = NormalizeU32Layout(kLayout__289, NondetRegLayout(Reg(171u)), kLayout__291, NondetRegLayout(Reg(172u)));
const kLayout__326: NormalizeU32Layout = NormalizeU32Layout(kLayout__292, NondetRegLayout(Reg(175u)), kLayout__295, NondetRegLayout(Reg(176u)));
const kLayout__325: CmpLessThanUnsignedLayout = CmpLessThanUnsignedLayout(kLayout__326);
const kLayout__316: DoDivLayout = DoDivLayout(NondetRegLayout(Reg(154u)), NondetRegLayout(Reg(155u)), kLayout__91, kLayout__92, kLayout__317, NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u)), kLayout__286, kLayout__323, kLayout__324, NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u)), kLayout__325);
const kLayout__315: OpDIVLayout = OpDIVLayout(kLayout__316);
const kLayout__314: Div0MulOutputArm4Layout = Div0MulOutputArm4Layout(kLayout__315, ArgU16Layout(NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u))), ArgU16Layout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u))));
const kLayout__328: OpDIVULayout = OpDIVULayout(kLayout__316);
const kLayout__327: Div0MulOutputArm5Layout = Div0MulOutputArm5Layout(kLayout__328, ArgU16Layout(NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u))), ArgU16Layout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u))));
const kLayout__330: OpREMLayout = OpREMLayout(kLayout__316);
const kLayout__329: Div0MulOutputArm6Layout = Div0MulOutputArm6Layout(kLayout__330, ArgU16Layout(NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u))), ArgU16Layout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u))));
const kLayout__332: OpREMULayout = OpREMULayout(kLayout__316);
const kLayout__331: Div0MulOutputArm7Layout = Div0MulOutputArm7Layout(kLayout__332, ArgU16Layout(NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u))), ArgU16Layout(NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u))));
const kLayout__262: Div0MulOutputLayout = Div0MulOutputLayout(kLayout__263, kLayout__297, kLayout__311, kLayout__313, kLayout__314, kLayout__327, kLayout__329, kLayout__331);
const kLayout__336: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(190u)), NondetRegLayout(Reg(189u)), NondetRegLayout(Reg(191u)), NondetRegLayout(Reg(192u)), NondetRegLayout(Reg(193u)));
const kLayout__337: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(194u)), NondetRegLayout(Reg(189u)), NondetRegLayout(Reg(195u)), NondetRegLayout(Reg(196u)), NondetRegLayout(Reg(197u)));
const kLayout__335: MemoryIOLayout = MemoryIOLayout(kLayout__336, kLayout__337);
const kLayout__339: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(198u)), NondetRegLayout(Reg(199u))));
const kLayout__338: IsForwardLayout = IsForwardLayout(kLayout__339);
const kLayout__334: MemoryWriteLayout = MemoryWriteLayout(kLayout__335, kLayout__338);
const kLayout__333: WriteRdLayout = WriteRdLayout(IsZeroLayout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u))), NondetRegLayout(Reg(188u)), kLayout__334);
const kLayout__341: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(200u)), NondetRegLayout(Reg(201u))));
const kLayout__342: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(203u)), NondetRegLayout(Reg(204u))));
const kLayout__340: NormalizeU32Layout = NormalizeU32Layout(kLayout__341, NondetRegLayout(Reg(202u)), kLayout__342, NondetRegLayout(Reg(205u)));
const kLayout__225: Div0Layout = Div0Layout(kLayout__226, kLayout__227, kLayout__259, kLayout__262, kLayout__333, kLayout__340);
const kLayout__344: DoCycleTableLayout = DoCycleTableLayout(CycleArgLayout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))), CycleArgLayout(NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u))));
const kLayout__347: DecoderLayout = DecoderLayout(NondetRegLayout(Reg(41u)), NondetRegLayout(Reg(42u)), NondetRegLayout(Reg(43u)), NondetRegLayout(Reg(44u)), NondetRegLayout(Reg(45u)), NondetRegLayout(Reg(46u)), NondetRegLayout(Reg(47u)), NondetRegLayout(Reg(48u)), NondetRegLayout(Reg(49u)), NondetRegLayout(Reg(50u)), NondetRegLayout(Reg(51u)), NondetRegLayout(Reg(52u)), NondetRegLayout(Reg(53u)), NondetRegLayout(Reg(54u)), NondetRegLayout(Reg(55u)), NondetRegLayout(Reg(56u)));
const kLayout__349: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(58u)), NondetRegLayout(Reg(59u))));
const kLayout__350: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(62u)), NondetRegLayout(Reg(63u))));
const kLayout__348: AddrDecomposeLayout = AddrDecomposeLayout(NondetRegLayout(Reg(57u)), kLayout__349, IsZeroLayout(NondetRegLayout(Reg(60u)), NondetRegLayout(Reg(61u))), kLayout__350);
const kLayout__353: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(64u)), NondetRegLayout(Reg(66u)), NondetRegLayout(Reg(67u)), NondetRegLayout(Reg(68u)));
const kLayout__354: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(64u)), NondetRegLayout(Reg(70u)), NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u)));
const kLayout__352: MemoryIOLayout = MemoryIOLayout(kLayout__353, kLayout__354);
const kLayout__356: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(74u))));
const kLayout__355: IsForwardLayout = IsForwardLayout(kLayout__356);
const kLayout__351: MemoryReadLayout = MemoryReadLayout(kLayout__352, kLayout__355);
const kLayout__346: DecodeInstLayout = DecodeInstLayout(kLayout__347, kLayout__348, kLayout__351);
const kLayout__360: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(76u)), NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u)), NondetRegLayout(Reg(79u)));
const kLayout__361: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(80u)), NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u)), NondetRegLayout(Reg(83u)));
const kLayout__359: MemoryIOLayout = MemoryIOLayout(kLayout__360, kLayout__361);
const kLayout__363: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(84u)), NondetRegLayout(Reg(85u))));
const kLayout__362: IsForwardLayout = IsForwardLayout(kLayout__363);
const kLayout__358: MemoryReadLayout = MemoryReadLayout(kLayout__359, kLayout__362);
const kLayout__357: ReadRegLayout = ReadRegLayout(kLayout__358, NondetRegLayout(Reg(86u)));
const kLayout__365: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u))));
const kLayout__364: NormalizeU32Layout = NormalizeU32Layout(kLayout__365, NondetRegLayout(Reg(89u)), kLayout__35, NondetRegLayout(Reg(92u)));
const kLayout__367: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(95u)), NondetRegLayout(Reg(96u))));
const kLayout__368: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(99u)), NondetRegLayout(Reg(100u))));
const kLayout__366: AddrDecomposeBitsLayout = AddrDecomposeBitsLayout(NondetRegLayout(Reg(93u)), NondetRegLayout(Reg(94u)), kLayout__367, IsZeroLayout(NondetRegLayout(Reg(97u)), NondetRegLayout(Reg(98u))), kLayout__368);
const kLayout__371: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(102u)), NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(103u)), NondetRegLayout(Reg(104u)), NondetRegLayout(Reg(105u)));
const kLayout__372: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(106u)), NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u)), NondetRegLayout(Reg(109u)));
const kLayout__370: MemoryIOLayout = MemoryIOLayout(kLayout__371, kLayout__372);
const kLayout__374: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(110u)), NondetRegLayout(Reg(111u))));
const kLayout__373: IsForwardLayout = IsForwardLayout(kLayout__374);
const kLayout__369: MemoryReadLayout = MemoryReadLayout(kLayout__370, kLayout__373);
const kLayout__345: MemLoadInputLayout = MemLoadInputLayout(kLayout__346, kLayout__357, kLayout__364, kLayout__366, kLayout__369);
const kLayout__376: ArgU8Layout3LayoutArray = ArgU8Layout3LayoutArray(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))));
const kLayout__375: _Arguments_Mem0OutputLayout = _Arguments_Mem0OutputLayout(kLayout__376, ArgU16Layout1LayoutArray(ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u)))));
const kLayout__381: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))));
const kLayout__382: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))));
const kLayout__380: SplitWordLayout = SplitWordLayout(kLayout__381, kLayout__382);
const kLayout__383: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))));
const kLayout__379: OpLBLayout = OpLBLayout(kLayout__380, NondetRegLayout(Reg(112u)), kLayout__383);
const kLayout__378: Mem0OutputArm0Layout = Mem0OutputArm0Layout(kLayout__379, ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__385: OpLHLayout = OpLHLayout(NondetRegLayout(Reg(112u)), kLayout__96);
const kLayout__384: Mem0OutputArm1Layout = Mem0OutputArm1Layout(kLayout__385, ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))));
const kLayout__386: Mem0OutputArm2Layout = Mem0OutputArm2Layout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__388: OpLBULayout = OpLBULayout(kLayout__380);
const kLayout__387: Mem0OutputArm3Layout = Mem0OutputArm3Layout(kLayout__388, ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__389: Mem0OutputArm4Layout = Mem0OutputArm4Layout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__390: Mem0OutputArm5Layout = Mem0OutputArm5Layout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__391: Mem0OutputArm6Layout = Mem0OutputArm6Layout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__392: Mem0OutputArm7Layout = Mem0OutputArm7Layout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU16Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__377: Mem0OutputLayout = Mem0OutputLayout(kLayout__378, kLayout__384, kLayout__386, kLayout__387, kLayout__389, kLayout__390, kLayout__391, kLayout__392);
const kLayout__396: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(116u)), NondetRegLayout(Reg(118u)), NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u)));
const kLayout__397: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(116u)), NondetRegLayout(Reg(122u)), NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u)));
const kLayout__395: MemoryIOLayout = MemoryIOLayout(kLayout__396, kLayout__397);
const kLayout__394: MemoryWriteLayout = MemoryWriteLayout(kLayout__395, kLayout__167);
const kLayout__393: WriteRdLayout = WriteRdLayout(IsZeroLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), NondetRegLayout(Reg(115u)), kLayout__394);
const kLayout__399: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))));
const kLayout__400: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(130u)), NondetRegLayout(Reg(131u))));
const kLayout__398: NormalizeU32Layout = NormalizeU32Layout(kLayout__399, NondetRegLayout(Reg(129u)), kLayout__400, NondetRegLayout(Reg(132u)));
const kLayout__343: Mem0Layout = Mem0Layout(kLayout__344, kLayout__345, kLayout__375, kLayout__377, kLayout__393, kLayout__398);
const kLayout__406: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u)), NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u)), NondetRegLayout(Reg(79u)));
const kLayout__407: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(80u)), NondetRegLayout(Reg(76u)), NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u)), NondetRegLayout(Reg(83u)));
const kLayout__408: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(84u)), NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u)), NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u)));
const kLayout__409: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(89u)), NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(90u)), NondetRegLayout(Reg(91u)), NondetRegLayout(Reg(92u)));
const kLayout__405: MemoryArgLayout4LayoutArray = MemoryArgLayout4LayoutArray(kLayout__406, kLayout__407, kLayout__408, kLayout__409);
const kLayout__410: CycleArgLayout2LayoutArray = CycleArgLayout2LayoutArray(CycleArgLayout(NondetRegLayout(Reg(93u)), NondetRegLayout(Reg(94u))), CycleArgLayout(NondetRegLayout(Reg(95u)), NondetRegLayout(Reg(96u))));
const kLayout__404: _Arguments_ReadSourceRegsSourceRegsLayout = _Arguments_ReadSourceRegsSourceRegsLayout(kLayout__405, kLayout__410);
const kLayout__416: MemoryIOLayout = MemoryIOLayout(kLayout__406, kLayout__407);
const kLayout__418: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(93u)), NondetRegLayout(Reg(94u))));
const kLayout__417: IsForwardLayout = IsForwardLayout(kLayout__418);
const kLayout__415: MemoryReadLayout = MemoryReadLayout(kLayout__416, kLayout__417);
const kLayout__414: ReadRegLayout = ReadRegLayout(kLayout__415, NondetRegLayout(Reg(98u)));
const kLayout__413: ReadSourceRegsSourceRegsArm0_SuperLayout = ReadSourceRegsSourceRegsArm0_SuperLayout(kLayout__414);
const kLayout__412: ReadSourceRegsSourceRegsArm0Layout = ReadSourceRegsSourceRegsArm0Layout(kLayout__413, kLayout__408, kLayout__409, CycleArgLayout(NondetRegLayout(Reg(95u)), NondetRegLayout(Reg(96u))));
const kLayout__422: MemoryIOLayout = MemoryIOLayout(kLayout__408, kLayout__409);
const kLayout__424: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(95u)), NondetRegLayout(Reg(96u))));
const kLayout__423: IsForwardLayout = IsForwardLayout(kLayout__424);
const kLayout__421: MemoryReadLayout = MemoryReadLayout(kLayout__422, kLayout__423);
const kLayout__420: ReadRegLayout = ReadRegLayout(kLayout__421, NondetRegLayout(Reg(99u)));
const kLayout__419: ReadSourceRegsSourceRegsArm1_SuperLayout = ReadSourceRegsSourceRegsArm1_SuperLayout(kLayout__414, kLayout__420);
const kLayout__411: ReadSourceRegsSourceRegsLayout = ReadSourceRegsSourceRegsLayout(kLayout__412, kLayout__419);
const kLayout__403: ReadSourceRegsLayout = ReadSourceRegsLayout(NondetRegLayout(Reg(97u)), kLayout__404, kLayout__411, NondetRegLayout(Reg(100u)), NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(102u)), NondetRegLayout(Reg(103u)));
const kLayout__426: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(104u)), NondetRegLayout(Reg(105u))));
const kLayout__427: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))));
const kLayout__425: NormalizeU32Layout = NormalizeU32Layout(kLayout__426, NondetRegLayout(Reg(106u)), kLayout__427, NondetRegLayout(Reg(109u)));
const kLayout__429: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(116u)), NondetRegLayout(Reg(117u))));
const kLayout__428: AddrDecomposeBitsLayout = AddrDecomposeBitsLayout(NondetRegLayout(Reg(110u)), NondetRegLayout(Reg(111u)), kLayout__232, IsZeroLayout(NondetRegLayout(Reg(114u)), NondetRegLayout(Reg(115u))), kLayout__429);
const kLayout__432: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(118u)), NondetRegLayout(Reg(120u)), NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u)));
const kLayout__433: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(118u)), NondetRegLayout(Reg(124u)), NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u)));
const kLayout__431: MemoryIOLayout = MemoryIOLayout(kLayout__432, kLayout__433);
const kLayout__435: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))));
const kLayout__434: IsForwardLayout = IsForwardLayout(kLayout__435);
const kLayout__430: MemoryReadLayout = MemoryReadLayout(kLayout__431, kLayout__434);
const kLayout__402: MemStoreInputLayout = MemStoreInputLayout(kLayout__346, kLayout__403, kLayout__425, kLayout__428, kLayout__430);
const kLayout__437: ArgU8Layout4LayoutArray = ArgU8Layout4LayoutArray(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU8Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__436: _Arguments_Mem1OutputLayout = _Arguments_Mem1OutputLayout(kLayout__437);
const kLayout__441: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__440: SplitWordLayout = SplitWordLayout(kLayout__383, kLayout__441);
const kLayout__439: OpSBLayout = OpSBLayout(kLayout__380, kLayout__440);
const kLayout__442: Mem1OutputArm1Layout = Mem1OutputArm1Layout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU8Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__443: Mem1OutputArm2Layout = Mem1OutputArm2Layout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU8Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__444: Mem1OutputArm3Layout = Mem1OutputArm3Layout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU8Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__445: Mem1OutputArm4Layout = Mem1OutputArm4Layout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU8Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__446: Mem1OutputArm5Layout = Mem1OutputArm5Layout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU8Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__447: Mem1OutputArm6Layout = Mem1OutputArm6Layout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU8Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__448: Mem1OutputArm7Layout = Mem1OutputArm7Layout(ArgU8Layout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), ArgU8Layout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))), ArgU8Layout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), ArgU8Layout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))));
const kLayout__438: Mem1OutputLayout = Mem1OutputLayout(kLayout__439, kLayout__442, kLayout__443, kLayout__444, kLayout__445, kLayout__446, kLayout__447, kLayout__448);
const kLayout__452: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(130u)), NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u)), NondetRegLayout(Reg(133u)));
const kLayout__453: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(134u)), NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u)), NondetRegLayout(Reg(137u)));
const kLayout__451: MemoryIOLayout = MemoryIOLayout(kLayout__452, kLayout__453);
const kLayout__455: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(138u)), NondetRegLayout(Reg(139u))));
const kLayout__454: IsForwardLayout = IsForwardLayout(kLayout__455);
const kLayout__450: MemoryWriteLayout = MemoryWriteLayout(kLayout__451, kLayout__454);
const kLayout__449: MemStoreFinalizeLayout = MemStoreFinalizeLayout(kLayout__450);
const kLayout__457: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u))));
const kLayout__458: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))));
const kLayout__456: NormalizeU32Layout = NormalizeU32Layout(kLayout__457, NondetRegLayout(Reg(142u)), kLayout__458, NondetRegLayout(Reg(145u)));
const kLayout__401: Mem1Layout = Mem1Layout(kLayout__344, kLayout__402, kLayout__436, kLayout__438, kLayout__449, kLayout__456);
const kLayout__466: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u)), NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u)), NondetRegLayout(Reg(33u)));
const kLayout__467: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(34u)), NondetRegLayout(Reg(30u)), NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u)), NondetRegLayout(Reg(37u)));
const kLayout__465: MemoryIOLayout = MemoryIOLayout(kLayout__466, kLayout__467);
const kLayout__464: MemoryPageInLayout = MemoryPageInLayout(kLayout__465);
const kLayout__470: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(38u)), NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u)), NondetRegLayout(Reg(41u)), NondetRegLayout(Reg(42u)));
const kLayout__471: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(43u)), NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(44u)), NondetRegLayout(Reg(45u)), NondetRegLayout(Reg(46u)));
const kLayout__469: MemoryIOLayout = MemoryIOLayout(kLayout__470, kLayout__471);
const kLayout__468: MemoryPageInLayout = MemoryPageInLayout(kLayout__469);
const kLayout__474: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(47u)), NondetRegLayout(Reg(48u)), NondetRegLayout(Reg(49u)), NondetRegLayout(Reg(50u)), NondetRegLayout(Reg(51u)));
const kLayout__475: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(52u)), NondetRegLayout(Reg(48u)), NondetRegLayout(Reg(53u)), NondetRegLayout(Reg(54u)), NondetRegLayout(Reg(55u)));
const kLayout__473: MemoryIOLayout = MemoryIOLayout(kLayout__474, kLayout__475);
const kLayout__472: MemoryPageInLayout = MemoryPageInLayout(kLayout__473);
const kLayout__478: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(56u)), NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u)), NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u)));
const kLayout__479: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(61u)), NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(62u)), NondetRegLayout(Reg(63u)), NondetRegLayout(Reg(64u)));
const kLayout__477: MemoryIOLayout = MemoryIOLayout(kLayout__478, kLayout__479);
const kLayout__476: MemoryPageInLayout = MemoryPageInLayout(kLayout__477);
const kLayout__482: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(66u)), NondetRegLayout(Reg(67u)), NondetRegLayout(Reg(68u)), NondetRegLayout(Reg(69u)));
const kLayout__483: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(70u)), NondetRegLayout(Reg(66u)), NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u)), NondetRegLayout(Reg(73u)));
const kLayout__481: MemoryIOLayout = MemoryIOLayout(kLayout__482, kLayout__483);
const kLayout__480: MemoryPageInLayout = MemoryPageInLayout(kLayout__481);
const kLayout__486: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(74u)), NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u)), NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u)));
const kLayout__487: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(80u)), NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u)));
const kLayout__485: MemoryIOLayout = MemoryIOLayout(kLayout__486, kLayout__487);
const kLayout__484: MemoryPageInLayout = MemoryPageInLayout(kLayout__485);
const kLayout__490: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u)), NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u)), NondetRegLayout(Reg(87u)));
const kLayout__491: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(88u)), NondetRegLayout(Reg(84u)), NondetRegLayout(Reg(89u)), NondetRegLayout(Reg(90u)), NondetRegLayout(Reg(91u)));
const kLayout__489: MemoryIOLayout = MemoryIOLayout(kLayout__490, kLayout__491);
const kLayout__488: MemoryPageInLayout = MemoryPageInLayout(kLayout__489);
const kLayout__494: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(92u)), NondetRegLayout(Reg(93u)), NondetRegLayout(Reg(94u)), NondetRegLayout(Reg(95u)), NondetRegLayout(Reg(96u)));
const kLayout__495: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(97u)), NondetRegLayout(Reg(93u)), NondetRegLayout(Reg(98u)), NondetRegLayout(Reg(99u)), NondetRegLayout(Reg(100u)));
const kLayout__493: MemoryIOLayout = MemoryIOLayout(kLayout__494, kLayout__495);
const kLayout__492: MemoryPageInLayout = MemoryPageInLayout(kLayout__493);
const kLayout__463: MemoryPageInLayout8LayoutArray = MemoryPageInLayout8LayoutArray(kLayout__464, kLayout__468, kLayout__472, kLayout__476, kLayout__480, kLayout__484, kLayout__488, kLayout__492);
const kLayout__462: ControlLoadRootAndNonceLayout = ControlLoadRootAndNonceLayout(kLayout__463);
const kLayout__461: Control0_SuperArm0Layout = Control0_SuperArm0Layout(kLayout__462, CycleArgLayout(NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(102u))), CycleArgLayout(NondetRegLayout(Reg(103u)), NondetRegLayout(Reg(104u))), CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))), ArgU16Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU16Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU16Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU16Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU16Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU16Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU16Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU16Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU16Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU16Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU16Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU16Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU16Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU8Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU8Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU8Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU8Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))), ArgU8Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u))), ArgU8Layout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u))), ArgU8Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u))), ArgU8Layout(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u))), ArgU8Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u))), ArgU8Layout(NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u))), ArgU8Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u))), ArgU8Layout(NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u))), ArgU8Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), ArgU8Layout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))));
const kLayout__501: MemoryReadLayout = MemoryReadLayout(kLayout__465, kLayout__40);
const kLayout__502: MemoryReadLayout = MemoryReadLayout(kLayout__469, kLayout__147);
const kLayout__500: ControlResume_SuperArm0_SuperLayout = ControlResume_SuperArm0_SuperLayout(kLayout__501, kLayout__502);
const kLayout__499: ControlResume_SuperArm0Layout = ControlResume_SuperArm0Layout(kLayout__500, kLayout__474, kLayout__475, kLayout__478, kLayout__479, kLayout__482, kLayout__483, kLayout__486, kLayout__487, kLayout__490, kLayout__491, kLayout__494, kLayout__495, CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))));
const kLayout__506: MemoryWriteLayout = MemoryWriteLayout(kLayout__465, kLayout__40);
const kLayout__505: ControlResume_SuperArm1_Super__0_SuperLayout = ControlResume_SuperArm1_Super__0_SuperLayout(kLayout__506);
const kLayout__508: MemoryWriteLayout = MemoryWriteLayout(kLayout__469, kLayout__147);
const kLayout__507: ControlResume_SuperArm1_Super__0_SuperLayout = ControlResume_SuperArm1_Super__0_SuperLayout(kLayout__508);
const kLayout__512: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))));
const kLayout__511: IsForwardLayout = IsForwardLayout(kLayout__512);
const kLayout__510: MemoryWriteLayout = MemoryWriteLayout(kLayout__473, kLayout__511);
const kLayout__509: ControlResume_SuperArm1_Super__0_SuperLayout = ControlResume_SuperArm1_Super__0_SuperLayout(kLayout__510);
const kLayout__516: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))));
const kLayout__515: IsForwardLayout = IsForwardLayout(kLayout__516);
const kLayout__514: MemoryWriteLayout = MemoryWriteLayout(kLayout__477, kLayout__515);
const kLayout__513: ControlResume_SuperArm1_Super__0_SuperLayout = ControlResume_SuperArm1_Super__0_SuperLayout(kLayout__514);
const kLayout__520: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))));
const kLayout__519: IsForwardLayout = IsForwardLayout(kLayout__520);
const kLayout__518: MemoryWriteLayout = MemoryWriteLayout(kLayout__481, kLayout__519);
const kLayout__517: ControlResume_SuperArm1_Super__0_SuperLayout = ControlResume_SuperArm1_Super__0_SuperLayout(kLayout__518);
const kLayout__524: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))));
const kLayout__523: IsForwardLayout = IsForwardLayout(kLayout__524);
const kLayout__522: MemoryWriteLayout = MemoryWriteLayout(kLayout__485, kLayout__523);
const kLayout__521: ControlResume_SuperArm1_Super__0_SuperLayout = ControlResume_SuperArm1_Super__0_SuperLayout(kLayout__522);
const kLayout__528: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))));
const kLayout__527: IsForwardLayout = IsForwardLayout(kLayout__528);
const kLayout__526: MemoryWriteLayout = MemoryWriteLayout(kLayout__489, kLayout__527);
const kLayout__525: ControlResume_SuperArm1_Super__0_SuperLayout = ControlResume_SuperArm1_Super__0_SuperLayout(kLayout__526);
const kLayout__532: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))));
const kLayout__531: IsForwardLayout = IsForwardLayout(kLayout__532);
const kLayout__530: MemoryWriteLayout = MemoryWriteLayout(kLayout__493, kLayout__531);
const kLayout__529: ControlResume_SuperArm1_Super__0_SuperLayout = ControlResume_SuperArm1_Super__0_SuperLayout(kLayout__530);
const kLayout__504: ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray = ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray(kLayout__505, kLayout__507, kLayout__509, kLayout__513, kLayout__517, kLayout__521, kLayout__525, kLayout__529);
const kLayout__503: ControlResume_SuperArm1_SuperLayout = ControlResume_SuperArm1_SuperLayout(kLayout__504);
const kLayout__498: ControlResume_SuperLayout = ControlResume_SuperLayout(kLayout__499, kLayout__503);
const kLayout__534: MemoryArgLayout16LayoutArray = MemoryArgLayout16LayoutArray(kLayout__466, kLayout__467, kLayout__470, kLayout__471, kLayout__474, kLayout__475, kLayout__478, kLayout__479, kLayout__482, kLayout__483, kLayout__486, kLayout__487, kLayout__490, kLayout__491, kLayout__494, kLayout__495);
const kLayout__535: CycleArgLayout8LayoutArray = CycleArgLayout8LayoutArray(CycleArgLayout(NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(102u))), CycleArgLayout(NondetRegLayout(Reg(103u)), NondetRegLayout(Reg(104u))), CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))));
const kLayout__533: _Arguments_ControlResume_SuperLayout = _Arguments_ControlResume_SuperLayout(kLayout__534, kLayout__535);
const kLayout__497: ControlResumeLayout = ControlResumeLayout(kLayout__498, IsZeroLayout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u))), kLayout__533);
const kLayout__496: Control0_SuperArm1Layout = Control0_SuperArm1Layout(kLayout__497, ArgU16Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU16Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU16Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU16Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU16Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU16Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU16Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU16Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU16Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU16Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU16Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU16Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU16Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU8Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU8Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU8Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU8Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))), ArgU8Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u))), ArgU8Layout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u))), ArgU8Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u))), ArgU8Layout(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u))), ArgU8Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u))), ArgU8Layout(NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u))), ArgU8Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u))), ArgU8Layout(NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u))), ArgU8Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), ArgU8Layout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))));
const kLayout__542: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))));
const kLayout__543: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))));
const kLayout__541: NormalizeU32Layout = NormalizeU32Layout(kLayout__542, NondetRegLayout(Reg(181u)), kLayout__543, NondetRegLayout(Reg(182u)));
const kLayout__540: ControlUserEcallOrFence_SuperArm0_SuperLayout = ControlUserEcallOrFence_SuperArm0_SuperLayout(kLayout__541);
const kLayout__539: ControlUserEcallOrFence_SuperArm0Layout = ControlUserEcallOrFence_SuperArm0Layout(kLayout__540, kLayout__470, kLayout__471, kLayout__474, kLayout__475, CycleArgLayout(NondetRegLayout(Reg(103u)), NondetRegLayout(Reg(104u))), CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))));
const kLayout__545: ControlUserEcallOrFence_SuperArm1_SuperLayout = ControlUserEcallOrFence_SuperArm1_SuperLayout(kLayout__502, kLayout__510);
const kLayout__544: ControlUserEcallOrFence_SuperArm1Layout = ControlUserEcallOrFence_SuperArm1Layout(kLayout__545, ArgU16Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU16Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))));
const kLayout__538: ControlUserEcallOrFence_SuperLayout = ControlUserEcallOrFence_SuperLayout(kLayout__539, kLayout__544);
const kLayout__547: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))));
const kLayout__548: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))));
const kLayout__546: AddrDecomposeBitsLayout = AddrDecomposeBitsLayout(NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(185u)), kLayout__547, IsZeroLayout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u))), kLayout__548);
const kLayout__550: ArgU16Layout2LayoutArray = ArgU16Layout2LayoutArray(ArgU16Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU16Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))));
const kLayout__551: MemoryArgLayout4LayoutArray = MemoryArgLayout4LayoutArray(kLayout__470, kLayout__471, kLayout__474, kLayout__475);
const kLayout__552: CycleArgLayout2LayoutArray = CycleArgLayout2LayoutArray(CycleArgLayout(NondetRegLayout(Reg(103u)), NondetRegLayout(Reg(104u))), CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))));
const kLayout__549: _Arguments_ControlUserEcallOrFence_SuperLayout = _Arguments_ControlUserEcallOrFence_SuperLayout(kLayout__550, kLayout__551, kLayout__552);
const kLayout__537: ControlUserEcallOrFenceLayout = ControlUserEcallOrFenceLayout(kLayout__538, NondetRegLayout(Reg(183u)), kLayout__546, kLayout__501, NondetRegLayout(Reg(188u)), kLayout__549);
const kLayout__536: Control0_SuperArm2Layout = Control0_SuperArm2Layout(kLayout__537, kLayout__478, kLayout__479, kLayout__482, kLayout__483, kLayout__486, kLayout__487, kLayout__490, kLayout__491, kLayout__494, kLayout__495, CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))), ArgU16Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU16Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU16Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU16Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU16Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU16Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU16Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU16Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU16Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU16Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU8Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU8Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU8Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU8Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))), ArgU8Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u))), ArgU8Layout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u))), ArgU8Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u))), ArgU8Layout(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u))), ArgU8Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u))), ArgU8Layout(NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u))), ArgU8Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u))), ArgU8Layout(NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u))), ArgU8Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), ArgU8Layout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))));
const kLayout__555: AddrDecomposeBitsLayout = AddrDecomposeBitsLayout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u)), kLayout__547, IsZeroLayout(NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(185u))), kLayout__548);
const kLayout__556: NormalizeU32Layout = NormalizeU32Layout(kLayout__542, NondetRegLayout(Reg(186u)), kLayout__543, NondetRegLayout(Reg(187u)));
const kLayout__554: ControlMRETLayout = ControlMRETLayout(NondetRegLayout(Reg(181u)), kLayout__555, kLayout__501, kLayout__502, kLayout__556);
const kLayout__553: Control0_SuperArm3Layout = Control0_SuperArm3Layout(kLayout__554, kLayout__474, kLayout__475, kLayout__478, kLayout__479, kLayout__482, kLayout__483, kLayout__486, kLayout__487, kLayout__490, kLayout__491, kLayout__494, kLayout__495, CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))), ArgU16Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU16Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU16Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU16Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU16Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU16Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU16Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU16Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU16Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU16Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU8Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU8Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU8Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU8Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))), ArgU8Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u))), ArgU8Layout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u))), ArgU8Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u))), ArgU8Layout(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u))), ArgU8Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u))), ArgU8Layout(NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u))), ArgU8Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u))), ArgU8Layout(NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u))), ArgU8Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), ArgU8Layout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))));
const kLayout__562: MemoryReadLayout = MemoryReadLayout(kLayout__473, kLayout__511);
const kLayout__563: MemoryReadLayout = MemoryReadLayout(kLayout__477, kLayout__515);
const kLayout__564: MemoryReadLayout = MemoryReadLayout(kLayout__481, kLayout__519);
const kLayout__565: MemoryReadLayout = MemoryReadLayout(kLayout__485, kLayout__523);
const kLayout__566: MemoryReadLayout = MemoryReadLayout(kLayout__489, kLayout__527);
const kLayout__567: MemoryReadLayout = MemoryReadLayout(kLayout__493, kLayout__531);
const kLayout__561: MemoryReadLayout8LayoutArray = MemoryReadLayout8LayoutArray(kLayout__501, kLayout__502, kLayout__562, kLayout__563, kLayout__564, kLayout__565, kLayout__566, kLayout__567);
const kLayout__560: ControlSuspend_SuperArm0_SuperLayout = ControlSuspend_SuperArm0_SuperLayout(kLayout__561);
const kLayout__569: ControlSuspend_SuperArm1_SuperLayout = ControlSuspend_SuperArm1_SuperLayout(NondetRegLayout(Reg(181u)), kLayout__506, kLayout__508);
const kLayout__568: ControlSuspend_SuperArm1Layout = ControlSuspend_SuperArm1Layout(kLayout__569, kLayout__474, kLayout__475, kLayout__478, kLayout__479, kLayout__482, kLayout__483, kLayout__486, kLayout__487, kLayout__490, kLayout__491, kLayout__494, kLayout__495, CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))));
const kLayout__559: ControlSuspend_SuperLayout = ControlSuspend_SuperLayout(kLayout__560, kLayout__568);
const kLayout__570: _Arguments_ControlSuspend_SuperLayout = _Arguments_ControlSuspend_SuperLayout(kLayout__534, kLayout__535);
const kLayout__558: ControlSuspendLayout = ControlSuspendLayout(kLayout__559, IsZeroLayout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u))), kLayout__570);
const kLayout__557: Control0_SuperArm4Layout = Control0_SuperArm4Layout(kLayout__558, ArgU16Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU16Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU16Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU16Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU16Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU16Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU16Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU16Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU16Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU16Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU16Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU16Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU16Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU8Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU8Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU8Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU8Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))), ArgU8Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u))), ArgU8Layout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u))), ArgU8Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u))), ArgU8Layout(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u))), ArgU8Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u))), ArgU8Layout(NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u))), ArgU8Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u))), ArgU8Layout(NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u))), ArgU8Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), ArgU8Layout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))));
const kLayout__574: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__465, kLayout__40);
const kLayout__575: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__469, kLayout__147);
const kLayout__576: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__473, kLayout__511);
const kLayout__577: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__477, kLayout__515);
const kLayout__578: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__481, kLayout__519);
const kLayout__579: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__485, kLayout__523);
const kLayout__580: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__489, kLayout__527);
const kLayout__581: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__493, kLayout__531);
const kLayout__573: MemoryPageOutLayout8LayoutArray = MemoryPageOutLayout8LayoutArray(kLayout__574, kLayout__575, kLayout__576, kLayout__577, kLayout__578, kLayout__579, kLayout__580, kLayout__581);
const kLayout__572: ControlStoreRootLayout = ControlStoreRootLayout(kLayout__573);
const kLayout__571: Control0_SuperArm5Layout = Control0_SuperArm5Layout(kLayout__572, ArgU16Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU16Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU16Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU16Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU16Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU16Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU16Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU16Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU16Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU16Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU16Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU16Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU16Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU8Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU8Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU8Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU8Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))), ArgU8Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u))), ArgU8Layout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u))), ArgU8Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u))), ArgU8Layout(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u))), ArgU8Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u))), ArgU8Layout(NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u))), ArgU8Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u))), ArgU8Layout(NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u))), ArgU8Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), ArgU8Layout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))));
const kLayout__588: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))));
const kLayout__589: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))));
const kLayout__590: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))));
const kLayout__591: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))));
const kLayout__592: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))));
const kLayout__593: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))));
const kLayout__594: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))));
const kLayout__595: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))));
const kLayout__596: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))));
const kLayout__597: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))));
const kLayout__598: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))));
const kLayout__599: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))));
const kLayout__600: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))));
const kLayout__601: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))));
const kLayout__602: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))));
const kLayout__603: ControlTable_SuperArm0_Super__0_SuperLayout = ControlTable_SuperArm0_Super__0_SuperLayout(ArgU16Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))));
const kLayout__587: ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray = ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray(kLayout__588, kLayout__589, kLayout__590, kLayout__591, kLayout__592, kLayout__593, kLayout__594, kLayout__595, kLayout__596, kLayout__597, kLayout__598, kLayout__599, kLayout__600, kLayout__601, kLayout__602, kLayout__603);
const kLayout__586: ControlTable_SuperArm0_SuperLayout = ControlTable_SuperArm0_SuperLayout(kLayout__587, IsZeroLayout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u))));
const kLayout__585: ControlTable_SuperArm0Layout = ControlTable_SuperArm0Layout(kLayout__586, ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU8Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU8Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU8Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU8Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))), ArgU8Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u))), ArgU8Layout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u))), ArgU8Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u))), ArgU8Layout(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u))), ArgU8Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u))), ArgU8Layout(NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u))), ArgU8Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u))), ArgU8Layout(NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u))), ArgU8Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), ArgU8Layout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))));
const kLayout__607: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))));
const kLayout__608: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))));
const kLayout__609: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))));
const kLayout__610: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))));
const kLayout__611: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))));
const kLayout__612: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))));
const kLayout__613: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u))));
const kLayout__614: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u))));
const kLayout__615: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u))));
const kLayout__616: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u))));
const kLayout__617: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u))));
const kLayout__618: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u))));
const kLayout__619: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u))));
const kLayout__620: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u))));
const kLayout__621: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))));
const kLayout__622: ControlTable_SuperArm1_Super__0_SuperLayout = ControlTable_SuperArm1_Super__0_SuperLayout(ArgU8Layout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))));
const kLayout__606: ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray = ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray(kLayout__607, kLayout__608, kLayout__609, kLayout__610, kLayout__611, kLayout__612, kLayout__613, kLayout__614, kLayout__615, kLayout__616, kLayout__617, kLayout__618, kLayout__619, kLayout__620, kLayout__621, kLayout__622);
const kLayout__605: ControlTable_SuperArm1_SuperLayout = ControlTable_SuperArm1_SuperLayout(kLayout__606, IsZeroLayout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u))));
const kLayout__604: ControlTable_SuperArm1Layout = ControlTable_SuperArm1Layout(kLayout__605, ArgU16Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU16Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU16Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU16Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU16Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU16Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU16Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU16Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU16Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU16Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU16Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU16Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU16Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))));
const kLayout__584: ControlTable_SuperLayout = ControlTable_SuperLayout(kLayout__585, kLayout__604);
const kLayout__624: ArgU16Layout16LayoutArray = ArgU16Layout16LayoutArray(ArgU16Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU16Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU16Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU16Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU16Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU16Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU16Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU16Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU16Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU16Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU16Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU16Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU16Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))));
const kLayout__625: ArgU8Layout16LayoutArray = ArgU8Layout16LayoutArray(ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU8Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU8Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU8Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU8Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))), ArgU8Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u))), ArgU8Layout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u))), ArgU8Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u))), ArgU8Layout(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u))), ArgU8Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u))), ArgU8Layout(NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u))), ArgU8Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u))), ArgU8Layout(NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u))), ArgU8Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), ArgU8Layout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))));
const kLayout__623: _Arguments_ControlTable_SuperLayout = _Arguments_ControlTable_SuperLayout(kLayout__624, kLayout__625);
const kLayout__583: ControlTableLayout = ControlTableLayout(kLayout__584, NondetRegLayout(Reg(183u)), NondetRegLayout(Reg(184u)), kLayout__623);
const kLayout__582: Control0_SuperArm6Layout = Control0_SuperArm6Layout(kLayout__583, kLayout__466, kLayout__467, kLayout__470, kLayout__471, kLayout__474, kLayout__475, kLayout__478, kLayout__479, kLayout__482, kLayout__483, kLayout__486, kLayout__487, kLayout__490, kLayout__491, kLayout__494, kLayout__495, CycleArgLayout(NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(102u))), CycleArgLayout(NondetRegLayout(Reg(103u)), NondetRegLayout(Reg(104u))), CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))));
const kLayout__628: _Arguments_ControlDone__0Layout = _Arguments_ControlDone__0Layout(CycleArgLayout1LayoutArray(CycleArgLayout(NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(102u)))));
const kLayout__630: ControlDone__0Arm0_SuperLayout = ControlDone__0Arm0_SuperLayout(kLayout__41);
const kLayout__631: ControlDone__0Arm1Layout = ControlDone__0Arm1Layout(CycleArgLayout(NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(102u))));
const kLayout__629: ControlDone__0Layout = ControlDone__0Layout(kLayout__630, kLayout__631);
const kLayout__627: ControlDoneLayout = ControlDoneLayout(kLayout__628, kLayout__629);
const kLayout__626: Control0_SuperArm7Layout = Control0_SuperArm7Layout(kLayout__627, kLayout__466, kLayout__467, kLayout__470, kLayout__471, kLayout__474, kLayout__475, kLayout__478, kLayout__479, kLayout__482, kLayout__483, kLayout__486, kLayout__487, kLayout__490, kLayout__491, kLayout__494, kLayout__495, CycleArgLayout(NondetRegLayout(Reg(103u)), NondetRegLayout(Reg(104u))), CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))), ArgU16Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU16Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU16Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU16Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU16Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU16Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU16Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU16Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU16Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU16Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU16Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU16Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU16Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU8Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU8Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU8Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU8Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))), ArgU8Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u))), ArgU8Layout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u))), ArgU8Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u))), ArgU8Layout(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u))), ArgU8Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u))), ArgU8Layout(NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u))), ArgU8Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u))), ArgU8Layout(NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u))), ArgU8Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), ArgU8Layout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))));
const kLayout__460: Control0_SuperLayout = Control0_SuperLayout(kLayout__461, kLayout__496, kLayout__536, kLayout__553, kLayout__557, kLayout__571, kLayout__582, kLayout__626);
const kLayout__632: DoCycleTableLayout = DoCycleTableLayout(CycleArgLayout(NondetRegLayout(Reg(189u)), NondetRegLayout(Reg(190u))), CycleArgLayout(NondetRegLayout(Reg(191u)), NondetRegLayout(Reg(192u))));
const kLayout__633: _Arguments_Control0_SuperLayout = _Arguments_Control0_SuperLayout(kLayout__534, kLayout__535, kLayout__624, kLayout__625);
const kLayout__459: Control0Layout = Control0Layout(kLayout__460, kLayout__632, kLayout__633);
const kLayout__635: DoCycleTableLayout = DoCycleTableLayout(CycleArgLayout(NondetRegLayout(Reg(92u)), NondetRegLayout(Reg(93u))), CycleArgLayout(NondetRegLayout(Reg(94u)), NondetRegLayout(Reg(95u))));
const kLayout__637: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(98u)), NondetRegLayout(Reg(99u))));
const kLayout__638: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(102u)), NondetRegLayout(Reg(103u))));
const kLayout__636: AddrDecomposeBitsLayout = AddrDecomposeBitsLayout(NondetRegLayout(Reg(96u)), NondetRegLayout(Reg(97u)), kLayout__637, IsZeroLayout(NondetRegLayout(Reg(100u)), NondetRegLayout(Reg(101u))), kLayout__638);
const kLayout__640: MemoryArgLayout8LayoutArray = MemoryArgLayout8LayoutArray(kLayout__466, kLayout__467, kLayout__470, kLayout__471, kLayout__474, kLayout__475, kLayout__478, kLayout__479);
const kLayout__641: CycleArgLayout4LayoutArray = CycleArgLayout4LayoutArray(CycleArgLayout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(66u))), CycleArgLayout(NondetRegLayout(Reg(67u)), NondetRegLayout(Reg(68u))), CycleArgLayout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u))), CycleArgLayout(NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u))));
const kLayout__642: ArgU16Layout4LayoutArray = ArgU16Layout4LayoutArray(ArgU16Layout(NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(74u))), ArgU16Layout(NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u))), ArgU16Layout(NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u))));
const kLayout__643: ArgU8Layout4LayoutArray = ArgU8Layout4LayoutArray(ArgU8Layout(NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u))), ArgU8Layout(NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u))), ArgU8Layout(NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u))), ArgU8Layout(NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u))));
const kLayout__639: _Arguments_ECall0OutputLayout = _Arguments_ECall0OutputLayout(kLayout__640, kLayout__641, kLayout__642, kLayout__643);
const kLayout__649: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(66u))));
const kLayout__648: IsForwardLayout = IsForwardLayout(kLayout__649);
const kLayout__647: MemoryReadLayout = MemoryReadLayout(kLayout__465, kLayout__648);
const kLayout__652: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(67u)), NondetRegLayout(Reg(68u))));
const kLayout__651: IsForwardLayout = IsForwardLayout(kLayout__652);
const kLayout__650: MemoryReadLayout = MemoryReadLayout(kLayout__469, kLayout__651);
const kLayout__654: NondetRegLayout6LayoutArray = NondetRegLayout6LayoutArray(NondetRegLayout(Reg(104u)), NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u)), NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u)), NondetRegLayout(Reg(109u)));
const kLayout__653: OneHot_6_Layout = OneHot_6_Layout(kLayout__654);
const kLayout__646: MachineECallLayout = MachineECallLayout(kLayout__647, kLayout__650, kLayout__653);
const kLayout__645: ECall0OutputArm0Layout = ECall0OutputArm0Layout(kLayout__646, kLayout__474, kLayout__475, kLayout__478, kLayout__479, CycleArgLayout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u))), CycleArgLayout(NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u))), ArgU16Layout(NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(74u))), ArgU16Layout(NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u))), ArgU16Layout(NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u))), ArgU8Layout(NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u))), ArgU8Layout(NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u))), ArgU8Layout(NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u))), ArgU8Layout(NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u))));
const kLayout__656: ECallTerminateLayout = ECallTerminateLayout(kLayout__647, kLayout__650);
const kLayout__655: ECall0OutputArm1Layout = ECall0OutputArm1Layout(kLayout__656, kLayout__474, kLayout__475, kLayout__478, kLayout__479, CycleArgLayout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u))), CycleArgLayout(NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u))), ArgU16Layout(NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(74u))), ArgU16Layout(NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u))), ArgU16Layout(NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u))), ArgU8Layout(NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u))), ArgU8Layout(NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u))), ArgU8Layout(NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u))), ArgU8Layout(NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u))));
const kLayout__661: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u))));
const kLayout__660: IsForwardLayout = IsForwardLayout(kLayout__661);
const kLayout__659: MemoryReadLayout = MemoryReadLayout(kLayout__473, kLayout__660);
const kLayout__662: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(74u))));
const kLayout__663: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u))));
const kLayout__666: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u))));
const kLayout__665: IsForwardLayout = IsForwardLayout(kLayout__666);
const kLayout__664: MemoryWriteLayout = MemoryWriteLayout(kLayout__477, kLayout__665);
const kLayout__668: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u))));
const kLayout__670: NondetRegLayout4LayoutArray = NondetRegLayout4LayoutArray(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u)), NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u)));
const kLayout__669: OneHot_4_Layout = OneHot_4_Layout(kLayout__670);
const kLayout__667: DecomposeLow2Layout = DecomposeLow2Layout(kLayout__668, NondetRegLayout(Reg(104u)), kLayout__669, IsZeroLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), NondetRegLayout(Reg(111u)));
const kLayout__672: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u))));
const kLayout__674: NondetRegLayout4LayoutArray = NondetRegLayout4LayoutArray(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u)), NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u)));
const kLayout__673: OneHot_4_Layout = OneHot_4_Layout(kLayout__674);
const kLayout__671: DecomposeLow2Layout = DecomposeLow2Layout(kLayout__672, NondetRegLayout(Reg(112u)), kLayout__673, IsZeroLayout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), NondetRegLayout(Reg(119u)));
const kLayout__658: ECallHostReadSetupLayout = ECallHostReadSetupLayout(kLayout__647, kLayout__650, kLayout__659, kLayout__662, kLayout__663, kLayout__664, kLayout__667, kLayout__671, NondetRegLayout(Reg(120u)), NondetRegLayout(Reg(121u)));
const kLayout__657: ECall0OutputArm2Layout = ECall0OutputArm2Layout(kLayout__658, ArgU8Layout(NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u))), ArgU8Layout(NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u))), ArgU8Layout(NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u))), ArgU8Layout(NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u))));
const kLayout__676: ECallHostWriteLayout = ECallHostWriteLayout(kLayout__647, kLayout__650, kLayout__659, kLayout__662, kLayout__663, kLayout__664);
const kLayout__675: ECall0OutputArm3Layout = ECall0OutputArm3Layout(kLayout__676, ArgU16Layout(NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u))), ArgU16Layout(NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u))), ArgU8Layout(NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u))), ArgU8Layout(NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u))), ArgU8Layout(NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u))), ArgU8Layout(NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u))));
const kLayout__679: DecomposeLow2Layout = DecomposeLow2Layout(kLayout__662, NondetRegLayout(Reg(104u)), kLayout__669, IsZeroLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), NondetRegLayout(Reg(111u)));
const kLayout__680: MemoryWriteUnconstrainedLayout = MemoryWriteUnconstrainedLayout(kLayout__469, kLayout__651);
const kLayout__681: SplitWordLayout = SplitWordLayout(kLayout__280, kLayout__282);
const kLayout__683: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u))));
const kLayout__682: SplitWordLayout = SplitWordLayout(kLayout__285, kLayout__683);
const kLayout__678: ECallHostReadBytesLayout = ECallHostReadBytesLayout(kLayout__679, NondetRegLayout(Reg(112u)), IsZeroLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), NondetRegLayout(Reg(115u)), IsZeroLayout(NondetRegLayout(Reg(116u)), NondetRegLayout(Reg(117u))), NondetRegLayout(Reg(118u)), NondetRegLayout(Reg(119u)), kLayout__647, kLayout__680, kLayout__681, kLayout__682);
const kLayout__677: ECall0OutputArm4Layout = ECall0OutputArm4Layout(kLayout__678, kLayout__474, kLayout__475, kLayout__478, kLayout__479, CycleArgLayout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u))), CycleArgLayout(NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u))), ArgU16Layout(NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u))), ArgU16Layout(NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u))));
const kLayout__686: DecomposeLow2Layout = DecomposeLow2Layout(kLayout__663, NondetRegLayout(Reg(112u)), kLayout__673, IsZeroLayout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), NondetRegLayout(Reg(119u)));
const kLayout__689: MemoryWriteUnconstrainedLayout = MemoryWriteUnconstrainedLayout(kLayout__465, kLayout__648);
const kLayout__688: ECallHostReadWords__0_SuperLayout = ECallHostReadWords__0_SuperLayout(NondetRegLayout(Reg(120u)), kLayout__689);
const kLayout__690: ECallHostReadWords__0_SuperLayout = ECallHostReadWords__0_SuperLayout(NondetRegLayout(Reg(121u)), kLayout__680);
const kLayout__692: MemoryWriteUnconstrainedLayout = MemoryWriteUnconstrainedLayout(kLayout__473, kLayout__660);
const kLayout__691: ECallHostReadWords__0_SuperLayout = ECallHostReadWords__0_SuperLayout(NondetRegLayout(Reg(122u)), kLayout__692);
const kLayout__694: MemoryWriteUnconstrainedLayout = MemoryWriteUnconstrainedLayout(kLayout__477, kLayout__665);
const kLayout__693: ECallHostReadWords__0_SuperLayout = ECallHostReadWords__0_SuperLayout(NondetRegLayout(Reg(123u)), kLayout__694);
const kLayout__687: ECallHostReadWords__0_SuperLayout4LayoutArray = ECallHostReadWords__0_SuperLayout4LayoutArray(kLayout__688, kLayout__690, kLayout__691, kLayout__693);
const kLayout__685: ECallHostReadWordsLayout = ECallHostReadWordsLayout(kLayout__679, kLayout__686, kLayout__687, IsZeroLayout(NondetRegLayout(Reg(124u)), NondetRegLayout(Reg(125u))), NondetRegLayout(Reg(126u)));
const kLayout__684: ECall0OutputArm5Layout = ECall0OutputArm5Layout(kLayout__685, ArgU16Layout(NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u))), ArgU16Layout(NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u))), ArgU8Layout(NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u))), ArgU8Layout(NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u))), ArgU8Layout(NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u))), ArgU8Layout(NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u))));
const kLayout__695: ECall0OutputArm6Layout = ECall0OutputArm6Layout(kLayout__466, kLayout__467, kLayout__470, kLayout__471, kLayout__474, kLayout__475, kLayout__478, kLayout__479, CycleArgLayout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(66u))), CycleArgLayout(NondetRegLayout(Reg(67u)), NondetRegLayout(Reg(68u))), CycleArgLayout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u))), CycleArgLayout(NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u))), ArgU16Layout(NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(74u))), ArgU16Layout(NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u))), ArgU16Layout(NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u))), ArgU8Layout(NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u))), ArgU8Layout(NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u))), ArgU8Layout(NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u))), ArgU8Layout(NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u))));
const kLayout__696: ECall0OutputArm7Layout = ECall0OutputArm7Layout(kLayout__466, kLayout__467, kLayout__470, kLayout__471, kLayout__474, kLayout__475, kLayout__478, kLayout__479, CycleArgLayout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(66u))), CycleArgLayout(NondetRegLayout(Reg(67u)), NondetRegLayout(Reg(68u))), CycleArgLayout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u))), CycleArgLayout(NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u))), ArgU16Layout(NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(74u))), ArgU16Layout(NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u))), ArgU16Layout(NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u))), ArgU8Layout(NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u))), ArgU8Layout(NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u))), ArgU8Layout(NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u))), ArgU8Layout(NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u))));
const kLayout__644: ECall0OutputLayout = ECall0OutputLayout(kLayout__645, kLayout__655, kLayout__657, kLayout__675, kLayout__677, kLayout__684, kLayout__695, kLayout__696);
const kLayout__698: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))));
const kLayout__697: NormalizeU32Layout = NormalizeU32Layout(kLayout__698, NondetRegLayout(Reg(139u)), kLayout__457, NondetRegLayout(Reg(142u)));
const kLayout__634: ECall0Layout = ECall0Layout(NondetRegLayout(Reg(89u)), NondetRegLayout(Reg(90u)), NondetRegLayout(Reg(91u)), kLayout__635, kLayout__636, kLayout__639, kLayout__644, IsZeroLayout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), IsZeroLayout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), IsZeroLayout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), IsZeroLayout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), IsZeroLayout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), kLayout__697);
const kLayout__700: DoCycleTableLayout = DoCycleTableLayout(CycleArgLayout(NondetRegLayout(Reg(192u)), NondetRegLayout(Reg(193u))), CycleArgLayout(NondetRegLayout(Reg(194u)), NondetRegLayout(Reg(195u))));
const kLayout__702: NondetRegLayout24LayoutArray = NondetRegLayout24LayoutArray(NondetRegLayout(Reg(40u)), NondetRegLayout(Reg(41u)), NondetRegLayout(Reg(42u)), NondetRegLayout(Reg(43u)), NondetRegLayout(Reg(44u)), NondetRegLayout(Reg(45u)), NondetRegLayout(Reg(46u)), NondetRegLayout(Reg(47u)), NondetRegLayout(Reg(48u)), NondetRegLayout(Reg(49u)), NondetRegLayout(Reg(50u)), NondetRegLayout(Reg(51u)), NondetRegLayout(Reg(52u)), NondetRegLayout(Reg(53u)), NondetRegLayout(Reg(54u)), NondetRegLayout(Reg(55u)), NondetRegLayout(Reg(56u)), NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u)), NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u)), NondetRegLayout(Reg(61u)), NondetRegLayout(Reg(62u)), NondetRegLayout(Reg(63u)));
const kLayout__701: PoseidonStateLayout = PoseidonStateLayout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u)), NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u)), NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u)), NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u)), NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u)), NondetRegLayout(Reg(39u)), kLayout__702, NondetExtRegLayout(Reg(64u)));
const kLayout__705: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(68u)), NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u)), NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u)));
const kLayout__706: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(74u)), NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u)));
const kLayout__707: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u)), NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u)), NondetRegLayout(Reg(81u)));
const kLayout__708: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(82u)), NondetRegLayout(Reg(78u)), NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u)), NondetRegLayout(Reg(85u)));
const kLayout__709: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(86u)), NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u)), NondetRegLayout(Reg(89u)), NondetRegLayout(Reg(90u)));
const kLayout__710: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(91u)), NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(92u)), NondetRegLayout(Reg(93u)), NondetRegLayout(Reg(94u)));
const kLayout__711: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(95u)), NondetRegLayout(Reg(96u)), NondetRegLayout(Reg(97u)), NondetRegLayout(Reg(98u)), NondetRegLayout(Reg(99u)));
const kLayout__712: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(100u)), NondetRegLayout(Reg(96u)), NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(102u)), NondetRegLayout(Reg(103u)));
const kLayout__713: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(104u)), NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u)), NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u)));
const kLayout__714: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(110u)), NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u)));
const kLayout__715: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u)), NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u)), NondetRegLayout(Reg(117u)));
const kLayout__716: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(118u)), NondetRegLayout(Reg(114u)), NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u)), NondetRegLayout(Reg(121u)));
const kLayout__717: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(122u)), NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u)), NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u)));
const kLayout__718: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(128u)), NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u)));
const kLayout__719: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u)), NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u)), NondetRegLayout(Reg(135u)));
const kLayout__720: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(136u)), NondetRegLayout(Reg(132u)), NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u)), NondetRegLayout(Reg(139u)));
const kLayout__704: MemoryArgLayout16LayoutArray = MemoryArgLayout16LayoutArray(kLayout__705, kLayout__706, kLayout__707, kLayout__708, kLayout__709, kLayout__710, kLayout__711, kLayout__712, kLayout__713, kLayout__714, kLayout__715, kLayout__716, kLayout__717, kLayout__718, kLayout__719, kLayout__720);
const kLayout__721: CycleArgLayout8LayoutArray = CycleArgLayout8LayoutArray(CycleArgLayout(NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u))), CycleArgLayout(NondetRegLayout(Reg(142u)), NondetRegLayout(Reg(143u))), CycleArgLayout(NondetRegLayout(Reg(144u)), NondetRegLayout(Reg(145u))), CycleArgLayout(NondetRegLayout(Reg(146u)), NondetRegLayout(Reg(147u))), CycleArgLayout(NondetRegLayout(Reg(148u)), NondetRegLayout(Reg(149u))), CycleArgLayout(NondetRegLayout(Reg(150u)), NondetRegLayout(Reg(151u))), CycleArgLayout(NondetRegLayout(Reg(152u)), NondetRegLayout(Reg(153u))), CycleArgLayout(NondetRegLayout(Reg(154u)), NondetRegLayout(Reg(155u))));
const kLayout__722: ArgU16Layout24LayoutArray = ArgU16Layout24LayoutArray(ArgU16Layout(NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(75u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(158u)), NondetRegLayout(Reg(159u))), ArgU16Layout(NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(84u))), ArgU16Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(85u))), ArgU16Layout(NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u))), ArgU16Layout(NondetRegLayout(Reg(164u)), NondetRegLayout(Reg(93u))), ArgU16Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(94u))), ArgU16Layout(NondetRegLayout(Reg(166u)), NondetRegLayout(Reg(167u))), ArgU16Layout(NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(102u))), ArgU16Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(103u))), ArgU16Layout(NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u))), ArgU16Layout(NondetRegLayout(Reg(172u)), NondetRegLayout(Reg(111u))), ArgU16Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(112u))), ArgU16Layout(NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u))), ArgU16Layout(NondetRegLayout(Reg(176u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(121u))), ArgU16Layout(NondetRegLayout(Reg(178u)), NondetRegLayout(Reg(179u))), ArgU16Layout(NondetRegLayout(Reg(180u)), NondetRegLayout(Reg(129u))), ArgU16Layout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u))), ArgU16Layout(NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(139u))), ArgU16Layout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u))));
const kLayout__723: ArgU8Layout2LayoutArray = ArgU8Layout2LayoutArray(ArgU8Layout(NondetRegLayout(Reg(188u)), NondetRegLayout(Reg(189u))), ArgU8Layout(NondetRegLayout(Reg(190u)), NondetRegLayout(Reg(191u))));
const kLayout__703: _Arguments_Poseidon0StateLayout = _Arguments_Poseidon0StateLayout(kLayout__704, kLayout__721, kLayout__722, kLayout__723);
const kLayout__728: PoseidonEntry_SuperArm0Layout = PoseidonEntry_SuperArm0Layout(kLayout__701, kLayout__705, kLayout__706, kLayout__707, kLayout__708, kLayout__709, kLayout__710, kLayout__711, kLayout__712, CycleArgLayout(NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u))), CycleArgLayout(NondetRegLayout(Reg(142u)), NondetRegLayout(Reg(143u))), CycleArgLayout(NondetRegLayout(Reg(144u)), NondetRegLayout(Reg(145u))), CycleArgLayout(NondetRegLayout(Reg(146u)), NondetRegLayout(Reg(147u))));
const kLayout__732: MemoryIOLayout = MemoryIOLayout(kLayout__705, kLayout__706);
const kLayout__734: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u))));
const kLayout__733: IsForwardLayout = IsForwardLayout(kLayout__734);
const kLayout__731: MemoryReadLayout = MemoryReadLayout(kLayout__732, kLayout__733);
const kLayout__730: ReadAddrLayout = ReadAddrLayout(kLayout__731);
const kLayout__737: MemoryIOLayout = MemoryIOLayout(kLayout__707, kLayout__708);
const kLayout__739: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(142u)), NondetRegLayout(Reg(143u))));
const kLayout__738: IsForwardLayout = IsForwardLayout(kLayout__739);
const kLayout__736: MemoryReadLayout = MemoryReadLayout(kLayout__737, kLayout__738);
const kLayout__735: ReadAddrLayout = ReadAddrLayout(kLayout__736);
const kLayout__742: MemoryIOLayout = MemoryIOLayout(kLayout__709, kLayout__710);
const kLayout__744: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(144u)), NondetRegLayout(Reg(145u))));
const kLayout__743: IsForwardLayout = IsForwardLayout(kLayout__744);
const kLayout__741: MemoryReadLayout = MemoryReadLayout(kLayout__742, kLayout__743);
const kLayout__740: ReadAddrLayout = ReadAddrLayout(kLayout__741);
const kLayout__746: MemoryIOLayout = MemoryIOLayout(kLayout__711, kLayout__712);
const kLayout__748: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(146u)), NondetRegLayout(Reg(147u))));
const kLayout__747: IsForwardLayout = IsForwardLayout(kLayout__748);
const kLayout__745: MemoryReadLayout = MemoryReadLayout(kLayout__746, kLayout__747);
const kLayout__729: PoseidonEcallLayout = PoseidonEcallLayout(kLayout__701, kLayout__730, kLayout__735, kLayout__740, kLayout__745, IsZeroLayout(NondetRegLayout(Reg(196u)), NondetRegLayout(Reg(197u))), NondetRegLayout(Reg(198u)), NondetRegLayout(Reg(199u)), IsZeroLayout(NondetRegLayout(Reg(200u)), NondetRegLayout(Reg(201u))));
const kLayout__727: PoseidonEntry_SuperLayout = PoseidonEntry_SuperLayout(kLayout__701, kLayout__728, kLayout__729);
const kLayout__750: MemoryArgLayout8LayoutArray = MemoryArgLayout8LayoutArray(kLayout__705, kLayout__706, kLayout__707, kLayout__708, kLayout__709, kLayout__710, kLayout__711, kLayout__712);
const kLayout__751: CycleArgLayout4LayoutArray = CycleArgLayout4LayoutArray(CycleArgLayout(NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u))), CycleArgLayout(NondetRegLayout(Reg(142u)), NondetRegLayout(Reg(143u))), CycleArgLayout(NondetRegLayout(Reg(144u)), NondetRegLayout(Reg(145u))), CycleArgLayout(NondetRegLayout(Reg(146u)), NondetRegLayout(Reg(147u))));
const kLayout__749: _Arguments_PoseidonEntry_SuperLayout = _Arguments_PoseidonEntry_SuperLayout(kLayout__750, kLayout__751);
const kLayout__726: PoseidonEntryLayout = PoseidonEntryLayout(kLayout__727, IsZeroLayout(NondetRegLayout(Reg(202u)), NondetRegLayout(Reg(203u))), kLayout__749);
const kLayout__725: Poseidon0StateArm0Layout = Poseidon0StateArm0Layout(kLayout__726, kLayout__713, kLayout__714, kLayout__715, kLayout__716, kLayout__717, kLayout__718, kLayout__719, kLayout__720, CycleArgLayout(NondetRegLayout(Reg(148u)), NondetRegLayout(Reg(149u))), CycleArgLayout(NondetRegLayout(Reg(150u)), NondetRegLayout(Reg(151u))), CycleArgLayout(NondetRegLayout(Reg(152u)), NondetRegLayout(Reg(153u))), CycleArgLayout(NondetRegLayout(Reg(154u)), NondetRegLayout(Reg(155u))), ArgU16Layout(NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(75u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(158u)), NondetRegLayout(Reg(159u))), ArgU16Layout(NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(84u))), ArgU16Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(85u))), ArgU16Layout(NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u))), ArgU16Layout(NondetRegLayout(Reg(164u)), NondetRegLayout(Reg(93u))), ArgU16Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(94u))), ArgU16Layout(NondetRegLayout(Reg(166u)), NondetRegLayout(Reg(167u))), ArgU16Layout(NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(102u))), ArgU16Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(103u))), ArgU16Layout(NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u))), ArgU16Layout(NondetRegLayout(Reg(172u)), NondetRegLayout(Reg(111u))), ArgU16Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(112u))), ArgU16Layout(NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u))), ArgU16Layout(NondetRegLayout(Reg(176u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(121u))), ArgU16Layout(NondetRegLayout(Reg(178u)), NondetRegLayout(Reg(179u))), ArgU16Layout(NondetRegLayout(Reg(180u)), NondetRegLayout(Reg(129u))), ArgU16Layout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u))), ArgU16Layout(NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(139u))), ArgU16Layout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u))), ArgU8Layout(NondetRegLayout(Reg(188u)), NondetRegLayout(Reg(189u))), ArgU8Layout(NondetRegLayout(Reg(190u)), NondetRegLayout(Reg(191u))));
const kLayout__755: ReadElemLayout = ReadElemLayout(kLayout__731);
const kLayout__756: ReadElemLayout = ReadElemLayout(kLayout__736);
const kLayout__757: ReadElemLayout = ReadElemLayout(kLayout__741);
const kLayout__758: ReadElemLayout = ReadElemLayout(kLayout__745);
const kLayout__761: MemoryIOLayout = MemoryIOLayout(kLayout__713, kLayout__714);
const kLayout__763: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(148u)), NondetRegLayout(Reg(149u))));
const kLayout__762: IsForwardLayout = IsForwardLayout(kLayout__763);
const kLayout__760: MemoryReadLayout = MemoryReadLayout(kLayout__761, kLayout__762);
const kLayout__759: ReadElemLayout = ReadElemLayout(kLayout__760);
const kLayout__766: MemoryIOLayout = MemoryIOLayout(kLayout__715, kLayout__716);
const kLayout__768: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(150u)), NondetRegLayout(Reg(151u))));
const kLayout__767: IsForwardLayout = IsForwardLayout(kLayout__768);
const kLayout__765: MemoryReadLayout = MemoryReadLayout(kLayout__766, kLayout__767);
const kLayout__764: ReadElemLayout = ReadElemLayout(kLayout__765);
const kLayout__771: MemoryIOLayout = MemoryIOLayout(kLayout__717, kLayout__718);
const kLayout__773: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(152u)), NondetRegLayout(Reg(153u))));
const kLayout__772: IsForwardLayout = IsForwardLayout(kLayout__773);
const kLayout__770: MemoryReadLayout = MemoryReadLayout(kLayout__771, kLayout__772);
const kLayout__769: ReadElemLayout = ReadElemLayout(kLayout__770);
const kLayout__776: MemoryIOLayout = MemoryIOLayout(kLayout__719, kLayout__720);
const kLayout__778: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(154u)), NondetRegLayout(Reg(155u))));
const kLayout__777: IsForwardLayout = IsForwardLayout(kLayout__778);
const kLayout__775: MemoryReadLayout = MemoryReadLayout(kLayout__776, kLayout__777);
const kLayout__774: ReadElemLayout = ReadElemLayout(kLayout__775);
const kLayout__754: ReadElemLayout8LayoutArray = ReadElemLayout8LayoutArray(kLayout__755, kLayout__756, kLayout__757, kLayout__758, kLayout__759, kLayout__764, kLayout__769, kLayout__774);
const kLayout__753: PoseidonLoadStateLayout = PoseidonLoadStateLayout(kLayout__701, kLayout__754);
const kLayout__752: Poseidon0StateArm1Layout = Poseidon0StateArm1Layout(kLayout__753, ArgU16Layout(NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(75u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(158u)), NondetRegLayout(Reg(159u))), ArgU16Layout(NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(84u))), ArgU16Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(85u))), ArgU16Layout(NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u))), ArgU16Layout(NondetRegLayout(Reg(164u)), NondetRegLayout(Reg(93u))), ArgU16Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(94u))), ArgU16Layout(NondetRegLayout(Reg(166u)), NondetRegLayout(Reg(167u))), ArgU16Layout(NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(102u))), ArgU16Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(103u))), ArgU16Layout(NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u))), ArgU16Layout(NondetRegLayout(Reg(172u)), NondetRegLayout(Reg(111u))), ArgU16Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(112u))), ArgU16Layout(NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u))), ArgU16Layout(NondetRegLayout(Reg(176u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(121u))), ArgU16Layout(NondetRegLayout(Reg(178u)), NondetRegLayout(Reg(179u))), ArgU16Layout(NondetRegLayout(Reg(180u)), NondetRegLayout(Reg(129u))), ArgU16Layout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u))), ArgU16Layout(NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(139u))), ArgU16Layout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u))), ArgU8Layout(NondetRegLayout(Reg(188u)), NondetRegLayout(Reg(189u))), ArgU8Layout(NondetRegLayout(Reg(190u)), NondetRegLayout(Reg(191u))));
const kLayout__783: OneHot_3_Layout = OneHot_3_Layout(NondetRegLayout3LayoutArray(NondetRegLayout(Reg(196u)), NondetRegLayout(Reg(197u)), NondetRegLayout(Reg(198u))));
const kLayout__788: MemoryPageInLayout = MemoryPageInLayout(kLayout__732);
const kLayout__787: MemoryGet_SuperArm1Layout = MemoryGet_SuperArm1Layout(kLayout__788, CycleArgLayout(NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u))));
const kLayout__789: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__732, kLayout__733);
const kLayout__786: MemoryGet_SuperLayout = MemoryGet_SuperLayout(kLayout__731, kLayout__787, kLayout__789);
const kLayout__791: MemoryArgLayout2LayoutArray = MemoryArgLayout2LayoutArray(kLayout__705, kLayout__706);
const kLayout__790: _Arguments_MemoryGet_SuperLayout = _Arguments_MemoryGet_SuperLayout(kLayout__791, CycleArgLayout1LayoutArray(CycleArgLayout(NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u)))));
const kLayout__785: MemoryGetLayout = MemoryGetLayout(kLayout__786, kLayout__790);
const kLayout__795: MemoryPageInLayout = MemoryPageInLayout(kLayout__737);
const kLayout__794: MemoryGet_SuperArm1Layout = MemoryGet_SuperArm1Layout(kLayout__795, CycleArgLayout(NondetRegLayout(Reg(142u)), NondetRegLayout(Reg(143u))));
const kLayout__796: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__737, kLayout__738);
const kLayout__793: MemoryGet_SuperLayout = MemoryGet_SuperLayout(kLayout__736, kLayout__794, kLayout__796);
const kLayout__798: MemoryArgLayout2LayoutArray = MemoryArgLayout2LayoutArray(kLayout__707, kLayout__708);
const kLayout__797: _Arguments_MemoryGet_SuperLayout = _Arguments_MemoryGet_SuperLayout(kLayout__798, CycleArgLayout1LayoutArray(CycleArgLayout(NondetRegLayout(Reg(142u)), NondetRegLayout(Reg(143u)))));
const kLayout__792: MemoryGetLayout = MemoryGetLayout(kLayout__793, kLayout__797);
const kLayout__802: MemoryPageInLayout = MemoryPageInLayout(kLayout__742);
const kLayout__801: MemoryGet_SuperArm1Layout = MemoryGet_SuperArm1Layout(kLayout__802, CycleArgLayout(NondetRegLayout(Reg(144u)), NondetRegLayout(Reg(145u))));
const kLayout__803: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__742, kLayout__743);
const kLayout__800: MemoryGet_SuperLayout = MemoryGet_SuperLayout(kLayout__741, kLayout__801, kLayout__803);
const kLayout__805: MemoryArgLayout2LayoutArray = MemoryArgLayout2LayoutArray(kLayout__709, kLayout__710);
const kLayout__804: _Arguments_MemoryGet_SuperLayout = _Arguments_MemoryGet_SuperLayout(kLayout__805, CycleArgLayout1LayoutArray(CycleArgLayout(NondetRegLayout(Reg(144u)), NondetRegLayout(Reg(145u)))));
const kLayout__799: MemoryGetLayout = MemoryGetLayout(kLayout__800, kLayout__804);
const kLayout__809: MemoryPageInLayout = MemoryPageInLayout(kLayout__746);
const kLayout__808: MemoryGet_SuperArm1Layout = MemoryGet_SuperArm1Layout(kLayout__809, CycleArgLayout(NondetRegLayout(Reg(146u)), NondetRegLayout(Reg(147u))));
const kLayout__810: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__746, kLayout__747);
const kLayout__807: MemoryGet_SuperLayout = MemoryGet_SuperLayout(kLayout__745, kLayout__808, kLayout__810);
const kLayout__812: MemoryArgLayout2LayoutArray = MemoryArgLayout2LayoutArray(kLayout__711, kLayout__712);
const kLayout__811: _Arguments_MemoryGet_SuperLayout = _Arguments_MemoryGet_SuperLayout(kLayout__812, CycleArgLayout1LayoutArray(CycleArgLayout(NondetRegLayout(Reg(146u)), NondetRegLayout(Reg(147u)))));
const kLayout__806: MemoryGetLayout = MemoryGetLayout(kLayout__807, kLayout__811);
const kLayout__816: MemoryPageInLayout = MemoryPageInLayout(kLayout__761);
const kLayout__815: MemoryGet_SuperArm1Layout = MemoryGet_SuperArm1Layout(kLayout__816, CycleArgLayout(NondetRegLayout(Reg(148u)), NondetRegLayout(Reg(149u))));
const kLayout__817: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__761, kLayout__762);
const kLayout__814: MemoryGet_SuperLayout = MemoryGet_SuperLayout(kLayout__760, kLayout__815, kLayout__817);
const kLayout__819: MemoryArgLayout2LayoutArray = MemoryArgLayout2LayoutArray(kLayout__713, kLayout__714);
const kLayout__818: _Arguments_MemoryGet_SuperLayout = _Arguments_MemoryGet_SuperLayout(kLayout__819, CycleArgLayout1LayoutArray(CycleArgLayout(NondetRegLayout(Reg(148u)), NondetRegLayout(Reg(149u)))));
const kLayout__813: MemoryGetLayout = MemoryGetLayout(kLayout__814, kLayout__818);
const kLayout__823: MemoryPageInLayout = MemoryPageInLayout(kLayout__766);
const kLayout__822: MemoryGet_SuperArm1Layout = MemoryGet_SuperArm1Layout(kLayout__823, CycleArgLayout(NondetRegLayout(Reg(150u)), NondetRegLayout(Reg(151u))));
const kLayout__824: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__766, kLayout__767);
const kLayout__821: MemoryGet_SuperLayout = MemoryGet_SuperLayout(kLayout__765, kLayout__822, kLayout__824);
const kLayout__826: MemoryArgLayout2LayoutArray = MemoryArgLayout2LayoutArray(kLayout__715, kLayout__716);
const kLayout__825: _Arguments_MemoryGet_SuperLayout = _Arguments_MemoryGet_SuperLayout(kLayout__826, CycleArgLayout1LayoutArray(CycleArgLayout(NondetRegLayout(Reg(150u)), NondetRegLayout(Reg(151u)))));
const kLayout__820: MemoryGetLayout = MemoryGetLayout(kLayout__821, kLayout__825);
const kLayout__830: MemoryPageInLayout = MemoryPageInLayout(kLayout__771);
const kLayout__829: MemoryGet_SuperArm1Layout = MemoryGet_SuperArm1Layout(kLayout__830, CycleArgLayout(NondetRegLayout(Reg(152u)), NondetRegLayout(Reg(153u))));
const kLayout__831: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__771, kLayout__772);
const kLayout__828: MemoryGet_SuperLayout = MemoryGet_SuperLayout(kLayout__770, kLayout__829, kLayout__831);
const kLayout__833: MemoryArgLayout2LayoutArray = MemoryArgLayout2LayoutArray(kLayout__717, kLayout__718);
const kLayout__832: _Arguments_MemoryGet_SuperLayout = _Arguments_MemoryGet_SuperLayout(kLayout__833, CycleArgLayout1LayoutArray(CycleArgLayout(NondetRegLayout(Reg(152u)), NondetRegLayout(Reg(153u)))));
const kLayout__827: MemoryGetLayout = MemoryGetLayout(kLayout__828, kLayout__832);
const kLayout__837: MemoryPageInLayout = MemoryPageInLayout(kLayout__776);
const kLayout__836: MemoryGet_SuperArm1Layout = MemoryGet_SuperArm1Layout(kLayout__837, CycleArgLayout(NondetRegLayout(Reg(154u)), NondetRegLayout(Reg(155u))));
const kLayout__838: MemoryPageOutLayout = MemoryPageOutLayout(kLayout__776, kLayout__777);
const kLayout__835: MemoryGet_SuperLayout = MemoryGet_SuperLayout(kLayout__775, kLayout__836, kLayout__838);
const kLayout__840: MemoryArgLayout2LayoutArray = MemoryArgLayout2LayoutArray(kLayout__719, kLayout__720);
const kLayout__839: _Arguments_MemoryGet_SuperLayout = _Arguments_MemoryGet_SuperLayout(kLayout__840, CycleArgLayout1LayoutArray(CycleArgLayout(NondetRegLayout(Reg(154u)), NondetRegLayout(Reg(155u)))));
const kLayout__834: MemoryGetLayout = MemoryGetLayout(kLayout__835, kLayout__839);
const kLayout__784: MemoryGetLayout8LayoutArray = MemoryGetLayout8LayoutArray(kLayout__785, kLayout__792, kLayout__799, kLayout__806, kLayout__813, kLayout__820, kLayout__827, kLayout__834);
const kLayout__782: PoseidonLoadInShortLayout = PoseidonLoadInShortLayout(kLayout__701, kLayout__783, kLayout__784);
const kLayout__841: PoseidonLoadInLowLayout = PoseidonLoadInLowLayout(kLayout__701, kLayout__783, kLayout__784);
const kLayout__842: PoseidonLoadInHighLayout = PoseidonLoadInHighLayout(kLayout__701, kLayout__783, kLayout__784);
const kLayout__781: PoseidonLoadIn_SuperLayout = PoseidonLoadIn_SuperLayout(kLayout__701, kLayout__782, kLayout__841, kLayout__842);
const kLayout__843: OneHot_3_Layout = OneHot_3_Layout(NondetRegLayout3LayoutArray(NondetRegLayout(Reg(199u)), NondetRegLayout(Reg(200u)), NondetRegLayout(Reg(201u))));
const kLayout__844: _Arguments_PoseidonLoadIn_SuperLayout = _Arguments_PoseidonLoadIn_SuperLayout(kLayout__704, kLayout__721);
const kLayout__780: PoseidonLoadInLayout = PoseidonLoadInLayout(kLayout__781, kLayout__843, kLayout__844);
const kLayout__779: Poseidon0StateArm2Layout = Poseidon0StateArm2Layout(kLayout__780, ArgU16Layout(NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(75u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(158u)), NondetRegLayout(Reg(159u))), ArgU16Layout(NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(84u))), ArgU16Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(85u))), ArgU16Layout(NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u))), ArgU16Layout(NondetRegLayout(Reg(164u)), NondetRegLayout(Reg(93u))), ArgU16Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(94u))), ArgU16Layout(NondetRegLayout(Reg(166u)), NondetRegLayout(Reg(167u))), ArgU16Layout(NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(102u))), ArgU16Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(103u))), ArgU16Layout(NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u))), ArgU16Layout(NondetRegLayout(Reg(172u)), NondetRegLayout(Reg(111u))), ArgU16Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(112u))), ArgU16Layout(NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u))), ArgU16Layout(NondetRegLayout(Reg(176u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(121u))), ArgU16Layout(NondetRegLayout(Reg(178u)), NondetRegLayout(Reg(179u))), ArgU16Layout(NondetRegLayout(Reg(180u)), NondetRegLayout(Reg(129u))), ArgU16Layout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u))), ArgU16Layout(NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(139u))), ArgU16Layout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u))), ArgU8Layout(NondetRegLayout(Reg(188u)), NondetRegLayout(Reg(189u))), ArgU8Layout(NondetRegLayout(Reg(190u)), NondetRegLayout(Reg(191u))));
const kLayout__845: Poseidon0StateArm3Layout = Poseidon0StateArm3Layout(kLayout__701, kLayout__705, kLayout__706, kLayout__707, kLayout__708, kLayout__709, kLayout__710, kLayout__711, kLayout__712, kLayout__713, kLayout__714, kLayout__715, kLayout__716, kLayout__717, kLayout__718, kLayout__719, kLayout__720, CycleArgLayout(NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u))), CycleArgLayout(NondetRegLayout(Reg(142u)), NondetRegLayout(Reg(143u))), CycleArgLayout(NondetRegLayout(Reg(144u)), NondetRegLayout(Reg(145u))), CycleArgLayout(NondetRegLayout(Reg(146u)), NondetRegLayout(Reg(147u))), CycleArgLayout(NondetRegLayout(Reg(148u)), NondetRegLayout(Reg(149u))), CycleArgLayout(NondetRegLayout(Reg(150u)), NondetRegLayout(Reg(151u))), CycleArgLayout(NondetRegLayout(Reg(152u)), NondetRegLayout(Reg(153u))), CycleArgLayout(NondetRegLayout(Reg(154u)), NondetRegLayout(Reg(155u))), ArgU16Layout(NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(75u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(158u)), NondetRegLayout(Reg(159u))), ArgU16Layout(NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(84u))), ArgU16Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(85u))), ArgU16Layout(NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u))), ArgU16Layout(NondetRegLayout(Reg(164u)), NondetRegLayout(Reg(93u))), ArgU16Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(94u))), ArgU16Layout(NondetRegLayout(Reg(166u)), NondetRegLayout(Reg(167u))), ArgU16Layout(NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(102u))), ArgU16Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(103u))), ArgU16Layout(NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u))), ArgU16Layout(NondetRegLayout(Reg(172u)), NondetRegLayout(Reg(111u))), ArgU16Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(112u))), ArgU16Layout(NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u))), ArgU16Layout(NondetRegLayout(Reg(176u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(121u))), ArgU16Layout(NondetRegLayout(Reg(178u)), NondetRegLayout(Reg(179u))), ArgU16Layout(NondetRegLayout(Reg(180u)), NondetRegLayout(Reg(129u))), ArgU16Layout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u))), ArgU16Layout(NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(139u))), ArgU16Layout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u))), ArgU8Layout(NondetRegLayout(Reg(188u)), NondetRegLayout(Reg(189u))), ArgU8Layout(NondetRegLayout(Reg(190u)), NondetRegLayout(Reg(191u))));
const kLayout__846: Poseidon0StateArm4Layout = Poseidon0StateArm4Layout(kLayout__701, kLayout__705, kLayout__706, kLayout__707, kLayout__708, kLayout__709, kLayout__710, kLayout__711, kLayout__712, kLayout__713, kLayout__714, kLayout__715, kLayout__716, kLayout__717, kLayout__718, kLayout__719, kLayout__720, CycleArgLayout(NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u))), CycleArgLayout(NondetRegLayout(Reg(142u)), NondetRegLayout(Reg(143u))), CycleArgLayout(NondetRegLayout(Reg(144u)), NondetRegLayout(Reg(145u))), CycleArgLayout(NondetRegLayout(Reg(146u)), NondetRegLayout(Reg(147u))), CycleArgLayout(NondetRegLayout(Reg(148u)), NondetRegLayout(Reg(149u))), CycleArgLayout(NondetRegLayout(Reg(150u)), NondetRegLayout(Reg(151u))), CycleArgLayout(NondetRegLayout(Reg(152u)), NondetRegLayout(Reg(153u))), CycleArgLayout(NondetRegLayout(Reg(154u)), NondetRegLayout(Reg(155u))), ArgU16Layout(NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(75u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(158u)), NondetRegLayout(Reg(159u))), ArgU16Layout(NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(84u))), ArgU16Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(85u))), ArgU16Layout(NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u))), ArgU16Layout(NondetRegLayout(Reg(164u)), NondetRegLayout(Reg(93u))), ArgU16Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(94u))), ArgU16Layout(NondetRegLayout(Reg(166u)), NondetRegLayout(Reg(167u))), ArgU16Layout(NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(102u))), ArgU16Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(103u))), ArgU16Layout(NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u))), ArgU16Layout(NondetRegLayout(Reg(172u)), NondetRegLayout(Reg(111u))), ArgU16Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(112u))), ArgU16Layout(NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u))), ArgU16Layout(NondetRegLayout(Reg(176u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(121u))), ArgU16Layout(NondetRegLayout(Reg(178u)), NondetRegLayout(Reg(179u))), ArgU16Layout(NondetRegLayout(Reg(180u)), NondetRegLayout(Reg(129u))), ArgU16Layout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u))), ArgU16Layout(NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(139u))), ArgU16Layout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u))), ArgU8Layout(NondetRegLayout(Reg(188u)), NondetRegLayout(Reg(189u))), ArgU8Layout(NondetRegLayout(Reg(190u)), NondetRegLayout(Reg(191u))));
const kLayout__853: PoseidonCheckOut__0_SuperLayout = PoseidonCheckOut__0_SuperLayout(kLayout__755);
const kLayout__854: PoseidonCheckOut__0_SuperLayout = PoseidonCheckOut__0_SuperLayout(kLayout__756);
const kLayout__855: PoseidonCheckOut__0_SuperLayout = PoseidonCheckOut__0_SuperLayout(kLayout__757);
const kLayout__856: PoseidonCheckOut__0_SuperLayout = PoseidonCheckOut__0_SuperLayout(kLayout__758);
const kLayout__857: PoseidonCheckOut__0_SuperLayout = PoseidonCheckOut__0_SuperLayout(kLayout__759);
const kLayout__858: PoseidonCheckOut__0_SuperLayout = PoseidonCheckOut__0_SuperLayout(kLayout__764);
const kLayout__859: PoseidonCheckOut__0_SuperLayout = PoseidonCheckOut__0_SuperLayout(kLayout__769);
const kLayout__860: PoseidonCheckOut__0_SuperLayout = PoseidonCheckOut__0_SuperLayout(kLayout__774);
const kLayout__852: PoseidonCheckOut__0_SuperLayout8LayoutArray = PoseidonCheckOut__0_SuperLayout8LayoutArray(kLayout__853, kLayout__854, kLayout__855, kLayout__856, kLayout__857, kLayout__858, kLayout__859, kLayout__860);
const kLayout__851: PoseidonCheckOutLayout = PoseidonCheckOutLayout(kLayout__701, kLayout__852, IsZeroLayout(NondetRegLayout(Reg(196u)), NondetRegLayout(Reg(197u))));
const kLayout__850: PoseidonDoOut_SuperArm0Layout = PoseidonDoOut_SuperArm0Layout(kLayout__851, ArgU16Layout(NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(75u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(76u))), ArgU16Layout(NondetRegLayout(Reg(158u)), NondetRegLayout(Reg(159u))), ArgU16Layout(NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(84u))), ArgU16Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(85u))), ArgU16Layout(NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u))), ArgU16Layout(NondetRegLayout(Reg(164u)), NondetRegLayout(Reg(93u))), ArgU16Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(94u))), ArgU16Layout(NondetRegLayout(Reg(166u)), NondetRegLayout(Reg(167u))), ArgU16Layout(NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(102u))), ArgU16Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(103u))), ArgU16Layout(NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u))), ArgU16Layout(NondetRegLayout(Reg(172u)), NondetRegLayout(Reg(111u))), ArgU16Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(112u))), ArgU16Layout(NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u))), ArgU16Layout(NondetRegLayout(Reg(176u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(121u))), ArgU16Layout(NondetRegLayout(Reg(178u)), NondetRegLayout(Reg(179u))), ArgU16Layout(NondetRegLayout(Reg(180u)), NondetRegLayout(Reg(129u))), ArgU16Layout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u))), ArgU16Layout(NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(139u))), ArgU16Layout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u))));
const kLayout__865: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(75u))));
const kLayout__866: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(76u))));
const kLayout__867: _Arguments_FieldToWord__0Layout = _Arguments_FieldToWord__0Layout(ArgU16Layout1LayoutArray(ArgU16Layout(NondetRegLayout(Reg(158u)), NondetRegLayout(Reg(159u)))));
const kLayout__870: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(158u)), NondetRegLayout(Reg(159u))));
const kLayout__869: FieldToWord__0Arm0_SuperLayout = FieldToWord__0Arm0_SuperLayout(kLayout__870);
const kLayout__871: FieldToWord__0Arm1_SuperLayout = FieldToWord__0Arm1_SuperLayout(kLayout__870);
const kLayout__868: FieldToWord__0Layout = FieldToWord__0Layout(kLayout__869, kLayout__871);
const kLayout__864: FieldToWordLayout = FieldToWordLayout(kLayout__865, kLayout__866, NondetRegLayout(Reg(196u)), kLayout__867, kLayout__868);
const kLayout__872: MemoryWriteLayout = MemoryWriteLayout(kLayout__732, kLayout__733);
const kLayout__863: PoseidonStoreOut__0_SuperLayout = PoseidonStoreOut__0_SuperLayout(kLayout__864, kLayout__872);
const kLayout__875: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(84u))));
const kLayout__876: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(85u))));
const kLayout__877: _Arguments_FieldToWord__0Layout = _Arguments_FieldToWord__0Layout(ArgU16Layout1LayoutArray(ArgU16Layout(NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u)))));
const kLayout__880: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u))));
const kLayout__879: FieldToWord__0Arm0_SuperLayout = FieldToWord__0Arm0_SuperLayout(kLayout__880);
const kLayout__881: FieldToWord__0Arm1_SuperLayout = FieldToWord__0Arm1_SuperLayout(kLayout__880);
const kLayout__878: FieldToWord__0Layout = FieldToWord__0Layout(kLayout__879, kLayout__881);
const kLayout__874: FieldToWordLayout = FieldToWordLayout(kLayout__875, kLayout__876, NondetRegLayout(Reg(197u)), kLayout__877, kLayout__878);
const kLayout__882: MemoryWriteLayout = MemoryWriteLayout(kLayout__737, kLayout__738);
const kLayout__873: PoseidonStoreOut__0_SuperLayout = PoseidonStoreOut__0_SuperLayout(kLayout__874, kLayout__882);
const kLayout__885: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(164u)), NondetRegLayout(Reg(93u))));
const kLayout__886: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(94u))));
const kLayout__887: _Arguments_FieldToWord__0Layout = _Arguments_FieldToWord__0Layout(ArgU16Layout1LayoutArray(ArgU16Layout(NondetRegLayout(Reg(166u)), NondetRegLayout(Reg(167u)))));
const kLayout__890: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(166u)), NondetRegLayout(Reg(167u))));
const kLayout__889: FieldToWord__0Arm0_SuperLayout = FieldToWord__0Arm0_SuperLayout(kLayout__890);
const kLayout__891: FieldToWord__0Arm1_SuperLayout = FieldToWord__0Arm1_SuperLayout(kLayout__890);
const kLayout__888: FieldToWord__0Layout = FieldToWord__0Layout(kLayout__889, kLayout__891);
const kLayout__884: FieldToWordLayout = FieldToWordLayout(kLayout__885, kLayout__886, NondetRegLayout(Reg(198u)), kLayout__887, kLayout__888);
const kLayout__892: MemoryWriteLayout = MemoryWriteLayout(kLayout__742, kLayout__743);
const kLayout__883: PoseidonStoreOut__0_SuperLayout = PoseidonStoreOut__0_SuperLayout(kLayout__884, kLayout__892);
const kLayout__895: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(102u))));
const kLayout__896: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(103u))));
const kLayout__897: _Arguments_FieldToWord__0Layout = _Arguments_FieldToWord__0Layout(ArgU16Layout1LayoutArray(ArgU16Layout(NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u)))));
const kLayout__899: FieldToWord__0Arm0_SuperLayout = FieldToWord__0Arm0_SuperLayout(kLayout__224);
const kLayout__900: FieldToWord__0Arm1_SuperLayout = FieldToWord__0Arm1_SuperLayout(kLayout__224);
const kLayout__898: FieldToWord__0Layout = FieldToWord__0Layout(kLayout__899, kLayout__900);
const kLayout__894: FieldToWordLayout = FieldToWordLayout(kLayout__895, kLayout__896, NondetRegLayout(Reg(199u)), kLayout__897, kLayout__898);
const kLayout__901: MemoryWriteLayout = MemoryWriteLayout(kLayout__746, kLayout__747);
const kLayout__893: PoseidonStoreOut__0_SuperLayout = PoseidonStoreOut__0_SuperLayout(kLayout__894, kLayout__901);
const kLayout__904: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(172u)), NondetRegLayout(Reg(111u))));
const kLayout__905: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(112u))));
const kLayout__906: _Arguments_FieldToWord__0Layout = _Arguments_FieldToWord__0Layout(ArgU16Layout1LayoutArray(ArgU16Layout(NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u)))));
const kLayout__909: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u))));
const kLayout__908: FieldToWord__0Arm0_SuperLayout = FieldToWord__0Arm0_SuperLayout(kLayout__909);
const kLayout__910: FieldToWord__0Arm1_SuperLayout = FieldToWord__0Arm1_SuperLayout(kLayout__909);
const kLayout__907: FieldToWord__0Layout = FieldToWord__0Layout(kLayout__908, kLayout__910);
const kLayout__903: FieldToWordLayout = FieldToWordLayout(kLayout__904, kLayout__905, NondetRegLayout(Reg(200u)), kLayout__906, kLayout__907);
const kLayout__911: MemoryWriteLayout = MemoryWriteLayout(kLayout__761, kLayout__762);
const kLayout__902: PoseidonStoreOut__0_SuperLayout = PoseidonStoreOut__0_SuperLayout(kLayout__903, kLayout__911);
const kLayout__914: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(176u)), NondetRegLayout(Reg(120u))));
const kLayout__915: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(121u))));
const kLayout__916: _Arguments_FieldToWord__0Layout = _Arguments_FieldToWord__0Layout(ArgU16Layout1LayoutArray(ArgU16Layout(NondetRegLayout(Reg(178u)), NondetRegLayout(Reg(179u)))));
const kLayout__919: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(178u)), NondetRegLayout(Reg(179u))));
const kLayout__918: FieldToWord__0Arm0_SuperLayout = FieldToWord__0Arm0_SuperLayout(kLayout__919);
const kLayout__920: FieldToWord__0Arm1_SuperLayout = FieldToWord__0Arm1_SuperLayout(kLayout__919);
const kLayout__917: FieldToWord__0Layout = FieldToWord__0Layout(kLayout__918, kLayout__920);
const kLayout__913: FieldToWordLayout = FieldToWordLayout(kLayout__914, kLayout__915, NondetRegLayout(Reg(201u)), kLayout__916, kLayout__917);
const kLayout__921: MemoryWriteLayout = MemoryWriteLayout(kLayout__766, kLayout__767);
const kLayout__912: PoseidonStoreOut__0_SuperLayout = PoseidonStoreOut__0_SuperLayout(kLayout__913, kLayout__921);
const kLayout__924: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(180u)), NondetRegLayout(Reg(129u))));
const kLayout__925: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(130u))));
const kLayout__926: _Arguments_FieldToWord__0Layout = _Arguments_FieldToWord__0Layout(ArgU16Layout1LayoutArray(ArgU16Layout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u)))));
const kLayout__929: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u))));
const kLayout__928: FieldToWord__0Arm0_SuperLayout = FieldToWord__0Arm0_SuperLayout(kLayout__929);
const kLayout__930: FieldToWord__0Arm1_SuperLayout = FieldToWord__0Arm1_SuperLayout(kLayout__929);
const kLayout__927: FieldToWord__0Layout = FieldToWord__0Layout(kLayout__928, kLayout__930);
const kLayout__923: FieldToWordLayout = FieldToWordLayout(kLayout__924, kLayout__925, NondetRegLayout(Reg(202u)), kLayout__926, kLayout__927);
const kLayout__931: MemoryWriteLayout = MemoryWriteLayout(kLayout__771, kLayout__772);
const kLayout__922: PoseidonStoreOut__0_SuperLayout = PoseidonStoreOut__0_SuperLayout(kLayout__923, kLayout__931);
const kLayout__934: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(138u))));
const kLayout__935: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(139u))));
const kLayout__936: _Arguments_FieldToWord__0Layout = _Arguments_FieldToWord__0Layout(ArgU16Layout1LayoutArray(ArgU16Layout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u)))));
const kLayout__939: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u))));
const kLayout__938: FieldToWord__0Arm0_SuperLayout = FieldToWord__0Arm0_SuperLayout(kLayout__939);
const kLayout__940: FieldToWord__0Arm1_SuperLayout = FieldToWord__0Arm1_SuperLayout(kLayout__939);
const kLayout__937: FieldToWord__0Layout = FieldToWord__0Layout(kLayout__938, kLayout__940);
const kLayout__933: FieldToWordLayout = FieldToWordLayout(kLayout__934, kLayout__935, NondetRegLayout(Reg(203u)), kLayout__936, kLayout__937);
const kLayout__941: MemoryWriteLayout = MemoryWriteLayout(kLayout__776, kLayout__777);
const kLayout__932: PoseidonStoreOut__0_SuperLayout = PoseidonStoreOut__0_SuperLayout(kLayout__933, kLayout__941);
const kLayout__862: PoseidonStoreOut__0_SuperLayout8LayoutArray = PoseidonStoreOut__0_SuperLayout8LayoutArray(kLayout__863, kLayout__873, kLayout__883, kLayout__893, kLayout__902, kLayout__912, kLayout__922, kLayout__932);
const kLayout__861: PoseidonStoreOutLayout = PoseidonStoreOutLayout(kLayout__701, kLayout__862, IsZeroLayout(NondetRegLayout(Reg(204u)), NondetRegLayout(Reg(205u))), NondetExtRegLayout(Reg(206u)));
const kLayout__849: PoseidonDoOut_SuperLayout = PoseidonDoOut_SuperLayout(kLayout__701, kLayout__850, kLayout__861);
const kLayout__942: _Arguments_PoseidonDoOut_SuperLayout = _Arguments_PoseidonDoOut_SuperLayout(kLayout__704, kLayout__721, kLayout__722);
const kLayout__848: PoseidonDoOutLayout = PoseidonDoOutLayout(kLayout__849, kLayout__942);
const kLayout__847: Poseidon0StateArm5Layout = Poseidon0StateArm5Layout(kLayout__848, ArgU8Layout(NondetRegLayout(Reg(188u)), NondetRegLayout(Reg(189u))), ArgU8Layout(NondetRegLayout(Reg(190u)), NondetRegLayout(Reg(191u))));
const kLayout__945: PoseidonPaging_SuperLayout = PoseidonPaging_SuperLayout(kLayout__701, kLayout__701, kLayout__701, kLayout__701, kLayout__701, kLayout__701, kLayout__701);
const kLayout__947: NondetRegLayout6LayoutArray = NondetRegLayout6LayoutArray(NondetRegLayout(Reg(198u)), NondetRegLayout(Reg(199u)), NondetRegLayout(Reg(200u)), NondetRegLayout(Reg(201u)), NondetRegLayout(Reg(202u)), NondetRegLayout(Reg(203u)));
const kLayout__946: OneHot_6_Layout = OneHot_6_Layout(kLayout__947);
const kLayout__949: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(188u)), NondetRegLayout(Reg(189u))));
const kLayout__948: IsU24Layout = IsU24Layout(kLayout__865, kLayout__949);
const kLayout__950: _Arguments_PoseidonPaging__1Layout = _Arguments_PoseidonPaging__1Layout(ArgU16Layout1LayoutArray(ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(76u)))), ArgU8Layout1LayoutArray(ArgU8Layout(NondetRegLayout(Reg(190u)), NondetRegLayout(Reg(191u)))));
const kLayout__954: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(190u)), NondetRegLayout(Reg(191u))));
const kLayout__953: IsU24Layout = IsU24Layout(kLayout__866, kLayout__954);
const kLayout__952: PoseidonPaging__1Arm0_SuperLayout = PoseidonPaging__1Arm0_SuperLayout(kLayout__953);
const kLayout__955: PoseidonPaging__1Arm1_SuperLayout = PoseidonPaging__1Arm1_SuperLayout(kLayout__953);
const kLayout__951: PoseidonPaging__1Layout = PoseidonPaging__1Layout(kLayout__952, kLayout__955);
const kLayout__944: PoseidonPagingLayout = PoseidonPagingLayout(kLayout__945, NondetRegLayout(Reg(196u)), NondetRegLayout(Reg(197u)), kLayout__946, kLayout__948, kLayout__950, kLayout__951, NondetRegLayout(Reg(204u)));
const kLayout__943: Poseidon0StateArm6Layout = Poseidon0StateArm6Layout(kLayout__944, kLayout__705, kLayout__706, kLayout__707, kLayout__708, kLayout__709, kLayout__710, kLayout__711, kLayout__712, kLayout__713, kLayout__714, kLayout__715, kLayout__716, kLayout__717, kLayout__718, kLayout__719, kLayout__720, CycleArgLayout(NondetRegLayout(Reg(140u)), NondetRegLayout(Reg(141u))), CycleArgLayout(NondetRegLayout(Reg(142u)), NondetRegLayout(Reg(143u))), CycleArgLayout(NondetRegLayout(Reg(144u)), NondetRegLayout(Reg(145u))), CycleArgLayout(NondetRegLayout(Reg(146u)), NondetRegLayout(Reg(147u))), CycleArgLayout(NondetRegLayout(Reg(148u)), NondetRegLayout(Reg(149u))), CycleArgLayout(NondetRegLayout(Reg(150u)), NondetRegLayout(Reg(151u))), CycleArgLayout(NondetRegLayout(Reg(152u)), NondetRegLayout(Reg(153u))), CycleArgLayout(NondetRegLayout(Reg(154u)), NondetRegLayout(Reg(155u))), ArgU16Layout(NondetRegLayout(Reg(158u)), NondetRegLayout(Reg(159u))), ArgU16Layout(NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(84u))), ArgU16Layout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(85u))), ArgU16Layout(NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u))), ArgU16Layout(NondetRegLayout(Reg(164u)), NondetRegLayout(Reg(93u))), ArgU16Layout(NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(94u))), ArgU16Layout(NondetRegLayout(Reg(166u)), NondetRegLayout(Reg(167u))), ArgU16Layout(NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(102u))), ArgU16Layout(NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(103u))), ArgU16Layout(NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u))), ArgU16Layout(NondetRegLayout(Reg(172u)), NondetRegLayout(Reg(111u))), ArgU16Layout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(112u))), ArgU16Layout(NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u))), ArgU16Layout(NondetRegLayout(Reg(176u)), NondetRegLayout(Reg(120u))), ArgU16Layout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(121u))), ArgU16Layout(NondetRegLayout(Reg(178u)), NondetRegLayout(Reg(179u))), ArgU16Layout(NondetRegLayout(Reg(180u)), NondetRegLayout(Reg(129u))), ArgU16Layout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(130u))), ArgU16Layout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u))), ArgU16Layout(NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(138u))), ArgU16Layout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(139u))), ArgU16Layout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u))));
const kLayout__959: PoseidonStoreState__0_SuperLayout = PoseidonStoreState__0_SuperLayout(kLayout__864, kLayout__872);
const kLayout__960: PoseidonStoreState__0_SuperLayout = PoseidonStoreState__0_SuperLayout(kLayout__874, kLayout__882);
const kLayout__961: PoseidonStoreState__0_SuperLayout = PoseidonStoreState__0_SuperLayout(kLayout__884, kLayout__892);
const kLayout__962: PoseidonStoreState__0_SuperLayout = PoseidonStoreState__0_SuperLayout(kLayout__894, kLayout__901);
const kLayout__963: PoseidonStoreState__0_SuperLayout = PoseidonStoreState__0_SuperLayout(kLayout__903, kLayout__911);
const kLayout__964: PoseidonStoreState__0_SuperLayout = PoseidonStoreState__0_SuperLayout(kLayout__913, kLayout__921);
const kLayout__965: PoseidonStoreState__0_SuperLayout = PoseidonStoreState__0_SuperLayout(kLayout__923, kLayout__931);
const kLayout__966: PoseidonStoreState__0_SuperLayout = PoseidonStoreState__0_SuperLayout(kLayout__933, kLayout__941);
const kLayout__958: PoseidonStoreState__0_SuperLayout8LayoutArray = PoseidonStoreState__0_SuperLayout8LayoutArray(kLayout__959, kLayout__960, kLayout__961, kLayout__962, kLayout__963, kLayout__964, kLayout__965, kLayout__966);
const kLayout__957: PoseidonStoreStateLayout = PoseidonStoreStateLayout(kLayout__701, kLayout__958);
const kLayout__956: Poseidon0StateArm7Layout = Poseidon0StateArm7Layout(kLayout__957, ArgU8Layout(NondetRegLayout(Reg(188u)), NondetRegLayout(Reg(189u))), ArgU8Layout(NondetRegLayout(Reg(190u)), NondetRegLayout(Reg(191u))));
const kLayout__724: Poseidon0StateLayout = Poseidon0StateLayout(kLayout__701, kLayout__725, kLayout__752, kLayout__779, kLayout__845, kLayout__846, kLayout__847, kLayout__943, kLayout__956);
const kLayout__699: Poseidon0Layout = Poseidon0Layout(kLayout__700, kLayout__701, kLayout__703, kLayout__724);
const kLayout__968: DoCycleTableLayout = DoCycleTableLayout(CycleArgLayout(NondetRegLayout(Reg(68u)), NondetRegLayout(Reg(69u))), CycleArgLayout(NondetRegLayout(Reg(70u)), NondetRegLayout(Reg(71u))));
const kLayout__973: SBoxLayout24LayoutArray = SBoxLayout24LayoutArray(SBoxLayout(NondetRegLayout(Reg(78u)), NondetRegLayout(Reg(79u))), SBoxLayout(NondetRegLayout(Reg(80u)), NondetRegLayout(Reg(81u))), SBoxLayout(NondetRegLayout(Reg(82u)), NondetRegLayout(Reg(83u))), SBoxLayout(NondetRegLayout(Reg(84u)), NondetRegLayout(Reg(85u))), SBoxLayout(NondetRegLayout(Reg(86u)), NondetRegLayout(Reg(87u))), SBoxLayout(NondetRegLayout(Reg(88u)), NondetRegLayout(Reg(89u))), SBoxLayout(NondetRegLayout(Reg(90u)), NondetRegLayout(Reg(91u))), SBoxLayout(NondetRegLayout(Reg(92u)), NondetRegLayout(Reg(93u))), SBoxLayout(NondetRegLayout(Reg(94u)), NondetRegLayout(Reg(95u))), SBoxLayout(NondetRegLayout(Reg(96u)), NondetRegLayout(Reg(97u))), SBoxLayout(NondetRegLayout(Reg(98u)), NondetRegLayout(Reg(99u))), SBoxLayout(NondetRegLayout(Reg(100u)), NondetRegLayout(Reg(101u))), SBoxLayout(NondetRegLayout(Reg(102u)), NondetRegLayout(Reg(103u))), SBoxLayout(NondetRegLayout(Reg(104u)), NondetRegLayout(Reg(105u))), SBoxLayout(NondetRegLayout(Reg(106u)), NondetRegLayout(Reg(107u))), SBoxLayout(NondetRegLayout(Reg(108u)), NondetRegLayout(Reg(109u))), SBoxLayout(NondetRegLayout(Reg(110u)), NondetRegLayout(Reg(111u))), SBoxLayout(NondetRegLayout(Reg(112u)), NondetRegLayout(Reg(113u))), SBoxLayout(NondetRegLayout(Reg(114u)), NondetRegLayout(Reg(115u))), SBoxLayout(NondetRegLayout(Reg(116u)), NondetRegLayout(Reg(117u))), SBoxLayout(NondetRegLayout(Reg(118u)), NondetRegLayout(Reg(119u))), SBoxLayout(NondetRegLayout(Reg(120u)), NondetRegLayout(Reg(121u))), SBoxLayout(NondetRegLayout(Reg(122u)), NondetRegLayout(Reg(123u))), SBoxLayout(NondetRegLayout(Reg(124u)), NondetRegLayout(Reg(125u))));
const kLayout__972: DoExtRoundLayout = DoExtRoundLayout(kLayout__973);
const kLayout__975: NondetRegLayout8LayoutArray = NondetRegLayout8LayoutArray(NondetRegLayout(Reg(126u)), NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u)), NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u)), NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u)), NondetRegLayout(Reg(133u)));
const kLayout__974: OneHot_8_Layout = OneHot_8_Layout(kLayout__975);
const kLayout__971: DoExtRoundByIdxLayout = DoExtRoundByIdxLayout(kLayout__972, kLayout__974);
const kLayout__970: PoseidonExtRoundLayout = PoseidonExtRoundLayout(kLayout__701, IsZeroLayout(NondetRegLayout(Reg(72u)), NondetRegLayout(Reg(73u))), IsZeroLayout(NondetRegLayout(Reg(74u)), NondetRegLayout(Reg(75u))), IsZeroLayout(NondetRegLayout(Reg(76u)), NondetRegLayout(Reg(77u))), kLayout__971);
const kLayout__979: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(72u)), NondetRegLayout(Reg(73u))));
const kLayout__980: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(74u)), NondetRegLayout(Reg(75u))));
const kLayout__981: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(76u)), NondetRegLayout(Reg(77u))));
const kLayout__982: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(78u)), NondetRegLayout(Reg(79u))));
const kLayout__983: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(80u)), NondetRegLayout(Reg(81u))));
const kLayout__984: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(82u)), NondetRegLayout(Reg(83u))));
const kLayout__985: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(84u)), NondetRegLayout(Reg(85u))));
const kLayout__986: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(86u)), NondetRegLayout(Reg(87u))));
const kLayout__987: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(88u)), NondetRegLayout(Reg(89u))));
const kLayout__988: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(90u)), NondetRegLayout(Reg(91u))));
const kLayout__989: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(92u)), NondetRegLayout(Reg(93u))));
const kLayout__990: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(94u)), NondetRegLayout(Reg(95u))));
const kLayout__991: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(96u)), NondetRegLayout(Reg(97u))));
const kLayout__992: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(98u)), NondetRegLayout(Reg(99u))));
const kLayout__993: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(100u)), NondetRegLayout(Reg(101u))));
const kLayout__994: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(102u)), NondetRegLayout(Reg(103u))));
const kLayout__995: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(104u)), NondetRegLayout(Reg(105u))));
const kLayout__996: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(106u)), NondetRegLayout(Reg(107u))));
const kLayout__997: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(108u)), NondetRegLayout(Reg(109u))));
const kLayout__998: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(110u)), NondetRegLayout(Reg(111u))));
const kLayout__999: DoIntRoundLayout = DoIntRoundLayout(SBoxLayout(NondetRegLayout(Reg(112u)), NondetRegLayout(Reg(113u))));
const kLayout__978: DoIntRoundLayout21LayoutArray = DoIntRoundLayout21LayoutArray(kLayout__979, kLayout__980, kLayout__981, kLayout__982, kLayout__983, kLayout__984, kLayout__985, kLayout__986, kLayout__987, kLayout__988, kLayout__989, kLayout__990, kLayout__991, kLayout__992, kLayout__993, kLayout__994, kLayout__995, kLayout__996, kLayout__997, kLayout__998, kLayout__999);
const kLayout__977: DoIntRoundsLayout = DoIntRoundsLayout(kLayout__978);
const kLayout__976: PoseidonIntRoundsLayout = PoseidonIntRoundsLayout(kLayout__701, kLayout__977);
const kLayout__969: Poseidon1StateLayout = Poseidon1StateLayout(kLayout__701, kLayout__970, kLayout__976, kLayout__701, kLayout__701, kLayout__701, kLayout__701, kLayout__701, kLayout__701);
const kLayout__967: Poseidon1Layout = Poseidon1Layout(kLayout__968, kLayout__701, kLayout__969);
const kLayout__1001: DoCycleTableLayout = DoCycleTableLayout(CycleArgLayout(NondetRegLayout(Reg(187u)), NondetRegLayout(Reg(188u))), CycleArgLayout(NondetRegLayout(Reg(189u)), NondetRegLayout(Reg(190u))));
const kLayout__1003: NondetRegLayout32LayoutArray = NondetRegLayout32LayoutArray(NondetRegLayout(Reg(36u)), NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u)), NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u)), NondetRegLayout(Reg(41u)), NondetRegLayout(Reg(42u)), NondetRegLayout(Reg(43u)), NondetRegLayout(Reg(44u)), NondetRegLayout(Reg(45u)), NondetRegLayout(Reg(46u)), NondetRegLayout(Reg(47u)), NondetRegLayout(Reg(48u)), NondetRegLayout(Reg(49u)), NondetRegLayout(Reg(50u)), NondetRegLayout(Reg(51u)), NondetRegLayout(Reg(52u)), NondetRegLayout(Reg(53u)), NondetRegLayout(Reg(54u)), NondetRegLayout(Reg(55u)), NondetRegLayout(Reg(56u)), NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u)), NondetRegLayout(Reg(59u)), NondetRegLayout(Reg(60u)), NondetRegLayout(Reg(61u)), NondetRegLayout(Reg(62u)), NondetRegLayout(Reg(63u)), NondetRegLayout(Reg(64u)), NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(66u)), NondetRegLayout(Reg(67u)));
const kLayout__1004: NondetRegLayout32LayoutArray = NondetRegLayout32LayoutArray(NondetRegLayout(Reg(68u)), NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u)), NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u)), NondetRegLayout(Reg(73u)), NondetRegLayout(Reg(74u)), NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u)), NondetRegLayout(Reg(77u)), NondetRegLayout(Reg(78u)), NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u)), NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u)), NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(84u)), NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u)), NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u)), NondetRegLayout(Reg(89u)), NondetRegLayout(Reg(90u)), NondetRegLayout(Reg(91u)), NondetRegLayout(Reg(92u)), NondetRegLayout(Reg(93u)), NondetRegLayout(Reg(94u)), NondetRegLayout(Reg(95u)), NondetRegLayout(Reg(96u)), NondetRegLayout(Reg(97u)), NondetRegLayout(Reg(98u)), NondetRegLayout(Reg(99u)));
const kLayout__1005: NondetRegLayout32LayoutArray = NondetRegLayout32LayoutArray(NondetRegLayout(Reg(100u)), NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(102u)), NondetRegLayout(Reg(103u)), NondetRegLayout(Reg(104u)), NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u)), NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u)), NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u)), NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u)), NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u)), NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u)), NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u)), NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u)), NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u)), NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u)), NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u)), NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u)), NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u)), NondetRegLayout(Reg(131u)));
const kLayout__1002: ShaStateLayout = ShaStateLayout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u)), NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u)), NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u)), NondetRegLayout(Reg(35u)), kLayout__1003, kLayout__1004, kLayout__1005);
const kLayout__1008: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(132u)), NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u)), NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u)));
const kLayout__1009: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(138u)), NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u)));
const kLayout__1010: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u)), NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u)), NondetRegLayout(Reg(145u)));
const kLayout__1011: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(146u)), NondetRegLayout(Reg(142u)), NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u)), NondetRegLayout(Reg(149u)));
const kLayout__1012: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(150u)), NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u)), NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u)));
const kLayout__1013: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(156u)), NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u)));
const kLayout__1014: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u)), NondetRegLayout(Reg(163u)));
const kLayout__1015: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(164u)), NondetRegLayout(Reg(160u)), NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u)), NondetRegLayout(Reg(167u)));
const kLayout__1016: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u)), NondetRegLayout(Reg(172u)));
const kLayout__1017: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(174u)), NondetRegLayout(Reg(175u)), NondetRegLayout(Reg(176u)));
const kLayout__1007: MemoryArgLayout10LayoutArray = MemoryArgLayout10LayoutArray(kLayout__1008, kLayout__1009, kLayout__1010, kLayout__1011, kLayout__1012, kLayout__1013, kLayout__1014, kLayout__1015, kLayout__1016, kLayout__1017);
const kLayout__1018: CycleArgLayout5LayoutArray = CycleArgLayout5LayoutArray(CycleArgLayout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), CycleArgLayout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))), CycleArgLayout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u))), CycleArgLayout(NondetRegLayout(Reg(183u)), NondetRegLayout(Reg(184u))), CycleArgLayout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(186u))));
const kLayout__1006: _Arguments_Sha0StateLayout = _Arguments_Sha0StateLayout(kLayout__1007, kLayout__1018);
const kLayout__1023: MemoryIOLayout = MemoryIOLayout(kLayout__1008, kLayout__1009);
const kLayout__1025: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))));
const kLayout__1024: IsForwardLayout = IsForwardLayout(kLayout__1025);
const kLayout__1022: MemoryReadLayout = MemoryReadLayout(kLayout__1023, kLayout__1024);
const kLayout__1021: ReadAddrLayout = ReadAddrLayout(kLayout__1022);
const kLayout__1028: MemoryIOLayout = MemoryIOLayout(kLayout__1010, kLayout__1011);
const kLayout__1030: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))));
const kLayout__1029: IsForwardLayout = IsForwardLayout(kLayout__1030);
const kLayout__1027: MemoryReadLayout = MemoryReadLayout(kLayout__1028, kLayout__1029);
const kLayout__1026: ReadAddrLayout = ReadAddrLayout(kLayout__1027);
const kLayout__1033: MemoryIOLayout = MemoryIOLayout(kLayout__1012, kLayout__1013);
const kLayout__1035: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u))));
const kLayout__1034: IsForwardLayout = IsForwardLayout(kLayout__1035);
const kLayout__1032: MemoryReadLayout = MemoryReadLayout(kLayout__1033, kLayout__1034);
const kLayout__1031: ReadAddrLayout = ReadAddrLayout(kLayout__1032);
const kLayout__1037: MemoryIOLayout = MemoryIOLayout(kLayout__1014, kLayout__1015);
const kLayout__1039: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(183u)), NondetRegLayout(Reg(184u))));
const kLayout__1038: IsForwardLayout = IsForwardLayout(kLayout__1039);
const kLayout__1036: MemoryReadLayout = MemoryReadLayout(kLayout__1037, kLayout__1038);
const kLayout__1042: MemoryIOLayout = MemoryIOLayout(kLayout__1016, kLayout__1017);
const kLayout__1044: IsCycleLayout = IsCycleLayout(CycleArgLayout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(186u))));
const kLayout__1043: IsForwardLayout = IsForwardLayout(kLayout__1044);
const kLayout__1041: MemoryReadLayout = MemoryReadLayout(kLayout__1042, kLayout__1043);
const kLayout__1040: ReadAddrLayout = ReadAddrLayout(kLayout__1041);
const kLayout__1020: ShaEcallLayout = ShaEcallLayout(kLayout__1002, kLayout__1021, kLayout__1026, kLayout__1031, kLayout__1036, kLayout__1040);
const kLayout__1047: MemoryWriteLayout = MemoryWriteLayout(kLayout__1033, kLayout__1034);
const kLayout__1048: MemoryWriteLayout = MemoryWriteLayout(kLayout__1037, kLayout__1038);
const kLayout__1046: ShaLoadStateLayout = ShaLoadStateLayout(kLayout__1002, IsZeroLayout(NondetRegLayout(Reg(191u)), NondetRegLayout(Reg(192u))), IsZeroLayout(NondetRegLayout(Reg(193u)), NondetRegLayout(Reg(194u))), kLayout__1022, kLayout__1027, kLayout__1047, kLayout__1048);
const kLayout__1045: Sha0StateArm1Layout = Sha0StateArm1Layout(kLayout__1046, kLayout__1016, kLayout__1017, CycleArgLayout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(186u))));
const kLayout__1052: UnpackReg_32__16_Layout = UnpackReg_32__16_Layout(kLayout__1003);
const kLayout__1053: CarryExtractLayout = CarryExtractLayout(NondetRegLayout(Reg(193u)), NondetRegLayout(Reg(194u)), NondetRegLayout(Reg(195u)));
const kLayout__1054: CarryExtractLayout = CarryExtractLayout(NondetRegLayout(Reg(196u)), NondetRegLayout(Reg(197u)), NondetRegLayout(Reg(198u)));
const kLayout__1051: CarryAndExpandLayout = CarryAndExpandLayout(kLayout__1052, kLayout__1053, kLayout__1054);
const kLayout__1056: UnpackReg_32__16_Layout = UnpackReg_32__16_Layout(kLayout__1004);
const kLayout__1057: CarryExtractLayout = CarryExtractLayout(NondetRegLayout(Reg(199u)), NondetRegLayout(Reg(200u)), NondetRegLayout(Reg(201u)));
const kLayout__1058: CarryExtractLayout = CarryExtractLayout(NondetRegLayout(Reg(202u)), NondetRegLayout(Reg(203u)), NondetRegLayout(Reg(204u)));
const kLayout__1055: CarryAndExpandLayout = CarryAndExpandLayout(kLayout__1056, kLayout__1057, kLayout__1058);
const kLayout__1050: ShaLoadDataLayout = ShaLoadDataLayout(kLayout__1002, IsZeroLayout(NondetRegLayout(Reg(191u)), NondetRegLayout(Reg(192u))), kLayout__1022, kLayout__1027, kLayout__1005, kLayout__1051, kLayout__1055);
const kLayout__1049: Sha0StateArm2Layout = Sha0StateArm2Layout(kLayout__1050, kLayout__1012, kLayout__1013, kLayout__1014, kLayout__1015, kLayout__1016, kLayout__1017, CycleArgLayout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u))), CycleArgLayout(NondetRegLayout(Reg(183u)), NondetRegLayout(Reg(184u))), CycleArgLayout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(186u))));
const kLayout__1062: UnpackReg_32__16_Layout = UnpackReg_32__16_Layout(kLayout__1005);
const kLayout__1061: CarryAndExpandLayout = CarryAndExpandLayout(kLayout__1062, kLayout__1053, kLayout__1054);
const kLayout__1063: CarryAndExpandLayout = CarryAndExpandLayout(kLayout__1052, kLayout__1057, kLayout__1058);
const kLayout__1065: CarryExtractLayout = CarryExtractLayout(NondetRegLayout(Reg(205u)), NondetRegLayout(Reg(206u)), NondetRegLayout(Reg(207u)));
const kLayout__1066: CarryExtractLayout = CarryExtractLayout(NondetRegLayout(Reg(208u)), NondetRegLayout(Reg(209u)), NondetRegLayout(Reg(210u)));
const kLayout__1064: CarryAndExpandLayout = CarryAndExpandLayout(kLayout__1056, kLayout__1065, kLayout__1066);
const kLayout__1060: ShaMixLayout = ShaMixLayout(kLayout__1002, IsZeroLayout(NondetRegLayout(Reg(191u)), NondetRegLayout(Reg(192u))), kLayout__1022, kLayout__1061, kLayout__1063, kLayout__1064);
const kLayout__1059: Sha0StateArm3Layout = Sha0StateArm3Layout(kLayout__1060, kLayout__1010, kLayout__1011, kLayout__1012, kLayout__1013, kLayout__1014, kLayout__1015, kLayout__1016, kLayout__1017, CycleArgLayout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))), CycleArgLayout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u))), CycleArgLayout(NondetRegLayout(Reg(183u)), NondetRegLayout(Reg(184u))), CycleArgLayout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(186u))));
const kLayout__1070: CarryExtractLayout = CarryExtractLayout(NondetRegLayout(Reg(195u)), NondetRegLayout(Reg(196u)), NondetRegLayout(Reg(197u)));
const kLayout__1071: CarryExtractLayout = CarryExtractLayout(NondetRegLayout(Reg(198u)), NondetRegLayout(Reg(199u)), NondetRegLayout(Reg(200u)));
const kLayout__1069: CarryAndExpandLayout = CarryAndExpandLayout(kLayout__1052, kLayout__1070, kLayout__1071);
const kLayout__1073: CarryExtractLayout = CarryExtractLayout(NondetRegLayout(Reg(201u)), NondetRegLayout(Reg(202u)), NondetRegLayout(Reg(203u)));
const kLayout__1074: CarryExtractLayout = CarryExtractLayout(NondetRegLayout(Reg(204u)), NondetRegLayout(Reg(205u)), NondetRegLayout(Reg(206u)));
const kLayout__1072: CarryAndExpandLayout = CarryAndExpandLayout(kLayout__1056, kLayout__1073, kLayout__1074);
const kLayout__1075: MemoryWriteLayout = MemoryWriteLayout(kLayout__1023, kLayout__1024);
const kLayout__1076: MemoryWriteLayout = MemoryWriteLayout(kLayout__1028, kLayout__1029);
const kLayout__1068: ShaStoreStateLayout = ShaStoreStateLayout(kLayout__1002, IsZeroLayout(NondetRegLayout(Reg(191u)), NondetRegLayout(Reg(192u))), IsZeroLayout(NondetRegLayout(Reg(193u)), NondetRegLayout(Reg(194u))), kLayout__1069, kLayout__1072, kLayout__1075, kLayout__1076);
const kLayout__1067: Sha0StateArm4Layout = Sha0StateArm4Layout(kLayout__1068, kLayout__1012, kLayout__1013, kLayout__1014, kLayout__1015, kLayout__1016, kLayout__1017, CycleArgLayout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u))), CycleArgLayout(NondetRegLayout(Reg(183u)), NondetRegLayout(Reg(184u))), CycleArgLayout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(186u))));
const kLayout__1077: Sha0StateArm5Layout = Sha0StateArm5Layout(kLayout__1002, kLayout__1008, kLayout__1009, kLayout__1010, kLayout__1011, kLayout__1012, kLayout__1013, kLayout__1014, kLayout__1015, kLayout__1016, kLayout__1017, CycleArgLayout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), CycleArgLayout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))), CycleArgLayout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u))), CycleArgLayout(NondetRegLayout(Reg(183u)), NondetRegLayout(Reg(184u))), CycleArgLayout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(186u))));
const kLayout__1078: Sha0StateArm6Layout = Sha0StateArm6Layout(kLayout__1002, kLayout__1008, kLayout__1009, kLayout__1010, kLayout__1011, kLayout__1012, kLayout__1013, kLayout__1014, kLayout__1015, kLayout__1016, kLayout__1017, CycleArgLayout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), CycleArgLayout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))), CycleArgLayout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u))), CycleArgLayout(NondetRegLayout(Reg(183u)), NondetRegLayout(Reg(184u))), CycleArgLayout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(186u))));
const kLayout__1079: Sha0StateArm7Layout = Sha0StateArm7Layout(kLayout__1002, kLayout__1008, kLayout__1009, kLayout__1010, kLayout__1011, kLayout__1012, kLayout__1013, kLayout__1014, kLayout__1015, kLayout__1016, kLayout__1017, CycleArgLayout(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u))), CycleArgLayout(NondetRegLayout(Reg(179u)), NondetRegLayout(Reg(180u))), CycleArgLayout(NondetRegLayout(Reg(181u)), NondetRegLayout(Reg(182u))), CycleArgLayout(NondetRegLayout(Reg(183u)), NondetRegLayout(Reg(184u))), CycleArgLayout(NondetRegLayout(Reg(185u)), NondetRegLayout(Reg(186u))));
const kLayout__1019: Sha0StateLayout = Sha0StateLayout(kLayout__1002, kLayout__1020, kLayout__1045, kLayout__1049, kLayout__1059, kLayout__1067, kLayout__1077, kLayout__1078, kLayout__1079);
const kLayout__1000: Sha0Layout = Sha0Layout(kLayout__1001, kLayout__1002, kLayout__1006, kLayout__1019);
const kLayout__1081: DoCycleTableLayout = DoCycleTableLayout(CycleArgLayout(NondetRegLayout(Reg(161u)), NondetRegLayout(Reg(162u))), CycleArgLayout(NondetRegLayout(Reg(163u)), NondetRegLayout(Reg(164u))));
const kLayout__1083: NondetRegLayout16LayoutArray = NondetRegLayout16LayoutArray(NondetRegLayout(Reg(34u)), NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u)), NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u)), NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u)), NondetRegLayout(Reg(41u)), NondetRegLayout(Reg(42u)), NondetRegLayout(Reg(43u)), NondetRegLayout(Reg(44u)), NondetRegLayout(Reg(45u)), NondetRegLayout(Reg(46u)), NondetRegLayout(Reg(47u)), NondetRegLayout(Reg(48u)), NondetRegLayout(Reg(49u)));
const kLayout__1082: BigIntStateLayout = BigIntStateLayout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u)), NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u)), NondetRegLayout(Reg(33u)), kLayout__1083, NondetRegLayout(Reg(50u)));
const kLayout__1086: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(51u)), NondetRegLayout(Reg(52u)), NondetRegLayout(Reg(53u)), NondetRegLayout(Reg(54u)), NondetRegLayout(Reg(55u)));
const kLayout__1087: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(56u)), NondetRegLayout(Reg(52u)), NondetRegLayout(Reg(57u)), NondetRegLayout(Reg(58u)), NondetRegLayout(Reg(59u)));
const kLayout__1088: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(60u)), NondetRegLayout(Reg(61u)), NondetRegLayout(Reg(62u)), NondetRegLayout(Reg(63u)), NondetRegLayout(Reg(64u)));
const kLayout__1089: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(65u)), NondetRegLayout(Reg(61u)), NondetRegLayout(Reg(66u)), NondetRegLayout(Reg(67u)), NondetRegLayout(Reg(68u)));
const kLayout__1090: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(69u)), NondetRegLayout(Reg(70u)), NondetRegLayout(Reg(71u)), NondetRegLayout(Reg(72u)), NondetRegLayout(Reg(73u)));
const kLayout__1091: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(74u)), NondetRegLayout(Reg(70u)), NondetRegLayout(Reg(75u)), NondetRegLayout(Reg(76u)), NondetRegLayout(Reg(77u)));
const kLayout__1092: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(78u)), NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(80u)), NondetRegLayout(Reg(81u)), NondetRegLayout(Reg(82u)));
const kLayout__1093: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(83u)), NondetRegLayout(Reg(79u)), NondetRegLayout(Reg(84u)), NondetRegLayout(Reg(85u)), NondetRegLayout(Reg(86u)));
const kLayout__1094: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u)), NondetRegLayout(Reg(89u)), NondetRegLayout(Reg(90u)), NondetRegLayout(Reg(91u)));
const kLayout__1095: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(92u)), NondetRegLayout(Reg(88u)), NondetRegLayout(Reg(93u)), NondetRegLayout(Reg(94u)), NondetRegLayout(Reg(95u)));
const kLayout__1096: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(96u)), NondetRegLayout(Reg(97u)), NondetRegLayout(Reg(98u)), NondetRegLayout(Reg(99u)), NondetRegLayout(Reg(100u)));
const kLayout__1097: MemoryArgLayout = MemoryArgLayout(NondetRegLayout(Reg(101u)), NondetRegLayout(Reg(97u)), NondetRegLayout(Reg(102u)), NondetRegLayout(Reg(103u)), NondetRegLayout(Reg(104u)));
const kLayout__1085: MemoryArgLayout12LayoutArray = MemoryArgLayout12LayoutArray(kLayout__1086, kLayout__1087, kLayout__1088, kLayout__1089, kLayout__1090, kLayout__1091, kLayout__1092, kLayout__1093, kLayout__1094, kLayout__1095, kLayout__1096, kLayout__1097);
const kLayout__1098: CycleArgLayout6LayoutArray = CycleArgLayout6LayoutArray(CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))));
const kLayout__1099: ArgU8Layout18LayoutArray = ArgU8Layout18LayoutArray(ArgU8Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU8Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU8Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU8Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU8Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU8Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU8Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU8Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU8Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU8Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU8Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU8Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU8Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU8Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU8Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU8Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))));
const kLayout__1100: ArgU16Layout4LayoutArray = ArgU16Layout4LayoutArray(ArgU16Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU16Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU16Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))));
const kLayout__1084: _Arguments_BigInt0StateLayout = _Arguments_BigInt0StateLayout(kLayout__1085, kLayout__1098, kLayout__1099, kLayout__1100);
const kLayout__1105: MemoryIOLayout = MemoryIOLayout(kLayout__1086, kLayout__1087);
const kLayout__1104: MemoryReadLayout = MemoryReadLayout(kLayout__1105, kLayout__511);
const kLayout__1108: MemoryIOLayout = MemoryIOLayout(kLayout__1088, kLayout__1089);
const kLayout__1107: MemoryReadLayout = MemoryReadLayout(kLayout__1108, kLayout__515);
const kLayout__1106: ReadAddrLayout = ReadAddrLayout(kLayout__1107);
const kLayout__1103: BigIntEcallLayout = BigIntEcallLayout(kLayout__1082, kLayout__1104, kLayout__1106);
const kLayout__1102: BigInt0StateArm0Layout = BigInt0StateArm0Layout(kLayout__1103, kLayout__1090, kLayout__1091, kLayout__1092, kLayout__1093, kLayout__1094, kLayout__1095, kLayout__1096, kLayout__1097, CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))), ArgU8Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU8Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU8Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU8Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU8Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU8Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU8Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU8Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU8Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU8Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU8Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU8Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU8Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU8Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU8Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU8Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU16Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU16Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU16Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))));
const kLayout__1111: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))));
const kLayout__1112: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))));
const kLayout__1110: SplitWordLayout = SplitWordLayout(kLayout__1111, kLayout__1112);
const kLayout__1113: NondetRegLayout5LayoutArray = NondetRegLayout5LayoutArray(NondetRegLayout(Reg(167u)), NondetRegLayout(Reg(168u)), NondetRegLayout(Reg(169u)), NondetRegLayout(Reg(170u)), NondetRegLayout(Reg(171u)));
const kLayout__1115: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))));
const kLayout__1116: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))));
const kLayout__1114: NormalizeU32Layout = NormalizeU32Layout(kLayout__1115, NondetRegLayout(Reg(175u)), kLayout__1116, NondetRegLayout(Reg(176u)));
const kLayout__1117: OneHot_3_Layout = OneHot_3_Layout(NondetRegLayout3LayoutArray(NondetRegLayout(Reg(177u)), NondetRegLayout(Reg(178u)), NondetRegLayout(Reg(179u))));
const kLayout__1119: ArgU16Layout2LayoutArray = ArgU16Layout2LayoutArray(ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU16Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))));
const kLayout__1120: ArgU8Layout16LayoutArray = ArgU8Layout16LayoutArray(ArgU8Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU8Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU8Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU8Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU8Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU8Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU8Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU8Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU8Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU8Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU8Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU8Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU8Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU8Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))));
const kLayout__1121: MemoryArgLayout8LayoutArray = MemoryArgLayout8LayoutArray(kLayout__1090, kLayout__1091, kLayout__1092, kLayout__1093, kLayout__1094, kLayout__1095, kLayout__1096, kLayout__1097);
const kLayout__1122: CycleArgLayout4LayoutArray = CycleArgLayout4LayoutArray(CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))));
const kLayout__1118: _Arguments_BigIntStepBytesLayout = _Arguments_BigIntStepBytesLayout(kLayout__1119, kLayout__1120, kLayout__1121, kLayout__1122);
const kLayout__1127: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))));
const kLayout__1128: NondetU16RegLayout = NondetU16RegLayout(ArgU16Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))));
const kLayout__1126: AddrDecomposeBitsLayout = AddrDecomposeBitsLayout(NondetRegLayout(Reg(180u)), NondetRegLayout(Reg(181u)), kLayout__1127, IsZeroLayout(NondetRegLayout(Reg(182u)), NondetRegLayout(Reg(183u))), kLayout__1128);
const kLayout__1125: BigIntAddrLayout = BigIntAddrLayout(kLayout__1126, IsZeroLayout(NondetRegLayout(Reg(184u)), NondetRegLayout(Reg(185u))));
const kLayout__1133: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))));
const kLayout__1134: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))));
const kLayout__1132: SplitWordLayout = SplitWordLayout(kLayout__1133, kLayout__1134);
const kLayout__1136: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))));
const kLayout__1137: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))));
const kLayout__1135: SplitWordLayout = SplitWordLayout(kLayout__1136, kLayout__1137);
const kLayout__1131: SplitU32Layout = SplitU32Layout(kLayout__1132, kLayout__1135);
const kLayout__1139: MemoryIOLayout = MemoryIOLayout(kLayout__1090, kLayout__1091);
const kLayout__1138: MemoryReadLayout = MemoryReadLayout(kLayout__1139, kLayout__519);
const kLayout__1130: BigIntReadWords_SuperLayout = BigIntReadWords_SuperLayout(kLayout__1131, kLayout__1138);
const kLayout__1143: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))));
const kLayout__1144: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))));
const kLayout__1142: SplitWordLayout = SplitWordLayout(kLayout__1143, kLayout__1144);
const kLayout__1146: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))));
const kLayout__1147: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))));
const kLayout__1145: SplitWordLayout = SplitWordLayout(kLayout__1146, kLayout__1147);
const kLayout__1141: SplitU32Layout = SplitU32Layout(kLayout__1142, kLayout__1145);
const kLayout__1149: MemoryIOLayout = MemoryIOLayout(kLayout__1092, kLayout__1093);
const kLayout__1148: MemoryReadLayout = MemoryReadLayout(kLayout__1149, kLayout__523);
const kLayout__1140: BigIntReadWords_SuperLayout = BigIntReadWords_SuperLayout(kLayout__1141, kLayout__1148);
const kLayout__1153: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))));
const kLayout__1154: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))));
const kLayout__1152: SplitWordLayout = SplitWordLayout(kLayout__1153, kLayout__1154);
const kLayout__1156: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))));
const kLayout__1157: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))));
const kLayout__1155: SplitWordLayout = SplitWordLayout(kLayout__1156, kLayout__1157);
const kLayout__1151: SplitU32Layout = SplitU32Layout(kLayout__1152, kLayout__1155);
const kLayout__1159: MemoryIOLayout = MemoryIOLayout(kLayout__1094, kLayout__1095);
const kLayout__1158: MemoryReadLayout = MemoryReadLayout(kLayout__1159, kLayout__527);
const kLayout__1150: BigIntReadWords_SuperLayout = BigIntReadWords_SuperLayout(kLayout__1151, kLayout__1158);
const kLayout__1163: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))));
const kLayout__1164: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))));
const kLayout__1162: SplitWordLayout = SplitWordLayout(kLayout__1163, kLayout__1164);
const kLayout__1166: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))));
const kLayout__1167: NondetU8RegLayout = NondetU8RegLayout(ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))));
const kLayout__1165: SplitWordLayout = SplitWordLayout(kLayout__1166, kLayout__1167);
const kLayout__1161: SplitU32Layout = SplitU32Layout(kLayout__1162, kLayout__1165);
const kLayout__1169: MemoryIOLayout = MemoryIOLayout(kLayout__1096, kLayout__1097);
const kLayout__1168: MemoryReadLayout = MemoryReadLayout(kLayout__1169, kLayout__531);
const kLayout__1160: BigIntReadWords_SuperLayout = BigIntReadWords_SuperLayout(kLayout__1161, kLayout__1168);
const kLayout__1129: BigIntReadWords_SuperLayout4LayoutArray = BigIntReadWords_SuperLayout4LayoutArray(kLayout__1130, kLayout__1140, kLayout__1150, kLayout__1160);
const kLayout__1124: BigIntReadLayout = BigIntReadLayout(kLayout__1125, kLayout__1129);
const kLayout__1172: NondetU8RegLayout16LayoutArray = NondetU8RegLayout16LayoutArray(kLayout__1133, kLayout__1134, kLayout__1136, kLayout__1137, kLayout__1143, kLayout__1144, kLayout__1146, kLayout__1147, kLayout__1153, kLayout__1154, kLayout__1156, kLayout__1157, kLayout__1163, kLayout__1164, kLayout__1166, kLayout__1167);
const kLayout__1171: BigIntWitnessLayout = BigIntWitnessLayout(kLayout__1172);
const kLayout__1175: MemoryWriteLayout = MemoryWriteLayout(kLayout__1139, kLayout__519);
const kLayout__1174: BigIntWrite__0_SuperLayout = BigIntWrite__0_SuperLayout(kLayout__1175);
const kLayout__1177: MemoryWriteLayout = MemoryWriteLayout(kLayout__1149, kLayout__523);
const kLayout__1176: BigIntWrite__0_SuperLayout = BigIntWrite__0_SuperLayout(kLayout__1177);
const kLayout__1179: MemoryWriteLayout = MemoryWriteLayout(kLayout__1159, kLayout__527);
const kLayout__1178: BigIntWrite__0_SuperLayout = BigIntWrite__0_SuperLayout(kLayout__1179);
const kLayout__1181: MemoryWriteLayout = MemoryWriteLayout(kLayout__1169, kLayout__531);
const kLayout__1180: BigIntWrite__0_SuperLayout = BigIntWrite__0_SuperLayout(kLayout__1181);
const kLayout__1173: BigIntWrite__0_SuperLayout4LayoutArray = BigIntWrite__0_SuperLayout4LayoutArray(kLayout__1174, kLayout__1176, kLayout__1178, kLayout__1180);
const kLayout__1170: BigIntWriteLayout = BigIntWriteLayout(kLayout__1171, kLayout__1125, kLayout__1173);
const kLayout__1182: BigIntStepBytesArm2Layout = BigIntStepBytesArm2Layout(kLayout__1171, ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU16Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))), kLayout__1090, kLayout__1091, kLayout__1092, kLayout__1093, kLayout__1094, kLayout__1095, kLayout__1096, kLayout__1097, CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))));
const kLayout__1123: BigIntStepBytesLayout = BigIntStepBytesLayout(kLayout__1124, kLayout__1170, kLayout__1182);
const kLayout__1109: BigIntStepLayout = BigIntStepLayout(kLayout__1082, kLayout__1104, kLayout__1110, NondetRegLayout(Reg(165u)), NondetRegLayout(Reg(166u)), kLayout__1113, NondetRegLayout3LayoutArray(NondetRegLayout(Reg(172u)), NondetRegLayout(Reg(173u)), NondetRegLayout(Reg(174u))), kLayout__1107, kLayout__1114, kLayout__1117, kLayout__1118, kLayout__1123, IsZeroLayout(NondetRegLayout(Reg(186u)), NondetRegLayout(Reg(187u))));
const kLayout__1183: BigInt0StateArm2Layout = BigInt0StateArm2Layout(kLayout__1082, kLayout__1086, kLayout__1087, kLayout__1088, kLayout__1089, kLayout__1090, kLayout__1091, kLayout__1092, kLayout__1093, kLayout__1094, kLayout__1095, kLayout__1096, kLayout__1097, CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))), ArgU8Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU8Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU8Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU8Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU8Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU8Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU8Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU8Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU8Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU8Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU8Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU8Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU8Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU8Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU8Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU8Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU16Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU16Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU16Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))));
const kLayout__1184: BigInt0StateArm3Layout = BigInt0StateArm3Layout(kLayout__1082, kLayout__1086, kLayout__1087, kLayout__1088, kLayout__1089, kLayout__1090, kLayout__1091, kLayout__1092, kLayout__1093, kLayout__1094, kLayout__1095, kLayout__1096, kLayout__1097, CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))), ArgU8Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU8Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU8Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU8Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU8Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU8Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU8Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU8Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU8Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU8Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU8Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU8Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU8Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU8Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU8Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU8Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU16Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU16Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU16Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))));
const kLayout__1185: BigInt0StateArm4Layout = BigInt0StateArm4Layout(kLayout__1082, kLayout__1086, kLayout__1087, kLayout__1088, kLayout__1089, kLayout__1090, kLayout__1091, kLayout__1092, kLayout__1093, kLayout__1094, kLayout__1095, kLayout__1096, kLayout__1097, CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))), ArgU8Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU8Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU8Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU8Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU8Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU8Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU8Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU8Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU8Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU8Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU8Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU8Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU8Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU8Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU8Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU8Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU16Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU16Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU16Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))));
const kLayout__1186: BigInt0StateArm5Layout = BigInt0StateArm5Layout(kLayout__1082, kLayout__1086, kLayout__1087, kLayout__1088, kLayout__1089, kLayout__1090, kLayout__1091, kLayout__1092, kLayout__1093, kLayout__1094, kLayout__1095, kLayout__1096, kLayout__1097, CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))), ArgU8Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU8Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU8Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU8Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU8Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU8Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU8Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU8Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU8Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU8Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU8Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU8Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU8Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU8Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU8Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU8Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU16Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU16Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU16Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))));
const kLayout__1187: BigInt0StateArm6Layout = BigInt0StateArm6Layout(kLayout__1082, kLayout__1086, kLayout__1087, kLayout__1088, kLayout__1089, kLayout__1090, kLayout__1091, kLayout__1092, kLayout__1093, kLayout__1094, kLayout__1095, kLayout__1096, kLayout__1097, CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))), ArgU8Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU8Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU8Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU8Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU8Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU8Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU8Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU8Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU8Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU8Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU8Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU8Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU8Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU8Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU8Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU8Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU16Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU16Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU16Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))));
const kLayout__1188: BigInt0StateArm7Layout = BigInt0StateArm7Layout(kLayout__1082, kLayout__1086, kLayout__1087, kLayout__1088, kLayout__1089, kLayout__1090, kLayout__1091, kLayout__1092, kLayout__1093, kLayout__1094, kLayout__1095, kLayout__1096, kLayout__1097, CycleArgLayout(NondetRegLayout(Reg(105u)), NondetRegLayout(Reg(106u))), CycleArgLayout(NondetRegLayout(Reg(107u)), NondetRegLayout(Reg(108u))), CycleArgLayout(NondetRegLayout(Reg(109u)), NondetRegLayout(Reg(110u))), CycleArgLayout(NondetRegLayout(Reg(111u)), NondetRegLayout(Reg(112u))), CycleArgLayout(NondetRegLayout(Reg(113u)), NondetRegLayout(Reg(114u))), CycleArgLayout(NondetRegLayout(Reg(115u)), NondetRegLayout(Reg(116u))), ArgU8Layout(NondetRegLayout(Reg(117u)), NondetRegLayout(Reg(118u))), ArgU8Layout(NondetRegLayout(Reg(119u)), NondetRegLayout(Reg(120u))), ArgU8Layout(NondetRegLayout(Reg(121u)), NondetRegLayout(Reg(122u))), ArgU8Layout(NondetRegLayout(Reg(123u)), NondetRegLayout(Reg(124u))), ArgU8Layout(NondetRegLayout(Reg(125u)), NondetRegLayout(Reg(126u))), ArgU8Layout(NondetRegLayout(Reg(127u)), NondetRegLayout(Reg(128u))), ArgU8Layout(NondetRegLayout(Reg(129u)), NondetRegLayout(Reg(130u))), ArgU8Layout(NondetRegLayout(Reg(131u)), NondetRegLayout(Reg(132u))), ArgU8Layout(NondetRegLayout(Reg(133u)), NondetRegLayout(Reg(134u))), ArgU8Layout(NondetRegLayout(Reg(135u)), NondetRegLayout(Reg(136u))), ArgU8Layout(NondetRegLayout(Reg(137u)), NondetRegLayout(Reg(138u))), ArgU8Layout(NondetRegLayout(Reg(139u)), NondetRegLayout(Reg(140u))), ArgU8Layout(NondetRegLayout(Reg(141u)), NondetRegLayout(Reg(142u))), ArgU8Layout(NondetRegLayout(Reg(143u)), NondetRegLayout(Reg(144u))), ArgU8Layout(NondetRegLayout(Reg(145u)), NondetRegLayout(Reg(146u))), ArgU8Layout(NondetRegLayout(Reg(147u)), NondetRegLayout(Reg(148u))), ArgU8Layout(NondetRegLayout(Reg(149u)), NondetRegLayout(Reg(150u))), ArgU8Layout(NondetRegLayout(Reg(151u)), NondetRegLayout(Reg(152u))), ArgU16Layout(NondetRegLayout(Reg(153u)), NondetRegLayout(Reg(154u))), ArgU16Layout(NondetRegLayout(Reg(155u)), NondetRegLayout(Reg(156u))), ArgU16Layout(NondetRegLayout(Reg(157u)), NondetRegLayout(Reg(158u))), ArgU16Layout(NondetRegLayout(Reg(159u)), NondetRegLayout(Reg(160u))));
const kLayout__1101: BigInt0StateLayout = BigInt0StateLayout(kLayout__1082, kLayout__1102, kLayout__1109, kLayout__1183, kLayout__1184, kLayout__1185, kLayout__1186, kLayout__1187, kLayout__1188);
const kLayout__1080: BigInt0Layout = BigInt0Layout(kLayout__1081, kLayout__1082, kLayout__1084, kLayout__1101);
const kLayout__13: TopInstResultLayout = TopInstResultLayout(kLayout__12, kLayout__14, kLayout__103, kLayout__121, kLayout__135, kLayout__225, kLayout__343, kLayout__401, kLayout__459, kLayout__634, kLayout__699, kLayout__967, kLayout__1000, kLayout__1080);
const kLayout__7: TopLayout = TopLayout(NondetRegLayout(Reg(0u)), NondetRegLayout(Reg(14u)), NondetRegLayout(Reg(15u)), NondetRegLayout(Reg(16u)), NondetRegLayout(Reg(17u)), NondetRegLayout(Reg(18u)), TopCycleLayout(NondetRegLayout(Reg(0u)), NondetRegLayout(Reg(0u)), NondetRegLayout(Reg(0u))), NondetRegLayout(Reg(19u)), NondetRegLayout(Reg(20u)), kLayout__8, kLayout__11, kLayout__13);
const kLayout__1190: DigestRegValues_SuperLayout8LayoutArray = DigestRegValues_SuperLayout8LayoutArray(DigestRegValues_SuperLayout(NondetRegLayout(Reg(0u)), NondetRegLayout(Reg(1u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(2u)), NondetRegLayout(Reg(3u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(4u)), NondetRegLayout(Reg(5u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(6u)), NondetRegLayout(Reg(7u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(8u)), NondetRegLayout(Reg(9u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(10u)), NondetRegLayout(Reg(11u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(12u)), NondetRegLayout(Reg(13u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(14u)), NondetRegLayout(Reg(15u))));
const kLayout__1189: DigestRegLayout = DigestRegLayout(kLayout__1190);
const kLayout__1192: DigestRegValues_SuperLayout8LayoutArray = DigestRegValues_SuperLayout8LayoutArray(DigestRegValues_SuperLayout(NondetRegLayout(Reg(17u)), NondetRegLayout(Reg(18u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(19u)), NondetRegLayout(Reg(20u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(21u)), NondetRegLayout(Reg(22u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(23u)), NondetRegLayout(Reg(24u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(25u)), NondetRegLayout(Reg(26u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(27u)), NondetRegLayout(Reg(28u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(29u)), NondetRegLayout(Reg(30u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(31u)), NondetRegLayout(Reg(32u))));
const kLayout__1191: DigestRegLayout = DigestRegLayout(kLayout__1192);
const kLayout__1194: DigestRegValues_SuperLayout8LayoutArray = DigestRegValues_SuperLayout8LayoutArray(DigestRegValues_SuperLayout(NondetRegLayout(Reg(33u)), NondetRegLayout(Reg(34u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(35u)), NondetRegLayout(Reg(36u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(37u)), NondetRegLayout(Reg(38u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(39u)), NondetRegLayout(Reg(40u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(41u)), NondetRegLayout(Reg(42u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(43u)), NondetRegLayout(Reg(44u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(45u)), NondetRegLayout(Reg(46u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(47u)), NondetRegLayout(Reg(48u))));
const kLayout__1193: DigestRegLayout = DigestRegLayout(kLayout__1194);
const kLayout__1196: DigestRegValues_SuperLayout8LayoutArray = DigestRegValues_SuperLayout8LayoutArray(DigestRegValues_SuperLayout(NondetRegLayout(Reg(54u)), NondetRegLayout(Reg(55u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(56u)), NondetRegLayout(Reg(57u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(58u)), NondetRegLayout(Reg(59u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(60u)), NondetRegLayout(Reg(61u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(62u)), NondetRegLayout(Reg(63u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(64u)), NondetRegLayout(Reg(65u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(66u)), NondetRegLayout(Reg(67u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(68u)), NondetRegLayout(Reg(69u))));
const kLayout__1195: DigestRegLayout = DigestRegLayout(kLayout__1196);
const kLayout__1198: DigestRegValues_SuperLayout8LayoutArray = DigestRegValues_SuperLayout8LayoutArray(DigestRegValues_SuperLayout(NondetRegLayout(Reg(70u)), NondetRegLayout(Reg(71u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(72u)), NondetRegLayout(Reg(73u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(74u)), NondetRegLayout(Reg(75u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(76u)), NondetRegLayout(Reg(77u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(78u)), NondetRegLayout(Reg(79u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(80u)), NondetRegLayout(Reg(81u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(82u)), NondetRegLayout(Reg(83u))), DigestRegValues_SuperLayout(NondetRegLayout(Reg(84u)), NondetRegLayout(Reg(85u))));
const kLayout__1197: DigestRegLayout = DigestRegLayout(kLayout__1198);
const kLayout__1199: _accumLayout = _accumLayout(Arg_ArgU8Layout(Reg(0u)), Arg_ArgU16Layout(Reg(4u)), Arg_MemoryArgLayout(Reg(8u), Reg(12u), Reg(16u), Reg(20u)), Arg_CycleArgLayout(Reg(24u)), Reg(28u), Reg1LayoutArray(Reg(32u)));
const kLayoutTestSuccRunAccum: LayoutAccumLayout = LayoutAccumLayout(kLayout__0, Reg20LayoutArray(Reg(23u), Reg(27u), Reg(31u), Reg(35u), Reg(39u), Reg(43u), Reg(47u), Reg(51u), Reg(55u), Reg(59u), Reg(63u), Reg(67u), Reg(71u), Reg(75u), Reg(79u), Reg(83u), Reg(87u), Reg(91u), Reg(95u), Reg(99u)));
const kLayout_TopAccum: LayoutAccumLayout = LayoutAccumLayout(kLayout__0, Reg20LayoutArray(Reg(23u), Reg(27u), Reg(31u), Reg(35u), Reg(39u), Reg(43u), Reg(47u), Reg(51u), Reg(55u), Reg(59u), Reg(63u), Reg(67u), Reg(71u), Reg(75u), Reg(79u), Reg(83u), Reg(87u), Reg(91u), Reg(95u), Reg(99u)));
const kLayoutTestSuccRun: TestSuccRunLayout = TestSuccRunLayout(kLayout__7);
const kLayout_Top: TopLayout = TopLayout(NondetRegLayout(Reg(0u)), NondetRegLayout(Reg(14u)), NondetRegLayout(Reg(15u)), NondetRegLayout(Reg(16u)), NondetRegLayout(Reg(17u)), NondetRegLayout(Reg(18u)), TopCycleLayout(NondetRegLayout(Reg(0u)), NondetRegLayout(Reg(0u)), NondetRegLayout(Reg(0u))), NondetRegLayout(Reg(19u)), NondetRegLayout(Reg(20u)), kLayout__8, kLayout__11, kLayout__13);
const kLayoutGlobal: _globalLayout = _globalLayout(kLayout__1189, NondetRegLayout(Reg(16u)), kLayout__1191, kLayout__1193, NondetExtRegLayout(Reg(49u)), NondetRegLayout(Reg(53u)), kLayout__1195, kLayout__1197, NondetRegLayout(Reg(86u)), NondetRegLayout(Reg(87u)), NondetRegLayout(Reg(88u)), NondetRegLayout(Reg(89u)));
const kLayoutMix: _mixLayout = _mixLayout(kLayout__1199);
