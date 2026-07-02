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
use std::collections::VecDeque;
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
struct WebGpuStageTimer {
    label: String,
    start_ms: f64,
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
impl WebGpuStageTimer {
    fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "browser-prove:stage start {label}"
        )));
        Self {
            label,
            start_ms: js_sys::Date::now(),
        }
    }
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
impl Drop for WebGpuStageTimer {
    fn drop(&mut self) {
        let elapsed_ms = js_sys::Date::now() - self.start_ms;
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "browser-prove:stage done {} elapsed_ms={elapsed_ms:.0}",
            self.label
        )));
    }
}

/// An implementation of a Prover that runs locally.
pub struct ProverImpl {
    opts: ProverOpts,
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    webgpu_hal: Option<Rc<WebGpuHal>>,
}

impl ProverImpl {
    /// Construct a [ProverImpl].
    pub fn new(opts: ProverOpts) -> Self {
        Self {
            opts,
            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            webgpu_hal: None,
        }
    }

    /// Construct a browser WebGPU [ProverImpl] from an initialized HAL.
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    pub fn new_webgpu(opts: ProverOpts, hal: Rc<WebGpuHal>) -> Self {
        Self {
            opts,
            webgpu_hal: Some(hal),
        }
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

        let mut segments = Vec::new();
        for segment_ref in session.segments.iter() {
            let segment = segment_ref.resolve()?;
            for hook in &session.hooks {
                hook.on_pre_prove_segment(&segment);
            }
            let _timer = WebGpuStageTimer::new(format!(
                "prove_segment_async index={} po2={} user_cycles={}",
                segment.index,
                segment.po2(),
                segment.user_cycles()
            ));
            let preflight_results = self.segment_preflight(&segment)?;
            segments.push(
                self.prove_segment_core_async(ctx, preflight_results)
                    .await?,
            );
            for hook in &session.hooks {
                hook.on_post_prove_segment(&segment);
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
        for (_idx, proof_request) in session.pending_keccaks.iter().enumerate() {
            let _timer = WebGpuStageTimer::new(format!("prove_keccak_request index={_idx}"));
            let receipt = prove_keccak_webgpu(proof_request, self.webgpu_hal()?).await?;
            tracing::debug!("adding keccak assumption: {}", receipt.claim.digest());
            self.insert_union_receipt_async(&mut keccak_receipts, receipt)
                .await?;
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
        let succinct_receipt = self.composite_to_succinct_async(&composite_receipt).await?;
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

    pub(crate) async fn prove_segment_core_async(
        &self,
        ctx: &VerifierContext,
        preflight_results: PreflightResults,
    ) -> Result<SegmentReceipt> {
        tracing::debug!("prove_segment_core_async");
        let _timer = WebGpuStageTimer::new(format!(
            "prove_segment_core_async index={} po2={}",
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
            let _timer = WebGpuStageTimer::new(format!(
                "segment_prove_core_async index={} po2={po2}",
                preflight_results.segment_index
            ));
            self.segment_prover()?
                .prove_core_async(preflight_results.inner)
                .await?
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
            let _timer = WebGpuStageTimer::new(format!("verify_segment index={}", receipt.index));
            receipt
                .verify_integrity_with_context(ctx)
                .with_context(|| format!("verify segment index={}", receipt.index))?;
        }

        Ok(receipt)
    }

    async fn lift_async(&self, receipt: &SegmentReceipt) -> Result<SuccinctReceipt<ReceiptClaim>> {
        let _timer = WebGpuStageTimer::new(format!("lift_async segment_index={}", receipt.index));
        let receipt = {
            let _timer =
                WebGpuStageTimer::new(format!("lift_prove_async segment_index={}", receipt.index));
            crate::host::recursion::prove::lift_webgpu(receipt, self.webgpu_hal()?).await?
        };
        {
            let _timer = WebGpuStageTimer::new("verify_lift");
            receipt.verify_integrity().context("verify lift")?;
        }
        Ok(receipt)
    }

    async fn join_async(
        &self,
        a: &SuccinctReceipt<ReceiptClaim>,
        b: &SuccinctReceipt<ReceiptClaim>,
    ) -> Result<SuccinctReceipt<ReceiptClaim>> {
        let _timer = WebGpuStageTimer::new("join_async");
        let receipt = {
            let _timer = WebGpuStageTimer::new("join_prove_async");
            crate::host::recursion::prove::join_webgpu(a, b, self.webgpu_hal()?).await?
        };
        {
            let _timer = WebGpuStageTimer::new("verify_join");
            receipt.verify_integrity().context("verify join")?;
        }
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
        {
            let _timer = WebGpuStageTimer::new("verify_resolve");
            receipt.verify_integrity().context("verify resolve")?;
        }
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
        {
            let _timer = WebGpuStageTimer::new("verify_union");
            receipt.verify_integrity().context("verify union")?;
        }
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
        let mut continuation_receipt = None;
        for right in composite_receipt.segments.iter() {
            let lifted = self.lift_async(right).await?;
            continuation_receipt = Some(match continuation_receipt {
                Some(left) => self.join_async(&left, &lifted).await?,
                None => lifted,
            });
        }
        let mut continuation_receipt = continuation_receipt.ok_or_else(|| {
            anyhow!("malformed composite receipt has no continuation segment receipts")
        })?;

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
