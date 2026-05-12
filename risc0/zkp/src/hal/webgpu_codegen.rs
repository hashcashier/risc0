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
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::Write as _;

use crate::adapter::{PolyExtStep, PolyExtStepDef};

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

struct WgslEmitter {
    body: String,
    field_mode: FieldMode,
    fp_idx: usize,
    mix_idx: usize,
    mix_exps: Vec<usize>,
}

impl WgslEmitter {
    fn new(def: &PolyExtStepDef, field_mode: FieldMode) -> Self {
        Self {
            body: String::new(),
            field_mode,
            fp_idx: 0,
            mix_idx: 0,
            mix_exps: def_mix_exponents(def),
        }
    }

    fn emit(&mut self, idx: usize, op: &PolyExtStep) -> Result<(), CodegenError> {
        match (op, self.field_mode) {
            (PolyExtStep::Const(v), FieldMode::Base) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Const({v})").unwrap();
                writeln!(self.body, "  fp[{n}] = {v}u;").unwrap();
            }
            (PolyExtStep::Const(v), FieldMode::Ext) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Const({v})").unwrap();
                writeln!(self.body, "  fp[{n}] = vec4<u32>({v}u, 0u, 0u, 0u);").unwrap();
            }
            (PolyExtStep::ConstExt(_, _, _, _), FieldMode::Base) => {
                return Err(CodegenError::ConstExtInBaseField);
            }
            (PolyExtStep::ConstExt(a, b, c, d), FieldMode::Ext) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(
                    self.body,
                    "  // [{idx}] fp[{n}] = ConstExt({a}, {b}, {c}, {d})"
                )
                .unwrap();
                writeln!(self.body, "  fp[{n}] = vec4<u32>({a}u, {b}u, {c}u, {d}u);").unwrap();
            }
            (PolyExtStep::Get(tap), mode) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Get(tap={tap})").unwrap();
                match mode {
                    FieldMode::Base => writeln!(
                        self.body,
                        "  fp[{n}] = read_tap_scalar({tap}u, cycle);"
                    )
                    .unwrap(),
                    FieldMode::Ext => writeln!(
                        self.body,
                        "  fp[{n}] = read_tap_ext({tap}u, cycle);"
                    )
                    .unwrap(),
                };
            }
            (PolyExtStep::GetGlobal(arg, off), mode) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(
                    self.body,
                    "  // [{idx}] fp[{n}] = GetGlobal(arg={arg}, off={off})"
                )
                .unwrap();
                match mode {
                    FieldMode::Base => writeln!(
                        self.body,
                        "  fp[{n}] = read_global_scalar({arg}u, {off}u);"
                    )
                    .unwrap(),
                    FieldMode::Ext => writeln!(
                        self.body,
                        "  fp[{n}] = read_global_ext({arg}u, {off}u);"
                    )
                    .unwrap(),
                };
            }
            (PolyExtStep::Add(x, y), FieldMode::Base) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Add(fp[{x}], fp[{y}])").unwrap();
                writeln!(self.body, "  fp[{n}] = add(fp[{x}], fp[{y}]);").unwrap();
            }
            (PolyExtStep::Add(x, y), FieldMode::Ext) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Add(fp[{x}], fp[{y}])").unwrap();
                writeln!(self.body, "  fp[{n}] = ext_add(fp[{x}], fp[{y}]);").unwrap();
            }
            (PolyExtStep::Sub(x, y), FieldMode::Base) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Sub(fp[{x}], fp[{y}])").unwrap();
                writeln!(self.body, "  fp[{n}] = sub(fp[{x}], fp[{y}]);").unwrap();
            }
            (PolyExtStep::Sub(x, y), FieldMode::Ext) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Sub(fp[{x}], fp[{y}])").unwrap();
                writeln!(self.body, "  fp[{n}] = ext_sub(fp[{x}], fp[{y}]);").unwrap();
            }
            (PolyExtStep::Mul(x, y), FieldMode::Base) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Mul(fp[{x}], fp[{y}])").unwrap();
                writeln!(self.body, "  fp[{n}] = mul(fp[{x}], fp[{y}]);").unwrap();
            }
            (PolyExtStep::Mul(x, y), FieldMode::Ext) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Mul(fp[{x}], fp[{y}])").unwrap();
                writeln!(self.body, "  fp[{n}] = ext_mul(fp[{x}], fp[{y}]);").unwrap();
            }
            (PolyExtStep::True, _) => {
                let n = self.mix_idx;
                self.mix_idx += 1;
                let exp = self.mix_exps[n];
                writeln!(self.body, "  // [{idx}] mix[{n}] = True (mix_pow exp={exp})").unwrap();
                writeln!(
                    self.body,
                    "  mix_tot[{n}] = vec4<u32>(0u, 0u, 0u, 0u); mix_mul[{n}] = load_mix_pow({exp}u);"
                )
                .unwrap();
            }
            (PolyExtStep::AndEqz(chain, inner), FieldMode::Base) => {
                let n = self.mix_idx;
                self.mix_idx += 1;
                let exp = self.mix_exps[n];
                writeln!(
                    self.body,
                    "  // [{idx}] mix[{n}] = AndEqz(chain=mix[{chain}], inner=fp[{inner}])"
                )
                .unwrap();
                // Base-field inner: combine vec4 mix_mul[chain] with scalar fp[inner] via ext_scale.
                writeln!(
                    self.body,
                    "  mix_tot[{n}] = ext_add(mix_tot[{chain}], ext_scale(mix_mul[{chain}], fp[{inner}])); mix_mul[{n}] = load_mix_pow({exp}u);"
                )
                .unwrap();
            }
            (PolyExtStep::AndEqz(chain, inner), FieldMode::Ext) => {
                let n = self.mix_idx;
                self.mix_idx += 1;
                let exp = self.mix_exps[n];
                writeln!(
                    self.body,
                    "  // [{idx}] mix[{n}] = AndEqz(chain=mix[{chain}], inner=fp[{inner}])"
                )
                .unwrap();
                // Ext-field inner: full ext_mul of two vec4s.
                writeln!(
                    self.body,
                    "  mix_tot[{n}] = ext_add(mix_tot[{chain}], ext_mul(mix_mul[{chain}], fp[{inner}])); mix_mul[{n}] = load_mix_pow({exp}u);"
                )
                .unwrap();
            }
            (PolyExtStep::AndCond(chain, cond, inner), FieldMode::Base) => {
                let n = self.mix_idx;
                self.mix_idx += 1;
                let exp = self.mix_exps[n];
                writeln!(
                    self.body,
                    "  // [{idx}] mix[{n}] = AndCond(chain=mix[{chain}], cond=fp[{cond}], inner=mix[{inner}])"
                )
                .unwrap();
                // Base-field cond: scalar fp[cond] applied via ext_scale to the inner.tot * chain.mul product.
                writeln!(
                    self.body,
                    "  mix_tot[{n}] = ext_add(mix_tot[{chain}], ext_scale(ext_mul(mix_tot[{inner}], mix_mul[{chain}]), fp[{cond}])); mix_mul[{n}] = load_mix_pow({exp}u);"
                )
                .unwrap();
            }
            (PolyExtStep::AndCond(chain, cond, inner), FieldMode::Ext) => {
                let n = self.mix_idx;
                self.mix_idx += 1;
                let exp = self.mix_exps[n];
                writeln!(
                    self.body,
                    "  // [{idx}] mix[{n}] = AndCond(chain=mix[{chain}], cond=fp[{cond}], inner=mix[{inner}])"
                )
                .unwrap();
                // Ext-field cond: full ext_mul.
                writeln!(
                    self.body,
                    "  mix_tot[{n}] = ext_add(mix_tot[{chain}], ext_mul(ext_mul(mix_tot[{inner}], mix_mul[{chain}]), fp[{cond}])); mix_mul[{n}] = load_mix_pow({exp}u);"
                )
                .unwrap();
            }
        }
        Ok(())
    }

    fn finalize(
        mut self,
        name: &str,
        fp_expected: usize,
        mix_expected: usize,
        ret: usize,
    ) -> StagedKernel {
        // Header comment + main entry point. Bindings, params, and arithmetic
        // helpers come from the runtime prelude shared with the interpreter.
        let mut wgsl = String::new();
        writeln!(wgsl, "// staged eval_check kernel: {name}").unwrap();
        writeln!(
            wgsl,
            "// field_mode = {:?}, fp_slots = {fp_expected}, mix_slots = {mix_expected}, ret = mix[{ret}]",
            self.field_mode
        )
        .unwrap();
        wgsl.push_str(
            "// runtime prelude (bindings, params, ext_*/add/sub/mul helpers,\n// read_tap_*, read_global_*, load_mix_pow, write_check) provided by webgpu.rs\n",
        );
        writeln!(wgsl, "@compute @workgroup_size(64)").unwrap();
        wgsl.push_str("fn main(@builtin(global_invocation_id) gid: vec3<u32>) {\n");
        wgsl.push_str("  let cycle = gid.x;\n");
        wgsl.push_str("  if (cycle >= params.domain) { return; }\n");
        writeln!(
            wgsl,
            "  var fp: array<{}, {fp_expected}>;",
            self.field_mode.fp_ty()
        )
        .unwrap();
        writeln!(wgsl, "  var mix_tot: array<vec4<u32>, {mix_expected}>;").unwrap();
        writeln!(wgsl, "  var mix_mul: array<vec4<u32>, {mix_expected}>;").unwrap();
        wgsl.push_str(&self.body);
        // Write the final ret value to the check buffer, scaled by the cycle's zerofier_inv.
        writeln!(wgsl, "  write_check(cycle, mix_tot[{ret}]);").unwrap();
        wgsl.push_str("}\n");
        self.body.clear();
        StagedKernel {
            id: name.to_string(),
            field_mode: self.field_mode,
            fp_slots: fp_expected,
            mix_slots: mix_expected,
            wgsl_source: wgsl,
        }
    }
}

/// Emit a staged WGSL kernel for an arbitrary `PolyExtStepDef` in the
/// specified `field_mode`. The result's `wgsl_source` is the kernel body
/// with helper references; the runtime prelude (bindings, params,
/// arithmetic helpers, read/write helpers) is contributed by `webgpu.rs`
/// when the kernel is linked into a pipeline.
///
/// Returns `Err(CodegenError::ConstExtInBaseField)` when the DEF requires
/// extension constants but the caller requested base-field mode.
pub fn staged_kernel_from_def_with_mode(
    name: &str,
    def: &PolyExtStepDef,
    field_mode: FieldMode,
) -> Result<StagedKernel, CodegenError> {
    let (fp_expected, mix_expected) = def_var_counts(def);
    let mut emitter = WgslEmitter::new(def, field_mode);
    for (idx, op) in def.block.iter().enumerate() {
        emitter.emit(idx, op)?;
    }
    debug_assert_eq!(
        emitter.fp_idx, fp_expected,
        "emitted fp slot count must equal PolyExtStepDef::fp_expected()"
    );
    debug_assert_eq!(
        emitter.mix_idx, mix_expected,
        "emitted mix slot count must equal PolyExtStepDef::mix_expected()"
    );
    Ok(emitter.finalize(name, fp_expected, mix_expected, def.ret))
}

/// Convenience: pick `FieldMode::Base` when possible, else `FieldMode::Ext`.
pub fn staged_kernel_from_def(name: &str, def: &PolyExtStepDef) -> StagedKernel {
    let mode = if def_is_base_field(def) {
        FieldMode::Base
    } else {
        FieldMode::Ext
    };
    staged_kernel_from_def_with_mode(name, def, mode)
        .expect("auto-mode never returns ConstExtInBaseField")
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
    field_mode: FieldMode,
) -> Result<String, CodegenError> {
    let body_kernel = staged_kernel_from_def_with_mode(name, def, field_mode)?;
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
            staged_kernel_from_def_with_mode("tiny_base", &TINY_DEF, FieldMode::Base).unwrap();
        assert_eq!(kernel.field_mode, FieldMode::Base);
        // fp array declared as u32 scalars, not vec4.
        assert!(kernel.wgsl_source.contains("var fp: array<u32, 3>"));
        // Const emits a scalar literal, not a vec4.
        assert!(kernel.wgsl_source.contains("fp[0] = 7u;"));
        assert!(kernel.wgsl_source.contains("fp[1] = 3u;"));
        // Add uses the scalar `add`, not `ext_add`.
        assert!(kernel.wgsl_source.contains("fp[2] = add(fp[0], fp[1]);"));
        assert!(!kernel.wgsl_source.contains("fp[2] = ext_add"));
        // AndEqz combines scalar `fp[inner]` with vec4 `mix_mul[chain]` via ext_scale.
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
            staged_kernel_from_def_with_mode("full_ext", &FULL_DEF, FieldMode::Ext).unwrap();
        assert_eq!(kernel.field_mode, FieldMode::Ext);
        // fp array declared as vec4<u32>.
        assert!(kernel.wgsl_source.contains("var fp: array<vec4<u32>, 7>"));
        // ConstExt emits a vec4.
        assert!(
            kernel
                .wgsl_source
                .contains("fp[1] = vec4<u32>(1u, 2u, 3u, 4u);")
        );
        // Add/Sub/Mul all use ext_* helpers.
        assert!(kernel.wgsl_source.contains("fp[4] = ext_add(fp[0], fp[2]);"));
        assert!(kernel.wgsl_source.contains("fp[5] = ext_sub(fp[4], fp[1]);"));
        assert!(kernel.wgsl_source.contains("fp[6] = ext_mul(fp[5], fp[3]);"));
        // AndEqz under Ext uses full ext_mul (not ext_scale).
        assert!(
            kernel
                .wgsl_source
                .contains("ext_mul(mix_mul[0], fp[6])"),
            "AndEqz under Ext mode must use ext_mul (vec4 * vec4)"
        );
    }

    #[test]
    fn base_field_rejects_const_ext() {
        let err = staged_kernel_from_def_with_mode("full_base", &FULL_DEF, FieldMode::Base);
        assert_eq!(err, Err(CodegenError::ConstExtInBaseField));
    }

    #[test]
    fn auto_mode_picks_base_for_const_only_def_and_ext_for_const_ext_def() {
        let tiny = staged_kernel_from_def("tiny_auto", &TINY_DEF);
        assert_eq!(tiny.field_mode, FieldMode::Base);
        let full = staged_kernel_from_def("full_auto", &FULL_DEF);
        assert_eq!(full.field_mode, FieldMode::Ext);
    }

    #[test]
    fn read_tap_and_global_helpers_specialize_per_mode() {
        let base =
            staged_kernel_from_def_with_mode("base", &TINY_DEF, FieldMode::Base).unwrap();
        // TINY_DEF has no Get/GetGlobal, so neither helper is referenced.
        assert!(!base.wgsl_source.contains("read_tap_scalar"));
        assert!(!base.wgsl_source.contains("read_tap_ext"));

        // Construct a def that uses Get + GetGlobal under Base mode.
        static BASE_GET_BLOCK: &[PolyExtStep] = &[
            PolyExtStep::Get(3),
            PolyExtStep::GetGlobal(1, 4),
            PolyExtStep::True,
        ];
        static BASE_GET_DEF: PolyExtStepDef = PolyExtStepDef {
            block: BASE_GET_BLOCK,
            ret: 0,
        };
        let base_get =
            staged_kernel_from_def_with_mode("base_get", &BASE_GET_DEF, FieldMode::Base).unwrap();
        assert!(base_get.wgsl_source.contains("read_tap_scalar(3u, cycle)"));
        assert!(base_get.wgsl_source.contains("read_global_scalar(1u, 4u)"));
        assert!(!base_get.wgsl_source.contains("read_tap_ext"));
        assert!(!base_get.wgsl_source.contains("read_global_ext"));

        let ext_get =
            staged_kernel_from_def_with_mode("ext_get", &BASE_GET_DEF, FieldMode::Ext).unwrap();
        assert!(ext_get.wgsl_source.contains("read_tap_ext(3u, cycle)"));
        assert!(ext_get.wgsl_source.contains("read_global_ext(1u, 4u)"));
    }

    #[test]
    fn staged_kernel_emits_one_line_per_step_with_op_comment() {
        let kernel = staged_kernel_from_def("tiny", &TINY_DEF);
        for (idx, op) in TINY_DEF.block.iter().enumerate() {
            let op_label = match op {
                PolyExtStep::Const(v) => format!("Const({v})"),
                PolyExtStep::Add(x, y) => format!("Add(fp[{x}], fp[{y}])"),
                PolyExtStep::True => "True".to_string(),
                PolyExtStep::AndEqz(c, i) => format!("AndEqz(chain=mix[{c}], inner=fp[{i}])"),
                _ => unreachable!(),
            };
            let marker = format!("[{idx}]");
            assert!(kernel.wgsl_source.contains(&marker));
            assert!(kernel.wgsl_source.contains(&op_label));
        }
    }

    #[test]
    fn staged_kernel_writes_ret_mix_tot_to_check_buffer() {
        let kernel = staged_kernel_from_def("tiny", &TINY_DEF);
        assert!(
            kernel.wgsl_source.contains("write_check(cycle, mix_tot[1]);"),
            "ret = 1 means we write mix_tot[1] to the check buffer at cycle"
        );
    }

    #[test]
    fn ext_mode_uses_ext_helpers_for_all_arithmetic_ops_for_full_def() {
        let kernel = staged_kernel_from_def_with_mode("full", &FULL_DEF, FieldMode::Ext).unwrap();
        for token in ["ext_add(", "ext_sub(", "ext_mul(", "read_tap_ext(", "read_global_ext(", "load_mix_pow("] {
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
        // Tap / global readers.
        assert!(prelude.contains("fn read_tap_scalar(tap: u32, cycle: u32) -> u32"));
        assert!(prelude.contains("fn read_tap_ext(tap: u32, cycle: u32) -> vec4<u32>"));
        assert!(prelude.contains("fn read_global_scalar(arg: u32, offset: u32) -> u32"));
        assert!(prelude.contains("fn read_global_ext(arg: u32, offset: u32) -> vec4<u32>"));
        // Mix-power loader and check writer.
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
        let full =
            staged_full_kernel_wgsl("tiny", &TINY_DEF, FieldMode::Base).expect("Base ok for TINY_DEF");
        // Prelude appears first.
        let prelude_anchor = "fn write_check(cycle: u32, val: vec4<u32>)";
        let body_anchor = "@compute @workgroup_size(64)";
        let prelude_pos = full.find(prelude_anchor).expect("prelude present");
        let body_pos = full.find(body_anchor).expect("emitter body present");
        assert!(
            prelude_pos < body_pos,
            "prelude must precede emitter body"
        );
        // Body uses the scalar `add` helper from the prelude.
        assert!(full.contains("fp[2] = add(fp[0], fp[1]);"));
    }

    #[test]
    fn full_kernel_rejects_const_ext_in_base_mode_via_codegen_error() {
        let err = staged_full_kernel_wgsl("full_base", &FULL_DEF, FieldMode::Base);
        assert_eq!(err, Err(CodegenError::ConstExtInBaseField));
    }

    #[test]
    fn full_kernel_assembles_for_ext_field_def() {
        let full = staged_full_kernel_wgsl("full_ext", &FULL_DEF, FieldMode::Ext)
            .expect("Ext mode accepts ConstExt");
        // Prelude helpers + body usage both present.
        assert!(full.contains("fn ext_mul(lhs: vec4<u32>, rhs: vec4<u32>)"));
        assert!(full.contains("fp[6] = ext_mul(fp[5], fp[3]);"));
        assert!(full.contains("@compute @workgroup_size(64)"));
        assert!(full.contains("write_check(cycle, mix_tot[2]);"));
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
        let kernel = staged_kernel_from_def("tiny", &TINY_DEF);
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
