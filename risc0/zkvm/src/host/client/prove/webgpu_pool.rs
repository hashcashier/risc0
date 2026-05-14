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

use std::{collections::HashMap, collections::VecDeque, rc::Rc};

use anyhow::{anyhow, bail, Context, Result};
use risc0_zkp::{
    core::hash::poseidon2::Poseidon2HashSuite,
    hal::webgpu::WebGpuHal,
};

use crate::{
    claim::merge::Merge,
    host::{
        prove_info::ProveInfo,
        recursion::prove::{join_webgpu, lift_webgpu},
        server::{
            exec::executor::ExecutorImpl,
            prove::{
                keccak::prove_keccak_webgpu,
                prover_impl::{
                    ProverImpl, WEBGPU_DEFAULT_KECCAK_MAX_PO2,
                    WEBGPU_DEFAULT_SEGMENT_LIMIT_PO2,
                },
                ProverServer,
            },
            session::{Segment, SimpleSegmentRef},
        },
    },
    receipt::{InnerReceipt, SegmentReceipt, SuccinctReceipt},
    sha::Digestible,
    Assumption, AssumptionReceipt, CompositeReceipt, ExecutorEnv,
    InnerAssumptionReceipt, MaybePruned, Output, ProveKeccakRequest, ProverOpts, Receipt,
    ReceiptClaim, ReceiptKind, Unknown, VerifierContext,
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

        let pool_size = self.provers.len();

        // Phase 1: lifts in chunks of pool_size. Each lift peaks at
        // ~6 GiB of CPU+GPU buffers; concurrent lifts > pool_size will
        // OOM wasm32 (Vec capacity overflow). Chunking matches the
        // bounded-concurrency pattern used by prove_keccak_requests_async.
        let mut lifted: Vec<SuccinctReceipt<ReceiptClaim>> =
            Vec::with_capacity(composite.segments.len());
        for chunk in composite.segments.chunks(pool_size) {
            let mut futures = Vec::with_capacity(chunk.len());
            for (offset, seg) in chunk.iter().enumerate() {
                let hal = self.provers[offset].hal_handle();
                let seg = seg.clone();
                futures.push(async move { lift_webgpu(&seg, hal).await });
            }
            let chunk_lifts = futures::future::try_join_all(futures)
                .await
                .context("pool lift phase chunk")?;
            lifted.extend(chunk_lifts);
        }

        // Phase 2: balanced-tree joins. At each level pair receipts
        // left-to-right and run pairs in chunks of pool_size; odd tail
        // receipt passes through unchanged.
        let mut tier = lifted;
        let mut level = 0_u32;
        while tier.len() > 1 {
            let pairs: Vec<(SuccinctReceipt<ReceiptClaim>, SuccinctReceipt<ReceiptClaim>)> = tier
                .chunks_exact(2)
                .map(|pair| (pair[0].clone(), pair[1].clone()))
                .collect();
            let carry: Option<SuccinctReceipt<ReceiptClaim>> = if tier.len() % 2 == 1 {
                tier.last().cloned()
            } else {
                None
            };
            let mut next_tier: Vec<SuccinctReceipt<ReceiptClaim>> = Vec::with_capacity(
                pairs.len() + carry.as_ref().map_or(0, |_| 1),
            );
            for chunk in pairs.chunks(pool_size) {
                let mut futures = Vec::with_capacity(chunk.len());
                for (offset, (a, b)) in chunk.iter().enumerate() {
                    let hal = self.provers[offset].hal_handle();
                    let a = a.clone();
                    let b = b.clone();
                    futures.push(async move { join_webgpu(&a, &b, hal).await });
                }
                let joined = futures::future::try_join_all(futures)
                    .await
                    .with_context(|| format!("pool join level {level}"))?;
                next_tier.extend(joined);
            }
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

    /// SP6d iter 8: end-to-end pool prove. Drives the same flow as
    /// `WebGpuProver::prove_with_ctx_async` but distributes the GPU-heavy
    /// phases across pool slots:
    ///
    /// 1. Apply WebGPU env defaults (po2 ≤ 18, keccak po2 ≤ 14).
    /// 2. Execute the session on CPU (single-threaded; not GPU work).
    /// 3. Distribute per-segment proves across slots in chunks of
    ///    `pool.len()` — bounded concurrency, segments preflight on their
    ///    own slot's `WebGpuSegmentProver` (which routes through the
    ///    slot's HAL).
    /// 4. Distribute pending keccak proofs via
    ///    [`Self::prove_keccak_requests_async`].
    /// 5. Build the keccak union root on slot 0 (tree depth is small;
    ///    distributing log-N levels adds little).
    /// 6. Verify the composite receipt.
    /// 7. Composite mode: return the composite receipt.
    /// 8. Succinct mode: distribute lifts + tree joins via
    ///    [`Self::lift_and_join_async`] and apply any assumption resolves
    ///    serially on slot 0.
    ///
    /// PoVW and Groth16 receipt kinds are not yet supported on the pool
    /// path (same constraints as `ProverImpl::prove_session_async`).
    pub async fn prove_with_ctx_async(
        &self,
        env: ExecutorEnv<'_>,
        ctx: &VerifierContext,
        elf: &[u8],
        opts: &ProverOpts,
    ) -> Result<ProveInfo> {
        anyhow::ensure!(
            !opts.dev_mode(),
            "browser WebGPU pool proving does not support dev-mode"
        );
        anyhow::ensure!(
            opts.hashfn == "poseidon2",
            "ProverOpts hashfn is unsupported: \"{}\"; expected \"poseidon2\"",
            opts.hashfn
        );

        let pool_size = self.provers.len();
        let prove_wall_start = js_sys::Date::now();

        let mut env = env;
        env.segment_limit_po2 = Some(
            env.segment_limit_po2
                .unwrap_or(WEBGPU_DEFAULT_SEGMENT_LIMIT_PO2)
                .min(WEBGPU_DEFAULT_SEGMENT_LIMIT_PO2),
        );
        env.keccak_max_po2 = Some(
            env.keccak_max_po2
                .unwrap_or(WEBGPU_DEFAULT_KECCAK_MAX_PO2)
                .min(WEBGPU_DEFAULT_KECCAK_MAX_PO2),
        );

        // One ProverImpl per slot so the per-phase async methods route
        // through the slot's HAL via `WebGpuSegmentProver::with_hal`.
        let slot_impls: Vec<Rc<ProverImpl>> = self
            .provers
            .iter()
            .map(|p| Rc::new(ProverImpl::new_webgpu(opts.clone(), p.hal_handle())))
            .collect();

        // Phase 1: execute. CPU-only and single-threaded; runs through
        // the slot-0 segment_prover for its preflight (unused on this
        // path) and emits SegmentRefs.
        let session = ExecutorImpl::from_elf(env, elf)?
            .run_with_callback(|seg| Ok(Box::new(SimpleSegmentRef::new(seg))))?;
        anyhow::ensure!(
            session.povw_job_id.is_none(),
            "browser WebGPU pool proving does not yet support PoVW receipts"
        );

        // Phase 2: per-segment proves. Resolve refs first (cheap).
        //
        // Segments are proved SERIALLY through slot 0. Iter 8 explored
        // running two segments concurrently across slots at po2_18; the
        // commit_group_async peak (≈1.8 GiB per segment for code + data +
        // accum buffers) overflows wasm32's isize-bounded `Vec` when
        // two segments allocate their accum poly group at the same
        // moment. Serial segment proves keep peak memory at one
        // segment's footprint while still enabling lift+join / keccak
        // distribution downstream.
        let resolved_segments: Vec<Segment> = session
            .segments
            .iter()
            .map(|r| r.resolve())
            .collect::<Result<_>>()?;

        let mut segment_receipts: Vec<SegmentReceipt> =
            Vec::with_capacity(resolved_segments.len());
        let segment_slot = slot_impls[0].clone();
        for seg in &resolved_segments {
            for hook in &session.hooks {
                hook.on_pre_prove_segment(seg);
            }
            let preflight = segment_slot
                .segment_preflight(seg)
                .with_context(|| format!("preflight segment {}", seg.index))?;
            let receipt = segment_slot
                .prove_segment_core_async(ctx, preflight)
                .await
                .with_context(|| format!("prove segment {}", seg.index))?;
            segment_receipts.push(receipt);
            for hook in &session.hooks {
                hook.on_post_prove_segment(seg);
            }
        }

        let (assumptions, session_assumption_receipts): (Vec<_>, Vec<_>) =
            session.assumptions.iter().cloned().unzip();

        segment_receipts
            .last_mut()
            .ok_or_else(|| anyhow!("session is empty"))?
            .claim
            .output
            .merge_with(
                &session
                    .journal
                    .as_ref()
                    .map(|journal| Output {
                        journal: MaybePruned::Pruned(journal.digest()),
                        assumptions: assumptions.into(),
                    })
                    .into(),
            )
            .context("failed to merge output into final segment claim")?;

        let verifier_parameters = ctx
            .composite_verifier_parameters()
            .ok_or_else(|| {
                anyhow!("composite receipt verifier parameters missing from context")
            })?
            .digest();

        // Phase 3: pending keccaks. Distribute via existing pool method.
        let keccak_receipts: Vec<SuccinctReceipt<Unknown>> =
            if session.pending_keccaks().is_empty() {
                Vec::new()
            } else {
                self.prove_keccak_requests_async(session.pending_keccaks())
                    .await
                    .context("pool prove_keccak_requests")?
            };

        // Phase 4: keccak union tree on slot 0. MMR insert may chain
        // unions; root collapses any remaining peaks.
        let mut zkr_receipts = HashMap::new();
        let mut peaks: VecDeque<(u32, SuccinctReceipt<Unknown>)> = VecDeque::new();
        let union_slot = slot_impls[0].clone();
        for receipt in keccak_receipts {
            union_slot
                .insert_union_receipt_async(&mut peaks, receipt)
                .await
                .context("pool keccak union insert")?;
        }
        if let Some(root_receipt) = union_slot
            .union_receipts_root_async(peaks)
            .await
            .context("pool keccak union root")?
        {
            let assumption = Assumption {
                claim: root_receipt.claim.digest(),
                control_root: root_receipt.control_root()?,
            };
            zkr_receipts.insert(assumption, root_receipt);
        }

        let inner_assumption_receipts: Vec<_> = session_assumption_receipts
            .into_iter()
            .map(|ar| match ar {
                AssumptionReceipt::Proven(r) => Ok(r),
                AssumptionReceipt::Unresolved(assumption) => {
                    let r = zkr_receipts.get(&assumption).ok_or_else(|| {
                        anyhow!("no receipt for unresolved assumption: {assumption:#?}")
                    })?;
                    Ok(InnerAssumptionReceipt::Succinct(r.clone()))
                }
            })
            .collect::<Result<_>>()?;

        let composite_receipt = CompositeReceipt {
            segments: segment_receipts,
            assumption_receipts: inner_assumption_receipts,
            verifier_parameters,
        };

        let session_claim = session.claim()?;
        composite_receipt
            .verify_integrity_with_context(ctx)
            .context("pool composite verify")?;
        let composite_claim_digest = composite_receipt.claim()?.digest();
        let session_claim_digest = session_claim.digest();
        if session_claim_digest != composite_claim_digest {
            bail!(
                "pool session claim mismatch: {} != {}",
                hex::encode(session_claim_digest),
                hex::encode(composite_claim_digest)
            );
        }

        if opts.receipt_kind == ReceiptKind::Composite {
            let segments_len = composite_receipt.segments.len();
            let receipt = Receipt::new(
                InnerReceipt::Composite(composite_receipt),
                session.journal.clone().unwrap_or_default().bytes,
            );
            let wall_ms: f64 = js_sys::Date::now() - prove_wall_start;
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "pool_prove_with_ctx_async receipt_kind=Composite wall_ms={wall_ms:.0} segments={segments_len} pool_size={pool_size}",
            ));
            return Ok(ProveInfo {
                receipt,
                work_receipt: None,
                stats: session.stats(),
            });
        }

        anyhow::ensure!(
            opts.receipt_kind == ReceiptKind::Succinct,
            "browser WebGPU pool proving currently supports Composite and Succinct receipts"
        );

        let succinct = self
            .composite_to_succinct_async(&composite_receipt)
            .await
            .context("pool composite_to_succinct")?;

        let receipt = Receipt::new(
            InnerReceipt::Succinct(succinct),
            session.journal.clone().unwrap_or_default().bytes,
        );
        let wall_ms: f64 = js_sys::Date::now() - prove_wall_start;
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "pool_prove_with_ctx_async receipt_kind=Succinct wall_ms={wall_ms:.0} segments={} pool_size={}",
            resolved_segments.len(),
            pool_size
        ));
        Ok(ProveInfo {
            receipt,
            work_receipt: None,
            stats: session.stats(),
        })
    }

    /// SP6d iter 8: pool composite-to-succinct. Distributes the segment
    /// lift + tree-join phase across slots (via
    /// [`Self::lift_and_join_async`]) and runs assumption resolves
    /// serially on slot 0. Nested composite assumptions recurse through
    /// the same pool path.
    pub async fn composite_to_succinct_async(
        &self,
        composite: &CompositeReceipt,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        let segments_only = CompositeReceipt {
            segments: composite.segments.clone(),
            assumption_receipts: Vec::new(),
            verifier_parameters: composite.verifier_parameters,
        };
        let mut continuation = self
            .lift_and_join_async(&segments_only)
            .await
            .context("pool lift+join")?;

        if composite.assumption_receipts.is_empty() {
            return Ok(continuation);
        }

        let slot0 = slot0_succinct_impl(self);
        for (idx, assumption) in composite.assumption_receipts.iter().enumerate() {
            continuation = match assumption {
                InnerAssumptionReceipt::Succinct(a) => slot0
                    .resolve_async(&continuation, a)
                    .await
                    .with_context(|| format!("pool resolve assumption {idx}"))?,
                InnerAssumptionReceipt::Composite(nested) => {
                    let nested_succinct =
                        Box::pin(self.composite_to_succinct_async(nested)).await?;
                    let unknown =
                        SuccinctReceipt::<ReceiptClaim>::into_unknown(nested_succinct);
                    slot0
                        .resolve_async(&continuation, &unknown)
                        .await
                        .with_context(|| format!("pool resolve nested assumption {idx}"))?
                }
                InnerAssumptionReceipt::Fake(_) => {
                    bail!(
                        "pool: composite receipts with Fake assumptions are not supported"
                    )
                }
                InnerAssumptionReceipt::Groth16(_) => {
                    bail!(
                        "pool: composite receipts with Groth16 assumptions are not supported"
                    )
                }
            };
        }

        Ok(continuation)
    }
}

/// Build a slot-0 ProverImpl with succinct options for the serial resolve
/// loop. Resolves are rare and ordered, so reusing one impl is fine.
fn slot0_succinct_impl(pool: &WebGpuProverPool) -> Rc<ProverImpl> {
    Rc::new(ProverImpl::new_webgpu(
        ProverOpts::succinct(),
        pool.provers[0].hal_handle(),
    ))
}
