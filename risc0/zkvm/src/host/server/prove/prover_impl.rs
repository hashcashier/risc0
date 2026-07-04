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

use std::collections::HashMap;
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
use std::collections::{BTreeMap, HashSet, VecDeque};
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
use std::rc::Rc;

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
use crate::host::server::session::SimpleSegmentRef;
use anyhow::{anyhow, bail, ensure, Context, Result};
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
use risc0_zkp::hal::webgpu::WebGpuHal;

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
use super::keccak::prove_keccak_webgpu;
use super::{keccak::prove_keccak, ProverServer};
use crate::{
    claim::merge::Merge,
    host::{
        client::prove::opts::ReceiptKind,
        prove_info::ProveInfo,
        recursion::{identity_p254, join, lift, resolve},
        server::{exec::executor::ExecutorImpl, prove::union_peak::UnionPeak},
    },
    mmr::MerkleMountainAccumulator,
    receipt::{InnerReceipt, SegmentReceipt, SuccinctReceipt},
    recursion::prove::{
        join_povw, join_unwrap_povw, lift_povw, resolve_povw, resolve_unwrap_povw, union,
        unwrap_povw,
    },
    sha::Digestible,
    Assumption, AssumptionReceipt, CompositeReceipt, ExecutorEnv, InnerAssumptionReceipt,
    MaybePruned, Output, PreflightResults, ProverOpts, Receipt, ReceiptClaim, Segment, Session,
    UnionClaim, Unknown, VerifierContext, WorkClaim,
};

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub(crate) const WEBGPU_DEFAULT_SEGMENT_LIMIT_PO2: u32 = 18;
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub(crate) const WEBGPU_DEFAULT_KECCAK_MAX_PO2: u32 = 14;

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
use risc0_zkp::hal::webgpu::WebGpuStageTimer;

/// M6d: output of a segment's witgen pipeline stage, awaiting its commit
/// stage. Carries the claim-side data the commit stage needs (the
/// rv32im-level job consumes the circuit `PreflightResults`) plus the
/// resolved segment so post-prove hooks fire with the same argument as
/// the serial path did.
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
struct StagedSegmentJob {
    job: risc0_circuit_rv32im::prove::WebGpuSegmentJob,
    po2: u32,
    output: Option<crate::Output>,
    segment_index: u32,
    segment: Segment,
}

/// An implementation of a Prover that runs locally.
pub struct ProverImpl {
    opts: ProverOpts,
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    webgpu_hal: Option<Rc<WebGpuHal>>,
    /// Extra WebGPU devices dedicated to the succinct-phase scheduler.
    /// Readbacks wait for the whole device queue, so proofs interleaved
    /// on one device serialize (wall = CPU + GPU); per-proof devices
    /// give each proof an independent queue.
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    webgpu_recursion_hals: Vec<Rc<WebGpuHal>>,
}

impl ProverImpl {
    /// Construct a [ProverImpl].
    pub fn new(opts: ProverOpts) -> Self {
        Self {
            opts,
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            webgpu_hal: None,
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            webgpu_recursion_hals: Vec::new(),
        }
    }

    /// Construct a browser WebGPU [ProverImpl] from an initialized HAL.
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    pub fn new_webgpu(opts: ProverOpts, hal: Rc<WebGpuHal>) -> Self {
        Self {
            opts,
            webgpu_hal: Some(hal),
            webgpu_recursion_hals: Vec::new(),
        }
    }

    /// Attach extra WebGPU devices for the succinct-phase scheduler.
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    pub fn with_webgpu_recursion_hals(mut self, hals: Vec<Rc<WebGpuHal>>) -> Self {
        self.webgpu_recursion_hals = hals;
        self
    }

    fn segment_prover(&self) -> Result<Box<dyn risc0_circuit_rv32im::prove::SegmentProver>> {
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        {
            if let Some(hal) = &self.webgpu_hal {
                return risc0_circuit_rv32im::prove::segment_prover_with_hal(hal.clone());
            }
        }

        risc0_circuit_rv32im::prove::segment_prover()
    }
}

/// M7a: run a receipt integrity check on a pool worker instead of blocking
/// this wasm thread. Each check is pure CPU over the owned receipt
/// (~15 ms), but a block of that size under in-flight readbacks delays
/// every other proof's mapAsync callback (the M6d starvation physics).
/// The receipt travels through the worker and back via the oneshot.
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
async fn verify_integrity_offloaded<R, F>(receipt: R, verify: F) -> Result<R>
where
    R: Send + 'static,
    F: FnOnce(&R) -> Result<()> + Send + 'static,
{
    let (tx, rx) = futures::channel::oneshot::channel();
    rayon::spawn(move || {
        let result = verify(&receipt);
        let _ = tx.send((result, receipt));
    });
    let (result, receipt) = rx
        .await
        .map_err(|_| anyhow!("verify offload worker dropped its result channel"))?;
    result?;
    Ok(receipt)
}

/// Join-tree nodes `(lo, hi, mid)` over segments `[0, N)`: every internal
/// node `(lo, hi)` joins `(lo, mid)` + `(mid, hi)` at the midpoint, giving
/// a balanced tree (depth `ceil(log2 N)`). Joins are associative over
/// adjacent execution spans, so any adjacency-preserving shape yields the
/// same session claim — the shape only affects scheduling. The `mid` is
/// carried in the node (not recomputed) so every consumer agrees on the
/// shape. An M7 probe (2026-07-04) reshaped this tree to isolate the
/// final leaf under the root so early joins could finish the whole
/// `(0, N-1)` subtree during the segment phase; it collapsed the
/// post-segment tail 5.0 -> 3.3 s but stretched the segment window by
/// more (xgboost 14348 -> 14554 ms) — a second concurrent recursion
/// proof under the segment phase costs more than it hides.
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
fn join_tree_internal_nodes(segment_count: usize) -> Vec<(usize, usize, usize)> {
    let mut internal_nodes = Vec::new();
    let mut stack = vec![(0usize, segment_count)];
    while let Some((lo, hi)) = stack.pop() {
        if hi - lo > 1 {
            let mid = lo + (hi - lo).div_ceil(2);
            internal_nodes.push((lo, hi, mid));
            stack.push((lo, mid));
            stack.push((mid, hi));
        }
    }
    internal_nodes
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
impl ProverImpl {
    fn webgpu_hal(&self) -> Result<Rc<WebGpuHal>> {
        self.webgpu_hal
            .clone()
            .ok_or_else(|| anyhow!("browser WebGPU HAL is not initialized"))
    }

    pub(crate) async fn prove_with_ctx_async(
        &self,
        env: ExecutorEnv<'_>,
        ctx: &VerifierContext,
        elf: &[u8],
    ) -> Result<ProveInfo> {
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
        let session = {
            let _timer = WebGpuStageTimer::new("execute");
            ExecutorImpl::from_elf(env, elf)?
                .run_with_callback(|segment| Ok(Box::new(SimpleSegmentRef::new(segment))))?
        };
        self.prove_session_async(ctx, &session).await
    }

    pub(crate) async fn compress_async(
        &self,
        opts: &ProverOpts,
        receipt: &Receipt,
    ) -> Result<Receipt> {
        match &receipt.inner {
            InnerReceipt::Composite(inner) => match opts.receipt_kind {
                ReceiptKind::Composite => Ok(receipt.clone()),
                ReceiptKind::Succinct => {
                    let succinct_receipt = self.composite_to_succinct_async(inner).await?;
                    Ok(Receipt::new(
                        InnerReceipt::Succinct(succinct_receipt),
                        receipt.journal.bytes.clone(),
                    ))
                }
                ReceiptKind::Groth16 => {
                    bail!("browser WebGPU async compression to Groth16 is not implemented")
                }
            },
            InnerReceipt::Succinct(_inner) => match opts.receipt_kind {
                ReceiptKind::Composite | ReceiptKind::Succinct => Ok(receipt.clone()),
                ReceiptKind::Groth16 => {
                    bail!("browser WebGPU async compression to Groth16 is not implemented")
                }
            },
            InnerReceipt::Groth16(_) => Ok(receipt.clone()),
            InnerReceipt::Fake(_) => {
                ensure!(
                    opts.dev_mode(),
                    "dev mode must be enabled to compress fake receipts"
                );
                Ok(receipt.clone())
            }
        }
    }

    async fn prove_session_async(
        &self,
        ctx: &VerifierContext,
        session: &Session,
    ) -> Result<ProveInfo> {
        tracing::debug!(
            "prove_session: exit_code = {:?}, journal = {:?}, segments: {}",
            session.exit_code,
            session.journal.as_ref().map(hex::encode),
            session.segments.len()
        );
        let _timer = WebGpuStageTimer::new(format!(
            "prove_session_async segments={} pending_keccaks={} assumptions={}",
            session.segments.len(),
            session.pending_keccaks.len(),
            session.assumptions.len()
        ));
        let prove_session_wall_start = js_sys::Date::now();
        // SP6d iter 4: use the HAL's per-instance counter so a multi-HAL
        // pool doesn't over-count by other slots' GPU work.
        let prove_session_active_start = self
            .webgpu_hal()
            .map(|hal| hal.gpu_active_ms())
            .unwrap_or(0.0);

        ensure!(
            self.opts.hashfn == "poseidon2",
            "provided `ProverOpts` has unsupported `hashfn` value of \"{}\"; \
            supported `hashfn` values are: \"poseidon2\".",
            &self.opts.hashfn
        );
        ensure!(
            session.povw_job_id.is_none(),
            "browser WebGPU async proving does not yet support PoVW receipts"
        );

        // M6d: two-deep segment pipeline. A segment prove is a CPU-heavy
        // witgen phase (preflight + buffer setup + rayon witness
        // generation) followed by a GPU-heavy commit phase (transcript
        // commits + eval_check + FRI, dominated by queue drains and
        // readback waits). Segments are independent proofs, so segment
        // N+1's witgen phase runs under segment N's commit tail —
        // `try_join` polls both on this thread; the M5 worker pool grinds
        // witgen while commit awaits its readbacks. Depth stays at 2: a
        // third in-flight segment costs ~350 MB of shadows for no overlap
        // gain (the witgen phase is the shorter stage), and commit phases
        // never overlap each other (readbacks serialize on the one device
        // queue anyway). Per-prove witgen state rides on the job's private
        // circuit HAL; the zkp-level GPU-authoritative flag is isolated
        // per stage future via `with_authoritative_context`, because the
        // plain scope guards assume stack discipline that interleaved
        // awaits violate.
        //
        // Hook ordering caveat: `on_pre_prove_segment(N+1)` fires when
        // N+1's witgen phase starts, i.e. before segment N's receipt
        // exists. Empty in every browser fixture; noted for future hook
        // users.
        // M7b/M7c: early lifts under the segment phase. A lift's only
        // input is one finished SegmentReceipt, and lifting is the leaf
        // tier of the composite_to_succinct join tree — so for Succinct
        // receipts ONE dedicated recursion device lifts segment i the
        // moment commit(i) lands, instead of idling until the whole
        // segment phase ends. Only non-final segments qualify: the final
        // segment's claim gets the session output merged after this loop.
        //
        // Scheduling physics (measured on xgboost, 11 segments,
        // 2026-07-04):
        // - M7a is the prerequisite — recursion preflight/witgen/verify
        //   CPU runs on pool workers, so an in-flight lift no longer
        //   blocks this thread's readback callbacks mid-commit (the M6d
        //   iteration-1 starvation).
        // - Width is capped at ONE early proof at a time, measured twice:
        //   two concurrent lifts hid 12.0 s of span but stretched the
        //   segment window 6.9 -> 9.8 s (xgboost 14695 vs 14348 ms at
        //   width 1); adding the second device back as a join-only worker
        //   collapsed the post-segment tail 5.0 -> 3.3 s but stretched
        //   the window to 11.1 s (14554 ms) with gpu_idle_ratio DOWN
        //   (0.30 -> 0.22) — a second concurrent recursion proof costs
        //   more than it hides, part un-offloaded main-thread CPU (zkp
        //   transcript + FRI continuations, ~0.2 s/proof), part physical
        //   GPU contention with the commit kernels. One lift device
        //   (cadence ~0.7 s vs segment cadence ~0.9 s) is the measured
        //   optimum of this machinery family.
        // - Early futures only advance while an await polls them, so both
        //   pipeline awaits below drive `early_tasks` alongside the main
        //   future via `future::select`; anything still in flight when
        //   the loop ends is drained before the keccak phase claims the
        //   recursion devices. Unstarted leftovers seed the join-tree
        //   scheduler in composite_to_succinct.
        use futures::future::FutureExt;
        use futures::stream::{FuturesUnordered, StreamExt};
        type EarlyTask<'a> = std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<(
                            (usize, usize),
                            SuccinctReceipt<ReceiptClaim>,
                            Rc<WebGpuHal>,
                        )>,
                    > + 'a,
            >,
        >;
        let segment_count = session.segments.len();
        let early_enabled = self.opts.receipt_kind == ReceiptKind::Succinct
            && segment_count > 1
            && !self.webgpu_recursion_hals.is_empty();
        let mut early_tasks: FuturesUnordered<EarlyTask<'_>> = FuturesUnordered::new();
        let mut early_done: BTreeMap<(usize, usize), SuccinctReceipt<ReceiptClaim>> =
            BTreeMap::new();
        let mut early_spawned: HashSet<(usize, usize)> = HashSet::new();
        let mut lift_queue: VecDeque<(usize, SegmentReceipt)> = VecDeque::new();
        let mut lift_hal: Option<Rc<WebGpuHal>> = if early_enabled {
            self.webgpu_recursion_hals.first().cloned()
        } else {
            None
        };
        let spawn_lift = |hal: Rc<WebGpuHal>, index: usize, seg: SegmentReceipt| {
            risc0_zkp::hal::webgpu::with_authoritative_context(hal.clone(), async move {
                let lifted = self.lift_async_on(&seg, hal.clone()).await?;
                Ok(((index, index + 1), lifted, hal))
            })
            .boxed_local()
        };

        let mut segments = Vec::new();
        let mut staged = None;
        for index in 0..segment_count {
            let current = match staged.take() {
                Some(staged) => staged,
                None => {
                    risc0_zkp::hal::webgpu::with_authoritative_context(
                        self.webgpu_hal()?,
                        self.segment_witgen_stage_async(session, index),
                    )
                    .await?
                }
            };
            let receipt = if index + 1 < segment_count {
                let hal = self.webgpu_hal()?;
                let commit_fut = risc0_zkp::hal::webgpu::with_authoritative_context(
                    hal.clone(),
                    self.segment_commit_stage_async(ctx, session, current),
                );
                let witgen_fut = risc0_zkp::hal::webgpu::with_authoritative_context(
                    hal,
                    self.segment_witgen_stage_async(session, index + 1),
                );
                let mut main = Box::pin(futures::future::try_join(commit_fut, witgen_fut));
                let (receipt, next) = loop {
                    if early_tasks.is_empty() {
                        break main.await?;
                    }
                    match futures::future::select(main, early_tasks.select_next_some()).await {
                        futures::future::Either::Left((joined, _early_next)) => break joined?,
                        futures::future::Either::Right((completed, main_rest)) => {
                            main = main_rest;
                            let (node, early_receipt, hal) = completed?;
                            early_done.insert(node, early_receipt);
                            match lift_queue.pop_front() {
                                Some((qi, qseg)) => {
                                    early_spawned.insert((qi, qi + 1));
                                    early_tasks.push(spawn_lift(hal, qi, qseg));
                                }
                                None => lift_hal = Some(hal),
                            }
                        }
                    }
                };
                staged = Some(next);
                receipt
            } else {
                let mut main = Box::pin(risc0_zkp::hal::webgpu::with_authoritative_context(
                    self.webgpu_hal()?,
                    self.segment_commit_stage_async(ctx, session, current),
                ));
                loop {
                    if early_tasks.is_empty() {
                        break main.await?;
                    }
                    match futures::future::select(main, early_tasks.select_next_some()).await {
                        futures::future::Either::Left((receipt, _early_next)) => break receipt?,
                        futures::future::Either::Right((completed, main_rest)) => {
                            main = main_rest;
                            let (node, early_receipt, hal) = completed?;
                            early_done.insert(node, early_receipt);
                            match lift_queue.pop_front() {
                                Some((qi, qseg)) => {
                                    early_spawned.insert((qi, qi + 1));
                                    early_tasks.push(spawn_lift(hal, qi, qseg));
                                }
                                None => lift_hal = Some(hal),
                            }
                        }
                    }
                }
            };
            if early_enabled && index + 1 < segment_count {
                lift_queue.push_back((index, receipt.clone()));
                if let Some(hal) = lift_hal.take() {
                    let (qi, qseg) = lift_queue.pop_front().expect("just queued a lift");
                    early_spawned.insert((qi, qi + 1));
                    early_tasks.push(spawn_lift(hal, qi, qseg));
                }
            }
            segments.push(receipt);
        }

        if early_enabled {
            // Await in-flight early tasks so the recursion devices are
            // free for the keccak phase (and so the futures keep being
            // polled at all — nothing after this point drives
            // `early_tasks`). No new work is dispatched here; unstarted
            // leftovers seed the join-tree scheduler, which finishes them
            // with the full device set.
            let _timer = WebGpuStageTimer::new(format!(
                "early_task_drain in_flight={} queued_lifts={} done_nodes={}",
                early_tasks.len(),
                lift_queue.len(),
                early_done.len()
            ));
            while let Some(completed) = early_tasks.next().await {
                let (node, early_receipt, _hal) = completed?;
                early_done.insert(node, early_receipt);
            }
        }

        let (assumptions, session_assumption_receipts): (Vec<_>, Vec<_>) =
            session.assumptions.iter().cloned().unzip();

        segments
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
            .ok_or_else(|| anyhow!("composite receipt verifier parameters missing from context"))?
            .digest();

        let mut zkr_receipts = HashMap::new();
        let mut keccak_receipts = VecDeque::new();
        // M4e: pipeline keccak proofs across the dedicated recursion
        // devices while the union peak-stack consumes receipts strictly
        // in request order on the main device. The union tree shape is
        // protocol-fixed (the guest computes the same binary-counter
        // tree over the request digests), so the scheduler only overlaps
        // work — consumption order is a correctness requirement. Union
        // inserts run as tasks inside the same FuturesUnordered (owning
        // the peak stack for their turn) so in-flight keccak proofs keep
        // being polled while a union proof awaits its readbacks.
        if !session.pending_keccaks.is_empty() {
            use futures::future::FutureExt;
            use futures::stream::{FuturesUnordered, StreamExt};

            enum KeccakPhaseTask {
                Keccak(usize, SuccinctReceipt<Unknown>, Rc<WebGpuHal>),
                Union(VecDeque<(u32, SuccinctReceipt<Unknown>)>, usize),
            }

            let keccak_count = session.pending_keccaks.len();
            // Keccak proofs get the dedicated devices; the main HAL runs
            // the union chain (it carries segment-phase queue residue
            // anyway). Without extras this degenerates to the serial
            // pre-M4e behavior on the main device.
            let main_hal = self.webgpu_hal()?;
            let mut free_hals: Vec<Rc<WebGpuHal>> = self.webgpu_recursion_hals.clone();
            if free_hals.is_empty() {
                free_hals.push(main_hal.clone());
            }
            let mut in_flight: FuturesUnordered<
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<KeccakPhaseTask>> + '_>>,
            > = FuturesUnordered::new();
            let mut done: BTreeMap<usize, SuccinctReceipt<Unknown>> = BTreeMap::new();
            let mut peaks = Some(VecDeque::new());
            let mut next_spawn = 0usize;
            let mut next_union = 0usize;

            while next_union < keccak_count {
                while next_spawn < keccak_count && !free_hals.is_empty() {
                    let index = next_spawn;
                    next_spawn += 1;
                    let proof_request = &session.pending_keccaks[index];
                    let proof_hal = free_hals.pop().expect("checked non-empty free hal list");
                    in_flight.push(
                        risc0_zkp::hal::webgpu::with_authoritative_context(proof_hal.clone(), {
                            async move {
                                let _timer = WebGpuStageTimer::new(format!(
                                    "prove_keccak_request index={index}"
                                ));
                                let receipt =
                                    prove_keccak_webgpu(proof_request, proof_hal.clone()).await?;
                                Ok(KeccakPhaseTask::Keccak(index, receipt, proof_hal))
                            }
                        })
                        .boxed_local(),
                    );
                }
                if peaks.is_some() && done.contains_key(&next_union) {
                    let mut batch = Vec::new();
                    while let Some(receipt) = done.remove(&(next_union + batch.len())) {
                        batch.push(receipt);
                    }
                    let batch_len = batch.len();
                    let mut union_peaks = peaks.take().expect("checked peak stack present");
                    in_flight.push(
                        risc0_zkp::hal::webgpu::with_authoritative_context(main_hal.clone(), {
                            async move {
                                for receipt in batch {
                                    tracing::debug!(
                                        "adding keccak assumption: {}",
                                        receipt.claim.digest()
                                    );
                                    self.insert_union_receipt_async(&mut union_peaks, receipt)
                                        .await?;
                                }
                                Ok(KeccakPhaseTask::Union(union_peaks, batch_len))
                            }
                        })
                        .boxed_local(),
                    );
                }
                let Some(completed) = in_flight.next().await else {
                    bail!(
                        "keccak pipeline stalled: {next_union} of {keccak_count} receipts unioned"
                    );
                };
                match completed? {
                    KeccakPhaseTask::Keccak(index, receipt, proof_hal) => {
                        free_hals.push(proof_hal);
                        done.insert(index, receipt);
                    }
                    KeccakPhaseTask::Union(union_peaks, consumed) => {
                        peaks = Some(union_peaks);
                        next_union += consumed;
                    }
                }
            }
            keccak_receipts = peaks
                .take()
                .expect("keccak pipeline left the peak stack in flight");
        }

        {
            let _timer = WebGpuStageTimer::new("keccak_receipts_root");
            if let Some(root_receipt) = self.union_receipts_root_async(keccak_receipts).await? {
                let assumption = Assumption {
                    claim: root_receipt.claim.digest(),
                    control_root: root_receipt.control_root()?,
                };

                tracing::debug!("keccak root assumption: {:?}", assumption);
                zkr_receipts.insert(assumption, root_receipt.clone());
            }
        }

        let inner_assumption_receipts: Vec<_> = session_assumption_receipts
            .into_iter()
            .map(|assumption_receipt| match assumption_receipt {
                AssumptionReceipt::Proven(receipt) => Ok(receipt),
                AssumptionReceipt::Unresolved(assumption) => {
                    let receipt = zkr_receipts.get(&assumption).ok_or_else(|| {
                        anyhow!("no receipt available for unresolved assumption: {assumption:#?}")
                    })?;
                    Ok(InnerAssumptionReceipt::Succinct(receipt.clone()))
                }
            })
            .collect::<Result<_>>()?;

        let composite_receipt = CompositeReceipt {
            segments,
            assumption_receipts: inner_assumption_receipts,
            verifier_parameters,
        };

        let session_claim = session.claim()?;

        {
            let _timer = WebGpuStageTimer::new("verify_composite");
            composite_receipt.verify_integrity_with_context(ctx)?;
        }
        check_claims(
            &session_claim,
            "composite",
            MaybePruned::Value(composite_receipt.claim()?),
        )?;

        if self.opts.receipt_kind == ReceiptKind::Composite {
            let receipt = Receipt::new(
                InnerReceipt::Composite(composite_receipt),
                session.journal.clone().unwrap_or_default().bytes,
            );
            return Ok(ProveInfo {
                receipt,
                work_receipt: None,
                stats: session.stats(),
            });
        }

        ensure!(
            self.opts.receipt_kind == ReceiptKind::Succinct,
            "browser WebGPU async proving currently supports Composite and Succinct receipts"
        );

        let _timer = WebGpuStageTimer::new("composite_to_succinct_async");
        let succinct_receipt = self
            .composite_to_succinct_with_predone_async(&composite_receipt, early_spawned, early_done)
            .await?;
        drop(_timer);
        let wall_ms: f64 = js_sys::Date::now() - prove_session_wall_start;
        let active_ms: f64 = self
            .webgpu_hal()
            .map(|hal| hal.gpu_active_ms())
            .unwrap_or(0.0)
            - prove_session_active_start;
        let idle_ratio: f64 = if wall_ms > 0.0 {
            (1.0_f64 - active_ms / wall_ms).max(0.0)
        } else {
            0.0
        };
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "prove_session_async wall_ms={wall_ms:.1} gpu_active_ms={active_ms:.1} gpu_idle_ratio={idle_ratio:.3}",
        ));
        let receipt = Receipt::new(
            InnerReceipt::Succinct(succinct_receipt),
            session.journal.clone().unwrap_or_default().bytes,
        );
        Ok(ProveInfo {
            receipt,
            work_receipt: None,
            stats: session.stats(),
        })
    }

    /// M6d pipeline stage A: resolve + preflight + witgen phase for one
    /// segment. Transcript-free, so it may run while another segment's
    /// commit stage is in flight. The witness CPU pass runs on pool
    /// workers so this wasm thread stays free to service that commit
    /// stage's readback callbacks; offloading the (~30 ms) preflight
    /// replay too was probed on 2026-07-04 and measured wall-neutral
    /// (xgboost 15267 -> 15532 ms), so it stays inline.
    async fn segment_witgen_stage_async(
        &self,
        session: &Session,
        index: usize,
    ) -> Result<StagedSegmentJob> {
        let segment = session.segments[index].resolve()?;
        for hook in &session.hooks {
            hook.on_pre_prove_segment(&segment);
        }
        let _timer = WebGpuStageTimer::new(format!(
            "segment_witgen_phase index={} po2={} user_cycles={}",
            segment.index,
            segment.po2(),
            segment.user_cycles()
        ));
        let preflight_results = self.segment_preflight(&segment)?;
        let po2 = preflight_results.inner.po2();
        let job = risc0_circuit_rv32im::prove::webgpu_segment_witgen_phase(
            self.webgpu_hal()?,
            preflight_results.inner,
        )
        .await?;
        Ok(StagedSegmentJob {
            job,
            po2,
            output: preflight_results.output,
            segment_index: preflight_results.segment_index,
            segment,
        })
    }

    /// M6d pipeline stage B: transcript commits + finalize + receipt
    /// decode + verify for a staged segment, then post-prove hooks.
    async fn segment_commit_stage_async(
        &self,
        ctx: &VerifierContext,
        session: &Session,
        staged: StagedSegmentJob,
    ) -> Result<SegmentReceipt> {
        let StagedSegmentJob {
            job,
            po2,
            output,
            segment_index,
            segment,
        } = staged;
        let receipt = self
            .finish_segment_commit_async(ctx, job, po2, output, segment_index)
            .await?;
        for hook in &session.hooks {
            hook.on_post_prove_segment(&segment);
        }
        Ok(receipt)
    }

    /// Commit phase + receipt decode + verify. Shared by the M6d segment
    /// pipeline and the SP6d pool's `prove_segment_core_async` (whose
    /// callers run session hooks themselves).
    async fn finish_segment_commit_async(
        &self,
        ctx: &VerifierContext,
        job: risc0_circuit_rv32im::prove::WebGpuSegmentJob,
        po2: u32,
        output: Option<crate::Output>,
        segment_index: u32,
    ) -> Result<SegmentReceipt> {
        let _timer = WebGpuStageTimer::new(format!(
            "segment_commit_phase index={segment_index} po2={po2}"
        ));
        let seal = risc0_circuit_rv32im::prove::webgpu_segment_commit_phase(job).await?;
        let mut claim = ReceiptClaim::decode_from_seal_v2(&seal, Some(po2))?;
        claim.output = output.into();

        let verifier_parameters = ctx
            .segment_verifier_parameters
            .as_ref()
            .ok_or_else(|| anyhow!("segment receipt verifier parameters missing from context"))?
            .digest();
        let receipt = SegmentReceipt {
            seal,
            index: segment_index,
            hashfn: self.opts.hashfn.clone(),
            claim,
            verifier_parameters,
        };
        {
            let _timer = WebGpuStageTimer::new(format!("verify_segment index={}", receipt.index));
            receipt
                .verify_integrity_with_context(ctx)
                .with_context(|| format!("verify segment index={}", receipt.index))?;
        }
        Ok(receipt)
    }

    /// Serial composition of the M6d pipeline stages, kept for callers
    /// that schedule segments themselves (the SP6d multi-device pool
    /// orchestrator). Hooks are the caller's responsibility here.
    pub(crate) async fn prove_segment_core_async(
        &self,
        ctx: &VerifierContext,
        preflight_results: PreflightResults,
    ) -> Result<SegmentReceipt> {
        tracing::debug!("prove_segment_core_async");
        let po2 = preflight_results.inner.po2();
        let _timer = WebGpuStageTimer::new(format!(
            "prove_segment_core_async index={} po2={po2}",
            preflight_results.segment_index
        ));

        ensure!(
            self.opts.hashfn == "poseidon2",
            "provided `ProverOpts` has unsupported `hashfn` value of \"{}\"; \
            supported `hashfn` values are: \"poseidon2\".",
            &self.opts.hashfn
        );

        let job = risc0_circuit_rv32im::prove::webgpu_segment_witgen_phase(
            self.webgpu_hal()?,
            preflight_results.inner,
        )
        .await?;
        self.finish_segment_commit_async(
            ctx,
            job,
            po2,
            preflight_results.output,
            preflight_results.segment_index,
        )
        .await
    }

    async fn lift_async_on(
        &self,
        receipt: &SegmentReceipt,
        hal: Rc<WebGpuHal>,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        let _timer = WebGpuStageTimer::new(format!("lift_async segment_index={}", receipt.index));
        let receipt = {
            let _timer =
                WebGpuStageTimer::new(format!("lift_prove_async segment_index={}", receipt.index));
            crate::host::recursion::prove::lift_webgpu(receipt, hal).await?
        };
        let receipt = {
            let _timer = WebGpuStageTimer::new("verify_lift");
            verify_integrity_offloaded(receipt, |r| r.verify_integrity().context("verify lift"))
                .await?
        };
        Ok(receipt)
    }

    async fn join_async_on(
        &self,
        a: &SuccinctReceipt<ReceiptClaim>,
        b: &SuccinctReceipt<ReceiptClaim>,
        hal: Rc<WebGpuHal>,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        let _timer = WebGpuStageTimer::new("join_async");
        let receipt = {
            let _timer = WebGpuStageTimer::new("join_prove_async");
            crate::host::recursion::prove::join_webgpu(a, b, hal).await?
        };
        let receipt = {
            let _timer = WebGpuStageTimer::new("verify_join");
            verify_integrity_offloaded(receipt, |r| r.verify_integrity().context("verify join"))
                .await?
        };
        Ok(receipt)
    }

    pub(crate) async fn resolve_async(
        &self,
        conditional: &SuccinctReceipt<ReceiptClaim>,
        assumption: &SuccinctReceipt<Unknown>,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        let _timer = WebGpuStageTimer::new("resolve_async");
        let receipt = {
            let _timer = WebGpuStageTimer::new("resolve_prove_async");
            crate::host::recursion::prove::resolve_webgpu(
                conditional,
                assumption,
                self.webgpu_hal()?,
            )
            .await?
        };
        let receipt = {
            let _timer = WebGpuStageTimer::new("verify_resolve");
            verify_integrity_offloaded(receipt, |r| r.verify_integrity().context("verify resolve"))
                .await?
        };
        Ok(receipt)
    }

    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    async fn union_unknown_async(
        &self,
        a: &SuccinctReceipt<Unknown>,
        b: &SuccinctReceipt<Unknown>,
    ) -> Result<SuccinctReceipt<Unknown>> {
        let _timer = WebGpuStageTimer::new("union_async");
        let receipt = {
            let _timer = WebGpuStageTimer::new("union_prove_async");
            crate::host::recursion::prove::union_webgpu(a, b, self.webgpu_hal()?).await?
        };
        let receipt = {
            let _timer = WebGpuStageTimer::new("verify_union");
            verify_integrity_offloaded(receipt, |r| r.verify_integrity().context("verify union"))
                .await?
        };
        Ok(receipt.into_unknown())
    }

    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    pub(crate) async fn insert_union_receipt_async(
        &self,
        peaks: &mut VecDeque<(u32, SuccinctReceipt<Unknown>)>,
        item: SuccinctReceipt<Unknown>,
    ) -> Result<()> {
        let mut to_add = (0, item);
        while peaks.back().is_some_and(|(height, _)| *height == to_add.0) {
            let (_, to_merge) = peaks
                .pop_back()
                .expect("checked non-empty keccak union peak stack");
            to_add = (
                to_add.0 + 1,
                self.union_unknown_async(&to_add.1, &to_merge).await?,
            );
        }
        peaks.push_back(to_add);
        Ok(())
    }

    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    pub(crate) async fn union_receipts_root_async(
        &self,
        mut peaks: VecDeque<(u32, SuccinctReceipt<Unknown>)>,
    ) -> Result<Option<SuccinctReceipt<Unknown>>> {
        let Some((_, mut item)) = peaks.pop_front() else {
            return Ok(None);
        };
        for (_, peak) in peaks {
            item = self.union_unknown_async(&item, &peak).await?;
        }
        Ok(Some(item))
    }

    pub(crate) async fn composite_to_succinct_async(
        &self,
        composite_receipt: &CompositeReceipt,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        self.composite_to_succinct_with_predone_async(
            composite_receipt,
            HashSet::new(),
            BTreeMap::new(),
        )
        .await
    }

    /// [`Self::composite_to_succinct_async`] with join-tree nodes that
    /// were already proven (M7: early lifts run under the segment phase).
    /// `predone` holds finished, unconsumed node receipts; `prespawned`
    /// additionally covers nodes whose receipts were already consumed.
    /// Today the early machinery only produces leaves, which exist in any
    /// tree shape; if it ever produces internal nodes again, it must use
    /// the same [`join_tree_internal_nodes`] shape as this scheduler.
    pub(crate) async fn composite_to_succinct_with_predone_async(
        &self,
        composite_receipt: &CompositeReceipt,
        prespawned: HashSet<(usize, usize)>,
        predone: BTreeMap<(usize, usize), SuccinctReceipt<ReceiptClaim>>,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        // M3c: run the lift/join phase as a balanced join tree under a
        // width-2 scheduler instead of a serial left fold.
        //
        // Joins are associative over adjacent execution spans, so any
        // pairing that preserves segment order yields the same session
        // claim; a balanced tree cuts the dependency depth from N-1 to
        // ceil(log2 N), turning the join chain from the critical path
        // into schedulable work. Two proofs run interleaved at a time
        // (recursion∥recursion is the concurrency SP6d validated for
        // wasm32 memory; segment∥anything OOMs at po2=18): while one
        // blocks on a merkle-root readback (GPU busy), the other's CPU
        // witgen fills the wait. Each proof future carries a private
        // copy of the HAL's GPU-authoritative flag
        // (`with_authoritative_context`) because the plain scope guards
        // assume stack discipline, which interleaved awaits violate.
        use futures::future::FutureExt;
        use futures::stream::{FuturesUnordered, StreamExt};

        let hal = self.webgpu_hal()?;
        let segment_count = composite_receipt.segments.len();
        ensure!(
            segment_count > 0,
            "malformed composite receipt has no continuation segment receipts"
        );

        let internal_nodes = join_tree_internal_nodes(segment_count);

        // One HAL per in-flight proof. A readback (`mapAsync`) waits for
        // everything previously submitted on its device queue, so proofs
        // sharing one device serialize at every transcript readback
        // (measured: wall ≈ CPU + GPU). Distinct devices have independent
        // queues. The main HAL goes first in the list so `pop()` hands
        // proofs the dedicated recursion devices before the main one,
        // whose queue may still hold residual segment-phase work. Without
        // extra devices, fall back to interleaving on the main one —
        // still slightly better than a serial fold.
        let mut free_hals: Vec<Rc<WebGpuHal>> = if segment_count > 1 {
            let mut hals = vec![hal.clone()];
            hals.extend(self.webgpu_recursion_hals.iter().cloned());
            if hals.len() == 1 {
                hals.push(hal.clone());
            }
            hals
        } else {
            vec![hal.clone()]
        };
        type ProofResult = Result<(usize, usize, SuccinctReceipt<ReceiptClaim>, Rc<WebGpuHal>)>;
        let mut done: BTreeMap<(usize, usize), SuccinctReceipt<ReceiptClaim>> = BTreeMap::new();
        let mut spawned: HashSet<(usize, usize)> = HashSet::new();
        for &(lo, hi) in &prespawned {
            ensure!(
                hi <= segment_count
                    && (hi == lo + 1 || internal_nodes.iter().any(|&(l, h, _)| (l, h) == (lo, hi))),
                "pre-proven node ({lo}, {hi}) is not part of the join tree over {segment_count} segments"
            );
        }
        for (node, receipt) in predone {
            ensure!(
                prespawned.contains(&node),
                "pre-proven node ({}, {}) missing from the prespawned set",
                node.0,
                node.1
            );
            done.insert(node, receipt);
        }
        spawned.extend(prespawned);
        let mut next_leaf = 0usize;
        let mut in_flight: FuturesUnordered<
            std::pin::Pin<Box<dyn std::future::Future<Output = ProofResult> + '_>>,
        > = FuturesUnordered::new();

        loop {
            while !free_hals.is_empty() {
                // Skip leaves that arrived pre-lifted.
                while next_leaf < segment_count && spawned.contains(&(next_leaf, next_leaf + 1)) {
                    next_leaf += 1;
                }
                // Prefer ready joins: they release receipts and advance
                // the tree toward the root; lifts are the slack work.
                let ready_join = internal_nodes.iter().copied().find(|&(lo, hi, mid)| {
                    !spawned.contains(&(lo, hi))
                        && done.contains_key(&(lo, mid))
                        && done.contains_key(&(mid, hi))
                });
                if let Some((lo, hi, mid)) = ready_join {
                    spawned.insert((lo, hi));
                    let left = done.remove(&(lo, mid)).expect("checked ready join left");
                    let right = done.remove(&(mid, hi)).expect("checked ready join right");
                    let proof_hal = free_hals.pop().expect("checked non-empty free hal list");
                    in_flight.push(
                        risc0_zkp::hal::webgpu::with_authoritative_context(proof_hal.clone(), {
                            async move {
                                let joined =
                                    self.join_async_on(&left, &right, proof_hal.clone()).await?;
                                Ok((lo, hi, joined, proof_hal))
                            }
                        })
                        .boxed_local(),
                    );
                } else if next_leaf < segment_count {
                    let index = next_leaf;
                    next_leaf += 1;
                    spawned.insert((index, index + 1));
                    let segment = &composite_receipt.segments[index];
                    let proof_hal = free_hals.pop().expect("checked non-empty free hal list");
                    in_flight.push(
                        risc0_zkp::hal::webgpu::with_authoritative_context(proof_hal.clone(), {
                            async move {
                                let lifted = self.lift_async_on(segment, proof_hal.clone()).await?;
                                Ok((index, index + 1, lifted, proof_hal))
                            }
                        })
                        .boxed_local(),
                    );
                } else {
                    break;
                }
            }
            let Some(completed) = in_flight.next().await else {
                break;
            };
            let (lo, hi, receipt, proof_hal) = completed?;
            free_hals.push(proof_hal);
            done.insert((lo, hi), receipt);
        }

        let mut continuation_receipt = done
            .remove(&(0, segment_count))
            .expect("join tree scheduler must complete the root node");

        for assumption in composite_receipt.assumption_receipts.iter() {
            continuation_receipt = match assumption {
                InnerAssumptionReceipt::Succinct(assumption) => {
                    self.resolve_async(&continuation_receipt, assumption).await?
                }
                InnerAssumptionReceipt::Composite(assumption) => {
                    let assumption_receipt =
                        Box::pin(self.composite_to_succinct_async(assumption)).await?;
                    self.resolve_async(
                        &continuation_receipt,
                        &SuccinctReceipt::<ReceiptClaim>::into_unknown(assumption_receipt),
                    )
                    .await?
                }
                InnerAssumptionReceipt::Fake(_) => bail!(
                    "compressing composite receipts with fake receipt assumptions is not supported"
                ),
                InnerAssumptionReceipt::Groth16(_) => bail!(
                    "compressing composite receipts with Groth16 receipt assumptions is not supported"
                ),
            };
        }

        Ok(continuation_receipt)
    }
}

impl ProverServer for ProverImpl {
    fn prove(&self, env: ExecutorEnv<'_>, elf: &[u8]) -> Result<ProveInfo> {
        let ctx = VerifierContext::default().with_dev_mode(self.opts.dev_mode());
        self.prove_with_ctx(env, &ctx, elf)
    }

    fn prove_with_ctx(
        &self,
        env: ExecutorEnv<'_>,
        ctx: &VerifierContext,
        elf: &[u8],
    ) -> Result<ProveInfo> {
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        {
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
            let session = {
                let _timer = WebGpuStageTimer::new("execute");
                ExecutorImpl::from_elf(env, elf)?
                    .run_with_callback(|segment| Ok(Box::new(SimpleSegmentRef::new(segment))))?
            };
            return self.prove_session(ctx, &session);
        }

        #[cfg(not(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown")))]
        {
            let session = ExecutorImpl::from_elf(env, elf)?.run()?;
            self.prove_session(ctx, &session)
        }
    }

    fn prove_session(&self, ctx: &VerifierContext, session: &Session) -> Result<ProveInfo> {
        tracing::debug!(
            "prove_session: exit_code = {:?}, journal = {:?}, segments: {}",
            session.exit_code,
            session.journal.as_ref().map(hex::encode),
            session.segments.len()
        );
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let _timer = WebGpuStageTimer::new(format!(
            "prove_session segments={} pending_keccaks={} assumptions={}",
            session.segments.len(),
            session.pending_keccaks.len(),
            session.assumptions.len()
        ));

        ensure!(
            self.opts.hashfn == "poseidon2",
            "provided `ProverOpts` has unsupported `hashfn` value of \"{}\"; \
            supported `hashfn` values are: \"poseidon2\".",
            &self.opts.hashfn
        );

        let mut segments = Vec::new();
        for segment_ref in session.segments.iter() {
            let segment = segment_ref.resolve()?;
            for hook in &session.hooks {
                hook.on_pre_prove_segment(&segment);
            }
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new(format!(
                "prove_segment index={} po2={} user_cycles={}",
                segment.index,
                segment.po2(),
                segment.user_cycles()
            ));
            segments.push(self.prove_segment(ctx, &segment)?);
            for hook in &session.hooks {
                hook.on_post_prove_segment(&segment);
            }
        }

        let (assumptions, session_assumption_receipts): (Vec<_>, Vec<_>) =
            session.assumptions.iter().cloned().unzip();

        // Merge the output, including journal digest and assumptions, into the last segment.
        segments
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
            .ok_or_else(|| anyhow!("composite receipt verifier parameters missing from context"))?
            .digest();

        let mut zkr_receipts = HashMap::new();
        let mut keccak_receipts: MerkleMountainAccumulator<UnionPeak> =
            MerkleMountainAccumulator::new();
        for (_idx, proof_request) in session.pending_keccaks.iter().enumerate() {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new(format!("prove_keccak_request index={_idx}"));
            let receipt = prove_keccak(proof_request)?;
            tracing::debug!("adding keccak assumption: {}", receipt.claim.digest());
            keccak_receipts.insert(receipt)?;
        }

        // NOTE: Calling keccak_receipts.root() proves the union tree.
        {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new("keccak_receipts_root");
            if let Ok(root_receipt) = keccak_receipts.root() {
                let assumption = Assumption {
                    claim: root_receipt.claim.digest(),
                    control_root: root_receipt.control_root()?,
                };

                tracing::debug!("keccak root assumption: {:?}", assumption);
                zkr_receipts.insert(assumption, root_receipt.clone());
            }
        }

        // TODO: add test case for when a single session refers to the same assumption multiple times
        let inner_assumption_receipts: Vec<_> = session_assumption_receipts
            .into_iter()
            .map(|assumption_receipt| match assumption_receipt {
                AssumptionReceipt::Proven(receipt) => Ok(receipt),
                AssumptionReceipt::Unresolved(assumption) => {
                    let receipt = zkr_receipts.get(&assumption).ok_or_else(|| {
                        anyhow!("no receipt available for unresolved assumption: {assumption:#?}")
                    })?;
                    Ok(InnerAssumptionReceipt::Succinct(receipt.clone()))
                }
            })
            .collect::<Result<_>>()?;

        let composite_receipt = CompositeReceipt {
            segments,
            assumption_receipts: inner_assumption_receipts,
            verifier_parameters,
        };

        let session_claim = session.claim()?;

        // Verify the receipt to catch if something is broken in the proving process.
        // NOTE: If the proof is very large, this could take > 1s, e.g. with 1000 segments.
        {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new("verify_composite");
            composite_receipt.verify_integrity_with_context(ctx)?;
        }
        check_claims(
            &session_claim,
            "composite",
            MaybePruned::Value(composite_receipt.claim()?),
        )?;

        if self.opts.receipt_kind == ReceiptKind::Composite {
            let receipt = Receipt::new(
                InnerReceipt::Composite(composite_receipt),
                session.journal.clone().unwrap_or_default().bytes,
            );
            return Ok(ProveInfo {
                receipt,
                work_receipt: None,
                stats: session.stats(),
            });
        }

        let (succinct_receipt, work_receipt) = match session.povw_job_id.is_some() {
            true => {
                #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
                let _timer = WebGpuStageTimer::new("composite_to_succinct_povw");
                let work_receipt = self.composite_to_succinct_povw(&composite_receipt)?;
                let unwrapped = self.unwrap_povw(&work_receipt)?;
                (unwrapped, Some(work_receipt))
            }
            false => {
                #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
                let _timer = WebGpuStageTimer::new("composite_to_succinct");
                (self.composite_to_succinct(&composite_receipt)?, None)
            }
        };

        if self.opts.receipt_kind == ReceiptKind::Succinct {
            let receipt = Receipt::new(
                InnerReceipt::Succinct(succinct_receipt),
                session.journal.clone().unwrap_or_default().bytes,
            );
            return Ok(ProveInfo {
                receipt,
                work_receipt: work_receipt.map(Into::into),
                stats: session.stats(),
            });
        }

        let groth16_receipt = self.succinct_to_groth16(&succinct_receipt)?;

        if self.opts.receipt_kind == ReceiptKind::Groth16 {
            let receipt = Receipt::new(
                InnerReceipt::Groth16(groth16_receipt),
                session.journal.clone().unwrap_or_default().bytes,
            );
            return Ok(ProveInfo {
                receipt,
                work_receipt: work_receipt.map(Into::into),
                stats: session.stats(),
            });
        }

        // As long as the checks above are exhaustive, this code is unreachable. If this statement
        // is reached, this is an implementation error.
        unreachable!(
            "proving not implemented for receipt kind {:?}",
            self.opts.receipt_kind
        );
    }

    fn segment_preflight(&self, segment: &Segment) -> Result<PreflightResults> {
        tracing::debug!("segment_preflight");
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let _timer = WebGpuStageTimer::new(format!(
            "segment_preflight index={} po2={} user_cycles={}",
            segment.index,
            segment.po2(),
            segment.user_cycles()
        ));

        ensure!(
            segment.po2() <= self.opts.max_segment_po2,
            "segment po2 exceeds max on ProverOpts: {} > {}",
            segment.po2(),
            self.opts.max_segment_po2
        );
        let inner = self.segment_prover()?.preflight(&segment.inner)?;

        Ok(PreflightResults {
            inner,
            terminate_state: segment.inner.claim.terminate_state,
            output: segment.output.clone(),
            segment_index: segment.index,
        })
    }

    fn prove_segment_core(
        &self,
        ctx: &VerifierContext,
        preflight_results: PreflightResults,
    ) -> Result<SegmentReceipt> {
        tracing::debug!("prove_segment_core");
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let _timer = WebGpuStageTimer::new(format!(
            "prove_segment_core index={} po2={}",
            preflight_results.segment_index,
            preflight_results.inner.po2()
        ));

        ensure!(
            self.opts.hashfn == "poseidon2",
            "provided `ProverOpts` has unsupported `hashfn` value of \"{}\"; \
            supported `hashfn` values are: \"poseidon2\".",
            &self.opts.hashfn
        );

        let po2 = preflight_results.inner.po2();
        let seal = {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new(format!(
                "segment_prove_core index={} po2={po2}",
                preflight_results.segment_index
            ));
            self.segment_prover()?.prove_core(preflight_results.inner)?
        };
        let mut claim = ReceiptClaim::decode_from_seal_v2(&seal, Some(po2))?;
        claim.output = preflight_results.output.into();

        let verifier_parameters = ctx
            .segment_verifier_parameters
            .as_ref()
            .ok_or_else(|| anyhow!("segment receipt verifier parameters missing from context"))?
            .digest();
        let receipt = SegmentReceipt {
            seal,
            index: preflight_results.segment_index,
            hashfn: self.opts.hashfn.clone(),
            claim,
            verifier_parameters,
        };
        {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new(format!("verify_segment index={}", receipt.index));
            receipt
                .verify_integrity_with_context(ctx)
                .with_context(|| format!("verify segment index={}", receipt.index))?;
        }

        Ok(receipt)
    }

    fn lift(&self, receipt: &SegmentReceipt) -> Result<SuccinctReceipt<ReceiptClaim>> {
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let _timer = WebGpuStageTimer::new(format!("lift segment_index={}", receipt.index));
        let receipt = {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer =
                WebGpuStageTimer::new(format!("lift_prove segment_index={}", receipt.index));
            lift(receipt)?
        };
        {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new("verify_lift");
            receipt.verify_integrity().context("verify lift")?;
        }
        Ok(receipt)
    }

    fn lift_povw(
        &self,
        receipt: &SegmentReceipt,
    ) -> Result<SuccinctReceipt<WorkClaim<ReceiptClaim>>> {
        lift_povw(receipt)
    }

    fn join(
        &self,
        a: &SuccinctReceipt<ReceiptClaim>,
        b: &SuccinctReceipt<ReceiptClaim>,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let _timer = WebGpuStageTimer::new("join");
        let receipt = {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new("join_prove");
            join(a, b)?
        };
        {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new("verify_join");
            receipt.verify_integrity().context("verify join")?;
        }
        Ok(receipt)
    }

    fn join_povw(
        &self,
        a: &SuccinctReceipt<WorkClaim<ReceiptClaim>>,
        b: &SuccinctReceipt<WorkClaim<ReceiptClaim>>,
    ) -> Result<SuccinctReceipt<WorkClaim<ReceiptClaim>>> {
        join_povw(a, b)
    }

    fn join_unwrap_povw(
        &self,
        a: &SuccinctReceipt<WorkClaim<ReceiptClaim>>,
        b: &SuccinctReceipt<WorkClaim<ReceiptClaim>>,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        join_unwrap_povw(a, b)
    }

    fn resolve(
        &self,
        conditional: &SuccinctReceipt<ReceiptClaim>,
        assumption: &SuccinctReceipt<Unknown>,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let _timer = WebGpuStageTimer::new("resolve");
        let receipt = {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new("resolve_prove");
            resolve(conditional, assumption)?
        };
        {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new("verify_resolve");
            receipt.verify_integrity().context("verify resolve")?;
        }
        Ok(receipt)
    }

    fn resolve_povw(
        &self,
        conditional: &SuccinctReceipt<WorkClaim<ReceiptClaim>>,
        assumption: &SuccinctReceipt<Unknown>,
    ) -> Result<SuccinctReceipt<WorkClaim<ReceiptClaim>>> {
        resolve_povw(conditional, assumption)
    }

    fn resolve_unwrap_povw(
        &self,
        conditional: &SuccinctReceipt<WorkClaim<ReceiptClaim>>,
        assumption: &SuccinctReceipt<Unknown>,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        resolve_unwrap_povw(conditional, assumption)
    }

    fn identity_p254(
        &self,
        a: &SuccinctReceipt<ReceiptClaim>,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        // TODO: figure out how to verify this
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let _timer = WebGpuStageTimer::new("identity_p254");
        identity_p254(a)
    }

    fn prove_keccak(
        &self,
        request: &crate::ProveKeccakRequest,
    ) -> Result<SuccinctReceipt<Unknown>> {
        // TODO: figure out how to verify this
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let _timer = WebGpuStageTimer::new("prove_keccak");
        prove_keccak(request)
    }

    fn union(
        &self,
        a: &SuccinctReceipt<Unknown>,
        b: &SuccinctReceipt<Unknown>,
    ) -> Result<SuccinctReceipt<UnionClaim>> {
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let _timer = WebGpuStageTimer::new("union");
        let receipt = {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new("union_prove");
            union(a, b)?
        };
        {
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = WebGpuStageTimer::new("verify_union");
            receipt.verify_integrity().context("verify union")?;
        }
        Ok(receipt)
    }

    fn unwrap_povw(
        &self,
        a: &SuccinctReceipt<WorkClaim<ReceiptClaim>>,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        unwrap_povw(a)
    }
}

fn check_claims(
    session_claim: &ReceiptClaim,
    other_name: &str,
    other_claim: MaybePruned<ReceiptClaim>,
) -> Result<()> {
    let session_claim_digest = session_claim.digest();
    let other_claim_digest = other_claim.digest();
    if session_claim_digest != other_claim_digest {
        tracing::debug!("session claim and {other_name} do not match");
        tracing::debug!("session claim: {session_claim:#?}");
        tracing::debug!("{other_name} claim: {other_claim:#?}");
        bail!(
            "session claim: {} != {other_name} claim: {}",
            hex::encode(session_claim_digest),
            hex::encode(other_claim_digest)
        );
    }
    Ok(())
}
