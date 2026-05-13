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
    /// SP3 iter 7m: WGSL `@compute @workgroup_size(N)` chosen by the
    /// emitter to maximize GPU SIMD utilization while keeping
    /// per-workgroup private memory below
    /// `WEBGPU_COMPUTE_WORKGROUP_STORAGE_BUDGET`. The dispatch wiring
    /// reads this to compute `dispatch_workgroups(domain / workgroup_size,
    /// 1, 1)`-shaped launches.
    pub workgroup_size: u32,
    /// The WGSL source code for this kernel's *body*. The runtime prelude
    /// (bindings, params, arithmetic helpers, `read_tap` / `read_global` /
    /// `load_mix_pow` / `write_check`) is provided by `webgpu.rs` and
    /// concatenated by the dispatch wiring (SP3 follow-on).
    pub wgsl_source: String,
}

/// SP3 iter 7p: default `workgroup_size` for the staged eval_check
/// kernel. Mirrors the runtime interpreter's Ext-mode dispatch
/// (`WEBGPU_EVAL_CHECK_INTERPRETER_WORKGROUP_SIZE` in `webgpu.rs`).
/// Function-scope `var` arrays are per-thread private memory —
/// drivers spill to per-thread local memory automatically — so we
/// don't bound the per-thread footprint here. The size of 32 covers
/// Nvidia's 32-wide warp and is the empirically validated choice in
/// the existing interpreter path for the same DEFs.
pub const WEBGPU_STAGED_WORKGROUP_SIZE: u32 = 32;

/// SP3 iter 7p: bytes-per-thread budget used for *workgroup-shared*
/// scratch in Base mode. The interpreter falls back to a
/// `var<workgroup>` scratch when fp_slots are too large for
/// per-thread `var` to be a clean win; we don't currently emit a
/// workgroup-scratch path from the staged emitter, so the codegen
/// just returns the constant `WEBGPU_STAGED_WORKGROUP_SIZE`. Left
/// here documenting the budget for any future workgroup-scratch
/// variant.
#[allow(dead_code)]
pub const WEBGPU_COMPUTE_WORKGROUP_STORAGE_BUDGET: u32 = 49152;

/// SP3 iter 7p: return the staged kernel's workgroup_size. Per-thread
/// state (`fp` / `mix_tot` / `mix_mul`) lives in WGSL function-scope
/// `var` arrays — driver-managed per-thread private memory — so we
/// don't divide by per-thread byte footprint the way a
/// workgroup-shared budget would. Matches the runtime interpreter's
/// Ext-mode pipeline.
fn choose_workgroup_size(
    field_mode: FieldMode,
    fp_slots: usize,
    mix_slots: usize,
) -> u32 {
    // Inputs are kept for future per-stage tuning hooks (e.g., a
    // workgroup-shared scratch variant for very large DEFs would
    // gate `workgroup_size` on `fp_slots` / `mix_slots`).
    let _ = (field_mode, fp_slots, mix_slots);
    WEBGPU_STAGED_WORKGROUP_SIZE
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

    /// SP3 iter 7t: reset the emitter's slot allocators and slot maps
    /// at a chunk boundary. Caller must then call `seed_live_fp` /
    /// `seed_live_mix` for each var that was live across the
    /// boundary, in `live_idx` order — that re-establishes the
    /// `var_idx → slot` mapping for the new chunk before any ops are
    /// emitted.
    fn reset_chunk_slots(&mut self) {
        self.fp_alloc = EvalCheckSlotAllocator::default();
        self.mix_alloc = EvalCheckSlotAllocator::default();
        for slot in self.fp_slot_map.iter_mut() {
            *slot = None;
        }
        for slot in self.mix_slot_map.iter_mut() {
            *slot = None;
        }
    }

    /// SP3 iter 7t: allocate a fresh slot for `var` in the current
    /// chunk's allocator. The caller has the var live (from prev
    /// boundary's `live_fp`) and emits a scratch-load into the
    /// returned slot. Returns the assigned slot.
    fn seed_live_fp(&mut self, var: usize) -> usize {
        let slot = self.fp_alloc.alloc();
        if var >= self.fp_slot_map.len() {
            self.fp_slot_map.resize(var + 1, None);
        }
        self.fp_slot_map[var] = Some(slot);
        slot
    }

    /// SP3 iter 7t: mix-side counterpart to `seed_live_fp`.
    fn seed_live_mix(&mut self, var: usize) -> usize {
        let slot = self.mix_alloc.alloc();
        if var >= self.mix_slot_map.len() {
            self.mix_slot_map.resize(var + 1, None);
        }
        self.mix_slot_map[var] = Some(slot);
        slot
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
    #[allow(dead_code)]
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
        // SP3 iter 7m: workgroup_size sized to fit `fp` + `mix_tot` +
        // `mix_mul` private arrays inside the typical 48 KiB per-
        // workgroup storage budget. Iter 6 used a hardcoded `1`; on
        // poseidon2_basic that left the GPU dramatically underutilized
        // and the staged kernel ran ~10x slower than the interpreter.
        let workgroup_size =
            choose_workgroup_size(self.field_mode, fp_slots, mix_slots);
        writeln!(wgsl, "@compute @workgroup_size({workgroup_size})").unwrap();
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
            workgroup_size,
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

// ============================================================================
// SP3 iter 7: multi-stage staged-kernel planner.
//
// The single-kernel emitter (iters 1-6) inlines the entire DEF block into one
// compute shader. For the rv32im production DEF (~20k PolyExtSteps) the
// resulting kernel exhausts the per-dispatch budget of Chrome's SwiftShader
// fallback (and likely of real-GPU TDR limits too — see iter 6 evidence).
//
// iter 7 splits the DEF into N chunks of ~5k ops each, mirroring CUDA's
// 4-file `eval_check_{0,1,2,3}.cu` layout. Between chunks, the kernel
// serializes the live fp/mix vars to a per-cycle scratch storage buffer; the
// next chunk reloads them at its entry. The scratch buffer is sized to
// `max_live_at_any_boundary * domain` entries — typically a few hundred
// elements per cycle, well within `maxStorageBufferBindingSize`.
//
// This module adds the planner that walks the DEF, runs the same slot
// allocator the single-kernel emitter uses, and captures the live-set
// snapshot at each chunk boundary. The follow-on iter (7b) emits per-chunk
// WGSL using the planner's output; iter 7c wires multi-kernel dispatch.
// ============================================================================

/// One chunk-boundary snapshot: which fp and mix vars are live at the end
/// of the chunk that produced ops `[prev_end, end)`, along with their
/// physical slot assignments at that moment.
#[derive(Debug, Clone, PartialEq, Eq)]
/// SP3 iter 7t: live-set snapshot at a chunk boundary.
///
/// Stores `var_idx` (poly_ext fp/mix var indices) for each live var.
/// The `live_idx` (position in the vec) is the stable cross-chunk
/// identifier used in the scratch buffer: chunk K stores `fp[slot]`
/// into `fp_scratch[live_idx]`, chunk K+1 reads `fp_scratch[live_idx]`
/// into its own freshly-allocated slot for the same var. Slots are
/// chunk-local (each chunk's emitter resets its allocator), so a
/// single `var_idx` may live in slot 5 in chunk K and slot 0 in
/// chunk K+1.
pub struct ChunkBoundary {
    /// Exclusive end op_idx for this chunk. Chunk K runs ops
    /// `[boundaries[K-1].end, boundaries[K].end)` (with boundary -1 = 0).
    pub end: usize,
    /// SP3 iter 7t: fp vars live at this boundary, in stable
    /// `live_idx` order. Each entry is the `var_idx` (poly_ext fp
    /// var index). The `live_idx` is the position in this vec and
    /// is the cross-chunk scratch buffer offset.
    pub live_fp: Vec<usize>,
    /// SP3 iter 7t: mix vars live at this boundary, in stable
    /// `live_idx` order. Each entry is the `var_idx` (poly_ext mix
    /// var index).
    pub live_mix: Vec<usize>,
}

/// Plan output: per-chunk boundaries + slot/scratch-stride dimensioning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiKernelPlan {
    /// One entry per chunk. The last boundary's `end` is `def.block.len()`.
    /// At least one boundary is produced (the trivial case of a single
    /// chunk that fits below `target_chunk_ops`).
    pub boundaries: Vec<ChunkBoundary>,
    /// High-water mark of fp / mix slot indices ever allocated, sized
    /// across the entire DEF. Each per-chunk kernel sizes its `fp` /
    /// `mix_tot` / `mix_mul` local arrays to these numbers — slots are
    /// physical, not chunk-local, so they're shared across chunks.
    pub fp_slots: usize,
    pub mix_slots: usize,
    /// Max `live_fp.len()` across all *non-final* boundaries — drives the
    /// fp scratch buffer stride (each cycle reserves this many u32 / vec4
    /// entries depending on field mode).
    pub max_live_fp: usize,
    /// Max `live_mix.len()` across non-final boundaries. The mix scratch
    /// holds both `mix_tot` and `mix_mul` for each live mix var.
    pub max_live_mix: usize,
}

/// Result of multi-stage emission for a DEF: per-chunk `StagedKernel`s
/// plus the scratch dimensioning the dispatch wiring needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedMultiKernel {
    pub id: String,
    pub field_mode: FieldMode,
    /// One kernel per chunk, in dispatch order. Each kernel's
    /// `wgsl_source` includes scratch loads (for chunks > 0) and
    /// scratch stores (for chunks < N-1).
    pub stages: Vec<StagedKernel>,
    /// Number of u32 words per cycle in `fp_scratch` (max live fp
    /// vars at any non-final boundary × 1 for Base mode or × 4 for
    /// Ext mode).
    pub fp_scratch_stride_u32: usize,
    /// Number of u32 words per cycle in each of `mix_tot_scratch` /
    /// `mix_mul_scratch` (max live mix vars × 4).
    pub mix_scratch_stride_u32: usize,
}

/// Per-chunk emission of WGSL with scratch I/O bracketing the chunk's ops.
///
/// `prev_boundary` is `None` for the first chunk (no live-in to load) or
/// `Some(&plan.boundaries[k-1])` for chunks `k > 0`. `this_boundary` is
/// the chunk's own boundary; for the last chunk the body emits
/// `write_check(cycle, mix_tot[ret_slot])` instead of scratch stores.
fn emit_chunk_wgsl(
    name: &str,
    chunk_idx: usize,
    is_last: bool,
    field_mode: FieldMode,
    plan: &MultiKernelPlan,
    prev_boundary: Option<&ChunkBoundary>,
    this_boundary: &ChunkBoundary,
    ops: &[PolyExtStep],
    op_start_idx: usize,
    taps: &[EmitterTap],
    emitter: &mut WgslEmitter,
    ret_var: usize,
) -> Result<StagedKernel, CodegenError> {
    emitter.body.clear();
    // SP3 iter 7t: reset the emitter's slot allocators at every chunk
    // boundary (including chunk 0, where the reset is a no-op since the
    // emitter was just created). Live-in vars re-allocate fresh slots
    // below. This is what bounds each chunk's `fp_slots` /
    // `mix_slots` to the per-chunk high-water instead of the global
    // DEF max — drops the rv32im poseidon2_basic emit from 927 fp
    // slots down to whatever the widest chunk actually needs.
    emitter.reset_chunk_slots();
    // 1) Live-in scratch loads (only for chunks > 0). Scratch is indexed
    // by `tile_local` (the thread's offset within the current tile),
    // NOT the global `cycle` — so the scratch buffer can be sized to
    // `tile_size * stride` regardless of the prove's full domain.
    if let Some(prev) = prev_boundary {
        writeln!(
            emitter.body,
            "  // === chunk {chunk_idx} live-in: restore {} fp + {} mix from scratch (tile_local) ===",
            prev.live_fp.len(),
            prev.live_mix.len()
        )
        .unwrap();
        let fp_helper = match field_mode {
            FieldMode::Base => "read_fp_scratch_scalar",
            FieldMode::Ext => "read_fp_scratch_ext",
        };
        for (live_idx, var) in prev.live_fp.iter().enumerate() {
            let slot = emitter.seed_live_fp(*var);
            writeln!(
                emitter.body,
                "  fp[{slot}] = {fp_helper}(tile_local, {live_idx}u); // fp_var{var}",
            )
            .unwrap();
        }
        for (live_idx, var) in prev.live_mix.iter().enumerate() {
            let slot = emitter.seed_live_mix(*var);
            writeln!(
                emitter.body,
                "  mix_tot[{slot}] = read_mix_tot_scratch(tile_local, {live_idx}u); mix_mul[{slot}] = read_mix_mul_scratch(tile_local, {live_idx}u); // mix_var{var}",
            )
            .unwrap();
        }
    }
    // 2) Run the chunk's ops via the (chunk-local) emitter.
    for (offset, op) in ops.iter().enumerate() {
        let global_idx = op_start_idx + offset;
        emitter.emit(global_idx, op)?;
    }
    // 3) Live-out scratch stores OR write_check. Use the emitter's
    // CURRENT slot for each live var (post-emit, possibly different
    // from the planner's slot because allocators diverged).
    if !is_last {
        writeln!(
            emitter.body,
            "  // === chunk {chunk_idx} live-out: save {} fp + {} mix to scratch (tile_local) ===",
            this_boundary.live_fp.len(),
            this_boundary.live_mix.len()
        )
        .unwrap();
        let fp_helper = match field_mode {
            FieldMode::Base => "write_fp_scratch_scalar",
            FieldMode::Ext => "write_fp_scratch_ext",
        };
        for (live_idx, var) in this_boundary.live_fp.iter().enumerate() {
            let slot = emitter.fp_slot_for(*var)?;
            writeln!(
                emitter.body,
                "  {fp_helper}(tile_local, {live_idx}u, fp[{slot}]); // fp_var{var}",
            )
            .unwrap();
        }
        for (live_idx, var) in this_boundary.live_mix.iter().enumerate() {
            let slot = emitter.mix_slot_for(*var)?;
            writeln!(
                emitter.body,
                "  write_mix_tot_scratch(tile_local, {live_idx}u, mix_tot[{slot}]); write_mix_mul_scratch(tile_local, {live_idx}u, mix_mul[{slot}]); // mix_var{var}",
            )
            .unwrap();
        }
    } else {
        let ret_slot = emitter
            .mix_slot_for(ret_var)
            .map_err(|err| match err {
                CodegenError::VarNotLive(msg) => CodegenError::VarNotLive(format!(
                    "final chunk: ret mix var {ret_var} not live: {msg}"
                )),
                other => other,
            })?;
        writeln!(
            emitter.body,
            "  // === chunk {chunk_idx} final: write_check ===",
        )
        .unwrap();
        writeln!(emitter.body, "  write_check(cycle, mix_tot[{ret_slot}]);").unwrap();
    }

    // 4) Materialize as a StagedKernel. SP3 iter 7t: `fp_slots` /
    // `mix_slots` are now PER-CHUNK (the emitter resets at every
    // boundary and re-seeds live-ins, so its `max_used()` after
    // emitting this chunk is the chunk-local high-water). Each
    // stage's WGSL declares `array<..., fp_slots>` sized to its own
    // tight maximum, not the global DEF max.
    let stage_name = format!("{name}_stage{chunk_idx}");
    let fp_slots = emitter.fp_alloc.max_used().max(1);
    let mix_slots = emitter.mix_alloc.max_used().max(1);
    let mut wgsl = String::new();
    writeln!(wgsl, "// staged eval_check kernel: {stage_name}").unwrap();
    writeln!(
        wgsl,
        "// field_mode = {:?}, fp_slots = {fp_slots}, mix_slots = {mix_slots}, chunk = {chunk_idx}/{} (final = {is_last})",
        field_mode,
        plan.boundaries.len(),
    )
    .unwrap();
    wgsl.push_str(
        "// runtime prelude (bindings, params, ext_*/add/sub/mul helpers,\n// read_g{0,1,2}_*, read_global_*, load_mix_pow, write_check,\n// read/write_*_scratch) is the `STAGED_EVAL_CHECK_PRELUDE_WGSL` constant.\n",
    );
    // SP3 iter 7m: pick workgroup_size to fit `fp` + `mix_tot` +
    // `mix_mul` private arrays in the per-workgroup storage budget
    // (~48 KiB). Iter 6/7's hardcoded `1` left the GPU heavily
    // underutilized; sizing per the actual private memory footprint
    // lets each workgroup pack 16-64 threads. Each chunk uses
    // `plan.fp_slots` / `plan.mix_slots` (the cross-chunk high-water)
    // so all stages share a stable size.
    let workgroup_size = choose_workgroup_size(field_mode, fp_slots, mix_slots);
    // SP3 iter 7z: CUDA-inspired structure. CUDA's `eval_check` calls
    // `poly_fp` which calls `rv32im_v2_0..19` device functions — nvcc
    // optimizes register allocation per function. We mirror that by
    // declaring `fp`/`mix_tot`/`mix_mul` at module scope (`var<private>`,
    // per-thread persistent) and emitting the chunk body as a separate
    // `fn chunk_body(...)`. Main does the dispatch math + bounds check
    // and calls chunk_body. The driver can analyze each function's
    // register pressure independently — should generate tighter code
    // than one ~1.6 MB straight-line main.
    writeln!(
        wgsl,
        "var<private> fp: array<{}, {fp_slots}>;",
        field_mode.fp_ty()
    )
    .unwrap();
    writeln!(wgsl, "var<private> mix_tot: array<vec4<u32>, {mix_slots}>;").unwrap();
    writeln!(wgsl, "var<private> mix_mul: array<vec4<u32>, {mix_slots}>;").unwrap();
    wgsl.push_str("fn chunk_body(tile_local: u32, cycle: u32) {\n");
    wgsl.push_str(&emitter.body);
    wgsl.push_str("}\n");
    writeln!(wgsl, "@compute @workgroup_size({workgroup_size})").unwrap();
    wgsl.push_str("fn main(@builtin(global_invocation_id) gid: vec3<u32>) {\n");
    // SP3 iter 7g: CUDA-shape dispatch. The host calls
    // `dispatch_workgroups(tile_size / workgroup_size, num_tiles, 1)` ONCE
    // per stage. `tile_local = gid.x` is the thread's offset within a
    // tile; `tile_idx = gid.y` is which tile this thread belongs to;
    // the global cycle is `tile_idx * tile_size + tile_local`.
    wgsl.push_str("  let tile_local = gid.x;\n");
    wgsl.push_str("  let tile_idx = gid.y;\n");
    wgsl.push_str("  let cycle = tile_idx * staged_scratch_params.tile_size + tile_local;\n");
    wgsl.push_str("  if (cycle >= params.domain) { return; }\n");
    wgsl.push_str("  chunk_body(tile_local, cycle);\n");
    wgsl.push_str("}\n");

    let _ = taps;
    Ok(StagedKernel {
        id: stage_name,
        field_mode,
        fp_slots,
        mix_slots,
        workgroup_size,
        wgsl_source: wgsl,
    })
}

/// Emit a multi-stage staged WGSL kernel for `def`. Each chunk runs
/// roughly `target_chunk_ops` PolyExtSteps; the final chunk additionally
/// writes the check buffer. The N stages share slot assignments (same
/// emitter state threads through chunks), so a fp/mix var defined in
/// chunk K and consumed in chunk K+M lives at the same slot index in
/// both — the per-chunk scratch loads/stores rebind that slot at chunk
/// boundaries.
pub fn staged_multi_kernel_from_def(
    name: &str,
    def: &PolyExtStepDef,
    taps: &[EmitterTap],
    field_mode: FieldMode,
    target_chunk_ops: usize,
) -> Result<StagedMultiKernel, CodegenError> {
    let plan = plan_multi_kernel(def, target_chunk_ops)?;
    let mut emitter = WgslEmitter::new(def, taps, field_mode)?;
    let mut stages = Vec::with_capacity(plan.boundaries.len());

    let mut prev_end = 0usize;
    let n = plan.boundaries.len();
    // Walk boundaries by index so we can borrow the previous boundary.
    for chunk_idx in 0..n {
        let is_last = chunk_idx + 1 == n;
        let this_boundary = &plan.boundaries[chunk_idx];
        let prev_boundary = if chunk_idx == 0 {
            None
        } else {
            Some(&plan.boundaries[chunk_idx - 1])
        };
        let ops = &def.block[prev_end..this_boundary.end];
        let stage = emit_chunk_wgsl(
            name,
            chunk_idx,
            is_last,
            field_mode,
            &plan,
            prev_boundary,
            this_boundary,
            ops,
            prev_end,
            taps,
            &mut emitter,
            def.ret,
        )?;
        stages.push(stage);
        prev_end = this_boundary.end;
    }

    let fp_per_slot_u32 = match field_mode {
        FieldMode::Base => 1,
        FieldMode::Ext => 4,
    };
    let fp_scratch_stride_u32 = plan.max_live_fp * fp_per_slot_u32;
    let mix_scratch_stride_u32 = plan.max_live_mix * 4;

    Ok(StagedMultiKernel {
        id: name.to_string(),
        field_mode,
        stages,
        fp_scratch_stride_u32,
        mix_scratch_stride_u32,
    })
}

/// Plan an N-chunk emission for `def` targeting roughly `target_chunk_ops`
/// PolyExtSteps per chunk. Each chunk runs the same slot-allocation
/// discipline as the single-kernel emitter (matching the runtime
/// interpreter); the planner additionally snapshots which vars are live
/// at each chunk boundary so the per-chunk emitter knows what to save to
/// the scratch buffer.
pub fn plan_multi_kernel(
    def: &PolyExtStepDef,
    target_chunk_ops: usize,
) -> Result<MultiKernelPlan, CodegenError> {
    let target_chunk_ops = target_chunk_ops.max(1);

    let (last_fp, last_mix, _fp_count, _mix_count) = eval_check_last_uses(def)
        .map_err(|err| CodegenError::LastUseAnalysisFailed(err.to_string()))?;

    let mut fp_alloc = EvalCheckSlotAllocator::default();
    let mut mix_alloc = EvalCheckSlotAllocator::default();
    let mut fp_slot_map: Vec<Option<usize>> = Vec::new();
    let mut mix_slot_map: Vec<Option<usize>> = Vec::new();

    let mut fp_var_count = 0usize;
    let mut mix_var_count = 0usize;

    let mut boundaries: Vec<ChunkBoundary> = Vec::new();
    let mut max_live_fp = 0usize;
    let mut max_live_mix = 0usize;

    let block_len = def.block.len();
    let mut next_boundary_at = target_chunk_ops.min(block_len);

    for (op_idx, op) in def.block.iter().enumerate() {
        // Walk the op exactly like the emitter: allocate output slot,
        // free dead operands. We don't emit WGSL here, just track slots.
        let alloc_fp_var = |fp_alloc: &mut EvalCheckSlotAllocator,
                            fp_slot_map: &mut Vec<Option<usize>>,
                            fp_var_count: &mut usize,
                            last_fp: &[Option<usize>]| {
            let var_idx = *fp_var_count;
            *fp_var_count += 1;
            let slot = fp_alloc.alloc();
            fp_slot_map.push(Some(slot));
            if last_fp.get(var_idx).copied().flatten().is_none() {
                fp_slot_map[var_idx] = None;
                fp_alloc.free(slot);
            }
        };
        let alloc_mix_var = |mix_alloc: &mut EvalCheckSlotAllocator,
                             mix_slot_map: &mut Vec<Option<usize>>,
                             mix_var_count: &mut usize,
                             last_mix: &[Option<usize>]| {
            let var_idx = *mix_var_count;
            *mix_var_count += 1;
            let slot = mix_alloc.alloc();
            mix_slot_map.push(Some(slot));
            if last_mix.get(var_idx).copied().flatten().is_none() {
                mix_slot_map[var_idx] = None;
                mix_alloc.free(slot);
            }
        };
        match op {
            PolyExtStep::Const(_)
            | PolyExtStep::ConstExt(_, _, _, _)
            | PolyExtStep::Get(_)
            | PolyExtStep::GetGlobal(_, _) => {
                alloc_fp_var(&mut fp_alloc, &mut fp_slot_map, &mut fp_var_count, &last_fp);
            }
            PolyExtStep::Add(x, y) | PolyExtStep::Sub(x, y) | PolyExtStep::Mul(x, y) => {
                alloc_fp_var(&mut fp_alloc, &mut fp_slot_map, &mut fp_var_count, &last_fp);
                // Free dead operands.
                for &var in &[*x, *y] {
                    if let Some(Some(last)) = last_fp.get(var) {
                        if *last == op_idx {
                            if let Some(slot) = fp_slot_map[var].take() {
                                fp_alloc.free(slot);
                            }
                        }
                    }
                }
            }
            PolyExtStep::True => {
                alloc_mix_var(
                    &mut mix_alloc,
                    &mut mix_slot_map,
                    &mut mix_var_count,
                    &last_mix,
                );
            }
            PolyExtStep::AndEqz(chain, inner) => {
                alloc_mix_var(
                    &mut mix_alloc,
                    &mut mix_slot_map,
                    &mut mix_var_count,
                    &last_mix,
                );
                if let Some(Some(last)) = last_mix.get(*chain) {
                    if *last == op_idx {
                        if let Some(slot) = mix_slot_map[*chain].take() {
                            mix_alloc.free(slot);
                        }
                    }
                }
                if let Some(Some(last)) = last_fp.get(*inner) {
                    if *last == op_idx {
                        if let Some(slot) = fp_slot_map[*inner].take() {
                            fp_alloc.free(slot);
                        }
                    }
                }
            }
            PolyExtStep::AndCond(chain, cond, inner) => {
                alloc_mix_var(
                    &mut mix_alloc,
                    &mut mix_slot_map,
                    &mut mix_var_count,
                    &last_mix,
                );
                for &var in &[*chain, *inner] {
                    if let Some(Some(last)) = last_mix.get(var) {
                        if *last == op_idx {
                            if let Some(slot) = mix_slot_map[var].take() {
                                mix_alloc.free(slot);
                            }
                        }
                    }
                }
                if let Some(Some(last)) = last_fp.get(*cond) {
                    if *last == op_idx {
                        if let Some(slot) = fp_slot_map[*cond].take() {
                            fp_alloc.free(slot);
                        }
                    }
                }
            }
        }
        // Snapshot at chunk boundary OR at the last op.
        let is_last_op = op_idx + 1 == block_len;
        let at_boundary = op_idx + 1 >= next_boundary_at;
        if at_boundary || is_last_op {
            // SP3 iter 7t: store only var_idx (not slot). The slot is
            // chunk-local now; the live_idx (position in this vec) is
            // the stable scratch-buffer identifier.
            let live_fp: Vec<usize> = fp_slot_map
                .iter()
                .enumerate()
                .filter_map(|(var, slot)| if slot.is_some() { Some(var) } else { None })
                .collect();
            let live_mix: Vec<usize> = mix_slot_map
                .iter()
                .enumerate()
                .filter_map(|(var, slot)| if slot.is_some() { Some(var) } else { None })
                .collect();
            if !is_last_op {
                max_live_fp = max_live_fp.max(live_fp.len());
                max_live_mix = max_live_mix.max(live_mix.len());
            }
            boundaries.push(ChunkBoundary {
                end: op_idx + 1,
                live_fp,
                live_mix,
            });
            next_boundary_at = (op_idx + 1 + target_chunk_ops).min(block_len);
        }
    }

    Ok(MultiKernelPlan {
        boundaries,
        fp_slots: fp_alloc.max_used().max(1),
        mix_slots: mix_alloc.max_used().max(1),
        max_live_fp,
        max_live_mix,
    })
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
    fn plan_multi_kernel_returns_single_chunk_when_block_fits() {
        // TINY_DEF has 5 ops; target 100 ops/chunk → one chunk.
        let plan = plan_multi_kernel(&TINY_DEF, 100).unwrap();
        assert_eq!(plan.boundaries.len(), 1);
        assert_eq!(plan.boundaries[0].end, TINY_DEF.block.len());
        // No earlier boundaries → max_live_* are 0.
        assert_eq!(plan.max_live_fp, 0);
        assert_eq!(plan.max_live_mix, 0);
        // Slot counts match the single-kernel emission.
        assert_eq!(plan.fp_slots, 3);
        assert_eq!(plan.mix_slots, 2);
    }

    #[test]
    fn plan_multi_kernel_splits_at_target_chunk_size() {
        // TINY_DEF has 5 ops; target 2 ops/chunk → 3 chunks ([0,2), [2,4), [4,5)).
        let plan = plan_multi_kernel(&TINY_DEF, 2).unwrap();
        assert_eq!(plan.boundaries.len(), 3);
        assert_eq!(plan.boundaries[0].end, 2);
        assert_eq!(plan.boundaries[1].end, 4);
        assert_eq!(plan.boundaries[2].end, 5);

        // After chunk 0 (ops Const(7), Const(3)): fp_var 0 and fp_var 1
        // are live (used by Add at op 2 in chunk 1). SP3 iter 7t: the
        // boundary now records just var_idx (slot is chunk-local).
        assert_eq!(
            plan.boundaries[0].live_fp,
            vec![0, 1],
            "after chunk 0 both consts are live for the upcoming Add"
        );
        assert!(plan.boundaries[0].live_mix.is_empty());

        // After chunk 1 (ops Add(0,1), True): fp_var 2 is live (used by
        // AndEqz at op 4); mix_var 0 is live.
        assert_eq!(plan.boundaries[1].live_fp, vec![2]);
        assert_eq!(plan.boundaries[1].live_mix, vec![0]);

        // Final chunk's snapshot is recorded for completeness but excluded
        // from max_live_* (no scratch save needed at the end of the program).
        assert_eq!(plan.boundaries[2].live_mix, vec![1]);

        // Max live across non-final boundaries: 2 fp vars (boundary 0), 1 mix var (boundary 1).
        assert_eq!(plan.max_live_fp, 2);
        assert_eq!(plan.max_live_mix, 1);
        // Slot maxima identical to single-kernel.
        assert_eq!(plan.fp_slots, 3);
        assert_eq!(plan.mix_slots, 2);
    }

    #[test]
    fn multi_kernel_single_chunk_matches_single_kernel_for_small_def() {
        // target 100 ops/chunk → 1 chunk; multi-stage output should match
        // the single-kernel output up to the per-stage header comment.
        let multi =
            staged_multi_kernel_from_def("tiny", &TINY_DEF, &[], FieldMode::Base, 100).unwrap();
        assert_eq!(multi.stages.len(), 1);
        let stage0 = &multi.stages[0];
        // No live-in/out — no scratch references.
        assert!(!stage0.wgsl_source.contains("read_fp_scratch_"));
        assert!(!stage0.wgsl_source.contains("write_fp_scratch_"));
        // Last chunk emits write_check.
        assert!(stage0.wgsl_source.contains("write_check(cycle, mix_tot[1])"));
        // Strides are zero — no scratch needed.
        assert_eq!(multi.fp_scratch_stride_u32, 0);
        assert_eq!(multi.mix_scratch_stride_u32, 0);
    }

    #[test]
    fn multi_kernel_three_chunks_emit_scratch_io_at_boundaries() {
        // target 2 ops/chunk → 3 chunks. Stage 0 stores 2 fp vars at the
        // end; stage 1 loads them and stores its live-out; stage 2 loads.
        let multi =
            staged_multi_kernel_from_def("tiny3", &TINY_DEF, &[], FieldMode::Base, 2).unwrap();
        assert_eq!(multi.stages.len(), 3);

        // Stage 0: no live-in, two fp live-out (live_idx 0 = var 0 slot 0,
        // live_idx 1 = var 1 slot 1). "write_check" appears in the
        // prelude-reference comment in every stage; the CALL doesn't.
        let s0 = &multi.stages[0].wgsl_source;
        assert!(!s0.contains("fp[0] = read_fp_scratch_scalar"));
        assert!(s0.contains("write_fp_scratch_scalar(tile_local, 0u, fp[0])"));
        assert!(s0.contains("write_fp_scratch_scalar(tile_local, 1u, fp[1])"));
        assert!(!s0.contains("write_check(cycle, mix_tot["));

        // Stage 1: load 2 fp, run Add + True, store 1 fp + 1 mix.
        let s1 = &multi.stages[1].wgsl_source;
        assert!(s1.contains("fp[0] = read_fp_scratch_scalar(tile_local, 0u)"));
        assert!(s1.contains("fp[1] = read_fp_scratch_scalar(tile_local, 1u)"));
        // After Add, fp_var 2 lives at slot 2 (slots 0/1 freed by Add).
        assert!(s1.contains("write_fp_scratch_scalar(tile_local, 0u, fp[2])"));
        // After True, mix_var 0 lives at slot 0.
        assert!(s1.contains("write_mix_tot_scratch(tile_local, 0u, mix_tot[0])"));
        assert!(!s1.contains("write_check(cycle, mix_tot["));

        // Stage 2: load 1 fp + 1 mix, run AndEqz, write_check.
        // SP3 iter 7t: per-chunk allocator restarts at slot 0 for each
        // stage. Stage 2's first live-in (fp_var 2) lands in fp[0],
        // not fp[2] (the global single-allocator slot).
        let s2 = &multi.stages[2].wgsl_source;
        assert!(s2.contains("fp[0] = read_fp_scratch_scalar(tile_local, 0u)"));
        assert!(s2.contains("mix_tot[0] = read_mix_tot_scratch(tile_local, 0u)"));
        assert!(s2.contains("write_check(cycle, mix_tot[1])"));

        // Strides: max_live_fp = 2 (after stage 0), max_live_mix = 1
        // (after stage 1). Base mode: 1 u32 per fp slot.
        assert_eq!(multi.fp_scratch_stride_u32, 2);
        assert_eq!(multi.mix_scratch_stride_u32, 4); // 1 mix * 4 u32 per vec4
    }

    #[test]
    fn multi_kernel_uses_ext_helpers_for_ext_field_mode() {
        // FULL_DEF requires Ext mode. Two-chunk split exercises ext
        // scratch helpers.
        let multi =
            staged_multi_kernel_from_def("full2", &FULL_DEF, TEST_TAPS, FieldMode::Ext, 5).unwrap();
        assert!(multi.stages.len() >= 2);
        let early = &multi.stages[0].wgsl_source;
        // Boundary live-out for ext mode uses _ext helpers.
        assert!(
            early.contains("write_fp_scratch_ext(tile_local"),
            "ext mode must use write_fp_scratch_ext at chunk boundaries"
        );
        let last = &multi.stages.last().unwrap().wgsl_source;
        assert!(
            last.contains("write_check(cycle, mix_tot["),
            "last stage must call write_check"
        );
        // Strides: ext mode uses 4 u32s per fp slot.
        assert_eq!(multi.fp_scratch_stride_u32 % 4, 0);
    }

    #[test]
    fn plan_multi_kernel_handles_full_def_with_slot_reuse() {
        // FULL_DEF has 10 ops; target 3 ops/chunk → 4 chunks ([0,3), [3,6), [6,9), [9,10)).
        let plan = plan_multi_kernel(&FULL_DEF, 3).unwrap();
        assert_eq!(plan.boundaries.len(), 4);
        // The slot reuse from iter 6 still applies — the planner runs the
        // same allocator.
        assert_eq!(plan.fp_slots, 5);
        // Liveness at boundary 2 (end of [6,9)): mix_var 1 is live
        // (used by AndCond at op 9). fp_var 0 is live (used by AndCond's
        // cond operand). All other vars freed.
        let b2 = &plan.boundaries[2];
        assert!(
            b2.live_fp.contains(&0),
            "fp_var 0 must be live at end of chunk 2 (AndCond at op 9 reads it)"
        );
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
        // SP3 iter 7x: mix_pows is a uniform buffer (CUDA `__constant__`
        // analog), not a storage buffer like the interpreter uses.
        assert!(prelude.contains("@group(0) @binding(7) var<uniform> mix_pows:"));
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
        // iter 7m: kernel uses `@compute @workgroup_size(N)` where N is
        // picked by `choose_workgroup_size` to fit private memory.
        let body_anchor = "@compute @workgroup_size";
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
        // iter 7m: workgroup_size is picked by choose_workgroup_size to
        // fit fp_slots+mix_slots in the per-workgroup storage budget.
        // For FULL_DEF in Ext mode the chosen size is non-zero; we
        // just assert the @workgroup_size attribute is present.
        assert!(full.contains("@compute @workgroup_size"));
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
