// Copyright 2024 RISC Zero, Inc.
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

use alloc::vec::Vec;

use risc0_core::{field::ExtElem, scope};
use tracing::debug;

use crate::{
    core::log2_ceil,
    hal::{Buffer, Hal},
    prove::{merkle::MerkleTreeProver, write_iop::WriteIOP},
    FRI_FOLD, FRI_MIN_DEGREE, INV_RATE, QUERIES,
};

struct ProveRoundInfo<H: Hal> {
    domain: usize,
    coeffs: H::Buffer<H::Elem>,
    merkle: MerkleTreeProver<H>,
}

impl<H: Hal> ProveRoundInfo<H> {
    /// Computes a round of the folding protocol. Takes in the coefficients of
    /// the current polynomial, and interacts with the IOP verifier to
    /// produce the evaluations of the polynomial, the merkle tree
    /// committing to the evaluation, and the coefficients of the folded
    /// polynomial.
    pub fn new(hal: &H, iop: &mut WriteIOP<H::Field>, coeffs: &H::Buffer<H::Elem>) -> Self {
        debug!("Doing FRI folding");
        let ext_size = H::ExtElem::EXT_SIZE;
        // Get the number of coefficients of the polynomial over the extension field.
        let size = coeffs.size() / ext_size;
        // Get a larger domain to interpolate over.
        let domain = size * INV_RATE;
        // Allocate space in which to put the interpolated values.
        let evaluated = hal.alloc_elem("evaluated", domain * ext_size);
        // Put in the coefficients, padding out with zeros so that we are left with the
        // same polynomial represented by a larger coefficient list
        // Evaluate the NTT in-place, filling the buffer with the evaluations of the
        // polynomial.
        hal.batch_expand_into_evaluate_ntt(&evaluated, coeffs, ext_size, log2_ceil(INV_RATE));
        // Compute a Merkle tree committing to the polynomial evaluations.
        let merkle = MerkleTreeProver::new(
            hal,
            &evaluated,
            domain / FRI_FOLD,
            FRI_FOLD * ext_size,
            QUERIES,
        );
        // Send the merkle tree (as a commitment) to the virtual IOP verifier
        merkle.commit(iop);
        // Retrieve from the IOP verifier a random value to mix the polynomial slices.
        let fold_mix = iop.random_ext_elem();
        // Create a buffer to hold the mixture of slices.
        let out_coeffs = hal.alloc_elem("out_coeffs", size / FRI_FOLD * ext_size);
        // Compute the folded polynomial
        hal.fri_fold(&out_coeffs, coeffs, &fold_mix);
        ProveRoundInfo {
            domain,
            coeffs: out_coeffs,
            merkle,
        }
    }

    pub fn prove_query(&mut self, hal: &H, iop: &mut WriteIOP<H::Field>, pos: &mut usize) {
        // Compute which group we are in
        let group = *pos % (self.domain / FRI_FOLD);
        // Generate the proof
        self.merkle.prove(hal, iop, group);
        // Update pos
        *pos = group;
    }
}

pub fn fri_prove<H: Hal, F>(
    hal: &H,
    iop: &mut WriteIOP<H::Field>,
    coeffs: &H::Buffer<H::Elem>,
    inner: F,
) where
    F: Fn(&mut WriteIOP<H::Field>, usize),
{
    scope!("fri_prove");
    let ext_size = H::ExtElem::EXT_SIZE;
    let orig_domain = coeffs.size() / ext_size * INV_RATE;
    let mut rounds = Vec::new();
    let mut coeffs = coeffs.clone();
    while coeffs.size() / ext_size > FRI_MIN_DEGREE {
        let round = ProveRoundInfo::new(hal, iop, &coeffs);
        coeffs = round.coeffs.clone();
        rounds.push(round);
    }
    // Put the final coefficients into natural order
    let final_coeffs = hal.alloc_elem("final_coeffs", coeffs.size());
    hal.eltwise_copy_elem(&final_coeffs, &coeffs);
    hal.batch_bit_reverse(&final_coeffs, ext_size);
    // Dump final polynomial + commit
    final_coeffs.view(|view| {
        iop.write_field_elem_slice::<H::Elem>(view);
        let digest = hal.get_hash_suite().hashfn.hash_elem_slice(view);
        iop.commit(&digest);
    });
    // Do queries
    debug!("Doing Queries");
    for _ in 0..QUERIES {
        // Get a 'random' index.
        let mut pos = iop.random_bits(log2_ceil(orig_domain)) as usize;
        // Do the 'inner' proof for this index
        inner(iop, pos);
        // Write the per-round proofs
        for round in rounds.iter_mut() {
            round.prove_query(hal, iop, &mut pos);
        }
    }
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
struct WebGpuProveRoundInfo {
    domain: usize,
    coeffs: crate::hal::webgpu::WebGpuBuffer<risc0_core::field::baby_bear::BabyBearElem>,
    merkle: MerkleTreeProver<crate::hal::webgpu::WebGpuHal>,
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
impl WebGpuProveRoundInfo {
    async fn new(
        hal: &crate::hal::webgpu::WebGpuHal,
        iop: &mut WriteIOP<risc0_core::field::baby_bear::BabyBear>,
        coeffs: &crate::hal::webgpu::WebGpuBuffer<risc0_core::field::baby_bear::BabyBearElem>,
        round_idx: usize,
    ) -> anyhow::Result<Self> {
        debug!("Doing FRI folding");
        let _round_timer = crate::hal::webgpu::WebGpuStageTimer::new(format!(
            "fri_prove round={round_idx} domain_in={}",
            coeffs.size() / <crate::hal::webgpu::WebGpuHal as Hal>::ExtElem::EXT_SIZE * INV_RATE
        ));
        let ext_size = <crate::hal::webgpu::WebGpuHal as Hal>::ExtElem::EXT_SIZE;
        let size = coeffs.size() / ext_size;
        let domain = size * INV_RATE;
        let evaluated = hal.alloc_elem("evaluated", domain * ext_size);
        {
            let _t = crate::hal::webgpu::WebGpuStageTimer::new(format!(
                "fri_prove round={round_idx} expand_evaluate_ntt domain={domain}"
            ));
            hal.batch_expand_into_evaluate_ntt_async(
                &evaluated,
                coeffs,
                ext_size,
                log2_ceil(INV_RATE),
            )
            .await?;
            if crate::hal::webgpu::poly_group_drain_diagnostic_enabled() {
                let _t = crate::hal::webgpu::WebGpuStageTimer::new_for(
                    format!("fri_prove round={round_idx} drain_after_expand_evaluate_ntt domain={domain}"),
                    hal,
                );
                hal.wait_idle().await?;
            }
        }
        let merkle = {
            let _t = crate::hal::webgpu::WebGpuStageTimer::new(format!(
                "fri_prove round={round_idx} merkle_new rows={} cols={}",
                domain / FRI_FOLD,
                FRI_FOLD * ext_size
            ));
            let merkle_name = format!("fri_round{round_idx}");
            MerkleTreeProver::new_committed_async(
                hal,
                &evaluated,
                domain / FRI_FOLD,
                FRI_FOLD * ext_size,
                QUERIES,
                iop,
                &merkle_name,
            )
            .await?
        };
        let fold_mix = iop.random_ext_elem();
        let count_out = size / FRI_FOLD;
        let out_coeffs = hal.alloc_elem("out_coeffs", count_out * ext_size);
        {
            let _t = crate::hal::webgpu::WebGpuStageTimer::new(format!(
                "fri_prove round={round_idx} fri_fold count_out={}",
                count_out
            ));
            hal.fri_fold_async(&out_coeffs, coeffs, &fold_mix).await?;
            if crate::hal::webgpu::poly_group_drain_diagnostic_enabled() {
                let _t = crate::hal::webgpu::WebGpuStageTimer::new_for(
                    format!(
                        "fri_prove round={round_idx} drain_after_fri_fold count_out={count_out}"
                    ),
                    hal,
                );
                hal.wait_idle().await?;
            }
        }
        Ok(WebGpuProveRoundInfo {
            domain,
            coeffs: out_coeffs,
            merkle,
        })
    }
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub async fn fri_prove_async(
    hal: &crate::hal::webgpu::WebGpuHal,
    iop: &mut WriteIOP<risc0_core::field::baby_bear::BabyBear>,
    coeffs: &crate::hal::webgpu::WebGpuBuffer<risc0_core::field::baby_bear::BabyBearElem>,
    inner_merkles: &[&MerkleTreeProver<crate::hal::webgpu::WebGpuHal>],
) -> anyhow::Result<()> {
    scope!("fri_prove");
    let ext_size = <crate::hal::webgpu::WebGpuHal as Hal>::ExtElem::EXT_SIZE;
    let orig_domain = coeffs.size() / ext_size * INV_RATE;
    let mut rounds = Vec::new();
    let mut coeffs = coeffs.clone();
    let mut round_idx = 0usize;
    while coeffs.size() / ext_size > FRI_MIN_DEGREE {
        let round = WebGpuProveRoundInfo::new(hal, iop, &coeffs, round_idx).await?;
        coeffs = round.coeffs.clone();
        rounds.push(round);
        round_idx += 1;
    }

    let final_coeffs = hal.alloc_elem("final_coeffs", coeffs.size());
    {
        let _t = crate::hal::webgpu::WebGpuStageTimer::new(format!(
            "fri_prove final size={}",
            final_coeffs.size()
        ));
        hal.eltwise_copy_elem(&final_coeffs, &coeffs);
        hal.batch_bit_reverse_async(&final_coeffs, ext_size).await?;
        final_coeffs.sync_gpu_to_cpu(hal).await?;
        final_coeffs.view(|view| {
            iop.write_field_elem_slice::<risc0_core::field::baby_bear::BabyBearElem>(view);
            let digest = hal.get_hash_suite().hashfn.hash_elem_slice(view);
            iop.commit(&digest);
        });
    }

    debug!("Doing Queries");
    let mut query_positions = Vec::with_capacity(QUERIES);
    let mut round_positions = Vec::with_capacity(QUERIES);
    for _ in 0..QUERIES {
        let pos = iop.random_bits(log2_ceil(orig_domain)) as usize;
        let mut pos_for_rounds = pos;
        let mut per_round = Vec::with_capacity(rounds.len());
        for round in rounds.iter() {
            let group = pos_for_rounds % (round.domain / FRI_FOLD);
            per_round.push(group);
            pos_for_rounds = group;
        }
        query_positions.push(pos);
        round_positions.push(per_round);
    }

    // Coalesce query openings across Merkle trees. Each proof still writes the
    // same tree/query order below, but the browser sees one indexed readback
    // for inner trees and one for FRI-round trees instead of one map per tree.
    let inner_positions_by_tree = inner_merkles
        .iter()
        .map(|_| query_positions.clone())
        .collect::<Vec<_>>();
    let inner_proofs = crate::prove::merkle::prove_batch_for_trees_async(
        hal,
        inner_merkles,
        inner_positions_by_tree.as_slice(),
    )
    .await?;

    let round_positions_per_round: Vec<Vec<usize>> = (0..rounds.len())
        .map(|round_idx| {
            round_positions
                .iter()
                .map(|per_round| per_round[round_idx])
                .collect()
        })
        .collect();
    let round_merkles = rounds.iter().map(|round| &round.merkle).collect::<Vec<_>>();
    let round_proofs = crate::prove::merkle::prove_batch_for_trees_async(
        hal,
        round_merkles.as_slice(),
        round_positions_per_round.as_slice(),
    )
    .await?;

    for query_idx in 0..QUERIES {
        for proofs in &inner_proofs {
            proofs[query_idx].write_to_iop(iop);
        }
        for proofs in &round_proofs {
            proofs[query_idx].write_to_iop(iop);
        }
    }
    Ok(())
}
