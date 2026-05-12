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

//! Circuit-specific staged WGSL `eval_check` codegen substrate.
//!
//! Walks a `PolyExtStepDef` and emits straight-line WGSL whose semantics
//! match `risc0_zkp::adapter::PolyExtExecutor` and the runtime interpreter
//! shader in `webgpu.rs` (`EVAL_CHECK_BASE_INTERPRETER_WGSL` /
//! `EVAL_CHECK_EXT_INTERPRETER_WGSL`). Two modes mirror the interpreter's
//! base-field vs extension-field split:
//!
//! * [`FieldMode::Base`] — `fp` slots are `u32` BabyBear scalars; `Add`/`Sub`/
//!   `Mul` use the scalar `add`/`sub`/`mul` helpers; `AndEqz` and `AndCond`
//!   use `ext_scale` to combine the scalar `fp[inner]`/`fp[cond]` with the
//!   `vec4<u32>` mix state. `ConstExt` is **rejected** because the slot is
//!   not wide enough to hold a `vec4<u32>` extension constant. This mode is
//!   what rv32im uses (no extension intermediates in its DEF).
//! * [`FieldMode::Ext`] — `fp` slots are `vec4<u32>` extension elements;
//!   every arithmetic op uses the `ext_*` helpers. `ConstExt` is supported.
//!   This is required for circuits whose DEF uses `ConstExt` or carries
//!   extension intermediates (e.g., recursion).
//!
//! Scope per `02-to-be-plan.md`:
//! - SP2 (seed): module exists, RED-marked parity test, no real emission.
//! - SP3 iter 1: real generic emitter (vec4 only) + 9 structural tests.
//! - SP3 iter 2 (this iteration): split into `Base` / `Ext` field modes
//!   matching the runtime interpreter; `rv32im_codegen_for_po2` defaults to
//!   base-field. Not yet wired into the prove path's fast-path.
//! - SP3 follow-on: full prelude (bindings, params, helpers) emitted as
//!   part of the kernel; dispatch wiring with staged-then-interpreter
//!   fallback; runtime parity test under `examples/browser-prove/src/lib.rs`;
//!   multi-stage split for the production rv32im DEF (~20k ops).

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::Write as _;

use anyhow::{anyhow, ensure, Result};

use crate::adapter::{PolyExtStep, PolyExtStepDef};

// ============================================================================
// Slot allocation primitives shared with the runtime interpreter in
// `risc0/zkp/src/hal/webgpu.rs`. Kept in this module (not in `webgpu.rs`)
// because `webgpu.rs` is wasm32-only while this module is feature-gated
// only by `webgpu` — the slot-allocation logic itself is pure Rust and is
// reused by the staged-WGSL emitter below to keep `fp_slots` / `mix_slots`
// bounded by the live set rather than `block.len()`.
// ============================================================================

/// Pool allocator that hands out the lowest free slot index, mirroring the
/// register-renaming pattern that `eval_check_interpreter_instructions_with_limit`
/// uses to keep the runtime interpreter's `fp[]` / `mix_tot[]` / `mix_mul[]`
/// arrays bounded by the live working set rather than `block.len()`.
#[derive(Default)]
pub(crate) struct EvalCheckSlotAllocator {
    free: Vec<usize>,
    next: usize,
    max: usize,
}

impl EvalCheckSlotAllocator {
    pub(crate) fn alloc(&mut self) -> usize {
        let slot = self.free.pop().unwrap_or_else(|| {
            let slot = self.next;
            self.next += 1;
            self.max = self.max.max(self.next);
            slot
        });
        self.max = self.max.max(slot + 1);
        slot
    }

    pub(crate) fn free(&mut self, slot: usize) {
        self.free.push(slot);
    }

    /// High-water mark of slot indices ever allocated. The kernel sizes
    /// its `fp` / `mix_tot` / `mix_mul` local arrays to this count.
    pub(crate) fn max_used(&self) -> usize {
        self.max
    }
}

pub(crate) fn eval_check_note_last(
    last_uses: &mut Vec<Option<usize>>,
    var: usize,
    op_idx: usize,
) {
    if var >= last_uses.len() {
        last_uses.resize(var + 1, None);
    }
    last_uses[var] = Some(op_idx);
}

/// Per-var last-use indices for both fp and mix vars, matching the
/// runtime interpreter's allocation discipline byte-for-byte. The ret
/// mix var's last use is set to `usize::MAX` so its slot stays live
/// through the entire program (the kernel writes it to the check buffer
/// at the end).
pub(crate) fn eval_check_last_uses(
    def: &PolyExtStepDef,
) -> Result<(Vec<Option<usize>>, Vec<Option<usize>>, usize, usize)> {
    let mut last_fp = Vec::new();
    let mut last_mix = Vec::new();
    let mut fp_count = 0usize;
    let mut mix_count = 0usize;

    for (op_idx, op) in def.block.iter().enumerate() {
        match op {
            PolyExtStep::Const(_)
            | PolyExtStep::ConstExt(_, _, _, _)
            | PolyExtStep::Get(_)
            | PolyExtStep::GetGlobal(_, _) => {
                fp_count += 1;
                last_fp.resize(last_fp.len().max(fp_count), None);
            }
            PolyExtStep::Add(lhs, rhs)
            | PolyExtStep::Sub(lhs, rhs)
            | PolyExtStep::Mul(lhs, rhs) => {
                eval_check_note_last(&mut last_fp, *lhs, op_idx);
                eval_check_note_last(&mut last_fp, *rhs, op_idx);
                fp_count += 1;
                last_fp.resize(last_fp.len().max(fp_count), None);
            }
            PolyExtStep::True => {
                mix_count += 1;
                last_mix.resize(last_mix.len().max(mix_count), None);
            }
            PolyExtStep::AndEqz(chain, inner) => {
                eval_check_note_last(&mut last_mix, *chain, op_idx);
                eval_check_note_last(&mut last_fp, *inner, op_idx);
                mix_count += 1;
                last_mix.resize(last_mix.len().max(mix_count), None);
            }
            PolyExtStep::AndCond(chain, cond, inner) => {
                eval_check_note_last(&mut last_mix, *chain, op_idx);
                eval_check_note_last(&mut last_fp, *cond, op_idx);
                eval_check_note_last(&mut last_mix, *inner, op_idx);
                mix_count += 1;
                last_mix.resize(last_mix.len().max(mix_count), None);
            }
        }
    }

    ensure!(
        def.ret < mix_count,
        "poly_ext return mix index {} exceeds generated mix count {}",
        def.ret,
        mix_count
    );
    last_mix[def.ret] = Some(usize::MAX);
    Ok((last_fp, last_mix, fp_count, mix_count))
}

pub(crate) fn eval_check_fp_slot(slots: &[Option<usize>], var: usize) -> Result<usize> {
    slots
        .get(var)
        .and_then(|slot| *slot)
        .ok_or_else(|| anyhow!("poly_ext fp var {var} is not live"))
}

pub(crate) fn eval_check_mix_slot(slots: &[Option<usize>], var: usize) -> Result<usize> {
    slots
        .get(var)
        .and_then(|slot| *slot)
        .ok_or_else(|| anyhow!("poly_ext mix var {var} is not live"))
}

/// Field-type discipline that the emitted kernel will use for `fp` slots.
/// Mirrors the runtime interpreter's `base_field` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldMode {
    /// Base-field optimization: `fp[n]` is a `u32` BabyBear scalar. Required
    /// path for circuits whose DEF has no `ConstExt` step.
    Base,
    /// Extension-field: `fp[n]` is a `vec4<u32>` BabyBearExt element.
    Ext,
}

impl FieldMode {
    fn fp_ty(self) -> &'static str {
        match self {
            FieldMode::Base => "u32",
            FieldMode::Ext => "vec4<u32>",
        }
    }
}

/// Description of one staged WGSL kernel produced by a circuit-specific
/// generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedKernel {
    /// Human-readable identifier for the kernel within a circuit's staged set.
    pub id: String,
    /// Field discipline used by the body's `fp` slots.
    pub field_mode: FieldMode,
    /// Number of FP locals the body uses.
    pub fp_slots: usize,
    /// Number of MixState locals (`mix_tot` + `mix_mul`) the body uses.
    pub mix_slots: usize,
    /// The WGSL source code for this kernel's *body*. The runtime prelude
    /// (bindings, params, arithmetic helpers, `read_tap` / `read_global` /
    /// `load_mix_pow` / `write_check`) is provided by `webgpu.rs` and
    /// concatenated by the dispatch wiring (SP3 follow-on).
    pub wgsl_source: String,
}

/// Errors that prevent generating a staged kernel for a DEF under a given
/// `FieldMode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    /// The DEF uses `PolyExtStep::ConstExt` but the requested mode is
    /// `FieldMode::Base`, where `fp` slots are scalar `u32`s and cannot
    /// hold a `vec4<u32>` constant.
    ConstExtInBaseField,
    /// The DEF references tap index `tap_idx` via `PolyExtStep::Get`, but
    /// the caller's tap table has fewer entries.
    TapIndexOutOfRange { tap_idx: usize, taps_len: usize },
    /// The DEF references tap with `group > 2`, but only 3 group bindings
    /// (`group0`/`group1`/`group2`) are wired by the prelude.
    TapGroupOutOfRange { tap_idx: usize, group: u32 },
    /// `eval_check_last_uses` rejected the DEF (e.g., `ret` mix index
    /// out of range). The string is the underlying anyhow error.
    LastUseAnalysisFailed(String),
    /// The DEF references an fp or mix var whose slot was already freed
    /// (the prior consumer hit the var's last-use and reclaimed it). A
    /// well-formed DEF from `risc0_zkp::adapter` won't produce this.
    VarNotLive(String),
}

/// Per-tap addressing info that the emitter inlines at the `Get(tap_idx)`
/// site. Decouples the codegen module from `risc0_zkp::taps::TapSet` so
/// unit tests can supply hand-rolled tap tables without depending on a
/// real circuit's TapSet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmitterTap {
    /// Register-group index: 0, 1, or 2 (matches the prelude's
    /// `group0`/`group1`/`group2` bindings).
    pub group: u32,
    /// Column offset within the group's flat layout.
    pub offset: u32,
    /// `tap.back() * INV_RATE` — the row-back distance in domain steps.
    /// Pre-multiplied by `INV_RATE` to match what the runtime interpreter
    /// stores in its opcode word.
    pub back_inv_rate: u32,
}

/// Compute `(fp_expected, mix_expected)` for a DEF without depending on the
/// private accessors in `crate::adapter`. Mirrors `PolyExtStepDef::fp_expected`
/// and `PolyExtStepDef::mix_expected` exactly.
fn def_var_counts(def: &PolyExtStepDef) -> (usize, usize) {
    let mix_expected = def.ret + 1;
    let fp_expected = def.block.len() - mix_expected;
    (fp_expected, mix_expected)
}

/// Mirror of `PolyExtStepDef::mix_exponents`: per-mix-var, the exponent of
/// `mix` that the executor multiplies into the running chain. The emitter
/// uses this to index into the `mix_pows` storage buffer at runtime.
fn def_mix_exponents(def: &PolyExtStepDef) -> Vec<usize> {
    let mut exps: Vec<usize> = Vec::with_capacity(def.ret + 1);
    for op in def.block {
        match op {
            PolyExtStep::True => exps.push(0),
            PolyExtStep::AndEqz(chain, _) => exps.push(exps[*chain] + 1),
            PolyExtStep::AndCond(chain, _, inner) => {
                exps.push(exps[*chain] + exps[*inner]);
            }
            _ => {}
        }
    }
    exps
}

/// True if the DEF can be lowered under `FieldMode::Base` (no `ConstExt`).
pub fn def_is_base_field(def: &PolyExtStepDef) -> bool {
    !def.block
        .iter()
        .any(|op| matches!(op, PolyExtStep::ConstExt(..)))
}

/// Internal state for emitting a single staged WGSL kernel from a DEF.
///
/// iter 6 (slot allocation):
/// - `fp_alloc` / `mix_alloc` hand out lowest free slot indices, mirroring
///   the runtime interpreter's allocation discipline.
/// - `fp_slot_map[var_idx]` records which physical slot was assigned to
///   the `var_idx`-th fp var (in poly_ext var-index order, i.e., the
///   order in which `Const`/`Get`/`Add`/etc. ops push fp vars). `None`
///   means the slot was freed after its last use.
/// - `mix_slot_map[var_idx]` is the parallel structure for `True` /
///   `AndEqz` / `AndCond` mix vars.
/// - `last_fp` / `last_mix` are the last-use op indices computed once at
///   emitter construction; a slot is freed when the op at `last_fp[var]`
///   (resp. `last_mix[var]`) finishes consuming it. The `ret` mix var's
///   last-use is `usize::MAX` so its slot stays live through the program.
/// - `fp_var_count` / `mix_var_count` track how many vars have been
///   emitted so far (poly_ext var indexing).
struct WgslEmitter<'a> {
    body: String,
    field_mode: FieldMode,
    fp_alloc: EvalCheckSlotAllocator,
    mix_alloc: EvalCheckSlotAllocator,
    fp_slot_map: Vec<Option<usize>>,
    mix_slot_map: Vec<Option<usize>>,
    last_fp: Vec<Option<usize>>,
    last_mix: Vec<Option<usize>>,
    fp_var_count: usize,
    mix_var_count: usize,
    mix_exps: Vec<usize>,
    taps: &'a [EmitterTap],
}

impl<'a> WgslEmitter<'a> {
    fn new(
        def: &PolyExtStepDef,
        taps: &'a [EmitterTap],
        field_mode: FieldMode,
    ) -> Result<Self, CodegenError> {
        let (last_fp, last_mix, _fp_count, _mix_count) = eval_check_last_uses(def)
            .map_err(|err| CodegenError::LastUseAnalysisFailed(err.to_string()))?;
        Ok(Self {
            body: String::new(),
            field_mode,
            fp_alloc: EvalCheckSlotAllocator::default(),
            mix_alloc: EvalCheckSlotAllocator::default(),
            fp_slot_map: Vec::new(),
            mix_slot_map: Vec::new(),
            last_fp,
            last_mix,
            fp_var_count: 0,
            mix_var_count: 0,
            mix_exps: def_mix_exponents(def),
            taps,
        })
    }

    /// Look up the physical slot a still-live fp var was assigned to.
    fn fp_slot_for(&self, var: usize) -> Result<usize, CodegenError> {
        eval_check_fp_slot(&self.fp_slot_map, var)
            .map_err(|err| CodegenError::VarNotLive(err.to_string()))
    }

    /// Look up the physical slot a still-live mix var was assigned to.
    fn mix_slot_for(&self, var: usize) -> Result<usize, CodegenError> {
        eval_check_mix_slot(&self.mix_slot_map, var)
            .map_err(|err| CodegenError::VarNotLive(err.to_string()))
    }

    /// Allocate a fresh fp slot for the next fp var. Returns
    /// `(var_idx, slot)`. If the var has no future use beyond this op
    /// (e.g., the program never reads from it), the slot is immediately
    /// freed and `fp_slot_map[var_idx]` is set to `None`.
    fn alloc_fp(&mut self, current_op_idx: usize) -> (usize, usize) {
        let var_idx = self.fp_var_count;
        self.fp_var_count += 1;
        let slot = self.fp_alloc.alloc();
        self.fp_slot_map.push(Some(slot));
        // If this var is never used downstream (its last_fp entry is
        // missing or its last use is the producer itself), immediately
        // free the slot — the kernel still writes to it for side-effect
        // parity with the interpreter, but the slot is reusable next op.
        let next_use = self.last_fp.get(var_idx).copied().flatten();
        if next_use.is_none() {
            self.fp_slot_map[var_idx] = None;
            self.fp_alloc.free(slot);
        }
        let _ = current_op_idx; // reserved for future ranges-based scheduling
        (var_idx, slot)
    }

    fn alloc_mix(&mut self, current_op_idx: usize) -> (usize, usize) {
        let var_idx = self.mix_var_count;
        self.mix_var_count += 1;
        let slot = self.mix_alloc.alloc();
        self.mix_slot_map.push(Some(slot));
        let next_use = self.last_mix.get(var_idx).copied().flatten();
        if next_use.is_none() {
            self.mix_slot_map[var_idx] = None;
            self.mix_alloc.free(slot);
        }
        let _ = current_op_idx;
        (var_idx, slot)
    }

    /// After emitting the op at `op_idx`, free any operand slots whose
    /// last-use was this op. Mirrors `eval_check_interpreter_instructions_with_limit`'s
    /// post-emit "free dead operands" pass.
    fn free_dead_fp_operands(&mut self, op_idx: usize, vars: &[usize]) {
        for &var in vars {
            if let Some(Some(last)) = self.last_fp.get(var) {
                if *last == op_idx {
                    if let Some(slot) = self.fp_slot_map[var].take() {
                        self.fp_alloc.free(slot);
                    }
                }
            }
        }
    }

    fn free_dead_mix_operands(&mut self, op_idx: usize, vars: &[usize]) {
        for &var in vars {
            if let Some(Some(last)) = self.last_mix.get(var) {
                if *last == op_idx {
                    if let Some(slot) = self.mix_slot_map[var].take() {
                        self.mix_alloc.free(slot);
                    }
                }
            }
        }
    }

    fn resolve_tap(&self, tap_idx: usize) -> Result<EmitterTap, CodegenError> {
        let tap = self
            .taps
            .get(tap_idx)
            .copied()
            .ok_or(CodegenError::TapIndexOutOfRange {
                tap_idx,
                taps_len: self.taps.len(),
            })?;
        if tap.group > 2 {
            return Err(CodegenError::TapGroupOutOfRange {
                tap_idx,
                group: tap.group,
            });
        }
        Ok(tap)
    }

    fn emit(&mut self, idx: usize, op: &PolyExtStep) -> Result<(), CodegenError> {
        match (op, self.field_mode) {
            (PolyExtStep::Const(v), FieldMode::Base) => {
                let (var, slot) = self.alloc_fp(idx);
                writeln!(self.body, "  // [{idx}] fp_var{var} (slot {slot}) = Const({v})").unwrap();
                writeln!(self.body, "  fp[{slot}] = {v}u;").unwrap();
            }
            (PolyExtStep::Const(v), FieldMode::Ext) => {
                let (var, slot) = self.alloc_fp(idx);
                writeln!(self.body, "  // [{idx}] fp_var{var} (slot {slot}) = Const({v})").unwrap();
                writeln!(self.body, "  fp[{slot}] = vec4<u32>({v}u, 0u, 0u, 0u);").unwrap();
            }
            (PolyExtStep::ConstExt(_, _, _, _), FieldMode::Base) => {
                return Err(CodegenError::ConstExtInBaseField);
            }
            (PolyExtStep::ConstExt(a, b, c, d), FieldMode::Ext) => {
                let (var, slot) = self.alloc_fp(idx);
                writeln!(
                    self.body,
                    "  // [{idx}] fp_var{var} (slot {slot}) = ConstExt({a}, {b}, {c}, {d})"
                )
                .unwrap();
                writeln!(self.body, "  fp[{slot}] = vec4<u32>({a}u, {b}u, {c}u, {d}u);").unwrap();
            }
            (PolyExtStep::Get(tap_idx), mode) => {
                let tap = self.resolve_tap(*tap_idx)?;
                let (var, slot) = self.alloc_fp(idx);
                writeln!(
                    self.body,
                    "  // [{idx}] fp_var{var} (slot {slot}) = Get(tap={tap_idx}) -> g{}_offset={}_back={}",
                    tap.group, tap.offset, tap.back_inv_rate
                )
                .unwrap();
                let suffix = match mode {
                    FieldMode::Base => "scalar",
                    FieldMode::Ext => "ext",
                };
                writeln!(
                    self.body,
                    "  fp[{slot}] = read_g{}_{suffix}({}u, {}u, cycle);",
                    tap.group, tap.offset, tap.back_inv_rate
                )
                .unwrap();
            }
            (PolyExtStep::GetGlobal(arg, off), mode) => {
                let (var, slot) = self.alloc_fp(idx);
                writeln!(
                    self.body,
                    "  // [{idx}] fp_var{var} (slot {slot}) = GetGlobal(arg={arg}, off={off})"
                )
                .unwrap();
                let suffix = match mode {
                    FieldMode::Base => "scalar",
                    FieldMode::Ext => "ext",
                };
                writeln!(
                    self.body,
                    "  fp[{slot}] = read_global_{suffix}({arg}u, {off}u);"
                )
                .unwrap();
            }
            (PolyExtStep::Add(x, y), mode) => {
                let x_slot = self.fp_slot_for(*x)?;
                let y_slot = self.fp_slot_for(*y)?;
                let (var, slot) = self.alloc_fp(idx);
                let helper = match mode {
                    FieldMode::Base => "add",
                    FieldMode::Ext => "ext_add",
                };
                writeln!(
                    self.body,
                    "  // [{idx}] fp_var{var} (slot {slot}) = Add(fp_var{x}/slot{x_slot}, fp_var{y}/slot{y_slot})"
                )
                .unwrap();
                writeln!(
                    self.body,
                    "  fp[{slot}] = {helper}(fp[{x_slot}], fp[{y_slot}]);"
                )
                .unwrap();
                self.free_dead_fp_operands(idx, &[*x, *y]);
            }
            (PolyExtStep::Sub(x, y), mode) => {
                let x_slot = self.fp_slot_for(*x)?;
                let y_slot = self.fp_slot_for(*y)?;
                let (var, slot) = self.alloc_fp(idx);
                let helper = match mode {
                    FieldMode::Base => "sub",
                    FieldMode::Ext => "ext_sub",
                };
                writeln!(
                    self.body,
                    "  // [{idx}] fp_var{var} (slot {slot}) = Sub(fp_var{x}/slot{x_slot}, fp_var{y}/slot{y_slot})"
                )
                .unwrap();
                writeln!(
                    self.body,
                    "  fp[{slot}] = {helper}(fp[{x_slot}], fp[{y_slot}]);"
                )
                .unwrap();
                self.free_dead_fp_operands(idx, &[*x, *y]);
            }
            (PolyExtStep::Mul(x, y), mode) => {
                let x_slot = self.fp_slot_for(*x)?;
                let y_slot = self.fp_slot_for(*y)?;
                let (var, slot) = self.alloc_fp(idx);
                let helper = match mode {
                    FieldMode::Base => "mul",
                    FieldMode::Ext => "ext_mul",
                };
                writeln!(
                    self.body,
                    "  // [{idx}] fp_var{var} (slot {slot}) = Mul(fp_var{x}/slot{x_slot}, fp_var{y}/slot{y_slot})"
                )
                .unwrap();
                writeln!(
                    self.body,
                    "  fp[{slot}] = {helper}(fp[{x_slot}], fp[{y_slot}]);"
                )
                .unwrap();
                self.free_dead_fp_operands(idx, &[*x, *y]);
            }
            (PolyExtStep::True, _) => {
                let (var, slot) = self.alloc_mix(idx);
                let exp = self.mix_exps[var];
                writeln!(
                    self.body,
                    "  // [{idx}] mix_var{var} (slot {slot}) = True (mix_pow exp={exp})"
                )
                .unwrap();
                writeln!(
                    self.body,
                    "  mix_tot[{slot}] = vec4<u32>(0u, 0u, 0u, 0u); mix_mul[{slot}] = load_mix_pow({exp}u);"
                )
                .unwrap();
            }
            (PolyExtStep::AndEqz(chain, inner), mode) => {
                let chain_slot = self.mix_slot_for(*chain)?;
                let inner_slot = self.fp_slot_for(*inner)?;
                let (var, slot) = self.alloc_mix(idx);
                let exp = self.mix_exps[var];
                let inner_combine = match mode {
                    // Base mode: scalar fp[inner] combined via ext_scale.
                    FieldMode::Base => format!("ext_scale(mix_mul[{chain_slot}], fp[{inner_slot}])"),
                    // Ext mode: vec4 fp[inner] combined via full ext_mul.
                    FieldMode::Ext => format!("ext_mul(mix_mul[{chain_slot}], fp[{inner_slot}])"),
                };
                writeln!(
                    self.body,
                    "  // [{idx}] mix_var{var} (slot {slot}) = AndEqz(chain=mix_var{chain}/slot{chain_slot}, inner=fp_var{inner}/slot{inner_slot})"
                )
                .unwrap();
                writeln!(
                    self.body,
                    "  mix_tot[{slot}] = ext_add(mix_tot[{chain_slot}], {inner_combine}); mix_mul[{slot}] = load_mix_pow({exp}u);"
                )
                .unwrap();
                self.free_dead_mix_operands(idx, &[*chain]);
                self.free_dead_fp_operands(idx, &[*inner]);
            }
            (PolyExtStep::AndCond(chain, cond, inner), mode) => {
                let chain_slot = self.mix_slot_for(*chain)?;
                let cond_slot = self.fp_slot_for(*cond)?;
                let inner_slot = self.mix_slot_for(*inner)?;
                let (var, slot) = self.alloc_mix(idx);
                let exp = self.mix_exps[var];
                let cond_combine = match mode {
                    FieldMode::Base => format!(
                        "ext_scale(ext_mul(mix_tot[{inner_slot}], mix_mul[{chain_slot}]), fp[{cond_slot}])"
                    ),
                    FieldMode::Ext => format!(
                        "ext_mul(ext_mul(mix_tot[{inner_slot}], mix_mul[{chain_slot}]), fp[{cond_slot}])"
                    ),
                };
                writeln!(
                    self.body,
                    "  // [{idx}] mix_var{var} (slot {slot}) = AndCond(chain=mix_var{chain}/slot{chain_slot}, cond=fp_var{cond}/slot{cond_slot}, inner=mix_var{inner}/slot{inner_slot})"
                )
                .unwrap();
                writeln!(
                    self.body,
                    "  mix_tot[{slot}] = ext_add(mix_tot[{chain_slot}], {cond_combine}); mix_mul[{slot}] = load_mix_pow({exp}u);"
                )
                .unwrap();
                self.free_dead_mix_operands(idx, &[*chain, *inner]);
                self.free_dead_fp_operands(idx, &[*cond]);
            }
        }
        Ok(())
    }

    /// Materialize the kernel. `ret_slot` is the slot the ret mix var
    /// was assigned (recorded by `alloc_mix` and persisted via
    /// `last_mix[ret] = usize::MAX` so it never frees).
    fn finalize(mut self, name: &str, ret_slot: usize) -> StagedKernel {
        let fp_slots = self.fp_alloc.max_used().max(1);
        let mix_slots = self.mix_alloc.max_used().max(1);
        let mut wgsl = String::new();
        writeln!(wgsl, "// staged eval_check kernel: {name}").unwrap();
        writeln!(
            wgsl,
            "// field_mode = {:?}, fp_slots = {fp_slots}, mix_slots = {mix_slots}, ret_mix_slot = {ret_slot}",
            self.field_mode
        )
        .unwrap();
        wgsl.push_str(
            "// runtime prelude (bindings, params, ext_*/add/sub/mul helpers,\n// read_g{0,1,2}_*, read_global_*, load_mix_pow, write_check) is the\n// `STAGED_EVAL_CHECK_PRELUDE_WGSL` constant; this body is appended after it.\n",
        );
        // SP3 iter 6: workgroup_size = 1. With slot allocation, each
        // thread's private state is small (fp_slots * 4 B + 2 * mix_slots
        // * 16 B), but the per-PolyExtStep straight-line code is long
        // (~20k ops for rv32im). Running each cycle on its own workgroup
        // lets the GPU schedule thousands of independent thread blocks
        // without the per-workgroup register-file pressure that a
        // larger workgroup size would impose. Matches the runtime
        // interpreter's `private_parallel = true` discipline.
        writeln!(wgsl, "@compute @workgroup_size(1)").unwrap();
        wgsl.push_str("fn main(@builtin(global_invocation_id) gid: vec3<u32>) {\n");
        wgsl.push_str("  let cycle = gid.x;\n");
        wgsl.push_str("  if (cycle >= params.domain) { return; }\n");
        writeln!(
            wgsl,
            "  var fp: array<{}, {fp_slots}>;",
            self.field_mode.fp_ty()
        )
        .unwrap();
        writeln!(wgsl, "  var mix_tot: array<vec4<u32>, {mix_slots}>;").unwrap();
        writeln!(wgsl, "  var mix_mul: array<vec4<u32>, {mix_slots}>;").unwrap();
        wgsl.push_str(&self.body);
        writeln!(wgsl, "  write_check(cycle, mix_tot[{ret_slot}]);").unwrap();
        wgsl.push_str("}\n");
        self.body.clear();
        StagedKernel {
            id: name.to_string(),
            field_mode: self.field_mode,
            fp_slots,
            mix_slots,
            wgsl_source: wgsl,
        }
    }
}

/// Emit a staged WGSL kernel for an arbitrary `PolyExtStepDef` in the
/// specified `field_mode`. The `taps` slice resolves every
/// `PolyExtStep::Get(tap_idx)` to a concrete `(group, offset, back)`
/// triple that the emitter inlines at the Get site. Tests can pass a
/// hand-rolled slice; production wiring extracts it from
/// `risc0_zkp::taps::TapSet` at dispatch time.
///
/// The result's `wgsl_source` is the kernel body; the runtime prelude
/// (bindings, params, arithmetic helpers, per-group readers) is
/// concatenated by `staged_full_kernel_wgsl` (or by the dispatch wiring
/// when SP3 iter 5 lands).
///
/// Returns `CodegenError::ConstExtInBaseField` if the DEF needs ext
/// constants under Base mode, or `CodegenError::TapIndexOutOfRange` /
/// `TapGroupOutOfRange` if any `Get` resolves outside the supplied table
/// or the 3 supported group bindings.
pub fn staged_kernel_from_def_with_mode(
    name: &str,
    def: &PolyExtStepDef,
    taps: &[EmitterTap],
    field_mode: FieldMode,
) -> Result<StagedKernel, CodegenError> {
    let (fp_expected, mix_expected) = def_var_counts(def);
    let mut emitter = WgslEmitter::new(def, taps, field_mode)?;
    for (idx, op) in def.block.iter().enumerate() {
        emitter.emit(idx, op)?;
    }
    debug_assert_eq!(
        emitter.fp_var_count, fp_expected,
        "emitted fp var count must equal PolyExtStepDef::fp_expected()"
    );
    debug_assert_eq!(
        emitter.mix_var_count, mix_expected,
        "emitted mix var count must equal PolyExtStepDef::mix_expected()"
    );
    let ret_slot = emitter
        .mix_slot_for(def.ret)
        .expect("ret mix var was marked usize::MAX last-use so its slot is still live");
    Ok(emitter.finalize(name, ret_slot))
}

/// Convenience: pick `FieldMode::Base` when possible, else `FieldMode::Ext`.
/// Propagates `CodegenError::TapIndexOutOfRange` / `TapGroupOutOfRange`
/// if the tap table is incomplete or has out-of-range groups; never
/// returns `ConstExtInBaseField` (the mode is auto-selected).
pub fn staged_kernel_from_def(
    name: &str,
    def: &PolyExtStepDef,
    taps: &[EmitterTap],
) -> Result<StagedKernel, CodegenError> {
    let mode = if def_is_base_field(def) {
        FieldMode::Base
    } else {
        FieldMode::Ext
    };
    staged_kernel_from_def_with_mode(name, def, taps, mode)
}

/// Static WGSL prelude that the staged kernel composes with the per-DEF
/// body emitted by [`WgslEmitter`]. The prelude declares the bindings,
/// params struct, field-arithmetic helpers (`add`/`sub`/`mul` and
/// `ext_add`/`ext_sub`/`ext_mul`/`ext_scale`), the read helpers
/// (`read_tap_scalar`/`read_tap_ext`/`read_global_scalar`/`read_global_ext`),
/// the mix-power loader (`load_mix_pow`), and the per-cycle output writer
/// (`write_check`).
///
/// The prelude binding layout intentionally matches the runtime
/// interpreter's at `webgpu.rs:EVAL_CHECK_BASE_INTERPRETER_WGSL`, MINUS
/// the `instrs` binding and `instr_*` params (the staged kernel embeds
/// the DEF inline, so there's no instruction stream to read). When SP3's
/// dispatch wiring lands, the same `Params` UBO can be reused — staged
/// dispatches simply leave `instr_*` fields ignored.
pub const STAGED_EVAL_CHECK_PRELUDE_WGSL: &str = include_str!("webgpu_codegen/prelude.wgsl");

/// Emit a complete, ready-to-compile WGSL shader for an arbitrary
/// `PolyExtStepDef` in the chosen `field_mode`. The result is the
/// [`STAGED_EVAL_CHECK_PRELUDE_WGSL`] prelude followed by the per-DEF
/// body from [`WgslEmitter`]. Callers feed this to
/// `WebGpuHal::create_compute_kernel` and dispatch on the same bindings
/// as the runtime interpreter (sans `instrs`).
pub fn staged_full_kernel_wgsl(
    name: &str,
    def: &PolyExtStepDef,
    taps: &[EmitterTap],
    field_mode: FieldMode,
) -> Result<String, CodegenError> {
    let body_kernel = staged_kernel_from_def_with_mode(name, def, taps, field_mode)?;
    let mut full = String::with_capacity(STAGED_EVAL_CHECK_PRELUDE_WGSL.len() + body_kernel.wgsl_source.len() + 64);
    full.push_str(STAGED_EVAL_CHECK_PRELUDE_WGSL);
    full.push('\n');
    full.push_str(&body_kernel.wgsl_source);
    Ok(full)
}

/// rv32im staged-WGSL `eval_check` generator.
///
/// rv32im is a pure base-field circuit (no `ConstExt` steps in its DEF), so
/// the generator unconditionally uses `FieldMode::Base` to mirror the
/// runtime base-field interpreter's `fp[lane][slot]: u32` slot layout.
///
/// At SP3 iter 3 this is still a scaffold: the rv32im production DEF is
/// ~20k ops and emitting one straight-line kernel may exceed WGSL shader
/// length / compile-time budgets in Chrome. Production wiring + multi-stage
/// split (mirroring the 4-file CUDA `eval_check_{0,1,2,3}.cu` layout) is
/// the next SP3 commit. For now this returns an empty set for all po2; the
/// production prove path continues to use the runtime interpreter.
pub fn rv32im_codegen_for_po2(_po2: u32) -> Vec<StagedKernel> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tiny PolyExt program used in structural-parity tests:
    /// - f0 = Const(7)
    /// - f1 = Const(3)
    /// - f2 = Add(f0, f1)
    /// - m0 = True
    /// - m1 = AndEqz(m0, f2)
    static TINY_BLOCK: &[PolyExtStep] = &[
        PolyExtStep::Const(7),
        PolyExtStep::Const(3),
        PolyExtStep::Add(0, 1),
        PolyExtStep::True,
        PolyExtStep::AndEqz(0, 2),
    ];

    static TINY_DEF: PolyExtStepDef = PolyExtStepDef {
        block: TINY_BLOCK,
        ret: 1,
    };

    /// Tiny PolyExt program exercising every PolyExtStep variant including
    /// `ConstExt`, so it requires `FieldMode::Ext`.
    static FULL_BLOCK: &[PolyExtStep] = &[
        PolyExtStep::Const(2),                  // f0
        PolyExtStep::ConstExt(1, 2, 3, 4),      // f1 (requires Ext)
        PolyExtStep::Get(5),                    // f2
        PolyExtStep::GetGlobal(0, 7),           // f3
        PolyExtStep::Add(0, 2),                 // f4
        PolyExtStep::Sub(4, 1),                 // f5
        PolyExtStep::Mul(5, 3),                 // f6
        PolyExtStep::True,                      // m0
        PolyExtStep::AndEqz(0, 6),              // m1
        PolyExtStep::AndCond(1, 0, 1),          // m2
    ];

    static FULL_DEF: PolyExtStepDef = PolyExtStepDef {
        block: FULL_BLOCK,
        ret: 2,
    };

    /// Tap table that covers the indices referenced by `FULL_DEF` and the
    /// reusable Get-only DEFs below. Indices 0-9 are populated so that
    /// `Get(5)` and `Get(3)` both resolve cleanly.
    static TEST_TAPS: &[EmitterTap] = &[
        EmitterTap { group: 0, offset: 0, back_inv_rate: 0 },
        EmitterTap { group: 1, offset: 1, back_inv_rate: 0 },
        EmitterTap { group: 2, offset: 2, back_inv_rate: 0 },
        EmitterTap { group: 0, offset: 3, back_inv_rate: 4 },
        EmitterTap { group: 1, offset: 4, back_inv_rate: 4 },
        EmitterTap { group: 2, offset: 5, back_inv_rate: 0 },
        EmitterTap { group: 0, offset: 6, back_inv_rate: 0 },
        EmitterTap { group: 1, offset: 7, back_inv_rate: 0 },
        EmitterTap { group: 2, offset: 8, back_inv_rate: 0 },
        EmitterTap { group: 0, offset: 9, back_inv_rate: 0 },
    ];

    #[test]
    fn def_var_counts_match_polyext_expected() {
        let (fp, mix) = def_var_counts(&TINY_DEF);
        assert_eq!(fp, 3);
        assert_eq!(mix, 2);
        let (fp, mix) = def_var_counts(&FULL_DEF);
        assert_eq!(fp, 7);
        assert_eq!(mix, 3);
    }

    #[test]
    fn def_mix_exponents_match_executor() {
        assert_eq!(def_mix_exponents(&TINY_DEF), vec![0, 1]);
        assert_eq!(def_mix_exponents(&FULL_DEF), vec![0, 1, 2]);
    }

    #[test]
    fn def_is_base_field_detects_const_ext() {
        assert!(def_is_base_field(&TINY_DEF));
        assert!(!def_is_base_field(&FULL_DEF));
    }

    #[test]
    fn base_field_emits_scalar_add_sub_mul_for_tiny_def() {
        let kernel =
            staged_kernel_from_def_with_mode("tiny_base", &TINY_DEF, &[], FieldMode::Base).unwrap();
        assert_eq!(kernel.field_mode, FieldMode::Base);
        assert!(kernel.wgsl_source.contains("var fp: array<u32, 3>"));
        assert!(kernel.wgsl_source.contains("fp[0] = 7u;"));
        assert!(kernel.wgsl_source.contains("fp[1] = 3u;"));
        assert!(kernel.wgsl_source.contains("fp[2] = add(fp[0], fp[1]);"));
        assert!(!kernel.wgsl_source.contains("fp[2] = ext_add"));
        assert!(
            kernel
                .wgsl_source
                .contains("ext_scale(mix_mul[0], fp[2])"),
            "AndEqz under Base mode must use ext_scale to promote fp[inner]"
        );
    }

    #[test]
    fn ext_field_emits_vec4_for_full_def() {
        let kernel =
            staged_kernel_from_def_with_mode("full_ext", &FULL_DEF, TEST_TAPS, FieldMode::Ext)
                .unwrap();
        assert_eq!(kernel.field_mode, FieldMode::Ext);
        // With slot allocation, FULL_DEF's 7 fp vars reuse 5 physical slots:
        // var 0 stays at slot 0 (last use is AndCond at op 9 — kept alive);
        // var 4 (Add) gets slot 4; var 5 (Sub) reuses slot 2 (vacated by
        // var 2 after Add); var 6 (Mul) reuses slot 1 (vacated by var 1
        // after Sub). Max slot = 5.
        assert!(kernel.wgsl_source.contains("var fp: array<vec4<u32>, 5>"));
        // var 1 ConstExt assigned to slot 1.
        assert!(
            kernel
                .wgsl_source
                .contains("fp[1] = vec4<u32>(1u, 2u, 3u, 4u);")
        );
        // var 4 Add(0, 2) -> fp[4] = ext_add(fp[0], fp[2]).
        assert!(kernel.wgsl_source.contains("fp[4] = ext_add(fp[0], fp[2]);"));
        // var 5 Sub(4, 1) reuses slot 2 (var 2's slot was freed after Add).
        assert!(kernel.wgsl_source.contains("fp[2] = ext_sub(fp[4], fp[1]);"));
        // var 6 Mul(5, 3) reuses slot 1 (var 1's slot was freed after Sub).
        assert!(kernel.wgsl_source.contains("fp[1] = ext_mul(fp[2], fp[3]);"));
        // AndEqz combines mix_mul[chain_slot=0] with fp[inner_slot=1] (var 6 lives in slot 1).
        assert!(
            kernel.wgsl_source.contains("ext_mul(mix_mul[0], fp[1])"),
            "AndEqz under Ext mode must use ext_mul over fp[6]'s reused slot 1"
        );
    }

    #[test]
    fn base_field_rejects_const_ext() {
        let err = staged_kernel_from_def_with_mode(
            "full_base",
            &FULL_DEF,
            TEST_TAPS,
            FieldMode::Base,
        );
        assert_eq!(err, Err(CodegenError::ConstExtInBaseField));
    }

    #[test]
    fn tap_index_out_of_range_surfaces_error() {
        // FULL_DEF has Get(5) but we supply only 3 taps.
        let err =
            staged_kernel_from_def_with_mode("oob", &FULL_DEF, &TEST_TAPS[..3], FieldMode::Ext);
        assert_eq!(
            err,
            Err(CodegenError::TapIndexOutOfRange {
                tap_idx: 5,
                taps_len: 3,
            })
        );
    }

    #[test]
    fn tap_group_out_of_range_surfaces_error() {
        static BAD_TAPS: &[EmitterTap] = &[EmitterTap {
            group: 7,
            offset: 0,
            back_inv_rate: 0,
        }];
        static GET_DEF: PolyExtStepDef = PolyExtStepDef {
            block: &[PolyExtStep::Get(0), PolyExtStep::True],
            ret: 0,
        };
        let err = staged_kernel_from_def_with_mode("bad_group", &GET_DEF, BAD_TAPS, FieldMode::Base);
        assert_eq!(
            err,
            Err(CodegenError::TapGroupOutOfRange { tap_idx: 0, group: 7 })
        );
    }

    #[test]
    fn auto_mode_picks_base_for_const_only_def_and_ext_for_const_ext_def() {
        let tiny = staged_kernel_from_def("tiny_auto", &TINY_DEF, &[]).unwrap();
        assert_eq!(tiny.field_mode, FieldMode::Base);
        let full = staged_kernel_from_def("full_auto", &FULL_DEF, TEST_TAPS).unwrap();
        assert_eq!(full.field_mode, FieldMode::Ext);
    }

    #[test]
    fn get_emits_per_group_helper_with_inlined_offset_and_back() {
        // Single-Get DEF under Base mode: tap 0 -> g1, offset 1, back_inv_rate 0
        // per TEST_TAPS[0]. Wait — TEST_TAPS[0] is g0; let me pick tap 1 which is g1.
        static GET_BLOCK: &[PolyExtStep] = &[PolyExtStep::Get(1), PolyExtStep::True];
        static GET_DEF: PolyExtStepDef = PolyExtStepDef {
            block: GET_BLOCK,
            ret: 0,
        };
        let kernel = staged_kernel_from_def_with_mode(
            "get_base",
            &GET_DEF,
            TEST_TAPS,
            FieldMode::Base,
        )
        .unwrap();
        // TEST_TAPS[1] = { group: 1, offset: 1, back_inv_rate: 0 }
        assert!(
            kernel.wgsl_source.contains("read_g1_scalar(1u, 0u, cycle)"),
            "Get(1) under Base mode must resolve to read_g1_scalar with inlined offset+back"
        );
        // Confirm placeholder `read_tap_scalar(...)` is gone.
        assert!(!kernel.wgsl_source.contains("read_tap_scalar"));

        let kernel_ext =
            staged_kernel_from_def_with_mode("get_ext", &GET_DEF, TEST_TAPS, FieldMode::Ext)
                .unwrap();
        assert!(kernel_ext.wgsl_source.contains("read_g1_ext(1u, 0u, cycle)"));
    }

    #[test]
    fn get_global_specializes_per_mode() {
        static GG_BLOCK: &[PolyExtStep] = &[PolyExtStep::GetGlobal(1, 4), PolyExtStep::True];
        static GG_DEF: PolyExtStepDef = PolyExtStepDef {
            block: GG_BLOCK,
            ret: 0,
        };
        let base =
            staged_kernel_from_def_with_mode("gg_base", &GG_DEF, &[], FieldMode::Base).unwrap();
        assert!(base.wgsl_source.contains("read_global_scalar(1u, 4u)"));
        assert!(!base.wgsl_source.contains("read_global_ext"));

        let ext =
            staged_kernel_from_def_with_mode("gg_ext", &GG_DEF, &[], FieldMode::Ext).unwrap();
        assert!(ext.wgsl_source.contains("read_global_ext(1u, 4u)"));
    }

    #[test]
    fn staged_kernel_emits_one_line_per_step_with_op_comment() {
        let kernel = staged_kernel_from_def("tiny", &TINY_DEF, &[]).unwrap();
        // iter 6: emit comments include both poly_ext var index and the
        // physical slot the allocator chose. Match the simpler op-name
        // anchor that's stable across slot-allocation outcomes.
        for (idx, op) in TINY_DEF.block.iter().enumerate() {
            let op_anchor = match op {
                PolyExtStep::Const(v) => format!("= Const({v})"),
                PolyExtStep::Add(_, _) => "= Add(fp_var".to_string(),
                PolyExtStep::True => "= True (mix_pow".to_string(),
                PolyExtStep::AndEqz(_, _) => "= AndEqz(chain=mix_var".to_string(),
                _ => unreachable!(),
            };
            let marker = format!("[{idx}]");
            assert!(
                kernel.wgsl_source.contains(&marker),
                "step {idx} ({op:?}) missing op-idx marker '{marker}'"
            );
            assert!(
                kernel.wgsl_source.contains(&op_anchor),
                "step {idx} ({op:?}) missing op-name anchor '{op_anchor}'"
            );
        }
    }

    #[test]
    fn staged_kernel_writes_ret_mix_tot_to_check_buffer() {
        let kernel = staged_kernel_from_def("tiny", &TINY_DEF, &[]).unwrap();
        assert!(
            kernel.wgsl_source.contains("write_check(cycle, mix_tot[1]);"),
            "ret = 1 means we write mix_tot[1] to the check buffer at cycle"
        );
    }

    #[test]
    fn ext_mode_uses_ext_helpers_for_all_arithmetic_ops_for_full_def() {
        let kernel = staged_kernel_from_def_with_mode("full", &FULL_DEF, TEST_TAPS, FieldMode::Ext)
            .unwrap();
        for token in [
            "ext_add(",
            "ext_sub(",
            "ext_mul(",
            "read_g2_ext(",     // FULL_DEF Get(5) -> TEST_TAPS[5] = g2
            "read_global_ext(",
            "load_mix_pow(",
        ] {
            assert!(
                kernel.wgsl_source.contains(token),
                "FULL_DEF under Ext mode must emit '{token}' at least once"
            );
        }
    }

    #[test]
    fn rv32im_codegen_for_po2_returns_empty_until_production_wiring_lands() {
        for po2 in [0u32, 1, 2, 3, 16, 21] {
            let kernels = rv32im_codegen_for_po2(po2);
            assert!(
                kernels.is_empty(),
                "rv32im_codegen_for_po2({po2}) is empty until SP3 follow-on lands the multi-stage split and dispatch wiring"
            );
        }
    }

    #[test]
    fn prelude_declares_all_helper_functions_emitter_calls() {
        let prelude = STAGED_EVAL_CHECK_PRELUDE_WGSL;
        // Scalar arithmetic.
        assert!(prelude.contains("fn add(lhs: u32, rhs: u32) -> u32"));
        assert!(prelude.contains("fn sub(lhs: u32, rhs: u32) -> u32"));
        assert!(prelude.contains("fn mul(lhs: u32, rhs: u32) -> u32"));
        // Extension arithmetic.
        assert!(prelude.contains("fn ext_add(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32>"));
        assert!(prelude.contains("fn ext_sub(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32>"));
        assert!(prelude.contains("fn ext_mul(lhs: vec4<u32>, rhs: vec4<u32>) -> vec4<u32>"));
        assert!(prelude.contains("fn ext_scale(lhs: vec4<u32>, rhs: u32) -> vec4<u32>"));
        // Per-group tap readers (one per group, in both modes).
        for g in 0..3u32 {
            assert!(
                prelude.contains(&format!(
                    "fn read_g{g}_scalar(offset: u32, back_inv_rate: u32, cycle: u32) -> u32"
                )),
                "prelude missing read_g{g}_scalar"
            );
            assert!(
                prelude.contains(&format!(
                    "fn read_g{g}_ext(offset: u32, back_inv_rate: u32, cycle: u32) -> vec4<u32>"
                )),
                "prelude missing read_g{g}_ext"
            );
        }
        // Placeholder helpers from iter 3 are gone.
        assert!(
            !prelude.contains("fn read_tap_scalar"),
            "iter 4 replaces placeholder read_tap_scalar with read_g{{0,1,2}}_scalar"
        );
        assert!(!prelude.contains("fn read_tap_ext"));
        // Global readers and mix-power loader and check writer.
        assert!(prelude.contains("fn read_global_scalar(arg: u32, offset: u32) -> u32"));
        assert!(prelude.contains("fn read_global_ext(arg: u32, offset: u32) -> vec4<u32>"));
        assert!(prelude.contains("fn load_mix_pow(mix_idx: u32) -> vec4<u32>"));
        assert!(prelude.contains("fn write_check(cycle: u32, val: vec4<u32>)"));
    }

    #[test]
    fn prelude_binds_same_layout_as_runtime_interpreter_minus_instrs() {
        let prelude = STAGED_EVAL_CHECK_PRELUDE_WGSL;
        // Same eight active bindings as the interpreter; binding 6 (`instrs`)
        // intentionally skipped.
        assert!(prelude.contains("@group(0) @binding(0) var<storage, read_write> check:"));
        assert!(prelude.contains("@group(0) @binding(1) var<storage, read> group0:"));
        assert!(prelude.contains("@group(0) @binding(2) var<storage, read> group1:"));
        assert!(prelude.contains("@group(0) @binding(3) var<storage, read> group2:"));
        assert!(prelude.contains("@group(0) @binding(4) var<storage, read> global0:"));
        assert!(prelude.contains("@group(0) @binding(5) var<storage, read> global1:"));
        assert!(prelude.contains("@group(0) @binding(7) var<storage, read> mix_pows:"));
        assert!(prelude.contains("@group(0) @binding(8) var<uniform> params:"));
        assert!(
            !prelude.contains("@group(0) @binding(6) "),
            "binding 6 (instrs) is intentionally absent from the staged prelude"
        );
    }

    #[test]
    fn full_kernel_concatenates_prelude_then_body_for_base_field_def() {
        let full = staged_full_kernel_wgsl("tiny", &TINY_DEF, &[], FieldMode::Base)
            .expect("Base ok for TINY_DEF (no Get ops)");
        let prelude_anchor = "fn write_check(cycle: u32, val: vec4<u32>)";
        // iter 6: kernel uses `@workgroup_size(1)` to bound private memory.
        let body_anchor = "@compute @workgroup_size(1)";
        let prelude_pos = full.find(prelude_anchor).expect("prelude present");
        let body_pos = full.find(body_anchor).expect("emitter body present");
        assert!(prelude_pos < body_pos, "prelude must precede emitter body");
        assert!(full.contains("fp[2] = add(fp[0], fp[1]);"));
    }

    #[test]
    fn full_kernel_rejects_const_ext_in_base_mode_via_codegen_error() {
        let err = staged_full_kernel_wgsl("full_base", &FULL_DEF, TEST_TAPS, FieldMode::Base);
        assert_eq!(err, Err(CodegenError::ConstExtInBaseField));
    }

    #[test]
    fn full_kernel_assembles_for_ext_field_def() {
        let full = staged_full_kernel_wgsl("full_ext", &FULL_DEF, TEST_TAPS, FieldMode::Ext)
            .expect("Ext mode accepts ConstExt");
        assert!(full.contains("fn ext_mul(lhs: vec4<u32>, rhs: vec4<u32>)"));
        // Mul(5, 3) under slot reuse: f5 lives in slot 2, f3 in slot 3, f6 in slot 1.
        assert!(full.contains("fp[1] = ext_mul(fp[2], fp[3]);"));
        // iter 6: @workgroup_size(1) for private-memory bounding.
        assert!(full.contains("@compute @workgroup_size(1)"));
        // AndCond's `ret` mix slot: in FULL_DEF, m2's slot is reused as 0
        // (m0 was freed before AndCond runs). The write_check uses the
        // ret mix slot (here 0), not the ret var index (2).
        assert!(full.contains("write_check(cycle, mix_tot[0]);"));
        // Get(5) -> TEST_TAPS[5] = g2, offset 5, back 0 — should reference read_g2_ext.
        assert!(full.contains("read_g2_ext(5u, 0u, cycle)"));
    }

    /// Structural-parity check: the generated kernel's slot layout and
    /// emitted ops mirror `PolyExtStepDef`'s declared fp_expected /
    /// mix_expected / mix_exponents exactly. Runtime byte-equivalent parity
    /// against `risc0_zkp::adapter::PolyExtStepDef::step` requires a real
    /// WebGPU device and lives in `examples/browser-prove/src/lib.rs` once
    /// SP3 wiring lands.
    #[test]
    fn structural_parity_matches_polyext_for_tiny_def_under_base_mode() {
        let (fp_expected, mix_expected) = def_var_counts(&TINY_DEF);
        let exps = def_mix_exponents(&TINY_DEF);
        let kernel = staged_kernel_from_def("tiny", &TINY_DEF, &[]).unwrap();
        assert_eq!(kernel.fp_slots, fp_expected);
        assert_eq!(kernel.mix_slots, mix_expected);
        for (n, exp) in exps.iter().enumerate() {
            assert!(
                kernel
                    .wgsl_source
                    .contains(&format!("mix_mul[{n}] = load_mix_pow({exp}u)")),
                "mix var {n} must load mix_pow at exponent {exp}"
            );
        }
    }
}
