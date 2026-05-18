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
    fmt::{Debug, Write as _},
    marker::PhantomData,
    mem,
    rc::Rc,
};

use anyhow::{anyhow, ensure, Result};
use risc0_core::field::{
    baby_bear::{BabyBear, BabyBearElem, BabyBearExtElem},
    Elem as _, ExtElem as _, RootsOfUnity,
};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use super::{
    cpu::{CpuBuffer, CpuHal},
    Buffer, Hal,
};
use crate::core::{
    digest::{Digest, DIGEST_WORDS},
    hash::{poseidon2, HashSuite},
    log2_ceil,
};
use crate::{
    adapter::{PolyExtStep, PolyExtStepDef},
    hal::webgpu_codegen::{
        eval_check_fp_slot, eval_check_last_uses, eval_check_mix_slot, eval_check_note_last,
        staged_multi_kernel_from_def, CodegenError, EmitterTap, EvalCheckSlotAllocator, FieldMode,
        STAGED_EVAL_CHECK_PRELUDE_WGSL,
    },
    taps::TapSet,
    INV_RATE,
};

// SP4 (R8): `BufferPool` + `TileLayout` for tiled multi-buffer source
// representations of recursion-sized data groups. Lives in a child
// module so the pure addressing math has its own unit tests without
// dragging in all of `webgpu.rs`. Public so the browser-prove harness
// can construct a BufferPool from its regression test.
pub mod buffer_pool;

/// Target chunk size for multi-stage staged WGSL emission. Chosen so the
/// rv32im production DEF (~20k ops) emits ~4 stages, mirroring the CUDA
/// `eval_check_{0,1,2,3}.cu` layout the runtime interpreter parallels.
const SP3_STAGED_TARGET_CHUNK_OPS: usize = 7000;

/// SP3 iter 7d: number of cycles processed per scratch-buffer tile. The
/// scratch buffer is sized to `tile_size * stride * 4 B` regardless of
/// the prove's full domain — the dispatch loop iterates
/// `ceil(domain / tile_size)` tiles, bumping `tile_base` between passes.
/// At 4096 with ~400 live ext fp vars + 5 live mix vars the per-cycle
/// stride is ~1620 u32, so fp_scratch = 4096 * 1620 * 4 ≈ 27 MiB; mix
/// scratches are < 1 MiB each. Comfortably within the device limits we
/// requested and independent of how many cycles the prove has overall.
const SP3_STAGED_TILE_SIZE: u32 = 4096;

/// SP3 iter 7i (2026-05-12): minimum `def.block.len()` to attempt the
/// staged WGSL fast path. Below this, the runtime interpreter is used
/// regardless of `staged_eval_check_enabled`. rv32im production DEFs
/// have ~20k+ ops; recursion's lift DEF and ad-hoc small DEFs sit
/// below and currently SIGKILL Chrome somewhere in the lift's
/// finalize_async flow when staged. Until that's root-caused, the gate
/// keeps staged where it wins (rv32im) and out of where it doesn't
/// (everything smaller).
const SP3_STAGED_MIN_BLOCK_OPS: usize = 8000;

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
const WEBGPU_WORKGROUP_SIZE: u32 = 256;
const WEBGPU_MAX_WORKGROUPS_PER_DIMENSION: u32 = 65_535;
const WEBGPU_REQUESTED_MAX_BUFFER_BYTES: u64 = 4 * 1024 * 1024 * 1024 - 4; // 4 GiB - alignment slack; many adapters cap at this
const WEBGPU_REQUESTED_MAX_STORAGE_BINDING_BYTES: u64 = 4 * 1024 * 1024 * 1024 - 4;
const WEBGPU_REQUESTED_MAX_WORKGROUP_STORAGE_BYTES: u32 = 128 * 1024;
/// SP3 iter 7c: staged eval_check uses 7 read-only storage buffers
/// (group0..2, global0..1, mix_pows, plus mix_tot/mix_mul scratch) and
/// 3 read-write storage buffers (check, fp_scratch, mix_tot/mix_mul
/// scratch — overlap intentional, the binding type is `storage` not
/// `read_only_storage`). WebGPU's default `maxStorageBuffersPerShaderStage`
/// is 8; we request more so the multi-stage pipeline's bind group fits.
const WEBGPU_REQUESTED_MAX_STORAGE_BUFFERS_PER_STAGE: u32 = 16;

/// SP3 iter 7x (2026-05-13): bump `maxUniformBufferBindingSize` so the
/// staged eval_check's `mix_pows` can live in a uniform buffer instead
/// of a storage buffer. CUDA's reference uses `__constant__` memory
/// for `poly_mix` (broadcast-cached, low-latency); UBOs are the
/// WebGPU analog. For rv32im poseidon2_basic, `mix_pow_words = 25864`
/// (= 103 KiB), so we need at least 128 KiB. 256 KiB gives headroom
/// for larger DEFs.
const WEBGPU_REQUESTED_MAX_UNIFORM_BUFFER_BINDING_BYTES: u64 = 256 * 1024;

/// SP3 iter 7x: capacity of the staged `mix_pows` uniform buffer in
/// `vec4<u32>` slots. The WGSL prelude declares
/// `array<vec4<u32>, SP3_STAGED_MIX_POWS_UBO_VEC4_CAPACITY>`; only the
/// first `mix_pow_words / 4` entries are populated per call. 16384
/// vec4 = 262144 B = 256 KiB matches the bumped UBO limit above.
const SP3_STAGED_MIX_POWS_UBO_VEC4_CAPACITY: usize = 16384;
const WEBGPU_SAFE_STORAGE_BINDING_BYTES: u64 = 1024 * 1024 * 1024;
const WEBGPU_SAFE_QUEUE_WRITE_BYTES: usize = 16 * 1024 * 1024;
const WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT: u64 = 256;
const WEBGPU_EVAL_CHECK_MAX_POLY_EXT_STEPS: usize = 4096;
const WEBGPU_EVAL_CHECK_ENABLE_SPLIT: bool = false;
const WEBGPU_EVAL_CHECK_SPLIT_TERMS_PER_SHADER: usize = 64;
const WEBGPU_EVAL_CHECK_SPLIT_FP_OPS_PER_SHADER: usize = 2048;
const WEBGPU_EVAL_CHECK_MAX_FP_SLOTS: usize = 1536;
const WEBGPU_EVAL_CHECK_MAX_FP_ELEM_SLOTS: usize = 8192;
const WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS: usize = 1536;
const WEBGPU_EVAL_CHECK_MAX_MIX_SLOTS: usize = 64;
const WEBGPU_EVAL_CHECK_INTERPRETER_WORKGROUP_SIZE: u32 = 32;
const WEBGPU_EVAL_CHECK_INSTRUCTION_WORDS: usize = 8;
const WEBGPU_BATCH_EVALUATE_CHUNK_SIZE: usize = 1024;

const WEBGPU_EVAL_OP_CONST: u32 = 0;
const WEBGPU_EVAL_OP_CONST_EXT: u32 = 1;
const WEBGPU_EVAL_OP_GET: u32 = 2;
const WEBGPU_EVAL_OP_GET_GLOBAL: u32 = 3;
const WEBGPU_EVAL_OP_ADD: u32 = 4;
const WEBGPU_EVAL_OP_SUB: u32 = 5;
const WEBGPU_EVAL_OP_MUL: u32 = 6;
const WEBGPU_EVAL_OP_TRUE: u32 = 7;
const WEBGPU_EVAL_OP_AND_EQZ: u32 = 8;
const WEBGPU_EVAL_OP_AND_COND: u32 = 9;

/// Browser console timer for proof-stage telemetry.
///
/// This is intentionally scoped to the browser WebGPU HAL so circuit crates can
/// add wasm-only timing probes without depending directly on JS bindings.
pub struct WebGpuStageTimer {
    label: String,
    start_ms: f64,
    gpu_active: bool,
    // SP6d iter 4: if set, increment THIS HAL's counter on drop instead
    // of the thread-local global. Each WebGpuHal owns its own counter via
    // `Rc<Cell<f64>>`. Multi-HAL pools (SP6d) need this so per-slot
    // gpu_active_ms doesn't over-count by other slots' activity.
    hal_active_counter: Option<Rc<Cell<f64>>>,
}

thread_local! {
    static WEBGPU_GPU_ACTIVE_MS: Cell<f64> = const { Cell::new(0.0) };
}

/// Optional circuit-specific WebGPU implementation of the check-polynomial
/// evaluation stage.
///
/// Returning `Ok(false)` means the circuit cannot handle the current buffers on
/// GPU and the prover should use the portable CPU-compatible fallback.
pub trait WebGpuCircuitEvalCheck {
    fn eval_check_webgpu(
        &self,
        _hal: &WebGpuHal,
        _check: &WebGpuBuffer<BabyBearElem>,
        _groups: &[&WebGpuBuffer<BabyBearElem>],
        _globals: &[&WebGpuBuffer<BabyBearElem>],
        _poly_mix: BabyBearExtElem,
        _po2: usize,
        _steps: usize,
    ) -> Result<bool> {
        Ok(false)
    }
}

impl WebGpuStageTimer {
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        log_webgpu_stage(&format!("browser-prove:stage start {label}"));
        Self {
            label,
            start_ms: js_sys::Date::now(),
            gpu_active: false,
            hal_active_counter: None,
        }
    }

    pub fn new_active(label: impl Into<String>) -> Self {
        let label = label.into();
        log_webgpu_stage(&format!("browser-prove:stage start {label}"));
        Self {
            label,
            start_ms: js_sys::Date::now(),
            gpu_active: true,
            hal_active_counter: None,
        }
    }

    /// SP6d iter 4: HAL-scoped active timer. The HAL's counter is
    /// incremented on drop instead of the thread-local global. Use this
    /// when running under a multi-HAL pool so each HAL's gpu_idle_ratio
    /// is accurate.
    pub fn new_active_for(label: impl Into<String>, hal: &WebGpuHal) -> Self {
        let label = label.into();
        log_webgpu_stage(&format!("browser-prove:stage start {label}"));
        Self {
            label,
            start_ms: js_sys::Date::now(),
            gpu_active: true,
            hal_active_counter: Some(hal.gpu_active_ms_handle()),
        }
    }

    pub fn snapshot_gpu_active_ms() -> f64 {
        WEBGPU_GPU_ACTIVE_MS.with(|c| c.get())
    }

    pub fn reset_gpu_active_ms() {
        WEBGPU_GPU_ACTIVE_MS.with(|c| c.set(0.0));
    }
}

impl Drop for WebGpuStageTimer {
    fn drop(&mut self) {
        let elapsed_ms = js_sys::Date::now() - self.start_ms;
        if self.gpu_active {
            if let Some(counter) = &self.hal_active_counter {
                counter.set(counter.get() + elapsed_ms);
            } else {
                WEBGPU_GPU_ACTIVE_MS.with(|c| c.set(c.get() + elapsed_ms));
            }
            log_webgpu_stage(&format!(
                "browser-prove:stage done {} elapsed_ms={elapsed_ms:.3} gpu_active=true",
                self.label
            ));
        } else {
            log_webgpu_stage(&format!(
                "browser-prove:stage done {} elapsed_ms={elapsed_ms:.3}",
                self.label
            ));
        }
    }
}

pub(crate) fn log_webgpu_stage(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(message));
}

pub fn log_webgpu_metric(message: &str) {
    log_webgpu_stage(&format!("browser-prove:metric {message}"));
}

#[derive(Clone, Copy)]
struct EvalCheckTap {
    group: usize,
    offset: usize,
    back: usize,
}

#[derive(Clone, Copy)]
enum EvalCheckFpOp {
    Const(u32),
    ConstExt(u32, u32, u32, u32),
    Get(usize),
    GetGlobal(usize, usize),
    Add(usize, usize),
    Sub(usize, usize),
    Mul(usize, usize),
}

#[derive(Clone, Copy)]
enum EvalCheckMixOp {
    True,
    AndEqz {
        chain: usize,
        inner: usize,
    },
    AndCond {
        chain: usize,
        cond: usize,
        inner: usize,
    },
}

#[derive(Clone)]
struct EvalCheckTerm {
    mix_exp: usize,
    conds: Vec<usize>,
    inner: usize,
}

struct EvalCheckProgram {
    fp_ops: Vec<EvalCheckFpOp>,
    mix_ops: Vec<EvalCheckMixOp>,
    mix_exps: Vec<usize>,
}

fn elem_word(value: BabyBearElem) -> u32 {
    value.to_u32_words()[0]
}

fn elem_const_word(value: u32) -> u32 {
    elem_word(BabyBearElem::from_u64(value as u64))
}

fn ext_words(value: BabyBearExtElem) -> [u32; 4] {
    let elems = value.subelems();
    [
        elem_word(elems[0]),
        elem_word(elems[1]),
        elem_word(elems[2]),
        elem_word(elems[3]),
    ]
}

fn eval_check_mix_exponents(def: &PolyExtStepDef) -> Result<Vec<usize>> {
    let mut exponents = Vec::new();
    for op in def.block {
        match op {
            PolyExtStep::True => exponents.push(0),
            PolyExtStep::AndEqz(chain, _) => {
                let exponent = exponents.get(*chain).ok_or_else(|| {
                    anyhow!("poly_ext AndEqz chain index {chain} is out of range")
                })? + 1;
                exponents.push(exponent);
            }
            PolyExtStep::AndCond(chain, _, inner) => {
                let chain_exp = exponents.get(*chain).ok_or_else(|| {
                    anyhow!("poly_ext AndCond chain index {chain} is out of range")
                })?;
                let inner_exp = exponents.get(*inner).ok_or_else(|| {
                    anyhow!("poly_ext AndCond inner index {inner} is out of range")
                })?;
                exponents.push(chain_exp + inner_exp);
            }
            _ => {}
        }
    }

    ensure!(
        def.ret < exponents.len(),
        "poly_ext return mix index {} exceeds mix count {}",
        def.ret,
        exponents.len()
    );
    Ok(exponents)
}

fn eval_check_mix_pows(
    def: &PolyExtStepDef,
    poly_mix: BabyBearExtElem,
) -> Result<Vec<BabyBearExtElem>> {
    let exponents = eval_check_mix_exponents(def)?;
    let max_exp = exponents.iter().copied().max().unwrap_or(0);
    let mut powers = Vec::with_capacity(max_exp + 1);
    let mut cur = BabyBearExtElem::ONE;
    for _ in 0..=max_exp {
        powers.push(cur);
        cur *= poly_mix;
    }
    Ok(exponents.into_iter().map(|exp| powers[exp]).collect())
}

fn eval_check_all_mix_pows(
    def: &PolyExtStepDef,
    poly_mix: BabyBearExtElem,
) -> Result<Vec<BabyBearExtElem>> {
    let max_exp = eval_check_mix_exponents(def)?
        .into_iter()
        .max()
        .unwrap_or(0);
    let mut powers = Vec::with_capacity(max_exp + 1);
    let mut cur = BabyBearExtElem::ONE;
    for _ in 0..=max_exp {
        powers.push(cur);
        cur *= poly_mix;
    }
    Ok(powers)
}

fn eval_check_program(def: &PolyExtStepDef) -> Result<EvalCheckProgram> {
    let mut fp_ops = Vec::new();
    let mut mix_ops = Vec::new();
    let mut mix_exps = Vec::new();
    for op in def.block {
        match op {
            PolyExtStep::Const(value) => fp_ops.push(EvalCheckFpOp::Const(*value)),
            PolyExtStep::ConstExt(x0, x1, x2, x3) => {
                fp_ops.push(EvalCheckFpOp::ConstExt(*x0, *x1, *x2, *x3));
            }
            PolyExtStep::Get(tap) => fp_ops.push(EvalCheckFpOp::Get(*tap)),
            PolyExtStep::GetGlobal(arg, offset) => {
                fp_ops.push(EvalCheckFpOp::GetGlobal(*arg, *offset));
            }
            PolyExtStep::Add(lhs, rhs) => fp_ops.push(EvalCheckFpOp::Add(*lhs, *rhs)),
            PolyExtStep::Sub(lhs, rhs) => fp_ops.push(EvalCheckFpOp::Sub(*lhs, *rhs)),
            PolyExtStep::Mul(lhs, rhs) => fp_ops.push(EvalCheckFpOp::Mul(*lhs, *rhs)),
            PolyExtStep::True => {
                mix_ops.push(EvalCheckMixOp::True);
                mix_exps.push(0);
            }
            PolyExtStep::AndEqz(chain, inner) => {
                let chain_exp = *mix_exps.get(*chain).ok_or_else(|| {
                    anyhow!("poly_ext AndEqz chain index {chain} is out of range")
                })?;
                ensure!(
                    *inner < fp_ops.len(),
                    "poly_ext AndEqz inner index {inner} is out of range"
                );
                mix_ops.push(EvalCheckMixOp::AndEqz {
                    chain: *chain,
                    inner: *inner,
                });
                mix_exps.push(chain_exp + 1);
            }
            PolyExtStep::AndCond(chain, cond, inner) => {
                let chain_exp = *mix_exps.get(*chain).ok_or_else(|| {
                    anyhow!("poly_ext AndCond chain index {chain} is out of range")
                })?;
                let inner_exp = *mix_exps.get(*inner).ok_or_else(|| {
                    anyhow!("poly_ext AndCond inner index {inner} is out of range")
                })?;
                ensure!(
                    *cond < fp_ops.len(),
                    "poly_ext AndCond cond index {cond} is out of range"
                );
                mix_ops.push(EvalCheckMixOp::AndCond {
                    chain: *chain,
                    cond: *cond,
                    inner: *inner,
                });
                mix_exps.push(chain_exp + inner_exp);
            }
        }
    }

    ensure!(
        def.ret < mix_ops.len(),
        "poly_ext return mix index {} exceeds mix count {}",
        def.ret,
        mix_ops.len()
    );
    Ok(EvalCheckProgram {
        fp_ops,
        mix_ops,
        mix_exps,
    })
}

fn eval_check_flatten_terms(program: &EvalCheckProgram, ret: usize) -> Result<Vec<EvalCheckTerm>> {
    ensure!(
        ret < program.mix_ops.len(),
        "poly_ext return mix index {ret} exceeds mix count {}",
        program.mix_ops.len()
    );

    let mut terms = Vec::new();
    let mut stack = vec![(ret, 0usize, Vec::new())];
    while let Some((mix_idx, extra_exp, conds)) = stack.pop() {
        match program.mix_ops[mix_idx] {
            EvalCheckMixOp::True => {}
            EvalCheckMixOp::AndEqz { chain, inner } => {
                stack.push((chain, extra_exp, conds.clone()));
                terms.push(EvalCheckTerm {
                    mix_exp: extra_exp + program.mix_exps[chain],
                    conds,
                    inner,
                });
            }
            EvalCheckMixOp::AndCond { chain, cond, inner } => {
                stack.push((chain, extra_exp, conds.clone()));
                let mut inner_conds = conds;
                inner_conds.push(cond);
                stack.push((inner, extra_exp + program.mix_exps[chain], inner_conds));
            }
        }
    }
    Ok(terms)
}

fn eval_check_fp_dependencies(op: EvalCheckFpOp) -> &'static [usize] {
    match op {
        EvalCheckFpOp::Add(_, _) | EvalCheckFpOp::Sub(_, _) | EvalCheckFpOp::Mul(_, _) => {
            // Handled by `eval_check_note_fp_dependencies`.
            &[]
        }
        _ => &[],
    }
}

fn eval_check_note_fp_dependencies(op: EvalCheckFpOp, stack: &mut Vec<usize>) {
    match op {
        EvalCheckFpOp::Add(lhs, rhs)
        | EvalCheckFpOp::Sub(lhs, rhs)
        | EvalCheckFpOp::Mul(lhs, rhs) => {
            stack.push(lhs);
            stack.push(rhs);
        }
        _ => {}
    }
}

fn eval_check_needed_fp_vars(
    program: &EvalCheckProgram,
    terms: &[EvalCheckTerm],
) -> Result<Vec<usize>> {
    let mut needed = HashSet::new();
    let mut stack = Vec::new();
    for term in terms {
        stack.push(term.inner);
        stack.extend(term.conds.iter().copied());
    }

    while let Some(var) = stack.pop() {
        if !needed.insert(var) {
            continue;
        }
        let op = *program
            .fp_ops
            .get(var)
            .ok_or_else(|| anyhow!("poly_ext fp var {var} is out of range"))?;
        let _ = eval_check_fp_dependencies(op);
        eval_check_note_fp_dependencies(op, &mut stack);
    }

    let mut needed: Vec<_> = needed.into_iter().collect();
    needed.sort_unstable();
    Ok(needed)
}

fn eval_check_split_term_chunks(
    program: &EvalCheckProgram,
    terms: &[EvalCheckTerm],
) -> Result<Vec<Vec<EvalCheckTerm>>> {
    let mut chunks = Vec::new();
    let mut cur_terms = Vec::new();
    let mut cur_needed = HashSet::new();

    for term in terms {
        let term_needed = eval_check_needed_fp_vars(program, std::slice::from_ref(term))?;
        ensure!(
            term_needed.len() <= WEBGPU_EVAL_CHECK_SPLIT_FP_OPS_PER_SHADER,
            "single split eval_check term needs {} FP ops, max is {}",
            term_needed.len(),
            WEBGPU_EVAL_CHECK_SPLIT_FP_OPS_PER_SHADER
        );

        let mut next_needed_len = cur_needed.len();
        for var in &term_needed {
            if !cur_needed.contains(var) {
                next_needed_len += 1;
            }
        }

        if !cur_terms.is_empty()
            && (cur_terms.len() >= WEBGPU_EVAL_CHECK_SPLIT_TERMS_PER_SHADER
                || next_needed_len > WEBGPU_EVAL_CHECK_SPLIT_FP_OPS_PER_SHADER)
        {
            chunks.push(std::mem::take(&mut cur_terms));
            cur_needed.clear();
        }

        for var in term_needed {
            cur_needed.insert(var);
        }
        cur_terms.push(term.clone());
    }

    if !cur_terms.is_empty() {
        chunks.push(cur_terms);
    }

    Ok(chunks)
}

fn eval_check_zerofier_inv_words(po2: usize, steps: usize) -> [u32; 4] {
    let exp_po2 = log2_ceil(INV_RATE);
    let rou = BabyBearElem::ROU_FWD[po2 + exp_po2];
    let three = BabyBearElem::from_u64(3);
    let three_to_steps = three.pow(steps);
    let rou_to_steps = rou.pow(steps);
    let mut x_to_steps = BabyBearElem::ONE;
    let mut invs = [0; 4];
    for inv in invs.iter_mut().take(INV_RATE) {
        *inv = elem_word((three_to_steps * x_to_steps - BabyBearElem::ONE).inv());
        x_to_steps *= rou_to_steps;
    }
    invs
}

fn eval_check_ext_const(words: [u32; 4]) -> String {
    format!(
        "vec4<u32>({}u, {}u, {}u, {}u)",
        words[0], words[1], words[2], words[3]
    )
}

// EvalCheckSlotAllocator, eval_check_last_uses, eval_check_fp_slot, and
// eval_check_mix_slot moved to `risc0/zkp/src/hal/webgpu_codegen.rs` so
// the staged-WGSL emitter (which is built on all targets when the
// `webgpu` feature is on) can share the same slot-allocation discipline
// as the runtime interpreter here. See iter 6 commit.

fn eval_check_interpreter_instructions_with_limit(
    taps: &TapSet<'_>,
    def: &PolyExtStepDef,
    max_fp_slots: usize,
    allow_const_ext: bool,
) -> Result<(Vec<u32>, usize, usize, usize)> {
    let tap_info: Vec<_> = taps
        .taps()
        .map(|tap| EvalCheckTap {
            group: tap.group(),
            offset: tap.offset(),
            back: tap.back(),
        })
        .collect();
    let (last_fp, last_mix, _fp_count, _mix_count) = eval_check_last_uses(def)?;
    let mut fp_alloc = EvalCheckSlotAllocator::default();
    let mut mix_alloc = EvalCheckSlotAllocator::default();
    let mut fp_slots: Vec<Option<usize>> = Vec::new();
    let mut mix_slots: Vec<Option<usize>> = Vec::new();
    let mut instructions = Vec::new();

    let mut push_instr = |words: [u32; WEBGPU_EVAL_CHECK_INSTRUCTION_WORDS]| {
        instructions.extend(words);
    };

    for (op_idx, op) in def.block.iter().enumerate() {
        let mut used_fp = Vec::new();
        let mut used_mix = Vec::new();
        match op {
            PolyExtStep::Const(value) => {
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                push_instr([
                    WEBGPU_EVAL_OP_CONST,
                    u32::try_from(out_slot).expect("eval_check fp slot exceeds u32"),
                    elem_const_word(*value),
                    0,
                    0,
                    0,
                    0,
                    0,
                ]);
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::ConstExt(x0, x1, x2, x3) => {
                ensure!(
                    allow_const_ext,
                    "WebGPU base-field eval_check does not support extension constants"
                );
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                push_instr([
                    WEBGPU_EVAL_OP_CONST_EXT,
                    u32::try_from(out_slot).expect("eval_check fp slot exceeds u32"),
                    elem_const_word(*x0),
                    elem_const_word(*x1),
                    elem_const_word(*x2),
                    elem_const_word(*x3),
                    0,
                    0,
                ]);
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::Get(tap_idx) => {
                let tap = tap_info
                    .get(*tap_idx)
                    .ok_or_else(|| anyhow!("poly_ext tap index {tap_idx} is out of range"))?;
                ensure!(tap.group < 3, "WebGPU eval_check only supports 3 groups");
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                push_instr([
                    WEBGPU_EVAL_OP_GET,
                    u32::try_from(out_slot).expect("eval_check fp slot exceeds u32"),
                    u32::try_from(tap.group).expect("eval_check tap group exceeds u32"),
                    u32::try_from(tap.offset).expect("eval_check tap offset exceeds u32"),
                    u32::try_from(tap.back * INV_RATE).expect("eval_check tap back exceeds u32"),
                    0,
                    0,
                    0,
                ]);
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::GetGlobal(arg, offset) => {
                ensure!(*arg < 2, "WebGPU eval_check only supports 2 global buffers");
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                push_instr([
                    WEBGPU_EVAL_OP_GET_GLOBAL,
                    u32::try_from(out_slot).expect("eval_check fp slot exceeds u32"),
                    u32::try_from(*arg).expect("eval_check global arg exceeds u32"),
                    u32::try_from(*offset).expect("eval_check global offset exceeds u32"),
                    0,
                    0,
                    0,
                    0,
                ]);
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::Add(lhs, rhs) => {
                let lhs_slot = eval_check_fp_slot(&fp_slots, *lhs)?;
                let rhs_slot = eval_check_fp_slot(&fp_slots, *rhs)?;
                used_fp.extend([*lhs, *rhs]);
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                push_instr([
                    WEBGPU_EVAL_OP_ADD,
                    u32::try_from(out_slot).expect("eval_check fp slot exceeds u32"),
                    u32::try_from(lhs_slot).expect("eval_check fp slot exceeds u32"),
                    u32::try_from(rhs_slot).expect("eval_check fp slot exceeds u32"),
                    0,
                    0,
                    0,
                    0,
                ]);
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::Sub(lhs, rhs) => {
                let lhs_slot = eval_check_fp_slot(&fp_slots, *lhs)?;
                let rhs_slot = eval_check_fp_slot(&fp_slots, *rhs)?;
                used_fp.extend([*lhs, *rhs]);
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                push_instr([
                    WEBGPU_EVAL_OP_SUB,
                    u32::try_from(out_slot).expect("eval_check fp slot exceeds u32"),
                    u32::try_from(lhs_slot).expect("eval_check fp slot exceeds u32"),
                    u32::try_from(rhs_slot).expect("eval_check fp slot exceeds u32"),
                    0,
                    0,
                    0,
                    0,
                ]);
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::Mul(lhs, rhs) => {
                let lhs_slot = eval_check_fp_slot(&fp_slots, *lhs)?;
                let rhs_slot = eval_check_fp_slot(&fp_slots, *rhs)?;
                used_fp.extend([*lhs, *rhs]);
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                push_instr([
                    WEBGPU_EVAL_OP_MUL,
                    u32::try_from(out_slot).expect("eval_check fp slot exceeds u32"),
                    u32::try_from(lhs_slot).expect("eval_check fp slot exceeds u32"),
                    u32::try_from(rhs_slot).expect("eval_check fp slot exceeds u32"),
                    0,
                    0,
                    0,
                    0,
                ]);
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::True => {
                let out_idx = mix_slots.len();
                let out_slot = mix_alloc.alloc();
                mix_slots.push(Some(out_slot));
                push_instr([
                    WEBGPU_EVAL_OP_TRUE,
                    u32::try_from(out_slot).expect("eval_check mix slot exceeds u32"),
                    u32::try_from(out_idx).expect("eval_check mix index exceeds u32"),
                    0,
                    0,
                    0,
                    0,
                    0,
                ]);
                if last_mix.get(out_idx).copied().flatten().is_none() {
                    mix_slots[out_idx] = None;
                    mix_alloc.free(out_slot);
                }
            }
            PolyExtStep::AndEqz(chain, inner) => {
                let chain_slot = eval_check_mix_slot(&mix_slots, *chain)?;
                let inner_slot = eval_check_fp_slot(&fp_slots, *inner)?;
                used_mix.push(*chain);
                used_fp.push(*inner);
                let out_idx = mix_slots.len();
                let out_slot = mix_alloc.alloc();
                mix_slots.push(Some(out_slot));
                push_instr([
                    WEBGPU_EVAL_OP_AND_EQZ,
                    u32::try_from(out_slot).expect("eval_check mix slot exceeds u32"),
                    u32::try_from(chain_slot).expect("eval_check mix slot exceeds u32"),
                    u32::try_from(inner_slot).expect("eval_check fp slot exceeds u32"),
                    u32::try_from(out_idx).expect("eval_check mix index exceeds u32"),
                    0,
                    0,
                    0,
                ]);
                if last_mix.get(out_idx).copied().flatten().is_none() {
                    mix_slots[out_idx] = None;
                    mix_alloc.free(out_slot);
                }
            }
            PolyExtStep::AndCond(chain, cond, inner) => {
                let chain_slot = eval_check_mix_slot(&mix_slots, *chain)?;
                let cond_slot = eval_check_fp_slot(&fp_slots, *cond)?;
                let inner_slot = eval_check_mix_slot(&mix_slots, *inner)?;
                used_mix.extend([*chain, *inner]);
                used_fp.push(*cond);
                let out_idx = mix_slots.len();
                let out_slot = mix_alloc.alloc();
                mix_slots.push(Some(out_slot));
                push_instr([
                    WEBGPU_EVAL_OP_AND_COND,
                    u32::try_from(out_slot).expect("eval_check mix slot exceeds u32"),
                    u32::try_from(chain_slot).expect("eval_check mix slot exceeds u32"),
                    u32::try_from(cond_slot).expect("eval_check fp slot exceeds u32"),
                    u32::try_from(inner_slot).expect("eval_check mix slot exceeds u32"),
                    u32::try_from(out_idx).expect("eval_check mix index exceeds u32"),
                    0,
                    0,
                ]);
                if last_mix.get(out_idx).copied().flatten().is_none() {
                    mix_slots[out_idx] = None;
                    mix_alloc.free(out_slot);
                }
            }
        }

        used_fp.sort_unstable();
        used_fp.dedup();
        used_mix.sort_unstable();
        used_mix.dedup();

        for var in used_fp {
            if last_fp.get(var).copied().flatten() == Some(op_idx) {
                let slot = eval_check_fp_slot(&fp_slots, var)?;
                fp_slots[var] = None;
                fp_alloc.free(slot);
            }
        }
        for var in used_mix {
            if last_mix.get(var).copied().flatten() == Some(op_idx) {
                let slot = eval_check_mix_slot(&mix_slots, var)?;
                mix_slots[var] = None;
                mix_alloc.free(slot);
            }
        }
    }

    let ret_slot = eval_check_mix_slot(&mix_slots, def.ret)?;
    ensure!(
        fp_alloc.max_used() <= max_fp_slots,
        "WebGPU interpreted eval_check needs {} FP slots, max is {}",
        fp_alloc.max_used(),
        max_fp_slots
    );
    ensure!(
        mix_alloc.max_used() <= WEBGPU_EVAL_CHECK_MAX_MIX_SLOTS,
        "WebGPU interpreted eval_check needs {} mix slots, max is {}",
        mix_alloc.max_used(),
        WEBGPU_EVAL_CHECK_MAX_MIX_SLOTS
    );

    Ok((instructions, fp_alloc.max_used(), mix_alloc.max_used(), ret_slot))
}

fn eval_check_interpreter_instructions(
    taps: &TapSet<'_>,
    def: &PolyExtStepDef,
) -> Result<(Vec<u32>, usize, usize, usize)> {
    eval_check_interpreter_instructions_with_limit(taps, def, WEBGPU_EVAL_CHECK_MAX_FP_SLOTS, true)
}

fn eval_check_base_interpreter_instructions(
    taps: &TapSet<'_>,
    def: &PolyExtStepDef,
) -> Result<(Vec<u32>, usize, usize, usize)> {
    eval_check_interpreter_instructions_with_limit(
        taps,
        def,
        WEBGPU_EVAL_CHECK_MAX_FP_ELEM_SLOTS,
        false,
    )
}

const EVAL_CHECK_WGSL_PREFIX: &str = r#"
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
    _pad0: u32,
    invs: vec4<u32>,
};

@group(0) @binding(0) var<storage, read_write> check: ElemBuffer;
@group(0) @binding(1) var<storage, read> group0: ElemBuffer;
@group(0) @binding(2) var<storage, read> group1: ElemBuffer;
@group(0) @binding(3) var<storage, read> group2: ElemBuffer;
@group(0) @binding(4) var<storage, read> global0: ElemBuffer;
@group(0) @binding(5) var<storage, read> global1: ElemBuffer;
@group(0) @binding(6) var<storage, read> mix_pows: ElemBuffer;
@group(0) @binding(7) var<uniform> params: Params;

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

fn ext_scale(lhs: vec4<u32>, rhs: u32) -> vec4<u32> {
    return vec4<u32>(
        mul(lhs.x, rhs),
        mul(lhs.y, rhs),
        mul(lhs.z, rhs),
        mul(lhs.w, rhs),
    );
}

fn load_mix_pow(idx: u32) -> vec4<u32> {
    let base = idx * 4u;
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

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cycle = gid.x + gid.y * 16776960u;
    if (cycle >= params.domain) {
        return;
    }
"#;

const EVAL_CHECK_INTERPRETER_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const INSTRUCTION_WORDS: u32 = 8u;
const LINEAR_DISPATCH_STRIDE: u32 = 2097120u;

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
    instr_count: u32,
    instr_base: u32,
    mix_pows_base: u32,
    ret_mix_slot: u32,
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
@group(0) @binding(6) var<storage, read> instrs: ElemBuffer;
@group(0) @binding(7) var<storage, read> mix_pows: ElemBuffer;
@group(0) @binding(8) var<uniform> params: Params;

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

fn instr_word(op_idx: u32, word_idx: u32) -> u32 {
    return instrs.data[params.instr_base + op_idx * INSTRUCTION_WORDS + word_idx];
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

@compute @workgroup_size(32)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let local_cycle = gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
    if (local_cycle >= params.dispatch_count) {
        return;
    }
    let cycle = params.cycle_base + local_cycle;
    if (cycle >= params.domain) {
        return;
    }

    var fp: array<vec4<u32>, {FP_SLOTS}>;
    var mix_tot: array<vec4<u32>, {MIX_SLOTS}>;
    var mix_mul: array<vec4<u32>, {MIX_SLOTS}>;

    for (var op_idx = 0u; op_idx < params.instr_count; op_idx = op_idx + 1u) {
        let op = instr_word(op_idx, 0u);
        switch (op) {
            case 0u: {
                fp[instr_word(op_idx, 1u)] =
                    vec4<u32>(instr_word(op_idx, 2u), 0u, 0u, 0u);
            }
            case 1u: {
                fp[instr_word(op_idx, 1u)] = vec4<u32>(
                    instr_word(op_idx, 2u),
                    instr_word(op_idx, 3u),
                    instr_word(op_idx, 4u),
                    instr_word(op_idx, 5u),
                );
            }
            case 2u: {
                let out = instr_word(op_idx, 1u);
                let group_id = instr_word(op_idx, 2u);
                let offset = instr_word(op_idx, 3u);
                let back = instr_word(op_idx, 4u);
                let row = (cycle + params.domain - (back % params.domain)) % params.domain;
                var value = 0u;
                if (group_id == 0u) {
                    let local_row =
                        (row + params.domain - (params.group0_chunk_base % params.domain)) %
                        params.domain;
                    value = group0.data[
                        params.group0_base + offset * params.group0_chunk_rows + local_row
                    ];
                } else if (group_id == 1u) {
                    let local_row =
                        (row + params.domain - (params.group1_chunk_base % params.domain)) %
                        params.domain;
                    value = group1.data[
                        params.group1_base + offset * params.group1_chunk_rows + local_row
                    ];
                } else {
                    let local_row =
                        (row + params.domain - (params.group2_chunk_base % params.domain)) %
                        params.domain;
                    value = group2.data[
                        params.group2_base + offset * params.group2_chunk_rows + local_row
                    ];
                }
                fp[out] = vec4<u32>(value, 0u, 0u, 0u);
            }
            case 3u: {
                let out = instr_word(op_idx, 1u);
                let arg = instr_word(op_idx, 2u);
                let offset = instr_word(op_idx, 3u);
                var value = 0u;
                if (arg == 0u) {
                    value = global0.data[params.global0_base + offset];
                } else {
                    value = global1.data[params.global1_base + offset];
                }
                fp[out] = vec4<u32>(value, 0u, 0u, 0u);
            }
            case 4u: {
                fp[instr_word(op_idx, 1u)] =
                    ext_add(fp[instr_word(op_idx, 2u)], fp[instr_word(op_idx, 3u)]);
            }
            case 5u: {
                fp[instr_word(op_idx, 1u)] =
                    ext_sub(fp[instr_word(op_idx, 2u)], fp[instr_word(op_idx, 3u)]);
            }
            case 6u: {
                fp[instr_word(op_idx, 1u)] =
                    ext_mul(fp[instr_word(op_idx, 2u)], fp[instr_word(op_idx, 3u)]);
            }
            case 7u: {
                let out = instr_word(op_idx, 1u);
                mix_tot[out] = vec4<u32>(0u, 0u, 0u, 0u);
                mix_mul[out] = load_mix_pow(instr_word(op_idx, 2u));
            }
            case 8u: {
                let out = instr_word(op_idx, 1u);
                let chain = instr_word(op_idx, 2u);
                let inner = instr_word(op_idx, 3u);
                mix_tot[out] = ext_add(mix_tot[chain], ext_mul(mix_mul[chain], fp[inner]));
                mix_mul[out] = load_mix_pow(instr_word(op_idx, 4u));
            }
            case 9u: {
                let out = instr_word(op_idx, 1u);
                let chain = instr_word(op_idx, 2u);
                let cond = instr_word(op_idx, 3u);
                let inner = instr_word(op_idx, 4u);
                mix_tot[out] =
                    ext_add(mix_tot[chain], ext_mul(ext_mul(fp[cond], mix_tot[inner]), mix_mul[chain]));
                mix_mul[out] = load_mix_pow(instr_word(op_idx, 5u));
            }
            default: {}
        }
    }

    let result = ext_mul(
        mix_tot[params.ret_mix_slot],
        vec4<u32>(zerofier_inv(cycle), 0u, 0u, 0u),
    );
    check.data[params.check_base + 0u * params.domain + cycle] = result.x;
    check.data[params.check_base + 1u * params.domain + cycle] = result.y;
    check.data[params.check_base + 2u * params.domain + cycle] = result.z;
    check.data[params.check_base + 3u * params.domain + cycle] = result.w;
}
"#;

fn build_eval_check_interpreter_wgsl(fp_slots: usize, mix_slots: usize) -> String {
    EVAL_CHECK_INTERPRETER_WGSL
        .replace("{FP_SLOTS}", &fp_slots.max(1).to_string())
        .replace("{MIX_SLOTS}", &mix_slots.max(1).to_string())
}

const EVAL_CHECK_BASE_INTERPRETER_WGSL: &str = r#"
const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const NBETA: u32 = 1073741848u;
const INSTRUCTION_WORDS: u32 = 8u;
const LINEAR_DISPATCH_STRIDE: u32 = {LINEAR_DISPATCH_STRIDE}u;

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
    instr_count: u32,
    instr_base: u32,
    mix_pows_base: u32,
    ret_mix_slot: u32,
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
@group(0) @binding(6) var<storage, read> instrs: ElemBuffer;
@group(0) @binding(7) var<storage, read> mix_pows: ElemBuffer;
@group(0) @binding(8) var<uniform> params: Params;

{BASE_SCRATCH_DECLS}

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

fn instr_word(op_idx: u32, word_idx: u32) -> u32 {
    return instrs.data[params.instr_base + op_idx * INSTRUCTION_WORDS + word_idx];
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

@compute @workgroup_size({WORKGROUP_SIZE})
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let local_cycle = gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
    if (local_cycle >= params.dispatch_count) {
        return;
    }
    let cycle = params.cycle_base + local_cycle;
    if (cycle >= params.domain) {
        return;
    }

{BASE_LOCAL_DECLS}

    let lane = {LANE_INDEX};
    for (var op_idx = 0u; op_idx < params.instr_count; op_idx = op_idx + 1u) {
        let op = instr_word(op_idx, 0u);
        switch (op) {
            case 0u: {
                fp[lane][instr_word(op_idx, 1u)] = instr_word(op_idx, 2u);
            }
            case 2u: {
                let out = instr_word(op_idx, 1u);
                let group_id = instr_word(op_idx, 2u);
                let offset = instr_word(op_idx, 3u);
                let back = instr_word(op_idx, 4u);
                let row = (cycle + params.domain - (back % params.domain)) % params.domain;
                var value = 0u;
                if (group_id == 0u) {
                    let local_row =
                        (row + params.domain - (params.group0_chunk_base % params.domain)) %
                        params.domain;
                    value = group0.data[
                        params.group0_base + offset * params.group0_chunk_rows + local_row
                    ];
                } else if (group_id == 1u) {
                    let local_row =
                        (row + params.domain - (params.group1_chunk_base % params.domain)) %
                        params.domain;
                    value = group1.data[
                        params.group1_base + offset * params.group1_chunk_rows + local_row
                    ];
                } else {
                    let local_row =
                        (row + params.domain - (params.group2_chunk_base % params.domain)) %
                        params.domain;
                    value = group2.data[
                        params.group2_base + offset * params.group2_chunk_rows + local_row
                    ];
                }
                fp[lane][out] = value;
            }
            case 3u: {
                let out = instr_word(op_idx, 1u);
                let arg = instr_word(op_idx, 2u);
                let offset = instr_word(op_idx, 3u);
                var value = 0u;
                if (arg == 0u) {
                    value = global0.data[params.global0_base + offset];
                } else {
                    value = global1.data[params.global1_base + offset];
                }
                fp[lane][out] = value;
            }
            case 4u: {
                fp[lane][instr_word(op_idx, 1u)] =
                    add(fp[lane][instr_word(op_idx, 2u)], fp[lane][instr_word(op_idx, 3u)]);
            }
            case 5u: {
                fp[lane][instr_word(op_idx, 1u)] =
                    sub(fp[lane][instr_word(op_idx, 2u)], fp[lane][instr_word(op_idx, 3u)]);
            }
            case 6u: {
                fp[lane][instr_word(op_idx, 1u)] =
                    mul(fp[lane][instr_word(op_idx, 2u)], fp[lane][instr_word(op_idx, 3u)]);
            }
            case 7u: {
                let out = instr_word(op_idx, 1u);
                mix_tot[lane][out] = vec4<u32>(0u, 0u, 0u, 0u);
                mix_mul[lane][out] = load_mix_pow(instr_word(op_idx, 2u));
            }
            case 8u: {
                let out = instr_word(op_idx, 1u);
                let chain = instr_word(op_idx, 2u);
                let inner = instr_word(op_idx, 3u);
                mix_tot[lane][out] = ext_add(
                    mix_tot[lane][chain],
                    ext_scale(mix_mul[lane][chain], fp[lane][inner]),
                );
                mix_mul[lane][out] = load_mix_pow(instr_word(op_idx, 4u));
            }
            case 9u: {
                let out = instr_word(op_idx, 1u);
                let chain = instr_word(op_idx, 2u);
                let cond = instr_word(op_idx, 3u);
                let inner = instr_word(op_idx, 4u);
                mix_tot[lane][out] = ext_add(
                    mix_tot[lane][chain],
                    ext_scale(ext_mul(mix_tot[lane][inner], mix_mul[lane][chain]), fp[lane][cond]),
                );
                mix_mul[lane][out] = load_mix_pow(instr_word(op_idx, 5u));
            }
            default: {}
        }
    }

    let result = ext_scale(mix_tot[lane][params.ret_mix_slot], zerofier_inv(cycle));
    check.data[params.check_base + 0u * params.domain + cycle] = result.x;
    check.data[params.check_base + 1u * params.domain + cycle] = result.y;
    check.data[params.check_base + 2u * params.domain + cycle] = result.z;
    check.data[params.check_base + 3u * params.domain + cycle] = result.w;
}
"#;

fn build_eval_check_base_interpreter_wgsl(
    fp_slots: usize,
    mix_slots: usize,
    private_parallel: bool,
    workgroup_size: u32,
) -> String {
    let fp_slots = fp_slots.max(1);
    let mix_slots = mix_slots.max(1);
    let scratch_lanes = if private_parallel { 1 } else { workgroup_size };
    let scratch_decls = if private_parallel {
        String::new()
    } else {
        format!(
            "var<workgroup> fp: array<array<u32, {fp_slots}>, {scratch_lanes}>;\nvar<workgroup> mix_tot: array<array<vec4<u32>, {mix_slots}>, {scratch_lanes}>;\nvar<workgroup> mix_mul: array<array<vec4<u32>, {mix_slots}>, {scratch_lanes}>;"
        )
    };
    let local_decls = if private_parallel {
        format!(
            "    var fp: array<array<u32, {fp_slots}>, 1>;\n    var mix_tot: array<array<vec4<u32>, {mix_slots}>, 1>;\n    var mix_mul: array<array<vec4<u32>, {mix_slots}>, 1>;"
        )
    } else {
        String::new()
    };
    let lane_index = if private_parallel { "0u" } else { "lid.x" };
    let linear_dispatch_stride = workgroup_size * WEBGPU_MAX_WORKGROUPS_PER_DIMENSION;

    EVAL_CHECK_BASE_INTERPRETER_WGSL
        .replace("{FP_SLOTS}", &fp_slots.to_string())
        .replace("{MIX_SLOTS}", &mix_slots.to_string())
        .replace("{BASE_SCRATCH_DECLS}", &scratch_decls)
        .replace("{BASE_LOCAL_DECLS}", &local_decls)
        .replace("{LANE_INDEX}", lane_index)
        .replace("{WORKGROUP_SIZE}", &workgroup_size.to_string())
        .replace(
            "{LINEAR_DISPATCH_STRIDE}",
            &linear_dispatch_stride.to_string(),
        )
}

#[allow(dead_code)]
fn build_eval_check_wgsl_unrolled(taps: &TapSet<'_>, def: &PolyExtStepDef) -> Result<String> {
    let tap_info: Vec<_> = taps
        .taps()
        .map(|tap| EvalCheckTap {
            group: tap.group(),
            offset: tap.offset(),
            back: tap.back(),
        })
        .collect();
    let mut wgsl = String::from(
        r#"
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
    _pad0: u32,
    invs: vec4<u32>,
};

@group(0) @binding(0) var<storage, read_write> check: ElemBuffer;
@group(0) @binding(1) var<storage, read> group0: ElemBuffer;
@group(0) @binding(2) var<storage, read> group1: ElemBuffer;
@group(0) @binding(3) var<storage, read> group2: ElemBuffer;
@group(0) @binding(4) var<storage, read> global0: ElemBuffer;
@group(0) @binding(5) var<storage, read> global1: ElemBuffer;
@group(0) @binding(6) var<storage, read> mix_pows: ElemBuffer;
@group(0) @binding(7) var<uniform> params: Params;

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

fn load_mix_pow(idx: u32) -> vec4<u32> {
    let base = idx * 4u;
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

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cycle = gid.x + gid.y * 16776960u;
    if (cycle >= params.domain) {
        return;
    }
"#,
    );

    let mut fp_count = 0usize;
    let mut mix_count = 0usize;
    for op in def.block {
        match op {
            PolyExtStep::Const(value) => {
                let word = elem_const_word(*value);
                let _ = writeln!(
                    wgsl,
                    "    let f{fp_count} = vec4<u32>({word}u, 0u, 0u, 0u);"
                );
                fp_count += 1;
            }
            PolyExtStep::ConstExt(x0, x1, x2, x3) => {
                let value = eval_check_ext_const([
                    elem_const_word(*x0),
                    elem_const_word(*x1),
                    elem_const_word(*x2),
                    elem_const_word(*x3),
                ]);
                let _ = writeln!(wgsl, "    let f{fp_count} = {value};");
                fp_count += 1;
            }
            PolyExtStep::Get(tap_idx) => {
                let tap = tap_info
                    .get(*tap_idx)
                    .ok_or_else(|| anyhow!("poly_ext tap index {tap_idx} is out of range"))?;
                ensure!(tap.group < 3, "WebGPU eval_check only supports 3 groups");
                let back = tap.back * INV_RATE;
                let _ = writeln!(
                    wgsl,
                    "    let f{fp_count}_row = (cycle + params.domain - ({back}u % params.domain)) % params.domain;"
                );
                let _ = writeln!(
                    wgsl,
                    "    let f{fp_count} = vec4<u32>(group{}.data[params.group{}_base + {}u * params.domain + f{fp_count}_row], 0u, 0u, 0u);",
                    tap.group, tap.group, tap.offset
                );
                fp_count += 1;
            }
            PolyExtStep::GetGlobal(arg, offset) => {
                ensure!(*arg < 2, "WebGPU eval_check only supports 2 global buffers");
                let _ = writeln!(
                    wgsl,
                    "    let f{fp_count} = vec4<u32>(global{arg}.data[params.global{arg}_base + {offset}u], 0u, 0u, 0u);"
                );
                fp_count += 1;
            }
            PolyExtStep::Add(lhs, rhs) => {
                let _ = writeln!(wgsl, "    let f{fp_count} = ext_add(f{lhs}, f{rhs});");
                fp_count += 1;
            }
            PolyExtStep::Sub(lhs, rhs) => {
                let _ = writeln!(wgsl, "    let f{fp_count} = ext_sub(f{lhs}, f{rhs});");
                fp_count += 1;
            }
            PolyExtStep::Mul(lhs, rhs) => {
                let _ = writeln!(wgsl, "    let f{fp_count} = ext_mul(f{lhs}, f{rhs});");
                fp_count += 1;
            }
            PolyExtStep::True => {
                let _ = writeln!(
                    wgsl,
                    "    let m{mix_count}_tot = vec4<u32>(0u, 0u, 0u, 0u);"
                );
                let _ = writeln!(
                    wgsl,
                    "    let m{mix_count}_mul = load_mix_pow({mix_count}u);"
                );
                mix_count += 1;
            }
            PolyExtStep::AndEqz(chain, inner) => {
                let _ = writeln!(
                    wgsl,
                    "    let m{mix_count}_tot = ext_add(m{chain}_tot, ext_mul(m{chain}_mul, f{inner}));"
                );
                let _ = writeln!(
                    wgsl,
                    "    let m{mix_count}_mul = load_mix_pow({mix_count}u);"
                );
                mix_count += 1;
            }
            PolyExtStep::AndCond(chain, cond, inner) => {
                let _ = writeln!(
                    wgsl,
                    "    let m{mix_count}_tot = ext_add(m{chain}_tot, ext_mul(ext_mul(f{cond}, m{inner}_tot), m{chain}_mul));"
                );
                let _ = writeln!(
                    wgsl,
                    "    let m{mix_count}_mul = load_mix_pow({mix_count}u);"
                );
                mix_count += 1;
            }
        }
    }

    ensure!(
        def.ret < mix_count,
        "poly_ext return mix index {} exceeds generated mix count {}",
        def.ret,
        mix_count
    );
    let _ = writeln!(
        wgsl,
        "    let result = ext_mul(m{}_tot, vec4<u32>(zerofier_inv(cycle), 0u, 0u, 0u));",
        def.ret
    );
    wgsl.push_str(
        r#"
    check.data[params.check_base + 0u * params.domain + cycle] = result.x;
    check.data[params.check_base + 1u * params.domain + cycle] = result.y;
    check.data[params.check_base + 2u * params.domain + cycle] = result.z;
    check.data[params.check_base + 3u * params.domain + cycle] = result.w;
}
"#,
    );
    Ok(wgsl)
}

fn build_eval_check_wgsl(taps: &TapSet<'_>, def: &PolyExtStepDef) -> Result<String> {
    let tap_info: Vec<_> = taps
        .taps()
        .map(|tap| EvalCheckTap {
            group: tap.group(),
            offset: tap.offset(),
            back: tap.back(),
        })
        .collect();
    let (last_fp, last_mix, _fp_count, _mix_count) = eval_check_last_uses(def)?;
    let mut fp_alloc = EvalCheckSlotAllocator::default();
    let mut mix_alloc = EvalCheckSlotAllocator::default();
    let mut fp_slots: Vec<Option<usize>> = Vec::new();
    let mut mix_slots: Vec<Option<usize>> = Vec::new();
    let mut body = String::new();

    for (op_idx, op) in def.block.iter().enumerate() {
        let mut used_fp = Vec::new();
        let mut used_mix = Vec::new();
        match op {
            PolyExtStep::Const(value) => {
                let word = elem_const_word(*value);
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                let _ = writeln!(body, "    f{out_slot} = vec4<u32>({word}u, 0u, 0u, 0u);");
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::ConstExt(x0, x1, x2, x3) => {
                let value = eval_check_ext_const([
                    elem_const_word(*x0),
                    elem_const_word(*x1),
                    elem_const_word(*x2),
                    elem_const_word(*x3),
                ]);
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                let _ = writeln!(body, "    f{out_slot} = {value};");
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::Get(tap_idx) => {
                let tap = tap_info
                    .get(*tap_idx)
                    .ok_or_else(|| anyhow!("poly_ext tap index {tap_idx} is out of range"))?;
                ensure!(tap.group < 3, "WebGPU eval_check only supports 3 groups");
                let back = tap.back * INV_RATE;
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                let _ = writeln!(
                    body,
                    "    let f{out_idx}_row = (cycle + params.domain - ({back}u % params.domain)) % params.domain;"
                );
                let _ = writeln!(
                    body,
                    "    f{out_slot} = vec4<u32>(group{}.data[params.group{}_base + {}u * params.domain + f{out_idx}_row], 0u, 0u, 0u);",
                    tap.group, tap.group, tap.offset
                );
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::GetGlobal(arg, offset) => {
                ensure!(*arg < 2, "WebGPU eval_check only supports 2 global buffers");
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                let _ = writeln!(
                    body,
                    "    f{out_slot} = vec4<u32>(global{arg}.data[params.global{arg}_base + {offset}u], 0u, 0u, 0u);"
                );
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::Add(lhs, rhs) => {
                let lhs_slot = eval_check_fp_slot(&fp_slots, *lhs)?;
                let rhs_slot = eval_check_fp_slot(&fp_slots, *rhs)?;
                used_fp.extend([*lhs, *rhs]);
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                let _ = writeln!(body, "    f{out_slot} = ext_add(f{lhs_slot}, f{rhs_slot});");
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::Sub(lhs, rhs) => {
                let lhs_slot = eval_check_fp_slot(&fp_slots, *lhs)?;
                let rhs_slot = eval_check_fp_slot(&fp_slots, *rhs)?;
                used_fp.extend([*lhs, *rhs]);
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                let _ = writeln!(body, "    f{out_slot} = ext_sub(f{lhs_slot}, f{rhs_slot});");
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::Mul(lhs, rhs) => {
                let lhs_slot = eval_check_fp_slot(&fp_slots, *lhs)?;
                let rhs_slot = eval_check_fp_slot(&fp_slots, *rhs)?;
                used_fp.extend([*lhs, *rhs]);
                let out_idx = fp_slots.len();
                let out_slot = fp_alloc.alloc();
                fp_slots.push(Some(out_slot));
                let _ = writeln!(body, "    f{out_slot} = ext_mul(f{lhs_slot}, f{rhs_slot});");
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    fp_alloc.free(out_slot);
                }
            }
            PolyExtStep::True => {
                let out_idx = mix_slots.len();
                let out_slot = mix_alloc.alloc();
                mix_slots.push(Some(out_slot));
                let _ = writeln!(body, "    m{out_slot}_tot = vec4<u32>(0u, 0u, 0u, 0u);");
                let _ = writeln!(body, "    m{out_slot}_mul = load_mix_pow({out_idx}u);");
                if last_mix.get(out_idx).copied().flatten().is_none() {
                    mix_slots[out_idx] = None;
                    mix_alloc.free(out_slot);
                }
            }
            PolyExtStep::AndEqz(chain, inner) => {
                let chain_slot = eval_check_mix_slot(&mix_slots, *chain)?;
                let inner_slot = eval_check_fp_slot(&fp_slots, *inner)?;
                used_mix.push(*chain);
                used_fp.push(*inner);
                let out_idx = mix_slots.len();
                let out_slot = mix_alloc.alloc();
                mix_slots.push(Some(out_slot));
                let _ = writeln!(
                    body,
                    "    m{out_slot}_tot = ext_add(m{chain_slot}_tot, ext_mul(m{chain_slot}_mul, f{inner_slot}));"
                );
                let _ = writeln!(body, "    m{out_slot}_mul = load_mix_pow({out_idx}u);");
                if last_mix.get(out_idx).copied().flatten().is_none() {
                    mix_slots[out_idx] = None;
                    mix_alloc.free(out_slot);
                }
            }
            PolyExtStep::AndCond(chain, cond, inner) => {
                let chain_slot = eval_check_mix_slot(&mix_slots, *chain)?;
                let cond_slot = eval_check_fp_slot(&fp_slots, *cond)?;
                let inner_slot = eval_check_mix_slot(&mix_slots, *inner)?;
                used_mix.extend([*chain, *inner]);
                used_fp.push(*cond);
                let out_idx = mix_slots.len();
                let out_slot = mix_alloc.alloc();
                mix_slots.push(Some(out_slot));
                let _ = writeln!(
                    body,
                    "    m{out_slot}_tot = ext_add(m{chain_slot}_tot, ext_mul(ext_mul(f{cond_slot}, m{inner_slot}_tot), m{chain_slot}_mul));"
                );
                let _ = writeln!(body, "    m{out_slot}_mul = load_mix_pow({out_idx}u);");
                if last_mix.get(out_idx).copied().flatten().is_none() {
                    mix_slots[out_idx] = None;
                    mix_alloc.free(out_slot);
                }
            }
        }

        used_fp.sort_unstable();
        used_fp.dedup();
        used_mix.sort_unstable();
        used_mix.dedup();

        for var in used_fp {
            if last_fp.get(var).copied().flatten() == Some(op_idx) {
                let slot = eval_check_fp_slot(&fp_slots, var)?;
                fp_slots[var] = None;
                fp_alloc.free(slot);
            }
        }
        for var in used_mix {
            if last_mix.get(var).copied().flatten() == Some(op_idx) {
                let slot = eval_check_mix_slot(&mix_slots, var)?;
                mix_slots[var] = None;
                mix_alloc.free(slot);
            }
        }
    }

    let ret_slot = eval_check_mix_slot(&mix_slots, def.ret)?;
    ensure!(
        fp_alloc.max_used() <= WEBGPU_EVAL_CHECK_MAX_FP_SLOTS,
        "WebGPU eval_check needs {} FP slots, max is {}",
        fp_alloc.max_used(),
        WEBGPU_EVAL_CHECK_MAX_FP_SLOTS
    );
    ensure!(
        mix_alloc.max_used() <= WEBGPU_EVAL_CHECK_MAX_MIX_SLOTS,
        "WebGPU eval_check needs {} mix slots, max is {}",
        mix_alloc.max_used(),
        WEBGPU_EVAL_CHECK_MAX_MIX_SLOTS
    );

    let mut wgsl = String::from(EVAL_CHECK_WGSL_PREFIX);
    let fp_slots = fp_alloc.max_used().max(1);
    let mix_slots = mix_alloc.max_used().max(1);
    for slot in 0..fp_slots {
        let _ = writeln!(wgsl, "    var f{slot}: vec4<u32>;");
    }
    for slot in 0..mix_slots {
        let _ = writeln!(wgsl, "    var m{slot}_tot: vec4<u32>;");
        let _ = writeln!(wgsl, "    var m{slot}_mul: vec4<u32>;");
    }
    wgsl.push_str(&body);
    let _ = writeln!(
        wgsl,
        "    let result = ext_mul(m{ret_slot}_tot, vec4<u32>(zerofier_inv(cycle), 0u, 0u, 0u));"
    );
    wgsl.push_str(
        r#"
    check.data[params.check_base + 0u * params.domain + cycle] = result.x;
    check.data[params.check_base + 1u * params.domain + cycle] = result.y;
    check.data[params.check_base + 2u * params.domain + cycle] = result.z;
    check.data[params.check_base + 3u * params.domain + cycle] = result.w;
}
"#,
    );
    Ok(wgsl)
}

fn build_eval_check_split_wgsl(
    taps: &TapSet<'_>,
    program: &EvalCheckProgram,
    terms: &[EvalCheckTerm],
    reset_check: bool,
) -> Result<String> {
    let needed = eval_check_needed_fp_vars(program, terms)?;
    ensure!(
        needed.len() <= WEBGPU_EVAL_CHECK_MAX_POLY_EXT_STEPS,
        "WebGPU split eval_check chunk needs {} FP ops, max is {}",
        needed.len(),
        WEBGPU_EVAL_CHECK_MAX_POLY_EXT_STEPS
    );

    let needed_set: HashSet<_> = needed.iter().copied().collect();
    let tap_info: Vec<_> = taps
        .taps()
        .map(|tap| EvalCheckTap {
            group: tap.group(),
            offset: tap.offset(),
            back: tap.back(),
        })
        .collect();

    let mut last_fp = vec![None; program.fp_ops.len()];
    for (op_pos, var) in needed.iter().copied().enumerate() {
        match program.fp_ops[var] {
            EvalCheckFpOp::Add(lhs, rhs)
            | EvalCheckFpOp::Sub(lhs, rhs)
            | EvalCheckFpOp::Mul(lhs, rhs) => {
                ensure!(
                    needed_set.contains(&lhs) && needed_set.contains(&rhs),
                    "split eval_check missing FP dependency"
                );
                eval_check_note_last(&mut last_fp, lhs, op_pos);
                eval_check_note_last(&mut last_fp, rhs, op_pos);
            }
            _ => {}
        }
    }
    let term_base = needed.len();
    for (term_idx, term) in terms.iter().enumerate() {
        let op_pos = term_base + term_idx;
        eval_check_note_last(&mut last_fp, term.inner, op_pos);
        for cond in &term.conds {
            eval_check_note_last(&mut last_fp, *cond, op_pos);
        }
    }

    let mut fp_alloc = EvalCheckSlotAllocator::default();
    let mut fp_slots = vec![None; program.fp_ops.len()];
    let mut body = String::new();

    for (op_pos, var) in needed.iter().copied().enumerate() {
        let mut used_fp = Vec::new();
        match program.fp_ops[var] {
            EvalCheckFpOp::Const(value) => {
                let word = elem_const_word(value);
                let out_slot = fp_alloc.alloc();
                fp_slots[var] = Some(out_slot);
                let _ = writeln!(body, "    f{out_slot} = vec4<u32>({word}u, 0u, 0u, 0u);");
            }
            EvalCheckFpOp::ConstExt(x0, x1, x2, x3) => {
                let value = eval_check_ext_const([
                    elem_const_word(x0),
                    elem_const_word(x1),
                    elem_const_word(x2),
                    elem_const_word(x3),
                ]);
                let out_slot = fp_alloc.alloc();
                fp_slots[var] = Some(out_slot);
                let _ = writeln!(body, "    f{out_slot} = {value};");
            }
            EvalCheckFpOp::Get(tap_idx) => {
                let tap = tap_info
                    .get(tap_idx)
                    .ok_or_else(|| anyhow!("poly_ext tap index {tap_idx} is out of range"))?;
                ensure!(tap.group < 3, "WebGPU eval_check only supports 3 groups");
                let back = tap.back * INV_RATE;
                let out_slot = fp_alloc.alloc();
                fp_slots[var] = Some(out_slot);
                let _ = writeln!(
                    body,
                    "    let f{var}_row = (cycle + params.domain - ({back}u % params.domain)) % params.domain;"
                );
                let _ = writeln!(
                    body,
                    "    f{out_slot} = vec4<u32>(group{}.data[params.group{}_base + {}u * params.domain + f{var}_row], 0u, 0u, 0u);",
                    tap.group, tap.group, tap.offset
                );
            }
            EvalCheckFpOp::GetGlobal(arg, offset) => {
                ensure!(arg < 2, "WebGPU eval_check only supports 2 global buffers");
                let out_slot = fp_alloc.alloc();
                fp_slots[var] = Some(out_slot);
                let _ = writeln!(
                    body,
                    "    f{out_slot} = vec4<u32>(global{arg}.data[params.global{arg}_base + {offset}u], 0u, 0u, 0u);"
                );
            }
            EvalCheckFpOp::Add(lhs, rhs) => {
                let lhs_slot = eval_check_fp_slot(&fp_slots, lhs)?;
                let rhs_slot = eval_check_fp_slot(&fp_slots, rhs)?;
                used_fp.extend([lhs, rhs]);
                let out_slot = fp_alloc.alloc();
                fp_slots[var] = Some(out_slot);
                let _ = writeln!(body, "    f{out_slot} = ext_add(f{lhs_slot}, f{rhs_slot});");
            }
            EvalCheckFpOp::Sub(lhs, rhs) => {
                let lhs_slot = eval_check_fp_slot(&fp_slots, lhs)?;
                let rhs_slot = eval_check_fp_slot(&fp_slots, rhs)?;
                used_fp.extend([lhs, rhs]);
                let out_slot = fp_alloc.alloc();
                fp_slots[var] = Some(out_slot);
                let _ = writeln!(body, "    f{out_slot} = ext_sub(f{lhs_slot}, f{rhs_slot});");
            }
            EvalCheckFpOp::Mul(lhs, rhs) => {
                let lhs_slot = eval_check_fp_slot(&fp_slots, lhs)?;
                let rhs_slot = eval_check_fp_slot(&fp_slots, rhs)?;
                used_fp.extend([lhs, rhs]);
                let out_slot = fp_alloc.alloc();
                fp_slots[var] = Some(out_slot);
                let _ = writeln!(body, "    f{out_slot} = ext_mul(f{lhs_slot}, f{rhs_slot});");
            }
        }

        if last_fp.get(var).copied().flatten().is_none() {
            let slot = eval_check_fp_slot(&fp_slots, var)?;
            fp_slots[var] = None;
            fp_alloc.free(slot);
        }

        used_fp.sort_unstable();
        used_fp.dedup();
        for used in used_fp {
            if last_fp.get(used).copied().flatten() == Some(op_pos) {
                let slot = eval_check_fp_slot(&fp_slots, used)?;
                fp_slots[used] = None;
                fp_alloc.free(slot);
            }
        }
    }

    body.push_str("    var total = vec4<u32>(0u, 0u, 0u, 0u);\n    var term: vec4<u32>;\n");
    for (term_idx, term) in terms.iter().enumerate() {
        ensure!(
            needed_set.contains(&term.inner),
            "split eval_check missing term inner"
        );
        let mut used_fp = vec![term.inner];
        let inner_slot = eval_check_fp_slot(&fp_slots, term.inner)?;
        let _ = writeln!(body, "    term = f{inner_slot};");
        for cond in &term.conds {
            ensure!(
                needed_set.contains(cond),
                "split eval_check missing term condition"
            );
            let cond_slot = eval_check_fp_slot(&fp_slots, *cond)?;
            used_fp.push(*cond);
            let _ = writeln!(body, "    term = ext_mul(term, f{cond_slot});");
        }
        let _ = writeln!(
            body,
            "    total = ext_add(total, ext_mul(load_mix_pow({}u), term));",
            term.mix_exp
        );

        let op_pos = term_base + term_idx;
        used_fp.sort_unstable();
        used_fp.dedup();
        for used in used_fp {
            if last_fp.get(used).copied().flatten() == Some(op_pos) {
                let slot = eval_check_fp_slot(&fp_slots, used)?;
                fp_slots[used] = None;
                fp_alloc.free(slot);
            }
        }
    }

    ensure!(
        fp_alloc.max_used() <= WEBGPU_EVAL_CHECK_MAX_FP_SLOTS,
        "WebGPU split eval_check needs {} FP slots, max is {}",
        fp_alloc.max_used(),
        WEBGPU_EVAL_CHECK_MAX_FP_SLOTS
    );

    let mut wgsl = String::from(EVAL_CHECK_WGSL_PREFIX);
    let fp_slots = fp_alloc.max_used().max(1);
    for slot in 0..fp_slots {
        let _ = writeln!(wgsl, "    var f{slot}: vec4<u32>;");
    }
    wgsl.push_str(&body);

    if reset_check {
        wgsl.push_str("    let prev = vec4<u32>(0u, 0u, 0u, 0u);\n");
    } else {
        wgsl.push_str(
            r#"
    let prev = vec4<u32>(
        check.data[params.check_base + 0u * params.domain + cycle],
        check.data[params.check_base + 1u * params.domain + cycle],
        check.data[params.check_base + 2u * params.domain + cycle],
        check.data[params.check_base + 3u * params.domain + cycle],
    );
"#,
        );
    }
    wgsl.push_str(
        r#"
    let contribution = ext_mul(total, vec4<u32>(zerofier_inv(cycle), 0u, 0u, 0u));
    let result = ext_add(prev, contribution);
    check.data[params.check_base + 0u * params.domain + cycle] = result.x;
    check.data[params.check_base + 1u * params.domain + cycle] = result.y;
    check.data[params.check_base + 2u * params.domain + cycle] = result.z;
    check.data[params.check_base + 3u * params.domain + cycle] = result.w;
}
"#,
    );
    Ok(wgsl)
}

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
    pub bind_group_layout_creations: u64,
    pub bind_group_layout_cache_hits: u64,
    pub bind_group_creations: u64,
    pub compute_pipeline_creations: u64,
    pub compute_pipeline_cache_hits: u64,
    pub gpu_dispatches: u64,
    pub cpu_mirrors: u64,
    pub cpu_fallbacks: u64,
    pub cpu_only_ops: u64,
    pub ops: Vec<WebGpuOpDiagnostics>,
    pub upload_sources: Vec<WebGpuUploadDiagnostics>,
    pub readback_sources: Vec<WebGpuReadbackDiagnostics>,
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

/// Per-buffer WebGPU host-to-device upload usage.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WebGpuUploadDiagnostics {
    pub name: &'static str,
    pub uploads: u64,
    pub upload_bytes: u64,
}

/// Per-buffer WebGPU readback usage.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WebGpuReadbackDiagnostics {
    pub name: &'static str,
    pub readbacks: u64,
    pub readback_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct WebGpuOpStats {
    gpu_dispatches: u64,
    cpu_mirrors: u64,
    cpu_fallbacks: u64,
    cpu_only_ops: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct WebGpuUploadStats {
    uploads: u64,
    upload_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct WebGpuReadbackStats {
    readbacks: u64,
    readback_bytes: u64,
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
    bind_group_layout_creations: Cell<u64>,
    bind_group_layout_cache_hits: Cell<u64>,
    bind_group_creations: Cell<u64>,
    compute_pipeline_creations: Cell<u64>,
    compute_pipeline_cache_hits: Cell<u64>,
    gpu_dispatches: Cell<u64>,
    cpu_mirrors: Cell<u64>,
    cpu_fallbacks: Cell<u64>,
    cpu_only_ops: Cell<u64>,
    ops: RefCell<BTreeMap<&'static str, WebGpuOpStats>>,
    upload_sources: RefCell<BTreeMap<&'static str, WebGpuUploadStats>>,
    readback_sources: RefCell<BTreeMap<&'static str, WebGpuReadbackStats>>,
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
            bind_group_layout_creations: self.bind_group_layout_creations.get(),
            bind_group_layout_cache_hits: self.bind_group_layout_cache_hits.get(),
            bind_group_creations: self.bind_group_creations.get(),
            compute_pipeline_creations: self.compute_pipeline_creations.get(),
            compute_pipeline_cache_hits: self.compute_pipeline_cache_hits.get(),
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
            upload_sources: self
                .upload_sources
                .borrow()
                .iter()
                .map(|(&name, stats)| WebGpuUploadDiagnostics {
                    name,
                    uploads: stats.uploads,
                    upload_bytes: stats.upload_bytes,
                })
                .collect(),
            readback_sources: self
                .readback_sources
                .borrow()
                .iter()
                .map(|(&name, stats)| WebGpuReadbackDiagnostics {
                    name,
                    readbacks: stats.readbacks,
                    readback_bytes: stats.readback_bytes,
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
        self.bind_group_layout_creations.set(0);
        self.bind_group_layout_cache_hits.set(0);
        self.bind_group_creations.set(0);
        self.compute_pipeline_creations.set(0);
        self.compute_pipeline_cache_hits.set(0);
        self.gpu_dispatches.set(0);
        self.cpu_mirrors.set(0);
        self.cpu_fallbacks.set(0);
        self.cpu_only_ops.set(0);
        self.ops.borrow_mut().clear();
        self.upload_sources.borrow_mut().clear();
        self.readback_sources.borrow_mut().clear();
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

    fn record_upload(&self, name: &'static str, byte_len: u64) {
        Self::add(&self.host_to_gpu_uploads, 1);
        Self::add(&self.host_to_gpu_bytes, byte_len);
        let mut uploads = self.upload_sources.borrow_mut();
        let stats = uploads.entry(name).or_default();
        stats.uploads = stats.uploads.saturating_add(1);
        stats.upload_bytes = stats.upload_bytes.saturating_add(byte_len);
    }

    fn record_device_copy(&self, byte_len: u64) {
        Self::add(&self.device_copies, 1);
        Self::add(&self.device_copy_bytes, byte_len);
    }

    fn record_readback(&self, name: &'static str, byte_len: u64) {
        Self::add(&self.readbacks, 1);
        Self::add(&self.readback_bytes, byte_len);
        let mut readbacks = self.readback_sources.borrow_mut();
        let stats = readbacks.entry(name).or_default();
        stats.readbacks = stats.readbacks.saturating_add(1);
        stats.readback_bytes = stats.readback_bytes.saturating_add(byte_len);
    }

    fn record_bind_group_layout_creation(&self) {
        Self::add(&self.bind_group_layout_creations, 1);
    }

    fn record_bind_group_layout_cache_hit(&self) {
        Self::add(&self.bind_group_layout_cache_hits, 1);
    }

    fn record_bind_group_creation(&self) {
        Self::add(&self.bind_group_creations, 1);
    }

    fn record_compute_pipeline_creation(&self) {
        Self::add(&self.compute_pipeline_creations, 1);
    }

    fn record_compute_pipeline_cache_hit(&self) {
        Self::add(&self.compute_pipeline_cache_hits, 1);
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

const ELTWISE_ADD_ELEM_WGSL: &str = r#"
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

const COMBOS_PREPARE_WGSL: &str = r#"
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

const COMBOS_DIVIDE_WGSL: &str = r#"
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

const BATCH_BIT_REVERSE_WGSL: &str = r#"
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

const BATCH_EXPAND_WGSL: &str = r#"
const LINEAR_DISPATCH_STRIDE: u32 = 16776960u;

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

fn linear_global_id(gid: vec3<u32>) -> u32 {
    return gid.x + gid.y * LINEAR_DISPATCH_STRIDE;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = linear_global_id(gid);
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

@compute @workgroup_size(256)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
) {
    let local_eval_idx = gid.x;
    if (local_eval_idx >= params.eval_count) {
        return;
    }

    let eval_idx = params.eval_base + local_eval_idx;
    let poly_id = which.data[params.which_base + eval_idx];
    let cur_poly = params.coeffs_base + poly_id * params.poly_stride;
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

const BATCH_EVALUATE_ANY_SLICE_PARTIAL_WGSL: &str = r#"
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

const EVAL_CHECK_PACK_GROUP_CHUNK_WGSL: &str = r#"
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
    fold_chain_layout: web_sys::GpuBindGroupLayout,
    fold_chain_kernel: WebGpuKernel,
    rows_layout: web_sys::GpuBindGroupLayout,
    rows_kernel: WebGpuKernel,
}

impl WebGpuPoseidon2Hash {
    fn new(hal: &WebGpuHal) -> Result<Self> {
        let round_constants = hal.copy_from_elem(
            "webgpu_poseidon2_round_constants",
            poseidon2::ROUND_CONSTANTS,
        );
        let m_int_diag =
            hal.copy_from_elem("webgpu_poseidon2_m_int_diag", poseidon2::M_INT_DIAG_HZN);

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
        let fold_chain_layout = hal.create_bind_group_layout(
            "webgpu_poseidon2_fold_chain_layout",
            &[
                WebGpuBindingLayout::read_only_storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::storage(2, 0),
                WebGpuBindingLayout::uniform_dynamic(4, 32),
            ],
        )?;
        let fold_chain_kernel = hal.create_compute_kernel(
            "webgpu_poseidon2_fold_chain",
            POSEIDON2_WGSL,
            "poseidon2_fold",
            &[fold_chain_layout.clone()],
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
            fold_chain_layout,
            fold_chain_kernel,
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct EvalCheckInterpreterPipelineKey {
    base_field_fp: bool,
    private_parallel: bool,
    fp_slots: usize,
    mix_slots: usize,
    workgroup_size: u32,
}

#[derive(Clone)]
struct EvalCheckInterpreterPipeline {
    layout: web_sys::GpuBindGroupLayout,
    kernel: WebGpuKernel,
}

/// SP3 iter 7c: a compiled multi-stage staged-WGSL `eval_check` pipeline
/// for a specific `(PolyExtStepDef, base_field_fp)` pair. The bind-group
/// layout has 12 active bindings (skipping the interpreter's `instrs`
/// at binding 6): 8 from iter 6 (check, group0..2, global0..1,
/// mix_pows, params) plus 4 new for multi-stage scratch (binding 9 =
/// `staged_scratch_params` UBO, 10/11/12 = `fp_scratch` /
/// `mix_tot_scratch` / `mix_mul_scratch` rw storage). All stages of
/// the same DEF share the same bind group; only the bound pipeline
/// changes per stage.
///
/// SP3 iter 7d: the four scratch buffers are cached on the pipeline so
/// they're allocated once per DEF (sized to `tile_size * stride * 4 B`,
/// independent of the prove's full domain) and reused across all
/// `eval_check` calls. This bounds total GPU memory by O(unique DEFs ×
/// per-tile scratch) instead of O(eval_check_call_count × per-domain
/// scratch).
#[derive(Clone)]
struct StagedEvalCheckPipeline {
    layout: web_sys::GpuBindGroupLayout,
    stages: Vec<WebGpuKernel>,
    /// SP3 iter 7m: `@compute @workgroup_size(N)` baked into every
    /// stage's WGSL by `choose_workgroup_size`. Dispatch shape is
    /// `(tile_size / workgroup_size, num_tiles, 1)` so the total
    /// thread count remains `tile_size * num_tiles` (= domain rounded
    /// up to `tile_size`). All stages of a pipeline share the same
    /// workgroup_size because the codegen picks it from `plan.fp_slots`
    /// / `plan.mix_slots`, which are cross-chunk high-water marks.
    workgroup_size: u32,
    base_field_fp: bool,
    /// u32 words per tile-local cycle in `fp_scratch` (max-live × 1 for
    /// Base or × 4 for Ext). 0 when `stages.len() == 1` (single-kernel,
    /// no scratch needed; cached buffers are still 4-byte dummies so
    /// the validator's binding-count rules don't trip).
    fp_scratch_stride_u32: usize,
    /// u32 words per tile-local cycle in `mix_tot_scratch` /
    /// `mix_mul_scratch` (max-live × 4). 0 when no multi-stage save
    /// is needed.
    mix_scratch_stride_u32: usize,
    /// Cached `tile_size * fp_stride * 4 B` storage buffer.
    fp_scratch: web_sys::GpuBuffer,
    /// Cached mix scratches (each `tile_size * mix_stride * 4 B`).
    mix_tot_scratch: web_sys::GpuBuffer,
    mix_mul_scratch: web_sys::GpuBuffer,
    /// SP3 iter 7g: cached 16-byte UBO holding pipeline-constant
    /// `{fp_stride, mix_stride, num_stages, tile_size}`. Written once at
    /// pipeline create; every `eval_check` call binds it as-is.
    scratch_params_buf: web_sys::GpuBuffer,
    /// SP3 iter 7f: cached storage buffer for `mix_pows` (sized to the
    /// DEF's `mix_expected * 4 u32`). Each `eval_check` call rewrites
    /// it with the call-specific `poly_mix^exp` values.
    mix_pows_buf: web_sys::GpuBuffer,
    /// SP3 iter 7f: cached 96-byte uniform buffer for the main Params
    /// UBO at binding 8. Each `eval_check` call rewrites it.
    params_buf: web_sys::GpuBuffer,
    /// SP3 iter 7f: `mix_expected` for this DEF. Recorded here so the
    /// dispatch knows how many `u32` words to write into `mix_pows_buf`
    /// without recomputing.
    mix_pow_words: usize,
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
    /// When true, the bind group accepts a dynamic byte offset for this
    /// binding via `setBindGroup`. SP3 iter 7e uses this to advance
    /// `tile_base` through a single `scratch_params` UBO without
    /// rebinding or resubmitting between tiles.
    pub has_dynamic_offset: bool,
}

impl WebGpuBindingLayout {
    /// Create a read/write storage buffer binding layout.
    pub fn storage(binding: u32, min_binding_size: u64) -> Self {
        Self {
            binding,
            ty: web_sys::GpuBufferBindingType::Storage,
            min_binding_size,
            has_dynamic_offset: false,
        }
    }

    /// Create a read-only storage buffer binding layout.
    pub fn read_only_storage(binding: u32, min_binding_size: u64) -> Self {
        Self {
            binding,
            ty: web_sys::GpuBufferBindingType::ReadOnlyStorage,
            min_binding_size,
            has_dynamic_offset: false,
        }
    }

    /// Create a uniform buffer binding layout.
    pub fn uniform(binding: u32, min_binding_size: u64) -> Self {
        Self {
            binding,
            ty: web_sys::GpuBufferBindingType::Uniform,
            min_binding_size,
            has_dynamic_offset: false,
        }
    }

    /// Create a uniform buffer binding layout with dynamic offset support
    /// (SP3 iter 7e). The bind group's `setBindGroup` call must then
    /// supply a `u32` byte offset for this binding.
    pub fn uniform_dynamic(binding: u32, min_binding_size: u64) -> Self {
        Self {
            binding,
            ty: web_sys::GpuBufferBindingType::Uniform,
            min_binding_size,
            has_dynamic_offset: true,
        }
    }
}

/// SP9 corrected (2026-05-15): hash a (label_ptr, entries-shape) tuple
/// for the bind-group-layout cache. label is a `&'static str` so its
/// pointer is a stable identity. Entry fields hash to a layout-shape
/// fingerprint; identical fingerprints share a layout instance.
fn compute_bind_group_layout_cache_key(
    label: &'static str,
    entries: &[WebGpuBindingLayout],
) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    let mut h = DefaultHasher::new();
    h.write_usize(label.as_ptr() as usize);
    h.write_usize(label.len());
    h.write_usize(entries.len());
    for entry in entries {
        h.write_u32(entry.binding);
        // GpuBufferBindingType has 3 variants on this target; encode
        // explicitly so the hash is stable across rebuilds.
        let ty_byte: u8 = if entry.ty == web_sys::GpuBufferBindingType::Storage {
            0
        } else if entry.ty == web_sys::GpuBufferBindingType::ReadOnlyStorage {
            1
        } else if entry.ty == web_sys::GpuBufferBindingType::Uniform {
            2
        } else {
            255
        };
        h.write_u8(ty_byte);
        h.write_u64(entry.min_binding_size);
        h.write_u8(entry.has_dynamic_offset as u8);
    }
    h.finish()
}

fn compute_compute_pipeline_cache_key(
    label: &'static str,
    wgsl: &str,
    entry_point: &str,
    layout_keys: &[String],
) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    let mut h = DefaultHasher::new();
    h.write_usize(label.as_ptr() as usize);
    h.write_usize(label.len());
    h.write(wgsl.as_bytes());
    h.write(entry_point.as_bytes());
    h.write_usize(layout_keys.len());
    for layout_key in layout_keys {
        h.write(layout_key.as_bytes());
    }
    h.finish()
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

/// SP-CR D16 (2026-05-12): owning wrapper that calls `GpuBuffer.destroy()` when
/// the last Rc reference drops. Without explicit destruction, Chrome WebGPU's
/// per-context VRAM budget is exhausted across multi-segment recursion lifts
/// (D14 surfaced `VK_ERROR_OUT_OF_DEVICE_MEMORY` after 7-8 segments), because
/// JS GC does not promptly reclaim GpuBuffer handles between hot-loop dispatches.
struct WebGpuBufferOwner {
    buffer: web_sys::GpuBuffer,
}

impl Drop for WebGpuBufferOwner {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

/// A browser WebGPU buffer with a CPU shadow for the existing synchronous HAL API.
#[derive(Clone)]
pub struct WebGpuBuffer<T> {
    cpu: CpuBuffer<T>,
    gpu: Option<Rc<WebGpuBufferOwner>>,
    elem_offset: usize,
    /// CPU-side changes that have not been uploaded to the GPU buffer.
    cpu_dirty: Rc<Cell<bool>>,
    /// GPU-side changes that have not been read back into the CPU shadow.
    cpu_stale: Rc<Cell<bool>>,
    marker: PhantomData<T>,
}

impl<T> WebGpuBuffer<T> {
    fn new(
        cpu: CpuBuffer<T>,
        gpu: Option<Rc<WebGpuBufferOwner>>,
        cpu_dirty: Rc<Cell<bool>>,
        cpu_stale: Rc<Cell<bool>>,
    ) -> Self {
        Self {
            cpu,
            gpu,
            elem_offset: 0,
            cpu_dirty,
            cpu_stale,
            marker: PhantomData,
        }
    }

    fn cpu(&self) -> &CpuBuffer<T> {
        &self.cpu
    }

    fn mark_cpu_dirty(&self) {
        self.cpu_dirty.set(true);
        self.cpu_stale.set(false);
    }

    /// Mark the GPU buffer as containing newer data than the CPU shadow.
    pub fn mark_gpu_dirty(&self) {
        self.cpu_dirty.set(false);
        self.cpu_stale.set(true);
    }

    fn mark_synced(&self) {
        self.cpu_dirty.set(false);
        self.cpu_stale.set(false);
    }

    fn mark_cpu_result(&self, gpu_current: bool) {
        self.cpu_dirty.set(!gpu_current);
        self.cpu_stale.set(false);
    }

    /// Returns true when synchronous CPU views are current.
    pub fn cpu_is_current(&self) -> bool {
        !self.cpu_stale.get()
    }

    /// Returns true when the browser `GPUBuffer` is current.
    pub fn gpu_is_current(&self) -> bool {
        !self.cpu_dirty.get()
    }

    /// Return the underlying browser `GPUBuffer`, when the allocation is non-empty.
    pub fn raw_buffer(&self) -> Option<&web_sys::GpuBuffer> {
        self.gpu.as_ref().map(|owner| &owner.buffer)
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

        ensure!(
            !self.cpu_stale.get(),
            "cannot upload stale CPU shadow for WebGPU buffer {}",
            self.cpu.name()
        );

        if let Some(gpu) = self.raw_buffer() {
            let mut upload = Ok(());
            self.cpu.view(|cpu| {
                let elems_per_chunk = (WEBGPU_SAFE_QUEUE_WRITE_BYTES / mem::size_of::<T>()).max(1);
                for (chunk_idx, chunk) in cpu.chunks(elems_per_chunk).enumerate() {
                    let chunk_offset = chunk_idx
                        .checked_mul(elems_per_chunk)
                        .expect("WebGPU upload chunk offset overflow");
                    upload = hal.write_buffer_named(
                        gpu,
                        self.cpu.name(),
                        self.byte_offset() + byte_len_for::<T>(chunk_offset),
                        bytemuck::cast_slice(chunk),
                    );
                    if upload.is_err() {
                        break;
                    }
                }
            });
            upload?;
        }

        self.cpu_dirty.set(false);
        Ok(())
    }

    /// Read the browser `GPUBuffer` back into the CPU shadow when the GPU owns
    /// newer contents. This is the async boundary WebGPU needs before any
    /// synchronous transcript, Merkle, or verifier-facing CPU view.
    pub async fn sync_gpu_to_cpu(&self, hal: &WebGpuHal) -> Result<()>
    where
        T: bytemuck::CheckedBitPattern + Clone,
    {
        if !self.cpu_stale.get() {
            return Ok(());
        }

        let Some(gpu) = self.raw_buffer() else {
            ensure!(
                self.cpu.size() == 0,
                "cannot read back missing GPU buffer {}",
                self.cpu.name()
            );
            self.mark_synced();
            return Ok(());
        };

        let byte_len = byte_len_for::<T>(self.cpu.size());
        let bytes = hal
            .read_buffer_range_named(gpu, self.byte_offset(), byte_len, self.cpu.name())
            .await?;
        let values = bytemuck::checked::try_cast_slice::<u8, T>(bytes.as_slice())
            .map_err(|err| anyhow!("invalid WebGPU readback for {}: {err}", self.cpu.name()))?;
        ensure!(
            values.len() == self.cpu.size(),
            "readback size mismatch for WebGPU buffer {}: got {} elems, expected {}",
            self.cpu.name(),
            values.len(),
            self.cpu.size()
        );
        self.cpu.view_mut(|cpu| {
            cpu.clone_from_slice(values);
        });
        self.mark_synced();
        Ok(())
    }

    /// SP7 iter 6d-g step 6.2.10 (2026-05-16): bitwise GPU->CPU sync
    /// that ALLOWS `Val::INVALID` (0xffffffff) values to flow back into
    /// the CPU shadow without failing the `CheckedBitPattern` validator.
    /// Required for the iter-6d-g pre-witgen dispatch path: GPU partially
    /// populates `data_buf` (shadow_init + per-arm chunks), other cells
    /// remain `INVALID`. Standard `sync_gpu_to_cpu` would error on the
    /// INVALID bytes; this variant transmutes raw u32 bytes into the
    /// target type via `repr(transparent)` semantics. Caller takes
    /// responsibility for ensuring T is bitwise-equivalent to u32 (i.e.,
    /// `BabyBearElem` is `#[repr(transparent)] struct(u32)`).
    pub async fn sync_gpu_to_cpu_unchecked(&self, hal: &WebGpuHal) -> Result<()>
    where
        T: Clone,
    {
        if !self.cpu_stale.get() {
            return Ok(());
        }

        let Some(gpu) = self.raw_buffer() else {
            ensure!(
                self.cpu.size() == 0,
                "cannot read back missing GPU buffer {}",
                self.cpu.name()
            );
            self.mark_synced();
            return Ok(());
        };

        let byte_len = byte_len_for::<T>(self.cpu.size());
        let bytes = hal
            .read_buffer_range_named(gpu, self.byte_offset(), byte_len, self.cpu.name())
            .await?;
        // SAFETY: caller asserts T is bitwise-equivalent to a sequence of
        // u32 (`repr(transparent)`). For `BabyBearElem` (T = Val) this
        // holds: it's `#[repr(transparent)] struct Elem(u32)`. We read
        // raw u32s (Pod) and reinterpret as T via unsafe transmute on
        // the slice pointer.
        let u32s: &[u32] = bytemuck::cast_slice(bytes.as_slice());
        ensure!(
            u32s.len() == self.cpu.size(),
            "readback size mismatch for WebGPU buffer {}: got {} elems, expected {}",
            self.cpu.name(),
            u32s.len(),
            self.cpu.size()
        );
        let values: &[T] = unsafe {
            std::slice::from_raw_parts(u32s.as_ptr() as *const T, u32s.len())
        };
        self.cpu.view_mut(|cpu| {
            cpu.clone_from_slice(values);
        });
        self.mark_synced();
        Ok(())
    }

    fn assert_cpu_current(&self, op: &str)
    where
        T: Clone,
    {
        assert!(
            !self.cpu_stale.get(),
            "{op} requires a current CPU shadow for WebGPU buffer {}; call sync_gpu_to_cpu(...).await first",
            self.cpu.name()
        );
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
            cpu_stale: self.cpu_stale.clone(),
            marker: PhantomData,
        }
    }

    fn get_at(&self, idx: usize) -> T {
        self.assert_cpu_current("get_at");
        self.cpu.get_at(idx)
    }

    fn view<F: FnOnce(&[T])>(&self, f: F) {
        self.assert_cpu_current("view");
        self.cpu.view(f);
    }

    fn view_mut<F: FnOnce(&mut [T])>(&self, f: F) {
        self.assert_cpu_current("view_mut");
        self.cpu.view_mut(f);
        self.mark_cpu_dirty();
    }

    fn to_vec(&self) -> Vec<T> {
        self.assert_cpu_current("to_vec");
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
    // SP6d iter 4: per-HAL accumulator for GPU-active stage elapsed_ms.
    // Wrapped in Rc<Cell<_>> so WebGpuStageTimer instances can hold a
    // cheap clone without back-references. Replaces the thread-local
    // WEBGPU_GPU_ACTIVE_MS when running under a multi-HAL pool.
    gpu_active_ms: Rc<Cell<f64>>,
    gpu_authoritative: Cell<bool>,
    eval_check_gpu_enabled: Cell<bool>,
    batch_expand_into_evaluate_ntt_gpu_enabled: Cell<bool>,
    batch_interpolate_ntt_gpu_enabled: Cell<bool>,
    batch_bit_reverse_gpu_enabled: Cell<bool>,
    hash_fold_gpu_enabled: Cell<bool>,
    hash_rows_gpu_enabled: Cell<bool>,
    zk_shift_gpu_enabled: Cell<bool>,
    max_buffer_size: u64,
    max_storage_buffer_binding_size: u64,
    max_compute_workgroup_storage_size: u32,
    min_uniform_buffer_offset_alignment: u32,
    eval_check_interpreter_pipelines:
        RefCell<BTreeMap<EvalCheckInterpreterPipelineKey, EvalCheckInterpreterPipeline>>,
    // SP3 iter 5 (2026-05-12): runtime flag that opts a HAL into the
    // staged-WGSL eval_check path before the runtime interpreter. Default
    // false — the interpreter remains the production path until iter 7's
    // browser parity test proves the staged path produces byte-equivalent
    // outputs and a multi-stage split (iter 6) handles the production
    // rv32im DEF's ~20k-op shader size.
    staged_eval_check_enabled: Cell<bool>,
    // SP3 iter 5 (2026-05-12): per-DEF cache of compiled staged eval_check
    // pipelines. Keyed by `def as *const PolyExtStepDef as usize` because
    // DEFs are `&'static` and program identity is the natural cache key.
    // Each cached entry holds the bind-group layout + the compiled compute
    // kernel so subsequent dispatches against the same DEF skip the
    // (potentially slow) WGSL compile step.
    staged_eval_check_pipelines: RefCell<BTreeMap<usize, StagedEvalCheckPipeline>>,
    // SP-CR D15 (2026-05-12): cache the static NTT roots-of-unity tables.
    // Previously `dispatch_batch_expand_into_evaluate_ntt` and
    // `dispatch_batch_interpolate_ntt` called `copy_from_elem` on each
    // invocation, creating ~352 transient 112-byte GPU buffers per xgboost
    // run. Cumulative Chrome WebGPU resource pressure is one suspect for the
    // zero-roots silent failure. Caching these once at HAL init removes a
    // measurable share of the per-dispatch buffer churn.
    ntt_roots_fwd: Option<WebGpuBuffer<BabyBearElem>>,
    ntt_roots_rev: Option<WebGpuBuffer<BabyBearElem>>,
    // SP9 corrected (2026-05-15): cache `GpuBindGroupLayout` instances by
    // (label.as_ptr(), entries-shape-hash). 31 `create_bind_group_layout`
    // sites in this file. Layouts are immutable shape descriptors;
    // identical shapes can safely share one instance. This is the
    // foundation for a later pipeline cache (pipelines must reference
    // the SAME layout INSTANCE as the bind groups dispatched against
    // them -- see 61d3163c9 failed-experiment ledger).
    bind_group_layout_cache: RefCell<BTreeMap<u64, web_sys::GpuBindGroupLayout>>,
    // SP6f (2026-05-18): map HAL-created bind-group-layout JS objects
    // back to their structural cache keys. Compute pipeline caching is only
    // enabled when every supplied layout comes from this HAL, so cache hits
    // cannot accidentally reuse a pipeline with an unrelated layout instance.
    bind_group_layout_key_map: js_sys::WeakMap,
    // SP6f (2026-05-18): cache compute pipelines after layout identity is
    // stable. Pipelines depend only on WGSL, entry point, and bind-group
    // layouts; unlike bind groups they do not retain per-proof buffers.
    compute_kernel_cache: RefCell<BTreeMap<u64, WebGpuKernel>>,
}

/// Restores the previous GPU-authoritative mode when dropped.
pub struct WebGpuAuthoritativeScope<'a> {
    hal: &'a WebGpuHal,
    previous: bool,
}

impl Drop for WebGpuAuthoritativeScope<'_> {
    fn drop(&mut self) {
        self.hal.set_gpu_authoritative(self.previous);
    }
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
        let limits = device.limits();
        let queue = device.queue();
        let max_buffer_size = limits.max_buffer_size() as u64;
        let max_storage_buffer_binding_size = limits.max_storage_buffer_binding_size() as u64;
        let max_compute_workgroup_storage_size = limits.max_compute_workgroup_storage_size();
        let min_uniform_buffer_offset_alignment = limits.min_uniform_buffer_offset_alignment();
        log_webgpu_stage(&format!(
            "browser-prove:webgpu-limits max_buffer_size={} max_storage_buffer_binding_size={} max_compute_workgroup_storage_size={}",
            max_buffer_size, max_storage_buffer_binding_size, max_compute_workgroup_storage_size
        ));

        // SP-CR D14 (2026-05-12): Surface Chrome WebGPU `uncapturederror` events
        // so silent validation/OOM failures during cumulative-pressure paths
        // (recursion lift/join at multi-segment scale) become visible. Without
        // this listener Chrome swallows async device errors and dispatches that
        // failed validation report success — see xgboost zero-roots regression.
        // The closure is leaked via `.forget()`; it lives for the device's
        // lifetime, which equals the HAL's lifetime.
        let uncaptured_error_listener =
            Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
                let error = js_sys::Reflect::get(&event, &JsValue::from_str("error"))
                    .unwrap_or(JsValue::NULL);
                let msg = js_sys::Reflect::get(&error, &JsValue::from_str("message"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_else(|| "(no message)".to_string());
                let kind = error
                    .dyn_ref::<js_sys::Object>()
                    .map(|obj| obj.constructor().name().as_string().unwrap_or_default())
                    .unwrap_or_else(|| "GPUError".to_string());
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "browser-prove:webgpu-uncaptured-error type={kind} msg={msg}"
                )));
            });
        device.set_onuncapturederror(Some(uncaptured_error_listener.as_ref().unchecked_ref()));
        uncaptured_error_listener.forget();

        let mut hal = Self {
            device,
            queue,
            cpu: CpuHal::new(hash_suite),
            poseidon2: None,
            diagnostics: WebGpuDiagnosticsState::default(),
            gpu_active_ms: Rc::new(Cell::new(0.0)),
            gpu_authoritative: Cell::new(false),
            eval_check_gpu_enabled: Cell::new(true),
            batch_expand_into_evaluate_ntt_gpu_enabled: Cell::new(true),
            batch_interpolate_ntt_gpu_enabled: Cell::new(true),
            batch_bit_reverse_gpu_enabled: Cell::new(true),
            hash_fold_gpu_enabled: Cell::new(true),
            hash_rows_gpu_enabled: Cell::new(true),
            zk_shift_gpu_enabled: Cell::new(true),
            max_buffer_size,
            max_storage_buffer_binding_size,
            max_compute_workgroup_storage_size,
            min_uniform_buffer_offset_alignment,
            eval_check_interpreter_pipelines: RefCell::new(BTreeMap::new()),
            staged_eval_check_enabled: Cell::new(false),
            staged_eval_check_pipelines: RefCell::new(BTreeMap::new()),
            ntt_roots_fwd: None,
            ntt_roots_rev: None,
            bind_group_layout_cache: RefCell::new(BTreeMap::new()),
            bind_group_layout_key_map: js_sys::WeakMap::new(),
            compute_kernel_cache: RefCell::new(BTreeMap::new()),
        };
        // SP-CR D15 (2026-05-12): allocate the NTT roots-of-unity tables once
        // at HAL init instead of per-dispatch. See struct field comment.
        hal.ntt_roots_fwd = Some(hal.copy_from_elem("webgpu_ntt_roots_fwd", BabyBearElem::ROU_FWD));
        hal.ntt_roots_rev = Some(hal.copy_from_elem("webgpu_ntt_roots_rev", BabyBearElem::ROU_REV));
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

    /// Enable or disable GPU-authoritative HAL outputs.
    ///
    /// When enabled, WebGPU kernels that successfully dispatch may skip their
    /// CPU mirror work and mark output buffers as requiring async readback
    /// before any synchronous CPU view.
    pub fn set_gpu_authoritative(&self, enabled: bool) {
        self.gpu_authoritative.set(enabled);
    }

    /// Returns whether successful WebGPU kernels are allowed to own outputs
    /// without an immediate CPU mirror.
    pub fn gpu_authoritative(&self) -> bool {
        self.gpu_authoritative.get()
    }

    /// SP6d iter 4: current gpu_active_ms for this HAL. Sum of all
    /// `WebGpuStageTimer::new_active_for(_, self)` scopes' elapsed_ms
    /// since the last reset.
    pub fn gpu_active_ms(&self) -> f64 {
        self.gpu_active_ms.get()
    }

    /// SP6d iter 4: reset this HAL's gpu_active_ms accumulator to 0.
    pub fn reset_gpu_active_ms(&self) {
        self.gpu_active_ms.set(0.0);
    }

    /// SP6d iter 4: clone the counter handle so a `WebGpuStageTimer`
    /// can increment it on drop without holding a back-reference to
    /// the HAL. Cheap (Rc::clone).
    pub fn gpu_active_ms_handle(&self) -> Rc<Cell<f64>> {
        self.gpu_active_ms.clone()
    }

    /// Enable or disable WebGPU eval_check dispatch.
    ///
    /// This is a diagnostic switch used by browser parity tests to isolate
    /// eval_check-specific correctness failures from the rest of the async
    /// GPU-authoritative proof path.
    pub fn set_eval_check_gpu_enabled(&self, enabled: bool) {
        self.eval_check_gpu_enabled.set(enabled);
    }

    /// Enable or disable the staged-WGSL `eval_check` fast path. When `true`,
    /// `dispatch_eval_check_poly_ext` tries the staged kernel first and
    /// falls through to the runtime interpreter on any failure (compile,
    /// dispatch, or `CodegenError`). Default `false` — browser parity tests
    /// enable this for individual fixtures once parity is established.
    pub fn set_staged_eval_check_enabled(&self, enabled: bool) {
        self.staged_eval_check_enabled.set(enabled);
    }

    /// Returns whether the staged-WGSL `eval_check` fast path is enabled.
    pub fn staged_eval_check_enabled(&self) -> bool {
        self.staged_eval_check_enabled.get()
    }

    /// Enable or disable specific WebGPU kernels for diagnostics.
    #[doc(hidden)]
    pub fn set_op_gpu_enabled(&self, op: &str, enabled: bool) {
        match op {
            "batch_expand_into_evaluate_ntt" => {
                self.batch_expand_into_evaluate_ntt_gpu_enabled.set(enabled)
            }
            "batch_interpolate_ntt" => self.batch_interpolate_ntt_gpu_enabled.set(enabled),
            "batch_bit_reverse" => self.batch_bit_reverse_gpu_enabled.set(enabled),
            "hash_fold" => self.hash_fold_gpu_enabled.set(enabled),
            "hash_rows" => self.hash_rows_gpu_enabled.set(enabled),
            "zk_shift" => self.zk_shift_gpu_enabled.set(enabled),
            _ => panic!("unknown WebGPU diagnostic op: {op}"),
        }
    }

    /// Temporarily set GPU-authoritative mode for a scoped proof stage.
    pub fn gpu_authoritative_scope(&self, enabled: bool) -> WebGpuAuthoritativeScope<'_> {
        let previous = self.gpu_authoritative();
        self.set_gpu_authoritative(enabled);
        WebGpuAuthoritativeScope {
            hal: self,
            previous,
        }
    }

    fn max_storage_binding_bytes(&self) -> u64 {
        self.max_storage_buffer_binding_size
            .min(WEBGPU_SAFE_STORAGE_BINDING_BYTES)
    }

    fn eval_check_base_workgroup_lanes(&self, fp_slots: usize, mix_slots: usize) -> u32 {
        let bytes_per_lane = fp_slots
            .checked_mul(mem::size_of::<u32>())
            .and_then(|bytes| {
                let mix_bytes = mix_slots
                    .checked_mul(mem::size_of::<[u32; 4]>())
                    .and_then(|bytes| bytes.checked_mul(2))?;
                bytes.checked_add(mix_bytes)
            })
            .expect("WebGPU eval_check base scratch size overflow");
        let lanes = (self.max_compute_workgroup_storage_size as usize / bytes_per_lane).max(1);
        lanes.min(WEBGPU_EVAL_CHECK_INTERPRETER_WORKGROUP_SIZE as usize) as u32
    }

    fn eval_check_interpreter_pipeline(
        &self,
        base_field_fp: bool,
        private_parallel: bool,
        fp_slots: usize,
        mix_slots: usize,
        workgroup_size: u32,
    ) -> Result<EvalCheckInterpreterPipeline> {
        let key = EvalCheckInterpreterPipelineKey {
            base_field_fp,
            private_parallel,
            fp_slots,
            mix_slots,
            workgroup_size,
        };
        if let Some(pipeline) = self.eval_check_interpreter_pipelines.borrow().get(&key) {
            return Ok(pipeline.clone());
        }

        let layout = self.create_bind_group_layout(
            "webgpu_eval_check_interpreter_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::read_only_storage(6, 0),
                WebGpuBindingLayout::read_only_storage(7, 0),
                WebGpuBindingLayout::uniform(8, 96),
            ],
        )?;
        let (kernel_name, wgsl) = if private_parallel {
            (
                "webgpu_eval_check_base_private_interpreter",
                build_eval_check_base_interpreter_wgsl(fp_slots, mix_slots, true, workgroup_size),
            )
        } else if base_field_fp {
            (
                "webgpu_eval_check_base_interpreter",
                build_eval_check_base_interpreter_wgsl(fp_slots, mix_slots, false, workgroup_size),
            )
        } else {
            (
                "webgpu_eval_check_interpreter",
                build_eval_check_interpreter_wgsl(fp_slots, mix_slots),
            )
        };
        let kernel = self.create_compute_kernel(kernel_name, &wgsl, "main", &[layout.clone()])?;
        let pipeline = EvalCheckInterpreterPipeline { layout, kernel };
        self.eval_check_interpreter_pipelines
            .borrow_mut()
            .insert(key, pipeline.clone());
        Ok(pipeline)
    }

    /// SP3 iter 5: cache lookup (or compile + insert) of a staged-WGSL
    /// `eval_check` pipeline for a specific DEF + field-mode pair. Cache
    /// is keyed by `def as *const _ as usize` because `PolyExtStepDef`s
    /// are `&'static` and program identity is the natural key. Compilation
    /// may be slow on large DEFs — the rv32im production DEF emits a
    /// ~1.6 MB shader whose Chrome compile time is empirical — so caching
    /// per-DEF avoids paying that cost on every dispatch.
    fn staged_eval_check_pipeline(
        &self,
        def: &PolyExtStepDef,
        taps: &TapSet<'_>,
        base_field_fp: bool,
    ) -> Result<StagedEvalCheckPipeline> {
        let key = def as *const PolyExtStepDef as usize;
        if let Some(pipeline) = self.staged_eval_check_pipelines.borrow().get(&key) {
            if pipeline.base_field_fp == base_field_fp {
                return Ok(pipeline.clone());
            }
        }
        let field_mode = if base_field_fp {
            FieldMode::Base
        } else {
            FieldMode::Ext
        };
        let emitter_taps: Vec<EmitterTap> = taps
            .taps()
            .map(|t| EmitterTap {
                group: t.group() as u32,
                offset: t.offset() as u32,
                back_inv_rate: (t.back() * INV_RATE) as u32,
            })
            .collect();
        let multi = staged_multi_kernel_from_def(
            "staged_eval_check",
            def,
            &emitter_taps,
            field_mode,
            SP3_STAGED_TARGET_CHUNK_OPS,
        )
        .map_err(|err: CodegenError| {
            anyhow!("staged eval_check codegen failed: {:?}", err)
        })?;
        // SP3 iter 7n+t: log per-stage fp_slots/mix_slots/workgroup_size
        // so we can see the per-chunk allocator's effect. Each stage
        // now resets the slot allocator (iter 7t), so the high-water
        // varies per chunk and may be much smaller than the cross-chunk
        // global max.
        let per_stage_summary = multi
            .stages
            .iter()
            .enumerate()
            .map(|(i, s)| format!("s{i}:fp={},mix={},wg={}", s.fp_slots, s.mix_slots, s.workgroup_size))
            .collect::<Vec<_>>()
            .join(" ");
        log_webgpu_stage(&format!(
            "browser-prove:staged-plan field_mode={field_mode:?} stages={} per_stage=[{per_stage_summary}] fp_scratch_stride_u32={} mix_scratch_stride_u32={} def_block_len={}",
            multi.stages.len(),
            multi.fp_scratch_stride_u32,
            multi.mix_scratch_stride_u32,
            def.block.len(),
        ));
        let layout = self.create_bind_group_layout(
            "webgpu_staged_eval_check_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::read_only_storage(5, 0),
                // binding 6 (instrs) intentionally omitted; staged kernel
                // inlines the DEF rather than reading an instruction stream.
                // SP3 iter 7x: binding 7 (mix_pows) is now a uniform
                // buffer (CUDA `__constant__` analog). The full 256 KiB
                // is bound regardless of the DEF's actual mix_pow_words —
                // the WGSL declares a fixed-size array and only reads
                // the prefix the DEF needs.
                WebGpuBindingLayout::uniform(
                    7,
                    (SP3_STAGED_MIX_POWS_UBO_VEC4_CAPACITY * 16) as u64,
                ),
                WebGpuBindingLayout::uniform(8, 96),
                // SP3 iter 7c: new scratch bindings. Always present in the
                // layout — single-stage emissions bind a dummy 4-byte fp
                // / mix scratch buffer to satisfy the validator.
                // SP3 iter 7g: binding 9 is a 16-byte UBO with
                // pipeline-constant {fp_stride, mix_stride, num_stages,
                // tile_size}, bound once and read by every stage.
                WebGpuBindingLayout::uniform(9, 16),
                WebGpuBindingLayout::storage(10, 0),
                WebGpuBindingLayout::storage(11, 0),
                WebGpuBindingLayout::storage(12, 0),
            ],
        )?;
        let mut stages = Vec::with_capacity(multi.stages.len());
        for (idx, stage) in multi.stages.iter().enumerate() {
            // Concatenate prelude + per-stage body for compilation.
            let mut wgsl = String::with_capacity(
                STAGED_EVAL_CHECK_PRELUDE_WGSL.len() + stage.wgsl_source.len() + 64,
            );
            wgsl.push_str(STAGED_EVAL_CHECK_PRELUDE_WGSL);
            wgsl.push('\n');
            wgsl.push_str(&stage.wgsl_source);
            let kernel_name = if multi.stages.len() == 1 {
                "webgpu_staged_eval_check"
            } else {
                "webgpu_staged_eval_check_stage"
            };
            // SP3 iter 7q (2026-05-12): time each stage's WGSL compile
            // separately so we can attribute the staged-path slowness.
            // Iter 7p (workgroup_size=32) didn't move total prove time,
            // so the 40 s overhead is either WGSL compile (one-shot per
            // pipeline) or kernel execution (per-call).
            let _stage_compile_timer = WebGpuStageTimer::new(format!(
                "staged_eval_check_compile stage={idx}/{} wgsl_bytes={}",
                multi.stages.len(),
                wgsl.len()
            ));
            let kernel =
                self.create_compute_kernel(kernel_name, &wgsl, "main", &[layout.clone()])?;
            stages.push(kernel);
        }
        // SP3 iter 7d: allocate scratch buffers once at pipeline create
        // and cache them; they're reused across every eval_check call
        // that hits this pipeline. Size is `SP3_STAGED_TILE_SIZE *
        // stride * 4 B`, independent of any single call's domain.
        let fp_scratch_byte_len = if multi.fp_scratch_stride_u32 == 0 {
            4
        } else {
            byte_len_for::<u32>(multi.fp_scratch_stride_u32 * SP3_STAGED_TILE_SIZE as usize)
        };
        let mix_scratch_byte_len = if multi.mix_scratch_stride_u32 == 0 {
            4
        } else {
            byte_len_for::<u32>(multi.mix_scratch_stride_u32 * SP3_STAGED_TILE_SIZE as usize)
        };
        let fp_scratch = self
            .create_storage_buffer("webgpu_staged_eval_check_fp_scratch", fp_scratch_byte_len)?;
        let mix_tot_scratch = self.create_storage_buffer(
            "webgpu_staged_eval_check_mix_tot_scratch",
            mix_scratch_byte_len,
        )?;
        let mix_mul_scratch = self.create_storage_buffer(
            "webgpu_staged_eval_check_mix_mul_scratch",
            mix_scratch_byte_len,
        )?;
        // SP3 iter 7g: scratch_params is a single 16-byte UBO with
        // {fp_stride, mix_stride, num_stages, tile_size}. All fields
        // are pipeline-constant — written ONCE at pipeline create, then
        // every eval_check call reuses it. Threads compute their cycle
        // via `gid.y * tile_size + gid.x`, so no per-tile UBO offset is
        // needed. Replaces iter 7e/7f's `MAX_TILES * 256 B` dynamic-
        // offset array with a single 16-byte entry.
        let scratch_params_buf = self.create_buffer(
            "webgpu_staged_eval_check_scratch_params",
            16,
            WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;
        let num_stages_u32 =
            u32::try_from(multi.stages.len()).expect("staged num_stages exceeds u32");
        let fp_stride_u32 = u32::try_from(multi.fp_scratch_stride_u32)
            .expect("staged fp_scratch_stride exceeds u32");
        let mix_stride_u32 = u32::try_from(multi.mix_scratch_stride_u32)
            .expect("staged mix_scratch_stride exceeds u32");
        let scratch_params_words: [u32; 4] = [
            fp_stride_u32,
            mix_stride_u32,
            num_stages_u32,
            SP3_STAGED_TILE_SIZE,
        ];
        self.write_buffer_named(
            &scratch_params_buf,
            "webgpu_staged_eval_check_scratch_params",
            0,
            bytemuck::cast_slice(&scratch_params_words),
        )?;
        // SP3 iter 7f: cache mix_pows and params buffers on the
        // pipeline. Sizes depend only on the DEF (mix_pow_words for
        // mix_pows, fixed 96 bytes for params) so they're stable
        // across all eval_check calls hitting this pipeline.
        let mix_expected = def.ret + 1;
        let mix_pow_words = mix_expected * BabyBearExtElem::EXT_SIZE;
        // SP3 iter 7x: mix_pows is a uniform buffer (CUDA `__constant__`
        // analog). Sized to the full UBO capacity so the WGSL's
        // fixed-size array<vec4<u32>, SP3_STAGED_MIX_POWS_UBO_VEC4_CAPACITY>
        // declaration matches the buffer size exactly. Each `eval_check`
        // call writes only `mix_pow_words` u32s starting at offset 0;
        // the remaining slots are unused.
        let mix_pows_capacity_bytes =
            (SP3_STAGED_MIX_POWS_UBO_VEC4_CAPACITY * 16) as u64;
        if (mix_pow_words * 4) as u64 > mix_pows_capacity_bytes {
            return Err(anyhow!(
                "staged eval_check: mix_pow_words ({}) exceeds UBO capacity ({} u32)",
                mix_pow_words,
                mix_pows_capacity_bytes / 4
            ));
        }
        let mix_pows_buf = self.create_buffer(
            "webgpu_staged_eval_check_mix_pows",
            mix_pows_capacity_bytes,
            WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;
        let params_buf = self.create_buffer(
            "webgpu_staged_eval_check_params",
            96,
            WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;
        // SP3 iter 7m: workgroup_size is shared across stages (codegen
        // picks the same value from `plan.fp_slots`/`plan.mix_slots`).
        // If multi.stages is empty (defensive — should never happen),
        // fall back to 1.
        let workgroup_size = multi
            .stages
            .first()
            .map(|s| s.workgroup_size)
            .unwrap_or(1);
        let pipeline = StagedEvalCheckPipeline {
            layout,
            stages,
            workgroup_size,
            base_field_fp,
            fp_scratch_stride_u32: multi.fp_scratch_stride_u32,
            mix_scratch_stride_u32: multi.mix_scratch_stride_u32,
            fp_scratch,
            mix_tot_scratch,
            mix_mul_scratch,
            scratch_params_buf,
            mix_pows_buf,
            params_buf,
            mix_pow_words,
        };
        self.staged_eval_check_pipelines
            .borrow_mut()
            .insert(key, pipeline.clone());
        Ok(pipeline)
    }

    /// SP3 iter 5: dispatch the staged-WGSL `eval_check` kernel for a DEF.
    /// Mirrors `dispatch_eval_check_poly_ext_interpreted` but without the
    /// runtime opcode stream — the per-DEF body is baked into the cached
    /// pipeline at first call. Returns `Ok(false)` if any GPU buffer is
    /// missing (caller falls through to interpreter); returns `Err` on
    /// codegen / compile / WebGPU API failure.
    fn dispatch_eval_check_poly_ext_staged(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        logical_globals: &[&WebGpuBuffer<BabyBearElem>; 2],
        taps: &TapSet<'_>,
        def: &PolyExtStepDef,
        poly_mix: BabyBearExtElem,
        domain_u32: u32,
        params: [u32; 12],
        base_field_fp: bool,
    ) -> Result<bool> {
        let (Some(check_gpu), Some(group0_gpu), Some(group1_gpu), Some(group2_gpu)) = (
            check.raw_buffer(),
            groups[0].raw_buffer(),
            groups[1].raw_buffer(),
            groups[2].raw_buffer(),
        ) else {
            return Ok(false);
        };
        let (Some(global0_gpu), Some(global1_gpu)) = (
            logical_globals[0].raw_buffer(),
            logical_globals[1].raw_buffer(),
        ) else {
            return Ok(false);
        };

        let pipeline = self.staged_eval_check_pipeline(def, taps, base_field_fp)?;

        let _timer = WebGpuStageTimer::new(format!(
            "eval_check_staged_submit domain={domain_u32} base_field_fp={base_field_fp}"
        ));

        let mix_pows = eval_check_mix_pows(def, poly_mix)?;
        let mut mix_pow_words = Vec::with_capacity(mix_pows.len() * BabyBearExtElem::EXT_SIZE);
        for value in mix_pows {
            mix_pow_words.extend(ext_words(value));
        }
        debug_assert_eq!(
            mix_pow_words.len(),
            pipeline.mix_pow_words,
            "staged mix_pows word count must match pipeline reservation"
        );
        // SP3 iter 7f: rewrite the cached `mix_pows` storage buffer
        // (owned by the pipeline) instead of creating a fresh one per
        // eval_check call.
        self.write_buffer_named(
            &pipeline.mix_pows_buf,
            "webgpu_staged_eval_check_mix_pows",
            0,
            bytemuck::cast_slice(&mix_pow_words),
        )?;

        // Params layout matches the interpreter's; staged kernel ignores the
        // `instr_count` / `instr_base` / `ret_mix_slot` slots.
        let params_words = [
            params[0],
            params[1],
            params[2],
            params[3],
            params[4],
            params[5],
            domain_u32,
            0u32, // instr_count
            0u32, // instr_base
            0u32, // mix_pows_base (fresh buffer, base = 0)
            0u32, // ret_mix_slot (baked into write_check at codegen time)
            domain_u32,
            0u32, // cycle_base
            0u32, // group0_chunk_base
            domain_u32, // group0_chunk_rows
            0u32, // group1_chunk_base
            domain_u32, // group1_chunk_rows
            0u32, // group2_chunk_base
            domain_u32, // group2_chunk_rows
            0u32, // _pad0
            params[8],
            params[9],
            params[10],
            params[11],
        ];
        // SP3 iter 7f: rewrite the cached params UBO (owned by the
        // pipeline) instead of allocating a fresh 96-byte uniform per
        // call.
        self.write_buffer_named(
            &pipeline.params_buf,
            "webgpu_staged_eval_check_params",
            0,
            bytemuck::cast_slice(&params_words),
        )?;

        // SP3 iter 7g: CUDA-shape dispatch. Closest analog to CUDA's
        // single `eval_check<<<grid, block>>>` launch covering the whole
        // domain — we issue ONE `dispatch_workgroups(tile_size,
        // num_tiles, 1)` per stage instead of iter 7e/7f's
        // `num_tiles * num_stages` setBindGroup+dispatch loop. Threads
        // compute `cycle = gid.y * tile_size + gid.x` inline, so no
        // per-tile UBO offset is needed. scratch_params is a single
        // 16-byte pipeline-constant UBO; pipeline/bind_group are bound
        // once and reused across all stages.
        let tile_size = SP3_STAGED_TILE_SIZE.min(domain_u32);
        let num_tiles = (domain_u32 + tile_size - 1) / tile_size;

        let bind_group = self.create_bind_group(
            "webgpu_staged_eval_check_bind_group",
            &pipeline.layout,
            &[
                WebGpuBufferBinding::new(0, check_gpu),
                WebGpuBufferBinding::new(1, group0_gpu),
                WebGpuBufferBinding::new(2, group1_gpu),
                WebGpuBufferBinding::new(3, group2_gpu),
                WebGpuBufferBinding::new(4, global0_gpu),
                WebGpuBufferBinding::new(5, global1_gpu),
                // binding 6 (instrs) omitted; not in pipeline layout
                WebGpuBufferBinding::new(7, &pipeline.mix_pows_buf),
                WebGpuBufferBinding {
                    binding: 8,
                    buffer: &pipeline.params_buf,
                    offset: 0,
                    size: Some(96),
                },
                WebGpuBufferBinding {
                    binding: 9,
                    buffer: &pipeline.scratch_params_buf,
                    offset: 0,
                    size: Some(16),
                },
                WebGpuBufferBinding::new(10, &pipeline.fp_scratch),
                WebGpuBufferBinding::new(11, &pipeline.mix_tot_scratch),
                WebGpuBufferBinding::new(12, &pipeline.mix_mul_scratch),
            ],
        )?;

        // SP3 iter 7h: each stage gets its OWN compute pass within one
        // encoder. WebGPU does not guarantee that storage writes from
        // dispatch N are visible to dispatch N+1 inside the same compute
        // pass — only across pass boundaries does the implementation
        // insert a memory barrier. Iter 7c through 7g packed all stages
        // into one pass; if stage K+1 reads scratch that stage K wrote,
        // it could read stale data, corrupt the check buffer, and
        // cascade into a bad merkle/FRI allocation that SIGKILLs Chrome
        // during finalize_async. CUDA's sequential kernel launches each
        // implicitly synchronize — this is the closest WebGPU analog.
        // Single submit is preserved (one encoder.finish()), so queue
        // pressure stays at iter 7e's level.
        // SP3 iter 7m: dispatch `(tile_size / workgroup_size, num_tiles,
        // 1)` workgroups per stage. Each workgroup runs `workgroup_size`
        // threads. Total threads = `tile_size * num_tiles` >= domain.
        // `tile_size` is 4096 and `workgroup_size` is a power of 2 in
        // [1, 64], so the division is exact.
        let workgroup_count_x = tile_size / pipeline.workgroup_size;
        let encoder = self.device.create_command_encoder();
        for stage in &pipeline.stages {
            let pass = encoder.begin_compute_pass();
            pass.set_pipeline(&stage.pipeline);
            pass.set_bind_group(0, Some(&bind_group));
            pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
                workgroup_count_x,
                num_tiles,
                1,
            );
            pass.end();
        }
        self.submit(encoder.finish());

        // SP3 iter 7l (2026-05-12): pipeline cache is re-enabled.
        // Iter 7j evicted per-call as a diagnostic — proved persistent
        // cached state isn't the SIGKILL cause, but added significant
        // recompile cost (each call recompiled 2-4 staged shaders).
        // Revert to keep the pipeline cache so subsequent eval_check
        // calls hitting the same DEF reuse compiled stages.
        self.record_gpu_result_authoritative("eval_check", true);
        Ok(true)
    }

    fn can_allocate_gpu_buffer(&self, byte_len: u64) -> bool {
        byte_len <= self.max_buffer_size && byte_len <= MAX_EXACT_JS_INTEGER
    }

    fn record_gpu_result_with_cpu_mirror(&self, name: &'static str, gpu_used: bool) {
        if gpu_used {
            self.diagnostics.record_gpu_dispatch(name);
            self.diagnostics.record_cpu_mirror(name);
        } else {
            self.diagnostics.record_cpu_fallback(name);
        }
    }

    fn record_gpu_result_authoritative(&self, name: &'static str, gpu_used: bool) {
        if gpu_used {
            self.diagnostics.record_gpu_dispatch(name);
        } else {
            self.diagnostics.record_cpu_fallback(name);
        }
    }

    fn finish_hal_op<T>(
        &self,
        name: &'static str,
        gpu_used: bool,
        output: &WebGpuBuffer<T>,
        cpu_mirror: impl FnOnce(),
    ) {
        if self.gpu_authoritative() && gpu_used {
            self.record_gpu_result_authoritative(name, gpu_used);
            output.mark_gpu_dirty();
        } else {
            cpu_mirror();
            self.record_gpu_result_with_cpu_mirror(name, gpu_used);
            output.mark_cpu_result(gpu_used);
        }
    }

    pub(crate) fn storage_binding_fits<T>(&self, buffer: &WebGpuBuffer<T>) -> bool
    where
        T: Clone + Debug + PartialEq,
    {
        byte_len_for::<T>(buffer.size()) <= self.max_storage_binding_bytes()
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_eval_check_poly_ext_interpreted(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        logical_globals: &[&WebGpuBuffer<BabyBearElem>; 2],
        taps: &TapSet<'_>,
        def: &PolyExtStepDef,
        poly_mix: BabyBearExtElem,
        domain_u32: u32,
        params: [u32; 12],
        base_field_fp: bool,
    ) -> Result<bool> {
        let (Some(group0_gpu), Some(group1_gpu), Some(group2_gpu)) = (
            groups[0].raw_buffer(),
            groups[1].raw_buffer(),
            groups[2].raw_buffer(),
        ) else {
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        };
        self.dispatch_eval_check_poly_ext_interpreted_with_groups(
            check,
            logical_globals,
            [group0_gpu, group1_gpu, group2_gpu],
            [params[1], params[2], params[3]],
            [0, 0, 0],
            [domain_u32, domain_u32, domain_u32],
            domain_u32,
            0,
            taps,
            def,
            poly_mix,
            domain_u32,
            params,
            base_field_fp,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_eval_check_poly_ext_interpreted_with_groups(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        logical_globals: &[&WebGpuBuffer<BabyBearElem>; 2],
        group_gpus: [&web_sys::GpuBuffer; 3],
        group_bases: [u32; 3],
        group_chunk_bases: [u32; 3],
        group_chunk_rows: [u32; 3],
        dispatch_count: u32,
        cycle_base: u32,
        taps: &TapSet<'_>,
        def: &PolyExtStepDef,
        poly_mix: BabyBearExtElem,
        domain_u32: u32,
        params: [u32; 12],
        base_field_fp: bool,
    ) -> Result<bool> {
        let (instructions, fp_slots, mix_slots, ret_mix_slot) = match if base_field_fp {
            eval_check_base_interpreter_instructions(taps, def)
        } else {
            eval_check_interpreter_instructions(taps, def)
        } {
            Ok(program) => program,
            Err(err) => {
                log_webgpu_stage(&format!(
                    "webgpu eval_check interpreter build failed: {err}"
                ));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
        };
        if instructions.is_empty() {
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }
        let instruction_count = instructions.len() / WEBGPU_EVAL_CHECK_INSTRUCTION_WORDS;
        let base_private_parallel =
            base_field_fp && fp_slots <= WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS;
        let base_workgroup_size = if base_private_parallel {
            WEBGPU_EVAL_CHECK_INTERPRETER_WORKGROUP_SIZE
        } else if base_field_fp {
            self.eval_check_base_workgroup_lanes(fp_slots, mix_slots)
        } else {
            WEBGPU_EVAL_CHECK_INTERPRETER_WORKGROUP_SIZE
        };
        let interpreter_label = if base_private_parallel {
            "eval_check_base_private_interpreter_submit"
        } else if base_field_fp {
            "eval_check_base_interpreter_submit"
        } else {
            "eval_check_interpreter_submit"
        };
        let _timer = WebGpuStageTimer::new(format!(
            "{interpreter_label} domain={} dispatch_count={} cycle_base={} instructions={} fp_slots={} mix_slots={} workgroup_size={}",
            domain_u32, dispatch_count, cycle_base, instruction_count, fp_slots, mix_slots, base_workgroup_size
        ));

        let mix_pows = eval_check_mix_pows(def, poly_mix)?;
        let mut mix_pow_words = Vec::with_capacity(mix_pows.len() * BabyBearExtElem::EXT_SIZE);
        for value in mix_pows {
            mix_pow_words.extend(ext_words(value));
        }

        let instructions_name = if base_field_fp {
            "webgpu_eval_check_base_interpreter_instructions"
        } else {
            "webgpu_eval_check_interpreter_instructions"
        };
        let instructions_gpu =
            self.create_storage_buffer(instructions_name, byte_len_for::<u32>(instructions.len()))?;
        self.write_buffer_named(
            &instructions_gpu,
            instructions_name,
            0,
            bytemuck::cast_slice(&instructions),
        )?;
        let mix_pows_name = if base_field_fp {
            "webgpu_eval_check_base_interpreter_mix_pows"
        } else {
            "webgpu_eval_check_interpreter_mix_pows"
        };
        let mix_pows_gpu =
            self.create_storage_buffer(mix_pows_name, byte_len_for::<u32>(mix_pow_words.len()))?;
        self.write_buffer_named(
            &mix_pows_gpu,
            mix_pows_name,
            0,
            bytemuck::cast_slice(&mix_pow_words),
        )?;

        let params = [
            params[0],
            group_bases[0],
            group_bases[1],
            group_bases[2],
            params[4],
            params[5],
            domain_u32,
            u32::try_from(instruction_count)
                .expect("WebGPU eval_check instruction count exceeds u32"),
            0,
            0,
            u32::try_from(ret_mix_slot).expect("WebGPU eval_check ret slot exceeds u32"),
            dispatch_count,
            cycle_base,
            group_chunk_bases[0],
            group_chunk_rows[0],
            group_chunk_bases[1],
            group_chunk_rows[1],
            group_chunk_bases[2],
            group_chunk_rows[2],
            0,
            params[8],
            params[9],
            params[10],
            params[11],
        ];
        let params = self.create_uniform_buffer(
            "webgpu_eval_check_interpreter_params",
            bytemuck::cast_slice(&params),
        )?;

        let pipeline = match self.eval_check_interpreter_pipeline(
            base_field_fp,
            base_private_parallel,
            fp_slots,
            mix_slots,
            base_workgroup_size,
        ) {
            Ok(pipeline) => pipeline,
            Err(err) => {
                log_webgpu_stage(&format!(
                    "webgpu eval_check interpreter pipeline failed: {err}"
                ));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
        };

        let bind_group = self.create_bind_group(
            "webgpu_eval_check_interpreter_bind_group",
            &pipeline.layout,
            &[
                WebGpuBufferBinding::new(
                    0,
                    check
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing interpreted eval_check check buffer"))?,
                ),
                WebGpuBufferBinding::new(1, group_gpus[0]),
                WebGpuBufferBinding::new(2, group_gpus[1]),
                WebGpuBufferBinding::new(3, group_gpus[2]),
                WebGpuBufferBinding::new(
                    4,
                    logical_globals[0]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing interpreted eval_check global 0 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    5,
                    logical_globals[1]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing interpreted eval_check global 1 buffer"))?,
                ),
                WebGpuBufferBinding::new(6, &instructions_gpu),
                WebGpuBufferBinding::new(7, &mix_pows_gpu),
                WebGpuBufferBinding {
                    binding: 8,
                    buffer: &params,
                    offset: 0,
                    size: Some(96),
                },
            ],
        )?;
        let workgroups = if base_field_fp && !base_private_parallel {
            dispatch_count.div_ceil(base_workgroup_size)
        } else {
            dispatch_count.div_ceil(WEBGPU_EVAL_CHECK_INTERPRETER_WORKGROUP_SIZE)
        };
        self.dispatch_compute_1d(&pipeline.kernel, &bind_group, workgroups);
        check.mark_gpu_dirty();
        self.record_gpu_result_authoritative("eval_check", true);
        Ok(true)
    }

    fn dispatch_eval_check_pack_group_chunk(
        &self,
        group: &WebGpuBuffer<BabyBearElem>,
        cols: usize,
        domain: usize,
        chunk_base: usize,
        chunk_rows: usize,
    ) -> Result<Option<web_sys::GpuBuffer>> {
        if cols == 0 || chunk_rows == 0 {
            return Ok(None);
        }
        let Some(group_gpu) = group.raw_buffer() else {
            return Ok(None);
        };
        if !group.gpu_is_current() {
            return Ok(None);
        }

        let domain_bytes = byte_len_for::<BabyBearElem>(domain);
        if domain_bytes == 0
            || domain_bytes > self.max_storage_binding_bytes()
            || domain_bytes % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0
        {
            return Ok(None);
        }

        let group_base = group.byte_offset();
        if group_base % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0 {
            return Ok(None);
        }
        let max_cols_per_binding = (self.max_storage_binding_bytes() / domain_bytes) as usize;
        if max_cols_per_binding == 0 {
            return Ok(None);
        }

        let chunk_len = cols
            .checked_mul(chunk_rows)
            .ok_or_else(|| anyhow!("WebGPU eval_check group chunk length overflow"))?;
        let chunk_bytes = byte_len_for::<BabyBearElem>(chunk_len);
        if chunk_bytes == 0 || chunk_bytes > self.max_storage_binding_bytes() {
            return Ok(None);
        }

        let chunk_gpu =
            self.create_storage_buffer("webgpu_eval_check_interpreter_group_chunk", chunk_bytes)?;
        let layout = self.create_bind_group_layout(
            "webgpu_eval_check_pack_group_chunk_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_eval_check_pack_group_chunk",
            EVAL_CHECK_PACK_GROUP_CHUNK_WGSL,
            "main",
            &[layout.clone()],
        )?;

        for col_start in (0..cols).step_by(max_cols_per_binding) {
            let col_count = (cols - col_start).min(max_cols_per_binding);
            let col_offset = group_base
                .checked_add(byte_len_for::<BabyBearElem>(
                    col_start
                        .checked_mul(domain)
                        .ok_or_else(|| anyhow!("WebGPU eval_check pack column overflow"))?,
                ))
                .ok_or_else(|| anyhow!("WebGPU eval_check pack column offset overflow"))?;
            if col_offset % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0 {
                return Ok(None);
            }
            let binding_bytes = domain_bytes
                .checked_mul(
                    u64::try_from(col_count)
                        .expect("WebGPU eval_check pack column count exceeds u64"),
                )
                .ok_or_else(|| anyhow!("WebGPU eval_check pack binding size overflow"))?;
            let params = [
                u32::try_from(domain).expect("WebGPU eval_check pack domain exceeds u32"),
                u32::try_from(chunk_base).expect("WebGPU eval_check pack chunk base exceeds u32"),
                u32::try_from(chunk_rows).expect("WebGPU eval_check pack chunk rows exceeds u32"),
                u32::try_from(
                    col_start
                        .checked_mul(chunk_rows)
                        .ok_or_else(|| anyhow!("WebGPU eval_check pack dst offset overflow"))?,
                )
                .expect("WebGPU eval_check pack dst offset exceeds u32"),
                u32::try_from(col_count).expect("WebGPU eval_check pack col count exceeds u32"),
                0,
                0,
                0,
            ];
            let params = self.create_uniform_buffer(
                "webgpu_eval_check_pack_group_chunk_params",
                bytemuck::cast_slice(&params),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_eval_check_pack_group_chunk_bind_group",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, &chunk_gpu),
                    WebGpuBufferBinding {
                        binding: 1,
                        buffer: group_gpu,
                        offset: col_offset,
                        size: Some(binding_bytes),
                    },
                    WebGpuBufferBinding {
                        binding: 2,
                        buffer: &params,
                        offset: 0,
                        size: Some(32),
                    },
                ],
            )?;
            self.dispatch_compute(
                &kernel,
                &bind_group,
                u32::try_from(chunk_rows)
                    .expect("WebGPU eval_check pack chunk rows exceeds u32")
                    .div_ceil(WEBGPU_WORKGROUP_SIZE),
                u32::try_from(col_count).expect("WebGPU eval_check pack col count exceeds u32"),
                1,
            );
        }

        Ok(Some(chunk_gpu))
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_eval_check_poly_ext_interpreted_group_chunks(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        logical_globals: &[&WebGpuBuffer<BabyBearElem>; 2],
        taps: &TapSet<'_>,
        def: &PolyExtStepDef,
        poly_mix: BabyBearExtElem,
        domain_u32: u32,
        params: [u32; 12],
        base_field_fp: bool,
    ) -> Result<bool> {
        let chunked_groups = [
            groups[0].raw_buffer().is_none() || !self.storage_binding_fits(groups[0]),
            groups[1].raw_buffer().is_none() || !self.storage_binding_fits(groups[1]),
            groups[2].raw_buffer().is_none() || !self.storage_binding_fits(groups[2]),
        ];

        for (name, buffer) in [
            ("check", check),
            ("global0", logical_globals[0]),
            ("global1", logical_globals[1]),
        ] {
            if buffer.raw_buffer().is_none() {
                log_webgpu_stage(&format!(
                    "browser-prove:stage eval_check chunked_groups unavailable missing_gpu_buffer name={name}"
                ));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
            if !self.storage_binding_fits(buffer) {
                log_webgpu_stage(&format!(
                    "browser-prove:stage eval_check chunked_groups unavailable storage_binding name={} bytes={} max_binding={}",
                    name,
                    byte_len_for::<BabyBearElem>(buffer.size()),
                    self.max_storage_binding_bytes()
                ));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
        }

        for group_id in 0..3 {
            if chunked_groups[group_id] {
                if !groups[group_id].cpu_is_current()
                    && !(groups[group_id].raw_buffer().is_some()
                        && groups[group_id].gpu_is_current())
                {
                    log_webgpu_stage(&format!(
                        "browser-prove:stage eval_check chunked_groups unavailable stale_group group={group_id}"
                    ));
                    self.record_gpu_result_authoritative("eval_check", false);
                    return Ok(false);
                }
            } else if groups[group_id].raw_buffer().is_none()
                || !self.storage_binding_fits(groups[group_id])
            {
                log_webgpu_stage(&format!(
                    "browser-prove:stage eval_check chunked_groups unavailable group={group_id}"
                ));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
        }

        let domain = domain_u32 as usize;
        if domain == 0 {
            return Ok(true);
        }
        let max_back_rows = taps
            .taps()
            .filter(|tap| chunked_groups[tap.group()])
            .map(|tap| (tap.back() * INV_RATE) % domain)
            .max()
            .unwrap_or(0);
        let max_chunk_elems =
            (self.max_storage_binding_bytes() / mem::size_of::<BabyBearElem>() as u64) as usize;
        let mut max_dispatch_rows = domain;
        for group_id in 0..3 {
            if !chunked_groups[group_id] {
                continue;
            }
            let cols = taps.group_size(group_id);
            if cols == 0 {
                continue;
            }
            let max_chunk_rows = max_chunk_elems / cols;
            if max_chunk_rows <= max_back_rows {
                log_webgpu_stage(&format!(
                    "browser-prove:stage eval_check chunked_groups unavailable group={} rows_per_chunk={} max_back_rows={} cols={}",
                    group_id, max_chunk_rows, max_back_rows, cols
                ));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
            max_dispatch_rows = max_dispatch_rows.min(max_chunk_rows - max_back_rows);
        }
        log_webgpu_stage(&format!(
            "browser-prove:stage eval_check chunked_groups start domain={} groups={:?} max_back_rows={} dispatch_rows={}",
            domain, chunked_groups, max_back_rows, max_dispatch_rows
        ));

        let mut cycle_base = 0usize;
        while cycle_base < domain {
            let dispatch_rows = max_dispatch_rows.min(domain - cycle_base);
            let chunk_rows = dispatch_rows + max_back_rows;
            let chunk_base = (cycle_base + domain - max_back_rows) % domain;
            let mut chunk_gpus: [Option<web_sys::GpuBuffer>; 3] = [None, None, None];

            for group_id in 0..3 {
                if !chunked_groups[group_id] {
                    continue;
                }
                let cols = taps.group_size(group_id);
                if let Some(chunk_gpu) = self.dispatch_eval_check_pack_group_chunk(
                    groups[group_id],
                    cols,
                    domain,
                    chunk_base,
                    chunk_rows,
                )? {
                    chunk_gpus[group_id] = Some(chunk_gpu);
                    continue;
                }

                ensure!(
                    groups[group_id].cpu_is_current(),
                    "WebGPU chunked eval_check group {group_id} requires a current CPU shadow"
                );
                let mut chunk = vec![BabyBearElem::ZERO; cols * chunk_rows];
                groups[group_id].cpu.view(|source| {
                    for col in 0..cols {
                        let src_col = &source[col * domain..(col + 1) * domain];
                        let dst_col = &mut chunk[col * chunk_rows..(col + 1) * chunk_rows];
                        for (local_row, dst) in dst_col.iter_mut().enumerate() {
                            *dst = src_col[(chunk_base + local_row) % domain];
                        }
                    }
                });

                let chunk_gpu = self.create_storage_buffer(
                    "webgpu_eval_check_interpreter_group_chunk",
                    byte_len_for::<BabyBearElem>(chunk.len()),
                )?;
                self.write_buffer_named(
                    &chunk_gpu,
                    "webgpu_eval_check_interpreter_group_chunk",
                    0,
                    bytemuck::cast_slice(&chunk),
                )?;
                chunk_gpus[group_id] = Some(chunk_gpu);
            }

            let group0_gpu = chunk_gpus[0]
                .as_ref()
                .or_else(|| groups[0].raw_buffer())
                .ok_or_else(|| anyhow!("missing eval_check group 0 chunk buffer"))?;
            let group1_gpu = chunk_gpus[1]
                .as_ref()
                .or_else(|| groups[1].raw_buffer())
                .ok_or_else(|| anyhow!("missing eval_check group 1 chunk buffer"))?;
            let group2_gpu = chunk_gpus[2]
                .as_ref()
                .or_else(|| groups[2].raw_buffer())
                .ok_or_else(|| anyhow!("missing eval_check group 2 chunk buffer"))?;

            let group_bases = [
                if chunked_groups[0] { 0 } else { params[1] },
                if chunked_groups[1] { 0 } else { params[2] },
                if chunked_groups[2] { 0 } else { params[3] },
            ];
            let group_chunk_bases = [
                if chunked_groups[0] {
                    u32::try_from(chunk_base)
                        .expect("WebGPU eval_check group 0 chunk base exceeds u32")
                } else {
                    0
                },
                if chunked_groups[1] {
                    u32::try_from(chunk_base)
                        .expect("WebGPU eval_check group 1 chunk base exceeds u32")
                } else {
                    0
                },
                if chunked_groups[2] {
                    u32::try_from(chunk_base)
                        .expect("WebGPU eval_check group 2 chunk base exceeds u32")
                } else {
                    0
                },
            ];
            let group_chunk_rows = [
                if chunked_groups[0] {
                    u32::try_from(chunk_rows)
                        .expect("WebGPU eval_check group 0 chunk rows exceeds u32")
                } else {
                    domain_u32
                },
                if chunked_groups[1] {
                    u32::try_from(chunk_rows)
                        .expect("WebGPU eval_check group 1 chunk rows exceeds u32")
                } else {
                    domain_u32
                },
                if chunked_groups[2] {
                    u32::try_from(chunk_rows)
                        .expect("WebGPU eval_check group 2 chunk rows exceeds u32")
                } else {
                    domain_u32
                },
            ];

            let dispatched = self.dispatch_eval_check_poly_ext_interpreted_with_groups(
                check,
                logical_globals,
                [group0_gpu, group1_gpu, group2_gpu],
                group_bases,
                group_chunk_bases,
                group_chunk_rows,
                u32::try_from(dispatch_rows).expect("WebGPU eval_check dispatch rows exceeds u32"),
                u32::try_from(cycle_base).expect("WebGPU eval_check cycle base exceeds u32"),
                taps,
                def,
                poly_mix,
                domain_u32,
                params,
                base_field_fp,
            )?;
            if !dispatched {
                return Ok(false);
            }
            cycle_base += dispatch_rows;
        }

        log_webgpu_stage("browser-prove:stage eval_check chunked_groups done");
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_eval_check_poly_ext_split(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        logical_globals: &[&WebGpuBuffer<BabyBearElem>; 2],
        taps: &TapSet<'_>,
        def: &PolyExtStepDef,
        poly_mix: BabyBearExtElem,
        domain_u32: u32,
        params: [u32; 12],
    ) -> Result<bool> {
        let program = eval_check_program(def)?;
        let terms = eval_check_flatten_terms(&program, def.ret)?;
        if terms.is_empty() {
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }

        let term_chunks = match eval_check_split_term_chunks(&program, &terms) {
            Ok(chunks) => chunks,
            Err(err) => {
                log_webgpu_stage(&format!("webgpu eval_check split chunking failed: {err}"));
                self.record_gpu_result_authoritative("eval_check", false);
                return Ok(false);
            }
        };

        let mut wgsl_chunks = Vec::new();
        for (chunk_idx, terms) in term_chunks.iter().enumerate() {
            let wgsl = match build_eval_check_split_wgsl(taps, &program, terms, chunk_idx == 0) {
                Ok(wgsl) => wgsl,
                Err(err) => {
                    log_webgpu_stage(&format!(
                        "webgpu eval_check split WGSL build failed at chunk {chunk_idx}: {err}"
                    ));
                    self.record_gpu_result_authoritative("eval_check", false);
                    return Ok(false);
                }
            };
            wgsl_chunks.push(wgsl);
        }

        let mix_pows = eval_check_all_mix_pows(def, poly_mix)?;
        let mut mix_pow_words = Vec::with_capacity(mix_pows.len() * BabyBearExtElem::EXT_SIZE);
        for value in mix_pows {
            mix_pow_words.extend(ext_words(value));
        }
        let _timer = WebGpuStageTimer::new(format!(
            "eval_check_split_submit domain={} chunks={}",
            domain_u32,
            term_chunks.len()
        ));
        let mix_pows_gpu = self.create_storage_buffer(
            "webgpu_eval_check_split_mix_pows",
            byte_len_for::<u32>(mix_pow_words.len()),
        )?;
        self.write_buffer_named(
            &mix_pows_gpu,
            "webgpu_eval_check_split_mix_pows",
            0,
            bytemuck::cast_slice(&mix_pow_words),
        )?;

        let params = self.create_uniform_buffer(
            "webgpu_eval_check_split_params",
            bytemuck::cast_slice(&params),
        )?;
        let layout = self.create_bind_group_layout(
            "webgpu_eval_check_split_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::read_only_storage(6, 0),
                WebGpuBindingLayout::uniform(7, 48),
            ],
        )?;

        let mut kernels = Vec::with_capacity(wgsl_chunks.len());
        for (chunk_idx, wgsl) in wgsl_chunks.iter().enumerate() {
            let kernel = match self.create_compute_kernel(
                "webgpu_eval_check_split",
                wgsl,
                "main",
                &[layout.clone()],
            ) {
                Ok(kernel) => kernel,
                Err(err) => {
                    log_webgpu_stage(&format!(
                        "webgpu eval_check split pipeline failed at chunk {chunk_idx}: {err}"
                    ));
                    self.record_gpu_result_authoritative("eval_check", false);
                    return Ok(false);
                }
            };
            kernels.push(kernel);
        }

        let bind_group = self.create_bind_group(
            "webgpu_eval_check_split_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(
                    0,
                    check
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing split eval_check check buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    1,
                    groups[0]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing split eval_check group 0 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    2,
                    groups[1]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing split eval_check group 1 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    3,
                    groups[2]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing split eval_check group 2 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    4,
                    logical_globals[0]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing split eval_check global 0 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    5,
                    logical_globals[1]
                        .raw_buffer()
                        .ok_or_else(|| anyhow!("missing split eval_check global 1 buffer"))?,
                ),
                WebGpuBufferBinding::new(6, &mix_pows_gpu),
                WebGpuBufferBinding {
                    binding: 7,
                    buffer: &params,
                    offset: 0,
                    size: Some(48),
                },
            ],
        )?;
        let workgroups = domain_u32.div_ceil(WEBGPU_WORKGROUP_SIZE);
        self.dispatch_compute_1d_sequence(kernels.as_slice(), &bind_group, workgroups);
        check.mark_gpu_dirty();
        self.record_gpu_result_authoritative("eval_check", true);
        Ok(true)
    }

    /// Dispatch a generated straight-line WebGPU check-polynomial evaluator.
    ///
    /// This is intentionally conservative while the full circuit path is being
    /// moved to GPU-authoritative execution: unsupported shapes return
    /// `Ok(false)` so callers can use the portable CPU fallback.
    pub fn dispatch_eval_check_poly_ext(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        globals: &[&WebGpuBuffer<BabyBearElem>],
        taps: &TapSet<'_>,
        def: &PolyExtStepDef,
        poly_mix: BabyBearExtElem,
        po2: usize,
        steps: usize,
    ) -> Result<bool> {
        if !self.eval_check_gpu_enabled.get() {
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }
        if INV_RATE != 4 || taps.num_groups() != 3 || groups.len() != 3 || globals.len() != 2 {
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }

        let domain = steps
            .checked_mul(INV_RATE)
            .ok_or_else(|| anyhow!("WebGPU eval_check domain overflow"))?;
        if domain == 0 {
            return Ok(true);
        }
        ensure!(
            check.size() == BabyBearExtElem::EXT_SIZE * domain,
            "WebGPU eval_check check size mismatch: got {}, expected {}",
            check.size(),
            BabyBearExtElem::EXT_SIZE * domain
        );
        for (group_id, group) in groups.iter().enumerate() {
            let expected = taps.group_size(group_id) * domain;
            ensure!(
                group.size() == expected,
                "WebGPU eval_check group {group_id} size mismatch: got {}, expected {expected}",
                group.size()
            );
        }

        let logical_globals = [globals[1], globals[0]];
        let mut min_global_sizes = [0usize; 2];
        for op in def.block {
            if let PolyExtStep::GetGlobal(arg, offset) = op {
                if *arg >= min_global_sizes.len() {
                    self.record_gpu_result_authoritative("eval_check", false);
                    return Ok(false);
                }
                min_global_sizes[*arg] = min_global_sizes[*arg].max(offset + 1);
            }
        }
        for (idx, min_size) in min_global_sizes.into_iter().enumerate() {
            ensure!(
                logical_globals[idx].size() >= min_size,
                "WebGPU eval_check global {idx} too small: got {}, need at least {min_size}",
                logical_globals[idx].size()
            );
        }

        let Some(check_gpu) = check.raw_buffer() else {
            log_webgpu_stage(
                "browser-prove:stage eval_check fallback missing_gpu_buffer name=check",
            );
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        };
        let group_gpus = [
            groups[0].raw_buffer(),
            groups[1].raw_buffer(),
            groups[2].raw_buffer(),
        ];
        for group_id in 0..3 {
            if group_gpus[group_id].is_none() {
                log_webgpu_stage(&format!(
                    "browser-prove:stage eval_check chunked_groups candidate missing_gpu_buffer group={group_id}"
                ));
            }
        }
        let Some(global0_gpu) = logical_globals[0].raw_buffer() else {
            log_webgpu_stage(
                "browser-prove:stage eval_check fallback missing_gpu_buffer name=global0",
            );
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        };
        let Some(global1_gpu) = logical_globals[1].raw_buffer() else {
            log_webgpu_stage(
                "browser-prove:stage eval_check fallback missing_gpu_buffer name=global1",
            );
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        };
        let group_storage_fits = [
            self.storage_binding_fits(groups[0]),
            self.storage_binding_fits(groups[1]),
            self.storage_binding_fits(groups[2]),
        ];
        if !self.storage_binding_fits(check)
            || !self.storage_binding_fits(logical_globals[0])
            || !self.storage_binding_fits(logical_globals[1])
        {
            let max_binding = self.max_storage_binding_bytes();
            for (name, elem_count) in [
                ("check", check.size()),
                ("group0", groups[0].size()),
                ("group1", groups[1].size()),
                ("global0", logical_globals[0].size()),
                ("global1", logical_globals[1].size()),
            ] {
                let byte_len = byte_len_for::<BabyBearElem>(elem_count);
                if byte_len > max_binding {
                    log_webgpu_stage(&format!(
                        "browser-prove:stage eval_check fallback storage_binding name={} bytes={} max_binding={}",
                        name, byte_len, max_binding
                    ));
                }
            }
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }
        for group_id in 0..3 {
            if !group_storage_fits[group_id] {
                log_webgpu_stage(&format!(
                    "browser-prove:stage eval_check chunked_groups candidate storage_binding group={} bytes={} max_binding={}",
                    group_id,
                    byte_len_for::<BabyBearElem>(groups[group_id].size()),
                    self.max_storage_binding_bytes()
                ));
            }
        }
        let group_can_bind = [
            group_gpus[0].is_some() && group_storage_fits[0],
            group_gpus[1].is_some() && group_storage_fits[1],
            group_gpus[2].is_some() && group_storage_fits[2],
        ];

        let check_base =
            u32::try_from(check.elem_offset).map_err(|_| anyhow!("check offset exceeds u32"))?;
        let group0_base = u32::try_from(groups[0].elem_offset)
            .map_err(|_| anyhow!("group 0 offset exceeds u32"))?;
        let group1_base = u32::try_from(groups[1].elem_offset)
            .map_err(|_| anyhow!("group 1 offset exceeds u32"))?;
        let group2_base = u32::try_from(groups[2].elem_offset)
            .map_err(|_| anyhow!("group 2 offset exceeds u32"))?;
        let global0_base = u32::try_from(logical_globals[0].elem_offset)
            .map_err(|_| anyhow!("global 0 offset exceeds u32"))?;
        let global1_base = u32::try_from(logical_globals[1].elem_offset)
            .map_err(|_| anyhow!("global 1 offset exceeds u32"))?;
        let domain_u32 =
            u32::try_from(domain).map_err(|_| anyhow!("WebGPU eval_check domain exceeds u32"))?;

        let use_chunked_groups = def.block.len() > WEBGPU_EVAL_CHECK_MAX_POLY_EXT_STEPS
            && !WEBGPU_EVAL_CHECK_ENABLE_SPLIT
            && !group_can_bind.iter().all(|can_bind| *can_bind);

        for (idx, group) in groups.iter().enumerate() {
            if !use_chunked_groups || group_can_bind[idx] {
                group.sync_cpu_to_gpu(self)?;
            }
        }
        for global in &logical_globals {
            global.sync_cpu_to_gpu(self)?;
        }

        let invs = eval_check_zerofier_inv_words(po2, steps);
        let params = [
            check_base,
            group0_base,
            group1_base,
            group2_base,
            global0_base,
            global1_base,
            domain_u32,
            0,
            invs[0],
            invs[1],
            invs[2],
            invs[3],
        ];

        // SP3 iter 5 (2026-05-12): try the staged-WGSL fast path first when
        // the runtime flag is set and all groups bind in a single dispatch.
        // Default `staged_eval_check_enabled = false`, so this branch is
        // dead code on the production path until browser parity tests
        // (iter 7) opt in per-fixture. Any failure (codegen, compile,
        // dispatch) falls through to the interpreter below.
        //
        // SP3 iter 7i (2026-05-12): only attempt staged for DEFs with
        // enough ops to make staging worthwhile (rv32im production has
        // ~20k+ ops; recursion's DEF is much smaller and currently
        // SIGKILLs Chrome somewhere in the lift's finalize_async flow
        // when staged. Limit the staged path to the heavy DEFs where
        // it's the perf win, and let the recursion lift use the proven
        // interpreter path until iter 7i+1 root-causes the recursion
        // breakage. Threshold of 8000 keeps rv32im above and recursion
        // / smaller DEFs below.
        if self.staged_eval_check_enabled.get()
            && def.block.len() >= SP3_STAGED_MIN_BLOCK_OPS
            && group_can_bind.iter().all(|can_bind| *can_bind)
        {
            // Base-field DEFs (rv32im) take the optimized `FieldMode::Base`
            // path; DEFs that use `ConstExt` must use `FieldMode::Ext`.
            let base_field_fp = !def
                .block
                .iter()
                .any(|op| matches!(op, PolyExtStep::ConstExt(_, _, _, _)));
            match self.dispatch_eval_check_poly_ext_staged(
                check,
                groups,
                &logical_globals,
                taps,
                def,
                poly_mix,
                domain_u32,
                params,
                base_field_fp,
            ) {
                Ok(true) => return Ok(true),
                Ok(false) => {
                    log_webgpu_stage(
                        "browser-prove:stage eval_check staged unavailable; falling back to interpreter",
                    );
                }
                Err(err) => {
                    log_webgpu_stage(&format!(
                        "browser-prove:stage eval_check staged failed: {err}; falling back to interpreter"
                    ));
                }
            }
        }

        if def.block.len() > WEBGPU_EVAL_CHECK_MAX_POLY_EXT_STEPS {
            let base_interpreter_program = eval_check_base_interpreter_instructions(taps, def);
            if let Ok((_, fp_slots, _, _)) = &base_interpreter_program {
                if WEBGPU_EVAL_CHECK_ENABLE_SPLIT
                    && *fp_slots > WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS
                    && group_can_bind.iter().all(|can_bind| *can_bind)
                {
                    match self.dispatch_eval_check_poly_ext_split(
                        check,
                        groups,
                        &logical_globals,
                        taps,
                        def,
                        poly_mix,
                        domain_u32,
                        params,
                    ) {
                        Ok(true) => return Ok(true),
                        Ok(false) => {
                            log_webgpu_stage(
                                "browser-prove:stage eval_check split unavailable; falling back to base interpreter",
                            );
                        }
                        Err(err) => {
                            log_webgpu_stage(&format!(
                                "browser-prove:stage eval_check split failed: {err}; falling back to base interpreter"
                            ));
                        }
                    }
                }
                if !group_can_bind.iter().all(|can_bind| *can_bind) {
                    return self.dispatch_eval_check_poly_ext_interpreted_group_chunks(
                        check,
                        groups,
                        &logical_globals,
                        taps,
                        def,
                        poly_mix,
                        domain_u32,
                        params,
                        true,
                    );
                }
                return self.dispatch_eval_check_poly_ext_interpreted(
                    check,
                    groups,
                    &logical_globals,
                    taps,
                    def,
                    poly_mix,
                    domain_u32,
                    params,
                    true,
                );
            }

            let interpreter_fits = eval_check_interpreter_instructions(taps, def).is_ok();
            if interpreter_fits {
                if !group_can_bind.iter().all(|can_bind| *can_bind) {
                    return self.dispatch_eval_check_poly_ext_interpreted_group_chunks(
                        check,
                        groups,
                        &logical_globals,
                        taps,
                        def,
                        poly_mix,
                        domain_u32,
                        params,
                        false,
                    );
                }
                return self.dispatch_eval_check_poly_ext_interpreted(
                    check,
                    groups,
                    &logical_globals,
                    taps,
                    def,
                    poly_mix,
                    domain_u32,
                    params,
                    false,
                );
            }

            if WEBGPU_EVAL_CHECK_ENABLE_SPLIT && group_can_bind.iter().all(|can_bind| *can_bind) {
                return self.dispatch_eval_check_poly_ext_split(
                    check,
                    groups,
                    &logical_globals,
                    taps,
                    def,
                    poly_mix,
                    domain_u32,
                    params,
                );
            }

            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }
        if !group_can_bind.iter().all(|can_bind| *can_bind) {
            self.record_gpu_result_authoritative("eval_check", false);
            return Ok(false);
        }

        let mix_pows = eval_check_mix_pows(def, poly_mix)?;
        let mut mix_pow_words = Vec::with_capacity(mix_pows.len() * BabyBearExtElem::EXT_SIZE);
        for value in mix_pows {
            mix_pow_words.extend(ext_words(value));
        }
        let mix_pows_gpu = self.create_storage_buffer(
            "webgpu_eval_check_mix_pows",
            byte_len_for::<u32>(mix_pow_words.len()),
        )?;
        self.write_buffer_named(
            &mix_pows_gpu,
            "webgpu_eval_check_mix_pows",
            0,
            bytemuck::cast_slice(&mix_pow_words),
        )?;

        let params =
            self.create_uniform_buffer("webgpu_eval_check_params", bytemuck::cast_slice(&params))?;

        let wgsl = build_eval_check_wgsl(taps, def)?;
        let layout = self.create_bind_group_layout(
            "webgpu_eval_check_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::read_only_storage(5, 0),
                WebGpuBindingLayout::read_only_storage(6, 0),
                WebGpuBindingLayout::uniform(7, 48),
            ],
        )?;
        let kernel =
            self.create_compute_kernel("webgpu_eval_check", &wgsl, "main", &[layout.clone()])?;
        let bind_group = self.create_bind_group(
            "webgpu_eval_check_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, check_gpu),
                WebGpuBufferBinding::new(
                    1,
                    group_gpus[0].ok_or_else(|| anyhow!("missing eval_check group 0 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    2,
                    group_gpus[1].ok_or_else(|| anyhow!("missing eval_check group 1 buffer"))?,
                ),
                WebGpuBufferBinding::new(
                    3,
                    group_gpus[2].ok_or_else(|| anyhow!("missing eval_check group 2 buffer"))?,
                ),
                WebGpuBufferBinding::new(4, global0_gpu),
                WebGpuBufferBinding::new(5, global1_gpu),
                WebGpuBufferBinding::new(6, &mix_pows_gpu),
                WebGpuBufferBinding {
                    binding: 7,
                    buffer: &params,
                    offset: 0,
                    size: Some(48),
                },
            ],
        )?;
        let workgroups = domain_u32.div_ceil(WEBGPU_WORKGROUP_SIZE);
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
        check.mark_gpu_dirty();
        self.record_gpu_result_authoritative("eval_check", true);
        Ok(true)
    }

    pub(crate) fn can_dispatch_batch_interpolate_ntt(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
    ) -> bool {
        self.batch_interpolate_ntt_gpu_enabled.get()
            && (io.size() == 0 || (io.raw_buffer().is_some() && self.storage_binding_fits(io)))
    }

    pub(crate) fn can_dispatch_zk_shift(&self, io: &WebGpuBuffer<BabyBearElem>) -> bool {
        self.zk_shift_gpu_enabled.get()
            && (io.size() == 0 || (io.raw_buffer().is_some() && self.storage_binding_fits(io)))
    }

    fn can_dispatch_batch_bit_reverse(&self, io: &WebGpuBuffer<BabyBearElem>) -> bool {
        self.batch_bit_reverse_gpu_enabled.get()
            && (io.size() == 0 || (io.raw_buffer().is_some() && self.storage_binding_fits(io)))
    }

    fn can_dispatch_batch_expand_into_evaluate_ntt(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
    ) -> bool {
        if !self.batch_expand_into_evaluate_ntt_gpu_enabled.get() {
            return false;
        }
        if output.size() == 0 {
            return true;
        }
        let (Some(output_gpu), Some(input_gpu)) = (output.raw_buffer(), input.raw_buffer()) else {
            return false;
        };
        output_gpu != input_gpu
            && self.storage_binding_fits(output)
            && self.storage_binding_fits(input)
    }

    fn can_dispatch_batch_evaluate_any(
        &self,
        out: &WebGpuBuffer<BabyBearExtElem>,
        coeffs: &WebGpuBuffer<BabyBearElem>,
        poly_count: usize,
        which: &WebGpuBuffer<u32>,
        xs: &WebGpuBuffer<BabyBearExtElem>,
    ) -> bool {
        if which.size() == 0 {
            return true;
        }
        if poly_count == 0 || coeffs.size() % poly_count != 0 {
            return false;
        }
        out.raw_buffer().is_some()
            && coeffs.raw_buffer().is_some()
            && which.raw_buffer().is_some()
            && xs.raw_buffer().is_some()
            && self.storage_binding_fits(out)
            && self.storage_binding_fits(coeffs)
            && self.storage_binding_fits(which)
            && self.storage_binding_fits(xs)
    }

    fn can_dispatch_batch_evaluate_any_chunked(
        &self,
        out: &WebGpuBuffer<BabyBearExtElem>,
        coeffs: &WebGpuBuffer<BabyBearElem>,
        poly_count: usize,
        which: &WebGpuBuffer<u32>,
        xs: &WebGpuBuffer<BabyBearExtElem>,
    ) -> bool {
        if which.size() == 0 {
            return true;
        }
        if poly_count == 0 || coeffs.size() % poly_count != 0 {
            return false;
        }
        let deg = coeffs.size() / poly_count;
        let coeff_slice_bytes = byte_len_for::<BabyBearElem>(deg);
        out.size() == which.size()
            && xs.size() == which.size()
            && coeffs.raw_buffer().is_some()
            && xs.raw_buffer().is_some()
            && coeff_slice_bytes != 0
            && coeff_slice_bytes <= self.max_storage_binding_bytes()
            && coeff_slice_bytes <= MAX_EXACT_JS_INTEGER
            && coeffs.byte_offset() % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT == 0
            && coeff_slice_bytes % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT == 0
            && self.storage_binding_fits(xs)
    }

    fn can_dispatch_eltwise_sum_extelem(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearExtElem>,
    ) -> bool {
        if output.size() == 0 {
            return true;
        }
        output.size() % BabyBearExtElem::EXT_SIZE == 0
            && output.raw_buffer().is_some()
            && input.raw_buffer().is_some()
            && self.storage_binding_fits(output)
            && self.storage_binding_fits(input)
    }

    fn can_dispatch_fri_fold(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
    ) -> bool {
        let count = output.size() / BabyBearExtElem::EXT_SIZE;
        count == 0
            || (output.raw_buffer().is_some()
                && input.raw_buffer().is_some()
                && self.storage_binding_fits(output)
                && self.storage_binding_fits(input))
    }

    fn can_dispatch_hash_fold(&self, io: &WebGpuBuffer<Digest>, output_size: usize) -> bool {
        if !self.hash_fold_gpu_enabled.get() {
            return false;
        }
        if output_size == 0 {
            return true;
        }
        // SP-CR fix 2026-05-12: mirror `dispatch_poseidon2_hash_fold`'s
        // `round_constants` / `m_int_diag` checks (this file, near line 9339).
        let Some(hash) = self.poseidon2.as_ref() else {
            return false;
        };
        io.raw_buffer().is_some()
            && hash.round_constants.raw_buffer().is_some()
            && hash.m_int_diag.raw_buffer().is_some()
            && self.storage_binding_fits(io)
    }

    fn can_dispatch_hash_rows(
        &self,
        output: &WebGpuBuffer<Digest>,
        matrix: &WebGpuBuffer<BabyBearElem>,
        row_size: usize,
    ) -> bool {
        if !self.hash_rows_gpu_enabled.get() {
            return false;
        }
        if row_size == 0 {
            return true;
        }
        // SP-CR fix 2026-05-12: `dispatch_poseidon2_hash_rows` (this file, near
        // line 9397) requires both `round_constants` and `m_int_diag` GPU
        // buffers to be materialized. If `can_dispatch_*` is more permissive
        // than the actual dispatch, the `_async` wrapper skips the input sync
        // and `finish_hal_op` invokes `cpu_mirror` against stale CPU shadows,
        // producing all-zero hash outputs — see `01.5-root-cause.md` D11.
        let Some(hash) = self.poseidon2.as_ref() else {
            return false;
        };
        output.raw_buffer().is_some()
            && matrix.raw_buffer().is_some()
            && hash.round_constants.raw_buffer().is_some()
            && hash.m_int_diag.raw_buffer().is_some()
            && self.storage_binding_fits(output)
            && self.storage_binding_fits(matrix)
    }

    fn can_dispatch_gather_sample(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src: &WebGpuBuffer<BabyBearElem>,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> bool {
        if size == 0 {
            return true;
        }
        let (Some(dst_gpu), Some(src_gpu)) = (dst.raw_buffer(), src.raw_buffer()) else {
            return false;
        };
        size <= dst.size()
            && gather_region_in_bounds(src.size(), idx, size, stride)
            && dst_gpu != src_gpu
            && self.storage_binding_fits(dst)
            && self.storage_binding_fits(src)
    }

    /// Async-safe variant of [`Hal::batch_interpolate_ntt`] for GPU-authoritative
    /// proof stages. If the WebGPU dispatch would fall back to CPU, make the
    /// CPU shadow current before running the synchronous HAL method.
    pub(crate) async fn batch_interpolate_ntt_async(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        count: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative() || !self.can_dispatch_batch_interpolate_ntt(io) {
            io.sync_gpu_to_cpu(self).await?;
        }
        self.batch_interpolate_ntt(io, count);
        Ok(())
    }

    /// Async-safe variant of [`Hal::zk_shift`].
    pub(crate) async fn zk_shift_async(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        count: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative() || !self.can_dispatch_zk_shift(io) {
            io.sync_gpu_to_cpu(self).await?;
        }
        self.zk_shift(io, count);
        Ok(())
    }

    /// Async-safe variant of [`Hal::batch_bit_reverse`].
    pub(crate) async fn batch_bit_reverse_async(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        count: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative() || !self.can_dispatch_batch_bit_reverse(io) {
            io.sync_gpu_to_cpu(self).await?;
        }
        self.batch_bit_reverse(io, count);
        Ok(())
    }

    /// Async-safe variant of [`Hal::batch_expand_into_evaluate_ntt`].
    pub(crate) async fn batch_expand_into_evaluate_ntt_async(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
        count: usize,
        expand_bits: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative()
            || !self.can_dispatch_batch_expand_into_evaluate_ntt(output, input)
        {
            input.sync_gpu_to_cpu(self).await?;
        }
        self.batch_expand_into_evaluate_ntt(output, input, count, expand_bits);
        Ok(())
    }

    /// Async-safe variant of [`Hal::batch_evaluate_any`].
    pub(crate) async fn batch_evaluate_any_async(
        &self,
        coeffs: &WebGpuBuffer<BabyBearElem>,
        poly_count: usize,
        which: &WebGpuBuffer<u32>,
        xs: &WebGpuBuffer<BabyBearExtElem>,
        out: &WebGpuBuffer<BabyBearExtElem>,
    ) -> Result<()> {
        // SP6b iter 4: prefer chunked when eval_count is VERY small
        // AND deg is large. The chunked path issues one dispatch per
        // (x, which) pair (per-eval-sequential — see
        // `batch_evaluate_any_chunked_async` loop), so its dispatch
        // overhead grows with eval_count. The win comes from
        // parallelizing the Horner reduction WITHIN each (x, which)
        // pair into `chunk_count` chunks. So chunked is only a net
        // win when:
        //   - eval_count is small enough that sequential dispatches
        //     don't dominate (empirically ≤ 64 on this machine), AND
        //   - deg is large enough that the Horner per non-chunked
        //     thread leaves the GPU underutilized (deg > 8192 means
        //     each thread does 8K+ iterations, with chunked it
        //     parallelizes into 8 chunks of 1024 each).
        //
        // Hits: recursion lift groups 0/1 (16, 23 evals, deg=1048576).
        // Misses: rv32im finalize all groups (evals ≥ 119) and
        // recursion group 2 (604 evals). Iter 3 used a broader
        // heuristic and regressed rv32im finalize from 30 ms to
        // 712 ms because it forced chunked on the high-eval-count
        // case.
        let prefer_chunked = if self.gpu_authoritative()
            && poly_count > 0
            && which.size() > 0
            && which.size() <= 64
        {
            let deg = coeffs.size() / poly_count.max(1);
            deg > 8192
        } else {
            false
        };

        if !prefer_chunked
            && self.gpu_authoritative()
            && self.can_dispatch_batch_evaluate_any(out, coeffs, poly_count, which, xs)
        {
            self.batch_evaluate_any(coeffs, poly_count, which, xs, out);
            return Ok(());
        }

        if self.gpu_authoritative()
            && self.can_dispatch_batch_evaluate_any_chunked(out, coeffs, poly_count, which, xs)
            && self
                .batch_evaluate_any_chunked_async(coeffs, poly_count, which, xs, out)
                .await?
        {
            return Ok(());
        }

        if self.gpu_authoritative()
            && self.can_dispatch_batch_evaluate_any(out, coeffs, poly_count, which, xs)
        {
            self.batch_evaluate_any(coeffs, poly_count, which, xs, out);
            return Ok(());
        }

        coeffs.sync_gpu_to_cpu(self).await?;
        which.sync_gpu_to_cpu(self).await?;
        xs.sync_gpu_to_cpu(self).await?;
        self.batch_evaluate_any(coeffs, poly_count, which, xs, out);
        Ok(())
    }

    async fn batch_evaluate_any_chunked_async(
        &self,
        coeffs: &WebGpuBuffer<BabyBearElem>,
        poly_count: usize,
        which: &WebGpuBuffer<u32>,
        xs: &WebGpuBuffer<BabyBearExtElem>,
        out: &WebGpuBuffer<BabyBearExtElem>,
    ) -> Result<bool> {
        let eval_count = which.size();
        if eval_count == 0 {
            return Ok(true);
        }
        if poly_count == 0 || coeffs.size() % poly_count != 0 {
            return Ok(false);
        }
        let deg = coeffs.size() / poly_count;
        let chunk_size = WEBGPU_BATCH_EVALUATE_CHUNK_SIZE.min(deg).max(1);
        let chunk_count = deg.div_ceil(chunk_size);
        let coeff_slice_bytes = byte_len_for::<BabyBearElem>(deg);
        if coeff_slice_bytes == 0
            || coeff_slice_bytes > self.max_storage_binding_bytes()
            || coeff_slice_bytes > MAX_EXACT_JS_INTEGER
            || coeffs.byte_offset() % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0
            || coeff_slice_bytes % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0
        {
            return Ok(false);
        }

        let (Some(coeffs_gpu), Some(xs_gpu)) = (coeffs.raw_buffer(), xs.raw_buffer()) else {
            return Ok(false);
        };
        if !self.storage_binding_fits(xs) {
            return Ok(false);
        }

        let partial_count = eval_count
            .checked_mul(chunk_count)
            .ok_or_else(|| anyhow!("WebGPU batch_evaluate_any partial count overflow"))?;
        let partials = self.alloc_extelem("batch_evaluate_partials", partial_count);
        if !self.storage_binding_fits(&partials) {
            return Ok(false);
        }
        let Some(partials_gpu) = partials.raw_buffer() else {
            return Ok(false);
        };

        coeffs.sync_cpu_to_gpu(self)?;
        which.sync_gpu_to_cpu(self).await?;
        xs.sync_cpu_to_gpu(self)?;

        let xs_base = xs
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU batch_evaluate_any xs offset exceeds u32"))?;
        let partials_base = partials
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU batch_evaluate_any partials offset exceeds u32"))?;

        let layout = self.create_bind_group_layout(
            "webgpu_batch_evaluate_any_partial_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::uniform(3, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_batch_evaluate_any_partial",
            BATCH_EVALUATE_ANY_SLICE_PARTIAL_WGSL,
            "main",
            &[layout.clone()],
        )?;

        let which_values = which.to_vec();
        for (eval_idx, poly_id) in which_values.iter().copied().enumerate() {
            let poly_id = poly_id as usize;
            if poly_id >= poly_count {
                return Err(anyhow!(
                    "WebGPU batch_evaluate_any poly id {poly_id} exceeds poly count {poly_count}"
                ));
            }
            let coeff_offset = coeffs
                .byte_offset()
                .checked_add(
                    u64::try_from(poly_id)
                        .ok()
                        .and_then(|id| id.checked_mul(coeff_slice_bytes))
                        .ok_or_else(|| anyhow!("WebGPU batch_evaluate_any coeff slice overflow"))?,
                )
                .ok_or_else(|| anyhow!("WebGPU batch_evaluate_any coeff offset overflow"))?;
            let eval_partials_base = partials_base
                .checked_add(
                    eval_idx
                        .checked_mul(chunk_count)
                        .and_then(|offset| offset.checked_mul(BabyBearExtElem::EXT_SIZE))
                        .and_then(|offset| u32::try_from(offset).ok())
                        .ok_or_else(|| {
                            anyhow!("WebGPU batch_evaluate_any partial offset overflow")
                        })?,
                )
                .ok_or_else(|| anyhow!("WebGPU batch_evaluate_any partial base overflow"))?;
            let params = [
                u32::try_from(deg).expect("WebGPU batch_evaluate_any degree exceeds u32"),
                u32::try_from(chunk_count)
                    .expect("WebGPU batch_evaluate_any chunk count exceeds u32"),
                eval_partials_base,
                xs_base,
                u32::try_from(eval_idx).expect("WebGPU batch_evaluate_any eval index exceeds u32"),
                u32::try_from(chunk_size)
                    .expect("WebGPU batch_evaluate_any chunk size exceeds u32"),
                0,
                0,
            ];
            let params = self.create_uniform_buffer(
                "webgpu_batch_evaluate_any_partial_params",
                bytemuck::cast_slice(&params),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_batch_evaluate_any_partial_bind_group",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, partials_gpu),
                    WebGpuBufferBinding {
                        binding: 1,
                        buffer: coeffs_gpu,
                        offset: coeff_offset,
                        size: Some(coeff_slice_bytes),
                    },
                    WebGpuBufferBinding::new(2, xs_gpu),
                    WebGpuBufferBinding {
                        binding: 3,
                        buffer: &params,
                        offset: 0,
                        size: Some(32),
                    },
                ],
            )?;
            self.dispatch_compute(
                &kernel,
                &bind_group,
                u32::try_from(chunk_count)
                    .expect("WebGPU batch_evaluate_any chunk count exceeds u32")
                    .div_ceil(WEBGPU_WORKGROUP_SIZE),
                1,
                1,
            );
        }

        partials.mark_gpu_dirty();
        partials.sync_gpu_to_cpu(self).await?;
        let partial_values = partials.to_vec();
        let mut output = vec![BabyBearExtElem::ZERO; eval_count];
        for eval_idx in 0..eval_count {
            let start = eval_idx * chunk_count;
            let mut total = BabyBearExtElem::ZERO;
            for value in &partial_values[start..start + chunk_count] {
                total += *value;
            }
            output[eval_idx] = total;
        }
        out.cpu.view_mut(|cpu| {
            cpu.clone_from_slice(output.as_slice());
        });
        out.mark_cpu_result(false);
        self.record_gpu_result_authoritative("batch_evaluate_any", true);
        Ok(true)
    }

    /// Async-safe variant of [`Hal::mix_poly_coeffs`].
    ///
    /// Unlike most HAL operations, `mix_poly_coeffs` accumulates into `out`
    /// across several calls. In GPU-authoritative proving a later call can
    /// legitimately fall back to CPU after an earlier call wrote `out` on the
    /// GPU, so the fallback path must first materialize the accumulated GPU
    /// contents into the CPU shadow.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn mix_poly_coeffs_async(
        &self,
        out: &WebGpuBuffer<BabyBearExtElem>,
        mix_start: &BabyBearExtElem,
        mix: &BabyBearExtElem,
        input: &WebGpuBuffer<BabyBearElem>,
        combos: &WebGpuBuffer<u32>,
        input_size: usize,
        count: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative() {
            out.sync_gpu_to_cpu(self).await?;
            input.sync_gpu_to_cpu(self).await?;
            combos.sync_gpu_to_cpu(self).await?;
        }
        let gpu_mixed = self.dispatch_mix_poly_coeffs_inner(
            out, mix_start, mix, input, combos, input_size, count, true,
        )?;
        let gpu_mixed = if self.gpu_authoritative() && !gpu_mixed {
            self.dispatch_mix_poly_coeffs_chunked(
                out, mix_start, mix, input, combos, input_size, count,
            )?
        } else {
            gpu_mixed
        };
        if self.gpu_authoritative() {
            if gpu_mixed {
                self.record_gpu_result_authoritative("mix_poly_coeffs", true);
                out.mark_gpu_dirty();
            } else {
                out.sync_gpu_to_cpu(self).await?;
                input.sync_gpu_to_cpu(self).await?;
                combos.sync_gpu_to_cpu(self).await?;
                self.cpu.mix_poly_coeffs(
                    out.cpu(),
                    mix_start,
                    mix,
                    input.cpu(),
                    combos.cpu(),
                    input_size,
                    count,
                );
                self.record_gpu_result_authoritative("mix_poly_coeffs", false);
                out.mark_cpu_result(false);
            }
        } else {
            self.finish_hal_op("mix_poly_coeffs", gpu_mixed, out, || {
                self.cpu.mix_poly_coeffs(
                    out.cpu(),
                    mix_start,
                    mix,
                    input.cpu(),
                    combos.cpu(),
                    input_size,
                    count,
                );
            });
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn combos_prepare_async(
        &self,
        combos: &WebGpuBuffer<BabyBearExtElem>,
        coeff_u: &[BabyBearExtElem],
        combo_count: usize,
        cycles: usize,
        reg_sizes: &[u32],
        reg_combo_ids: &[u32],
        mix: &BabyBearExtElem,
    ) -> Result<()> {
        if !self.gpu_authoritative() {
            combos.sync_gpu_to_cpu(self).await?;
        }
        let gpu_prepared = self.dispatch_combos_prepare(
            combos,
            coeff_u,
            combo_count,
            cycles,
            reg_sizes,
            reg_combo_ids,
            mix,
        )?;
        if self.gpu_authoritative() {
            if gpu_prepared {
                self.record_gpu_result_authoritative("combos_prepare", true);
                combos.mark_gpu_dirty();
            } else {
                combos.sync_gpu_to_cpu(self).await?;
                self.cpu.combos_prepare(
                    combos.cpu(),
                    coeff_u,
                    combo_count,
                    cycles,
                    reg_sizes,
                    reg_combo_ids,
                    mix,
                );
                self.record_gpu_result_authoritative("combos_prepare", false);
                combos.mark_cpu_result(false);
            }
        } else {
            self.finish_hal_op("combos_prepare", gpu_prepared, combos, || {
                self.cpu.combos_prepare(
                    combos.cpu(),
                    coeff_u,
                    combo_count,
                    cycles,
                    reg_sizes,
                    reg_combo_ids,
                    mix,
                );
            });
        }
        Ok(())
    }

    pub(crate) async fn combos_divide_async(
        &self,
        combos: &WebGpuBuffer<BabyBearExtElem>,
        chunks: Vec<(usize, Vec<BabyBearExtElem>)>,
        cycles: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative() {
            combos.sync_gpu_to_cpu(self).await?;
        }
        let gpu_divided = self.dispatch_combos_divide(combos, &chunks, cycles)?;
        if self.gpu_authoritative() {
            if gpu_divided {
                self.record_gpu_result_authoritative("combos_divide", true);
                combos.mark_gpu_dirty();
            } else {
                combos.sync_gpu_to_cpu(self).await?;
                self.cpu.combos_divide(combos.cpu(), chunks, cycles);
                self.record_gpu_result_authoritative("combos_divide", false);
                combos.mark_cpu_result(false);
            }
        } else {
            self.finish_hal_op("combos_divide", gpu_divided, combos, || {
                self.cpu.combos_divide(combos.cpu(), chunks, cycles);
            });
        }
        Ok(())
    }

    /// Async-safe variant of [`Hal::eltwise_sum_extelem`].
    pub(crate) async fn eltwise_sum_extelem_async(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearExtElem>,
    ) -> Result<()> {
        if !self.gpu_authoritative() || !self.can_dispatch_eltwise_sum_extelem(output, input) {
            input.sync_gpu_to_cpu(self).await?;
        }
        self.eltwise_sum_extelem(output, input);
        Ok(())
    }

    /// Async-safe variant of [`Hal::fri_fold`].
    pub(crate) async fn fri_fold_async(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
        mix: &BabyBearExtElem,
    ) -> Result<()> {
        if !self.gpu_authoritative() || !self.can_dispatch_fri_fold(output, input) {
            input.sync_gpu_to_cpu(self).await?;
        }
        self.fri_fold(output, input, mix);
        Ok(())
    }

    /// Async-safe variant of [`Hal::hash_fold`].
    pub(crate) async fn hash_fold_async(
        &self,
        io: &WebGpuBuffer<Digest>,
        input_size: usize,
        output_size: usize,
    ) -> Result<()> {
        if !self.gpu_authoritative() || !self.can_dispatch_hash_fold(io, output_size) {
            io.sync_gpu_to_cpu(self).await?;
        }
        self.hash_fold(io, input_size, output_size);
        Ok(())
    }

    /// SP-submission iter 2: batch a chain of `hash_fold_async` calls
    /// (the merkle build loop) into a single submit. Saves
    /// ~`output_sizes.len() - 1` GPU-process IPC round-trips. Falls
    /// through to per-call hash_fold_async when GPU dispatch is
    /// unavailable so the diagnostic counters stay accurate.
    pub async fn hash_fold_chain_async(
        &self,
        io: &WebGpuBuffer<Digest>,
        output_sizes: &[usize],
    ) -> Result<()> {
        let can_chain = self.gpu_authoritative()
            && output_sizes
                .iter()
                .all(|&out| self.can_dispatch_hash_fold(io, out));
        if can_chain {
            if self
                .dispatch_poseidon2_hash_fold_chain(io, output_sizes)?
            {
                return Ok(());
            }
        }
        // Fallback: serial per-call path (CPU mirror branches still
        // need the per-call sync_gpu_to_cpu).
        for &output_size in output_sizes {
            self.hash_fold_async(io, 2 * output_size, output_size)
                .await?;
        }
        Ok(())
    }

    /// Async-safe variant of [`Hal::hash_rows`].
    pub(crate) async fn hash_rows_async(
        &self,
        output: &WebGpuBuffer<Digest>,
        matrix: &WebGpuBuffer<BabyBearElem>,
    ) -> Result<()> {
        let row_size = output.size();
        if !self.gpu_authoritative() || !self.can_dispatch_hash_rows(output, matrix, row_size) {
            matrix.sync_gpu_to_cpu(self).await?;
        }
        self.hash_rows(output, matrix);
        Ok(())
    }

    /// Async-safe variant of [`Hal::gather_sample`].
    pub(crate) async fn gather_sample_async(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src: &WebGpuBuffer<BabyBearElem>,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> Result<()> {
        let can_dispatch = self.can_dispatch_gather_sample(dst, src, idx, size, stride);
        if self.gpu_authoritative() && can_dispatch {
            self.gather_sample(dst, src, idx, size, stride);
            return Ok(());
        }

        if self.gpu_authoritative()
            && !src.cpu_is_current()
            && size <= dst.size()
            && gather_region_in_bounds(src.size(), idx, size, stride)
        {
            src.sync_cpu_to_gpu(self)?;
            if let Some(src_gpu) = src.raw_buffer() {
                let sample = self
                    .read_gathered_elem_sample(
                        src_gpu,
                        src.name(),
                        src.elem_offset,
                        idx,
                        size,
                        stride,
                    )
                    .await?;
                dst.cpu.view_mut(|cpu| {
                    cpu[..sample.len()].clone_from_slice(sample.as_slice());
                });
                dst.mark_cpu_result(false);
                self.diagnostics.record_cpu_fallback("gather_sample");
                return Ok(());
            }
        }

        if !self.gpu_authoritative() || !can_dispatch {
            src.sync_gpu_to_cpu(self).await?;
        }
        self.gather_sample(dst, src, idx, size, stride);
        Ok(())
    }

    /// Test hook for the async gather path used by Merkle query openings.
    #[doc(hidden)]
    pub async fn debug_gather_sample_async(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src: &WebGpuBuffer<BabyBearElem>,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> Result<()> {
        self.gather_sample_async(dst, src, idx, size, stride).await
    }

    /// Test hook for validating chunked gather bindings without changing the
    /// normal proving path.
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub fn debug_dispatch_gather_sample_chunked(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src: &WebGpuBuffer<BabyBearElem>,
        idx: usize,
        size: usize,
        stride: usize,
        chunk_cols: usize,
    ) -> Result<()> {
        if size == 0 {
            return Ok(());
        }
        ensure!(chunk_cols > 0, "WebGPU gather chunk size must be nonzero");
        ensure!(
            size <= dst.size() && gather_region_in_bounds(src.size(), idx, size, stride),
            "WebGPU gather chunk test region is out of bounds"
        );

        let (Some(dst_gpu), Some(src_gpu)) = (dst.raw_buffer(), src.raw_buffer()) else {
            return Err(anyhow!("WebGPU gather chunk test requires GPU buffers"));
        };
        ensure!(
            dst_gpu != src_gpu,
            "WebGPU gather chunk test aliases buffers"
        );

        src.sync_cpu_to_gpu(self)?;
        dst.sync_cpu_to_gpu(self)?;
        self.dispatch_gather_sample_chunked(
            dst, src, dst_gpu, src_gpu, idx, size, stride, chunk_cols,
        )?;
        dst.mark_gpu_dirty();
        Ok(())
    }

    /// SP4 iter 3 test hook: exercise `dispatch_gather_sample_tiled`
    /// over a `BufferPool` source. The pool's `layout` must match the
    /// caller's `stride` and `size`. Production callers will fall into
    /// this path automatically once SP5 wires the recursion data group
    /// to be `BufferPool`-backed; for now the test exercises it
    /// directly.
    #[doc(hidden)]
    pub fn debug_dispatch_gather_sample_tiled(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src_pool: &buffer_pool::BufferPool,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> Result<()> {
        self.dispatch_gather_sample_tiled(dst, src_pool, idx, size, stride)?;
        dst.mark_gpu_dirty();
        Ok(())
    }

    /// Test hook for validating `mix_poly_coeffs` under GPU-authoritative state
    /// without enabling that path in production proving.
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub fn debug_dispatch_mix_poly_coeffs_authoritative(
        &self,
        output: &WebGpuBuffer<BabyBearExtElem>,
        mix_start: &BabyBearExtElem,
        mix: &BabyBearExtElem,
        input: &WebGpuBuffer<BabyBearElem>,
        combos: &WebGpuBuffer<u32>,
        input_size: usize,
        count: usize,
    ) -> Result<bool> {
        self.dispatch_mix_poly_coeffs_inner(
            output, mix_start, mix, input, combos, input_size, count, true,
        )
    }

    fn alloc_shadowed_buffer<T>(&self, name: &'static str, cpu: CpuBuffer<T>) -> WebGpuBuffer<T>
    where
        T: Clone,
    {
        let byte_len = byte_len_for::<T>(cpu.size());
        let gpu = if byte_len == 0 || !self.can_allocate_gpu_buffer(byte_len) {
            None
        } else {
            Some(Rc::new(WebGpuBufferOwner {
                buffer: self
                    .create_storage_buffer(name, byte_len)
                    .unwrap_or_else(|err| panic!("failed to allocate WebGPU buffer {name}: {err}")),
            }))
        };
        WebGpuBuffer::new(
            cpu,
            gpu,
            Rc::new(Cell::new(true)),
            Rc::new(Cell::new(false)),
        )
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
            self.write_buffer_named(gpu, name, 0, bytemuck::cast_slice(slice))
                .unwrap_or_else(|err| panic!("failed to upload WebGPU buffer {name}: {err}"));
        }
        buffer.mark_synced();
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
        self.write_buffer_named(&buffer, label, 0, bytes)?;
        Ok(buffer)
    }

    /// Upload raw bytes to a WebGPU buffer.
    pub fn write_buffer(
        &self,
        buffer: &web_sys::GpuBuffer,
        byte_offset: u64,
        bytes: &[u8],
    ) -> Result<()> {
        self.write_buffer_named(buffer, "unattributed", byte_offset, bytes)
    }

    /// Upload raw bytes to a WebGPU buffer and attribute diagnostics to `name`.
    pub fn write_buffer_named(
        &self,
        buffer: &web_sys::GpuBuffer,
        name: &'static str,
        byte_offset: u64,
        bytes: &[u8],
    ) -> Result<()> {
        self.queue
            .write_buffer_with_f64_and_u8_slice(buffer, byte_offset_as_f64(byte_offset)?, bytes)
            .map_err(js_error)?;
        self.diagnostics.record_upload(name, bytes.len() as u64);
        Ok(())
    }

    /// Create a bind group layout for compute kernels.
    pub fn create_bind_group_layout(
        &self,
        label: &'static str,
        entries: &[WebGpuBindingLayout],
    ) -> Result<web_sys::GpuBindGroupLayout> {
        // SP9 corrected (2026-05-15): cache layouts by (label_ptr,
        // entries-shape) so callers that ask for the same layout get
        // the same JS instance. WebGPU pipelines bind specifically to
        // the layout INSTANCE they were created with -- a later
        // pipeline cache requires this invariant to be sound.
        let cache_key = compute_bind_group_layout_cache_key(label, entries);
        if let Some(cached) = self.bind_group_layout_cache.borrow().get(&cache_key).cloned() {
            self.diagnostics.record_bind_group_layout_cache_hit();
            return Ok(cached);
        }

        let layout_entries = js_sys::Array::new();
        for entry in entries {
            let buffer = web_sys::GpuBufferBindingLayout::new();
            buffer.set_type(entry.ty);
            if entry.min_binding_size != 0 {
                buffer.set_min_binding_size(byte_len_as_f64(entry.min_binding_size)?);
            }
            if entry.has_dynamic_offset {
                buffer.set_has_dynamic_offset(true);
            }

            let layout_entry =
                web_sys::GpuBindGroupLayoutEntry::new(entry.binding, WEBGPU_SHADER_STAGE_COMPUTE);
            layout_entry.set_buffer(&buffer);
            layout_entries.push(layout_entry.as_ref());
        }

        let desc = web_sys::GpuBindGroupLayoutDescriptor::new(layout_entries.as_ref());
        desc.set_label(label);
        let layout = self
            .device
            .create_bind_group_layout(&desc)
            .map_err(js_error)?;
        self.diagnostics.record_bind_group_layout_creation();
        self.bind_group_layout_cache
            .borrow_mut()
            .insert(cache_key, layout.clone());
        self.bind_group_layout_key_map.set(
            layout.as_ref(),
            &JsValue::from_str(&cache_key.to_string()),
        );
        Ok(layout)
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
        self.diagnostics.record_bind_group_creation();
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
        let cache_key =
            self.compute_kernel_cache_key(label, wgsl, entry_point, bind_group_layouts);
        if let Some(cache_key) = cache_key {
            if let Some(cached) = self.compute_kernel_cache.borrow().get(&cache_key).cloned() {
                self.diagnostics.record_compute_pipeline_cache_hit();
                return Ok(cached);
            }
        }

        let pipeline_desc =
            self.build_compute_pipeline_desc(label, wgsl, entry_point, bind_group_layouts);
        let kernel = WebGpuKernel {
            pipeline: self.device.create_compute_pipeline(&pipeline_desc),
        };
        self.diagnostics.record_compute_pipeline_creation();
        if let Some(cache_key) = cache_key {
            self.compute_kernel_cache
                .borrow_mut()
                .insert(cache_key, kernel.clone());
        }
        Ok(kernel)
    }

    /// SP7 iter 6d-d (2026-05-15): async-compile the same kernel via
    /// `device.createComputePipelineAsync()`. The Tint compile runs in
    /// the browser GPU process while the wasm thread does other work
    /// (guest execution, session setup); when the returned Future
    /// resolves, the kernel is ready to dispatch. Useful for the
    /// iter-6d-c probe so the ~60 s exec_TopChunk0 compile overlaps with
    /// xgboost's segment-level prover work instead of blocking the
    /// first witgen call.
    pub async fn create_compute_kernel_async(
        &self,
        label: &'static str,
        wgsl: &str,
        entry_point: &str,
        bind_group_layouts: &[web_sys::GpuBindGroupLayout],
    ) -> Result<WebGpuKernel> {
        let cache_key =
            self.compute_kernel_cache_key(label, wgsl, entry_point, bind_group_layouts);
        if let Some(cache_key) = cache_key {
            if let Some(cached) = self.compute_kernel_cache.borrow().get(&cache_key).cloned() {
                self.diagnostics.record_compute_pipeline_cache_hit();
                return Ok(cached);
            }
        }

        let pipeline_desc =
            self.build_compute_pipeline_desc(label, wgsl, entry_point, bind_group_layouts);
        let promise = self.device.create_compute_pipeline_async(&pipeline_desc);
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        let pipeline_value = future
            .await
            .map_err(|err| anyhow::anyhow!("createComputePipelineAsync rejected: {err:?}"))?;
        let pipeline: web_sys::GpuComputePipeline = pipeline_value
            .dyn_into()
            .map_err(|_| anyhow::anyhow!("createComputePipelineAsync resolved to non-pipeline value"))?;
        let kernel = WebGpuKernel { pipeline };
        self.diagnostics.record_compute_pipeline_creation();
        if let Some(cache_key) = cache_key {
            self.compute_kernel_cache
                .borrow_mut()
                .insert(cache_key, kernel.clone());
        }
        Ok(kernel)
    }

    fn compute_kernel_cache_key(
        &self,
        label: &'static str,
        wgsl: &str,
        entry_point: &str,
        bind_group_layouts: &[web_sys::GpuBindGroupLayout],
    ) -> Option<u64> {
        let layout_keys = bind_group_layouts
            .iter()
            .map(|layout| self.bind_group_layout_key_map.get(layout.as_ref()).as_string())
            .collect::<Option<Vec<_>>>()?;
        Some(compute_compute_pipeline_cache_key(
            label,
            wgsl,
            entry_point,
            layout_keys.as_slice(),
        ))
    }

    fn build_compute_pipeline_desc(
        &self,
        label: &'static str,
        wgsl: &str,
        entry_point: &str,
        bind_group_layouts: &[web_sys::GpuBindGroupLayout],
    ) -> web_sys::GpuComputePipelineDescriptor {
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
        pipeline_desc
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

    /// Dispatch a logical 1D kernel, spilling workgroups into `y` when the
    /// `x` dimension would exceed WebGPU's portable per-dimension limit.
    pub fn dispatch_compute_1d(
        &self,
        kernel: &WebGpuKernel,
        bind_group: &web_sys::GpuBindGroup,
        workgroups: u32,
    ) {
        if workgroups <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION {
            self.dispatch_compute(kernel, bind_group, workgroups, 1, 1);
            return;
        }

        let workgroups_y = workgroups.div_ceil(WEBGPU_MAX_WORKGROUPS_PER_DIMENSION);
        assert!(
            workgroups_y <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION,
            "WebGPU 1D dispatch exceeds portable 2D workgroup capacity"
        );
        self.dispatch_compute(
            kernel,
            bind_group,
            WEBGPU_MAX_WORKGROUPS_PER_DIMENSION,
            workgroups_y,
            1,
        );
    }

    /// Dispatch multiple logical 1D kernels against the same bind group in one
    /// compute pass and queue submit.
    pub fn dispatch_compute_1d_sequence(
        &self,
        kernels: &[WebGpuKernel],
        bind_group: &web_sys::GpuBindGroup,
        workgroups: u32,
    ) {
        if kernels.is_empty() {
            return;
        }

        let (workgroups_x, workgroups_y) = if workgroups <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION {
            (workgroups, 1)
        } else {
            let workgroups_y = workgroups.div_ceil(WEBGPU_MAX_WORKGROUPS_PER_DIMENSION);
            assert!(
                workgroups_y <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION,
                "WebGPU 1D dispatch exceeds portable 2D workgroup capacity"
            );
            (WEBGPU_MAX_WORKGROUPS_PER_DIMENSION, workgroups_y)
        };

        let encoder = self.device.create_command_encoder();
        let pass = encoder.begin_compute_pass();
        pass.set_bind_group(0, Some(bind_group));
        for kernel in kernels {
            pass.set_pipeline(&kernel.pipeline);
            pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
                workgroups_x,
                workgroups_y,
                1,
            );
        }
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
        self.read_buffer_range_named(source, 0, byte_len, "read_buffer")
            .await
    }

    /// Copy a byte range from a GPU buffer into WASM memory.
    pub async fn read_buffer_range(
        &self,
        source: &web_sys::GpuBuffer,
        source_offset: u64,
        byte_len: u64,
    ) -> Result<Vec<u8>> {
        self.read_buffer_range_named(source, source_offset, byte_len, "read_buffer_range")
            .await
    }

    /// Copy a byte range from a GPU buffer into WASM memory and attribute the
    /// readback to the source buffer's logical name.
    pub async fn read_buffer_range_named(
        &self,
        source: &web_sys::GpuBuffer,
        source_offset: u64,
        byte_len: u64,
        name: &'static str,
    ) -> Result<Vec<u8>> {
        let readback = self.create_buffer(
            "webgpu_readback",
            byte_len,
            WEBGPU_BUFFER_USAGE_MAP_READ | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;

        let encoder = self.device.create_command_encoder();
        let source_offset = byte_offset_as_f64(source_offset)?;
        let byte_len = byte_len_as_f64(byte_len)?;
        encoder
            .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                source,
                source_offset,
                &readback,
                0.0,
                byte_len,
            )
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
        self.diagnostics.record_readback(name, bytes.len() as u64);
        Ok(bytes)
    }

    /// Copy fixed-size elements at arbitrary indices from a GPU buffer into
    /// contiguous WASM memory.
    pub async fn read_buffer_indices(
        &self,
        source: &web_sys::GpuBuffer,
        base_byte_offset: u64,
        elem_size: u64,
        indices: &[usize],
    ) -> Result<Vec<u8>> {
        self.read_buffer_indices_named(
            source,
            base_byte_offset,
            elem_size,
            indices,
            "indexed_readback",
        )
        .await
    }

    /// Copy fixed-size elements at arbitrary indices from a GPU buffer into
    /// contiguous WASM memory and attribute the readback to the source buffer's
    /// logical name.
    pub async fn read_buffer_indices_named(
        &self,
        source: &web_sys::GpuBuffer,
        base_byte_offset: u64,
        elem_size: u64,
        indices: &[usize],
        name: &'static str,
    ) -> Result<Vec<u8>> {
        if indices.is_empty() {
            return Ok(Vec::new());
        }
        ensure!(
            elem_size > 0,
            "WebGPU indexed readback element size is zero"
        );

        let byte_len = indices
            .len()
            .checked_mul(
                usize::try_from(elem_size)
                    .map_err(|_| anyhow!("WebGPU indexed readback element size exceeds usize"))?,
            )
            .and_then(|value| value.try_into().ok())
            .ok_or_else(|| anyhow!("WebGPU indexed readback length overflow"))?;
        let readback = self.create_buffer(
            "webgpu_indexed_readback",
            byte_len,
            WEBGPU_BUFFER_USAGE_MAP_READ | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;

        let encoder = self.device.create_command_encoder();
        for (out_idx, source_idx) in indices.iter().copied().enumerate() {
            let indexed_byte_offset = u64::try_from(source_idx)
                .ok()
                .and_then(|idx| idx.checked_mul(elem_size))
                .and_then(|offset| base_byte_offset.checked_add(offset))
                .ok_or_else(|| anyhow!("WebGPU indexed readback source offset overflow"))?;
            let destination_byte_offset = u64::try_from(out_idx)
                .ok()
                .and_then(|idx| idx.checked_mul(elem_size))
                .ok_or_else(|| anyhow!("WebGPU indexed readback destination offset overflow"))?;
            encoder
                .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                    source,
                    byte_offset_as_f64(indexed_byte_offset)?,
                    &readback,
                    byte_offset_as_f64(destination_byte_offset)?,
                    byte_len_as_f64(elem_size)?,
                )
                .map_err(js_error)?;
        }
        self.submit(encoder.finish());

        let byte_len_f64 = byte_len_as_f64(byte_len)?;
        JsFuture::from(readback.map_async_with_f64_and_f64(
            WEBGPU_MAP_MODE_READ,
            0.0,
            byte_len_f64,
        ))
        .await
        .map_err(js_error)?;

        let mapped = readback
            .get_mapped_range_with_f64_and_f64(0.0, byte_len_f64)
            .map_err(js_error)?;
        let bytes = js_sys::Uint8Array::new(&mapped).to_vec();
        readback.unmap();
        self.diagnostics.record_readback(name, bytes.len() as u64);
        Ok(bytes)
    }

    async fn read_gathered_elem_sample(
        &self,
        source: &web_sys::GpuBuffer,
        name: &'static str,
        source_elem_offset: usize,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> Result<Vec<BabyBearElem>> {
        if size == 0 {
            return Ok(Vec::new());
        }

        let byte_len = byte_len_for::<BabyBearElem>(size);
        let readback = self.create_buffer(
            "webgpu_gather_sample_readback",
            byte_len,
            WEBGPU_BUFFER_USAGE_MAP_READ | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;

        let encoder = self.device.create_command_encoder();
        let elem_bytes = byte_len_for::<BabyBearElem>(1);
        for out_idx in 0..size {
            let source_idx = source_elem_offset
                .checked_add(idx)
                .and_then(|base| {
                    out_idx
                        .checked_mul(stride)
                        .and_then(|offset| base.checked_add(offset))
                })
                .ok_or_else(|| anyhow!("WebGPU gather readback source offset overflow"))?;
            let source_offset = byte_offset_as_f64(byte_len_for::<BabyBearElem>(source_idx))?;
            let destination_offset = byte_offset_as_f64(byte_len_for::<BabyBearElem>(out_idx))?;
            encoder
                .copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                    source,
                    source_offset,
                    &readback,
                    destination_offset,
                    byte_len_as_f64(elem_bytes)?,
                )
                .map_err(js_error)?;
        }
        self.submit(encoder.finish());

        JsFuture::from(readback.map_async_with_f64_and_f64(
            WEBGPU_MAP_MODE_READ,
            0.0,
            byte_len_as_f64(byte_len)?,
        ))
        .await
        .map_err(js_error)?;

        let mapped = readback
            .get_mapped_range_with_f64_and_f64(0.0, byte_len_as_f64(byte_len)?)
            .map_err(js_error)?;
        let bytes = js_sys::Uint8Array::new(&mapped).to_vec();
        readback.unmap();
        self.diagnostics.record_readback(name, bytes.len() as u64);

        let values = bytemuck::checked::try_cast_slice::<u8, BabyBearElem>(bytes.as_slice())
            .map_err(|err| anyhow!("invalid WebGPU gather readback: {err}"))?;
        ensure!(
            values.len() == size,
            "WebGPU gather readback size mismatch: got {} elems, expected {size}",
            values.len()
        );
        Ok(values.to_vec())
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
        // SP9 phase 2 take 3 (2026-05-15): min_binding_size=0 to keep the
        // layout shape stable across byte_len-variant calls; the runtime
        // bind-validation still uses the actual buffer size at dispatch.
        let layout = self.create_bind_group_layout(
            "webgpu_zeroize_elem_layout",
            &[WebGpuBindingLayout::storage(0, 0)],
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
            .div_ceil(WEBGPU_WORKGROUP_SIZE);
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
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
        // SP9 phase 2 take 3: stable layout shape via min_binding_size=0.
        let layout = self.create_bind_group_layout(
            "webgpu_eltwise_add_elem_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
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
            .div_ceil(WEBGPU_WORKGROUP_SIZE);
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
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
        if !self.storage_binding_fits(output) || !self.storage_binding_fits(input) {
            return Ok(false);
        }

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
        // SP9 phase 2 take 3: stable layout shape via min_binding_size=0.
        let layout = self.create_bind_group_layout(
            "webgpu_eltwise_sum_extelem_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
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
        // SP9 phase 2 take 3: stable layout shape via min_binding_size=0.
        let layout = self.create_bind_group_layout(
            "webgpu_eltwise_copy_elem_slice_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
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
        // SP9 phase 2 take 3: stable layout shape via min_binding_size=0.
        let layout = self.create_bind_group_layout(
            "webgpu_scatter_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
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
        if !self.storage_binding_fits(dst) || !self.storage_binding_fits(src) {
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

    #[allow(clippy::too_many_arguments)]
    fn dispatch_gather_sample_chunked(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src: &WebGpuBuffer<BabyBearElem>,
        dst_gpu: &web_sys::GpuBuffer,
        src_gpu: &web_sys::GpuBuffer,
        idx: usize,
        size: usize,
        stride: usize,
        chunk_cols: usize,
    ) -> Result<()> {
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

        let mut params_buffers = Vec::new();
        let mut bind_groups = Vec::new();
        for col_start in (0..size).step_by(chunk_cols) {
            let cols = chunk_cols.min(size - col_start);
            let src_elem_offset = src
                .elem_offset
                .checked_add(
                    col_start
                        .checked_mul(stride)
                        .ok_or_else(|| anyhow!("WebGPU gather chunk offset overflow"))?,
                )
                .ok_or_else(|| anyhow!("WebGPU gather chunk offset overflow"))?;
            let src_byte_offset = byte_len_for::<BabyBearElem>(src_elem_offset);
            let src_binding_offset = src_byte_offset / WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT
                * WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT;
            let src_binding_base_bytes = src_byte_offset - src_binding_offset;
            let src_binding_base_elems =
                usize::try_from(src_binding_base_bytes / mem::size_of::<BabyBearElem>() as u64)
                    .expect("WebGPU gather chunk binding offset exceeds usize");
            let src_chunk_elems = cols
                .checked_sub(1)
                .and_then(|last_col| last_col.checked_mul(stride))
                .and_then(|last_col_start| last_col_start.checked_add(idx))
                .and_then(|last_idx| last_idx.checked_add(1))
                .ok_or_else(|| anyhow!("WebGPU gather chunk size overflow"))?;
            let src_chunk_bytes = src_binding_base_bytes
                .checked_add(byte_len_for::<BabyBearElem>(src_chunk_elems))
                .ok_or_else(|| anyhow!("WebGPU gather chunk binding size overflow"))?;
            ensure!(
                src_chunk_bytes <= WEBGPU_SAFE_STORAGE_BINDING_BYTES,
                "WebGPU gather chunk binding size exceeds safe WebGPU storage binding limit"
            );

            let params = [
                u32::try_from(dst.elem_offset + col_start)
                    .expect("WebGPU gather dst offset exceeds u32"),
                u32::try_from(src_binding_base_elems)
                    .expect("WebGPU gather source base exceeds u32"),
                u32::try_from(idx).expect("WebGPU gather idx exceeds u32"),
                u32::try_from(cols).expect("WebGPU gather chunk size exceeds u32"),
                u32::try_from(stride).expect("WebGPU gather stride exceeds u32"),
                0,
                0,
                0,
            ];
            let params = self.create_uniform_buffer(
                "webgpu_gather_sample_params",
                bytemuck::cast_slice(&params),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_gather_sample_bind_group",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, dst_gpu),
                    WebGpuBufferBinding {
                        binding: 1,
                        buffer: src_gpu,
                        offset: src_binding_offset,
                        size: Some(src_chunk_bytes),
                    },
                    WebGpuBufferBinding {
                        binding: 2,
                        buffer: &params,
                        offset: 0,
                        size: Some(32),
                    },
                ],
            )?;
            let workgroups = u32::try_from(cols)
                .expect("WebGPU gather chunk size exceeds u32")
                .div_ceil(WEBGPU_WORKGROUP_SIZE);
            self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
            params_buffers.push(params);
            bind_groups.push(bind_group);
        }

        drop(bind_groups);
        drop(params_buffers);
        Ok(())
    }

    /// SP4 iter 2 (R8): tiled gather variant operating over a
    /// `BufferPool`. Each tile holds a contiguous column-slab of the
    /// logical 2D source; the kernel runs per-tile with the same
    /// `GATHER_SAMPLE_ELEM_WGSL` shader as `dispatch_gather_sample`,
    /// but binding the per-tile buffer instead of a sub-range of one
    /// oversize buffer. Replaces the locked CPU fallback in
    /// `gather_sample_async` that previously fired when the source
    /// exceeded `maxStorageBufferBindingSize`.
    ///
    /// `idx`, `size`, and `stride` follow the same contract as
    /// `gather_sample`: read `pool[col * stride + idx]` into `dst[col]`
    /// for `col` in `[0, size)`. `size` must equal `pool.layout.total_cols`
    /// and `stride` must equal `pool.layout.stride`; the layout's
    /// `tile_cols` is what we iterate over.
    fn dispatch_gather_sample_tiled(
        &self,
        dst: &WebGpuBuffer<BabyBearElem>,
        src_pool: &buffer_pool::BufferPool,
        idx: usize,
        size: usize,
        stride: usize,
    ) -> Result<()> {
        ensure!(
            stride == src_pool.layout.stride,
            "dispatch_gather_sample_tiled: stride mismatch (caller={stride}, pool={})",
            src_pool.layout.stride
        );
        ensure!(
            size == src_pool.layout.total_cols,
            "dispatch_gather_sample_tiled: size mismatch (caller={size}, pool.total_cols={})",
            src_pool.layout.total_cols
        );
        ensure!(
            size <= dst.size(),
            "dispatch_gather_sample_tiled: dst capacity {} < size {size}",
            dst.size()
        );
        if size == 0 {
            return Ok(());
        }
        let dst_gpu = dst
            .raw_buffer()
            .ok_or_else(|| anyhow!("dispatch_gather_sample_tiled: dst has no GPU backing"))?;
        ensure!(
            self.storage_binding_fits(dst),
            "dispatch_gather_sample_tiled: dst exceeds max storage binding"
        );
        dst.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_gather_sample_tiled_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform(2, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_gather_sample_tiled",
            GATHER_SAMPLE_ELEM_WGSL,
            "main",
            &[layout.clone()],
        )?;

        // Hold per-tile params + bind group references until submit
        // completes (we issue one dispatch per tile inside this fn).
        let mut params_buffers: Vec<web_sys::GpuBuffer> = Vec::with_capacity(src_pool.num_tiles());
        let mut bind_groups: Vec<web_sys::GpuBindGroup> =
            Vec::with_capacity(src_pool.num_tiles());

        for tile_idx in 0..src_pool.num_tiles() {
            let cols = src_pool.layout.cols_in_tile(tile_idx);
            if cols == 0 {
                continue;
            }
            let col_start = tile_idx * src_pool.layout.tile_cols;
            // Per-tile params: dst_base advances by col_start; src_base
            // is 0 because each tile buffer starts at the first column
            // it owns (no sub-range offset inside the buffer).
            let params = [
                u32::try_from(dst.elem_offset + col_start)
                    .expect("WebGPU gather dst offset exceeds u32"),
                0u32,
                u32::try_from(idx).expect("WebGPU gather idx exceeds u32"),
                u32::try_from(cols).expect("WebGPU gather tile cols exceeds u32"),
                u32::try_from(stride).expect("WebGPU gather stride exceeds u32"),
                0,
                0,
                0,
            ];
            let params_buf = self.create_uniform_buffer(
                "webgpu_gather_sample_tiled_params",
                bytemuck::cast_slice(&params),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_gather_sample_tiled_bind_group",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, dst_gpu),
                    WebGpuBufferBinding::new(1, src_pool.tile_buffer(tile_idx)),
                    WebGpuBufferBinding {
                        binding: 2,
                        buffer: &params_buf,
                        offset: 0,
                        size: Some(32),
                    },
                ],
            )?;
            let workgroups = u32::try_from(cols)
                .expect("WebGPU gather tile cols exceeds u32")
                .div_ceil(WEBGPU_WORKGROUP_SIZE);
            self.dispatch_compute(&kernel, &bind_group, workgroups, 1, 1);
            params_buffers.push(params_buf);
            bind_groups.push(bind_group);
        }

        drop(bind_groups);
        drop(params_buffers);
        Ok(())
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
        // SP9 phase 2 take 3: stable layout shape via min_binding_size=0.
        let layout = self.create_bind_group_layout(
            "webgpu_prefix_products_extelem_layout",
            &[WebGpuBindingLayout::storage(0, 0)],
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
        if !self.storage_binding_fits(output) || !self.storage_binding_fits(input) {
            return Ok(false);
        }

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
        let kernel = self.create_compute_kernel(
            "webgpu_fri_fold",
            FRI_FOLD_WGSL,
            "main",
            &[layout.clone()],
        )?;
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
        if !self.zk_shift_gpu_enabled.get() {
            return Ok(false);
        }
        if io.size() == 0 {
            return Ok(true);
        }

        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(io) {
            return Ok(false);
        }

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
        let kernel = self.create_compute_kernel(
            "webgpu_zk_shift",
            ZK_SHIFT_WGSL,
            "main",
            &[layout.clone()],
        )?;
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
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
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
        self.dispatch_mix_poly_coeffs_inner(
            output, mix_start, mix, input, combos, input_size, count, false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_mix_poly_coeffs_inner(
        &self,
        output: &WebGpuBuffer<BabyBearExtElem>,
        mix_start: &BabyBearExtElem,
        mix: &BabyBearExtElem,
        input: &WebGpuBuffer<BabyBearElem>,
        combos: &WebGpuBuffer<u32>,
        input_size: usize,
        count: usize,
        allow_gpu_authoritative: bool,
    ) -> Result<bool> {
        if count == 0 {
            return Ok(true);
        }
        if self.gpu_authoritative() && !allow_gpu_authoritative {
            return Ok(false);
        }

        let (Some(output_gpu), Some(input_gpu), Some(combos_gpu)) =
            (output.raw_buffer(), input.raw_buffer(), combos.raw_buffer())
        else {
            return Ok(false);
        };
        if !self.storage_binding_fits(output)
            || !self.storage_binding_fits(input)
            || !self.storage_binding_fits(combos)
        {
            return Ok(false);
        }

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

    #[allow(clippy::too_many_arguments)]
    fn dispatch_mix_poly_coeffs_chunked(
        &self,
        output: &WebGpuBuffer<BabyBearExtElem>,
        mix_start: &BabyBearExtElem,
        mix: &BabyBearExtElem,
        input: &WebGpuBuffer<BabyBearElem>,
        combos: &WebGpuBuffer<u32>,
        input_size: usize,
        count: usize,
    ) -> Result<bool> {
        if count == 0 || input_size == 0 {
            return Ok(true);
        }
        let (Some(output_gpu), Some(input_gpu), Some(combos_gpu)) =
            (output.raw_buffer(), input.raw_buffer(), combos.raw_buffer())
        else {
            return Ok(false);
        };
        if !self.storage_binding_fits(output) || !self.storage_binding_fits(combos) {
            return Ok(false);
        }

        let input_poly_bytes = byte_len_for::<BabyBearElem>(count);
        if input_poly_bytes == 0
            || input_poly_bytes > self.max_storage_binding_bytes()
            || input.byte_offset() % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0
            || input_poly_bytes % WEBGPU_STORAGE_BUFFER_OFFSET_ALIGNMENT != 0
        {
            return Ok(false);
        }
        let max_polys_per_chunk =
            (self.max_storage_binding_bytes() / input_poly_bytes).max(1) as usize;
        if max_polys_per_chunk == 0 {
            return Ok(false);
        }

        let Some(output_base) = output
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
        else {
            return Err(anyhow!("WebGPU mix_poly_coeffs output offset exceeds u32"));
        };
        let combos_base = u32::try_from(combos.elem_offset)
            .expect("WebGPU mix_poly_coeffs combos offset exceeds u32");

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

        let mix_words = mix.subelems();
        for chunk_start in (0..input_size).step_by(max_polys_per_chunk) {
            let chunk_size = (input_size - chunk_start).min(max_polys_per_chunk);
            let chunk_input_bytes = input_poly_bytes
                .checked_mul(u64::try_from(chunk_size).expect("WebGPU mix chunk size exceeds u64"))
                .ok_or_else(|| anyhow!("WebGPU mix_poly_coeffs chunk byte length overflow"))?;
            let input_offset = input
                .byte_offset()
                .checked_add(
                    u64::try_from(chunk_start)
                        .ok()
                        .and_then(|start| start.checked_mul(input_poly_bytes))
                        .ok_or_else(|| anyhow!("WebGPU mix_poly_coeffs input offset overflow"))?,
                )
                .ok_or_else(|| anyhow!("WebGPU mix_poly_coeffs input offset overflow"))?;
            let chunk_mix_start = *mix_start * mix.pow(chunk_start);
            let chunk_mix_start = chunk_mix_start.subelems();
            let params = [
                u32::try_from(chunk_size).expect("WebGPU mix_poly_coeffs input_size exceeds u32"),
                u32::try_from(count).expect("WebGPU mix_poly_coeffs count exceeds u32"),
                output_base,
                0,
                combos_base
                    .checked_add(
                        u32::try_from(chunk_start)
                            .expect("WebGPU mix_poly_coeffs combo chunk offset exceeds u32"),
                    )
                    .ok_or_else(|| anyhow!("WebGPU mix_poly_coeffs combo offset overflow"))?,
                0,
                0,
                0,
                chunk_mix_start[0].as_u32_montgomery(),
                chunk_mix_start[1].as_u32_montgomery(),
                chunk_mix_start[2].as_u32_montgomery(),
                chunk_mix_start[3].as_u32_montgomery(),
                mix_words[0].as_u32_montgomery(),
                mix_words[1].as_u32_montgomery(),
                mix_words[2].as_u32_montgomery(),
                mix_words[3].as_u32_montgomery(),
            ];
            let params = self.create_uniform_buffer(
                "webgpu_mix_poly_coeffs_params",
                bytemuck::cast_slice(&params),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_mix_poly_coeffs_bind_group",
                &layout,
                &[
                    WebGpuBufferBinding::new(0, output_gpu),
                    WebGpuBufferBinding {
                        binding: 1,
                        buffer: input_gpu,
                        offset: input_offset,
                        size: Some(chunk_input_bytes),
                    },
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
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_combos_prepare(
        &self,
        combos: &WebGpuBuffer<BabyBearExtElem>,
        coeff_u: &[BabyBearExtElem],
        combo_count: usize,
        cycles: usize,
        reg_sizes: &[u32],
        reg_combo_ids: &[u32],
        mix: &BabyBearExtElem,
    ) -> Result<bool> {
        if reg_sizes.len() != reg_combo_ids.len() {
            return Ok(false);
        }
        let total_reg_coeffs = reg_sizes
            .iter()
            .try_fold(0usize, |acc, size| acc.checked_add(*size as usize))
            .ok_or_else(|| anyhow!("WebGPU combos_prepare register size overflow"))?;
        ensure!(
            coeff_u.len() == total_reg_coeffs + <Self as Hal>::CHECK_SIZE,
            "WebGPU combos_prepare coeff_u length mismatch: got {}, expected {}",
            coeff_u.len(),
            total_reg_coeffs + <Self as Hal>::CHECK_SIZE
        );
        if combos.size() == 0 {
            return Ok(true);
        }

        let Some(combos_gpu) = combos.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(combos) {
            return Ok(false);
        }

        let coeff_u = self.copy_from_extelem("webgpu_combos_prepare_coeff_u", coeff_u);
        let reg_sizes = self.copy_from_u32("webgpu_combos_prepare_reg_sizes", reg_sizes);
        let reg_combo_ids =
            self.copy_from_u32("webgpu_combos_prepare_reg_combo_ids", reg_combo_ids);

        let mut mix_pows = Vec::with_capacity(reg_sizes.size() + <Self as Hal>::CHECK_SIZE);
        let mut cur = BabyBearExtElem::ONE;
        for _ in 0..reg_sizes.size() {
            mix_pows.push(cur);
            cur *= *mix;
        }
        for _ in 0..<Self as Hal>::CHECK_SIZE {
            mix_pows.push(cur);
            cur *= *mix;
        }
        let mix_pows = self.copy_from_extelem("webgpu_combos_prepare_mix_pows", &mix_pows);

        let (Some(coeff_u_gpu), Some(reg_sizes_gpu), Some(reg_combo_ids_gpu), Some(mix_pows_gpu)) = (
            coeff_u.raw_buffer(),
            reg_sizes.raw_buffer(),
            reg_combo_ids.raw_buffer(),
            mix_pows.raw_buffer(),
        ) else {
            return Ok(false);
        };
        if !self.storage_binding_fits(&coeff_u)
            || !self.storage_binding_fits(&reg_sizes)
            || !self.storage_binding_fits(&reg_combo_ids)
            || !self.storage_binding_fits(&mix_pows)
        {
            return Ok(false);
        }

        let combos_base = combos
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU combos_prepare combos offset exceeds u32"))?;
        let coeff_u_base = coeff_u
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU combos_prepare coeff_u offset exceeds u32"))?;
        let mix_pows_base = mix_pows
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU combos_prepare mix_pows offset exceeds u32"))?;
        let params = [
            u32::try_from(total_reg_coeffs)
                .expect("WebGPU combos_prepare total_reg_coeffs exceeds u32"),
            u32::try_from(combo_count).expect("WebGPU combos_prepare combo_count exceeds u32"),
            u32::try_from(cycles).expect("WebGPU combos_prepare cycles exceeds u32"),
            u32::try_from(reg_sizes.size()).expect("WebGPU combos_prepare regs_count exceeds u32"),
            combos_base,
            coeff_u_base,
            u32::try_from(reg_sizes.elem_offset)
                .expect("WebGPU combos_prepare reg_sizes offset exceeds u32"),
            u32::try_from(reg_combo_ids.elem_offset)
                .expect("WebGPU combos_prepare reg_combo_ids offset exceeds u32"),
            mix_pows_base,
            u32::try_from(<Self as Hal>::CHECK_SIZE)
                .expect("WebGPU combos_prepare check size exceeds u32"),
            0,
            0,
        ];
        let params = self.create_uniform_buffer(
            "webgpu_combos_prepare_params",
            bytemuck::cast_slice(&params),
        )?;

        combos.sync_cpu_to_gpu(self)?;
        coeff_u.sync_cpu_to_gpu(self)?;
        reg_sizes.sync_cpu_to_gpu(self)?;
        reg_combo_ids.sync_cpu_to_gpu(self)?;
        mix_pows.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_combos_prepare_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::read_only_storage(4, 0),
                WebGpuBindingLayout::uniform(5, 48),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_combos_prepare",
            COMBOS_PREPARE_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_combos_prepare_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, combos_gpu),
                WebGpuBufferBinding::new(1, coeff_u_gpu),
                WebGpuBufferBinding::new(2, reg_sizes_gpu),
                WebGpuBufferBinding::new(3, reg_combo_ids_gpu),
                WebGpuBufferBinding::new(4, mix_pows_gpu),
                WebGpuBufferBinding {
                    binding: 5,
                    buffer: &params,
                    offset: 0,
                    size: Some(48),
                },
            ],
        )?;
        let work_items = u32::try_from(total_reg_coeffs + 1)
            .expect("WebGPU combos_prepare work item count exceeds u32");
        self.dispatch_compute(
            &kernel,
            &bind_group,
            work_items.div_ceil(WEBGPU_WORKGROUP_SIZE),
            1,
            1,
        );
        Ok(true)
    }

    fn dispatch_combos_divide(
        &self,
        combos: &WebGpuBuffer<BabyBearExtElem>,
        chunks: &[(usize, Vec<BabyBearExtElem>)],
        cycles: usize,
    ) -> Result<bool> {
        if chunks.is_empty() || cycles == 0 {
            return Ok(true);
        }

        let Some(combos_gpu) = combos.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(combos) {
            return Ok(false);
        }

        let mut chunk_indices = Vec::with_capacity(chunks.len());
        let mut chunk_offsets = Vec::with_capacity(chunks.len() + 1);
        let mut pows = Vec::new();
        chunk_offsets.push(0u32);
        for (combo_idx, chunk_pows) in chunks {
            ensure!(
                combo_idx
                    .checked_mul(cycles)
                    .and_then(|start| start.checked_add(cycles))
                    .is_some_and(|end| end <= combos.size()),
                "WebGPU combos_divide combo index {combo_idx} is out of range"
            );
            chunk_indices.push(
                u32::try_from(*combo_idx).expect("WebGPU combos_divide combo index exceeds u32"),
            );
            pows.extend(chunk_pows.iter().copied());
            chunk_offsets.push(
                u32::try_from(pows.len()).expect("WebGPU combos_divide pow count exceeds u32"),
            );
        }

        let pows = self.copy_from_extelem("webgpu_combos_divide_pows", &pows);
        let chunk_indices =
            self.copy_from_u32("webgpu_combos_divide_chunk_indices", &chunk_indices);
        let chunk_offsets =
            self.copy_from_u32("webgpu_combos_divide_chunk_offsets", &chunk_offsets);
        let (Some(pows_gpu), Some(chunk_indices_gpu), Some(chunk_offsets_gpu)) = (
            pows.raw_buffer(),
            chunk_indices.raw_buffer(),
            chunk_offsets.raw_buffer(),
        ) else {
            return Ok(false);
        };
        if !self.storage_binding_fits(&pows)
            || !self.storage_binding_fits(&chunk_indices)
            || !self.storage_binding_fits(&chunk_offsets)
        {
            return Ok(false);
        }

        let combos_base = combos
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU combos_divide combos offset exceeds u32"))?;
        let pows_base = pows
            .elem_offset
            .checked_mul(BabyBearExtElem::EXT_SIZE)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| anyhow!("WebGPU combos_divide pows offset exceeds u32"))?;
        let params = [
            u32::try_from(chunks.len()).expect("WebGPU combos_divide chunk count exceeds u32"),
            u32::try_from(cycles).expect("WebGPU combos_divide cycles exceeds u32"),
            combos_base,
            pows_base,
            u32::try_from(chunk_indices.elem_offset)
                .expect("WebGPU combos_divide chunk index offset exceeds u32"),
            u32::try_from(chunk_offsets.elem_offset)
                .expect("WebGPU combos_divide chunk offset offset exceeds u32"),
            0,
            0,
        ];
        let params = self
            .create_uniform_buffer("webgpu_combos_divide_params", bytemuck::cast_slice(&params))?;

        combos.sync_cpu_to_gpu(self)?;
        pows.sync_cpu_to_gpu(self)?;
        chunk_indices.sync_cpu_to_gpu(self)?;
        chunk_offsets.sync_cpu_to_gpu(self)?;

        let layout = self.create_bind_group_layout(
            "webgpu_combos_divide_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::read_only_storage(2, 0),
                WebGpuBindingLayout::read_only_storage(3, 0),
                WebGpuBindingLayout::uniform(4, 32),
            ],
        )?;
        let kernel = self.create_compute_kernel(
            "webgpu_combos_divide",
            COMBOS_DIVIDE_WGSL,
            "main",
            &[layout.clone()],
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_combos_divide_bind_group",
            &layout,
            &[
                WebGpuBufferBinding::new(0, combos_gpu),
                WebGpuBufferBinding::new(1, pows_gpu),
                WebGpuBufferBinding::new(2, chunk_indices_gpu),
                WebGpuBufferBinding::new(3, chunk_offsets_gpu),
                WebGpuBufferBinding {
                    binding: 4,
                    buffer: &params,
                    offset: 0,
                    size: Some(32),
                },
            ],
        )?;
        self.dispatch_compute(
            &kernel,
            &bind_group,
            u32::try_from(chunks.len()).expect("WebGPU combos_divide chunk count exceeds u32"),
            1,
            1,
        );
        Ok(true)
    }

    fn dispatch_batch_expand_into_evaluate_ntt(
        &self,
        output: &WebGpuBuffer<BabyBearElem>,
        input: &WebGpuBuffer<BabyBearElem>,
        count: usize,
        expand_bits: usize,
    ) -> Result<bool> {
        if !self.batch_expand_into_evaluate_ntt_gpu_enabled.get() {
            return Ok(false);
        }
        if output.size() == 0 {
            return Ok(true);
        }

        let (Some(output_gpu), Some(input_gpu)) = (output.raw_buffer(), input.raw_buffer()) else {
            return Ok(false);
        };
        if output_gpu == input_gpu {
            return Ok(false);
        }
        if !self.storage_binding_fits(output) || !self.storage_binding_fits(input) {
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
            u32::try_from(actual_expand_bits).expect("WebGPU NTT expand_bits exceeds u32"),
            u32::try_from(output.elem_offset).expect("WebGPU NTT output offset exceeds u32"),
            u32::try_from(input.elem_offset).expect("WebGPU NTT input offset exceeds u32"),
            0,
            0,
        ];
        let expand_params = self.create_uniform_buffer(
            "webgpu_batch_expand_params",
            bytemuck::cast_slice(&expand_params),
        )?;
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
            .div_ceil(WEBGPU_WORKGROUP_SIZE);
        self.dispatch_compute_1d(&expand_kernel, &expand_bind_group, expand_workgroups);

        let row_size = output.size() / count;
        assert_eq!(row_size * count, output.size());
        let n_bits = crate::core::log2_ceil(row_size);
        assert_eq!(row_size, 1 << n_bits);
        assert!(n_bits >= expand_bits);
        assert!(n_bits < BabyBearElem::MAX_ROU_PO2);
        if n_bits == expand_bits {
            return Ok(true);
        }

        // SP-CR D15 (2026-05-12): use cached roots buffer instead of per-call alloc.
        let roots = self
            .ntt_roots_fwd
            .as_ref()
            .ok_or_else(|| anyhow!("WebGPU NTT roots_fwd not initialized"))?;
        let Some(roots_gpu) = roots.raw_buffer() else {
            return Ok(false);
        };
        let ntt_layout = self.create_bind_group_layout(
            "webgpu_ntt_step_dynamic_layout",
            &[
                WebGpuBindingLayout::storage(0, 0),
                WebGpuBindingLayout::read_only_storage(1, 0),
                WebGpuBindingLayout::uniform_dynamic(2, 32),
            ],
        )?;
        let ntt_kernel = self.create_compute_kernel(
            "webgpu_ntt_step_dynamic",
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
        // SP-submission iter 1 (2026-05-15): batch all NTT step
        // dispatches into ONE command encoder + submit instead of
        // per-step submit. Each NTT level reads its predecessor's
        // output; within a single compute pass, dispatches execute
        // serially on the queue so write-after-write ordering is
        // preserved without inserting barriers. For xgboost po2_18
        // this collapses 16 GPU-process IPC round-trips per NTT call
        // into 1. NTT runs many times per segment, so the cumulative
        // submission-overhead savings stack.
        //
        let params_len = mem::size_of::<[u32; 8]>();
        let params_stride = align_up(
            params_len,
            self.min_uniform_buffer_offset_alignment as usize,
        );
        let mut params_bytes = Vec::new();
        let mut params_offsets: Vec<u32> = Vec::with_capacity((n_bits - expand_bits) as usize);
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
            let params_offset = params_bytes.len();
            params_bytes.resize(params_offset + params_stride, 0);
            params_bytes[params_offset..params_offset + params_len]
                .copy_from_slice(bytemuck::cast_slice(&params));
            params_offsets.push(
                u32::try_from(params_offset)
                    .map_err(|_| anyhow!("WebGPU NTT params offset exceeds u32"))?,
            );
        }
        let params_buf = self.create_buffer(
            "webgpu_ntt_step_params",
            params_bytes.len() as u64,
            WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;
        self.write_buffer_named(&params_buf, "webgpu_ntt_step_params", 0, params_bytes.as_slice())?;
        let bind_group = self.create_bind_group(
            "webgpu_ntt_step_bind_group",
            &ntt_layout,
            &[
                WebGpuBufferBinding::new(0, output_gpu),
                WebGpuBufferBinding::new(1, roots_gpu),
                WebGpuBufferBinding {
                    binding: 2,
                    buffer: &params_buf,
                    offset: 0,
                    size: Some(params_len as u64),
                },
            ],
        )?;
        let (workgroups_x, workgroups_y) = if workgroups <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION {
            (workgroups, 1)
        } else {
            let workgroups_y = workgroups.div_ceil(WEBGPU_MAX_WORKGROUPS_PER_DIMENSION);
            assert!(
                workgroups_y <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION,
                "WebGPU NTT 1D dispatch exceeds portable 2D workgroup capacity"
            );
            (WEBGPU_MAX_WORKGROUPS_PER_DIMENSION, workgroups_y)
        };
        let encoder = self.device.create_command_encoder();
        let pass = encoder.begin_compute_pass();
        pass.set_pipeline(&ntt_kernel.pipeline);
        for params_offset in &params_offsets {
            let dynamic_offsets = [*params_offset];
            pass.set_bind_group_with_u32_slice_and_u32_and_dynamic_offsets_data_length(
                0,
                Some(&bind_group),
                &dynamic_offsets,
                0,
                1,
            )
            .map_err(js_error)?;
            pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
                workgroups_x,
                workgroups_y,
                1,
            );
        }
        pass.end();
        self.submit(encoder.finish());
        Ok(true)
    }

    fn dispatch_batch_interpolate_ntt(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        count: usize,
    ) -> Result<bool> {
        if !self.batch_interpolate_ntt_gpu_enabled.get() {
            return Ok(false);
        }
        if io.size() == 0 {
            return Ok(true);
        }

        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(io) {
            return Ok(false);
        }

        let row_size = io.size() / count;
        assert_eq!(row_size * count, io.size());
        let n_bits = crate::core::log2_ceil(row_size);
        assert_eq!(row_size, 1 << n_bits);
        assert!(n_bits < BabyBearElem::MAX_ROU_PO2);

        io.sync_cpu_to_gpu(self)?;

        if n_bits != 0 {
            // SP-CR D15 (2026-05-12): use cached roots buffer instead of per-call alloc.
            let roots = self
                .ntt_roots_rev
                .as_ref()
                .ok_or_else(|| anyhow!("WebGPU NTT roots_rev not initialized"))?;
            let Some(roots_gpu) = roots.raw_buffer() else {
                return Ok(false);
            };
            let ntt_layout = self.create_bind_group_layout(
                "webgpu_ntt_step_dynamic_layout",
                &[
                    WebGpuBindingLayout::storage(0, 0),
                    WebGpuBindingLayout::read_only_storage(1, 0),
                    WebGpuBindingLayout::uniform_dynamic(2, 32),
                ],
            )?;
            let ntt_kernel = self.create_compute_kernel(
                "webgpu_ntt_step_dynamic",
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
            let params_len = mem::size_of::<[u32; 8]>();
            let params_stride = align_up(
                params_len,
                self.min_uniform_buffer_offset_alignment as usize,
            );
            let mut params_bytes = Vec::new();
            let mut params_offsets: Vec<u32> = Vec::with_capacity(n_bits as usize);
            for s_bits in (1..=n_bits).rev() {
                let params = [
                    u32::try_from(n_bits).expect("WebGPU inverse NTT n_bits exceeds u32"),
                    u32::try_from(s_bits).expect("WebGPU inverse NTT s_bits exceeds u32"),
                    u32::try_from(count).expect("WebGPU inverse NTT row count exceeds u32"),
                    u32::try_from(total_pairs).expect("WebGPU inverse NTT total pairs exceeds u32"),
                    u32::try_from(io.elem_offset)
                        .expect("WebGPU inverse NTT io offset exceeds u32"),
                    u32::try_from(roots.elem_offset)
                        .expect("WebGPU inverse NTT roots offset exceeds u32"),
                    1,
                    0,
                ];
                let params_offset = params_bytes.len();
                params_bytes.resize(params_offset + params_stride, 0);
                params_bytes[params_offset..params_offset + params_len]
                    .copy_from_slice(bytemuck::cast_slice(&params));
                params_offsets.push(
                    u32::try_from(params_offset)
                        .map_err(|_| anyhow!("WebGPU inverse NTT params offset exceeds u32"))?,
                );
            }
            let params_buf = self.create_buffer(
                "webgpu_ntt_step_params",
                params_bytes.len() as u64,
                WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
            )?;
            self.write_buffer_named(
                &params_buf,
                "webgpu_ntt_step_params",
                0,
                params_bytes.as_slice(),
            )?;
            let bind_group = self.create_bind_group(
                "webgpu_ntt_step_bind_group",
                &ntt_layout,
                &[
                    WebGpuBufferBinding::new(0, io_gpu),
                    WebGpuBufferBinding::new(1, roots_gpu),
                    WebGpuBufferBinding {
                        binding: 2,
                        buffer: &params_buf,
                        offset: 0,
                        size: Some(params_len as u64),
                    },
                ],
            )?;
            let (workgroups_x, workgroups_y) = if workgroups <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION
            {
                (workgroups, 1)
            } else {
                let workgroups_y = workgroups.div_ceil(WEBGPU_MAX_WORKGROUPS_PER_DIMENSION);
                assert!(
                    workgroups_y <= WEBGPU_MAX_WORKGROUPS_PER_DIMENSION,
                    "WebGPU inverse NTT 1D dispatch exceeds portable 2D workgroup capacity"
                );
                (WEBGPU_MAX_WORKGROUPS_PER_DIMENSION, workgroups_y)
            };
            let encoder = self.device.create_command_encoder();
            let pass = encoder.begin_compute_pass();
            pass.set_pipeline(&ntt_kernel.pipeline);
            for params_offset in &params_offsets {
                let dynamic_offsets = [*params_offset];
                pass.set_bind_group_with_u32_slice_and_u32_and_dynamic_offsets_data_length(
                    0,
                    Some(&bind_group),
                    &dynamic_offsets,
                    0,
                    1,
                )
                .map_err(js_error)?;
                pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
                    workgroups_x,
                    workgroups_y,
                    1,
                );
            }
            pass.end();
            self.submit(encoder.finish());
        }

        let norm = BabyBearElem::new(row_size as u32).inv().as_u32_montgomery();
        let params = [
            u32::try_from(io.size()).expect("WebGPU inverse NTT size exceeds u32"),
            u32::try_from(io.elem_offset).expect("WebGPU inverse NTT io offset exceeds u32"),
            norm,
            0,
        ];
        let params = self
            .create_uniform_buffer("webgpu_ntt_normalize_params", bytemuck::cast_slice(&params))?;
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
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
        Ok(true)
    }

    fn dispatch_batch_bit_reverse(
        &self,
        io: &WebGpuBuffer<BabyBearElem>,
        bits: usize,
    ) -> Result<bool> {
        if !self.batch_bit_reverse_gpu_enabled.get() {
            return Ok(false);
        }
        if io.size() == 0 {
            return Ok(true);
        }

        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(io) {
            return Ok(false);
        }

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
            .div_ceil(WEBGPU_WORKGROUP_SIZE);
        self.dispatch_compute_1d(&kernel, &bind_group, workgroups);
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
        if !self.storage_binding_fits(out)
            || !self.storage_binding_fits(coeffs)
            || !self.storage_binding_fits(which)
            || !self.storage_binding_fits(xs)
        {
            return Ok(false);
        }

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
            u32::try_from(deg).expect("WebGPU batch_evaluate_any degree exceeds u32"),
        ];

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

        if self.storage_binding_fits(coeffs) {
            let params = self.create_uniform_buffer(
                "webgpu_batch_evaluate_any_params",
                bytemuck::cast_slice(&params),
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
            self.dispatch_compute(
                &kernel,
                &bind_group,
                u32::try_from(eval_count)
                    .expect("WebGPU batch_evaluate_any eval_count exceeds u32")
                    .div_ceil(WEBGPU_WORKGROUP_SIZE),
                1,
                1,
            );
            return Ok(true);
        }
        Ok(false)
    }

    fn dispatch_poseidon2_hash_fold(
        &self,
        io: &WebGpuBuffer<Digest>,
        input_size: usize,
        output_size: usize,
    ) -> Result<bool> {
        if !self.hash_fold_gpu_enabled.get() {
            return Ok(false);
        }
        let Some(hash) = self.poseidon2.as_ref() else {
            return Ok(false);
        };
        if output_size == 0 {
            return Ok(true);
        }

        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(io) {
            return Ok(false);
        }
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
        let params = self.create_uniform_buffer(
            "webgpu_poseidon2_fold_params",
            bytemuck::cast_slice(&params),
        )?;

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

    /// SP-submission iter 2 (2026-05-15): batch a chain of hash_folds
    /// (the merkle tree-build loop) into a single command encoder. Each
    /// individual fold writes to a different slice of the same `nodes`
    /// buffer; within a compute pass dispatches execute serially on the
    /// queue so write-after-write ordering across layers is preserved.
    ///
    /// `output_sizes` lists the per-layer output_size; input_size is
    /// always `2 * output_size` per the merkle tree-build invariant.
    /// Returns true when the GPU path is used; falls back through
    /// individual hash_fold dispatches on any short-circuit condition.
    pub(crate) fn dispatch_poseidon2_hash_fold_chain(
        &self,
        io: &WebGpuBuffer<Digest>,
        output_sizes: &[usize],
    ) -> Result<bool> {
        if !self.hash_fold_gpu_enabled.get() {
            return Ok(false);
        }
        let Some(hash) = self.poseidon2.as_ref() else {
            return Ok(false);
        };
        if output_sizes.is_empty() {
            return Ok(true);
        }
        let Some(io_gpu) = io.raw_buffer() else {
            return Ok(false);
        };
        if !self.storage_binding_fits(io) {
            return Ok(false);
        }
        let (Some(round_constants_gpu), Some(m_int_diag_gpu)) = (
            hash.round_constants.raw_buffer(),
            hash.m_int_diag.raw_buffer(),
        ) else {
            return Ok(false);
        };
        io.sync_cpu_to_gpu(self)?;

        // SP6g (2026-05-18): pack all per-layer params into one dynamic
        // uniform buffer and bind it once. This keeps the existing single-pass
        // dispatch batching while avoiding one bind group per Merkle layer.
        let params_len = mem::size_of::<[u32; 8]>();
        let params_stride = align_up(
            params_len,
            self.min_uniform_buffer_offset_alignment as usize,
        );
        let mut params_bytes = Vec::new();
        let mut params_offsets: Vec<u32> = Vec::with_capacity(output_sizes.len());
        let mut workgroups_per_layer: Vec<u32> = Vec::with_capacity(output_sizes.len());
        for &output_size in output_sizes {
            if output_size == 0 {
                continue;
            }
            let input_size = 2 * output_size;
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
            let params_offset = params_bytes.len();
            params_bytes.resize(params_offset + params_stride, 0);
            params_bytes[params_offset..params_offset + params_len]
                .copy_from_slice(bytemuck::cast_slice(&params));
            params_offsets.push(
                u32::try_from(params_offset)
                    .map_err(|_| anyhow!("WebGPU hash_fold params offset exceeds u32"))?,
            );
            workgroups_per_layer.push(
                u32::try_from(output_size)
                    .expect("WebGPU hash_fold output size exceeds u32")
                    .div_ceil(256),
            );
        }
        if params_offsets.is_empty() {
            return Ok(true);
        }
        let params_buf = self.create_buffer(
            "webgpu_poseidon2_fold_chain_params",
            params_bytes.len() as u64,
            WEBGPU_BUFFER_USAGE_UNIFORM | WEBGPU_BUFFER_USAGE_COPY_DST,
        )?;
        self.write_buffer_named(
            &params_buf,
            "webgpu_poseidon2_fold_chain_params",
            0,
            params_bytes.as_slice(),
        )?;
        let bind_group = self.create_bind_group(
            "webgpu_poseidon2_fold_chain_bind_group",
            &hash.fold_chain_layout,
            &[
                WebGpuBufferBinding::new(0, round_constants_gpu),
                WebGpuBufferBinding::new(1, m_int_diag_gpu),
                WebGpuBufferBinding::new(2, io_gpu),
                WebGpuBufferBinding {
                    binding: 4,
                    buffer: &params_buf,
                    offset: 0,
                    size: Some(params_len as u64),
                },
            ],
        )?;
        let encoder = self.device.create_command_encoder();
        let pass = encoder.begin_compute_pass();
        pass.set_pipeline(&hash.fold_chain_kernel.pipeline);
        for (params_offset, workgroups) in params_offsets.iter().zip(workgroups_per_layer.iter()) {
            let dynamic_offsets = [*params_offset];
            pass.set_bind_group_with_u32_slice_and_u32_and_dynamic_offsets_data_length(
                0,
                Some(&bind_group),
                &dynamic_offsets,
                0,
                1,
            )
            .map_err(js_error)?;
            pass.dispatch_workgroups_with_workgroup_count_y_and_workgroup_count_z(
                *workgroups, 1, 1,
            );
        }
        pass.end();
        self.submit(encoder.finish());
        // Mirror finish_hal_op accounting for each layer.
        for _ in 0..params_offsets.len() {
            self.record_gpu_result_authoritative("hash_fold", true);
        }
        io.mark_gpu_dirty();
        Ok(true)
    }

    fn dispatch_poseidon2_hash_rows(
        &self,
        output: &WebGpuBuffer<Digest>,
        matrix: &WebGpuBuffer<BabyBearElem>,
        row_size: usize,
        col_size: usize,
    ) -> Result<bool> {
        if !self.hash_rows_gpu_enabled.get() {
            return Ok(false);
        }
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
        if !self.storage_binding_fits(output) || !self.storage_binding_fits(matrix) {
            return Ok(false);
        }
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
        let params = self.create_uniform_buffer(
            "webgpu_poseidon2_rows_params",
            bytemuck::cast_slice(&params),
        )?;

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

fn align_up(value: usize, alignment: usize) -> usize {
    if alignment == 0 {
        return value;
    }
    value.div_ceil(alignment) * alignment
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

    let adapter_limits = adapter.limits();
    let max_buffer_size = (adapter_limits.max_buffer_size() as u64)
        .min(WEBGPU_REQUESTED_MAX_BUFFER_BYTES)
        .min(MAX_EXACT_JS_INTEGER);
    let max_storage_buffer_binding_size = (adapter_limits.max_storage_buffer_binding_size() as u64)
        .min(WEBGPU_REQUESTED_MAX_STORAGE_BINDING_BYTES)
        .min(max_buffer_size)
        .min(MAX_EXACT_JS_INTEGER);
    let max_compute_workgroup_storage_size = adapter_limits
        .max_compute_workgroup_storage_size()
        .min(WEBGPU_REQUESTED_MAX_WORKGROUP_STORAGE_BYTES);
    let required_limits = js_sys::Object::new();
    set_required_limit(&required_limits, "maxBufferSize", max_buffer_size)?;
    set_required_limit(
        &required_limits,
        "maxStorageBufferBindingSize",
        max_storage_buffer_binding_size,
    )?;
    set_required_limit(
        &required_limits,
        "maxComputeWorkgroupStorageSize",
        max_compute_workgroup_storage_size as u64,
    )?;
    // SP3 iter 7c: bump storage-buffers-per-stage so the staged multi-stage
    // bind group (10 storage bindings) fits. Default WebGPU is 8.
    let max_storage_buffers_per_stage = adapter_limits
        .max_storage_buffers_per_shader_stage()
        .min(WEBGPU_REQUESTED_MAX_STORAGE_BUFFERS_PER_STAGE);
    set_required_limit(
        &required_limits,
        "maxStorageBuffersPerShaderStage",
        max_storage_buffers_per_stage as u64,
    )?;
    // SP3 iter 7x: bump `maxUniformBufferBindingSize` so `mix_pows`
    // can live in a UBO (CUDA `__constant__` analog).
    let max_uniform_buffer_binding_size = (adapter_limits
        .max_uniform_buffer_binding_size() as u64)
        .min(WEBGPU_REQUESTED_MAX_UNIFORM_BUFFER_BINDING_BYTES);
    set_required_limit(
        &required_limits,
        "maxUniformBufferBindingSize",
        max_uniform_buffer_binding_size,
    )?;
    let descriptor = web_sys::GpuDeviceDescriptor::new();
    descriptor.set_required_limits(&required_limits);

    JsFuture::from(adapter.request_device_with_descriptor(&descriptor))
        .await
        .map_err(js_error)
        .and_then(required_js("GPUDevice"))?
        .dyn_into::<web_sys::GpuDevice>()
        .map_err(|_| anyhow!("requestDevice did not return a GPUDevice"))
}

fn set_required_limit(limits: &js_sys::Object, name: &'static str, value: u64) -> Result<()> {
    js_sys::Reflect::set(
        limits,
        &JsValue::from_str(name),
        &JsValue::from_f64(value as f64),
    )
    .map_err(js_error)?;
    Ok(())
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

    Ok(Some(value.dyn_into::<web_sys::GpuAdapter>().map_err(
        |_| anyhow!("requestAdapter did not return a GPUAdapter"),
    )?))
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
            .unwrap_or_else(|err| panic!("failed to expand and evaluate NTT with WebGPU: {err}"));
        self.finish_hal_op(
            "batch_expand_into_evaluate_ntt",
            gpu_evaluated,
            output,
            || {
                self.cpu.batch_expand_into_evaluate_ntt(
                    output.cpu(),
                    input.cpu(),
                    count,
                    expand_bits,
                );
            },
        );
    }

    fn batch_interpolate_ntt(&self, io: &Self::Buffer<Self::Elem>, count: usize) {
        let gpu_interpolated = self
            .dispatch_batch_interpolate_ntt(io, count)
            .unwrap_or_else(|err| panic!("failed to interpolate NTT with WebGPU: {err}"));
        self.finish_hal_op("batch_interpolate_ntt", gpu_interpolated, io, || {
            self.cpu.batch_interpolate_ntt(io.cpu(), count);
        });
    }

    fn batch_bit_reverse(&self, io: &Self::Buffer<Self::Elem>, count: usize) {
        let row_size = io.size() / count;
        assert_eq!(row_size * count, io.size());
        let bits = crate::core::log2_ceil(row_size);
        assert_eq!(row_size, 1 << bits);
        let gpu_reversed = self
            .dispatch_batch_bit_reverse(io, bits)
            .unwrap_or_else(|err| panic!("failed to bit-reverse with WebGPU: {err}"));
        self.finish_hal_op("batch_bit_reverse", gpu_reversed, io, || {
            self.cpu.batch_bit_reverse(io.cpu(), count);
        });
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
        self.finish_hal_op("batch_evaluate_any", gpu_evaluated, out, || {
            self.cpu
                .batch_evaluate_any(coeffs.cpu(), poly_count, which.cpu(), xs.cpu(), out.cpu());
        });
    }

    fn zk_shift(&self, io: &Self::Buffer<Self::Elem>, count: usize) {
        let bits = crate::core::log2_ceil(io.size() / count);
        assert_eq!(io.size(), count * (1 << bits));
        let gpu_shifted = self
            .dispatch_zk_shift(io, bits)
            .unwrap_or_else(|err| panic!("failed to zk_shift with WebGPU: {err}"));
        self.finish_hal_op("zk_shift", gpu_shifted, io, || {
            self.cpu.zk_shift(io.cpu(), count);
        });
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
        self.finish_hal_op("mix_poly_coeffs", gpu_mixed, out, || {
            self.cpu.mix_poly_coeffs(
                out.cpu(),
                mix_start,
                mix,
                input.cpu(),
                combos.cpu(),
                input_size,
                count,
            );
        });
    }

    fn combos_prepare(
        &self,
        combos: &Self::Buffer<Self::ExtElem>,
        coeff_u: &[Self::ExtElem],
        combo_count: usize,
        cycles: usize,
        reg_sizes: &[u32],
        reg_combo_ids: &[u32],
        mix: &Self::ExtElem,
    ) {
        let gpu_prepared = self
            .dispatch_combos_prepare(
                combos,
                coeff_u,
                combo_count,
                cycles,
                reg_sizes,
                reg_combo_ids,
                mix,
            )
            .unwrap_or_else(|err| panic!("failed to prepare combos with WebGPU: {err}"));
        self.finish_hal_op("combos_prepare", gpu_prepared, combos, || {
            self.cpu.combos_prepare(
                combos.cpu(),
                coeff_u,
                combo_count,
                cycles,
                reg_sizes,
                reg_combo_ids,
                mix,
            );
        });
    }

    fn combos_divide(
        &self,
        combos: &Self::Buffer<Self::ExtElem>,
        chunks: Vec<(usize, Vec<Self::ExtElem>)>,
        cycles: usize,
    ) {
        let gpu_divided = self
            .dispatch_combos_divide(combos, &chunks, cycles)
            .unwrap_or_else(|err| panic!("failed to divide combos with WebGPU: {err}"));
        self.finish_hal_op("combos_divide", gpu_divided, combos, || {
            self.cpu.combos_divide(combos.cpu(), chunks, cycles);
        });
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
        self.finish_hal_op("eltwise_add_elem", gpu_added, output, || {
            self.cpu
                .eltwise_add_elem(output.cpu(), input1.cpu(), input2.cpu());
        });
    }

    fn eltwise_sum_extelem(
        &self,
        output: &Self::Buffer<Self::Elem>,
        input: &Self::Buffer<Self::ExtElem>,
    ) {
        let gpu_summed = self
            .dispatch_eltwise_sum_extelem(output, input)
            .unwrap_or_else(|err| panic!("failed to sum WebGPU extension buffers: {err}"));
        self.finish_hal_op("eltwise_sum_extelem", gpu_summed, output, || {
            self.cpu.eltwise_sum_extelem(output.cpu(), input.cpu());
        });
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

        self.finish_hal_op("eltwise_copy_elem", gpu_copied, output, || {
            self.cpu.eltwise_copy_elem(output.cpu(), input.cpu());
        });
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
        self.finish_hal_op("eltwise_copy_elem_slice", gpu_copied, into, || {
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
        });
    }

    fn eltwise_zeroize_elem(&self, elems: &Self::Buffer<Self::Elem>) {
        let gpu_zeroized = self
            .dispatch_zeroize_elem(elems)
            .unwrap_or_else(|err| panic!("failed to zeroize WebGPU buffer: {err}"));
        self.finish_hal_op("eltwise_zeroize_elem", gpu_zeroized, elems, || {
            self.cpu.eltwise_zeroize_elem(elems.cpu());
        });
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
        self.finish_hal_op("fri_fold", gpu_folded, output, || {
            self.cpu.fri_fold(output.cpu(), input.cpu(), mix);
        });
    }

    fn hash_rows(&self, output: &Self::Buffer<Digest>, matrix: &Self::Buffer<Self::Elem>) {
        let row_size = output.size();
        let col_size = matrix.size() / output.size();
        assert_eq!(matrix.size(), col_size * row_size);
        let gpu_hashed = self
            .dispatch_poseidon2_hash_rows(output, matrix, row_size, col_size)
            .unwrap_or_else(|err| panic!("failed to hash rows with WebGPU Poseidon2: {err}"));
        self.finish_hal_op("hash_rows", gpu_hashed, output, || {
            self.cpu.hash_rows(output.cpu(), matrix.cpu());
        });
    }

    fn hash_fold(&self, io: &Self::Buffer<Digest>, input_size: usize, output_size: usize) {
        assert!(io.size() >= 2 * input_size);
        assert_eq!(input_size, 2 * output_size);
        let gpu_hashed = self
            .dispatch_poseidon2_hash_fold(io, input_size, output_size)
            .unwrap_or_else(|err| panic!("failed to hash fold with WebGPU Poseidon2: {err}"));
        self.finish_hal_op("hash_fold", gpu_hashed, io, || {
            self.cpu.hash_fold(io.cpu(), input_size, output_size);
        });
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
        self.finish_hal_op("gather_sample", gpu_gathered, dst, || {
            self.cpu
                .gather_sample(dst.cpu(), src.cpu(), idx, size, stride);
        });
    }

    fn scatter(
        &self,
        into: &Self::Buffer<Self::Elem>,
        index: &[u32],
        offsets: &[u32],
        values: &[Self::Elem],
    ) {
        if !index.windows(2).any(|window| window[0] < window[1]) {
            return;
        }
        let gpu_scattered = self
            .dispatch_scatter(into, index, offsets, values)
            .unwrap_or_else(|err| panic!("failed to scatter WebGPU buffers: {err}"));
        self.finish_hal_op("scatter", gpu_scattered, into, || {
            self.cpu.scatter(into.cpu(), index, offsets, values);
        });
    }

    fn prefix_products(&self, io: &Self::Buffer<Self::ExtElem>) {
        let gpu_computed = self
            .dispatch_prefix_products(io)
            .unwrap_or_else(|err| panic!("failed to compute WebGPU prefix products: {err}"));
        self.finish_hal_op("prefix_products", gpu_computed, io, || {
            self.cpu.prefix_products(io.cpu());
        });
    }
}
