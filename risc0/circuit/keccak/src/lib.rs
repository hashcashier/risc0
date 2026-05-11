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

mod control_id;
#[cfg(feature = "prove")]
pub mod prove;
#[cfg(feature = "prove")]
pub(crate) mod zirgen;

use risc0_zkp::core::digest::Digest;

pub use self::control_id::{KECCAK_CONTROL_IDS, KECCAK_CONTROL_ROOT};

pub const KECCAK_DEFAULT_PO2: usize = 17;

pub const KECCAK_PO2_RANGE: core::ops::RangeInclusive<usize> = 14..=18;

pub const RECURSION_PO2: usize = 18;

pub const KECCAK_PERMUTE_CYCLES: usize = 200;

pub type KeccakState = [u64; 25];

pub fn get_control_id(po2: usize) -> &'static Digest {
    assert!(KECCAK_PO2_RANGE.contains(&po2), "po2 {po2} out of range");
    &KECCAK_CONTROL_IDS[po2 - KECCAK_PO2_RANGE.min().unwrap()]
}

pub fn max_keccak_inputs(po2: usize) -> usize {
    let max_keccak_cycles: usize = 1 << po2;
    max_keccak_cycles / KECCAK_PERMUTE_CYCLES
}

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

    use crate::zirgen::{
        circuit::{REGISTER_GROUP_ACCUM, REGISTER_GROUP_CODE, REGISTER_GROUP_DATA},
        CircuitImpl,
    };

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
        let accum = hal.copy_from_elem(
            "keccak_eval_check_accum",
            &deterministic_fps(taps.group_size(REGISTER_GROUP_ACCUM) * domain, 1000),
        );
        let code = hal.copy_from_elem(
            "keccak_eval_check_code",
            &deterministic_fps(taps.group_size(REGISTER_GROUP_CODE) * domain, 2000),
        );
        let data = hal.copy_from_elem(
            "keccak_eval_check_data",
            &deterministic_fps(taps.group_size(REGISTER_GROUP_DATA) * domain, 3000),
        );
        let mix = hal.copy_from_elem(
            "keccak_eval_check_mix",
            &deterministic_fps(CircuitImpl::MIX_SIZE, 4000),
        );
        let out = hal.copy_from_elem(
            "keccak_eval_check_out",
            &deterministic_fps(CircuitImpl::OUTPUT_SIZE, 5000),
        );
        let poly_mix = deterministic_ext(6000);

        let expected = hal.alloc_elem(
            "keccak_eval_check_expected",
            BabyBearExtElem::EXT_SIZE * domain,
        );
        risc0_zkp::hal::portable::eval_check::<WebGpuHal, CircuitImpl>(
            &CircuitImpl,
            &expected,
            &[&accum, &code, &data],
            &[&mix, &out],
            poly_mix,
            po2,
            steps,
        );

        let actual = hal.alloc_elem(
            "keccak_eval_check_actual",
            BabyBearExtElem::EXT_SIZE * domain,
        );
        let dispatched = hal.dispatch_eval_check_poly_ext(
            &actual,
            &[&accum, &code, &data],
            &[&mix, &out],
            crate::zirgen::taps::TAPSET,
            &crate::zirgen::poly_ext::DEF,
            poly_mix,
            po2,
            steps,
        )?;
        ensure!(dispatched, "keccak eval_check did not dispatch on WebGPU");

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
                "keccak eval_check WebGPU output differed from portable output at index {idx}: actual={actual:?} expected={expected:?}"
            );
        }
        ensure!(
            actual_values == expected_values,
            "keccak eval_check WebGPU output differed from portable output"
        );
        Ok(())
    }
}

/// Given a slice of `KeccakState`, encoded as `[u8]`, produce the SHA-256 digest matching what is produced by the keccak circuit.
#[cfg(feature = "prove")]
pub fn compute_keccak_digest(input: &[u8]) -> Digest {
    use risc0_zkp::core::digest::{Digest, DIGEST_BYTES};
    use risc0_zkp::core::hash::{
        sha,
        sha::{Sha256, SHA256_INIT},
    };

    let mut transcript = vec![];

    let mut input: Vec<u8> = input.to_vec();
    let input_states: &mut [KeccakState] = bytemuck::cast_slice_mut(&mut input);
    for input in input_states.iter_mut() {
        let mut data = [0u64; 32];
        data[0..25].clone_from_slice(input);
        transcript.push(data);

        keccak::f1600(input);

        data[0..25].clone_from_slice(input);
        transcript.push(data);
    }

    let mut digest = SHA256_INIT;
    for halfs in bytemuck::cast_slice::<[u64; 32], [u64; 8]>(transcript.as_slice()) {
        let mut first_half = [0u8; DIGEST_BYTES];
        first_half.clone_from_slice(bytemuck::cast_slice(&halfs[0..4]));

        let mut second_half = [0u8; DIGEST_BYTES];
        second_half.clone_from_slice(bytemuck::cast_slice(&halfs[4..8]));

        digest = *sha::Impl::compress(
            &digest,
            &Digest::from_bytes(first_half),
            &Digest::from_bytes(second_half),
        );
    }

    // reorder to match the keccak accelerator
    for word in digest.as_mut_words() {
        *word = word.to_be();
    }
    digest
}
