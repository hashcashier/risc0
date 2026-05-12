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
//! Walks a `PolyExtStepDef` and emits straight-line WGSL that performs the
//! same FpExt / MixState arithmetic as `risc0_zkp::adapter::PolyExtExecutor`
//! does on the CPU. The emitted body uses the same `vec4<u32>` BabyBearExt
//! encoding and the same `ext_add` / `ext_sub` / `ext_mul` helpers as the
//! existing runtime interpreter in `webgpu.rs`, so a generated kernel can
//! reuse the interpreter's prelude (bindings, params, helpers) and only
//! differ in the body.
//!
//! Scope per `02-to-be-plan.md`:
//! - SP2 (seed): module exists, RED-marked parity test, no real emission.
//! - SP3 (this iteration): real emitter for arbitrary `PolyExtStepDef`,
//!   structural parity tests, not yet wired into the prove path's fast-path.
//! - SP3 follow-on: wire into `dispatch_eval_check_poly_ext`, multi-stage
//!   split for the production-shape rv32im DEF (currently ~20k ops), browser
//!   runtime-parity test, `cpu_fallbacks=0` assertion for poseidon2_basic.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::Write as _;

use crate::adapter::{PolyExtStep, PolyExtStepDef};

/// Description of one staged WGSL kernel produced by a circuit-specific
/// generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedKernel {
    /// Human-readable identifier for the kernel within a circuit's staged set.
    pub id: String,
    /// Number of FP locals (BabyBearExt vec4<u32>) the body uses.
    pub fp_slots: usize,
    /// Number of MixState locals the body uses.
    pub mix_slots: usize,
    /// The WGSL source code for this kernel's *body*. The runtime prelude
    /// (bindings, params, `ext_*` helpers) is provided by `webgpu.rs` and
    /// concatenated by the dispatch wiring (SP3 follow-on).
    pub wgsl_source: String,
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

struct WgslEmitter {
    body: String,
    fp_idx: usize,
    mix_idx: usize,
    /// Per-mix-var, the exponent that selects `mix_pows[exp]` for the new
    /// `mul` field. Filled before emission.
    mix_exps: Vec<usize>,
}

impl WgslEmitter {
    fn new(def: &PolyExtStepDef) -> Self {
        Self {
            body: String::new(),
            fp_idx: 0,
            mix_idx: 0,
            mix_exps: def_mix_exponents(def),
        }
    }

    fn emit(&mut self, idx: usize, op: &PolyExtStep) {
        match op {
            PolyExtStep::Const(v) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Const({v})").unwrap();
                writeln!(
                    self.body,
                    "  fp[{n}] = vec4<u32>({v}u, 0u, 0u, 0u);"
                )
                .unwrap();
            }
            PolyExtStep::ConstExt(a, b, c, d) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(
                    self.body,
                    "  // [{idx}] fp[{n}] = ConstExt({a}, {b}, {c}, {d})"
                )
                .unwrap();
                writeln!(
                    self.body,
                    "  fp[{n}] = vec4<u32>({a}u, {b}u, {c}u, {d}u);"
                )
                .unwrap();
            }
            PolyExtStep::Get(tap) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Get(tap={tap})").unwrap();
                writeln!(self.body, "  fp[{n}] = read_tap({tap}u, idx);").unwrap();
            }
            PolyExtStep::GetGlobal(arg, off) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(
                    self.body,
                    "  // [{idx}] fp[{n}] = GetGlobal(arg={arg}, off={off})"
                )
                .unwrap();
                writeln!(
                    self.body,
                    "  fp[{n}] = read_global({arg}u, {off}u);"
                )
                .unwrap();
            }
            PolyExtStep::Add(x, y) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Add(fp[{x}], fp[{y}])").unwrap();
                writeln!(self.body, "  fp[{n}] = ext_add(fp[{x}], fp[{y}]);").unwrap();
            }
            PolyExtStep::Sub(x, y) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Sub(fp[{x}], fp[{y}])").unwrap();
                writeln!(self.body, "  fp[{n}] = ext_sub(fp[{x}], fp[{y}]);").unwrap();
            }
            PolyExtStep::Mul(x, y) => {
                let n = self.fp_idx;
                self.fp_idx += 1;
                writeln!(self.body, "  // [{idx}] fp[{n}] = Mul(fp[{x}], fp[{y}])").unwrap();
                writeln!(self.body, "  fp[{n}] = ext_mul(fp[{x}], fp[{y}]);").unwrap();
            }
            PolyExtStep::True => {
                let n = self.mix_idx;
                self.mix_idx += 1;
                let exp = self.mix_exps[n];
                writeln!(self.body, "  // [{idx}] mix[{n}] = True (mix_pow exp={exp})").unwrap();
                writeln!(
                    self.body,
                    "  mix_tot[{n}] = vec4<u32>(0u, 0u, 0u, 0u); mix_mul[{n}] = read_mix_pow({exp}u);"
                )
                .unwrap();
            }
            PolyExtStep::AndEqz(chain, inner) => {
                let n = self.mix_idx;
                self.mix_idx += 1;
                let exp = self.mix_exps[n];
                writeln!(
                    self.body,
                    "  // [{idx}] mix[{n}] = AndEqz(chain=mix[{chain}], inner=fp[{inner}])"
                )
                .unwrap();
                writeln!(
                    self.body,
                    "  mix_tot[{n}] = ext_add(mix_tot[{chain}], ext_mul(mix_mul[{chain}], fp[{inner}])); mix_mul[{n}] = read_mix_pow({exp}u);"
                )
                .unwrap();
            }
            PolyExtStep::AndCond(chain, cond, inner) => {
                let n = self.mix_idx;
                self.mix_idx += 1;
                let exp = self.mix_exps[n];
                writeln!(
                    self.body,
                    "  // [{idx}] mix[{n}] = AndCond(chain=mix[{chain}], cond=fp[{cond}], inner=mix[{inner}])"
                )
                .unwrap();
                writeln!(
                    self.body,
                    "  mix_tot[{n}] = ext_add(mix_tot[{chain}], ext_mul(ext_mul(fp[{cond}], mix_tot[{inner}]), mix_mul[{chain}])); mix_mul[{n}] = read_mix_pow({exp}u);"
                )
                .unwrap();
            }
        }
    }

    fn finalize(mut self, name: &str, fp_expected: usize, mix_expected: usize, ret: usize) -> StagedKernel {
        // Header comment + main entry point. Bindings, params, and `ext_*`
        // helpers come from the runtime prelude shared with the interpreter.
        let mut wgsl = String::new();
        writeln!(wgsl, "// staged eval_check kernel: {name}").unwrap();
        writeln!(
            wgsl,
            "// fp_slots = {fp_expected}, mix_slots = {mix_expected}, ret = mix[{ret}]"
        )
        .unwrap();
        wgsl.push_str("// runtime prelude (bindings, params, ext_* helpers) provided by webgpu.rs\n");
        writeln!(wgsl, "@compute @workgroup_size(64)").unwrap();
        wgsl.push_str("fn main(@builtin(global_invocation_id) gid: vec3<u32>) {\n");
        wgsl.push_str("  let idx = gid.x;\n");
        wgsl.push_str("  if (idx >= params.domain) { return; }\n");
        writeln!(wgsl, "  var fp: array<vec4<u32>, {fp_expected}>;").unwrap();
        writeln!(wgsl, "  var mix_tot: array<vec4<u32>, {mix_expected}>;").unwrap();
        writeln!(wgsl, "  var mix_mul: array<vec4<u32>, {mix_expected}>;").unwrap();
        wgsl.push_str(&self.body);
        // Write the final ret value to the check buffer.
        writeln!(
            wgsl,
            "  write_check(idx, mix_tot[{ret}]);"
        )
        .unwrap();
        wgsl.push_str("}\n");
        // Clear body since we've consumed it.
        self.body.clear();
        StagedKernel {
            id: name.to_string(),
            fp_slots: fp_expected,
            mix_slots: mix_expected,
            wgsl_source: wgsl,
        }
    }
}

/// Emit a staged WGSL kernel for an arbitrary `PolyExtStepDef`. The result's
/// `wgsl_source` is the kernel body with helper references; the runtime
/// prelude (bindings, params, `ext_*` arithmetic helpers, `read_tap`,
/// `read_global`, `read_mix_pow`, `write_check`) is contributed by
/// `webgpu.rs` when the kernel is linked into a pipeline.
pub fn staged_kernel_from_def(name: &str, def: &PolyExtStepDef) -> StagedKernel {
    let (fp_expected, mix_expected) = def_var_counts(def);
    let mut emitter = WgslEmitter::new(def);
    for (idx, op) in def.block.iter().enumerate() {
        emitter.emit(idx, op);
    }
    debug_assert_eq!(
        emitter.fp_idx, fp_expected,
        "emitted fp slot count must equal PolyExtStepDef::fp_expected()"
    );
    debug_assert_eq!(
        emitter.mix_idx, mix_expected,
        "emitted mix slot count must equal PolyExtStepDef::mix_expected()"
    );
    emitter.finalize(name, fp_expected, mix_expected, def.ret)
}

/// rv32im staged-WGSL `eval_check` generator.
///
/// At SP3 first iteration this is a thin scaffold: the rv32im production DEF
/// is ~20k ops and emitting one straight-line kernel may exceed WGSL shader
/// length / compile-time budgets in Chrome. Production wiring + multi-stage
/// split (mirroring the 4-file CUDA `eval_check_{0,1,2,3}.cu` layout) is the
/// next SP3 commit. For now this returns an empty set for all po2; the
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

    /// Tiny PolyExt program exercising every PolyExtStep variant once.
    static FULL_BLOCK: &[PolyExtStep] = &[
        PolyExtStep::Const(2),                  // f0
        PolyExtStep::ConstExt(1, 2, 3, 4),      // f1
        PolyExtStep::Get(5),                    // f2 (tap 5)
        PolyExtStep::GetGlobal(0, 7),           // f3 (global 0, offset 7)
        PolyExtStep::Add(0, 2),                 // f4 = f0 + f2
        PolyExtStep::Sub(4, 1),                 // f5 = f4 - f1
        PolyExtStep::Mul(5, 3),                 // f6 = f5 * f3
        PolyExtStep::True,                      // m0
        PolyExtStep::AndEqz(0, 6),              // m1 = AndEqz(m0, f6)
        PolyExtStep::AndCond(1, 0, 1),          // m2 = AndCond(m1, f0, m1)
    ];

    static FULL_DEF: PolyExtStepDef = PolyExtStepDef {
        block: FULL_BLOCK,
        ret: 2,
    };

    #[test]
    fn tiny_def_counts_match_polyext_expected() {
        let (fp, mix) = def_var_counts(&TINY_DEF);
        assert_eq!(fp, 3, "fp count = block.len() - (ret+1) = 5 - 2 = 3");
        assert_eq!(mix, 2, "mix count = ret + 1 = 2");
    }

    #[test]
    fn tiny_def_mix_exponents_match_executor() {
        let exps = def_mix_exponents(&TINY_DEF);
        assert_eq!(exps, vec![0, 1], "True -> exp 0; AndEqz(True, _) -> exp 1");
    }

    #[test]
    fn full_def_mix_exponents_match_executor_andcond() {
        let exps = def_mix_exponents(&FULL_DEF);
        assert_eq!(
            exps,
            vec![0, 1, 2],
            "True -> exp 0; AndEqz(True, _) -> exp 1; AndCond(m1, _, m1) -> exp 1 + exp 1 = 2"
        );
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
                _ => unreachable!("TINY_BLOCK uses only these variants"),
            };
            let marker = format!("[{idx}]");
            assert!(
                kernel.wgsl_source.contains(&marker),
                "every step gets a [{idx}] comment in WGSL"
            );
            assert!(
                kernel.wgsl_source.contains(&op_label),
                "step {idx} ({op:?}) labeled in WGSL: missing '{op_label}'"
            );
        }
    }

    #[test]
    fn staged_kernel_declares_slot_arrays_with_correct_size() {
        let kernel = staged_kernel_from_def("tiny", &TINY_DEF);
        assert_eq!(kernel.fp_slots, 3);
        assert_eq!(kernel.mix_slots, 2);
        assert!(
            kernel.wgsl_source.contains("var fp: array<vec4<u32>, 3>"),
            "fp local array sized to fp_slots"
        );
        assert!(
            kernel.wgsl_source.contains("var mix_tot: array<vec4<u32>, 2>"),
            "mix_tot local array sized to mix_slots"
        );
        assert!(
            kernel.wgsl_source.contains("var mix_mul: array<vec4<u32>, 2>"),
            "mix_mul local array sized to mix_slots"
        );
    }

    #[test]
    fn staged_kernel_writes_ret_mix_tot_to_check_buffer() {
        let kernel = staged_kernel_from_def("tiny", &TINY_DEF);
        assert!(
            kernel.wgsl_source.contains("write_check(idx, mix_tot[1]);"),
            "ret = 1 means we write mix_tot[1] to the check buffer at idx"
        );
    }

    #[test]
    fn staged_kernel_emits_all_arithmetic_ops_for_full_def() {
        let kernel = staged_kernel_from_def("full", &FULL_DEF);
        for token in [
            "ext_add(",
            "ext_sub(",
            "ext_mul(",
            "read_tap(",
            "read_global(",
            "read_mix_pow(",
        ] {
            assert!(
                kernel.wgsl_source.contains(token),
                "FULL_DEF must emit '{token}' at least once"
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

    /// Structural-parity check: the generated kernel's slot layout and
    /// emitted ops mirror `PolyExtStepDef`'s declared fp_expected /
    /// mix_expected / mix_exponents exactly. Runtime byte-equivalent parity
    /// against `risc0_zkp::adapter::PolyExtStepDef::step` requires a real
    /// WebGPU device and lives in `examples/browser-prove/src/lib.rs` once
    /// SP3 wiring lands.
    #[test]
    fn structural_parity_matches_polyext_for_tiny_def() {
        let (fp_expected, mix_expected) = def_var_counts(&TINY_DEF);
        let exps = def_mix_exponents(&TINY_DEF);
        let kernel = staged_kernel_from_def("tiny", &TINY_DEF);
        assert_eq!(kernel.fp_slots, fp_expected);
        assert_eq!(kernel.mix_slots, mix_expected);
        for (n, exp) in exps.iter().enumerate() {
            assert!(
                kernel
                    .wgsl_source
                    .contains(&format!("mix_mul[{n}] = read_mix_pow({exp}u)")),
                "mix var {n} must read mix_pow at exponent {exp}"
            );
        }
    }
}
