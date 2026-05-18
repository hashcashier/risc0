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

use std::{
    cell::Cell,
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
    rc::Rc,
};

use anyhow::{anyhow, bail, Context, Result};
use risc0_zkp::{
    core::hash::poseidon2::Poseidon2HashSuite,
    hal::webgpu::{
        WebGpuDeviceCopyDiagnostics, WebGpuDiagnostics, WebGpuHal, WebGpuOpDiagnostics,
        WebGpuReadbackDiagnostics, WebGpuUploadDiagnostics,
    },
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

/// Construct a browser WebGPU prover pool with `slots` independent devices.
pub async fn webgpu_prover_pool(slots: usize) -> Result<Rc<WebGpuProverPool>> {
    Ok(Rc::new(WebGpuProverPool::new(slots).await?))
}

/// A pool of independent WebGPU provers. Each prover owns its own
/// `web_sys::GpuDevice` and `WebGpuHal`. Concurrent prove calls across
/// different pool slots execute on independent GPU queues.
pub struct WebGpuProverPool {
    provers: Vec<Rc<WebGpuProver>>,
    next: Cell<usize>,
    last_prove_strategy: Cell<Option<&'static str>>,
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
            next: Cell::new(0),
            last_prove_strategy: Cell::new(None),
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

    /// Return aggregate backend usage diagnostics across all pool slots.
    pub fn diagnostics(&self) -> WebGpuDiagnostics {
        let mut aggregate = WebGpuDiagnostics::default();
        for prover in &self.provers {
            merge_webgpu_diagnostics(&mut aggregate, prover.diagnostics());
        }
        aggregate
    }

    /// Reset backend usage diagnostics across all pool slots.
    pub fn reset_diagnostics(&self) {
        for prover in &self.provers {
            prover.reset_diagnostics();
        }
    }

    /// Returns the proving strategy used by the most recent pool-level
    /// `prove_with_ctx*` call. This is intentionally diagnostics-only so
    /// browser performance tests can assert that public entrypoints route
    /// through the intended scheduler.
    #[doc(hidden)]
    pub fn last_prove_strategy_for_diagnostics(&self) -> Option<&'static str> {
        self.last_prove_strategy.get()
    }

    /// Async browser WebGPU pool variant of `Prover::prove`.
    pub async fn prove_async(&self, env: ExecutorEnv<'_>, elf: &[u8]) -> Result<ProveInfo> {
        let opts = ProverOpts::default();
        let ctx = VerifierContext::default();
        self.prove_with_ctx_async(env, &ctx, elf, &opts).await
    }

    /// Async browser WebGPU pool variant of `Prover::prove_with_opts`.
    pub async fn prove_with_opts_async(
        &self,
        env: ExecutorEnv<'_>,
        elf: &[u8],
        opts: &ProverOpts,
    ) -> Result<ProveInfo> {
        let ctx = VerifierContext::default().with_dev_mode(opts.dev_mode());
        self.prove_with_ctx_async(env, &ctx, elf, opts).await
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
        self.prove_with_ctx_scheduled_async(env, ctx, elf, opts)
            .await
    }

    /// SP6d iter 8 legacy phased pool prove. This is retained for A/B
    /// comparisons and recovery, but the public pool entrypoint routes
    /// through [`Self::prove_with_ctx_scheduled_async`] because the
    /// dependency-graph scheduler is the measured positive SP6d path.
    pub async fn prove_with_ctx_sequential_async(
        &self,
        env: ExecutorEnv<'_>,
        ctx: &VerifierContext,
        elf: &[u8],
        opts: &ProverOpts,
    ) -> Result<ProveInfo> {
        self.last_prove_strategy.set(Some("sequential"));
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

    /// SP6d iter 8: pool composite-to-succinct.
    ///
    /// Runs the **serial interleaved** lift→join→lift→join chain on slot
    /// 0 (`ProverImpl::composite_to_succinct_async`), which also handles
    /// the assumption resolve phase natively.
    ///
    /// Iter 8 measured `lift_and_join_async`'s distributed all-lifts-
    /// then-balanced-tree-joins structure against the serial chain on
    /// the xgboost R9 fixture (11 segments):
    ///
    /// | Path | xgboost wall | mean GPU util |
    /// |---|---:|---:|
    /// | 2-slot `lift_and_join_async` | 139 977 ms | 33.5% |
    /// | serial `composite_to_succinct_async` | 141 983 ms | 41.7% |
    ///
    /// The walls are within 1.4% (noise), but the serial chain keeps the
    /// GPU *better* engaged (41.7% vs 33.5%) — distributing lift+join
    /// across two `GpuDevice`s on a single physical GPU just time-slices
    /// the same hardware, while the tree restructuring + chunk barriers
    /// add idle gaps. On a single JS thread + single physical GPU,
    /// lift+join does not parallelize; the keccak phase
    /// ([`Self::prove_keccak_requests_async`]) is where the pool's real
    /// win lives (many genuinely-independent proofs → 47.5% mean util).
    ///
    /// `lift_and_join_async` is retained as a validated building block
    /// (iter 5/7 smokes) but is no longer the orchestrator default.
    pub async fn composite_to_succinct_async(
        &self,
        composite: &CompositeReceipt,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        let slot0 = Rc::new(ProverImpl::new_webgpu(
            ProverOpts::succinct(),
            self.provers[0].hal_handle(),
        ));
        slot0
            .composite_to_succinct_async(composite)
            .await
            .context("pool composite_to_succinct (serial slot 0)")
    }

    /// SP6d iter 9: end-to-end pool prove driven by a **dependency-graph
    /// scheduler** instead of fixed sequential phases.
    ///
    /// `prove_with_ctx_async` runs phases strictly in order (all segments
    /// → all keccaks → union → lift+join → resolve), so segment-proving
    /// and keccak-proving never overlap. This method instead maintains a
    /// ready-task set over the full dependency graph and assigns ready
    /// tasks to free pool slots as they become available — segment
    /// proves, keccak proves, the keccak union build, and segment lifts
    /// can all be in flight simultaneously.
    ///
    /// The hypothesis under test: a mixed segment+keccak workload is the
    /// one case where concurrency *could* win on wall time — segment
    /// proves are CPU-witgen-heavy, keccak proves spend more of their
    /// time on GPU commit, so overlapping them stresses different
    /// resources. The homogeneous SP6d A/B tests (keccak-only,
    /// lift+join-only) showed flat wall time; this scheduler tests
    /// whether a *heterogeneous* job mix changes that.
    ///
    /// **Memory admission — the binding constraint.** A po2_18 rv32im
    /// segment prove cannot share wasm32's ~2 GiB address space with ANY
    /// other prove. Iter 9 measured this directly: running 1 segment + 1
    /// keccak concurrently crashed with `RuntimeError: unreachable` when
    /// the segment hit its accum poly-group allocation while a keccak's
    /// finalize buffers were still resident. So a segment runs strictly
    /// **alone** — admitted only when nothing else is in flight, and
    /// blocking all other admissions while it runs. Light tasks (keccak
    /// proofs, lifts, joins, the union build) are individually smaller
    /// and fill the pool up to `pool_size` among themselves.
    ///
    /// Consequence: at po2_18 the scheduler *cannot* overlap segment
    /// proving with anything — the wasm32 address space, not the GPU or
    /// the submission mechanism, forbids it. The heterogeneous overlap
    /// the scheduler was built to exploit is unreachable until the
    /// per-segment buffer peak shrinks (SP7's GPU-resident witness).
    ///
    /// A 1-slot pool runs this scheduler strictly serially (one task at a
    /// time, dependency-ordered) — that is the honest serial baseline for
    /// an A/B against an N-slot pool: same code, same fixture, only the
    /// slot count varies.
    pub async fn prove_with_ctx_scheduled_async(
        &self,
        env: ExecutorEnv<'_>,
        ctx: &VerifierContext,
        elf: &[u8],
        opts: &ProverOpts,
    ) -> Result<ProveInfo> {
        self.last_prove_strategy.set(Some("scheduled"));
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

        let slot_impls: Vec<Rc<ProverImpl>> = self
            .provers
            .iter()
            .map(|p| Rc::new(ProverImpl::new_webgpu(opts.clone(), p.hal_handle())))
            .collect();

        let session = ExecutorImpl::from_elf(env, elf)?
            .run_with_callback(|seg| Ok(Box::new(SimpleSegmentRef::new(seg))))?;
        anyhow::ensure!(
            session.povw_job_id.is_none(),
            "browser WebGPU pool proving does not yet support PoVW receipts"
        );

        let resolved_segments: Vec<Segment> = session
            .segments
            .iter()
            .map(|r| r.resolve())
            .collect::<Result<_>>()?;
        let n_seg = resolved_segments.len();
        anyhow::ensure!(n_seg > 0, "session has no segments");
        let n_kec = session.pending_keccaks().len();

        // Pre-prove hooks fire for all segments up front; post-prove
        // hooks after the whole scheduler. With concurrent scheduling the
        // strict per-segment pre→prove→post ordering cannot be preserved;
        // test hooks use Rc<RefCell> flags and are race-free on a single
        // JS thread (validated by the iter-8 multi-segment smoke).
        for seg in &resolved_segments {
            for hook in &session.hooks {
                hook.on_pre_prove_segment(seg);
            }
        }

        let (assumptions, session_assumption_receipts): (Vec<_>, Vec<_>) =
            session.assumptions.iter().cloned().unzip();

        let verifier_parameters = ctx
            .composite_verifier_parameters()
            .ok_or_else(|| {
                anyhow!("composite receipt verifier parameters missing from context")
            })?
            .digest();

        // Join-tree shape: tier 0 has the N lifts; each tier halves
        // (ceil) until a single receipt remains.
        let mut tier_sizes = vec![n_seg];
        while *tier_sizes.last().unwrap() > 1 {
            let s = *tier_sizes.last().unwrap();
            tier_sizes.push((s + 1) / 2);
        }
        let tier_count = tier_sizes.len();

        // Scheduler state.
        let mut seg_done: Vec<Option<SegmentReceipt>> = (0..n_seg).map(|_| None).collect();
        let mut seg_started = vec![false; n_seg];
        let mut kec_done: Vec<Option<SuccinctReceipt<Unknown>>> =
            (0..n_kec).map(|_| None).collect();
        let mut kec_started = vec![false; n_kec];
        let mut kec_root_started = false;
        let mut kec_root: Option<Option<SuccinctReceipt<Unknown>>> = None;
        let mut merged = false;
        let mut lift_started = vec![false; n_seg];
        let mut tiers: Vec<Vec<Option<SuccinctReceipt<ReceiptClaim>>>> =
            tier_sizes.iter().map(|&s| (0..s).map(|_| None).collect()).collect();
        let mut join_started: Vec<Vec<bool>> =
            tier_sizes.iter().map(|&s| vec![false; s]).collect();

        let mut free_slots: Vec<usize> = (0..pool_size).collect();
        let mut seg_in_flight = false;
        let mut in_flight: Vec<
            Pin<Box<dyn Future<Output = Result<(usize, SchedDone)>> + '_>>,
        > = Vec::new();

        loop {
            // Admission: hand ready tasks to free slots.
            //
            // Hard memory constraint (iter 9, measured): a po2_18 rv32im
            // segment prove cannot share wasm32's ~2 GiB address space
            // with ANY other prove. Iter 9's first scheduler attempt ran
            // 1 segment + 1 keccak concurrently and crashed with
            // `RuntimeError: unreachable` when the segment hit its accum
            // poly-group allocation while a keccak's finalize buffers
            // were resident. So a segment runs strictly ALONE: it is only
            // admitted when nothing else is in flight, and while it runs
            // nothing else is admitted. Light tasks (keccak / lift / join
            // / union) are individually smaller and fill the pool up to
            // `pool_size` among themselves.
            while let Some(&slot) = free_slots.last() {
                let picked: Option<SchedTask> = 'pick: {
                    // A segment in flight blocks everything.
                    if seg_in_flight {
                        break 'pick None;
                    }
                    if !kec_root_started
                        && n_kec > 0
                        && kec_done.iter().all(Option::is_some)
                    {
                        break 'pick Some(SchedTask::KeccakRoot);
                    }
                    // A segment can only start when nothing else is in
                    // flight, so it runs alone.
                    if in_flight.is_empty() {
                        if let Some(i) = seg_started.iter().position(|s| !s) {
                            break 'pick Some(SchedTask::Segment(i));
                        }
                    }
                    if let Some(j) = kec_started.iter().position(|s| !s) {
                        break 'pick Some(SchedTask::Keccak(j));
                    }
                    for i in 0..n_seg {
                        if lift_started[i] {
                            continue;
                        }
                        let dep_ok = if i + 1 < n_seg {
                            seg_done[i].is_some()
                        } else {
                            merged
                        };
                        if dep_ok {
                            break 'pick Some(SchedTask::Lift(i));
                        }
                    }
                    for t in 0..tier_count.saturating_sub(1) {
                        for p in 0..(tier_sizes[t] / 2) {
                            if join_started[t][p] {
                                continue;
                            }
                            if tiers[t][2 * p].is_some() && tiers[t][2 * p + 1].is_some() {
                                break 'pick Some(SchedTask::Join(t, p));
                            }
                        }
                    }
                    None
                };

                let Some(task) = picked else {
                    break;
                };
                free_slots.pop();

                let fut: Pin<Box<dyn Future<Output = Result<(usize, SchedDone)>> + '_>> =
                    match task {
                        SchedTask::Segment(i) => {
                            seg_started[i] = true;
                            seg_in_flight = true;
                            let impl_rc = slot_impls[slot].clone();
                            let seg = resolved_segments[i].clone();
                            Box::pin(async move {
                                let pf = impl_rc
                                    .segment_preflight(&seg)
                                    .with_context(|| {
                                        format!("preflight segment {}", seg.index)
                                    })?;
                                let r = impl_rc
                                    .prove_segment_core_async(ctx, pf)
                                    .await
                                    .with_context(|| {
                                        format!("prove segment {}", seg.index)
                                    })?;
                                Ok((slot, SchedDone::Segment(i, r)))
                            })
                        }
                        SchedTask::Keccak(j) => {
                            kec_started[j] = true;
                            let hal = self.provers[slot].hal_handle();
                            let req = session.pending_keccaks()[j].clone();
                            Box::pin(async move {
                                let r = prove_keccak_webgpu(&req, hal)
                                    .await
                                    .with_context(|| format!("pool keccak request {j}"))?;
                                Ok((slot, SchedDone::Keccak(j, r)))
                            })
                        }
                        SchedTask::KeccakRoot => {
                            kec_root_started = true;
                            let impl_rc = slot_impls[slot].clone();
                            let receipts: Vec<SuccinctReceipt<Unknown>> = kec_done
                                .iter_mut()
                                .map(|r| {
                                    r.take().expect("all keccaks done before KeccakRoot")
                                })
                                .collect();
                            Box::pin(async move {
                                let mut peaks: VecDeque<(u32, SuccinctReceipt<Unknown>)> =
                                    VecDeque::new();
                                for r in receipts {
                                    impl_rc
                                        .insert_union_receipt_async(&mut peaks, r)
                                        .await
                                        .context("pool keccak union insert")?;
                                }
                                let root = impl_rc
                                    .union_receipts_root_async(peaks)
                                    .await
                                    .context("pool keccak union root")?;
                                Ok((slot, SchedDone::KeccakRoot(root)))
                            })
                        }
                        SchedTask::Lift(i) => {
                            lift_started[i] = true;
                            let hal = self.provers[slot].hal_handle();
                            let seg_receipt =
                                seg_done[i].clone().expect("segment done before lift");
                            Box::pin(async move {
                                let r = lift_webgpu(&seg_receipt, hal)
                                    .await
                                    .with_context(|| format!("pool lift segment {i}"))?;
                                Ok((slot, SchedDone::Lift(i, r)))
                            })
                        }
                        SchedTask::Join(t, p) => {
                            join_started[t][p] = true;
                            let hal = self.provers[slot].hal_handle();
                            let a = tiers[t][2 * p].clone().expect("join left ready");
                            let b = tiers[t][2 * p + 1].clone().expect("join right ready");
                            Box::pin(async move {
                                let r = join_webgpu(&a, &b, hal)
                                    .await
                                    .with_context(|| {
                                        format!("pool join tier {t} pos {p}")
                                    })?;
                                Ok((slot, SchedDone::Join(t, p, r)))
                            })
                        }
                    };
                in_flight.push(fut);
            }

            if in_flight.is_empty() {
                break;
            }

            let (result, _idx, remaining) =
                futures::future::select_all(in_flight).await;
            in_flight = remaining;
            let (slot, done) = result?;
            free_slots.push(slot);

            match done {
                SchedDone::Segment(i, r) => {
                    seg_done[i] = Some(r);
                    seg_in_flight = false;
                    if !merged && seg_done.iter().all(Option::is_some) {
                        // Merge journal digest + assumptions into the
                        // final segment claim. Depends on all segments;
                        // gates Lift(n_seg-1).
                        seg_done
                            .last_mut()
                            .unwrap()
                            .as_mut()
                            .unwrap()
                            .claim
                            .output
                            .merge_with(
                                &session
                                    .journal
                                    .as_ref()
                                    .map(|journal| Output {
                                        journal: MaybePruned::Pruned(journal.digest()),
                                        assumptions: assumptions.clone().into(),
                                    })
                                    .into(),
                            )
                            .context("failed to merge output into final segment claim")?;
                        merged = true;
                    }
                }
                SchedDone::Keccak(j, r) => {
                    kec_done[j] = Some(r);
                }
                SchedDone::KeccakRoot(root) => {
                    kec_root = Some(root);
                }
                SchedDone::Lift(i, r) => {
                    tiers[0][i] = Some(r);
                    propagate_join_carries(&mut tiers, &tier_sizes);
                }
                SchedDone::Join(t, p, r) => {
                    tiers[t + 1][p] = Some(r);
                    propagate_join_carries(&mut tiers, &tier_sizes);
                }
            }
        }

        for seg in &resolved_segments {
            for hook in &session.hooks {
                hook.on_post_prove_segment(seg);
            }
        }

        // Post-scheduler: assemble + verify the composite receipt, then
        // run the (inherently serial) resolve chain on slot 0.
        let kec_root_receipt: Option<SuccinctReceipt<Unknown>> = kec_root.flatten();
        let mut zkr_receipts = HashMap::new();
        if let Some(root) = &kec_root_receipt {
            let assumption = Assumption {
                claim: root.claim.digest(),
                control_root: root.control_root()?,
            };
            zkr_receipts.insert(assumption, root.clone());
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

        let segment_receipts: Vec<SegmentReceipt> =
            seg_done.into_iter().map(|r| r.unwrap()).collect();
        let composite_receipt = CompositeReceipt {
            segments: segment_receipts,
            assumption_receipts: inner_assumption_receipts,
            verifier_parameters,
        };

        let session_claim = session.claim()?;
        composite_receipt
            .verify_integrity_with_context(ctx)
            .context("pool scheduled composite verify")?;
        let composite_claim_digest = composite_receipt.claim()?.digest();
        if session_claim.digest() != composite_claim_digest {
            bail!(
                "pool scheduled session claim mismatch: {} != {}",
                hex::encode(session_claim.digest()),
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
                "pool_prove_scheduled_async receipt_kind=Composite wall_ms={wall_ms:.0} segments={segments_len} keccaks={n_kec} pool_size={pool_size}",
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

        // The join tree's single output is the lifted+joined continuation
        // over all segments; resolve the assumptions onto it serially.
        let mut continuation = tiers
            .last()
            .and_then(|t| t.first())
            .and_then(|r| r.clone())
            .ok_or_else(|| anyhow!("scheduler produced no joined receipt"))?;

        if !composite_receipt.assumption_receipts.is_empty() {
            let slot0 = Rc::new(ProverImpl::new_webgpu(
                ProverOpts::succinct(),
                self.provers[0].hal_handle(),
            ));
            for (idx, assumption) in
                composite_receipt.assumption_receipts.iter().enumerate()
            {
                continuation = match assumption {
                    InnerAssumptionReceipt::Succinct(a) => slot0
                        .resolve_async(&continuation, a)
                        .await
                        .with_context(|| format!("pool scheduled resolve {idx}"))?,
                    InnerAssumptionReceipt::Composite(nested) => {
                        let nested_succinct =
                            self.composite_to_succinct_async(nested).await?;
                        let unknown = SuccinctReceipt::<ReceiptClaim>::into_unknown(
                            nested_succinct,
                        );
                        slot0
                            .resolve_async(&continuation, &unknown)
                            .await
                            .with_context(|| {
                                format!("pool scheduled resolve nested {idx}")
                            })?
                    }
                    InnerAssumptionReceipt::Fake(_) => bail!(
                        "pool: composite receipts with Fake assumptions are not supported"
                    ),
                    InnerAssumptionReceipt::Groth16(_) => bail!(
                        "pool: composite receipts with Groth16 assumptions are not supported"
                    ),
                };
            }
        }

        let receipt = Receipt::new(
            InnerReceipt::Succinct(continuation),
            session.journal.clone().unwrap_or_default().bytes,
        );
        let wall_ms: f64 = js_sys::Date::now() - prove_wall_start;
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "pool_prove_scheduled_async receipt_kind=Succinct wall_ms={wall_ms:.0} segments={n_seg} keccaks={n_kec} pool_size={pool_size}",
        ));
        Ok(ProveInfo {
            receipt,
            work_receipt: None,
            stats: session.stats(),
        })
    }
}

fn merge_webgpu_diagnostics(aggregate: &mut WebGpuDiagnostics, diagnostics: WebGpuDiagnostics) {
    aggregate.buffers_allocated = aggregate
        .buffers_allocated
        .saturating_add(diagnostics.buffers_allocated);
    aggregate.bytes_allocated = aggregate
        .bytes_allocated
        .saturating_add(diagnostics.bytes_allocated);
    aggregate.host_to_gpu_uploads = aggregate
        .host_to_gpu_uploads
        .saturating_add(diagnostics.host_to_gpu_uploads);
    aggregate.host_to_gpu_bytes = aggregate
        .host_to_gpu_bytes
        .saturating_add(diagnostics.host_to_gpu_bytes);
    aggregate.device_copies = aggregate
        .device_copies
        .saturating_add(diagnostics.device_copies);
    aggregate.device_copy_bytes = aggregate
        .device_copy_bytes
        .saturating_add(diagnostics.device_copy_bytes);
    aggregate.readbacks = aggregate.readbacks.saturating_add(diagnostics.readbacks);
    aggregate.readback_bytes = aggregate
        .readback_bytes
        .saturating_add(diagnostics.readback_bytes);
    aggregate.bind_group_layout_creations = aggregate
        .bind_group_layout_creations
        .saturating_add(diagnostics.bind_group_layout_creations);
    aggregate.bind_group_layout_cache_hits = aggregate
        .bind_group_layout_cache_hits
        .saturating_add(diagnostics.bind_group_layout_cache_hits);
    aggregate.bind_group_creations = aggregate
        .bind_group_creations
        .saturating_add(diagnostics.bind_group_creations);
    aggregate.compute_pipeline_creations = aggregate
        .compute_pipeline_creations
        .saturating_add(diagnostics.compute_pipeline_creations);
    aggregate.compute_pipeline_cache_hits = aggregate
        .compute_pipeline_cache_hits
        .saturating_add(diagnostics.compute_pipeline_cache_hits);
    aggregate.gpu_dispatches = aggregate
        .gpu_dispatches
        .saturating_add(diagnostics.gpu_dispatches);
    aggregate.cpu_mirrors = aggregate.cpu_mirrors.saturating_add(diagnostics.cpu_mirrors);
    aggregate.cpu_fallbacks = aggregate
        .cpu_fallbacks
        .saturating_add(diagnostics.cpu_fallbacks);
    aggregate.cpu_only_ops = aggregate
        .cpu_only_ops
        .saturating_add(diagnostics.cpu_only_ops);

    for op in diagnostics.ops {
        merge_webgpu_op_diagnostics(&mut aggregate.ops, op);
    }
    for source in diagnostics.upload_sources {
        merge_webgpu_upload_diagnostics(&mut aggregate.upload_sources, source);
    }
    for source in diagnostics.device_copy_sources {
        merge_webgpu_device_copy_diagnostics(&mut aggregate.device_copy_sources, source);
    }
    for source in diagnostics.readback_sources {
        merge_webgpu_readback_diagnostics(&mut aggregate.readback_sources, source);
    }
}

fn merge_webgpu_op_diagnostics(
    aggregate: &mut Vec<WebGpuOpDiagnostics>,
    diagnostics: WebGpuOpDiagnostics,
) {
    if let Some(existing) = aggregate
        .iter_mut()
        .find(|existing| existing.name == diagnostics.name)
    {
        existing.gpu_dispatches = existing
            .gpu_dispatches
            .saturating_add(diagnostics.gpu_dispatches);
        existing.cpu_mirrors = existing.cpu_mirrors.saturating_add(diagnostics.cpu_mirrors);
        existing.cpu_fallbacks = existing
            .cpu_fallbacks
            .saturating_add(diagnostics.cpu_fallbacks);
        existing.cpu_only_ops = existing.cpu_only_ops.saturating_add(diagnostics.cpu_only_ops);
    } else {
        aggregate.push(diagnostics);
    }
}

fn merge_webgpu_upload_diagnostics(
    aggregate: &mut Vec<WebGpuUploadDiagnostics>,
    diagnostics: WebGpuUploadDiagnostics,
) {
    if let Some(existing) = aggregate
        .iter_mut()
        .find(|existing| existing.name == diagnostics.name)
    {
        existing.uploads = existing.uploads.saturating_add(diagnostics.uploads);
        existing.upload_bytes = existing
            .upload_bytes
            .saturating_add(diagnostics.upload_bytes);
    } else {
        aggregate.push(diagnostics);
    }
}

fn merge_webgpu_readback_diagnostics(
    aggregate: &mut Vec<WebGpuReadbackDiagnostics>,
    diagnostics: WebGpuReadbackDiagnostics,
) {
    if let Some(existing) = aggregate
        .iter_mut()
        .find(|existing| existing.name == diagnostics.name)
    {
        existing.readbacks = existing.readbacks.saturating_add(diagnostics.readbacks);
        existing.readback_bytes = existing
            .readback_bytes
            .saturating_add(diagnostics.readback_bytes);
    } else {
        aggregate.push(diagnostics);
    }
}

fn merge_webgpu_device_copy_diagnostics(
    aggregate: &mut Vec<WebGpuDeviceCopyDiagnostics>,
    diagnostics: WebGpuDeviceCopyDiagnostics,
) {
    if let Some(existing) = aggregate
        .iter_mut()
        .find(|existing| existing.name == diagnostics.name)
    {
        existing.device_copies = existing
            .device_copies
            .saturating_add(diagnostics.device_copies);
        existing.device_copy_bytes = existing
            .device_copy_bytes
            .saturating_add(diagnostics.device_copy_bytes);
    } else {
        aggregate.push(diagnostics);
    }
}

/// SP6d iter 9 — dependency-graph scheduler task kinds for
/// [`WebGpuProverPool::prove_with_ctx_scheduled_async`].
enum SchedTask {
    /// Prove rv32im segment `index` (preflight + prove_core).
    Segment(usize),
    /// Prove pending keccak request `index`.
    Keccak(usize),
    /// Build the keccak union root from all keccak receipts.
    KeccakRoot,
    /// Lift segment receipt `index` to a succinct receipt.
    Lift(usize),
    /// Join tier `t` position `p` (consumes tier `t` slots `2p`, `2p+1`).
    Join(usize, usize),
}

/// SP6d iter 9 — completed-task payloads carried back from the scheduler
/// futures, tagged so the scheduler can record them into its state.
enum SchedDone {
    Segment(usize, SegmentReceipt),
    Keccak(usize, SuccinctReceipt<Unknown>),
    KeccakRoot(Option<SuccinctReceipt<Unknown>>),
    Lift(usize, SuccinctReceipt<ReceiptClaim>),
    Join(usize, usize, SuccinctReceipt<ReceiptClaim>),
}

/// Fill in any odd-tail "carry" slots in the join tier table. When a
/// tier has an odd element count, its last element is not joined — it
/// passes straight through to the last slot of the next tier. Carries
/// can cascade, so iterate to a fixed point.
fn propagate_join_carries(
    tiers: &mut [Vec<Option<SuccinctReceipt<ReceiptClaim>>>],
    tier_sizes: &[usize],
) {
    loop {
        let mut changed = false;
        for t in 0..tier_sizes.len().saturating_sub(1) {
            if tier_sizes[t] % 2 == 1 {
                let last_in = tier_sizes[t] - 1;
                let last_out = tier_sizes[t + 1] - 1;
                if tiers[t][last_in].is_some() && tiers[t + 1][last_out].is_none() {
                    tiers[t + 1][last_out] = tiers[t][last_in].clone();
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
}
