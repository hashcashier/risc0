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
//! SP2 foundation per `/.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`.
//! This module is intentionally minimal in the seed iteration: it locks in the
//! `StagedKernel` shape, a tiny `rv32im_codegen_for_po2` placeholder, and a
//! RED→GREEN unit-test discipline for the WGSL string output, **without
//! wiring into the production prove path**. SP3 extends to production rv32im
//! domains; SP5 extends to recursion; SP6 extends to keccak.
//!
//! The reason this lives as a sibling module (`webgpu_codegen`) rather than
//! under `webgpu/eval_check_codegen/` as Phase 2 originally planned is that
//! `webgpu.rs` is still a single file; refactoring it into a directory is
//! out of scope for SP2's seed iteration and would risk the locked Phase 0–8
//! artifact set. Future iterations that perform that refactor MAY move this
//! module accordingly.

use alloc::{format, string::String};

/// Description of one staged WGSL kernel produced by a circuit-specific
/// generator. SP3+ will extend this with bind-group layouts and explicit
/// intermediate-buffer descriptions; SP2 keeps it minimal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedKernel {
    /// Human-readable identifier for the kernel within a circuit's staged set.
    pub id: String,
    /// Number of FP slots this kernel uses (for workgroup-storage budgeting).
    pub fp_slots: usize,
    /// Number of mix slots this kernel uses.
    pub mix_slots: usize,
    /// The WGSL source code for this kernel.
    pub wgsl_source: String,
}

impl StagedKernel {
    /// Trivial placeholder kernel: returns extension-field zero unconditionally.
    /// Useful as a sanity-check that the codegen pipeline compiles and runs;
    /// not yet a real eval_check kernel.
    fn trivial_zero(id: &str) -> Self {
        Self {
            id: String::from(id),
            fp_slots: 0,
            mix_slots: 0,
            wgsl_source: String::from(
                "// trivial_zero: returns FpExt zero
@compute @workgroup_size(1)
fn main() {}
",
            ),
        }
    }
}

/// rv32im staged-WGSL `eval_check` generator (SP2 seed).
///
/// For SP2's tiny prototype scope, this returns a single trivial placeholder
/// kernel for `po2 = 0..=3`. SP3 will extend to production-shape domains
/// (`po2 = 16` segment-sized, `po2 = 21` production-shaped) and the generator
/// will then consume `risc0_circuit_rv32im::zirgen::poly_ext::DEF` to emit
/// staged kernels mirroring the 4-file CUDA `eval_check_{0,1,2,3}.cu` layout.
pub fn rv32im_codegen_for_po2(po2: u32) -> Vec<StagedKernel> {
    if po2 > 3 {
        // SP2 scope limit: tiny po2 only. Production-shape support is SP3.
        return Vec::new();
    }
    vec![StagedKernel::trivial_zero(&format!("rv32im_po2_{po2}"))]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that the SP2 seed generator produces a single staged kernel
    /// for tiny `po2` values and an empty set for out-of-SP2-scope values.
    /// This is the GREEN side of the SP2 codegen-skeleton RED→GREEN cycle.
    #[test]
    fn rv32im_codegen_returns_one_kernel_for_tiny_po2() {
        for po2 in 0..=3u32 {
            let kernels = rv32im_codegen_for_po2(po2);
            assert_eq!(
                kernels.len(),
                1,
                "SP2 seed must return exactly one kernel for po2={po2}"
            );
            assert_eq!(
                kernels[0].id,
                format!("rv32im_po2_{po2}"),
                "kernel id encodes po2"
            );
            assert!(
                kernels[0].wgsl_source.contains("@compute"),
                "WGSL source must declare a compute entry point"
            );
            assert_eq!(kernels[0].fp_slots, 0, "SP2 seed kernel uses no FP slots");
            assert_eq!(kernels[0].mix_slots, 0, "SP2 seed kernel uses no mix slots");
        }
    }

    /// Verifies that out-of-SP2-scope `po2` values return no kernels. SP3+
    /// will replace this assertion with a non-empty staged set; until then
    /// callers MUST handle the empty case (the production prove path falls
    /// back to the existing interpreter).
    #[test]
    fn rv32im_codegen_returns_empty_for_production_po2() {
        for po2 in [4u32, 16, 21] {
            let kernels = rv32im_codegen_for_po2(po2);
            assert!(
                kernels.is_empty(),
                "SP2 scope: po2={po2} must return empty until SP3 lands"
            );
        }
    }

    /// SP3+ will add a parity test comparing this generator's WGSL output
    /// against `risc0_zkp::hal::portable::eval_check` for a tiny PolyExt
    /// program. That test belongs in this module so SP3's parity discipline
    /// lives alongside the generator under TDD.
    #[test]
    #[ignore = "SP3 parity test not yet implemented; SP2 seed validates structure only"]
    fn rv32im_codegen_matches_portable_for_tiny_circuit() {
        // GREEN target for SP3:
        //   1. Construct a tiny PolyExtStepDef (e.g., Const(0) -> ret).
        //   2. Generate WGSL via rv32im_codegen_for_po2 at po2=0.
        //   3. Compile + dispatch via WebGpuHal at po2=0 domain.
        //   4. Compute portable::eval_check on the same DEF + inputs.
        //   5. Assert byte-equivalent FpExt outputs.
        //
        // This test is intentionally `#[ignore]` until the parity dispatch
        // helper lands. It is committed RED so the SP3 contributor can
        // observe the failure on `--ignored` removal.
        unimplemented!("SP3 parity test")
    }
}
