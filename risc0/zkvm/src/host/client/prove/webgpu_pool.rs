// Copyright 2026 RISC Zero, Inc.
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

//! SP6d iter 1 — multi-HAL prover pool scaffold.
//!
//! Holds N independent `WebGpuHal` instances, each backed by its own
//! `web_sys::GpuDevice` acquired via `request_device()`. Provides a simple
//! round-robin lease so callers can issue concurrent prove jobs across
//! devices.
//!
//! Per measurement on 2026-05-13 the 5090 sat at 12.6% mean utilization on
//! single-device WebGPU proving; the GPU has ~7× headroom that can only be
//! exploited via independent submission queues. Each `web_sys::GpuDevice`
//! has its own `GpuQueue`; multi-device proving on a single JS thread
//! parallelizes at the driver level because devices have independent
//! command queues per the WebGPU spec.
//!
//! NOTE: in iter 1 this is the type structure plus a smoke test only — the
//! `prove_session_async` work-distribution layer that issues different
//! segments / lifts / joins to different HALs is deferred to iter 2+.

use std::rc::Rc;

use anyhow::{anyhow, Context, Result};
use risc0_zkp::{
    core::hash::poseidon2::Poseidon2HashSuite,
    hal::webgpu::WebGpuHal,
};

use crate::{
    host::{
        recursion::prove::{join_webgpu, lift_webgpu},
        server::prove::keccak::prove_keccak_webgpu,
    },
    receipt::SuccinctReceipt,
    CompositeReceipt, ProveKeccakRequest, ReceiptClaim, Unknown,
};

use super::webgpu::WebGpuProver;

/// A pool of independent WebGPU provers. Each prover owns its own
/// `web_sys::GpuDevice` and `WebGpuHal`. Concurrent prove calls across
/// different pool slots execute on independent GPU queues.
pub struct WebGpuProverPool {
    provers: Vec<Rc<WebGpuProver>>,
    next: std::cell::Cell<usize>,
}

impl WebGpuProverPool {
    /// Construct a pool with `slots` independent HALs. Each HAL calls
    /// `request_device()` and may fail individually if the browser refuses
    /// further devices.
    pub async fn new(slots: usize) -> Result<Self> {
        anyhow::ensure!(slots > 0, "WebGpuProverPool requires at least 1 slot");
        let mut provers = Vec::with_capacity(slots);
        for idx in 0..slots {
            let hal = WebGpuHal::new(Poseidon2HashSuite::new_suite()).await?;
            let prover =
                WebGpuProver::from_hal(&format!("webgpu-pool-{idx}"), Rc::new(hal));
            provers.push(Rc::new(prover));
        }
        Ok(Self {
            provers,
            next: std::cell::Cell::new(0),
        })
    }

    /// Number of provers in the pool.
    pub fn len(&self) -> usize {
        self.provers.len()
    }

    /// Returns `true` if the pool has no provers (cannot be constructed).
    pub fn is_empty(&self) -> bool {
        self.provers.is_empty()
    }

    /// Get the prover at `idx`. Panics if `idx >= len()`.
    pub fn get(&self, idx: usize) -> Rc<WebGpuProver> {
        self.provers[idx].clone()
    }

    /// Acquire the next prover in round-robin order. Returns
    /// `(slot_index, prover_handle)`.
    pub fn next_slot(&self) -> (usize, Rc<WebGpuProver>) {
        let n = self.provers.len();
        let idx = self.next.get();
        self.next.set((idx + 1) % n);
        (idx, self.provers[idx].clone())
    }

    /// SP6d iter 5: distribute lifts + tree-joins across pool slots to
    /// compress a composite receipt. The composite's segments may have
    /// been proved by any (single) prover; this method only handles the
    /// lift+join phase.
    ///
    /// Lift assignment: segment `N` lifts on slot `N % pool.len()`.
    /// Join structure: balanced tree (left-to-right pairing per level).
    /// Pairs at each tree level run concurrently across slots.
    ///
    /// Limitation: this method does NOT currently handle composite
    /// receipts with assumption_receipts (resolve phase). For those,
    /// fall back to `WebGpuProver::compress_async` on a single slot.
    pub async fn lift_and_join_async(
        &self,
        composite: &CompositeReceipt,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        anyhow::ensure!(
            composite.assumption_receipts.is_empty(),
            "lift_and_join_async does not handle assumptions; use compress_async on a single prover"
        );
        anyhow::ensure!(
            !composite.segments.is_empty(),
            "composite receipt has no segments"
        );

        // Phase 1: lifts across slots in parallel.
        let mut lift_futures = Vec::with_capacity(composite.segments.len());
        for (idx, seg) in composite.segments.iter().enumerate() {
            let hal = self.provers[idx % self.provers.len()].hal_handle();
            let seg = seg.clone();
            lift_futures.push(async move { lift_webgpu(&seg, hal).await });
        }
        let mut tier = futures::future::try_join_all(lift_futures)
            .await
            .context("pool lift phase")?;

        // Phase 2: balanced-tree joins. At each level pair receipts
        // left-to-right; odd receipt at the end passes through unchanged.
        let mut level = 0_u32;
        while tier.len() > 1 {
            let mut join_futures = Vec::with_capacity((tier.len() + 1) / 2);
            let mut carry: Option<SuccinctReceipt<ReceiptClaim>> = None;
            let mut chunks = tier.chunks_exact(2);
            let mut slot_idx = 0;
            for pair in &mut chunks {
                let hal = self.provers[slot_idx % self.provers.len()].hal_handle();
                let a = pair[0].clone();
                let b = pair[1].clone();
                join_futures.push(async move { join_webgpu(&a, &b, hal).await });
                slot_idx += 1;
            }
            // Odd tail receipt passes through this level.
            if let Some(remainder) = chunks.remainder().first() {
                carry = Some(remainder.clone());
            }
            let mut next_tier = futures::future::try_join_all(join_futures)
                .await
                .with_context(|| format!("pool join level {level}"))?;
            if let Some(c) = carry {
                next_tier.push(c);
            }
            tier = next_tier;
            level += 1;
        }

        tier.into_iter()
            .next()
            .ok_or_else(|| anyhow!("lift_and_join produced no receipt"))
    }

    /// SP6d iter 6: distribute keccak proof requests across pool slots.
    /// Each `ProveKeccakRequest` runs `prove_keccak_webgpu` on a
    /// different slot's `WebGpuHal` in parallel via
    /// `futures::future::try_join_all`. Returns receipts in input order.
    ///
    /// Keccak is the canonical "accelerator" workload: many independent
    /// proofs, each one a complete prove of the keccak circuit. With N
    /// requests on a K-slot pool, wall time approaches
    /// `ceil(N/K) × single-request-wall` instead of `N × wall`.
    pub async fn prove_keccak_requests_async(
        &self,
        requests: &[ProveKeccakRequest],
    ) -> Result<Vec<SuccinctReceipt<Unknown>>> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }
        // Bound active concurrency to pool size. Each keccak prove peaks
        // at ~500 MiB-2 GiB of CPU+GPU buffers; with try_join_all of N
        // requests, ALL allocate simultaneously before any drain, and
        // wasm32's isize-bounded Vec OOMs at ~2 GiB. Batching by slot
        // count keeps at-most-pool-size proves in flight.
        let pool_size = self.provers.len();
        let mut receipts = Vec::with_capacity(requests.len());
        for chunk in requests.chunks(pool_size) {
            let mut futures = Vec::with_capacity(chunk.len());
            for (offset_in_chunk, req) in chunk.iter().enumerate() {
                let hal = self.provers[offset_in_chunk].hal_handle();
                let req = req.clone();
                futures.push(async move { prove_keccak_webgpu(&req, hal).await });
            }
            let chunk_receipts = futures::future::try_join_all(futures)
                .await
                .context("pool keccak requests chunk")?;
            receipts.extend(chunk_receipts);
        }
        Ok(receipts)
    }
}
