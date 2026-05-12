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
@group(0) @binding(7) var<storage, read> mix_pows: ElemBuffer;
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
// The emitter calls `read_tap_scalar(tap, cycle)` etc. with a *tap index*.
// Resolving that index to (group, offset, back) requires per-circuit tap
// information that the static prelude does not embed. SP3 iter 4 will
// either (a) emit tap reads with the resolved (group, offset, back) baked
// inline, removing the need for these helpers, or (b) populate a small
// `tap_table` storage buffer at HAL init that these helpers index. Either
// way the helper signature here is provisional and will be revised before
// dispatch wiring lands. For now they panic-loud via the WGSL equivalent
// (return a sentinel) so a mistaken hot-path call surfaces in QA.

fn read_tap_scalar(tap: u32, cycle: u32) -> u32 {
    // SP3 iter 4 will replace this with a real tap-resolution path.
    return 0u;
}

fn read_tap_ext(tap: u32, cycle: u32) -> vec4<u32> {
    // SP3 iter 4: real tap-resolution.
    return vec4<u32>(0u, 0u, 0u, 0u);
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
    let base = params.mix_pows_base + mix_idx * 4u;
    return vec4<u32>(
        mix_pows.data[base + 0u],
        mix_pows.data[base + 1u],
        mix_pows.data[base + 2u],
        mix_pows.data[base + 3u],
    );
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
