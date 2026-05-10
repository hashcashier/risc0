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

//! Browser WebGPU implementation of the ZKP HAL.

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashSet},
    fmt::Debug,
    marker::PhantomData,
    mem,
    rc::Rc,
};

use anyhow::{anyhow, ensure, Result};
use risc0_core::field::{
    baby_bear::{BabyBear, BabyBearElem, BabyBearExtElem},
    Elem as _, ExtElem as _, RootsOfUnity,
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use super::{
    cpu::{CpuBuffer, CpuHal},
    Buffer, Hal,
};
use crate::core::{
    digest::{Digest, DIGEST_WORDS},
    hash::{poseidon2, HashSuite},
};

/// `GPUBufferUsage.MAP_READ`.
pub const WEBGPU_BUFFER_USAGE_MAP_READ: u32 = 0x0001;
/// `GPUBufferUsage.MAP_WRITE`.
pub const WEBGPU_BUFFER_USAGE_MAP_WRITE: u32 = 0x0002;
/// `GPUBufferUsage.COPY_SRC`.
pub const WEBGPU_BUFFER_USAGE_COPY_SRC: u32 = 0x0004;
/// `GPUBufferUsage.COPY_DST`.
pub const WEBGPU_BUFFER_USAGE_COPY_DST: u32 = 0x0008;
/// `GPUBufferUsage.UNIFORM`.
pub const WEBGPU_BUFFER_USAGE_UNIFORM: u32 = 0x0040;
/// `GPUBufferUsage.STORAGE`.
pub const WEBGPU_BUFFER_USAGE_STORAGE: u32 = 0x0080;
/// `GPUShaderStage.COMPUTE`.
pub const WEBGPU_SHADER_STAGE_COMPUTE: u32 = 0x0004;
/// `GPUMapMode.READ`.
pub const WEBGPU_MAP_MODE_READ: u32 = 0x0001;

const MAX_EXACT_JS_INTEGER: u64 = 1 << 53;

/// Snapshot of WebGPU HAL backend usage.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WebGpuDiagnostics {
    pub buffers_allocated: u64,
    pub bytes_allocated: u64,
    pub host_to_gpu_uploads: u64,
    pub host_to_gpu_bytes: u64,
    pub device_copies: u64,
    pub device_copy_bytes: u64,
    pub readbacks: u64,
    pub readback_bytes: u64,
    pub gpu_dispatches: u64,
    pub cpu_mirrors: u64,
    pub cpu_fallbacks: u64,
    pub cpu_only_ops: u64,
    pub ops: Vec<WebGpuOpDiagnostics>,
}

/// Per-operation WebGPU HAL backend usage.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WebGpuOpDiagnostics {
    pub name: &'static str,
    pub gpu_dispatches: u64,
    pub cpu_mirrors: u64,
    pub cpu_fallbacks: u64,
    pub cpu_only_ops: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct WebGpuOpStats {
    gpu_dispatches: u64,
    cpu_mirrors: u64,
    cpu_fallbacks: u64,
    cpu_only_ops: u64,
}

#[derive(Debug, Default)]
struct WebGpuDiagnosticsState {
    buffers_allocated: Cell<u64>,
    bytes_allocated: Cell<u64>,
    host_to_gpu_uploads: Cell<u64>,
    host_to_gpu_bytes: Cell<u64>,
    device_copies: Cell<u64>,
    device_copy_bytes: Cell<u64>,
    readbacks: Cell<u64>,
    readback_bytes: Cell<u64>,
    gpu_dispatches: Cell<u64>,
    cpu_mirrors: Cell<u64>,
    cpu_fallbacks: Cell<u64>,
    cpu_only_ops: Cell<u64>,
    ops: RefCell<BTreeMap<&'static str, WebGpuOpStats>>,
}

impl WebGpuDiagnosticsState {
    fn snapshot(&self) -> WebGpuDiagnostics {
        WebGpuDiagnostics {
            buffers_allocated: self.buffers_allocated.get(),
            bytes_allocated: self.bytes_allocated.get(),
            host_to_gpu_uploads: self.host_to_gpu_uploads.get(),
            host_to_gpu_bytes: self.host_to_gpu_bytes.get(),
            device_copies: self.device_copies.get(),
            device_copy_bytes: self.device_copy_bytes.get(),
            readbacks: self.readbacks.get(),
            readback_bytes: self.readback_bytes.get(),
            gpu_dispatches: self.gpu_dispatches.get(),
            cpu_mirrors: self.cpu_mirrors.get(),
            cpu_fallbacks: self.cpu_fallbacks.get(),
            cpu_only_ops: self.cpu_only_ops.get(),
            ops: self
                .ops
                .borrow()
                .iter()
                .map(|(&name, stats)| WebGpuOpDiagnostics {
                    name,
                    gpu_dispatches: stats.gpu_dispatches,
                    cpu_mirrors: stats.cpu_mirrors,
                    cpu_fallbacks: stats.cpu_fallbacks,
                    cpu_only_ops: stats.cpu_only_ops,
                })
                .collect(),
        }
    }

    fn reset(&self) {
        self.buffers_allocated.set(0);
        self.bytes_allocated.set(0);
        self.host_to_gpu_uploads.set(0);
        self.host_to_gpu_bytes.set(0);
        self.device_copies.set(0);
        self.device_copy_bytes.set(0);
        self.readbacks.set(0);
        self.readback_bytes.set(0);
        self.gpu_dispatches.set(0);
        self.cpu_mirrors.set(0);
        self.cpu_fallbacks.set(0);
        self.cpu_only_ops.set(0);
        self.ops.borrow_mut().clear();
    }

    fn add(cell: &Cell<u64>, value: u64) {
        cell.set(cell.get().saturating_add(value));
    }

    fn record_op(&self, name: &'static str, update: impl FnOnce(&mut WebGpuOpStats)) {
        let mut ops = self.ops.borrow_mut();
        update(ops.entry(name).or_default());
    }

    fn record_buffer_allocated(&self, byte_len: u64) {
        Self::add(&self.buffers_allocated, 1);
        Self::add(&self.bytes_allocated, byte_len);
    }

    fn record_upload(&self, byte_len: u64) {
        Self::add(&self.host_to_gpu_uploads, 1);
        Self::add(&self.host_to_gpu_bytes, byte_len);
    }

    fn record_device_copy(&self, byte_len: u64) {
        Self::add(&self.device_copies, 1);
        Self::add(&self.device_copy_bytes, byte_len);
    }

    fn record_readback(&self, byte_len: u64) {
        Self::add(&self.readbacks, 1);
        Self::add(&self.readback_bytes, byte_len);
    }

    fn record_gpu_dispatch(&self, name: &'static str) {
        Self::add(&self.gpu_dispatches, 1);
        self.record_op(name, |stats| {
            stats.gpu_dispatches = stats.gpu_dispatches.saturating_add(1);
        });
    }

    fn record_cpu_mirror(&self, name: &'static str) {
        Self::add(&self.cpu_mirrors, 1);
        self.record_op(name, |stats| {
            stats.cpu_mirrors = stats.cpu_mirrors.saturating_add(1);
        });
    }

    fn record_cpu_fallback(&self, name: &'static str) {
        Self::add(&self.cpu_fallbacks, 1);
        self.record_op(name, |stats| {
            stats.cpu_fallbacks = stats.cpu_fallbacks.saturating_add(1);
        });
    }

}

const ZEROIZE_ELEM_WGSL: &str = r#"
struct ElemBuffer {
    data: array<u32>,
};

@group(0) @binding(0) var<storage, read_write> elems: ElemBuffer;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&elems.data)) {
        return;
    }

    let value = elems.data[idx];
    if (value == 0xffffffffu) {
        elems.data[idx] = 0u;
    }
}
"#;

const ELTWISE_ADD_ELEM_WGSL: &str = r#"
const BABY_BEAR_MODULUS: u32 = 2013265921u;

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

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&out.data)) {
        return;
    }

    out.data[idx] = add(in1.data[idx], in2.data[idx]);
}
"#;

const ELTWISE_SUM_EXTELEM_WGSL: &str = r#"
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

const ELTWISE_COPY_ELEM_SLICE_WGSL: &str = r#"
struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    from_rows: u32,
    from_cols: u32,
    from_offset: u32,
    from_stride: u32,
    into_offset: u32,
    into_stride: u32,
    total: u32,
    _pad0: u32,
};

@group(0) @binding(0) var<storage, read_write> dst: ElemBuffer;
@group(0) @binding(1) var<storage, read> src: ElemBuffer;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.total) {
        return;
    }

    let row = idx / params.from_cols;
    let col = idx - row * params.from_cols;
    dst.data[params.into_offset + row * params.into_stride + col] =
        src.data[params.from_offset + row * params.from_stride + col];
}
"#;

const SCATTER_ELEM_WGSL: &str = r#"
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

const GATHER_SAMPLE_ELEM_WGSL: &str = r#"
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

const PREFIX_PRODUCTS_EXTELEM_WGSL: &str = r#"
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

const FRI_FOLD_WGSL: &str = r#"
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

const ZK_SHIFT_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const MONT_ONE: u32 = 268435454u;
const MONT_THREE: u32 = 805306362u;

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

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
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

const MIX_POLY_COEFFS_WGSL: &str = r#"
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

const BATCH_BIT_REVERSE_WGSL: &str = r#"
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

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let total_idx = gid.x;
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

const BATCH_EXPAND_WGSL: &str = r#"
struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    total: u32,
    out_size: u32,
    in_size: u32,
    expand_bits: u32,
    output_base: u32,
    input_base: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read_write> output: ElemBuffer;
@group(0) @binding(1) var<storage, read> input: ElemBuffer;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.total) {
        return;
    }

    let row = idx / params.out_size;
    let col = idx - row * params.out_size;
    output.data[params.output_base + idx] =
        input.data[params.input_base + row * params.in_size + (col >> params.expand_bits)];
}
"#;

const NTT_STEP_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const MONT_ONE: u32 = 268435454u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    n_bits: u32,
    s_bits: u32,
    row_count: u32,
    total_pairs: u32,
    io_base: u32,
    roots_base: u32,
    inverse: u32,
    _pad0: u32,
};

@group(0) @binding(0) var<storage, read_write> io: ElemBuffer;
@group(0) @binding(1) var<storage, read> roots: ElemBuffer;
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

fn pow_elem(base: u32, exponent: u32) -> u32 {
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

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
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
    let cur_mul = pow_elem(roots.data[params.roots_base + params.s_bits], s);
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

const NTT_NORMALIZE_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;

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

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.total) {
        return;
    }

    let elem_idx = params.base + idx;
    io.data[elem_idx] = mul(io.data[elem_idx], params.factor);
}
"#;

const BATCH_EVALUATE_ANY_WGSL: &str = r#"
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
    _pad0: u32,
    _pad1: u32,
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

@compute @workgroup_size(256)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
) {
    let eval_idx = gid.x;
    if (eval_idx >= params.eval_count) {
        return;
    }

    let poly_id = which.data[params.which_base + eval_idx];
    let cur_poly = params.coeffs_base + poly_id * params.deg;
    let x = load_ext(params.xs_base, eval_idx);
    var pow_x = vec4<u32>(MONT_ONE, 0u, 0u, 0u);
    var total = vec4<u32>(0u, 0u, 0u, 0u);

    for (var i = 0u; i < params.deg; i = i + 1u) {
        total = ext_add(total, ext_mul_elem(pow_x, coeffs.data[cur_poly + i]));
        pow_x = ext_mul(pow_x, x);
    }
    store_output(eval_idx, total);
}
"#;

const POSEIDON2_WGSL: &str = r#"
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

#[derive(Clone)]
struct WebGpuPoseidon2Hash {
    round_constants: WebGpuBuffer<BabyBearElem>,
    m_int_diag: WebGpuBuffer<BabyBearElem>,
    fold_layout: web_sys::GpuBindGroupLayout,
    fold_kernel: WebGpuKernel,
    rows_layout: web_sys::GpuBindGroupLayout,
    rows_kernel: WebGpuKernel,
}

impl WebGpuPoseidon2Hash {
    fn new(hal: &WebGpuHal) -> Result<Self> {
        let round_constants =
            hal.copy_from_elem("webgpu_poseidon2_round_constants", poseidon2::ROUND_CONSTANTS);
        let m_int_diag = hal.copy_from_elem("webgpu_poseidon2_m_int_diag", poseidon2::M_INT_DIAG_HZN);

        let fold_layout = hal.create_bind_group_layout(
            "webgpu_poseidon2_fold_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let fold_kernel = hal.create_compute_kernel(
            "webgpu_poseidon2_fold",
            POSEIDON2_WGSL,
            "poseidon2_fold",
            &[fold_layout.clone()],
        )?;

        let rows_layout = hal.create_bind_group_layout(
            "webgpu_poseidon2_rows_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let rows_kernel = hal.create_compute_kernel(
            "webgpu_poseidon2_rows",
            POSEIDON2_WGSL,
            "poseidon2_rows",
            &[rows_layout.clone()],
        )?;

        Ok(Self {
            round_constants,
            m_int_diag,
            fold_layout,
            fold_kernel,
            rows_layout,
            rows_kernel,
        })
    }
}

/// A compiled WebGPU compute pipeline.
#[derive(Clone)]
pub struct WebGpuKernel {
    pipeline: web_sys::GpuComputePipeline,
}

impl WebGpuKernel {
    /// Return the underlying browser `GPUComputePipeline`.
    pub fn pipeline(&self) -> &web_sys::GpuComputePipeline {
        &self.pipeline
    }
}

/// A single storage or uniform buffer binding in a WebGPU bind group layout.
#[derive(Clone, Copy)]
pub struct WebGpuBindingLayout {
    /// The WGSL binding number.
    pub binding: u32,
    /// The WebGPU buffer binding type.
    pub ty: web_sys::GpuBufferBindingType,
    /// Optional minimum binding size in bytes.
    pub min_binding_size: u64,
}

impl WebGpuBindingLayout {
    /// Create a read/write storage buffer binding layout.
    pub fn storage(binding: u32, min_binding_size: u64) -> Self {
        Self {
            binding,
            ty: web_sys::GpuBufferBindingType::Storage,
            min_binding_size,
        }
    }

    /// Create a read-only storage buffer binding layout.
    pub fn read_only_storage(binding: u32, min_binding_size: u64) -> Self {
        Self {
            binding,
            ty: web_sys::GpuBufferBindingType::ReadOnlyStorage,
            min_binding_size,
        }
    }

    /// Create a uniform buffer binding layout.
    pub fn uniform(binding: u32, min_binding_size: u64) -> Self {
        Self {
            binding,
            ty: web_sys::GpuBufferBindingType::Uniform,
            min_binding_size,
        }
    }
}

/// A concrete buffer binding used to create a WebGPU bind group.
pub struct WebGpuBufferBinding<'a> {
    /// The WGSL binding number.
    pub binding: u32,
    /// The WebGPU buffer bound at this index.
    pub buffer: &'a web_sys::GpuBuffer,
    /// Byte offset into the buffer.
    pub offset: u64,
    /// Optional byte size for the binding.
    pub size: Option<u64>,
}

impl<'a> WebGpuBufferBinding<'a> {
    /// Bind a full buffer at `binding`.
    pub fn new(binding: u32, buffer: &'a web_sys::GpuBuffer) -> Self {
        Self {
            binding,
            buffer,
            offset: 0,
            size: None,
        }
    }
}

/// A browser WebGPU buffer with a CPU shadow for the existing synchronous HAL API.
#[derive(Clone)]
pub struct WebGpuBuffer<T> {
    cpu: CpuBuffer<T>,
    gpu: Option<web_sys::GpuBuffer>,
    elem_offset: usize,
    cpu_dirty: Rc<Cell<bool>>,
    marker: PhantomData<T>,
}

impl<T> WebGpuBuffer<T> {
    fn new(cpu: CpuBuffer<T>, gpu: Option<web_sys::GpuBuffer>, cpu_dirty: Rc<Cell<bool>>) -> Self {
        Self {
            cpu,
            gpu,
            elem_offset: 0,
            cpu_dirty,
            marker: PhantomData,
        }
    }

    fn cpu(&self) -> &CpuBuffer<T> {
        &self.cpu
    }

    fn mark_cpu_dirty(&self) {
        self.cpu_dirty.set(true);
    }

    /// Return the underlying browser `GPUBuffer`, when the allocation is non-empty.
    pub fn raw_buffer(&self) -> Option<&web_sys::GpuBuffer> {
        self.gpu.as_ref()
    }

    /// Byte offset of this buffer view into the underlying browser `GPUBuffer`.
    pub fn byte_offset(&self) -> u64 {
        (self.elem_offset * mem::size_of::<T>()) as u64
    }

    /// Upload the CPU shadow to the browser `GPUBuffer` if it has changed.
    pub fn sync_cpu_to_gpu(&self, hal: &WebGpuHal) -> Result<()>
    where
        T: bytemuck::NoUninit + Clone,
    {
        if !self.cpu_dirty.get() {
            return Ok(());
        }

        if let Some(gpu) = self.raw_buffer() {
            let cpu = self.cpu.to_vec();
            hal.write_buffer(gpu, 0, bytemuck::cast_slice(&cpu))?;
        }

        self.cpu_dirty.set(false);
        Ok(())
    }
}

impl<T: Clone> super::Buffer<T> for WebGpuBuffer<T> {
    fn name(&self) -> &'static str {
        self.cpu.name()
    }

    fn size(&self) -> usize {
        self.cpu.size()
    }

    fn slice(&self, offset: usize, size: usize) -> Self {
        let cpu = self.cpu.slice(offset, size);
        Self {
            cpu,
            gpu: self.gpu.clone(),
            elem_offset: self.elem_offset + offset,
            cpu_dirty: self.cpu_dirty.clone(),
            marker: PhantomData,
        }
    }

    fn get_at(&self, idx: usize) -> T {
        self.cpu.get_at(idx)
    }

    fn view<F: FnOnce(&[T])>(&self, f: F) {
        self.cpu.view(f);
    }

    fn view_mut<F: FnOnce(&mut [T])>(&self, f: F) {
        self.cpu.view_mut(f);
        self.mark_cpu_dirty();
    }

    fn to_vec(&self) -> Vec<T> {
        self.cpu.to_vec()
    }
}

/// A browser WebGPU HAL.
///
/// The type owns a real `GPUDevice` and is the integration point for WGSL
/// kernels. Buffers own browser `GPUBuffer` storage plus a CPU shadow so the
/// current synchronous prover interfaces can remain correct while operations
/// are moved to WebGPU incrementally.
pub struct WebGpuHal {
    pub device: web_sys::GpuDevice,
    pub queue: web_sys::GpuQueue,
    cpu: CpuHal<BabyBear>,
    poseidon2: Option<WebGpuPoseidon2Hash>,
    diagnostics: WebGpuDiagnosticsState,
}

impl WebGpuHal {
    /// Request a browser WebGPU device and construct a HAL with the given hash suite.
    pub async fn new(hash_suite: HashSuite<BabyBear>) -> Result<Self> {
        let device = request_device().await?;
        Ok(Self::from_device(device, hash_suite))
    }

    /// Construct a HAL from a browser `GPUDevice` supplied by the crate consumer.
    pub fn from_device(device: web_sys::GpuDevice, hash_suite: HashSuite<BabyBear>) -> Self {
        let use_poseidon2 = hash_suite.name == "poseidon2";
        let queue = device.queue();
        let mut hal = Self {
            device,
            queue,
            cpu: CpuHal::new(hash_suite),
            poseidon2: None,
            diagnostics: WebGpuDiagnosticsState::default(),
        };
        if use_poseidon2 {
            hal.poseidon2 = Some(
                WebGpuPoseidon2Hash::new(&hal)
                    .unwrap_or_else(|err| panic!("failed to initialize WebGPU Poseidon2: {err}")),
            );
        }
        hal
    }

    /// Return a snapshot of backend usage since construction or the last reset.
    pub fn diagnostics(&self) -> WebGpuDiagnostics {
        self.diagnostics.snapshot()
    }

    /// Reset backend usage diagnostics.
    pub fn reset_diagnostics(&self) {
        self.diagnostics.reset();
    }

    fn record_gpu_result_with_cpu_mirror(&self, name: &'static str, gpu_used: bool) {
        if gpu_used {
            self.diagnostics.record_gpu_dispatch(name);
            self.diagnostics.record_cpu_mirror(name);
        } else {
            self.diagnostics.record_cpu_fallback(name);
        }
    }

    fn alloc_shadowed_buffer<T>(&self, name: &'static str, cpu: CpuBuffer<T>) -> WebGpuBuffer<T>
    where
        T: Clone,
    {
        let byte_len = byte_len_for::<T>(cpu.size());
        let gpu = if byte_len == 0 {
            None
        } else {
            Some(
                self.create_storage_buffer(name, byte_len)
                    .unwrap_or_else(|err| panic!("failed to allocate WebGPU buffer {name}: {err}")),
            )
        };
        WebGpuBuffer::new(cpu, gpu, Rc::new(Cell::new(true)))
    }

    fn copy_shadowed_buffer<T>(
        &self,
        name: &'static str,
        cpu: CpuBuffer<T>,
        slice: &[T],
    ) -> WebGpuBuffer<T>
    where
        T: bytemuck::NoUninit + Clone,
    {
        let buffer = self.alloc_shadowed_buffer(name, cpu);
        if let Some(gpu) = buffer.raw_buffer() {
            self.write_buffer(gpu, 0, bytemuck::cast_slice(slice))
                .unwrap_or_else(|err| panic!("failed to upload WebGPU buffer {name}: {err}"));
        }
        buffer.cpu_dirty.set(false);
        buffer
    }

    /// Create a raw WebGPU buffer with the requested usage flags.
    pub fn create_buffer(
        &self,
        label: &'static str,
        byte_len: u64,
        usage: u32,
    ) -> Result<web_sys::GpuBuffer> {
        let desc = web_sys::GpuBufferDescriptor::new(byte_len_as_f64(byte_len)?, usage);
        desc.set_label(label);
        let buffer = self.device.create_buffer(&desc).map_err(js_error)?;
        self.diagnostics.record_buffer_allocated(byte_len);
        Ok(buffer)
    }

    /// Create a storage buffer suitable for compute kernels and host uploads.
    pub fn create_storage_buffer(
        &self,
        label: &'static str,
        byte_len: u64,
    ) -> Result<web_sys::GpuBuffer> {
        self.create_buffer(
            label,
            byte_len,
            WEBGPU_BUFFER_USAGE_STORAGE
                | WEBGPU_BUFFER_USAGE_COPY_DST
                | WEBGPU_BUFFER_USAGE_COPY_SRC,
        )
    }

    /// Create a uniform buffer suitable for compute kernel parameters.
    pub fn create_uniform_buffer(
        &self,
        label: &'static str,
        bytes: &[u8],
    ) -> Result<web_sys::GpuBuffer> {
        let buffer = self.create_buffer(
            label,
            bytes.len() as u64,
            WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;
        self.write_buffer(&buffer, 0, bytes)?;
        Ok(buffer)
    }

    /// Upload raw bytes to a WebGPU buffer.
    pub fn write_buffer(
        &self,
        buffer: &web_sys::GpuBuffer,
        byte_offset: u64,
        bytes: &[u8],
    ) -> Result<()> {
        self.queue
            .write_buffer_with_f64_and_u8_slice(buffer, byte_offset_as_f64(byte_offset)?, bytes)
            .map_err(js_error)?;
        self.diagnostics.record_upload(bytes.len() as u64);
        Ok(())
    }

    /// Create a bind group layout for compute kernels.
    pub fn create_bind_group_layout(
        &self,
        label: &'static str,
        entries: &[WebGpuBindingLayout],
    ) -> Result<web_sys::GpuBindGroupLayout> {
        let layout_entries = js_sys::Array::new();
        for entry in entries {
            let buffer = web_sys::GpuBufferBindingLayout::new();
            buffer.set_type(entry.ty);
            if entry.min_binding_size != 0 {
                buffer.set_min_binding_size(byte_len_as_f64(entry.min_binding_size)?);
            }

            let layout_entry =
                web_sys::GpuBindGroupLayoutEntry::new(entry.binding, WEBGPU_SHADER_STAGE_COMPUTE);
            layout_entry.set_buffer(&buffer);
            layout_entries.push(layout_entry.as_ref());
        }

        let desc = web_sys::GpuBindGroupLayoutDescriptor::new(layout_entries.as_ref());
        desc.set_label(label);
        self.device
            .create_bind_group_layout(&desc)
            .map_err(js_error)
    }

    /// Create a bind group from concrete WebGPU buffers.
    pub fn create_bind_group(
        &self,
        label: &'static str,
        layout: &web_sys::GpuBindGroupLayout,
        entries: &[WebGpuBufferBinding<'_>],
    ) -> Result<web_sys::GpuBindGroup> {
        let bind_entries = js_sys::Array::new();
        for entry in entries {
            let binding = web_sys::GpuBufferBinding::new(entry.buffer);
            if entry.offset != 0 {
                binding.set_offset(byte_offset_as_f64(entry.offset)?);
            }
            if let Some(size) = entry.size {
                binding.set_size(byte_len_as_f64(size)?);
            }

            let resource = JsValue::from(binding);
            let bind_entry = web_sys::GpuBindGroupEntry::new(entry.binding, &resource);
            bind_entries.push(bind_entry.as_ref());
        }

        let desc = web_sys::GpuBindGroupDescriptor::new(bind_entries.as_ref(), layout);
        desc.set_label(label);
        Ok(self.device.create_bind_group(&desc))
    }

    /// Compile a WGSL compute kernel with explicit bind group layouts.
    pub fn create_compute_kernel(
        &self,
        label: &'static str,
        wgsl: &str,
        entry_point: &str,
        bind_group_layouts: &[web_sys::GpuBindGroupLayout],
    ) -> Result<WebGpuKernel> {
        let shader_desc = web_sys::GpuShaderModuleDescriptor::new(wgsl);
        shader_desc.set_label(label);
        let shader = self.device.create_shader_module(&shader_desc);

        let stage = web_sys::GpuProgrammableStage::new(&shader);
        stage.set_entry_point(entry_point);

        let layouts = js_sys::Array::new();
        for layout in bind_group_layouts {
            layouts.push(layout.as_ref());
        }
        let layout_desc = web_sys::GpuPipelineLayoutDescriptor::new(layouts.as_ref());
        let pipeline_layout = self.device.create_pipeline_layout(&layout_desc);
        let pipeline_layout = JsValue::from(pipeline_layout);

        let pipeline_desc = web_sys::GpuComputePipelineDescriptor::new(&pipeline_layout, &stage);
        pipeline_desc.set_label(label);

        Ok(WebGpuKernel {
            pipeline: self.device.create_compute_pipeline(&pipeline_desc),
        })
    }

    /// Dispatch a compute kernel once and submit the command buffer.
    pub fn dispatch_compute(
        &self,
        kernel: &WebGpuKernel,
        bind_group: &web_sys::GpuBindGroup,
        workgroups_x: u32,
        workgroups_y: u32,
        workgroups_z: u32,
    ) {
        let encoder = self.device.create_command_encoder();
        let pass = encoder.begin_compute_pass();
        pass.set_pipeline(&kernel.pipeline);
        pass.set_bind_group(0, Some(bind_group));
        pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
            workgroups_x,
            workgroups_y,
            workgroups_z,
        );
        pass.end();
        self.submit(encoder.finish());
    }

    /// Submit a finished command buffer to the WebGPU queue.
    pub fn submit(&self, command_buffer: web_sys::GpuCommandBuffer) {
        let commands = js_sys::Array::new();
        commands.push(command_buffer.as_ref());
        self.queue.submit(commands.as_ref());
    }

    /// Clear a byte range in a WebGPU buffer.
    pub fn clear_gpu_buffer(
        &self,
        buffer: &web_sys::GpuBuffer,
        byte_offset: u64,
        byte_len: u64,
    ) -> Result<()> {
        let encoder = self.device.create_command_encoder();
        encoder.clear_buffer_with_f64_and_f64(
            buffer,
            byte_offset_as_f64(byte_offset)?,
            byte_len_as_f64(byte_len)?,
        );
        self.submit(encoder.finish());
        Ok(())
    }

    /// Copy a byte range between WebGPU buffers.
    pub fn copy_gpu_buffer(
        &self,
        source: &web_sys::GpuBuffer,
        source_offset: u64,
        destination: &web_sys::GpuBuffer,
        destination_offset: u64,
        byte_len: u64,
    ) -> Result<()> {
        let encoder = self.device.create_command_encoder();
        encoder
            .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                source,
                byte_offset_as_f64(source_offset)?,
                destination,
                byte_offset_as_f64(destination_offset)?,
                byte_len_as_f64(byte_len)?,
            )
            .map_err(js_error)?;
        self.submit(encoder.finish());
        self.diagnostics.record_device_copy(byte_len);
        Ok(())
    }

    /// Wait for all queued WebGPU work to complete.
    pub async fn wait_idle(&self) -> Result<()> {
        JsFuture::from(self.queue.on_submitted_work_done())
            .await
            .map_err(js_error)?;
        Ok(())
    }

    /// Copy a GPU buffer into WASM memory.
    pub async fn read_buffer(&self, source: &web_sys::GpuBuffer, byte_len: u64) -> Result<Vec<u8>> {
        let readback = self.create_buffer(
            "webgpu_readback",
            byte_len,
            WEBGPU_BUFFER_USAGE_MAP_READ | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;

        let encoder = self.device.create_command_encoder();
        let byte_len = byte_len_as_f64(byte_len)?;
        encoder
            .copy_buffer_to_buffer_with_f64_and_f64_and_f64(source, 0.0, &readback, 0.0, byte_len)
            .map_err(js_error)?;
        self.submit(encoder.finish());

        JsFuture::from(readback.map_async_with_f64_and_f64(WEBGPU_MAP_MODE_READ, 0.0, byte_len))
            .await
            .map_err(js_error)?;

        let mapped = readback
            .get_mapped_range_with_f64_and_f64(0.0, byte_len)
            .map_err(js_error)?;
        let bytes = js_sys::Uint8Array::new(&mapped).to_vec();
        readback.unmap();
        self.diagnostics.record_readback(bytes.len() as u64);
        Ok(bytes)
    }

    fn dispatch_zeroize_elem(&self, elems: &WebGpuBuffer<BabyBearElem>) -> Result<bool> {
        if elems.size() == 0 {
            return Ok(true);
        }
        if elems.byte_offset() != 0 {
            return Ok(false);
        }

        let Some(gpu) = elems.raw_buffer() else {
            return Ok(false);
        };

        elems.sync_cpu_to_gpu(self)?;

        let byte_len = byte_len_for::<BabyBearElem>(elems.size());
        let layout = self.create_bind_group_layout(
            "webgpu_zeroize_elem_layout",
            &[WebGpuBindingLayout::storage(0, byte_len)],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_zeroize_elem",
            ZEROIZE_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_zeroize_elem_bind_group",
            &layout,
            &[WebGpuBufferBinding {
                binding: 0,
                buffer: gpu,
                offset: elems.byte_offset(),
                size: Some(byte_len),
            }],
        )?;
        let workgroups = u32::try_from(elems.size())
            .expect("WebGPU zeroize element count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    fn dispatch_eltwise_add_elem(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input1: &WebGpuBuffer<BabyBearElem>,
        input2: &WebGpuBuffer<BabyBearElem>,
    ) -> Result<bool> {
        if output.size() != input1.size() || output.size() != input2.size() {
            return Ok(false);
        }
        if output.size() == 0 {
            return Ok(true);
        }
        if output.byte_offset() != 0 || input1.byte_offset() != 0 || input2.byte_offset() != 0 {
            return Ok(false);
        }

        let (Some(output_gpu), Some(input1_gpu), Some(input2_gpu)) = (
            output.raw_buffer(),
            input1.raw_buffer(),
            input2.raw_buffer(),
        ) else {
            return Ok(false);
        };

        if output_gpu == input1_gpu || output_gpu == input2_gpu {
            return Ok(false);
        }

        input1.sync_cpu_to_gpu(self)?;
        input2.sync_cpu_to_gpu(self)?;

        let byte_len = byte_len_for::<BabyBearElem>(output.size());
        let layout = self.create_bind_group_layout(
            "webgpu_eltwise_add_elem_layout",
            &[
                WebGpuBindingLayout::storage(0, byte_len),
                WebGpuBindingLayout::read_only_storage(1, byte_len),
                WebGpuBindingLayout::read_only_storage(2, byte_len),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_eltwise_add_elem",
            ELTWISE_ADD_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_eltwise_add_elem_bind_group",
            &layout,
            &[
                WebGpuBufferBinding {
                    binding: 0,
                    buffer: output_gpu,
                    offset: output.byte_offset(),
                    size: Some(byte_len),
                },
                WebGpuBufferBinding {
                    binding: 1,
                    buffer: input1_gpu,
                    offset: input1.byte_offset(),
                    size: Some(byte_len),
                },
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: input2_gpu,
                    offset: input2.byte_offset(),
                    size: Some(byte_len),
                },
            ],
        )?;
        let workgroups = u32::try_from(output.size())
            .expect("WebGPU element count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    fn dispatch_eltwise_sum_extelem(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearExtElem>,
    ) -> Result<bool> {
        if output.size() % BabyBearExtElem::EXT_SIZE != 0 {
            return Ok(false);
        }
        if output.size() == 0 {
            return Ok(true);
        }
        if output.byte_offset() != 0 || input.byte_offset() != 0 {
            return Ok(false);
        }

        let (Some(output_gpu), Some(input_gpu)) = (output.raw_buffer(), input.raw_buffer()) else {
            return Ok(false);
        };

        input.sync_cpu_to_gpu(self)?;

        let count = output.size() / BabyBearExtElem::EXT_SIZE;
        if count == 0 || input.size() % count != 0 {
            return Ok(false);
        }
        let to_add = input.size() / count;
        let params = [
            u32::try_from(count).expect("WebGPU eltwise_sum count exceeds u32"),
            u32::try_from(to_add).expect("WebGPU eltwise_sum to_add exceeds u32"),
            0,
            0,
        ];
        let params = self.create_uniform_buffer(
            "webgpu_eltwise_sum_extelem_params",
            bytemuck::cast_slice(&params),
        )?;

        let output_byte_len = byte_len_for::<BabyBearElem>(output.size());
        let input_byte_len = byte_len_for::<BabyBearExtElem>(input.size());
        let layout = self.create_bind_group_layout(
            "webgpu_eltwise_sum_extelem_layout",
            &[
                WebGpuBindingLayout::storage(0, output_byte_len),
                WebGpuBindingLayout::read_only_storage(1, input_byte_len),
                WebGpuBindingLayout::uniform(2, 16),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_eltwise_sum_extelem",
            ELTWISE_SUM_EXTELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_eltwise_sum_extelem_bind_group",
            &layout,
            &[
                WebGpuBufferBinding {
                    binding: 0,
                    buffer: output_gpu,
                    offset: output.byte_offset(),
                    size: Some(output_byte_len),
                },
                WebGpuBufferBinding {
                    binding: 1,
                    buffer: input_gpu,
                    offset: input.byte_offset(),
                    size: Some(input_byte_len),
                },
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: &params,
                    offset: 0,
                    size: Some(16),
                },
            ],
        )?;
        let workgroups = u32::try_from(count)
            .expect("WebGPU eltwise_sum count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_eltwise_copy_elem_slice(
        &self,
        into: &WebGpuBuffer<BabyBearElem>,
        from: &[BabyBearElem],
        from_rows: usize,
        from_cols: usize,
        from_offset: usize,
        from_stride: usize,
        into_offset: usize,
        into_stride: usize,
    ) -> Result<bool> {
        let Some(total) = from_rows.checked_mul(from_cols) else {
            return Ok(false);
        };
        if total == 0 {
            return Ok(false);
        }
        if into.byte_offset() != 0 {
            return Ok(false);
        }

        if !slice_region_in_bounds(from.len(), from_rows, from_cols, from_offset, from_stride)
            || !slice_region_in_bounds(into.size(), from_rows, from_cols, into_offset, into_stride)
        {
            return Ok(false);
        }

        let Some(into_gpu) = into.raw_buffer() else {
            return Ok(false);
        };
        into.sync_cpu_to_gpu(self)?;

        let from_buf = self.copy_from_elem("webgpu_eltwise_copy_elem_slice_from", from);
        let Some(from_gpu) = from_buf.raw_buffer() else {
            return Ok(false);
        };

        let params = [
            u32::try_from(from_rows).expect("WebGPU copy slice from_rows exceeds u32"),
            u32::try_from(from_cols).expect("WebGPU copy slice from_cols exceeds u32"),
            u32::try_from(from_offset).expect("WebGPU copy slice from_offset exceeds u32"),
            u32::try_from(from_stride).expect("WebGPU copy slice from_stride exceeds u32"),
            u32::try_from(into_offset).expect("WebGPU copy slice into_offset exceeds u32"),
            u32::try_from(into_stride).expect("WebGPU copy slice into_stride exceeds u32"),
            u32::try_from(total).expect("WebGPU copy slice total exceeds u32"),
            0,
        ];
        let params = self.create_uniform_buffer(
            "webgpu_eltwise_copy_elem_slice_params",
            bytemuck::cast_slice(&params),
        )?;

        let into_byte_len = byte_len_for::<BabyBearElem>(into.size());
        let from_byte_len = byte_len_for::<BabyBearElem>(from.len());
        let layout = self.create_bind_group_layout(
            "webgpu_eltwise_copy_elem_slice_layout",
            &[
                WebGpuBindingLayout::storage(0, into_byte_len),
                WebGpuBindingLayout::read_only_storage(1, from_byte_len),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_eltwise_copy_elem_slice",
            ELTWISE_COPY_ELEM_SLICE_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_eltwise_copy_elem_slice_bind_group",
            &layout,
            &[
                WebGpuBufferBinding {
                    binding: 0,
                    buffer: into_gpu,
                    offset: 0,
                    size: Some(into_byte_len),
                },
                WebGpuBufferBinding {
                    binding: 1,
                    buffer: from_gpu,
                    offset: 0,
                    size: Some(from_byte_len),
                },
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: &params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        let workgroups = u32::try_from(total)
            .expect("WebGPU copy slice total exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    fn dispatch_scatter(
        &self,
        into: &WebGpuBuffer<BabyBearElem>,
        index: &[u32],
        offsets: &[u32],
        values: &[BabyBearElem],
    ) -> Result<bool> {
        if index.len() < 2 {
            return Ok(false);
        }
        if into.byte_offset() != 0 {
            return Ok(false);
        }

        let start = index[0] as usize;
        let end = *index.last().expect("index has at least two entries") as usize;
        if end <= start {
            return Ok(false);
        }
        if end > offsets.len() || end > values.len() {
            return Ok(false);
        }
        if !index.windows(2).all(|window| window[0] <= window[1]) {
            return Ok(false);
        }

        let mut seen_offsets = HashSet::with_capacity(end - start);
        for &offset in &offsets[start..end] {
            let offset = offset as usize;
            if offset >= into.size() || !seen_offsets.insert(offset) {
                return Ok(false);
            }
        }

        let Some(into_gpu) = into.raw_buffer() else {
            return Ok(false);
        };
        into.sync_cpu_to_gpu(self)?;

        let offsets = self.copy_from_u32("webgpu_scatter_offsets", offsets);
        let values = self.copy_from_elem("webgpu_scatter_values", values);
        let (Some(offsets_gpu), Some(values_gpu)) = (offsets.raw_buffer(), values.raw_buffer())
        else {
            return Ok(false);
        };

        let count = end - start;
        let params = [
            u32::try_from(start).expect("WebGPU scatter start exceeds u32"),
            u32::try_from(count).expect("WebGPU scatter count exceeds u32"),
            0,
            0,
        ];
        let params =
            self.create_uniform_buffer("webgpu_scatter_params", bytemuck::cast_slice(&params))?;

        let into_byte_len = byte_len_for::<BabyBearElem>(into.size());
        let offsets_byte_len = byte_len_for::<u32>(offsets.size());
        let values_byte_len = byte_len_for::<BabyBearElem>(values.size());
        let layout = self.create_bind_group_layout(
            "webgpu_scatter_layout",
            &[
                WebGpuBindingLayout::storage(0, into_byte_len),
                WebGpuBindingLayout::read_only_storage(1, offsets_byte_len),
                WebGpuBindingLayout::read_only_storage(2, values_byte_len),
                WebGpuBindingLayout::uniform(3, 16),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_scatter",
            SCATTER_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_scatter_bind_group",
            &layout,
            &[
                WebGpuBufferBinding {
                    binding: 0,
                    buffer: into_gpu,
                    offset: 0,
                    size: Some(into_byte_len),
                },
                WebGpuBufferBinding {
                    binding: 1,
                    buffer: offsets_gpu,
                    offset: 0,
                    size: Some(offsets_byte_len),
                },
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: values_gpu,
                    offset: 0,
                    size: Some(values_byte_len),
                },
                WebGpuBufferBinding {
                    binding: 3,
                    buffer: &params,
                    offset: 0,
                    size: Some(16),
                },
            ],
        )?;
        let workgroups = u32::try_from(count)
            .expect("WebGPU scatter count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    fn dispatch_gather_sample(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src: &WebGpuBuffer<BabyBearElem>,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> Result<bool> {
        if size == 0 {
            return Ok(false);
        }
        if size > dst.size() || !gather_region_in_bounds(src.size(), idx, size, stride) {
            return Ok(false);
        }

        let (Some(dst_gpu), Some(src_gpu)) = (dst.raw_buffer(), src.raw_buffer()) else {
            return Ok(false);
        };
        if dst_gpu == src_gpu {
            return Ok(false);
        }

        src.sync_cpu_to_gpu(self)?;
        dst.sync_cpu_to_gpu(self)?;

        let params = [
            u32::try_from(dst.elem_offset).expect("WebGPU gather dst offset exceeds u32"),
            u32::try_from(src.elem_offset).expect("WebGPU gather src offset exceeds u32"),
            u32::try_from(idx).expect("WebGPU gather idx exceeds u32"),
            u32::try_from(size).expect("WebGPU gather size exceeds u32"),
            u32::try_from(stride).expect("WebGPU gather stride exceeds u32"),
            0,
            0,
            0,
        ];
        let params = self
            .create_uniform_buffer("webgpu_gather_sample_params", bytemuck::cast_slice(&params))?;

        let layout = self.create_bind_group_layout(
            "webgpu_gather_sample_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_gather_sample",
            GATHER_SAMPLE_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_gather_sample_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, dst_gpu),
                WebGpuBufferBinding::new(1, src_gpu),
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: &params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        let workgroups = u32::try_from(size)
            .expect("WebGPU gather size exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    fn dispatch_prefix_products(&self, io: &WebGpuBuffer<BabyBearExtElem>) -> Result<bool> {
        if io.size() == 0 {
            return Ok(true);
        }

        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };
        io.sync_cpu_to_gpu(self)?;

        let byte_len = byte_len_for::<BabyBearExtElem>(io.size());
        let layout = self.create_bind_group_layout(
            "webgpu_prefix_products_extelem_layout",
            &[WebGpuBindingLayout::storage(0, byte_len)],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_prefix_products_extelem",
            PREFIX_PRODUCTS_EXTELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_prefix_products_extelem_bind_group",
            &layout,
            &[WebGpuBufferBinding {
                binding: 0,
                buffer: io_gpu,
                offset: io.byte_offset(),
                size: Some(byte_len),
            }],
        )?;
        self.dispatch_compute(&kernel, &bind_group, 1, 1, 1);
        Ok(true)
    }

    fn dispatch_fri_fold(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
        mix: &BabyBearExtElem,
    ) -> Result<bool> {
        let count = output.size() / BabyBearExtElem::EXT_SIZE;
        if count == 0 {
            return Ok(true);
        }

        let (Some(output_gpu), Some(input_gpu)) = (output.raw_buffer(), input.raw_buffer()) else {
            return Ok(false);
        };

        let mix = mix.subelems();
        let params = [
            u32::try_from(count).expect("WebGPU fri_fold count exceeds u32"),
            u32::try_from(output.elem_offset).expect("WebGPU fri_fold output offset exceeds u32"),
            u32::try_from(input.elem_offset).expect("WebGPU fri_fold input offset exceeds u32"),
            0,
            mix[0].as_u32_montgomery(),
            mix[1].as_u32_montgomery(),
            mix[2].as_u32_montgomery(),
            mix[3].as_u32_montgomery(),
        ];
        let params =
            self.create_uniform_buffer("webgpu_fri_fold_params", bytemuck::cast_slice(&params))?;

        output.sync_cpu_to_gpu(self)?;
        input.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_fri_fold_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let kernel =
            self.create_compute_kernel("webgpu_fri_fold", FRI_FOLD_WGSL, "main", &[layout.clone()])?;
        let bind_group = self.create_bind_group(
            "webgpu_fri_fold_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, output_gpu),
                WebGpuBufferBinding::new(1, input_gpu),
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: &params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        let workgroups = u32::try_from(count)
            .expect("WebGPU fri_fold count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    fn dispatch_zk_shift(&self, io: &WebGpuBuffer<BabyBearElem>, bits: usize) -> Result<bool> {
        if io.size() == 0 {
            return Ok(true);
        }

        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };

        let params = [
            u32::try_from(io.size()).expect("WebGPU zk_shift count exceeds u32"),
            u32::try_from(bits).expect("WebGPU zk_shift bits exceeds u32"),
            u32::try_from(io.elem_offset).expect("WebGPU zk_shift offset exceeds u32"),
            0,
        ];
        let params =
            self.create_uniform_buffer("webgpu_zk_shift_params", bytemuck::cast_slice(&params))?;

        io.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_zk_shift_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::uniform(1, 16),
            ],
        )?;
        let kernel =
            self.create_compute_kernel("webgpu_zk_shift", ZK_SHIFT_WGSL, "main", &[layout.clone()])?;
        let bind_group = self.create_bind_group(
            "webgpu_zk_shift_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, io_gpu),
                WebGpuBufferBinding {
                    binding: 1,
                    buffer: &params,
                    offset: 0,
                    size: Some(16),
                },
            ],
        )?;
        let workgroups = u32::try_from(io.size())
            .expect("WebGPU zk_shift count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_mix_poly_coeffs(
        &self,
        output: &WebGpuBuffer<BabyBearExtElem>,
        mix_start: &BabyBearExtElem,
        mix: &BabyBearExtElem,
        input: &WebGpuBuffer<BabyBearElem>,
        combos: &WebGpuBuffer<u32>,
        input_size: usize,
        count: usize,
    ) -> Result<bool> {
        if count == 0 {
            return Ok(true);
        }

        let (Some(output_gpu), Some(input_gpu), Some(combos_gpu)) =
            (output.raw_buffer(), input.raw_buffer(), combos.raw_buffer())
        else {
            return Ok(false);
        };

        let Some(output_base) = output
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
        else {
            return Err(anyhow!("WebGPU mix_poly_coeffs output offset exceeds u32"));
        };
        let mix_start = mix_start.subelems();
        let mix = mix.subelems();
        let params = [
            u32::try_from(input_size).expect("WebGPU mix_poly_coeffs input_size exceeds u32"),
            u32::try_from(count).expect("WebGPU mix_poly_coeffs count exceeds u32"),
            output_base,
            u32::try_from(input.elem_offset)
                .expect("WebGPU mix_poly_coeffs input offset exceeds u32"),
            u32::try_from(combos.elem_offset)
                .expect("WebGPU mix_poly_coeffs combos offset exceeds u32"),
            0,
            0,
            0,
            mix_start[0].as_u32_montgomery(),
            mix_start[1].as_u32_montgomery(),
            mix_start[2].as_u32_montgomery(),
            mix_start[3].as_u32_montgomery(),
            mix[0].as_u32_montgomery(),
            mix[1].as_u32_montgomery(),
            mix[2].as_u32_montgomery(),
            mix[3].as_u32_montgomery(),
        ];
        let params = self.create_uniform_buffer(
            "webgpu_mix_poly_coeffs_params",
            bytemuck::cast_slice(&params),
        )?;

        output.sync_cpu_to_gpu(self)?;
        input.sync_cpu_to_gpu(self)?;
        combos.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_mix_poly_coeffs_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::uniform(3, 64),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_mix_poly_coeffs",
            MIX_POLY_COEFFS_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_mix_poly_coeffs_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, output_gpu),
                WebGpuBufferBinding::new(1, input_gpu),
                WebGpuBufferBinding::new(2, combos_gpu),
                WebGpuBufferBinding {
                    binding: 3,
                    buffer: &params,
                    offset: 0,
                    size: Some(64),
                },
            ],
        )?;
        let workgroups = u32::try_from(count)
            .expect("WebGPU mix_poly_coeffs count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    fn dispatch_batch_expand_into_evaluate_ntt(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
        count: usize,
        expand_bits: usize,
    ) -> Result<bool> {
        if output.size() == 0 {
            return Ok(true);
        }

        let (Some(output_gpu), Some(input_gpu)) = (output.raw_buffer(), input.raw_buffer()) else {
            return Ok(false);
        };
        if output_gpu == input_gpu {
            return Ok(false);
        }

        let out_size = output.size() / count;
        let in_size = input.size() / count;
        let actual_expand_bits = crate::core::log2_ceil(out_size / in_size);
        assert_eq!(output.size(), out_size * count);
        assert_eq!(input.size(), in_size * count);
        assert_eq!(out_size, in_size * (1 << actual_expand_bits));

        input.sync_cpu_to_gpu(self)?;

        let expand_params = [
            u32::try_from(output.size()).expect("WebGPU NTT expand total exceeds u32"),
            u32::try_from(out_size).expect("WebGPU NTT expand out_size exceeds u32"),
            u32::try_from(in_size).expect("WebGPU NTT expand in_size exceeds u32"),
            u32::try_from(actual_expand_bits)
                .expect("WebGPU NTT expand_bits exceeds u32"),
            u32::try_from(output.elem_offset).expect("WebGPU NTT output offset exceeds u32"),
            u32::try_from(input.elem_offset).expect("WebGPU NTT input offset exceeds u32"),
            0,
            0,
        ];
        let expand_params =
            self.create_uniform_buffer("webgpu_batch_expand_params", bytemuck::cast_slice(&expand_params))?;
        let expand_layout = self.create_bind_group_layout(
            "webgpu_batch_expand_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let expand_kernel = self.create_compute_kernel(
            "webgpu_batch_expand",
            BATCH_EXPAND_WGSL,
            "main",
            &[expand_layout.clone()],
        )?;
        let expand_bind_group = self.create_bind_group(
            "webgpu_batch_expand_bind_group",
            &expand_layout,
            &[
                WebGpuBufferBinding::new(0, output_gpu),
                WebGpuBufferBinding::new(1, input_gpu),
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: &expand_params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        let expand_workgroups = u32::try_from(output.size())
            .expect("WebGPU NTT expand total exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&expand_kernel, &expand_bind_group, expand_workgroups, 1, 1);

        let row_size = output.size() / count;
        assert_eq!(row_size * count, output.size());
        let n_bits = crate::core::log2_ceil(row_size);
        assert_eq!(row_size, 1 << n_bits);
        assert!(n_bits >= expand_bits);
        assert!(n_bits < BabyBearElem::MAX_ROU_PO2);
        if n_bits == expand_bits {
            return Ok(true);
        }

        let roots = self.copy_from_elem("webgpu_ntt_roots_fwd", BabyBearElem::ROU_FWD);
        let Some(roots_gpu) = roots.raw_buffer() else {
            return Ok(false);
        };
        let ntt_layout = self.create_bind_group_layout(
            "webgpu_ntt_step_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let ntt_kernel = self.create_compute_kernel(
            "webgpu_ntt_step",
            NTT_STEP_WGSL,
            "main",
            &[ntt_layout.clone()],
        )?;
        let pairs_per_row = row_size / 2;
        let total_pairs = pairs_per_row
            .checked_mul(count)
            .ok_or_else(|| anyhow!("WebGPU NTT total pair count overflow"))?;
        let workgroups = u32::try_from(total_pairs)
            .expect("WebGPU NTT total pairs exceeds u32")
            .div_ceil(256);
        for s_bits in 1 + expand_bits..=n_bits {
            let params = [
                u32::try_from(n_bits).expect("WebGPU NTT n_bits exceeds u32"),
                u32::try_from(s_bits).expect("WebGPU NTT s_bits exceeds u32"),
                u32::try_from(count).expect("WebGPU NTT row count exceeds u32"),
                u32::try_from(total_pairs).expect("WebGPU NTT total pairs exceeds u32"),
                u32::try_from(output.elem_offset).expect("WebGPU NTT output offset exceeds u32"),
                u32::try_from(roots.elem_offset).expect("WebGPU NTT roots offset exceeds u32"),
                0,
                0,
            ];
            let params =
                self.create_uniform_buffer("webgpu_ntt_step_params", bytemuck::cast_slice(&params))?;
            let bind_group = self.create_bind_group(
                "webgpu_ntt_step_bind_group",
                &ntt_layout,
                &[
                    WebGpuBufferBinding::new(0, output_gpu),
                    WebGpuBufferBinding::new(1, roots_gpu),
                    WebGpuBufferBinding {
                        binding: 2,
                        buffer: &params,
                        offset: 0,
                        size: Some(32),
                    },
                ],
            )?;
            self.dispatch_compute(&ntt_kernel, &bind_group, workgroups, 1, 1);
        }
        Ok(true)
    }

    fn dispatch_batch_interpolate_ntt(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        count: usize,
    ) -> Result<bool> {
        if io.size() == 0 {
            return Ok(true);
        }

        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };

        let row_size = io.size() / count;
        assert_eq!(row_size * count, io.size());
        let n_bits = crate::core::log2_ceil(row_size);
        assert_eq!(row_size, 1 << n_bits);
        assert!(n_bits < BabyBearElem::MAX_ROU_PO2);

        io.sync_cpu_to_gpu(self)?;

        if n_bits != 0 {
            let roots = self.copy_from_elem("webgpu_ntt_roots_rev", BabyBearElem::ROU_REV);
            let Some(roots_gpu) = roots.raw_buffer() else {
                return Ok(false);
            };
            let ntt_layout = self.create_bind_group_layout(
                "webgpu_ntt_step_layout",
                &[
                    WebGpuBindingLayout::storage(0, 0),
                    WebGpuBindingLayout::read_only_storage(1, 0),
                    WebGpuBindingLayout::uniform(2, 32),
                ],
            )?;
            let ntt_kernel = self.create_compute_kernel(
                "webgpu_ntt_step",
                NTT_STEP_WGSL,
                "main",
                &[ntt_layout.clone()],
            )?;
            let pairs_per_row = row_size / 2;
            let total_pairs = pairs_per_row
                .checked_mul(count)
                .ok_or_else(|| anyhow!("WebGPU inverse NTT total pair count overflow"))?;
            let workgroups = u32::try_from(total_pairs)
                .expect("WebGPU inverse NTT total pairs exceeds u32")
                .div_ceil(256);
            for s_bits in (1..=n_bits).rev() {
                let params = [
                    u32::try_from(n_bits).expect("WebGPU inverse NTT n_bits exceeds u32"),
                    u32::try_from(s_bits).expect("WebGPU inverse NTT s_bits exceeds u32"),
                    u32::try_from(count).expect("WebGPU inverse NTT row count exceeds u32"),
                    u32::try_from(total_pairs)
                        .expect("WebGPU inverse NTT total pairs exceeds u32"),
                    u32::try_from(io.elem_offset).expect("WebGPU inverse NTT io offset exceeds u32"),
                    u32::try_from(roots.elem_offset)
                        .expect("WebGPU inverse NTT roots offset exceeds u32"),
                    1,
                    0,
                ];
                let params = self
                    .create_uniform_buffer("webgpu_ntt_step_params", bytemuck::cast_slice(&params))?;
                let bind_group = self.create_bind_group(
                    "webgpu_ntt_step_bind_group",
                    &ntt_layout,
                    &[
                        WebGpuBufferBinding::new(0, io_gpu),
                        WebGpuBufferBinding::new(1, roots_gpu),
                        WebGpuBufferBinding {
                            binding: 2,
                            buffer: &params,
                            offset: 0,
                            size: Some(32),
                        },
                    ],
                )?;
                self.dispatch_compute(&ntt_kernel, &bind_group, workgroups, 1, 1);
            }
        }

        let norm = BabyBearElem::new(row_size as u32).inv().as_u32_montgomery();
        let params = [
            u32::try_from(io.size()).expect("WebGPU inverse NTT size exceeds u32"),
            u32::try_from(io.elem_offset).expect("WebGPU inverse NTT io offset exceeds u32"),
            norm,
            0,
        ];
        let params =
            self.create_uniform_buffer("webgpu_ntt_normalize_params", bytemuck::cast_slice(&params))?;
        let layout = self.create_bind_group_layout(
            "webgpu_ntt_normalize_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::uniform(1, 16),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_ntt_normalize",
            NTT_NORMALIZE_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_ntt_normalize_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, io_gpu),
                WebGpuBufferBinding {
                    binding: 1,
                    buffer: &params,
                    offset: 0,
                    size: Some(16),
                },
            ],
        )?;
        let workgroups = u32::try_from(io.size())
            .expect("WebGPU inverse NTT size exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    fn dispatch_batch_bit_reverse(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        bits: usize,
    ) -> Result<bool> {
        if io.size() == 0 {
            return Ok(true);
        }

        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };

        let params = [
            u32::try_from(io.size()).expect("WebGPU bit_reverse count exceeds u32"),
            u32::try_from(bits).expect("WebGPU bit_reverse bits exceeds u32"),
            u32::try_from(io.elem_offset).expect("WebGPU bit_reverse offset exceeds u32"),
            0,
        ];
        let params = self.create_uniform_buffer(
            "webgpu_batch_bit_reverse_params",
            bytemuck::cast_slice(&params),
        )?;

        io.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_batch_bit_reverse_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::uniform(1, 16),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_batch_bit_reverse",
            BATCH_BIT_REVERSE_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_batch_bit_reverse_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, io_gpu),
                WebGpuBufferBinding {
                    binding: 1,
                    buffer: &params,
                    offset: 0,
                    size: Some(16),
                },
            ],
        )?;
        let workgroups = u32::try_from(io.size())
            .expect("WebGPU bit_reverse count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    fn dispatch_batch_evaluate_any(
        &self,
        coeffs: &WebGpuBuffer<BabyBearElem>,
        which: &WebGpuBuffer<u32>,
        xs: &WebGpuBuffer<BabyBearExtElem>,
        out: &WebGpuBuffer<BabyBearExtElem>,
        deg: usize,
    ) -> Result<bool> {
        let eval_count = which.size();
        if eval_count == 0 {
            return Ok(true);
        }

        let (Some(out_gpu), Some(coeffs_gpu), Some(which_gpu), Some(xs_gpu)) = (
            out.raw_buffer(),
            coeffs.raw_buffer(),
            which.raw_buffer(),
            xs.raw_buffer(),
        ) else {
            return Ok(false);
        };

        let Some(output_base) = out
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
        else {
            return Err(anyhow!(
                "WebGPU batch_evaluate_any output offset exceeds u32"
            ));
        };
        let Some(xs_base) = xs
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
        else {
            return Err(anyhow!("WebGPU batch_evaluate_any xs offset exceeds u32"));
        };
        let params = [
            u32::try_from(deg).expect("WebGPU batch_evaluate_any degree exceeds u32"),
            u32::try_from(eval_count).expect("WebGPU batch_evaluate_any eval_count exceeds u32"),
            output_base,
            u32::try_from(coeffs.elem_offset)
                .expect("WebGPU batch_evaluate_any coeffs offset exceeds u32"),
            u32::try_from(which.elem_offset)
                .expect("WebGPU batch_evaluate_any which offset exceeds u32"),
            xs_base,
            0,
            0,
        ];
        let params = self.create_uniform_buffer(
            "webgpu_batch_evaluate_any_params",
            bytemuck::cast_slice(&params),
        )?;

        out.sync_cpu_to_gpu(self)?;
        coeffs.sync_cpu_to_gpu(self)?;
        which.sync_cpu_to_gpu(self)?;
        xs.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_batch_evaluate_any_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_batch_evaluate_any",
            BATCH_EVALUATE_ANY_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_batch_evaluate_any_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, out_gpu),
                WebGpuBufferBinding::new(1, coeffs_gpu),
                WebGpuBufferBinding::new(2, which_gpu),
                WebGpuBufferBinding::new(3, xs_gpu),
                WebGpuBufferBinding {
                    binding: 4,
                    buffer: &params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        let workgroups = u32::try_from(eval_count)
            .expect("WebGPU batch_evaluate_any eval_count exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    fn dispatch_poseidon2_hash_fold(
        &self,
        io: &WebGpuBuffer<Digest>,
        input_size: usize,
        output_size: usize,
    ) -> Result<bool> {
        let Some(hash) = self.poseidon2.as_ref() else {
            return Ok(false);
        };
        if output_size == 0 {
            return Ok(true);
        }

        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };
        let (Some(round_constants_gpu), Some(m_int_diag_gpu)) = (
            hash.round_constants.raw_buffer(),
            hash.m_int_diag.raw_buffer(),
        ) else {
            return Ok(false);
        };

        let output_base = digest_word_offset(io.elem_offset + output_size)?;
        let input_base = digest_word_offset(io.elem_offset + input_size)?;
        let params = [
            u32::try_from(output_size).expect("WebGPU hash_fold output size exceeds u32"),
            u32::try_from(input_size).expect("WebGPU hash_fold input size exceeds u32"),
            0,
            0,
            output_base,
            input_base,
            0,
            0,
        ];
        let params =
            self.create_uniform_buffer("webgpu_poseidon2_fold_params", bytemuck::cast_slice(&params))?;

        io.sync_cpu_to_gpu(self)?;

        let bind_group = self.create_bind_group(
            "webgpu_poseidon2_fold_bind_group",
            &hash.fold_layout,
            &[
                WebGpuBufferBinding::new(0, round_constants_gpu),
                WebGpuBufferBinding::new(1, m_int_diag_gpu),
                WebGpuBufferBinding::new(2, io_gpu),
                WebGpuBufferBinding {
                    binding: 4,
                    buffer: &params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        let workgroups = u32::try_from(output_size)
            .expect("WebGPU hash_fold output size exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&hash.fold_kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }

    fn dispatch_poseidon2_hash_rows(
        &self,
        output: &WebGpuBuffer<Digest>,
        matrix: &WebGpuBuffer<BabyBearElem>,
        row_size: usize,
        col_size: usize,
    ) -> Result<bool> {
        let Some(hash) = self.poseidon2.as_ref() else {
            return Ok(false);
        };
        if row_size == 0 {
            return Ok(true);
        }

        let (Some(output_gpu), Some(matrix_gpu)) = (output.raw_buffer(), matrix.raw_buffer())
        else {
            return Ok(false);
        };
        let (Some(round_constants_gpu), Some(m_int_diag_gpu)) = (
            hash.round_constants.raw_buffer(),
            hash.m_int_diag.raw_buffer(),
        ) else {
            return Ok(false);
        };

        let output_base = digest_word_offset(output.elem_offset)?;
        let matrix_base =
            u32::try_from(matrix.elem_offset).expect("WebGPU hash_rows matrix offset exceeds u32");
        let params = [
            0,
            0,
            u32::try_from(row_size).expect("WebGPU hash_rows row size exceeds u32"),
            u32::try_from(col_size).expect("WebGPU hash_rows col size exceeds u32"),
            output_base,
            0,
            matrix_base,
            0,
        ];
        let params =
            self.create_uniform_buffer("webgpu_poseidon2_rows_params", bytemuck::cast_slice(&params))?;

        output.sync_cpu_to_gpu(self)?;
        matrix.sync_cpu_to_gpu(self)?;

        let bind_group = self.create_bind_group(
            "webgpu_poseidon2_rows_bind_group",
            &hash.rows_layout,
            &[
                WebGpuBufferBinding::new(0, round_constants_gpu),
                WebGpuBufferBinding::new(1, m_int_diag_gpu),
                WebGpuBufferBinding::new(2, output_gpu),
                WebGpuBufferBinding::new(3, matrix_gpu),
                WebGpuBufferBinding {
                    binding: 4,
                    buffer: &params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        let workgroups = u32::try_from(row_size)
            .expect("WebGPU hash_rows row size exceeds u32")
            .div_ceil(256);
        self.dispatch_compute(&hash.rows_kernel, &bind_group, workgroups, 1, 1);
        Ok(true)
    }
}

fn byte_len_as_f64(byte_len: u64) -> Result<f64> {
    ensure!(
        byte_len <= MAX_EXACT_JS_INTEGER,
        "WebGPU byte length exceeds JS integer precision: {byte_len}"
    );
    Ok(byte_len as f64)
}

fn byte_offset_as_f64(byte_offset: u64) -> Result<f64> {
    ensure!(
        byte_offset <= MAX_EXACT_JS_INTEGER,
        "WebGPU byte offset exceeds JS integer precision: {byte_offset}"
    );
    Ok(byte_offset as f64)
}

fn byte_len_for<T>(size: usize) -> u64 {
    size.checked_mul(mem::size_of::<T>())
        .and_then(|bytes| bytes.try_into().ok())
        .expect("WebGPU buffer size overflow")
}

fn digest_word_offset(digest_offset: usize) -> Result<u32> {
    digest_offset
        .checked_mul(DIGEST_WORDS)
        .and_then(|offset| offset.try_into().ok())
        .ok_or_else(|| anyhow!("WebGPU digest word offset exceeds u32"))
}

fn slice_region_in_bounds(
    len: usize,
    rows: usize,
    cols: usize,
    offset: usize,
    stride: usize,
) -> bool {
    if rows == 0 || cols == 0 {
        return true;
    }
    let Some(last_row_start) = rows.checked_sub(1).and_then(|row| row.checked_mul(stride)) else {
        return false;
    };
    let Some(last_col) = cols.checked_sub(1) else {
        return false;
    };
    let Some(last_idx) = offset
        .checked_add(last_row_start)
        .and_then(|idx| idx.checked_add(last_col))
    else {
        return false;
    };
    last_idx < len
}

fn gather_region_in_bounds(len: usize, idx: usize, size: usize, stride: usize) -> bool {
    if size == 0 {
        return true;
    }
    let Some(last_row) = size.checked_sub(1) else {
        return false;
    };
    let Some(last_idx) = last_row
        .checked_mul(stride)
        .and_then(|value| value.checked_add(idx))
    else {
        return false;
    };
    last_idx < len
}

async fn request_device() -> Result<web_sys::GpuDevice> {
    let global = js_sys::global();
    let navigator = js_sys::Reflect::get(&global, &JsValue::from_str("navigator"))
        .map_err(js_error)
        .and_then(required_js("globalThis.navigator"))?;
    let gpu = js_sys::Reflect::get(&navigator, &JsValue::from_str("gpu"))
        .map_err(js_error)
        .and_then(required_js("navigator.gpu"))?
        .dyn_into::<web_sys::Gpu>()
        .map_err(|_| anyhow!("navigator.gpu is not a GPU object"))?;

    let adapter = match request_adapter(&gpu, false).await? {
        Some(adapter) => adapter,
        None => request_adapter(&gpu, true)
            .await?
            .ok_or_else(|| anyhow!("GPUAdapter is not available"))?,
    };

    JsFuture::from(adapter.request_device())
        .await
        .map_err(js_error)
        .and_then(required_js("GPUDevice"))?
        .dyn_into::<web_sys::GpuDevice>()
        .map_err(|_| anyhow!("requestDevice did not return a GPUDevice"))
}

async fn request_adapter(
    gpu: &web_sys::Gpu,
    force_fallback: bool,
) -> Result<Option<web_sys::GpuAdapter>> {
    let promise = if force_fallback {
        let options = web_sys::GpuRequestAdapterOptions::new();
        options.set_force_fallback_adapter(true);
        gpu.request_adapter_with_options(&options)
    } else {
        gpu.request_adapter()
    };

    let value = JsFuture::from(promise).await.map_err(js_error)?;
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }

    Ok(Some(
        value
            .dyn_into::<web_sys::GpuAdapter>()
            .map_err(|_| anyhow!("requestAdapter did not return a GPUAdapter"))?,
    ))
}

fn required_js(name: &'static str) -> impl FnOnce(JsValue) -> Result<JsValue> {
    move |value| {
        if value.is_null() || value.is_undefined() {
            Err(anyhow!("{name} is not available"))
        } else {
            Ok(value)
        }
    }
}

fn js_error(value: JsValue) -> anyhow::Error {
    anyhow!("{value:?}")
}

impl Hal for WebGpuHal {
    type Field = BabyBear;
    type Elem = BabyBearElem;
    type ExtElem = BabyBearExtElem;
    type Buffer<T: Clone + Debug + PartialEq> = WebGpuBuffer<T>;

    fn has_unified_memory(&self) -> bool {
        false
    }

    fn get_hash_suite(&self) -> &HashSuite<Self::Field> {
        self.cpu.get_hash_suite()
    }

    fn alloc_digest(&self, name: &'static str, size: usize) -> Self::Buffer<Digest> {
        self.alloc_shadowed_buffer(name, self.cpu.alloc_digest(name, size))
    }

    fn alloc_elem(&self, name: &'static str, size: usize) -> Self::Buffer<Self::Elem> {
        self.alloc_shadowed_buffer(name, self.cpu.alloc_elem(name, size))
    }

    fn alloc_extelem(&self, name: &'static str, size: usize) -> Self::Buffer<Self::ExtElem> {
        self.alloc_shadowed_buffer(name, self.cpu.alloc_extelem(name, size))
    }

    fn alloc_u32(&self, name: &'static str, size: usize) -> Self::Buffer<u32> {
        self.alloc_shadowed_buffer(name, self.cpu.alloc_u32(name, size))
    }

    fn alloc_elem_init(
        &self,
        name: &'static str,
        size: usize,
        value: Self::Elem,
    ) -> Self::Buffer<Self::Elem> {
        self.alloc_shadowed_buffer(name, self.cpu.alloc_elem_init(name, size, value))
    }

    fn alloc_extelem_zeroed(&self, name: &'static str, size: usize) -> Self::Buffer<Self::ExtElem> {
        self.alloc_shadowed_buffer(name, self.cpu.alloc_extelem_zeroed(name, size))
    }

    fn copy_from_digest(&self, name: &'static str, slice: &[Digest]) -> Self::Buffer<Digest> {
        self.copy_shadowed_buffer(name, self.cpu.copy_from_digest(name, slice), slice)
    }

    fn copy_from_elem(&self, name: &'static str, slice: &[Self::Elem]) -> Self::Buffer<Self::Elem> {
        self.copy_shadowed_buffer(name, self.cpu.copy_from_elem(name, slice), slice)
    }

    fn copy_from_extelem(
        &self,
        name: &'static str,
        slice: &[Self::ExtElem],
    ) -> Self::Buffer<Self::ExtElem> {
        self.copy_shadowed_buffer(name, self.cpu.copy_from_extelem(name, slice), slice)
    }

    fn copy_from_u32(&self, name: &'static str, slice: &[u32]) -> Self::Buffer<u32> {
        self.copy_shadowed_buffer(name, self.cpu.copy_from_u32(name, slice), slice)
    }

    fn batch_expand_into_evaluate_ntt(
        &self,
        output: &Self::Buffer<Self::Elem>,
        input: &Self::Buffer<Self::Elem>,
        count: usize,
        expand_bits: usize,
    ) {
        let gpu_evaluated = self
            .dispatch_batch_expand_into_evaluate_ntt(output, input, count, expand_bits)
            .unwrap_or_else(|err| {
                panic!("failed to expand and evaluate NTT with WebGPU: {err}")
            });
        self.cpu
            .batch_expand_into_evaluate_ntt(output.cpu(), input.cpu(), count, expand_bits);
        self.record_gpu_result_with_cpu_mirror("batch_expand_into_evaluate_ntt", gpu_evaluated);
        output.cpu_dirty.set(!gpu_evaluated);
    }

    fn batch_interpolate_ntt(&self, io: &Self::Buffer<Self::Elem>, count: usize) {
        let gpu_interpolated = self
            .dispatch_batch_interpolate_ntt(io, count)
            .unwrap_or_else(|err| panic!("failed to interpolate NTT with WebGPU: {err}"));
        self.cpu.batch_interpolate_ntt(io.cpu(), count);
        self.record_gpu_result_with_cpu_mirror("batch_interpolate_ntt", gpu_interpolated);
        io.cpu_dirty.set(!gpu_interpolated);
    }

    fn batch_bit_reverse(&self, io: &Self::Buffer<Self::Elem>, count: usize) {
        let row_size = io.size() / count;
        assert_eq!(row_size * count, io.size());
        let bits = crate::core::log2_ceil(row_size);
        assert_eq!(row_size, 1 << bits);
        let gpu_reversed = self
            .dispatch_batch_bit_reverse(io, bits)
            .unwrap_or_else(|err| panic!("failed to bit-reverse with WebGPU: {err}"));
        self.cpu.batch_bit_reverse(io.cpu(), count);
        self.record_gpu_result_with_cpu_mirror("batch_bit_reverse", gpu_reversed);
        io.cpu_dirty.set(!gpu_reversed);
    }

    fn batch_evaluate_any(
        &self,
        coeffs: &Self::Buffer<Self::Elem>,
        poly_count: usize,
        which: &Self::Buffer<u32>,
        xs: &Self::Buffer<Self::ExtElem>,
        out: &Self::Buffer<Self::ExtElem>,
    ) {
        let po2 = crate::core::log2_ceil(coeffs.size() / poly_count);
        let deg = 1 << po2;
        assert_eq!(poly_count * deg, coeffs.size());
        let eval_count = which.size();
        assert_eq!(xs.size(), eval_count);
        assert_eq!(out.size(), eval_count);
        let gpu_evaluated = self
            .dispatch_batch_evaluate_any(coeffs, which, xs, out, deg)
            .unwrap_or_else(|err| panic!("failed to batch evaluate with WebGPU: {err}"));
        self.cpu
            .batch_evaluate_any(coeffs.cpu(), poly_count, which.cpu(), xs.cpu(), out.cpu());
        self.record_gpu_result_with_cpu_mirror("batch_evaluate_any", gpu_evaluated);
        out.cpu_dirty.set(!gpu_evaluated);
    }

    fn zk_shift(&self, io: &Self::Buffer<Self::Elem>, count: usize) {
        let bits = crate::core::log2_ceil(io.size() / count);
        assert_eq!(io.size(), count * (1 << bits));
        let gpu_shifted = self
            .dispatch_zk_shift(io, bits)
            .unwrap_or_else(|err| panic!("failed to zk_shift with WebGPU: {err}"));
        self.cpu.zk_shift(io.cpu(), count);
        self.record_gpu_result_with_cpu_mirror("zk_shift", gpu_shifted);
        io.cpu_dirty.set(!gpu_shifted);
    }

    fn mix_poly_coeffs(
        &self,
        out: &Self::Buffer<Self::ExtElem>,
        mix_start: &Self::ExtElem,
        mix: &Self::ExtElem,
        input: &Self::Buffer<Self::Elem>,
        combos: &Self::Buffer<u32>,
        input_size: usize,
        count: usize,
    ) {
        let gpu_mixed = self
            .dispatch_mix_poly_coeffs(out, mix_start, mix, input, combos, input_size, count)
            .unwrap_or_else(|err| panic!("failed to mix polynomial coeffs with WebGPU: {err}"));
        self.cpu.mix_poly_coeffs(
            out.cpu(),
            mix_start,
            mix,
            input.cpu(),
            combos.cpu(),
            input_size,
            count,
        );
        self.record_gpu_result_with_cpu_mirror("mix_poly_coeffs", gpu_mixed);
        out.cpu_dirty.set(!gpu_mixed);
    }

    fn eltwise_add_elem(
        &self,
        output: &Self::Buffer<Self::Elem>,
        input1: &Self::Buffer<Self::Elem>,
        input2: &Self::Buffer<Self::Elem>,
    ) {
        let gpu_added = self
            .dispatch_eltwise_add_elem(output, input1, input2)
            .unwrap_or_else(|err| panic!("failed to add WebGPU buffers: {err}"));
        self.cpu
            .eltwise_add_elem(output.cpu(), input1.cpu(), input2.cpu());
        self.record_gpu_result_with_cpu_mirror("eltwise_add_elem", gpu_added);
        output.cpu_dirty.set(!gpu_added);
    }

    fn eltwise_sum_extelem(
        &self,
        output: &Self::Buffer<Self::Elem>,
        input: &Self::Buffer<Self::ExtElem>,
    ) {
        let gpu_summed = self
            .dispatch_eltwise_sum_extelem(output, input)
            .unwrap_or_else(|err| panic!("failed to sum WebGPU extension buffers: {err}"));
        self.cpu.eltwise_sum_extelem(output.cpu(), input.cpu());
        self.record_gpu_result_with_cpu_mirror("eltwise_sum_extelem", gpu_summed);
        output.cpu_dirty.set(!gpu_summed);
    }

    fn eltwise_copy_elem(
        &self,
        output: &Self::Buffer<Self::Elem>,
        input: &Self::Buffer<Self::Elem>,
    ) {
        let gpu_copied = if output.size() == input.size() {
            if let (Some(input_gpu), Some(output_gpu)) = (input.raw_buffer(), output.raw_buffer()) {
                input
                    .sync_cpu_to_gpu(self)
                    .unwrap_or_else(|err| panic!("failed to sync WebGPU input buffer: {err}"));
                if input_gpu == output_gpu {
                    false
                } else {
                    self.copy_gpu_buffer(
                        input_gpu,
                        input.byte_offset(),
                        output_gpu,
                        output.byte_offset(),
                        byte_len_for::<Self::Elem>(output.size()),
                    )
                    .unwrap_or_else(|err| panic!("failed to copy WebGPU buffer: {err}"));
                    true
                }
            } else {
                false
            }
        } else {
            false
        };

        self.cpu.eltwise_copy_elem(output.cpu(), input.cpu());
        self.record_gpu_result_with_cpu_mirror("eltwise_copy_elem", gpu_copied);
        output.cpu_dirty.set(!gpu_copied);
    }

    fn eltwise_copy_elem_slice(
        &self,
        into: &Self::Buffer<Self::Elem>,
        from: &[Self::Elem],
        from_rows: usize,
        from_cols: usize,
        from_offset: usize,
        from_stride: usize,
        into_offset: usize,
        into_stride: usize,
    ) {
        let gpu_copied = self
            .dispatch_eltwise_copy_elem_slice(
                into,
                from,
                from_rows,
                from_cols,
                from_offset,
                from_stride,
                into_offset,
                into_stride,
            )
            .unwrap_or_else(|err| panic!("failed to copy WebGPU element slice: {err}"));
        self.cpu.eltwise_copy_elem_slice(
            into.cpu(),
            from,
            from_rows,
            from_cols,
            from_offset,
            from_stride,
            into_offset,
            into_stride,
        );
        self.record_gpu_result_with_cpu_mirror("eltwise_copy_elem_slice", gpu_copied);
        into.cpu_dirty.set(!gpu_copied);
    }

    fn eltwise_zeroize_elem(&self, elems: &Self::Buffer<Self::Elem>) {
        let gpu_zeroized = self
            .dispatch_zeroize_elem(elems)
            .unwrap_or_else(|err| panic!("failed to zeroize WebGPU buffer: {err}"));
        self.cpu.eltwise_zeroize_elem(elems.cpu());
        self.record_gpu_result_with_cpu_mirror("eltwise_zeroize_elem", gpu_zeroized);
        elems.cpu_dirty.set(!gpu_zeroized);
    }

    fn fri_fold(
        &self,
        output: &Self::Buffer<Self::Elem>,
        input: &Self::Buffer<Self::Elem>,
        mix: &Self::ExtElem,
    ) {
        let count = output.size() / Self::ExtElem::EXT_SIZE;
        assert_eq!(output.size(), count * Self::ExtElem::EXT_SIZE);
        assert_eq!(input.size(), output.size() * crate::FRI_FOLD);
        let gpu_folded = self
            .dispatch_fri_fold(output, input, mix)
            .unwrap_or_else(|err| panic!("failed to FRI fold with WebGPU: {err}"));
        self.cpu.fri_fold(output.cpu(), input.cpu(), mix);
        self.record_gpu_result_with_cpu_mirror("fri_fold", gpu_folded);
        output.cpu_dirty.set(!gpu_folded);
    }

    fn hash_rows(&self, output: &Self::Buffer<Digest>, matrix: &Self::Buffer<Self::Elem>) {
        let row_size = output.size();
        let col_size = matrix.size() / output.size();
        assert_eq!(matrix.size(), col_size * row_size);
        let gpu_hashed = self
            .dispatch_poseidon2_hash_rows(output, matrix, row_size, col_size)
            .unwrap_or_else(|err| panic!("failed to hash rows with WebGPU Poseidon2: {err}"));
        self.cpu.hash_rows(output.cpu(), matrix.cpu());
        self.record_gpu_result_with_cpu_mirror("hash_rows", gpu_hashed);
        output.cpu_dirty.set(!gpu_hashed);
    }

    fn hash_fold(&self, io: &Self::Buffer<Digest>, input_size: usize, output_size: usize) {
        assert!(io.size() >= 2 * input_size);
        assert_eq!(input_size, 2 * output_size);
        let gpu_hashed = self
            .dispatch_poseidon2_hash_fold(io, input_size, output_size)
            .unwrap_or_else(|err| panic!("failed to hash fold with WebGPU Poseidon2: {err}"));
        self.cpu.hash_fold(io.cpu(), input_size, output_size);
        self.record_gpu_result_with_cpu_mirror("hash_fold", gpu_hashed);
        io.cpu_dirty.set(!gpu_hashed);
    }

    fn gather_sample(
        &self,
        dst: &Self::Buffer<Self::Elem>,
        src: &Self::Buffer<Self::Elem>,
        idx: usize,
        size: usize,
        stride: usize,
    ) {
        let gpu_gathered = self
            .dispatch_gather_sample(dst, src, idx, size, stride)
            .unwrap_or_else(|err| panic!("failed to gather WebGPU sample: {err}"));
        self.cpu
            .gather_sample(dst.cpu(), src.cpu(), idx, size, stride);
        self.record_gpu_result_with_cpu_mirror("gather_sample", gpu_gathered);
        dst.cpu_dirty.set(!gpu_gathered);
    }

    fn scatter(
        &self,
        into: &Self::Buffer<Self::Elem>,
        index: &[u32],
        offsets: &[u32],
        values: &[Self::Elem],
    ) {
        let gpu_scattered = self
            .dispatch_scatter(into, index, offsets, values)
            .unwrap_or_else(|err| panic!("failed to scatter WebGPU buffers: {err}"));
        self.cpu.scatter(into.cpu(), index, offsets, values);
        self.record_gpu_result_with_cpu_mirror("scatter", gpu_scattered);
        into.cpu_dirty.set(!gpu_scattered);
    }

    fn prefix_products(&self, io: &Self::Buffer<Self::ExtElem>) {
        let gpu_computed = self
            .dispatch_prefix_products(io)
            .unwrap_or_else(|err| panic!("failed to compute WebGPU prefix products: {err}"));
        self.cpu.prefix_products(io.cpu());
        self.record_gpu_result_with_cpu_mirror("prefix_products", gpu_computed);
        io.cpu_dirty.set(!gpu_computed);
    }
}
