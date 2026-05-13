// SP3 staged eval_check WGSL prelude.
//
// Layout mirrors `webgpu.rs:EVAL_CHECK_BASE_INTERPRETER_WGSL`'s bindings,
// `Params` UBO, and field-arithmetic helpers — minus the `instrs` storage
// buffer and the `instr_*` reading machinery (the staged kernel embeds the
// DEF inline, so there's no instruction stream to walk at runtime).
//
// The per-DEF body emitted by `WgslEmitter` is appended after this prelude
// and references `add`/`sub`/`mul`, `ext_add`/`ext_sub`/`ext_mul`/`ext_scale`,
// `read_tap_scalar`/`read_tap_ext`, `read_global_scalar`/`read_global_ext`,
// `load_mix_pow`, and `write_check` — all defined here.
//
// SP3 iter 3 (this file) ships the static prelude only. SP3 iter 4 wires
// the dispatch path: a `dispatch_eval_check_poly_ext_staged` helper in
// `webgpu.rs` concatenates this prelude with the body, creates a pipeline
// via the existing `create_compute_kernel`, binds the same buffers as the
// interpreter (sans `instrs`), and dispatches. Until iter 4 lands, this
// prelude is reachable only from unit tests in `webgpu_codegen::tests`
// that validate its shape.

const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    check_base: u32,
    group0_base: u32,
    group1_base: u32,
    group2_base: u32,
    global0_base: u32,
    global1_base: u32,
    domain: u32,
    instr_count: u32,       // ignored by staged path; kept for UBO layout parity
    instr_base: u32,        // ignored by staged path
    mix_pows_base: u32,
    ret_mix_slot: u32,      // ignored by staged path (ret is baked into write_check call)
    dispatch_count: u32,
    cycle_base: u32,
    group0_chunk_base: u32,
    group0_chunk_rows: u32,
    group1_chunk_base: u32,
    group1_chunk_rows: u32,
    group2_chunk_base: u32,
    group2_chunk_rows: u32,
    _pad0: u32,
    invs: vec4<u32>,
};

@group(0) @binding(0) var<storage, read_write> check: ElemBuffer;
@group(0) @binding(1) var<storage, read> group0: ElemBuffer;
@group(0) @binding(2) var<storage, read> group1: ElemBuffer;
@group(0) @binding(3) var<storage, read> group2: ElemBuffer;
@group(0) @binding(4) var<storage, read> global0: ElemBuffer;
@group(0) @binding(5) var<storage, read> global1: ElemBuffer;
// binding 6 is intentionally skipped — it's `instrs` for the runtime
// interpreter, unused by the staged kernel.
// SP3 iter 7x: `mix_pows` is a uniform buffer (CUDA `__constant__`
// analog). Fixed-size 16384 vec4 = 256 KiB matches the bumped
// `maxUniformBufferBindingSize` limit in the device descriptor. Each
// `eval_check` call writes only the prefix the DEF actually uses;
// `load_mix_pow` indexes into the array. UBO reads are GPU-cached
// and broadcast-friendly, which is the win over the previous
// `var<storage, read>` declaration.
struct MixPowsUbo {
    data: array<vec4<u32>, 16384>,
};
@group(0) @binding(7) var<uniform> mix_pows: MixPowsUbo;
@group(0) @binding(8) var<uniform> params: Params;

// ----- BabyBear scalar arithmetic --------------------------------------

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
}

fn sub(lhs: u32, rhs: u32) -> u32 {
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

fn mul(lhs: u32, rhs: u32) -> u32 {
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

// ----- BabyBearExt (vec4<u32>) arithmetic -------------------------------

fn ext_add(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        add(lhs.x, rhs.x),
        add(lhs.y, rhs.y),
        add(lhs.z, rhs.z),
        add(lhs.w, rhs.w),
    );
}

fn ext_sub(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        sub(lhs.x, rhs.x),
        sub(lhs.y, rhs.y),
        sub(lhs.z, rhs.z),
        sub(lhs.w, rhs.w),
    );
}

fn ext_mul(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        add(
            mul(lhs.x, rhs.x),
            mul(NBETA, add(add(mul(lhs.y, rhs.w), mul(lhs.z, rhs.z)), mul(lhs.w, rhs.y))),
        ),
        add(
            add(mul(lhs.x, rhs.y), mul(lhs.y, rhs.x)),
            mul(NBETA, add(mul(lhs.z, rhs.w), mul(lhs.w, rhs.z))),
        ),
        add(
            add(add(mul(lhs.x, rhs.z), mul(lhs.y, rhs.y)), mul(lhs.z, rhs.x)),
            mul(NBETA, mul(lhs.w, rhs.w)),
        ),
        add(add(add(mul(lhs.x, rhs.w), mul(lhs.y, rhs.z)), mul(lhs.z, rhs.y)), mul(lhs.w, rhs.x)),
    );
}

fn ext_scale(lhs: vec4<u32>, rhs: u32) -> vec4<u32> {
    return vec4<u32>(
        mul(lhs.x, rhs),
        mul(lhs.y, rhs),
        mul(lhs.z, rhs),
        mul(lhs.w, rhs),
    );
}

// ----- Tap / global reads -----------------------------------------------
//
// The emitter resolves `PolyExtStep::Get(tap_idx)` to a concrete
// `(group, offset, back_inv_rate)` triple at codegen time and emits one
// of `read_g{0,1,2}_{scalar,ext}({offset}u, {back_inv_rate}u, cycle)`.
// Each helper indexes its dedicated group binding using the same chunk
// math as the runtime interpreter at
// `webgpu.rs:EVAL_CHECK_BASE_INTERPRETER_WGSL` case `2u` (Get).

fn read_g0_scalar(offset: u32, back_inv_rate: u32, cycle: u32) -> u32 {
    let row = (cycle + params.domain - (back_inv_rate % params.domain)) % params.domain;
    let local_row =
        (row + params.domain - (params.group0_chunk_base % params.domain)) % params.domain;
    return group0.data[params.group0_base + offset * params.group0_chunk_rows + local_row];
}

fn read_g1_scalar(offset: u32, back_inv_rate: u32, cycle: u32) -> u32 {
    let row = (cycle + params.domain - (back_inv_rate % params.domain)) % params.domain;
    let local_row =
        (row + params.domain - (params.group1_chunk_base % params.domain)) % params.domain;
    return group1.data[params.group1_base + offset * params.group1_chunk_rows + local_row];
}

fn read_g2_scalar(offset: u32, back_inv_rate: u32, cycle: u32) -> u32 {
    let row = (cycle + params.domain - (back_inv_rate % params.domain)) % params.domain;
    let local_row =
        (row + params.domain - (params.group2_chunk_base % params.domain)) % params.domain;
    return group2.data[params.group2_base + offset * params.group2_chunk_rows + local_row];
}

// Extension variants embed the BabyBear scalar into the canonical
// `(x, 0, 0, 0)` BabyBearExt encoding, matching
// `F::ExtElem::from_subfield(F::Elem)`.
fn read_g0_ext(offset: u32, back_inv_rate: u32, cycle: u32) -> vec4<u32> {
    return vec4<u32>(read_g0_scalar(offset, back_inv_rate, cycle), 0u, 0u, 0u);
}

fn read_g1_ext(offset: u32, back_inv_rate: u32, cycle: u32) -> vec4<u32> {
    return vec4<u32>(read_g1_scalar(offset, back_inv_rate, cycle), 0u, 0u, 0u);
}

fn read_g2_ext(offset: u32, back_inv_rate: u32, cycle: u32) -> vec4<u32> {
    return vec4<u32>(read_g2_scalar(offset, back_inv_rate, cycle), 0u, 0u, 0u);
}

fn read_global_scalar(arg: u32, offset: u32) -> u32 {
    if (arg == 0u) {
        return global0.data[params.global0_base + offset];
    }
    return global1.data[params.global1_base + offset];
}

fn read_global_ext(arg: u32, offset: u32) -> vec4<u32> {
    // BabyBear scalar promoted to BabyBearExt via the canonical
    // (x, 0, 0, 0) embedding. Matches `F::ExtElem::from_subfield(F::Elem)`.
    let v = read_global_scalar(arg, offset);
    return vec4<u32>(v, 0u, 0u, 0u);
}

fn load_mix_pow(mix_idx: u32) -> vec4<u32> {
    // SP3 iter 7x: indexed UBO read. `mix_idx` is the logical mix
    // power index; with `mix_pows_base` (storage-buffer-era u32 offset)
    // mapped to vec4-aligned space, `mix_pows_base / 4u` would be the
    // vec4 offset. Pre-iter-7x callers (host side) pre-bake
    // `mix_pows_base = 0` for the staged path, so this just indexes
    // by `mix_idx`. Keep the divide-by-4 here for forward compatibility
    // if a future variant uses a non-zero base.
    return mix_pows.data[(params.mix_pows_base / 4u) + mix_idx];
}

fn zerofier_inv(idx: u32) -> u32 {
    switch (idx & 3u) {
        case 0u: { return params.invs.x; }
        case 1u: { return params.invs.y; }
        case 2u: { return params.invs.z; }
        default: { return params.invs.w; }
    }
}

// The emitted body finalizes with `write_check(cycle, mix_tot[ret])` which
// scales the `mix_tot` accumulator by the per-cycle zerofier inverse and
// writes the four BabyBear components into the strided check buffer at
// the same offsets the interpreter uses.
fn write_check(cycle: u32, val: vec4<u32>) {
    let result = ext_scale(val, zerofier_inv(cycle));
    check.data[params.check_base + 0u * params.domain + cycle] = result.x;
    check.data[params.check_base + 1u * params.domain + cycle] = result.y;
    check.data[params.check_base + 2u * params.domain + cycle] = result.z;
    check.data[params.check_base + 3u * params.domain + cycle] = result.w;
}

// ----- Multi-stage scratch I/O (SP3 iter 7) -----------------------------
//
// When a DEF is too large for a single-kernel emission to fit in the
// per-dispatch budget (e.g., the rv32im production DEF at ~20k ops), the
// codegen splits it into N stages joined via a per-cycle scratch buffer.
// Each stage's WGSL reads its live-in vars from scratch at entry, runs
// its slice of ops, and writes live-out vars to scratch at exit. The
// last stage additionally calls `write_check`.
//
// `staged_scratch_params` is a separate uniform binding (binding 9)
// carrying the strides chosen by the multi-stage planner. `fp_scratch`,
// `mix_tot_scratch`, `mix_mul_scratch` are `read_write` storage bindings
// (10/11/12) sized to `(stride * domain)` u32s.
//
// These bindings are present on *all* staged-eval_check pipelines, but
// the single-kernel stages (iter 6) never reference them, so the wgpu
// validator's "binding declared but unused" rule does not fire. When the
// dispatch wiring binds these buffers, single-kernel pipelines pass an
// empty stride and a small dummy storage buffer.

struct StagedScratchParams {
    fp_stride: u32,    // u32s per tile-local cycle for fp_scratch
    mix_stride: u32,   // u32s per tile-local cycle for each of mix_tot_scratch / mix_mul_scratch
    num_stages: u32,   // 1 for single-kernel emission, N for multi-stage
    // SP3 iter 7g: cycle stride per tile (CUDA-shape dispatch). Each
    // staged eval_check call dispatches `dispatch_workgroups(tile_size,
    // num_tiles, 1)`. Threads compute `cycle = gid.y * tile_size +
    // gid.x` so all tiles run in one dispatch per stage — no per-tile
    // setBindGroup. Iter 7e/7f used `tile_base` advanced via dynamic UBO
    // offsets; iter 7g collapses that loop into `gid.y`.
    tile_size: u32,
};

// SP3 iter 7bb (2026-05-13): the storage bindings on 10/11/12 are now
// the per-cycle WORKING SET (renamed from "scratch") for fp / mix_tot
// / mix_mul. Each thread's slot N lives at
// `tile_local * stride + N * (elem_words)`. This mimics what nvcc
// implicitly does with register spill to per-thread local memory in
// CUDA — but explicit, so the WGSL compiler doesn't fight itself
// trying to keep 900+ live vec4 values in registers across a 5000-op
// straight-line body. Coalesced access at workgroup_size=32: adjacent
// threads access adjacent words for the same slot.
//
// `fp_stride` is now `plan.fp_slots * fp_elem_words` (not max-live-fp);
// `mix_stride` is `plan.mix_slots * 4` (vec4 per mix slot). Buffers
// are sized to `tile_size * stride * 4 B`.
@group(0) @binding(9) var<uniform> staged_scratch_params: StagedScratchParams;
@group(0) @binding(10) var<storage, read_write> fp_workset: ElemBuffer;
@group(0) @binding(11) var<storage, read_write> mix_tot_workset: ElemBuffer;
@group(0) @binding(12) var<storage, read_write> mix_mul_workset: ElemBuffer;

// Per-tile-local-cycle base for fp_workset in u32 indices.
fn fp_workset_base(tile_local: u32) -> u32 {
    return tile_local * staged_scratch_params.fp_stride;
}

fn mix_workset_base(tile_local: u32) -> u32 {
    return tile_local * staged_scratch_params.mix_stride;
}

// Base-field fp read/write: one u32 per slot.
fn read_fp_workset_scalar(tile_local: u32, slot: u32) -> u32 {
    return fp_workset.data[fp_workset_base(tile_local) + slot];
}

fn write_fp_workset_scalar(tile_local: u32, slot: u32, val: u32) {
    fp_workset.data[fp_workset_base(tile_local) + slot] = val;
}

// Extension fp read/write: four u32s per slot (a vec4 BabyBearExt).
fn read_fp_workset_ext(tile_local: u32, slot: u32) -> vec4<u32> {
    let base = fp_workset_base(tile_local) + slot * 4u;
    return vec4<u32>(
        fp_workset.data[base + 0u],
        fp_workset.data[base + 1u],
        fp_workset.data[base + 2u],
        fp_workset.data[base + 3u],
    );
}

fn write_fp_workset_ext(tile_local: u32, slot: u32, val: vec4<u32>) {
    let base = fp_workset_base(tile_local) + slot * 4u;
    fp_workset.data[base + 0u] = val.x;
    fp_workset.data[base + 1u] = val.y;
    fp_workset.data[base + 2u] = val.z;
    fp_workset.data[base + 3u] = val.w;
}

// Mix state is always vec4<u32> regardless of field mode.
fn read_mix_tot_workset(tile_local: u32, slot: u32) -> vec4<u32> {
    let base = mix_workset_base(tile_local) + slot * 4u;
    return vec4<u32>(
        mix_tot_workset.data[base + 0u],
        mix_tot_workset.data[base + 1u],
        mix_tot_workset.data[base + 2u],
        mix_tot_workset.data[base + 3u],
    );
}

fn write_mix_tot_workset(tile_local: u32, slot: u32, val: vec4<u32>) {
    let base = mix_workset_base(tile_local) + slot * 4u;
    mix_tot_workset.data[base + 0u] = val.x;
    mix_tot_workset.data[base + 1u] = val.y;
    mix_tot_workset.data[base + 2u] = val.z;
    mix_tot_workset.data[base + 3u] = val.w;
}

fn read_mix_mul_workset(tile_local: u32, slot: u32) -> vec4<u32> {
    let base = mix_workset_base(tile_local) + slot * 4u;
    return vec4<u32>(
        mix_mul_workset.data[base + 0u],
        mix_mul_workset.data[base + 1u],
        mix_mul_workset.data[base + 2u],
        mix_mul_workset.data[base + 3u],
    );
}

fn write_mix_mul_workset(tile_local: u32, slot: u32, val: vec4<u32>) {
    let base = mix_workset_base(tile_local) + slot * 4u;
    mix_mul_workset.data[base + 0u] = val.x;
    mix_mul_workset.data[base + 1u] = val.y;
    mix_mul_workset.data[base + 2u] = val.z;
    mix_mul_workset.data[base + 3u] = val.w;
}
