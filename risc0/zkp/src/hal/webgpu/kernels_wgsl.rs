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

//! Embedded WGSL kernel sources for the non-eval_check operations
//! (zeroize/pack/scatter/gather, prefix products, FRI fold,
//! mix/combos, NTT, batch evaluate, Poseidon2).

#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use super::{device::*, diagnostics::*, dispatch::*, eval_check::*, ops::*, resources::*};

pub(crate) const ZEROIZE_ELEM_WGSL: &str = r#"
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

struct ElemBuffer {
    data: array<u32>,
};

@group(0) @binding(0) var<storage, read_write> elems: ElemBuffer;

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = linear_global_id(gid);
    if (idx >= arrayLength(&elems.data)) {
        return;
    }

    let value = elems.data[idx];
    if (value == 0xffffffffu) {
        elems.data[idx] = 0u;
    }
}
"#;

pub(crate) const ZEROIZE_SPARSE_UPLOAD_ELEM_WGSL: &str = r#"
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    value_count: u32,
    range_count: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read_write> elems: ElemBuffer;
@group(0) @binding(1) var<storage, read> values: array<u32>;
@group(0) @binding(2) var<storage, read> ranges: array<u32>;
@group(0) @binding(3) var<uniform> params: Params;

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let range_idx = linear_global_id(gid);
    if (range_idx >= params.range_count) {
        return;
    }
    let range_base = range_idx * 2u;
    let src = ranges[range_base];
    let dst = ranges[range_base + 1u];
    var next_src = params.value_count;
    if (range_idx + 1u < params.range_count) {
        next_src = ranges[range_base + 2u];
    }
    let len = next_src - src;

    var offset = 0u;
    loop {
        if (offset >= len) {
            break;
        }
        elems.data[dst + offset] = values[src + offset];
        offset = offset + 1u;
    }
}
"#;

pub(crate) const FILL_INVALID_ELEM_WGSL: &str = r#"
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

struct ElemBuffer {
    data: array<u32>,
};

@group(0) @binding(0) var<storage, read_write> elems: ElemBuffer;

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = linear_global_id(gid);
    if (idx >= arrayLength(&elems.data)) {
        return;
    }

    elems.data[idx] = 0xffffffffu;
}
"#;

pub(crate) const PACK_COLUMN_PREFIX_ROWS_ELEM_WGSL: &str = r#"
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

struct Params {
    total_rows: u32,
    column_count: u32,
    row_count: u32,
    _pad: u32,
};

@group(0) @binding(0) var<storage, read> source: array<u32>;
@group(0) @binding(1) var<storage, read> rows: array<u32>;
@group(0) @binding(2) var<storage, read_write> packed: array<u32>;
@group(0) @binding(3) var<uniform> params: Params;

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = linear_global_id(gid);
    let total = params.column_count * params.row_count;
    if (idx >= total) {
        return;
    }

    let row_pos = idx % params.row_count;
    let col = idx / params.row_count;
    let row = rows[row_pos];
    packed[idx] = source[col * params.total_rows + row];
}
"#;

pub(crate) const PACK_COLUMN_PREFIX_ROW_GROUPS_ELEM_WGSL: &str = r#"
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

struct Params {
    total_rows: u32,
    group_count: u32,
    total_packed: u32,
    _pad: u32,
};

@group(0) @binding(0) var<storage, read> source: array<u32>;
@group(0) @binding(1) var<storage, read> rows: array<u32>;
@group(0) @binding(2) var<storage, read_write> packed: array<u32>;
@group(0) @binding(3) var<uniform> params: Params;
@group(0) @binding(4) var<storage, read> groups: array<vec4<u32>>;

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = linear_global_id(gid);
    if (idx >= params.total_packed) {
        return;
    }

    for (var group_idx = 0u; group_idx < params.group_count; group_idx = group_idx + 1u) {
        let group = groups[group_idx];
        let row_base = group.x;
        let row_count = group.y;
        let column_count = group.z;
        let packed_base = group.w;
        let group_packed = row_count * column_count;
        if (idx < packed_base || idx >= packed_base + group_packed) {
            continue;
        }

        let local = idx - packed_base;
        let row_pos = local % row_count;
        let col = local / row_count;
        let row = rows[row_base + row_pos];
        packed[idx] = source[col * params.total_rows + row];
        return;
    }
}
"#;

pub(crate) const PACK_COLUMN_SET_ROW_GROUPS_ELEM_WGSL: &str = r#"
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

struct Params {
    total_rows: u32,
    group_count: u32,
    total_packed: u32,
    _pad: u32,
};

@group(0) @binding(0) var<storage, read> source: array<u32>;
@group(0) @binding(1) var<storage, read> rows: array<u32>;
@group(0) @binding(2) var<storage, read_write> packed: array<u32>;
@group(0) @binding(3) var<uniform> params: Params;
@group(0) @binding(4) var<storage, read> groups: array<u32>;
@group(0) @binding(5) var<storage, read> columns: array<u32>;

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = linear_global_id(gid);
    if (idx >= params.total_packed) {
        return;
    }

    for (var group_idx = 0u; group_idx < params.group_count; group_idx = group_idx + 1u) {
        let spec_base = group_idx * 5u;
        let row_base = groups[spec_base + 0u];
        let row_count = groups[spec_base + 1u];
        let column_base = groups[spec_base + 2u];
        let column_count = groups[spec_base + 3u];
        let packed_base = groups[spec_base + 4u];
        let group_packed = row_count * column_count;
        if (idx < packed_base || idx >= packed_base + group_packed) {
            continue;
        }

        let local = idx - packed_base;
        let row_pos = local % row_count;
        let column_pos = local / row_count;
        let row = rows[row_base + row_pos];
        let col = columns[column_base + column_pos];
        packed[idx] = source[col * params.total_rows + row];
        return;
    }
}
"#;

pub(crate) const TRANSPOSE_ZERO_PAD_ELEM_WGSL: &str = r#"
struct Params {
    rows: u32,
    cols: u32,
    total_rows: u32,
    total_elems: u32,
};

@group(0) @binding(0) var<storage, read> src: array<u32>;
@group(0) @binding(1) var<storage, read_write> dst: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.total_elems) {
        return;
    }
    let row = idx / params.cols;
    let col = idx - row * params.cols;
    dst[col * params.total_rows + row] = src[idx];
}
"#;

pub(crate) const ELTWISE_ADD_ELEM_WGSL: &str = r#"
const BABY_BEAR_MODULUS: u32 = 2013265921u;
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

struct ElemBuffer {
    data: array<u32>,
};

@group(0) @binding(0) var<storage, read_write> out: ElemBuffer;
@group(0) @binding(1) var<storage, read> in1: ElemBuffer;
@group(0) @binding(2) var<storage, read> in2: ElemBuffer;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= BABY_BEAR_MODULUS) {
        return sum - BABY_BEAR_MODULUS;
    }
    return sum;
}

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = linear_global_id(gid);
    if (idx >= arrayLength(&out.data)) {
        return;
    }

    out.data[idx] = add(in1.data[idx], in2.data[idx]);
}
"#;

pub(crate) const ELTWISE_SUM_EXTELEM_WGSL: &str = r#"
const BABY_BEAR_MODULUS: u32 = 2013265921u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    count: u32,
    to_add: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read_write> out: ElemBuffer;
@group(0) @binding(1) var<storage, read> input: ElemBuffer;
@group(0) @binding(2) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= BABY_BEAR_MODULUS) {
        return sum - BABY_BEAR_MODULUS;
    }
    return sum;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.count) {
        return;
    }

    var sum0 = 0u;
    var sum1 = 0u;
    var sum2 = 0u;
    var sum3 = 0u;
    for (var i = 0u; i < params.to_add; i = i + 1u) {
        let base = (i * params.count + idx) * 4u;
        sum0 = add(sum0, input.data[base + 0u]);
        sum1 = add(sum1, input.data[base + 1u]);
        sum2 = add(sum2, input.data[base + 2u]);
        sum3 = add(sum3, input.data[base + 3u]);
    }

    out.data[idx + 0u * params.count] = sum0;
    out.data[idx + 1u * params.count] = sum1;
    out.data[idx + 2u * params.count] = sum2;
    out.data[idx + 3u * params.count] = sum3;
}
"#;

pub(crate) const SCATTER_ELEM_WGSL: &str = r#"
struct ElemBuffer {
    data: array<u32>,
};

struct U32Buffer {
    data: array<u32>,
};

struct Params {
    start: u32,
    count: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read_write> into: ElemBuffer;
@group(0) @binding(1) var<storage, read> offsets: U32Buffer;
@group(0) @binding(2) var<storage, read> values: ElemBuffer;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let local_idx = gid.x;
    if (local_idx >= params.count) {
        return;
    }

    let idx = params.start + local_idx;
    into.data[offsets.data[idx]] = values.data[idx];
}
"#;

pub(crate) const GATHER_SAMPLE_ELEM_WGSL: &str = r#"
struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    dst_base: u32,
    src_base: u32,
    idx: u32,
    size: u32,
    stride: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<storage, read_write> dst: ElemBuffer;
@group(0) @binding(1) var<storage, read> src: ElemBuffer;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let out_idx = gid.x;
    if (out_idx >= params.size) {
        return;
    }

    dst.data[params.dst_base + out_idx] =
        src.data[params.src_base + out_idx * params.stride + params.idx];
}
"#;

pub(crate) const PREFIX_PRODUCTS_EXTELEM_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;

struct ExtElemBuffer {
    data: array<u32>,
};

@group(0) @binding(0) var<storage, read_write> io: ExtElemBuffer;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
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

fn load_ext(idx: u32) -> vec4<u32> {
    let base = idx * 4u;
    return vec4<u32>(
        io.data[base + 0u],
        io.data[base + 1u],
        io.data[base + 2u],
        io.data[base + 3u],
    );
}

fn store_ext(idx: u32, value: vec4<u32>) {
    let base = idx * 4u;
    io.data[base + 0u] = value.x;
    io.data[base + 1u] = value.y;
    io.data[base + 2u] = value.z;
    io.data[base + 3u] = value.w;
}

@compute @workgroup_size(1)
fn main() {
    let len = arrayLength(&io.data) / 4u;
    if (len <= 1u) {
        return;
    }

    var product = load_ext(0u);
    for (var idx = 1u; idx < len; idx = idx + 1u) {
        product = ext_mul(load_ext(idx), product);
        store_ext(idx, product);
    }
}
"#;

pub(crate) const FRI_FOLD_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const MONT_ONE: u32 = 268435454u;
const FRI_FOLD: u32 = 16u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    count: u32,
    output_base: u32,
    input_base: u32,
    _pad0: u32,
    mix: vec4<u32>,
};

@group(0) @binding(0) var<storage, read_write> output: ElemBuffer;
@group(0) @binding(1) var<storage, read> input: ElemBuffer;
@group(0) @binding(2) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
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

fn ext_add(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        add(lhs.x, rhs.x),
        add(lhs.y, rhs.y),
        add(lhs.z, rhs.z),
        add(lhs.w, rhs.w),
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

fn bit_reverse_4(value: u32) -> u32 {
    return ((value & 1u) << 3u)
        | ((value & 2u) << 1u)
        | ((value & 4u) >> 1u)
        | ((value & 8u) >> 3u);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.count) {
        return;
    }

    var total = vec4<u32>(0u, 0u, 0u, 0u);
    var cur_mix = vec4<u32>(MONT_ONE, 0u, 0u, 0u);
    for (var i = 0u; i < FRI_FOLD; i = i + 1u) {
        let rev_idx = bit_reverse_4(i) * params.count + idx;
        let input_stride = params.count * FRI_FOLD;
        let factor = vec4<u32>(
            input.data[params.input_base + 0u * input_stride + rev_idx],
            input.data[params.input_base + 1u * input_stride + rev_idx],
            input.data[params.input_base + 2u * input_stride + rev_idx],
            input.data[params.input_base + 3u * input_stride + rev_idx],
        );
        total = ext_add(total, ext_mul(cur_mix, factor));
        cur_mix = ext_mul(cur_mix, params.mix);
    }

    output.data[params.output_base + 0u * params.count + idx] = total.x;
    output.data[params.output_base + 1u * params.count + idx] = total.y;
    output.data[params.output_base + 2u * params.count + idx] = total.z;
    output.data[params.output_base + 3u * params.count + idx] = total.w;
}
"#;

pub(crate) const ZK_SHIFT_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const MONT_ONE: u32 = 268435454u;
const MONT_THREE: u32 = 805306362u;
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    count: u32,
    bits: u32,
    base: u32,
    _pad0: u32,
};

@group(0) @binding(0) var<storage, read_write> io: ElemBuffer;
@group(0) @binding(1) var<uniform> params: Params;

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

fn reverse_bits32(value: u32) -> u32 {
    var v = value;
    v = ((v & 0x55555555u) << 1u) | ((v >> 1u) & 0x55555555u);
    v = ((v & 0x33333333u) << 2u) | ((v >> 2u) & 0x33333333u);
    v = ((v & 0x0f0f0f0fu) << 4u) | ((v >> 4u) & 0x0f0f0f0fu);
    v = ((v & 0x00ff00ffu) << 8u) | ((v >> 8u) & 0x00ff00ffu);
    return (v << 16u) | (v >> 16u);
}

fn pow(base: u32, exponent: u32) -> u32 {
    var x = base;
    var n = exponent;
    var total = MONT_ONE;
    while (n != 0u) {
        if ((n & 1u) == 1u) {
            total = mul(total, x);
        }
        n = n >> 1u;
        x = mul(x, x);
    }
    return total;
}

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = linear_global_id(gid);
    if (idx >= params.count) {
        return;
    }

    let mask = (1u << params.bits) - 1u;
    let pos = idx & mask;
    let rev = reverse_bits32(pos) >> (32u - params.bits);
    let elem_idx = params.base + idx;
    io.data[elem_idx] = mul(io.data[elem_idx], pow(MONT_THREE, rev));
}
"#;

pub(crate) const MIX_POLY_COEFFS_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;

struct ElemBuffer {
    data: array<u32>,
};

struct U32Buffer {
    data: array<u32>,
};

struct Params {
    input_size: u32,
    count: u32,
    output_base: u32,
    input_base: u32,
    combos_base: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    mix_start: vec4<u32>,
    mix: vec4<u32>,
};

@group(0) @binding(0) var<storage, read_write> output: ElemBuffer;
@group(0) @binding(1) var<storage, read> input: ElemBuffer;
@group(0) @binding(2) var<storage, read> combos: U32Buffer;
@group(0) @binding(3) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
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

fn ext_add(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        add(lhs.x, rhs.x),
        add(lhs.y, rhs.y),
        add(lhs.z, rhs.z),
        add(lhs.w, rhs.w),
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

fn ext_mul_elem(lhs: vec4<u32>, rhs: u32) -> vec4<u32> {
    return vec4<u32>(
        mul(lhs.x, rhs),
        mul(lhs.y, rhs),
        mul(lhs.z, rhs),
        mul(lhs.w, rhs),
    );
}

fn load_ext(elem_idx: u32) -> vec4<u32> {
    let base = params.output_base + elem_idx * 4u;
    return vec4<u32>(
        output.data[base + 0u],
        output.data[base + 1u],
        output.data[base + 2u],
        output.data[base + 3u],
    );
}

fn store_ext(elem_idx: u32, value: vec4<u32>) {
    let base = params.output_base + elem_idx * 4u;
    output.data[base + 0u] = value.x;
    output.data[base + 1u] = value.y;
    output.data[base + 2u] = value.z;
    output.data[base + 3u] = value.w;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.count) {
        return;
    }

    var cur = params.mix_start;
    for (var i = 0u; i < params.input_size; i = i + 1u) {
        let id = combos.data[params.combos_base + i];
        let out_idx = params.count * id + idx;
        let in_elem = input.data[params.input_base + params.count * i + idx];
        let next = ext_add(load_ext(out_idx), ext_mul_elem(cur, in_elem));
        store_ext(out_idx, next);
        cur = ext_mul(cur, params.mix);
    }
}
"#;

pub(crate) const COMBOS_PREPARE_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;

struct ElemBuffer {
    data: array<u32>,
};

struct U32Buffer {
    data: array<u32>,
};

struct Params {
    total_reg_coeffs: u32,
    combo_count: u32,
    cycles: u32,
    regs_count: u32,
    combos_base: u32,
    coeff_u_base: u32,
    reg_sizes_base: u32,
    reg_combo_ids_base: u32,
    mix_pows_base: u32,
    check_size: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read_write> combos: ElemBuffer;
@group(0) @binding(1) var<storage, read> coeff_u: ElemBuffer;
@group(0) @binding(2) var<storage, read> reg_sizes: U32Buffer;
@group(0) @binding(3) var<storage, read> reg_combo_ids: U32Buffer;
@group(0) @binding(4) var<storage, read> mix_pows: ElemBuffer;
@group(0) @binding(5) var<uniform> params: Params;

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

fn load_combo(elem_idx: u32) -> vec4<u32> {
    let base = params.combos_base + elem_idx * 4u;
    return vec4<u32>(
        combos.data[base + 0u],
        combos.data[base + 1u],
        combos.data[base + 2u],
        combos.data[base + 3u],
    );
}

fn store_combo(elem_idx: u32, value: vec4<u32>) {
    let base = params.combos_base + elem_idx * 4u;
    combos.data[base + 0u] = value.x;
    combos.data[base + 1u] = value.y;
    combos.data[base + 2u] = value.z;
    combos.data[base + 3u] = value.w;
}

fn load_coeff(elem_idx: u32) -> vec4<u32> {
    let base = params.coeff_u_base + elem_idx * 4u;
    return vec4<u32>(
        coeff_u.data[base + 0u],
        coeff_u.data[base + 1u],
        coeff_u.data[base + 2u],
        coeff_u.data[base + 3u],
    );
}

fn load_mix_pow(elem_idx: u32) -> vec4<u32> {
    let base = params.mix_pows_base + elem_idx * 4u;
    return vec4<u32>(
        mix_pows.data[base + 0u],
        mix_pows.data[base + 1u],
        mix_pows.data[base + 2u],
        mix_pows.data[base + 3u],
    );
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx > params.total_reg_coeffs) {
        return;
    }

    if (idx == params.total_reg_coeffs) {
        let check_idx = params.combo_count * params.cycles;
        var total = load_combo(check_idx);
        for (var i = 0u; i < params.check_size; i = i + 1u) {
            total = ext_sub(
                total,
                ext_mul(load_mix_pow(params.regs_count + i), load_coeff(params.total_reg_coeffs + i)),
            );
        }
        store_combo(check_idx, total);
        return;
    }

    var base = 0u;
    for (var reg = 0u; reg < params.regs_count; reg = reg + 1u) {
        let reg_size = reg_sizes.data[params.reg_sizes_base + reg];
        if (idx < base + reg_size) {
            let local_idx = idx - base;
            let combo_id = reg_combo_ids.data[params.reg_combo_ids_base + reg];
            let combo_idx = combo_id * params.cycles + local_idx;
            let next = ext_sub(load_combo(combo_idx), ext_mul(load_mix_pow(reg), load_coeff(idx)));
            store_combo(combo_idx, next);
            return;
        }
        base = base + reg_size;
    }
}
"#;

pub(crate) const COMBOS_DIVIDE_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;

struct ElemBuffer {
    data: array<u32>,
};

struct U32Buffer {
    data: array<u32>,
};

struct Params {
    chunk_count: u32,
    cycles: u32,
    combos_base: u32,
    pows_base: u32,
    chunk_indices_base: u32,
    chunk_offsets_base: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read_write> combos: ElemBuffer;
@group(0) @binding(1) var<storage, read> pows: ElemBuffer;
@group(0) @binding(2) var<storage, read> chunk_indices: U32Buffer;
@group(0) @binding(3) var<storage, read> chunk_offsets: U32Buffer;
@group(0) @binding(4) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
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

fn ext_add(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        add(lhs.x, rhs.x),
        add(lhs.y, rhs.y),
        add(lhs.z, rhs.z),
        add(lhs.w, rhs.w),
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

fn load_combo(elem_idx: u32) -> vec4<u32> {
    let base = params.combos_base + elem_idx * 4u;
    return vec4<u32>(
        combos.data[base + 0u],
        combos.data[base + 1u],
        combos.data[base + 2u],
        combos.data[base + 3u],
    );
}

fn store_combo(elem_idx: u32, value: vec4<u32>) {
    let base = params.combos_base + elem_idx * 4u;
    combos.data[base + 0u] = value.x;
    combos.data[base + 1u] = value.y;
    combos.data[base + 2u] = value.z;
    combos.data[base + 3u] = value.w;
}

fn load_pow(elem_idx: u32) -> vec4<u32> {
    let base = params.pows_base + elem_idx * 4u;
    return vec4<u32>(
        pows.data[base + 0u],
        pows.data[base + 1u],
        pows.data[base + 2u],
        pows.data[base + 3u],
    );
}

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let chunk = gid.x;
    if (chunk >= params.chunk_count) {
        return;
    }

    let combo_id = chunk_indices.data[params.chunk_indices_base + chunk];
    let pow_start = chunk_offsets.data[params.chunk_offsets_base + chunk];
    let pow_end = chunk_offsets.data[params.chunk_offsets_base + chunk + 1u];
    for (var pow_idx = pow_start; pow_idx < pow_end; pow_idx = pow_idx + 1u) {
        let z = load_pow(pow_idx);
        var cur = vec4<u32>(0u, 0u, 0u, 0u);
        var i = params.cycles;
        loop {
            if (i == 0u) {
                break;
            }
            i = i - 1u;
            let elem_idx = combo_id * params.cycles + i;
            let next = ext_add(ext_mul(z, cur), load_combo(elem_idx));
            store_combo(elem_idx, cur);
            cur = next;
        }
    }
}
"#;

// Parallel-scan combos_divide. The legacy COMBOS_DIVIDE_WGSL
// runs synthetic division of each combo polynomial by (x - z) as a single
// sequential `cycles`-iteration loop on ONE thread per chunk, which serializes
// ~26 s of the xgboost proof on ~11 threads. The quotient coefficient
//   b_i = sum_{j>i} a_j * z^(j-i-1)
// is an associative weighted suffix sum, so it decomposes into three
// data-parallel kernels over blocks of 256 coefficients:
//   1. scan:  per block, in-workgroup suffix scan s_t = sum_{j>=t} a_j z^(j-t)
//      (Hillis-Steele doubling with weight z^(2^step)); writes s_{t} shifted
//      by one into scratch (scratch[i] holds the in-block part of b_i) and the
//      block summary S_k = s_0 into the carries buffer.
//   2. carry: per chunk, serial scan over the (cycles/256) block summaries:
//      C_k = S_{k+1} + z^256 * C_{k+1}, C_last = 0 — the suffix value entering
//      block k from the right.
//   3. fixup: per element, b_i = scratch[i] + z^(255 - t) * C_k.
// Successive divisions of the same chunk (multiple pows) are dependent, so
// rounds are dispatched back-to-back in one compute pass; chunks whose pow
// list is shorter than the round index no-op. All arithmetic matches the
// legacy kernel exactly (same Montgomery ops), so results are bit-identical.
pub(crate) const COMBOS_DIVIDE_SCAN_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const WORKGROUP_DISPATCH_STRIDE: u32 = 65535u;

struct ElemBuffer {
    data: array<u32>,
};

struct U32Buffer {
    data: array<u32>,
};

struct Params {
    chunk_count: u32,
    cycles: u32,
    combos_base: u32,
    pows_base: u32,
    chunk_indices_base: u32,
    chunk_offsets_base: u32,
    nblocks: u32,
    round_index: u32,
};

@group(0) @binding(0) var<storage, read> combos: ElemBuffer;
@group(0) @binding(1) var<storage, read_write> scratch: ElemBuffer;
@group(0) @binding(2) var<storage, read_write> carries: ElemBuffer;
@group(0) @binding(3) var<storage, read> pows: ElemBuffer;
@group(0) @binding(4) var<storage, read> chunk_indices: U32Buffer;
@group(0) @binding(5) var<storage, read> chunk_offsets: U32Buffer;
@group(0) @binding(6) var<uniform> params: Params;

var<workgroup> sm: array<vec4<u32>, 256>;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
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

fn ext_add(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        add(lhs.x, rhs.x),
        add(lhs.y, rhs.y),
        add(lhs.z, rhs.z),
        add(lhs.w, rhs.w),
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

fn load_combo(elem_idx: u32) -> vec4<u32> {
    let base = params.combos_base + elem_idx * 4u;
    return vec4<u32>(
        combos.data[base + 0u],
        combos.data[base + 1u],
        combos.data[base + 2u],
        combos.data[base + 3u],
    );
}

fn store_scratch(elem_idx: u32, value: vec4<u32>) {
    let base = elem_idx * 4u;
    scratch.data[base + 0u] = value.x;
    scratch.data[base + 1u] = value.y;
    scratch.data[base + 2u] = value.z;
    scratch.data[base + 3u] = value.w;
}

fn store_carry(elem_idx: u32, value: vec4<u32>) {
    let base = elem_idx * 4u;
    carries.data[base + 0u] = value.x;
    carries.data[base + 1u] = value.y;
    carries.data[base + 2u] = value.z;
    carries.data[base + 3u] = value.w;
}

fn load_pow(elem_idx: u32) -> vec4<u32> {
    let base = params.pows_base + elem_idx * 4u;
    return vec4<u32>(
        pows.data[base + 0u],
        pows.data[base + 1u],
        pows.data[base + 2u],
        pows.data[base + 3u],
    );
}

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    // Barriers below require workgroup-uniform control flow, and the active
    // guards depend on storage loads, so inactive groups run the scan on
    // zeros instead of returning early and simply skip their stores.
    let group = workgroup_id.x + workgroup_id.y * WORKGROUP_DISPATCH_STRIDE;
    let in_range = group < params.chunk_count * params.nblocks;
    var chunk = 0u;
    var block = 0u;
    if (in_range) {
        chunk = group / params.nblocks;
        block = group - chunk * params.nblocks;
    }
    var is_active = false;
    var z = vec4<u32>(0u, 0u, 0u, 0u);
    var combo = 0u;
    if (in_range) {
        let pow_start = chunk_offsets.data[params.chunk_offsets_base + chunk];
        let pow_end = chunk_offsets.data[params.chunk_offsets_base + chunk + 1u];
        if (pow_start + params.round_index < pow_end) {
            is_active = true;
            z = load_pow(pow_start + params.round_index);
            combo = chunk_indices.data[params.chunk_indices_base + chunk];
        }
    }

    let t = local_id.x;
    let pos = block * 256u + t;
    var v = vec4<u32>(0u, 0u, 0u, 0u);
    if (is_active && pos < params.cycles) {
        v = load_combo(combo * params.cycles + pos);
    }
    sm[t] = v;
    workgroupBarrier();

    var w = z;
    var stride = 1u;
    loop {
        if (stride >= 256u) {
            break;
        }
        var partner = vec4<u32>(0u, 0u, 0u, 0u);
        if (t + stride < 256u) {
            partner = sm[t + stride];
        }
        workgroupBarrier();
        v = ext_add(v, ext_mul(w, partner));
        sm[t] = v;
        workgroupBarrier();
        w = ext_mul(w, w);
        stride = stride << 1u;
    }

    if (!is_active) {
        return;
    }
    let region = combo * params.cycles;
    if (t == 0u) {
        store_carry(chunk * params.nblocks + block, v);
        let last_pos = block * 256u + 255u;
        if (last_pos < params.cycles) {
            store_scratch(region + last_pos, vec4<u32>(0u, 0u, 0u, 0u));
        }
    } else if (pos - 1u < params.cycles) {
        store_scratch(region + pos - 1u, v);
    }
}
"#;

pub(crate) const COMBOS_DIVIDE_CARRY_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const WORKGROUP_DISPATCH_STRIDE: u32 = 65535u;

struct ElemBuffer {
    data: array<u32>,
};

struct U32Buffer {
    data: array<u32>,
};

struct Params {
    chunk_count: u32,
    cycles: u32,
    combos_base: u32,
    pows_base: u32,
    chunk_indices_base: u32,
    chunk_offsets_base: u32,
    nblocks: u32,
    round_index: u32,
};

@group(0) @binding(0) var<storage, read_write> carries: ElemBuffer;
@group(0) @binding(1) var<storage, read> pows: ElemBuffer;
@group(0) @binding(2) var<storage, read> chunk_offsets: U32Buffer;
@group(0) @binding(3) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
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

fn ext_add(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        add(lhs.x, rhs.x),
        add(lhs.y, rhs.y),
        add(lhs.z, rhs.z),
        add(lhs.w, rhs.w),
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

fn load_carry(elem_idx: u32) -> vec4<u32> {
    let base = elem_idx * 4u;
    return vec4<u32>(
        carries.data[base + 0u],
        carries.data[base + 1u],
        carries.data[base + 2u],
        carries.data[base + 3u],
    );
}

fn store_carry(elem_idx: u32, value: vec4<u32>) {
    let base = elem_idx * 4u;
    carries.data[base + 0u] = value.x;
    carries.data[base + 1u] = value.y;
    carries.data[base + 2u] = value.z;
    carries.data[base + 3u] = value.w;
}

fn load_pow(elem_idx: u32) -> vec4<u32> {
    let base = params.pows_base + elem_idx * 4u;
    return vec4<u32>(
        pows.data[base + 0u],
        pows.data[base + 1u],
        pows.data[base + 2u],
        pows.data[base + 3u],
    );
}

@compute @workgroup_size(1)
fn main(@builtin(workgroup_id) workgroup_id: vec3<u32>) {
    let chunk = workgroup_id.x + workgroup_id.y * WORKGROUP_DISPATCH_STRIDE;
    if (chunk >= params.chunk_count) {
        return;
    }
    let pow_start = chunk_offsets.data[params.chunk_offsets_base + chunk];
    let pow_end = chunk_offsets.data[params.chunk_offsets_base + chunk + 1u];
    if (pow_start + params.round_index >= pow_end) {
        return;
    }
    let z = load_pow(pow_start + params.round_index);

    var z_block = z;
    var squarings = 0u;
    loop {
        if (squarings >= 8u) {
            break;
        }
        z_block = ext_mul(z_block, z_block);
        squarings = squarings + 1u;
    }

    let carries_c_base = params.chunk_count * params.nblocks;
    var cur = vec4<u32>(0u, 0u, 0u, 0u);
    var k = params.nblocks;
    loop {
        if (k == 0u) {
            break;
        }
        k = k - 1u;
        store_carry(carries_c_base + chunk * params.nblocks + k, cur);
        cur = ext_add(load_carry(chunk * params.nblocks + k), ext_mul(z_block, cur));
    }
}
"#;

pub(crate) const COMBOS_DIVIDE_FIXUP_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const WORKGROUP_DISPATCH_STRIDE: u32 = 65535u;

struct ElemBuffer {
    data: array<u32>,
};

struct U32Buffer {
    data: array<u32>,
};

struct Params {
    chunk_count: u32,
    cycles: u32,
    combos_base: u32,
    pows_base: u32,
    chunk_indices_base: u32,
    chunk_offsets_base: u32,
    nblocks: u32,
    round_index: u32,
};

@group(0) @binding(0) var<storage, read_write> combos: ElemBuffer;
@group(0) @binding(1) var<storage, read> scratch: ElemBuffer;
@group(0) @binding(2) var<storage, read> carries: ElemBuffer;
@group(0) @binding(3) var<storage, read> pows: ElemBuffer;
@group(0) @binding(4) var<storage, read> chunk_indices: U32Buffer;
@group(0) @binding(5) var<storage, read> chunk_offsets: U32Buffer;
@group(0) @binding(6) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
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

fn ext_add(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        add(lhs.x, rhs.x),
        add(lhs.y, rhs.y),
        add(lhs.z, rhs.z),
        add(lhs.w, rhs.w),
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

fn load_scratch(elem_idx: u32) -> vec4<u32> {
    let base = elem_idx * 4u;
    return vec4<u32>(
        scratch.data[base + 0u],
        scratch.data[base + 1u],
        scratch.data[base + 2u],
        scratch.data[base + 3u],
    );
}

fn load_carry(elem_idx: u32) -> vec4<u32> {
    let base = elem_idx * 4u;
    return vec4<u32>(
        carries.data[base + 0u],
        carries.data[base + 1u],
        carries.data[base + 2u],
        carries.data[base + 3u],
    );
}

fn store_combo(elem_idx: u32, value: vec4<u32>) {
    let base = params.combos_base + elem_idx * 4u;
    combos.data[base + 0u] = value.x;
    combos.data[base + 1u] = value.y;
    combos.data[base + 2u] = value.z;
    combos.data[base + 3u] = value.w;
}

fn load_pow(elem_idx: u32) -> vec4<u32> {
    let base = params.pows_base + elem_idx * 4u;
    return vec4<u32>(
        pows.data[base + 0u],
        pows.data[base + 1u],
        pows.data[base + 2u],
        pows.data[base + 3u],
    );
}

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let group = workgroup_id.x + workgroup_id.y * WORKGROUP_DISPATCH_STRIDE;
    if (group >= params.chunk_count * params.nblocks) {
        return;
    }
    let chunk = group / params.nblocks;
    let block = group - chunk * params.nblocks;
    let pow_start = chunk_offsets.data[params.chunk_offsets_base + chunk];
    let pow_end = chunk_offsets.data[params.chunk_offsets_base + chunk + 1u];
    if (pow_start + params.round_index >= pow_end) {
        return;
    }
    let t = local_id.x;
    let pos = block * 256u + t;
    if (pos >= params.cycles) {
        return;
    }
    let z = load_pow(pow_start + params.round_index);
    let combo = chunk_indices.data[params.chunk_indices_base + chunk];

    let local_part = load_scratch(combo * params.cycles + pos);
    let carries_c_base = params.chunk_count * params.nblocks;
    let carry = load_carry(carries_c_base + chunk * params.nblocks + block);

    // b_i = local_part + z^(255 - t) * carry, computed without materializing
    // z^e so no Montgomery ONE constant is needed.
    var acc = carry;
    var base = z;
    var exponent = 255u - t;
    loop {
        if (exponent == 0u) {
            break;
        }
        if ((exponent & 1u) == 1u) {
            acc = ext_mul(base, acc);
        }
        base = ext_mul(base, base);
        exponent = exponent >> 1u;
    }

    store_combo(combo * params.cycles + pos, ext_add(local_part, acc));
}
"#;

pub(crate) const BATCH_BIT_REVERSE_WGSL: &str = r#"
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    count: u32,
    n_bits: u32,
    base: u32,
    _pad0: u32,
};

@group(0) @binding(0) var<storage, read_write> io: ElemBuffer;
@group(0) @binding(1) var<uniform> params: Params;

fn reverse_bits32(value: u32) -> u32 {
    var v = value;
    v = ((v & 0x55555555u) << 1u) | ((v >> 1u) & 0x55555555u);
    v = ((v & 0x33333333u) << 2u) | ((v >> 2u) & 0x33333333u);
    v = ((v & 0x0f0f0f0fu) << 4u) | ((v >> 4u) & 0x0f0f0f0fu);
    v = ((v & 0x00ff00ffu) << 8u) | ((v >> 8u) & 0x00ff00ffu);
    return (v << 16u) | (v >> 16u);
}

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let total_idx = linear_global_id(gid);
    if (total_idx >= params.count) {
        return;
    }

    let row_size = 1u << params.n_bits;
    let idx = total_idx & (row_size - 1u);
    let row = total_idx >> params.n_bits;
    let rev_idx = reverse_bits32(idx) >> (32u - params.n_bits);
    if (idx < rev_idx) {
        let idx1 = params.base + row * row_size + idx;
        let idx2 = params.base + row * row_size + rev_idx;
        let tmp = io.data[idx1];
        io.data[idx1] = io.data[idx2];
        io.data[idx2] = tmp;
    }
}
"#;

pub(crate) const BATCH_EXPAND_LOCAL_NTT_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const WORKGROUP_DISPATCH_STRIDE: u32 = 65535u;
const WORKGROUP_SIZE: u32 = 256u;
const FUSED_BITS: u32 = 10u;
const FUSED_BLOCK_SIZE: u32 = 1024u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    out_size: u32,
    in_size: u32,
    row_count: u32,
    expand_bits: u32,
    output_base: u32,
    input_base: u32,
    twiddles_base: u32,
    n_bits: u32,
    blocks_per_row: u32,
    total_blocks: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read_write> output: ElemBuffer;
@group(0) @binding(1) var<storage, read> input: ElemBuffer;
@group(0) @binding(2) var<storage, read> twiddles: ElemBuffer;
@group(0) @binding(3) var<uniform> params: Params;

var<workgroup> scratch: array<u32, 1024>;

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

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let block_linear = workgroup_id.x + workgroup_id.y * WORKGROUP_DISPATCH_STRIDE;
    if (block_linear >= params.total_blocks) {
        return;
    }

    let row = block_linear / params.blocks_per_row;
    if (row >= params.row_count) {
        return;
    }
    let block = block_linear - row * params.blocks_per_row;
    let block_offset = block * FUSED_BLOCK_SIZE;
    let active_size = min(FUSED_BLOCK_SIZE, params.out_size - block_offset);

    var i = local_id.x;
    loop {
        if (i >= FUSED_BLOCK_SIZE) {
            break;
        }
        if (i < active_size) {
            let col = block_offset + i;
            scratch[i] = input.data[
                params.input_base + row * params.in_size + (col >> params.expand_bits)
            ];
        }
        i = i + WORKGROUP_SIZE;
    }
    workgroupBarrier();

    let max_stage = min(params.n_bits, FUSED_BITS);
    var stage = params.expand_bits + 1u;
    loop {
        if (stage > max_stage) {
            break;
        }
        let s_size = 1u << (stage - 1u);
        let pair_count = active_size >> 1u;
        var pair = local_id.x;
        loop {
            if (pair >= pair_count) {
                break;
            }
            let g = pair / s_size;
            let s = pair - g * s_size;
            let idx1 = g * 2u * s_size + s;
            let idx2 = idx1 + s_size;
            let stage_base = (1u << (stage - 1u)) - 1u;
            let cur_mul = twiddles.data[params.twiddles_base + stage_base + s];
            let a = scratch[idx1];
            let b = scratch[idx2];
            let b_mul = mul(b, cur_mul);
            scratch[idx1] = add(a, b_mul);
            scratch[idx2] = sub(a, b_mul);
            pair = pair + WORKGROUP_SIZE;
        }
        workgroupBarrier();
        stage = stage + 1u;
    }

    i = local_id.x;
    loop {
        if (i >= FUSED_BLOCK_SIZE) {
            break;
        }
        if (i < active_size) {
            output.data[params.output_base + row * params.out_size + block_offset + i] = scratch[i];
        }
        i = i + WORKGROUP_SIZE;
    }
}
"#;

pub(crate) const NTT_STEP_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    n_bits: u32,
    s_bits: u32,
    row_count: u32,
    total_pairs: u32,
    io_base: u32,
    twiddles_base: u32,
    inverse: u32,
    _pad0: u32,
};

@group(0) @binding(0) var<storage, read_write> io: ElemBuffer;
@group(0) @binding(1) var<storage, read> twiddles: ElemBuffer;
@group(0) @binding(2) var<uniform> params: Params;

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

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = linear_global_id(gid);
    if (idx >= params.total_pairs) {
        return;
    }

    let row_size = 1u << params.n_bits;
    let pairs_per_row = row_size >> 1u;
    let row = idx / pairs_per_row;
    if (row >= params.row_count) {
        return;
    }
    let pair = idx - row * pairs_per_row;
    let s_size = 1u << (params.s_bits - 1u);
    let g = pair / s_size;
    let s = pair - g * s_size;
    let row_base = params.io_base + row * row_size;
    let idx1 = row_base + g * 2u * s_size + s;
    let idx2 = idx1 + s_size;
    let stage_base = (1u << (params.s_bits - 1u)) - 1u;
    let cur_mul = twiddles.data[params.twiddles_base + stage_base + s];
    let a = io.data[idx1];
    let b = io.data[idx2];

    if (params.inverse == 0u) {
        let b_mul = mul(b, cur_mul);
        io.data[idx1] = add(a, b_mul);
        io.data[idx2] = sub(a, b_mul);
    } else {
        io.data[idx1] = add(a, b);
        io.data[idx2] = mul(sub(a, b), cur_mul);
    }
}
"#;

pub(crate) const BATCH_INTERPOLATE_NTT_FROM_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    n_bits: u32,
    row_count: u32,
    total_pairs: u32,
    output_base: u32,
    input_base: u32,
    twiddles_base: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read_write> output: ElemBuffer;
@group(0) @binding(1) var<storage, read> input: ElemBuffer;
@group(0) @binding(2) var<storage, read> twiddles: ElemBuffer;
@group(0) @binding(3) var<uniform> params: Params;

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

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = linear_global_id(gid);
    if (idx >= params.total_pairs) {
        return;
    }

    let row_size = 1u << params.n_bits;
    let pairs_per_row = row_size >> 1u;
    let row = idx / pairs_per_row;
    if (row >= params.row_count) {
        return;
    }
    let pair = idx - row * pairs_per_row;
    let s_size = 1u << (params.n_bits - 1u);
    let g = pair / s_size;
    let s = pair - g * s_size;
    let input_row_base = params.input_base + row * row_size;
    let output_row_base = params.output_base + row * row_size;
    let input_idx1 = input_row_base + g * 2u * s_size + s;
    let input_idx2 = input_idx1 + s_size;
    let output_idx1 = output_row_base + g * 2u * s_size + s;
    let output_idx2 = output_idx1 + s_size;
    let stage_base = (1u << (params.n_bits - 1u)) - 1u;
    let cur_mul = twiddles.data[params.twiddles_base + stage_base + s];
    let a = input.data[input_idx1];
    let b = input.data[input_idx2];

    output.data[output_idx1] = add(a, b);
    output.data[output_idx2] = mul(sub(a, b), cur_mul);
}
"#;

pub(crate) const NTT_NORMALIZE_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    total: u32,
    base: u32,
    factor: u32,
    _pad0: u32,
};

@group(0) @binding(0) var<storage, read_write> io: ElemBuffer;
@group(0) @binding(1) var<uniform> params: Params;

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

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = linear_global_id(gid);
    if (idx >= params.total) {
        return;
    }

    let elem_idx = params.base + idx;
    io.data[elem_idx] = mul(io.data[elem_idx], params.factor);
}
"#;

pub(crate) const BATCH_EVALUATE_ANY_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const MONT_ONE: u32 = 268435454u;
struct ElemBuffer {
    data: array<u32>,
};

struct U32Buffer {
    data: array<u32>,
};

struct Params {
    deg: u32,
    eval_count: u32,
    output_base: u32,
    coeffs_base: u32,
    which_base: u32,
    xs_base: u32,
    eval_base: u32,
    poly_stride: u32,
};

@group(0) @binding(0) var<storage, read_write> output: ElemBuffer;
@group(0) @binding(1) var<storage, read> coeffs: ElemBuffer;
@group(0) @binding(2) var<storage, read> which: U32Buffer;
@group(0) @binding(3) var<storage, read> xs: ElemBuffer;
@group(0) @binding(4) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
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

fn ext_add(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        add(lhs.x, rhs.x),
        add(lhs.y, rhs.y),
        add(lhs.z, rhs.z),
        add(lhs.w, rhs.w),
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

fn ext_mul_elem(lhs: vec4<u32>, rhs: u32) -> vec4<u32> {
    return vec4<u32>(
        mul(lhs.x, rhs),
        mul(lhs.y, rhs),
        mul(lhs.z, rhs),
        mul(lhs.w, rhs),
    );
}

fn ext_pow(base: vec4<u32>, exponent: u32) -> vec4<u32> {
    var x = base;
    var n = exponent;
    var total = vec4<u32>(MONT_ONE, 0u, 0u, 0u);
    while (n != 0u) {
        if ((n & 1u) == 1u) {
            total = ext_mul(total, x);
        }
        n = n >> 1u;
        x = ext_mul(x, x);
    }
    return total;
}

fn load_ext(buffer_base: u32, elem_idx: u32) -> vec4<u32> {
    let base = buffer_base + elem_idx * 4u;
    return vec4<u32>(
        xs.data[base + 0u],
        xs.data[base + 1u],
        xs.data[base + 2u],
        xs.data[base + 3u],
    );
}

fn store_output(elem_idx: u32, value: vec4<u32>) {
    let base = params.output_base + elem_idx * 4u;
    output.data[base + 0u] = value.x;
    output.data[base + 1u] = value.y;
    output.data[base + 2u] = value.z;
    output.data[base + 3u] = value.w;
}

var<workgroup> partials: array<vec4<u32>, 256>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let local_eval_idx = workgroup_id.x;
    let lane = local_id.x;
    if (local_eval_idx >= params.eval_count) {
        return;
    }

    let eval_idx = params.eval_base + local_eval_idx;
    let poly_id = which.data[params.which_base + eval_idx];
    let cur_poly = params.coeffs_base + poly_id * params.poly_stride;
    let x = load_ext(params.xs_base, eval_idx);
    let step_x = ext_pow(x, 256u);
    var pow_x = ext_pow(x, lane);
    var total = vec4<u32>(0u, 0u, 0u, 0u);

    for (var i = lane; i < params.deg; i = i + 256u) {
        total = ext_add(total, ext_mul_elem(pow_x, coeffs.data[cur_poly + i]));
        pow_x = ext_mul(pow_x, step_x);
    }
    partials[lane] = total;
    workgroupBarrier();

    var stride = 128u;
    loop {
        if (stride == 0u) {
            break;
        }
        if (lane < stride) {
            partials[lane] = ext_add(partials[lane], partials[lane + stride]);
        }
        workgroupBarrier();
        stride = stride >> 1u;
    }
    if (lane == 0u) {
        store_output(eval_idx, partials[0u]);
    }
}
"#;

pub(crate) const BATCH_EVALUATE_ANY_SLICE_PARTIAL_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const MONT_ONE: u32 = 268435454u;
struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    deg: u32,
    chunk_count: u32,
    partials_base: u32,
    xs_base: u32,
    eval_idx: u32,
    chunk_size: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read_write> partials: ElemBuffer;
@group(0) @binding(1) var<storage, read> coeffs: ElemBuffer;
@group(0) @binding(2) var<storage, read> xs: ElemBuffer;
@group(0) @binding(3) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
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

fn ext_add(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        add(lhs.x, rhs.x),
        add(lhs.y, rhs.y),
        add(lhs.z, rhs.z),
        add(lhs.w, rhs.w),
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

fn ext_mul_elem(lhs: vec4<u32>, rhs: u32) -> vec4<u32> {
    return vec4<u32>(
        mul(lhs.x, rhs),
        mul(lhs.y, rhs),
        mul(lhs.z, rhs),
        mul(lhs.w, rhs),
    );
}

fn ext_pow(base: vec4<u32>, exponent: u32) -> vec4<u32> {
    var x = base;
    var n = exponent;
    var total = vec4<u32>(MONT_ONE, 0u, 0u, 0u);
    while (n != 0u) {
        if ((n & 1u) == 1u) {
            total = ext_mul(total, x);
        }
        n = n >> 1u;
        x = ext_mul(x, x);
    }
    return total;
}

fn load_ext(buffer_base: u32, elem_idx: u32) -> vec4<u32> {
    let base = buffer_base + elem_idx * 4u;
    return vec4<u32>(
        xs.data[base + 0u],
        xs.data[base + 1u],
        xs.data[base + 2u],
        xs.data[base + 3u],
    );
}

fn store_partial(chunk_idx: u32, value: vec4<u32>) {
    let base = params.partials_base + chunk_idx * 4u;
    partials.data[base + 0u] = value.x;
    partials.data[base + 1u] = value.y;
    partials.data[base + 2u] = value.z;
    partials.data[base + 3u] = value.w;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let chunk_idx = gid.x;
    if (chunk_idx >= params.chunk_count) {
        return;
    }

    let x = load_ext(params.xs_base, params.eval_idx);
    let start = chunk_idx * params.chunk_size;
    var end = start + params.chunk_size;
    if (end > params.deg) {
        end = params.deg;
    }
    var pow_x = ext_pow(x, start);
    var total = vec4<u32>(0u, 0u, 0u, 0u);

    for (var i = start; i < end; i = i + 1u) {
        total = ext_add(total, ext_mul_elem(pow_x, coeffs.data[i]));
        pow_x = ext_mul(pow_x, x);
    }
    store_partial(chunk_idx, total);
}
"#;

pub(crate) const BATCH_EVALUATE_ANY_PARTIAL_2D_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const MONT_ONE: u32 = 268435454u;
struct ElemBuffer {
    data: array<u32>,
};

struct U32Buffer {
    data: array<u32>,
};

struct Params {
    deg: u32,
    chunk_count: u32,
    chunk_size: u32,
    eval_count: u32,
    partials_base: u32,
    coeffs_base: u32,
    which_base: u32,
    xs_base: u32,
    poly_stride: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<storage, read_write> partials: ElemBuffer;
@group(0) @binding(1) var<storage, read> coeffs: ElemBuffer;
@group(0) @binding(2) var<storage, read> which: U32Buffer;
@group(0) @binding(3) var<storage, read> xs: ElemBuffer;
@group(0) @binding(4) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
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

fn ext_add(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        add(lhs.x, rhs.x),
        add(lhs.y, rhs.y),
        add(lhs.z, rhs.z),
        add(lhs.w, rhs.w),
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

fn ext_mul_elem(lhs: vec4<u32>, rhs: u32) -> vec4<u32> {
    return vec4<u32>(
        mul(lhs.x, rhs),
        mul(lhs.y, rhs),
        mul(lhs.z, rhs),
        mul(lhs.w, rhs),
    );
}

fn ext_pow(base: vec4<u32>, exponent: u32) -> vec4<u32> {
    var x = base;
    var n = exponent;
    var total = vec4<u32>(MONT_ONE, 0u, 0u, 0u);
    while (n != 0u) {
        if ((n & 1u) == 1u) {
            total = ext_mul(total, x);
        }
        n = n >> 1u;
        x = ext_mul(x, x);
    }
    return total;
}

fn load_ext(buffer_base: u32, elem_idx: u32) -> vec4<u32> {
    let base = buffer_base + elem_idx * 4u;
    return vec4<u32>(
        xs.data[base + 0u],
        xs.data[base + 1u],
        xs.data[base + 2u],
        xs.data[base + 3u],
    );
}

fn store_partial(eval_idx: u32, chunk_idx: u32, value: vec4<u32>) {
    let base = params.partials_base + (eval_idx * params.chunk_count + chunk_idx) * 4u;
    partials.data[base + 0u] = value.x;
    partials.data[base + 1u] = value.y;
    partials.data[base + 2u] = value.z;
    partials.data[base + 3u] = value.w;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let chunk_idx = gid.x;
    let eval_idx = gid.y;
    if (chunk_idx >= params.chunk_count || eval_idx >= params.eval_count) {
        return;
    }

    let poly_id = which.data[params.which_base + eval_idx];
    let cur_poly = params.coeffs_base + poly_id * params.poly_stride;
    let x = load_ext(params.xs_base, eval_idx);
    let start = chunk_idx * params.chunk_size;
    var end = start + params.chunk_size;
    if (end > params.deg) {
        end = params.deg;
    }
    var pow_x = ext_pow(x, start);
    var total = vec4<u32>(0u, 0u, 0u, 0u);

    for (var i = start; i < end; i = i + 1u) {
        total = ext_add(total, ext_mul_elem(pow_x, coeffs.data[cur_poly + i]));
        pow_x = ext_mul(pow_x, x);
    }
    store_partial(eval_idx, chunk_idx, total);
}
"#;

pub(crate) const BATCH_EVALUATE_ANY_PARTIAL_REDUCE_WGSL: &str = r#"
const P: u32 = 2013265921u;
struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    eval_count: u32,
    chunk_count: u32,
    partials_base: u32,
    output_base: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
};

@group(0) @binding(0) var<storage, read_write> output: ElemBuffer;
@group(0) @binding(1) var<storage, read> partials: ElemBuffer;
@group(0) @binding(2) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
}

fn ext_add(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        add(lhs.x, rhs.x),
        add(lhs.y, rhs.y),
        add(lhs.z, rhs.z),
        add(lhs.w, rhs.w),
    );
}

fn load_partial(eval_idx: u32, chunk_idx: u32) -> vec4<u32> {
    let base = params.partials_base + (eval_idx * params.chunk_count + chunk_idx) * 4u;
    return vec4<u32>(
        partials.data[base + 0u],
        partials.data[base + 1u],
        partials.data[base + 2u],
        partials.data[base + 3u],
    );
}

fn store_output(eval_idx: u32, value: vec4<u32>) {
    let base = params.output_base + eval_idx * 4u;
    output.data[base + 0u] = value.x;
    output.data[base + 1u] = value.y;
    output.data[base + 2u] = value.z;
    output.data[base + 3u] = value.w;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let eval_idx = gid.x;
    if (eval_idx >= params.eval_count) {
        return;
    }

    var total = vec4<u32>(0u, 0u, 0u, 0u);
    for (var chunk_idx = 0u; chunk_idx < params.chunk_count; chunk_idx = chunk_idx + 1u) {
        total = ext_add(total, load_partial(eval_idx, chunk_idx));
    }
    store_output(eval_idx, total);
}
"#;

pub(crate) const EVAL_CHECK_PACK_GROUP_CHUNK_WGSL: &str = r#"
struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    domain: u32,
    chunk_base: u32,
    chunk_rows: u32,
    dst_col_base: u32,
    col_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<storage, read_write> dst: ElemBuffer;
@group(0) @binding(1) var<storage, read> src_col: ElemBuffer;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let local_row = gid.x;
    let local_col = gid.y;
    if (local_row >= params.chunk_rows || local_col >= params.col_count) {
        return;
    }
    let src_row = (params.chunk_base + local_row) % params.domain;
    dst.data[params.dst_col_base + local_col * params.chunk_rows + local_row] =
        src_col.data[local_col * params.domain + src_row];
}
"#;

pub(crate) const POSEIDON2_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const MONT_TWO: u32 = 536870908u;
const MONT_FOUR: u32 = 1073741816u;
const CELLS: u32 = 24u;
const ROUNDS_HALF_FULL: u32 = 4u;
const ROUNDS_PARTIAL: u32 = 21u;
const CELLS_RATE: u32 = 16u;
const CELLS_OUT: u32 = 8u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    output_size: u32,
    input_size: u32,
    row_size: u32,
    col_size: u32,
    output_base: u32,
    input_base: u32,
    matrix_base: u32,
    _pad0: u32,
};

@group(0) @binding(0) var<storage, read> round_constants: ElemBuffer;
@group(0) @binding(1) var<storage, read> m_int_diag: ElemBuffer;
@group(0) @binding(2) var<storage, read_write> io: ElemBuffer;
@group(0) @binding(3) var<storage, read> matrix: ElemBuffer;
@group(0) @binding(4) var<uniform> params: Params;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
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

fn sbox(x: u32) -> u32 {
    let x2 = mul(x, x);
    let x4 = mul(x2, x2);
    let x6 = mul(x4, x2);
    return mul(x6, x);
}

fn add_round_constants_full(cells: ptr<function, array<u32, 24>>, round: u32) {
    for (var i = 0u; i < CELLS; i = i + 1u) {
        (*cells)[i] = add((*cells)[i], round_constants.data[round * CELLS + i]);
    }
}

fn add_round_constants_partial(cells: ptr<function, array<u32, 24>>, round: u32) {
    (*cells)[0] = add((*cells)[0], round_constants.data[round * CELLS]);
}

fn do_full_sboxes(cells: ptr<function, array<u32, 24>>) {
    for (var i = 0u; i < CELLS; i = i + 1u) {
        (*cells)[i] = sbox((*cells)[i]);
    }
}

fn do_partial_sboxes(cells: ptr<function, array<u32, 24>>) {
    (*cells)[0] = sbox((*cells)[0]);
}

fn multiply_by_m_int(cells: ptr<function, array<u32, 24>>) {
    var sum = 0u;
    for (var i = 0u; i < CELLS; i = i + 1u) {
        sum = add(sum, (*cells)[i]);
    }
    for (var i = 0u; i < CELLS; i = i + 1u) {
        (*cells)[i] = add(sum, mul(m_int_diag.data[i], (*cells)[i]));
    }
}

fn multiply_by_4x4_circulant(x: vec4<u32>) -> vec4<u32> {
    let t0 = add(x.x, x.y);
    let t1 = add(x.z, x.w);
    let t2 = add(mul(MONT_TWO, x.y), t1);
    let t3 = add(mul(MONT_TWO, x.w), t0);
    let t4 = add(mul(MONT_FOUR, t1), t3);
    let t5 = add(mul(MONT_FOUR, t0), t2);
    let t6 = add(t3, t5);
    let t7 = add(t2, t4);
    return vec4<u32>(t6, t5, t7, t4);
}

fn multiply_by_m_ext(cells: ptr<function, array<u32, 24>>) {
    var next_cells: array<u32, 24>;
    var tmp_sums: array<u32, 4>;

    for (var i = 0u; i < CELLS / 4u; i = i + 1u) {
        let base = i * 4u;
        let out = multiply_by_4x4_circulant(vec4<u32>(
            (*cells)[base + 0u],
            (*cells)[base + 1u],
            (*cells)[base + 2u],
            (*cells)[base + 3u],
        ));
        for (var j = 0u; j < 4u; j = j + 1u) {
            let value = out[j];
            tmp_sums[j] = add(tmp_sums[j], value);
            next_cells[base + j] = add(next_cells[base + j], value);
        }
    }

    for (var i = 0u; i < CELLS; i = i + 1u) {
        (*cells)[i] = add(next_cells[i], tmp_sums[i % 4u]);
    }
}

fn full_round(cells: ptr<function, array<u32, 24>>, round: u32) {
    add_round_constants_full(cells, round);
    do_full_sboxes(cells);
    multiply_by_m_ext(cells);
}

fn partial_round(cells: ptr<function, array<u32, 24>>, round: u32) {
    add_round_constants_partial(cells, round);
    do_partial_sboxes(cells);
    multiply_by_m_int(cells);
}

fn poseidon2_mix(cells: ptr<function, array<u32, 24>>) {
    var round = 0u;
    multiply_by_m_ext(cells);

    for (var i = 0u; i < ROUNDS_HALF_FULL; i = i + 1u) {
        full_round(cells, round);
        round = round + 1u;
    }
    for (var i = 0u; i < ROUNDS_PARTIAL; i = i + 1u) {
        partial_round(cells, round);
        round = round + 1u;
    }
    for (var i = 0u; i < ROUNDS_HALF_FULL; i = i + 1u) {
        full_round(cells, round);
        round = round + 1u;
    }
}

@compute @workgroup_size(256)
fn poseidon2_fold(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.output_size) {
        return;
    }

    var cells: array<u32, 24>;
    let input_base = params.input_base + idx * 2u * CELLS_OUT;
    for (var i = 0u; i < CELLS_OUT; i = i + 1u) {
        cells[i] = io.data[input_base + i];
        cells[CELLS_OUT + i] = io.data[input_base + CELLS_OUT + i];
    }

    poseidon2_mix(&cells);

    let output_base = params.output_base + idx * CELLS_OUT;
    for (var i = 0u; i < CELLS_OUT; i = i + 1u) {
        io.data[output_base + i] = cells[i];
    }
}

@compute @workgroup_size(256)
fn poseidon2_rows(@builtin(global_invocation_id) gid: vec3<u32>) {
    let row = gid.x;
    if (row >= params.row_size) {
        return;
    }

    var cells: array<u32, 24>;
    var used = 0u;
    for (var col = 0u; col < params.col_size; col = col + 1u) {
        cells[used] = matrix.data[params.matrix_base + col * params.row_size + row];
        used = used + 1u;
        if (used == CELLS_RATE) {
            poseidon2_mix(&cells);
            used = 0u;
        }
    }

    if (used != 0u || params.col_size == 0u) {
        for (var i = used; i < CELLS_RATE; i = i + 1u) {
            cells[i] = 0u;
        }
        poseidon2_mix(&cells);
    }

    let output_base = params.output_base + row * CELLS_OUT;
    for (var i = 0u; i < CELLS_OUT; i = i + 1u) {
        io.data[output_base + i] = cells[i];
    }
}
"#;
