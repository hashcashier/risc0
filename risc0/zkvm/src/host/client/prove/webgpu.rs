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
        exec::executor::ExecutorImpl, prove::get_webgpu_prover_server, session::NullSegmentRef,
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

    fn with_circuit_hal<T>(&self, f: impl FnOnce() -> T) -> T {
        risc0_circuit_rv32im::prove::with_webgpu_hal(self.hal.clone(), || {
            risc0_circuit_keccak::prove::with_webgpu_hal(self.hal.clone(), || {
                risc0_circuit_recursion::prove::with_webgpu_hal(self.hal.clone(), f)
            })
        })
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
