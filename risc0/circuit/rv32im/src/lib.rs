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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "execute")]
pub mod execute;
#[cfg(feature = "prove")]
pub mod prove;
pub mod trace;
mod zirgen;

use core::num::TryFromIntError;

use anyhow::{anyhow, ensure, Result};
use derive_more::Debug;
use risc0_binfmt::PovwNonce;
use risc0_zkp::{
    adapter::CircuitInfo as _,
    core::{digest::Digest, hash::poseidon2::Poseidon2HashSuite},
    layout::Tree,
    verify::VerificationError,
};
use serde::{Deserialize, Serialize};

use self::zirgen::circuit::{Val, LAYOUT_GLOBAL};

pub use self::zirgen::CircuitImpl;

// NOTE: Seal version two introduced with PoVW, changing the output size from 74 to 90.
pub const RV32IM_SEAL_VERSION: u32 = 2;

/// This number was picked by running `bigint2-analyze` on all the current bigint programs
pub const MAX_INSN_CYCLES: usize = 25_000;

/// This is a smaller number used by lower po2's < 15 which can't fit a large bigint program.
pub const MAX_INSN_CYCLES_LOWER_PO2: usize = 2_000;

pub fn verify(seal: &[u32]) -> Result<(), VerificationError> {
    tracing::debug!("verify");

    // We don't have a `code' buffer to verify.
    let check_code_fn = |_: u32, _: &Digest| Ok(());

    if seal[0] != RV32IM_SEAL_VERSION {
        return Err(VerificationError::ReceiptFormatError);
    }

    let seal = &seal[1..];

    let hash_suite = Poseidon2HashSuite::new_suite();
    risc0_zkp::verify::verify(&CircuitImpl, &hash_suite, seal, check_code_fn)
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HighLowU16(pub u16, pub u16);

impl From<HighLowU16> for u32 {
    fn from(x: HighLowU16) -> Self {
        ((x.0 as u32) << 16) | (x.1 as u32)
    }
}

impl From<u32> for HighLowU16 {
    fn from(x: u32) -> Self {
        Self((x >> 16) as u16, (x & 0xffff) as u16)
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TerminateState {
    pub a0: HighLowU16,
    pub a1: HighLowU16,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rv32imV2Claim {
    pub pre_state: Digest,
    pub post_state: Digest,
    pub input: Digest,
    pub output: Option<Digest>,
    pub terminate_state: Option<TerminateState>,
    pub shutdown_cycle: Option<u32>,
}

impl Rv32imV2Claim {
    pub fn decode(segment_seal: &[u32]) -> Result<Rv32imV2Claim> {
        ensure!(
            segment_seal[0] == RV32IM_SEAL_VERSION,
            "seal version mismatch"
        );
        let segment_seal = &segment_seal[1..];

        let io: &[Val] = bytemuck::checked::cast_slice(&segment_seal[..CircuitImpl::OUTPUT_SIZE]);
        let global = Tree::new(io, LAYOUT_GLOBAL);

        // NOTE: rng and povw are not read from the globals here. Neither need to be checked to
        // establish the integrity of the Rv32imV2Claim.
        let pre_state = global.map(|c| c.state_in).get_digest_from_shorts()?;
        let post_state = global.map(|c| c.state_out).get_digest_from_shorts()?;
        let input = global.map(|c| c.input).get_digest_from_shorts()?;
        let output = global.map(|c| c.output).get_digest_from_shorts()?;
        let is_terminate = global.map(|c| c.is_terminate).get_u32_from_elem()?;
        let term_a0_high = global.map(|c| c.term_a0high).get_u32_from_elem()?;
        let term_a0_low = global.map(|c| c.term_a0low).get_u32_from_elem()?;
        let term_a1_high = global.map(|c| c.term_a1high).get_u32_from_elem()?;
        let term_a1_low = global.map(|c| c.term_a1low).get_u32_from_elem()?;
        let shutdown_cycle = global.map(|c| c.shutdown_cycle).get_u32_from_elem()?;

        fn try_as_u16(x: u32) -> Result<u16> {
            x.try_into()
                .map_err(|err: TryFromIntError| anyhow!("{err}"))
        }

        let terminate_state = if is_terminate == 1 {
            Some(TerminateState {
                a0: HighLowU16(try_as_u16(term_a0_high)?, try_as_u16(term_a0_low)?),
                a1: HighLowU16(try_as_u16(term_a1_high)?, try_as_u16(term_a1_low)?),
            })
        } else {
            None
        };

        let output = if is_terminate == 1 {
            Some(output)
        } else {
            None
        };

        Ok(Rv32imV2Claim {
            pre_state,
            post_state,
            input,
            output,
            terminate_state,
            shutdown_cycle: Some(shutdown_cycle),
        })
    }
}

/// Decodes a PoVW nonce from a segment seal.
pub fn decode_povw_nonce(segment_seal: &[u32]) -> Result<PovwNonce> {
    ensure!(
        segment_seal[0] == RV32IM_SEAL_VERSION,
        "seal version mismatch"
    );
    let segment_seal = &segment_seal[1..];

    let io: &[Val] = bytemuck::checked::cast_slice(&segment_seal[..CircuitImpl::OUTPUT_SIZE]);
    let global = Tree::new(io, LAYOUT_GLOBAL);

    let povw_nonce_shorts_vec = global.map(|c| c.povw_nonce).get_shorts()?;
    let povw_nonce_shorts_arr = povw_nonce_shorts_vec
        .try_into()
        .map_err(|_| anyhow!("povw nonce global has unexpected length"))?;
    Ok(PovwNonce::from_u16s(povw_nonce_shorts_arr))
}

/// Browser WebGPU test/bench helpers that need the crate-private
/// production `TAPSET`/`DEF`. Mirrors the recursion crate's `testutil`
/// and keccak's `webgpu_testutil` so `examples/browser-prove` can run
/// rv32im eval_check parity and focused benchmarks against the same
/// tape the prover uses.
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub mod webgpu_testutil {
    use anyhow::{bail, ensure, Result};
    use risc0_zkp::{
        adapter::{CircuitInfo as _, TapsProvider as _},
        field::{
            baby_bear::{BabyBearElem, BabyBearExtElem},
            ExtElem as _,
        },
        hal::{webgpu::WebGpuHal, Buffer as _, Hal as _},
        INV_RATE,
    };

    use crate::zirgen::circuit::{REGISTER_GROUP_ACCUM, REGISTER_GROUP_CODE, REGISTER_GROUP_DATA};
    use crate::CircuitImpl;

    fn deterministic_fp(seed: usize) -> BabyBearElem {
        BabyBearElem::new((seed as u32).wrapping_mul(0x1f12bb5).wrapping_add(0x12345))
    }

    fn deterministic_ext(seed: usize) -> BabyBearExtElem {
        BabyBearExtElem::new(
            deterministic_fp(seed),
            deterministic_fp(seed + 1),
            deterministic_fp(seed + 2),
            deterministic_fp(seed + 3),
        )
    }

    fn deterministic_fps(size: usize, seed: usize) -> Vec<BabyBearElem> {
        (0..size).map(|idx| deterministic_fp(seed + idx)).collect()
    }

    pub async fn eval_check_webgpu_matches_portable(hal: &WebGpuHal, po2: usize) -> Result<()> {
        let steps = 1 << po2;
        let domain = steps * INV_RATE;
        let circuit = CircuitImpl;
        let taps = circuit.get_taps();
        let mut groups = [None, None, None];
        for (group_id, name, seed) in [
            (REGISTER_GROUP_ACCUM, "rv32im_eval_check_accum", 1000),
            (REGISTER_GROUP_CODE, "rv32im_eval_check_code", 2000),
            (REGISTER_GROUP_DATA, "rv32im_eval_check_data", 3000),
        ] {
            groups[group_id] = Some(hal.copy_from_elem(
                name,
                &deterministic_fps(taps.group_size(group_id) * domain, seed),
            ));
        }
        let groups: Vec<_> = groups.into_iter().map(Option::unwrap).collect();
        let groups: Vec<&_> = groups.iter().collect();
        let mix = hal.copy_from_elem(
            "rv32im_eval_check_mix",
            &deterministic_fps(CircuitImpl::MIX_SIZE, 4000),
        );
        let out = hal.copy_from_elem(
            "rv32im_eval_check_out",
            &deterministic_fps(CircuitImpl::OUTPUT_SIZE, 5000),
        );
        let poly_mix = deterministic_ext(6000);

        let expected = hal.alloc_elem(
            "rv32im_eval_check_expected",
            BabyBearExtElem::EXT_SIZE * domain,
        );
        risc0_zkp::hal::portable::eval_check::<WebGpuHal, CircuitImpl>(
            &CircuitImpl,
            &expected,
            groups.as_slice(),
            &[&mix, &out],
            poly_mix,
            po2,
            steps,
        );

        let actual = hal.alloc_elem(
            "rv32im_eval_check_actual",
            BabyBearExtElem::EXT_SIZE * domain,
        );
        let dispatched = hal.dispatch_eval_check_poly_ext(
            &actual,
            groups.as_slice(),
            &[&mix, &out],
            crate::zirgen::taps::TAPSET,
            &crate::zirgen::poly_ext::DEF,
            poly_mix,
            po2,
            steps,
        )?;
        ensure!(dispatched, "rv32im eval_check did not dispatch on WebGPU");

        actual.sync_gpu_to_cpu(hal).await?;
        let actual_values = actual.to_vec();
        let expected_values = expected.to_vec();
        if let Some((idx, (actual, expected))) = actual_values
            .iter()
            .zip(expected_values.iter())
            .enumerate()
            .find(|(_, (actual, expected))| actual != expected)
        {
            bail!(
                "rv32im eval_check WebGPU output differed from portable output at index {idx}: actual={actual:?} expected={expected:?}"
            );
        }
        ensure!(
            actual_values == expected_values,
            "rv32im eval_check WebGPU output differed from portable output"
        );
        Ok(())
    }

    /// Times `reps` full eval_check dispatches (dispatch + queue drain)
    /// at the given `po2` using zero-filled group/global buffers.
    /// eval_check arithmetic is data-independent, so zero inputs give
    /// production-representative timing without generating ~1 GiB of
    /// deterministic data on the wasm CPU. Returns per-rep wall ms.
    pub async fn eval_check_webgpu_bench(
        hal: &WebGpuHal,
        po2: usize,
        reps: usize,
    ) -> Result<Vec<f64>> {
        let steps = 1 << po2;
        let domain = steps * INV_RATE;
        let circuit = CircuitImpl;
        let taps = circuit.get_taps();
        let accum = hal.alloc_elem(
            "rv32im_eval_check_bench_accum",
            taps.group_size(REGISTER_GROUP_ACCUM) * domain,
        );
        let code = hal.alloc_elem(
            "rv32im_eval_check_bench_code",
            taps.group_size(REGISTER_GROUP_CODE) * domain,
        );
        let data = hal.alloc_elem(
            "rv32im_eval_check_bench_data",
            taps.group_size(REGISTER_GROUP_DATA) * domain,
        );
        let mut groups = [None, None, None];
        groups[REGISTER_GROUP_ACCUM] = Some(&accum);
        groups[REGISTER_GROUP_CODE] = Some(&code);
        groups[REGISTER_GROUP_DATA] = Some(&data);
        let groups: Vec<&_> = groups.into_iter().map(Option::unwrap).collect();
        let mix = hal.alloc_elem("rv32im_eval_check_bench_mix", CircuitImpl::MIX_SIZE);
        let out = hal.alloc_elem("rv32im_eval_check_bench_out", CircuitImpl::OUTPUT_SIZE);
        let poly_mix = deterministic_ext(6000);
        let check = hal.alloc_elem(
            "rv32im_eval_check_bench_check",
            BabyBearExtElem::EXT_SIZE * domain,
        );

        let mut times = Vec::with_capacity(reps);
        for _ in 0..reps {
            hal.wait_idle().await?;
            let t0 = js_sys::Date::now();
            let dispatched = hal.dispatch_eval_check_poly_ext(
                &check,
                groups.as_slice(),
                &[&mix, &out],
                crate::zirgen::taps::TAPSET,
                &crate::zirgen::poly_ext::DEF,
                poly_mix,
                po2,
                steps,
            )?;
            ensure!(
                dispatched,
                "rv32im eval_check bench did not dispatch on WebGPU"
            );
            hal.wait_idle().await?;
            times.push(js_sys::Date::now() - t0);
        }
        Ok(times)
    }
}
