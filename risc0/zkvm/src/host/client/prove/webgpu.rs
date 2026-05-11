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

use std::rc::Rc;

use anyhow::Result;
use risc0_zkp::{
    core::hash::poseidon2::Poseidon2HashSuite,
    hal::webgpu::{WebGpuDiagnostics, WebGpuHal},
};

use super::{Executor, Prover, ProverOpts};
use crate::{
    host::server::{
        exec::executor::ExecutorImpl,
        prove::{compress_webgpu, get_webgpu_prover_server, prove_webgpu_with_ctx},
        session::NullSegmentRef,
    },
    ExecutorEnv, ProveInfo, Receipt, SegmentInfo, SessionInfo, VerifierContext,
};

/// Construct a browser WebGPU prover.
pub async fn webgpu_prover() -> Result<Rc<WebGpuProver>> {
    Ok(Rc::new(WebGpuProver::new().await?))
}

/// Browser WebGPU prover entrypoint.
pub struct WebGpuProver {
    name: String,
    hal: Rc<WebGpuHal>,
}

impl WebGpuProver {
    /// Request a WebGPU device from the browser and construct a prover.
    pub async fn new() -> Result<Self> {
        let hal = WebGpuHal::new(Poseidon2HashSuite::new_suite()).await?;
        Ok(Self::from_hal("webgpu", Rc::new(hal)))
    }

    /// Construct a prover from a caller-supplied WebGPU HAL.
    pub fn from_hal(name: &str, hal: Rc<WebGpuHal>) -> Self {
        Self {
            name: name.to_string(),
            hal,
        }
    }

    /// Return backend usage diagnostics accumulated by the underlying WebGPU HAL.
    pub fn diagnostics(&self) -> WebGpuDiagnostics {
        self.hal.diagnostics()
    }

    /// Reset backend usage diagnostics accumulated by the underlying WebGPU HAL.
    pub fn reset_diagnostics(&self) {
        self.hal.reset_diagnostics();
    }

    /// Enable or disable WebGPU eval_check dispatch for diagnostics.
    #[doc(hidden)]
    pub fn set_eval_check_gpu_enabled(&self, enabled: bool) {
        self.hal.set_eval_check_gpu_enabled(enabled);
    }

    /// Enable or disable a specific WebGPU HAL kernel for diagnostics.
    #[doc(hidden)]
    pub fn set_webgpu_op_gpu_enabled(&self, op: &str, enabled: bool) {
        self.hal.set_op_gpu_enabled(op, enabled);
    }

    /// Enable or disable RV32IM async GPU-authoritative proof stages for diagnostics.
    #[doc(hidden)]
    pub fn set_rv32im_async_gpu_authoritative_scopes(&self, code_data: bool, accum_finalize: bool) {
        risc0_circuit_rv32im::prove::set_webgpu_async_authoritative_scopes(
            code_data,
            accum_finalize,
        );
    }

    /// Enable or disable individual RV32IM async GPU-authoritative proof stages for diagnostics.
    #[doc(hidden)]
    pub fn set_rv32im_async_gpu_authoritative_stages(
        &self,
        code_data: bool,
        accum_commit: bool,
        finalize: bool,
    ) {
        risc0_circuit_rv32im::prove::set_webgpu_async_authoritative_stages(
            code_data,
            accum_commit,
            finalize,
        );
    }

    /// Enable or disable individual RV32IM ACCUM commit sub-stages for diagnostics.
    #[doc(hidden)]
    pub fn set_rv32im_accum_commit_gpu_authoritative_stages(
        &self,
        make_coeffs: bool,
        poly_group: bool,
        merkle: bool,
    ) {
        risc0_circuit_rv32im::prove::set_webgpu_accum_commit_authoritative_stages(
            make_coeffs,
            poly_group,
            merkle,
        );
    }

    fn with_circuit_hal<T>(&self, f: impl FnOnce() -> T) -> T {
        risc0_circuit_rv32im::prove::with_webgpu_hal(self.hal.clone(), || {
            risc0_circuit_keccak::prove::with_webgpu_hal(self.hal.clone(), || {
                risc0_circuit_recursion::prove::with_webgpu_hal(self.hal.clone(), f)
            })
        })
    }

    /// Async browser WebGPU variant of [`Prover::prove`].
    pub async fn prove_async(&self, env: ExecutorEnv<'_>, elf: &[u8]) -> Result<ProveInfo> {
        let opts = ProverOpts::default();
        let ctx = VerifierContext::default();
        self.prove_with_ctx_async(env, &ctx, elf, &opts).await
    }

    /// Async browser WebGPU variant of [`Prover::prove_with_opts`].
    pub async fn prove_with_opts_async(
        &self,
        env: ExecutorEnv<'_>,
        elf: &[u8],
        opts: &ProverOpts,
    ) -> Result<ProveInfo> {
        let ctx = VerifierContext::default().with_dev_mode(opts.dev_mode());
        self.prove_with_ctx_async(env, &ctx, elf, opts).await
    }

    /// Async browser WebGPU variant of [`Prover::prove_with_ctx`].
    pub async fn prove_with_ctx_async(
        &self,
        env: ExecutorEnv<'_>,
        ctx: &VerifierContext,
        elf: &[u8],
        opts: &ProverOpts,
    ) -> Result<ProveInfo> {
        prove_webgpu_with_ctx(opts, self.hal.clone(), env, ctx, elf).await
    }

    /// Async browser WebGPU variant of [`Prover::compress`].
    pub async fn compress_async(&self, opts: &ProverOpts, receipt: &Receipt) -> Result<Receipt> {
        compress_webgpu(opts, self.hal.clone(), receipt).await
    }
}

impl Prover for WebGpuProver {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn prove_with_ctx(
        &self,
        env: ExecutorEnv<'_>,
        ctx: &VerifierContext,
        elf: &[u8],
        opts: &ProverOpts,
    ) -> Result<ProveInfo> {
        self.with_circuit_hal(|| {
            get_webgpu_prover_server(opts, self.hal.clone())?.prove_with_ctx(env, ctx, elf)
        })
    }

    fn compress(&self, opts: &ProverOpts, receipt: &Receipt) -> Result<Receipt> {
        self.with_circuit_hal(|| {
            get_webgpu_prover_server(opts, self.hal.clone())?.compress(opts, receipt)
        })
    }
}

impl Executor for WebGpuProver {
    fn execute(&self, env: ExecutorEnv<'_>, elf: &[u8]) -> Result<SessionInfo> {
        let mut segments = Vec::new();
        let session = ExecutorImpl::from_elf(env, elf)?.run_with_callback(|segment| {
            segments.push(SegmentInfo {
                po2: segment.po2() as u32,
                cycles: segment.user_cycles(),
            });
            Ok(Box::new(NullSegmentRef))
        })?;

        let receipt_claim = session.claim()?;
        Ok(SessionInfo {
            segments,
            journal: session.journal.unwrap_or_default(),
            exit_code: session.exit_code,
            receipt_claim: Some(receipt_claim),
        })
    }
}
