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

use std::collections::VecDeque;
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
use std::rc::Rc;

use anyhow::{ensure, Result};
use risc0_binfmt::read_sha_halfs;
use risc0_circuit_keccak::{
    get_control_id,
    prove::{keccak_prover, zkr::get_keccak_zkr, KeccakProver, Seal},
    KECCAK_CONTROL_IDS,
};
use risc0_core::field::baby_bear::BabyBearElem;
use risc0_zkp::core::digest::{Digest, DIGEST_SHORTS};
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
use risc0_zkp::hal::webgpu::WebGpuHal;

use crate::{
    host::client::env::ProveKeccakRequest, host::recursion::prove::prove_zkr,
    receipt::SuccinctReceipt, Unknown,
};

/// Generate a keccak proof that has been lifted.
pub(crate) fn prove_keccak(request: &ProveKeccakRequest) -> Result<SuccinctReceipt<Unknown>> {
    let zkr_input = {
        let prover = keccak_prover()?;
        let seal = prover.prove(&request.input, request.po2)?;
        keccak_zkr_input(request, prover.as_ref(), seal)?
    };

    prove_zkr(
        get_keccak_zkr(request.po2)?,
        get_control_id(request.po2),
        KECCAK_CONTROL_IDS.to_vec(),
        bytemuck::cast_slice(zkr_input.as_slice()),
    )
}

/// Browser WebGPU async variant of [`prove_keccak`].
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub(crate) async fn prove_keccak_webgpu(
    request: &ProveKeccakRequest,
    hal: Rc<WebGpuHal>,
) -> Result<SuccinctReceipt<Unknown>> {
    let zkr_input = {
        let prover = risc0_circuit_keccak::prove::keccak_prover_with_hal(hal.clone())?;
        let seal = prover.prove_async(&request.input, request.po2).await?;
        keccak_zkr_input(request, prover.as_ref(), seal)?
    };

    crate::host::recursion::prove::prove_zkr_webgpu(
        get_keccak_zkr(request.po2)?,
        get_control_id(request.po2),
        KECCAK_CONTROL_IDS.to_vec(),
        bytemuck::cast_slice(zkr_input.as_slice()),
        hal,
    )
    .await
}

fn keccak_zkr_input(
    request: &ProveKeccakRequest,
    prover: &dyn KeccakProver,
    seal: Seal,
) -> Result<Vec<u32>> {
    let claim_digest: Digest = read_sha_halfs(&mut VecDeque::from_iter(
        bytemuck::checked::cast_slice::<_, BabyBearElem>(&seal[0..DIGEST_SHORTS])
            .iter()
            .copied()
            .map(u32::from),
    ))?;

    ensure!(
        request.claim_digest == claim_digest,
        "keccak claim digest mismatch, expected: {:?}, actual: {claim_digest:?}",
        request.claim_digest
    );

    // Make sure we have a valid seal so we can fail early if anything went wrong.
    prover.verify(&seal)?;

    let claim_sha_input = claim_digest
        .as_words()
        .iter()
        .copied()
        .flat_map(|x| [x & 0xffff, x >> 16])
        .map(BabyBearElem::new)
        .collect::<Vec<_>>();

    let mut zkr_input: Vec<u32> = Vec::new();
    zkr_input.extend(request.control_root.as_words());
    zkr_input.extend(seal);
    zkr_input.extend(bytemuck::cast_slice(claim_sha_input.as_slice()));
    zkr_input.push(1 << request.po2);

    Ok(zkr_input)
}
