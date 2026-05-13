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

use anyhow::Result;
use risc0_zkp::{
    core::hash::poseidon2::Poseidon2HashSuite,
    hal::webgpu::WebGpuHal,
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
}
