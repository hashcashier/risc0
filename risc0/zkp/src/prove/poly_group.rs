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

use risc0_core::scope_with;

use crate::{
    core::log2_ceil,
    hal::{Buffer, Hal},
    prove::merkle::MerkleTreeProver,
    INV_RATE, QUERIES,
};

/// A PolyGroup represents a group of polynomials, all of the same maximum
/// degree, as well as the evaluation of those polynomials over some domain that
/// is larger than that degree by some invRate. Additionally, it includes a
/// dense Merkle tree, where each entry is a single point of the domain, and the
/// leaf hash is a simple linear hash of all of the values at that point.  That
/// is, if we have 100 polynomials evaluated on 2^16 points, the merkle tree has
/// 2^16 entries, each being a hash of 100 values.  The size of the domain is
/// always a power of 2 so that we can use NTTs.
///
/// The primary purpose of the PolyGroup is for use in the DEEP-ALI protocol,
/// which basically needs 4 methods during proof generation, specifically we
/// need to: 1) Resolve queries (i.e. make MerkleColProofs)
/// 2) Do evaluation of the polynomials at 'randomly' chosen points
/// 3) Mix the polynomials via a random set of linear coefficients
/// 4) Access the raw values in the evaluation domain to 'evaluate' the
///    constraint polynomial
///
/// The poly group holds 3 buffers:
/// 1) The per-polynomial coefficients, used for evaluation + mixing
/// 2) The points evaluated on the domain in question (for the 'col' part of
///    merkle proofs)
/// 3) The Merkle tree itself.
///
/// PolyGroups are constructed from two basic sources: steps of a computations,
/// and a single higher degree polynomial that has been split into lower degree
/// parts.  In the case of computations, the resulting steps must be padded
/// (possibly with randomized data), which is presumed to be done by the caller.
/// The constructor additionally 'shifts' the polynomial so that f(x) -> f(3*x),
/// which means that the normal NTT evaluation domain does not reveal anything
/// about the original datapoints (i.e. is zero knowledge) so long as there is
/// sufficient randomized padding.
pub struct PolyGroup<H: Hal> {
    pub coeffs: H::Buffer<H::Elem>,
    pub count: usize,
    pub evaluated: H::Buffer<H::Elem>,
    pub merkle: MerkleTreeProver<H>,
}

impl<H: Hal> PolyGroup<H> {
    pub fn new(
        hal: &H,
        coeffs: H::Buffer<H::Elem>,
        count: usize,
        size: usize,
        name: &'static str,
    ) -> Self {
        scope_with!("poly_group({})", name);
        assert_eq!(coeffs.size(), count * size);
        let domain = size * INV_RATE;
        let evaluated = hal.alloc_elem("evaluated", count * domain);
        hal.batch_expand_into_evaluate_ntt(&evaluated, &coeffs, count, log2_ceil(INV_RATE));
        hal.batch_bit_reverse(&coeffs, count);
        let merkle = MerkleTreeProver::new(hal, &evaluated, domain, count, QUERIES);
        PolyGroup {
            coeffs,
            count,
            evaluated,
            merkle,
        }
    }
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
impl PolyGroup<crate::hal::webgpu::WebGpuHal> {
    /// Async WebGPU variant of [`Self::new`].
    pub async fn new_async(
        hal: &crate::hal::webgpu::WebGpuHal,
        coeffs: crate::hal::webgpu::WebGpuBuffer<risc0_core::field::baby_bear::BabyBearElem>,
        count: usize,
        size: usize,
        name: &'static str,
    ) -> anyhow::Result<Self> {
        scope_with!("poly_group({})", name);
        assert_eq!(coeffs.size(), count * size);
        let domain = size * INV_RATE;
        let evaluated = hal.alloc_elem("evaluated", count * domain);
        {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new_for(
                format!(
                    "poly_group {name} batch_expand_into_evaluate_ntt count={count} size={size} domain={domain}"
                ),
                hal,
            );
            hal.batch_expand_into_evaluate_ntt_async(
                &evaluated,
                &coeffs,
                count,
                log2_ceil(INV_RATE),
            )
            .await?;
            if crate::hal::webgpu::poly_group_drain_diagnostic_enabled() {
                let _timer = crate::hal::webgpu::WebGpuStageTimer::new_for(
                    format!(
                        "poly_group {name} drain_after_batch_expand_into_evaluate_ntt count={count} size={size} domain={domain}"
                    ),
                    hal,
                );
                hal.wait_idle().await?;
            }
        }
        {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new_for(
                format!("poly_group {name} batch_bit_reverse count={count} size={size}"),
                hal,
            );
            hal.batch_bit_reverse_async(&coeffs, count).await?;
            if crate::hal::webgpu::poly_group_drain_diagnostic_enabled() {
                let _timer = crate::hal::webgpu::WebGpuStageTimer::new_for(
                    format!(
                        "poly_group {name} drain_after_batch_bit_reverse count={count} size={size}"
                    ),
                    hal,
                );
                hal.wait_idle().await?;
            }
        }
        let merkle =
            MerkleTreeProver::new_async(hal, &evaluated, domain, count, QUERIES, name).await?;
        Ok(PolyGroup {
            coeffs,
            count,
            evaluated,
            merkle,
        })
    }

    /// Async WebGPU constructor for callers that commit the Merkle tree
    /// immediately after construction.
    pub async fn new_committed_async(
        hal: &crate::hal::webgpu::WebGpuHal,
        iop: &mut crate::prove::write_iop::WriteIOP<risc0_core::field::baby_bear::BabyBear>,
        coeffs: crate::hal::webgpu::WebGpuBuffer<risc0_core::field::baby_bear::BabyBearElem>,
        count: usize,
        size: usize,
        name: &'static str,
    ) -> anyhow::Result<Self> {
        scope_with!("poly_group({})", name);
        assert_eq!(coeffs.size(), count * size);
        let domain = size * INV_RATE;
        let evaluated = hal.alloc_elem("evaluated", count * domain);
        {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new_for(
                format!(
                    "poly_group {name} batch_expand_into_evaluate_ntt count={count} size={size} domain={domain}"
                ),
                hal,
            );
            hal.batch_expand_into_evaluate_ntt_async(
                &evaluated,
                &coeffs,
                count,
                log2_ceil(INV_RATE),
            )
            .await?;
            if crate::hal::webgpu::poly_group_drain_diagnostic_enabled() {
                let _timer = crate::hal::webgpu::WebGpuStageTimer::new_for(
                    format!(
                        "poly_group {name} drain_after_batch_expand_into_evaluate_ntt count={count} size={size} domain={domain}"
                    ),
                    hal,
                );
                hal.wait_idle().await?;
            }
        }
        {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new_for(
                format!("poly_group {name} batch_bit_reverse count={count} size={size}"),
                hal,
            );
            hal.batch_bit_reverse_async(&coeffs, count).await?;
            if crate::hal::webgpu::poly_group_drain_diagnostic_enabled() {
                let _timer = crate::hal::webgpu::WebGpuStageTimer::new_for(
                    format!(
                        "poly_group {name} drain_after_batch_bit_reverse count={count} size={size}"
                    ),
                    hal,
                );
                hal.wait_idle().await?;
            }
        }
        let merkle = MerkleTreeProver::new_committed_async(
            hal, &evaluated, domain, count, QUERIES, iop, name,
        )
        .await?;
        Ok(PolyGroup {
            coeffs,
            count,
            evaluated,
            merkle,
        })
    }
}
