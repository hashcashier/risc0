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

//! `eval_check` program compilation: PolyExt tape -> interpreter
//! instruction stream and staged WGSL bodies.

use super::*;
#[allow(unused_imports)]
use super::{device::*, diagnostics::*, dispatch::*, kernels_wgsl::*, ops::*, resources::*};

#[derive(Clone, Copy)]
pub(crate) struct EvalCheckTap {
    pub(crate) group: usize,
    pub(crate) offset: usize,
    pub(crate) back: usize,
}

#[derive(Clone, Copy)]
pub(crate) enum EvalCheckFpOp {
    Const(u32),
    ConstExt(u32, u32, u32, u32),
    Get(usize),
    GetGlobal(usize, usize),
    Add(usize, usize),
    Sub(usize, usize),
    Mul(usize, usize),
}

#[derive(Clone, Copy)]
pub(crate) enum EvalCheckMixOp {
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
pub(crate) struct EvalCheckTerm {
    pub(crate) mix_exp: usize,
    pub(crate) conds: Vec<usize>,
    pub(crate) inner: usize,
}

pub(crate) struct EvalCheckProgram {
    pub(crate) fp_ops: Vec<EvalCheckFpOp>,
    pub(crate) mix_ops: Vec<EvalCheckMixOp>,
    pub(crate) mix_exps: Vec<usize>,
}

pub(crate) fn elem_word(value: BabyBearElem) -> u32 {
    value.to_u32_words()[0]
}

pub(crate) fn elem_const_word(value: u32) -> u32 {
    elem_word(BabyBearElem::from_u64(value as u64))
}

pub(crate) fn ext_words(value: BabyBearExtElem) -> [u32; 4] {
    let elems = value.subelems();
    [
        elem_word(elems[0]),
        elem_word(elems[1]),
        elem_word(elems[2]),
        elem_word(elems[3]),
    ]
}

pub(crate) fn eval_check_mix_exponents(def: &PolyExtStepDef) -> Result<Vec<usize>> {
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

pub(crate) fn eval_check_mix_pows(
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

pub(crate) fn eval_check_all_mix_pows(
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

pub(crate) fn eval_check_program(def: &PolyExtStepDef) -> Result<EvalCheckProgram> {
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

pub(crate) fn eval_check_flatten_terms(
    program: &EvalCheckProgram,
    ret: usize,
) -> Result<Vec<EvalCheckTerm>> {
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

pub(crate) fn eval_check_fp_dependencies(op: EvalCheckFpOp) -> &'static [usize] {
    match op {
        EvalCheckFpOp::Add(_, _) | EvalCheckFpOp::Sub(_, _) | EvalCheckFpOp::Mul(_, _) => {
            // Handled by `eval_check_note_fp_dependencies`.
            &[]
        }
        _ => &[],
    }
}

pub(crate) fn eval_check_note_fp_dependencies(op: EvalCheckFpOp, stack: &mut Vec<usize>) {
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

pub(crate) fn eval_check_needed_fp_vars(
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

pub(crate) fn eval_check_split_term_chunks(
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

pub(crate) fn eval_check_zerofier_inv_words(po2: usize, steps: usize) -> [u32; 4] {
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

pub(crate) fn eval_check_ext_const(words: [u32; 4]) -> String {
    format!(
        "vec4<u32>({}u, {}u, {}u, {}u)",
        words[0], words[1], words[2], words[3]
    )
}

// EvalCheckSlotAllocator, eval_check_last_uses, eval_check_fp_slot, and
// eval_check_mix_slot moved to `risc0/zkp/src/hal/webgpu_codegen.rs` so
// the staged-WGSL emitter (which is built on all targets when the
// `webgpu` feature is on) can share the same slot-allocation discipline
// as the runtime interpreter here.

/// Lazy (on-demand) reordering of a poly_ext tape. Mix ops keep their
/// original relative order (so mix var ids and mix-pow indices are
/// unchanged); each fp op is emitted immediately before its first
/// consumer via an iterative post-order walk of the operand DAG, and fp
/// var ids are renumbered to the new emission order. Every emitted op
/// computes the same field values from the same operands, so the check
/// output is bit-identical — only peak fp liveness (and therefore the
/// interpreter's scratch array size) changes.
pub(crate) fn eval_check_reorder_lazy(block: &[PolyExtStep]) -> Result<Vec<PolyExtStep>> {
    // Producing op index for each fp var, in original tape order.
    let mut fp_producer: Vec<usize> = Vec::new();
    for (op_idx, op) in block.iter().enumerate() {
        match op {
            PolyExtStep::Const(_)
            | PolyExtStep::ConstExt(_, _, _, _)
            | PolyExtStep::Get(_)
            | PolyExtStep::GetGlobal(_, _)
            | PolyExtStep::Add(_, _)
            | PolyExtStep::Sub(_, _)
            | PolyExtStep::Mul(_, _) => fp_producer.push(op_idx),
            PolyExtStep::True | PolyExtStep::AndEqz(_, _) | PolyExtStep::AndCond(_, _, _) => {}
        }
    }

    let mut new_fp_id: Vec<Option<usize>> = vec![None; fp_producer.len()];
    let mut out: Vec<PolyExtStep> = Vec::with_capacity(block.len());
    let mut next_fp = 0usize;
    let mut stack: Vec<(usize, bool)> = Vec::new();

    let emit_fp = |root: usize,
                   new_fp_id: &mut Vec<Option<usize>>,
                   out: &mut Vec<PolyExtStep>,
                   next_fp: &mut usize,
                   stack: &mut Vec<(usize, bool)>|
     -> Result<usize> {
        ensure!(
            root < fp_producer.len(),
            "poly_ext fp operand {root} out of range"
        );
        if let Some(id) = new_fp_id[root] {
            return Ok(id);
        }
        stack.clear();
        stack.push((root, false));
        while let Some((var, expanded)) = stack.pop() {
            if new_fp_id[var].is_some() {
                continue;
            }
            let op = &block[fp_producer[var]];
            if expanded {
                let remapped = match op {
                    PolyExtStep::Const(value) => PolyExtStep::Const(*value),
                    PolyExtStep::ConstExt(x0, x1, x2, x3) => {
                        PolyExtStep::ConstExt(*x0, *x1, *x2, *x3)
                    }
                    PolyExtStep::Get(tap) => PolyExtStep::Get(*tap),
                    PolyExtStep::GetGlobal(arg, offset) => PolyExtStep::GetGlobal(*arg, *offset),
                    PolyExtStep::Add(lhs, rhs) => PolyExtStep::Add(
                        new_fp_id[*lhs].expect("lazy reorder emitted op before its lhs"),
                        new_fp_id[*rhs].expect("lazy reorder emitted op before its rhs"),
                    ),
                    PolyExtStep::Sub(lhs, rhs) => PolyExtStep::Sub(
                        new_fp_id[*lhs].expect("lazy reorder emitted op before its lhs"),
                        new_fp_id[*rhs].expect("lazy reorder emitted op before its rhs"),
                    ),
                    PolyExtStep::Mul(lhs, rhs) => PolyExtStep::Mul(
                        new_fp_id[*lhs].expect("lazy reorder emitted op before its lhs"),
                        new_fp_id[*rhs].expect("lazy reorder emitted op before its rhs"),
                    ),
                    PolyExtStep::True
                    | PolyExtStep::AndEqz(_, _)
                    | PolyExtStep::AndCond(_, _, _) => {
                        bail!("poly_ext fp producer table pointed at a mix op")
                    }
                };
                out.push(remapped);
                new_fp_id[var] = Some(*next_fp);
                *next_fp += 1;
            } else {
                stack.push((var, true));
                if let PolyExtStep::Add(lhs, rhs)
                | PolyExtStep::Sub(lhs, rhs)
                | PolyExtStep::Mul(lhs, rhs) = op
                {
                    ensure!(
                        *lhs < var && *rhs < var,
                        "poly_ext fp operand references a later var"
                    );
                    if new_fp_id[*rhs].is_none() {
                        stack.push((*rhs, false));
                    }
                    if new_fp_id[*lhs].is_none() {
                        stack.push((*lhs, false));
                    }
                }
            }
        }
        new_fp_id[root].ok_or_else(|| anyhow!("lazy reorder failed to emit fp var {root}"))
    };

    for op in block {
        match op {
            PolyExtStep::True => out.push(PolyExtStep::True),
            PolyExtStep::AndEqz(chain, inner) => {
                let inner = emit_fp(*inner, &mut new_fp_id, &mut out, &mut next_fp, &mut stack)?;
                out.push(PolyExtStep::AndEqz(*chain, inner));
            }
            PolyExtStep::AndCond(chain, cond, inner) => {
                let cond = emit_fp(*cond, &mut new_fp_id, &mut out, &mut next_fp, &mut stack)?;
                out.push(PolyExtStep::AndCond(*chain, cond, *inner));
            }
            _ => {}
        }
    }
    Ok(out)
}

pub(crate) fn eval_check_interpreter_instructions_with_limit(
    taps: &TapSet<'_>,
    def: &PolyExtStepDef,
    max_fp_slots: usize,
) -> Result<(Vec<u32>, usize, usize, usize)> {
    let tap_info: Vec<_> = taps
        .taps()
        .map(|tap| EvalCheckTap {
            group: tap.group(),
            offset: tap.offset(),
            back: tap.back(),
        })
        .collect();
    let reordered;
    let block: &[PolyExtStep] = if WEBGPU_EVAL_CHECK_LAZY_REORDER {
        reordered = eval_check_reorder_lazy(def.block)?;
        &reordered
    } else {
        def.block
    };
    let (last_fp, last_mix, _fp_count, _mix_count) = eval_check_last_uses_block(block, def.ret)?;
    let mut fp_alloc = EvalCheckSlotAllocator::default();
    let mut mix_alloc = EvalCheckSlotAllocator::default();
    let mut fp_slots: Vec<Option<usize>> = Vec::new();
    let mut mix_slots: Vec<Option<usize>> = Vec::new();
    let mut instructions = Vec::new();

    let mut push_instr = |words: [u32; WEBGPU_EVAL_CHECK_INSTRUCTION_WORDS]| {
        instructions.extend(words);
    };

    for (op_idx, op) in block.iter().enumerate() {
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

    Ok((
        instructions,
        fp_alloc.max_used(),
        mix_alloc.max_used(),
        ret_slot,
    ))
}

pub(crate) fn eval_check_interpreter_instructions(
    taps: &TapSet<'_>,
    def: &PolyExtStepDef,
) -> Result<(Vec<u32>, usize, usize, usize)> {
    eval_check_interpreter_instructions_with_limit(taps, def, WEBGPU_EVAL_CHECK_MAX_FP_SLOTS)
}

/// Hybrid base-field instruction stream: base-field values live in a
/// scalar `u32` bank (ops 0..=9, unchanged from the original base
/// interpreter, so pure-base tapes like recursion's produce a
/// byte-identical stream), and values tainted by `ConstExt` live in a
/// small vec4 ext bank (ops 10..=18). Taint is static: `ConstExt` is
/// ext, and Add/Sub/Mul is ext iff either operand is. All field
/// operations remain exact canonical mod-P arithmetic, so results are
/// bit-identical to the all-ext interpreter.
///
/// Returns `(instructions, fp_slots, ext_slots, mix_slots, ret_mix_slot)`.
pub(crate) fn eval_check_base_interpreter_instructions(
    taps: &TapSet<'_>,
    def: &PolyExtStepDef,
) -> Result<(Vec<u32>, usize, usize, usize, usize)> {
    let tap_info: Vec<_> = taps
        .taps()
        .map(|tap| EvalCheckTap {
            group: tap.group(),
            offset: tap.offset(),
            back: tap.back(),
        })
        .collect();
    let reordered;
    let block: &[PolyExtStep] = if WEBGPU_EVAL_CHECK_LAZY_REORDER {
        reordered = eval_check_reorder_lazy(def.block)?;
        &reordered
    } else {
        def.block
    };
    let (last_fp, last_mix, _fp_count, _mix_count) = eval_check_last_uses_block(block, def.ret)?;

    // Per fp var: which bank it lives in and its slot there.
    #[derive(Clone, Copy)]
    struct HybridSlot {
        is_ext: bool,
        slot: usize,
    }
    let mut base_alloc = EvalCheckSlotAllocator::default();
    let mut ext_alloc = EvalCheckSlotAllocator::default();
    let mut mix_alloc = EvalCheckSlotAllocator::default();
    let mut fp_slots: Vec<Option<HybridSlot>> = Vec::new();
    let mut mix_slots: Vec<Option<usize>> = Vec::new();
    let mut instructions = Vec::new();

    let mut push_instr = |words: [u32; WEBGPU_EVAL_CHECK_INSTRUCTION_WORDS]| {
        instructions.extend(words);
    };
    let slot_u32 = |slot: usize| u32::try_from(slot).expect("eval_check slot exceeds u32");
    let fp_slot = |slots: &[Option<HybridSlot>], var: usize| -> Result<HybridSlot> {
        slots
            .get(var)
            .copied()
            .flatten()
            .ok_or_else(|| anyhow!("poly_ext fp var {var} used after free or before def"))
    };

    for (op_idx, op) in block.iter().enumerate() {
        let mut used_fp = Vec::new();
        let mut used_mix = Vec::new();
        match op {
            PolyExtStep::Const(value) => {
                let out_idx = fp_slots.len();
                let out_slot = base_alloc.alloc();
                fp_slots.push(Some(HybridSlot {
                    is_ext: false,
                    slot: out_slot,
                }));
                push_instr([
                    WEBGPU_EVAL_OP_CONST,
                    slot_u32(out_slot),
                    elem_const_word(*value),
                    0,
                    0,
                    0,
                    0,
                    0,
                ]);
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    base_alloc.free(out_slot);
                }
            }
            PolyExtStep::ConstExt(x0, x1, x2, x3) => {
                let out_idx = fp_slots.len();
                let out_slot = ext_alloc.alloc();
                fp_slots.push(Some(HybridSlot {
                    is_ext: true,
                    slot: out_slot,
                }));
                push_instr([
                    WEBGPU_EVAL_OP_CONST_EXT,
                    slot_u32(out_slot),
                    elem_const_word(*x0),
                    elem_const_word(*x1),
                    elem_const_word(*x2),
                    elem_const_word(*x3),
                    0,
                    0,
                ]);
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    ext_alloc.free(out_slot);
                }
            }
            PolyExtStep::Get(tap_idx) => {
                let tap = tap_info
                    .get(*tap_idx)
                    .ok_or_else(|| anyhow!("poly_ext tap index {tap_idx} is out of range"))?;
                ensure!(tap.group < 3, "WebGPU eval_check only supports 3 groups");
                let out_idx = fp_slots.len();
                let out_slot = base_alloc.alloc();
                fp_slots.push(Some(HybridSlot {
                    is_ext: false,
                    slot: out_slot,
                }));
                push_instr([
                    WEBGPU_EVAL_OP_GET,
                    slot_u32(out_slot),
                    u32::try_from(tap.group).expect("eval_check tap group exceeds u32"),
                    u32::try_from(tap.offset).expect("eval_check tap offset exceeds u32"),
                    u32::try_from(tap.back * INV_RATE).expect("eval_check tap back exceeds u32"),
                    0,
                    0,
                    0,
                ]);
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    base_alloc.free(out_slot);
                }
            }
            PolyExtStep::GetGlobal(arg, offset) => {
                ensure!(*arg < 2, "WebGPU eval_check only supports 2 global buffers");
                let out_idx = fp_slots.len();
                let out_slot = base_alloc.alloc();
                fp_slots.push(Some(HybridSlot {
                    is_ext: false,
                    slot: out_slot,
                }));
                push_instr([
                    WEBGPU_EVAL_OP_GET_GLOBAL,
                    slot_u32(out_slot),
                    u32::try_from(*arg).expect("eval_check global arg exceeds u32"),
                    u32::try_from(*offset).expect("eval_check global offset exceeds u32"),
                    0,
                    0,
                    0,
                    0,
                ]);
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    base_alloc.free(out_slot);
                }
            }
            PolyExtStep::Add(lhs, rhs)
            | PolyExtStep::Sub(lhs, rhs)
            | PolyExtStep::Mul(lhs, rhs) => {
                let lhs_slot = fp_slot(&fp_slots, *lhs)?;
                let rhs_slot = fp_slot(&fp_slots, *rhs)?;
                used_fp.extend([*lhs, *rhs]);
                let out_is_ext = lhs_slot.is_ext || rhs_slot.is_ext;
                let out_idx = fp_slots.len();
                let out_slot = if out_is_ext {
                    ext_alloc.alloc()
                } else {
                    base_alloc.alloc()
                };
                fp_slots.push(Some(HybridSlot {
                    is_ext: out_is_ext,
                    slot: out_slot,
                }));
                // Pick the opcode + operand order for the bank pattern.
                // Commutative ops normalize BE -> EB by swapping.
                let (opcode, word_a, word_b) = match (op, lhs_slot.is_ext, rhs_slot.is_ext) {
                    (PolyExtStep::Add(_, _), false, false) => {
                        (WEBGPU_EVAL_OP_ADD, lhs_slot.slot, rhs_slot.slot)
                    }
                    (PolyExtStep::Add(_, _), true, true) => {
                        (WEBGPU_EVAL_OP_ADD_EE, lhs_slot.slot, rhs_slot.slot)
                    }
                    (PolyExtStep::Add(_, _), true, false) => {
                        (WEBGPU_EVAL_OP_ADD_EB, lhs_slot.slot, rhs_slot.slot)
                    }
                    (PolyExtStep::Add(_, _), false, true) => {
                        (WEBGPU_EVAL_OP_ADD_EB, rhs_slot.slot, lhs_slot.slot)
                    }
                    (PolyExtStep::Sub(_, _), false, false) => {
                        (WEBGPU_EVAL_OP_SUB, lhs_slot.slot, rhs_slot.slot)
                    }
                    (PolyExtStep::Sub(_, _), true, true) => {
                        (WEBGPU_EVAL_OP_SUB_EE, lhs_slot.slot, rhs_slot.slot)
                    }
                    (PolyExtStep::Sub(_, _), true, false) => {
                        (WEBGPU_EVAL_OP_SUB_EB, lhs_slot.slot, rhs_slot.slot)
                    }
                    (PolyExtStep::Sub(_, _), false, true) => {
                        (WEBGPU_EVAL_OP_SUB_BE, lhs_slot.slot, rhs_slot.slot)
                    }
                    (PolyExtStep::Mul(_, _), false, false) => {
                        (WEBGPU_EVAL_OP_MUL, lhs_slot.slot, rhs_slot.slot)
                    }
                    (PolyExtStep::Mul(_, _), true, true) => {
                        (WEBGPU_EVAL_OP_MUL_EE, lhs_slot.slot, rhs_slot.slot)
                    }
                    (PolyExtStep::Mul(_, _), true, false) => {
                        (WEBGPU_EVAL_OP_MUL_EB, lhs_slot.slot, rhs_slot.slot)
                    }
                    (PolyExtStep::Mul(_, _), false, true) => {
                        (WEBGPU_EVAL_OP_MUL_EB, rhs_slot.slot, lhs_slot.slot)
                    }
                    _ => unreachable!("match arm covers only Add/Sub/Mul"),
                };
                push_instr([
                    opcode,
                    slot_u32(out_slot),
                    slot_u32(word_a),
                    slot_u32(word_b),
                    0,
                    0,
                    0,
                    0,
                ]);
                if last_fp.get(out_idx).copied().flatten().is_none() {
                    fp_slots[out_idx] = None;
                    if out_is_ext {
                        ext_alloc.free(out_slot);
                    } else {
                        base_alloc.free(out_slot);
                    }
                }
            }
            PolyExtStep::True => {
                let out_idx = mix_slots.len();
                let out_slot = mix_alloc.alloc();
                mix_slots.push(Some(out_slot));
                push_instr([
                    WEBGPU_EVAL_OP_TRUE,
                    slot_u32(out_slot),
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
                let inner_slot = fp_slot(&fp_slots, *inner)?;
                used_mix.push(*chain);
                used_fp.push(*inner);
                let out_idx = mix_slots.len();
                let out_slot = mix_alloc.alloc();
                mix_slots.push(Some(out_slot));
                let opcode = if inner_slot.is_ext {
                    WEBGPU_EVAL_OP_AND_EQZ_EXT
                } else {
                    WEBGPU_EVAL_OP_AND_EQZ
                };
                push_instr([
                    opcode,
                    slot_u32(out_slot),
                    slot_u32(chain_slot),
                    slot_u32(inner_slot.slot),
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
                let cond_slot = fp_slot(&fp_slots, *cond)?;
                let inner_slot = eval_check_mix_slot(&mix_slots, *inner)?;
                used_mix.extend([*chain, *inner]);
                used_fp.push(*cond);
                let out_idx = mix_slots.len();
                let out_slot = mix_alloc.alloc();
                mix_slots.push(Some(out_slot));
                let opcode = if cond_slot.is_ext {
                    WEBGPU_EVAL_OP_AND_COND_EXT
                } else {
                    WEBGPU_EVAL_OP_AND_COND
                };
                push_instr([
                    opcode,
                    slot_u32(out_slot),
                    slot_u32(chain_slot),
                    slot_u32(cond_slot.slot),
                    slot_u32(inner_slot),
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
                let slot = fp_slot(&fp_slots, var)?;
                fp_slots[var] = None;
                if slot.is_ext {
                    ext_alloc.free(slot.slot);
                } else {
                    base_alloc.free(slot.slot);
                }
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
        base_alloc.max_used() <= WEBGPU_EVAL_CHECK_MAX_FP_ELEM_SLOTS,
        "WebGPU hybrid eval_check needs {} base FP slots, max is {}",
        base_alloc.max_used(),
        WEBGPU_EVAL_CHECK_MAX_FP_ELEM_SLOTS
    );
    ensure!(
        ext_alloc.max_used() <= WEBGPU_EVAL_CHECK_MAX_EXT_BANK_SLOTS,
        "WebGPU hybrid eval_check needs {} ext bank slots, max is {}",
        ext_alloc.max_used(),
        WEBGPU_EVAL_CHECK_MAX_EXT_BANK_SLOTS
    );
    ensure!(
        mix_alloc.max_used() <= WEBGPU_EVAL_CHECK_MAX_MIX_SLOTS,
        "WebGPU hybrid eval_check needs {} mix slots, max is {}",
        mix_alloc.max_used(),
        WEBGPU_EVAL_CHECK_MAX_MIX_SLOTS
    );

    Ok((
        instructions,
        base_alloc.max_used(),
        ext_alloc.max_used(),
        mix_alloc.max_used(),
        ret_slot,
    ))
}

pub(crate) const EVAL_CHECK_WGSL_PREFIX: &str = r#"
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

pub(crate) const EVAL_CHECK_INTERPRETER_WGSL: &str = r#"
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

pub(crate) fn build_eval_check_interpreter_wgsl(fp_slots: usize, mix_slots: usize) -> String {
    EVAL_CHECK_INTERPRETER_WGSL
        .replace("{FP_SLOTS}", &fp_slots.max(1).to_string())
        .replace("{MIX_SLOTS}", &mix_slots.max(1).to_string())
}

pub(crate) const EVAL_CHECK_BASE_INTERPRETER_WGSL: &str = r#"
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

fn ext_sub(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32> {
    return vec4<u32>(
        sub(lhs.x, rhs.x),
        sub(lhs.y, rhs.y),
        sub(lhs.z, rhs.z),
        sub(lhs.w, rhs.w),
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

fn promote(value: u32) -> vec4<u32> {
    return vec4<u32>(value, 0u, 0u, 0u);
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

{EXT_BANK_DECLS}

{MIX_BANK_DECLS}

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
                mix_tot_store(lane, out, vec4<u32>(0u, 0u, 0u, 0u));
                mix_mul_store(lane, out, load_mix_pow(instr_word(op_idx, 2u)));
            }
            case 8u: {
                let out = instr_word(op_idx, 1u);
                let chain = instr_word(op_idx, 2u);
                let inner = instr_word(op_idx, 3u);
                mix_tot_store(lane, out, ext_add(
                    mix_tot_load(lane, chain),
                    ext_scale(mix_mul_load(lane, chain), fp[lane][inner]),
                ));
                mix_mul_store(lane, out, load_mix_pow(instr_word(op_idx, 4u)));
            }
            case 9u: {
                let out = instr_word(op_idx, 1u);
                let chain = instr_word(op_idx, 2u);
                let cond = instr_word(op_idx, 3u);
                let inner = instr_word(op_idx, 4u);
                mix_tot_store(lane, out, ext_add(
                    mix_tot_load(lane, chain),
                    ext_scale(
                        ext_mul(mix_tot_load(lane, inner), mix_mul_load(lane, chain)),
                        fp[lane][cond],
                    ),
                ));
                mix_mul_store(lane, out, load_mix_pow(instr_word(op_idx, 5u)));
            }
            case 1u: {
                ext_store(lane, instr_word(op_idx, 1u), vec4<u32>(
                    instr_word(op_idx, 2u),
                    instr_word(op_idx, 3u),
                    instr_word(op_idx, 4u),
                    instr_word(op_idx, 5u),
                ));
            }
            case 10u: {
                ext_store(lane, instr_word(op_idx, 1u), ext_add(
                    ext_load(lane, instr_word(op_idx, 2u)),
                    ext_load(lane, instr_word(op_idx, 3u)),
                ));
            }
            case 11u: {
                ext_store(lane, instr_word(op_idx, 1u), ext_add(
                    ext_load(lane, instr_word(op_idx, 2u)),
                    promote(fp[lane][instr_word(op_idx, 3u)]),
                ));
            }
            case 12u: {
                ext_store(lane, instr_word(op_idx, 1u), ext_sub(
                    ext_load(lane, instr_word(op_idx, 2u)),
                    ext_load(lane, instr_word(op_idx, 3u)),
                ));
            }
            case 13u: {
                ext_store(lane, instr_word(op_idx, 1u), ext_sub(
                    ext_load(lane, instr_word(op_idx, 2u)),
                    promote(fp[lane][instr_word(op_idx, 3u)]),
                ));
            }
            case 14u: {
                ext_store(lane, instr_word(op_idx, 1u), ext_sub(
                    promote(fp[lane][instr_word(op_idx, 2u)]),
                    ext_load(lane, instr_word(op_idx, 3u)),
                ));
            }
            case 15u: {
                ext_store(lane, instr_word(op_idx, 1u), ext_mul(
                    ext_load(lane, instr_word(op_idx, 2u)),
                    ext_load(lane, instr_word(op_idx, 3u)),
                ));
            }
            case 16u: {
                ext_store(lane, instr_word(op_idx, 1u), ext_scale(
                    ext_load(lane, instr_word(op_idx, 2u)),
                    fp[lane][instr_word(op_idx, 3u)],
                ));
            }
            case 17u: {
                let out = instr_word(op_idx, 1u);
                let chain = instr_word(op_idx, 2u);
                let inner = instr_word(op_idx, 3u);
                mix_tot_store(lane, out, ext_add(
                    mix_tot_load(lane, chain),
                    ext_mul(mix_mul_load(lane, chain), ext_load(lane, inner)),
                ));
                mix_mul_store(lane, out, load_mix_pow(instr_word(op_idx, 4u)));
            }
            case 18u: {
                let out = instr_word(op_idx, 1u);
                let chain = instr_word(op_idx, 2u);
                let cond = instr_word(op_idx, 3u);
                let inner = instr_word(op_idx, 4u);
                mix_tot_store(lane, out, ext_add(
                    mix_tot_load(lane, chain),
                    ext_mul(
                        ext_mul(mix_tot_load(lane, inner), mix_mul_load(lane, chain)),
                        ext_load(lane, cond),
                    ),
                ));
                mix_mul_store(lane, out, load_mix_pow(instr_word(op_idx, 5u)));
            }
            default: {}
        }
    }

    let result = ext_scale(mix_tot_load(lane, params.ret_mix_slot), zerofier_inv(cycle));
    check.data[params.check_base + 0u * params.domain + cycle] = result.x;
    check.data[params.check_base + 1u * params.domain + cycle] = result.y;
    check.data[params.check_base + 2u * params.domain + cycle] = result.z;
    check.data[params.check_base + 3u * params.domain + cycle] = result.w;
}
"#;

pub(crate) fn build_eval_check_base_interpreter_wgsl(
    fp_slots: usize,
    ext_slots: usize,
    mix_slots: usize,
    private_parallel: bool,
    workgroup_size: u32,
) -> String {
    let fp_slots = fp_slots.max(1);
    let ext_slots = ext_slots.max(1);
    let mix_slots = mix_slots.max(1);
    let ext_words = ext_slots * 4;
    let mix_words = mix_slots * 4;
    let scratch_lanes = if private_parallel { 1 } else { workgroup_size };
    let scratch_decls = if private_parallel {
        String::new()
    } else {
        format!("var<workgroup> fp: array<array<u32, {fp_slots}>, {scratch_lanes}>;")
    };
    let local_decls = if private_parallel {
        format!("    var fp: array<u32, {fp_slots}>;")
    } else {
        String::new()
    };
    // The ext bank is stored as scalar u32 words (ext value k at
    // fpe[4k..4k+3]) instead of a vec4 array. The probe matrix showed
    // executing vec4-typed accesses against a local-memory scratch array
    // in this kernel costs ~17x a scalar access (~430 ps vs ~25 ps per
    // op-cycle) while the scalar u32 `fp` bank is fast; assembling the
    // vec4 from four scalar loads sidesteps whatever lowering Tint/the
    // driver picks for dynamically-indexed vec4 local arrays. Helpers
    // live at module scope so both scratch modes share the case bodies.
    let ext_bank_decls = if private_parallel {
        format!(
            "var<private> fpe: array<u32, {ext_words}>;\n\
             fn ext_load(lane: u32, idx: u32) -> vec4<u32> {{\n\
                 let base = idx * 4u;\n\
                 return vec4<u32>(fpe[base], fpe[base + 1u], fpe[base + 2u], fpe[base + 3u]);\n\
             }}\n\
             fn ext_store(lane: u32, idx: u32, value: vec4<u32>) {{\n\
                 let base = idx * 4u;\n\
                 fpe[base] = value.x;\n\
                 fpe[base + 1u] = value.y;\n\
                 fpe[base + 2u] = value.z;\n\
                 fpe[base + 3u] = value.w;\n\
             }}"
        )
    } else {
        format!(
            "var<workgroup> fpe: array<array<u32, {ext_words}>, {scratch_lanes}>;\n\
             fn ext_load(lane: u32, idx: u32) -> vec4<u32> {{\n\
                 let base = idx * 4u;\n\
                 return vec4<u32>(fpe[lane][base], fpe[lane][base + 1u], fpe[lane][base + 2u], fpe[lane][base + 3u]);\n\
             }}\n\
             fn ext_store(lane: u32, idx: u32, value: vec4<u32>) {{\n\
                 let base = idx * 4u;\n\
                 fpe[lane][base] = value.x;\n\
                 fpe[lane][base + 1u] = value.y;\n\
                 fpe[lane][base + 2u] = value.z;\n\
                 fpe[lane][base + 3u] = value.w;\n\
             }}"
        )
    };
    // The mix accumulators are stored as scalar u32 words (mix value
    // k at words 4k..4k+3), exactly like the interpreter's ext bank. vec4-typed
    // dynamically-indexed local arrays are the pathological case under
    // Chrome/Dawn (~17x a scalar access once past the register-select
    // size threshold); rv32im runs with mix_slots=29, right at that
    // boundary. Helpers live at module scope so both scratch modes share
    // the case bodies.
    let mix_bank_decls = if private_parallel {
        format!(
            "var<private> mixw_tot: array<u32, {mix_words}>;\n\
             var<private> mixw_mul: array<u32, {mix_words}>;\n\
             fn mix_tot_load(lane: u32, idx: u32) -> vec4<u32> {{\n\
                 let base = idx * 4u;\n\
                 return vec4<u32>(mixw_tot[base], mixw_tot[base + 1u], mixw_tot[base + 2u], mixw_tot[base + 3u]);\n\
             }}\n\
             fn mix_tot_store(lane: u32, idx: u32, value: vec4<u32>) {{\n\
                 let base = idx * 4u;\n\
                 mixw_tot[base] = value.x;\n\
                 mixw_tot[base + 1u] = value.y;\n\
                 mixw_tot[base + 2u] = value.z;\n\
                 mixw_tot[base + 3u] = value.w;\n\
             }}\n\
             fn mix_mul_load(lane: u32, idx: u32) -> vec4<u32> {{\n\
                 let base = idx * 4u;\n\
                 return vec4<u32>(mixw_mul[base], mixw_mul[base + 1u], mixw_mul[base + 2u], mixw_mul[base + 3u]);\n\
             }}\n\
             fn mix_mul_store(lane: u32, idx: u32, value: vec4<u32>) {{\n\
                 let base = idx * 4u;\n\
                 mixw_mul[base] = value.x;\n\
                 mixw_mul[base + 1u] = value.y;\n\
                 mixw_mul[base + 2u] = value.z;\n\
                 mixw_mul[base + 3u] = value.w;\n\
             }}"
        )
    } else {
        format!(
            "var<workgroup> mixw_tot: array<array<u32, {mix_words}>, {scratch_lanes}>;\n\
             var<workgroup> mixw_mul: array<array<u32, {mix_words}>, {scratch_lanes}>;\n\
             fn mix_tot_load(lane: u32, idx: u32) -> vec4<u32> {{\n\
                 let base = idx * 4u;\n\
                 return vec4<u32>(mixw_tot[lane][base], mixw_tot[lane][base + 1u], mixw_tot[lane][base + 2u], mixw_tot[lane][base + 3u]);\n\
             }}\n\
             fn mix_tot_store(lane: u32, idx: u32, value: vec4<u32>) {{\n\
                 let base = idx * 4u;\n\
                 mixw_tot[lane][base] = value.x;\n\
                 mixw_tot[lane][base + 1u] = value.y;\n\
                 mixw_tot[lane][base + 2u] = value.z;\n\
                 mixw_tot[lane][base + 3u] = value.w;\n\
             }}\n\
             fn mix_mul_load(lane: u32, idx: u32) -> vec4<u32> {{\n\
                 let base = idx * 4u;\n\
                 return vec4<u32>(mixw_mul[lane][base], mixw_mul[lane][base + 1u], mixw_mul[lane][base + 2u], mixw_mul[lane][base + 3u]);\n\
             }}\n\
             fn mix_mul_store(lane: u32, idx: u32, value: vec4<u32>) {{\n\
                 let base = idx * 4u;\n\
                 mixw_mul[lane][base] = value.x;\n\
                 mixw_mul[lane][base + 1u] = value.y;\n\
                 mixw_mul[lane][base + 2u] = value.z;\n\
                 mixw_mul[lane][base + 3u] = value.w;\n\
             }}"
        )
    };
    let lane_index = if private_parallel { "0u" } else { "lid.x" };
    let linear_dispatch_stride = workgroup_size * WEBGPU_MAX_WORKGROUPS_PER_DIMENSION;

    let wgsl = EVAL_CHECK_BASE_INTERPRETER_WGSL
        .replace("{FP_SLOTS}", &fp_slots.to_string())
        .replace("{MIX_SLOTS}", &mix_slots.to_string())
        .replace("{BASE_SCRATCH_DECLS}", &scratch_decls)
        .replace("{BASE_LOCAL_DECLS}", &local_decls)
        .replace("{EXT_BANK_DECLS}", &ext_bank_decls)
        .replace("{MIX_BANK_DECLS}", &mix_bank_decls)
        .replace("{LANE_INDEX}", lane_index)
        .replace("{WORKGROUP_SIZE}", &workgroup_size.to_string())
        .replace(
            "{LINEAR_DISPATCH_STRIDE}",
            &linear_dispatch_stride.to_string(),
        );
    if private_parallel {
        // Private mode uses per-invocation scratch, so the per-lane outer
        // array dimension is pure overhead — flatten the remaining
        // scratch accesses to match the all-ext interpreter's shape.
        wgsl.replace("fp[lane]", "fp")
    } else {
        wgsl
    }
}

#[allow(dead_code)]
pub(crate) fn build_eval_check_wgsl_unrolled(
    taps: &TapSet<'_>,
    def: &PolyExtStepDef,
) -> Result<String> {
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

pub(crate) fn build_eval_check_wgsl(taps: &TapSet<'_>, def: &PolyExtStepDef) -> Result<String> {
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

pub(crate) fn build_eval_check_split_wgsl(
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
