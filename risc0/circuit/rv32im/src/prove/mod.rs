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

mod hal;
#[cfg(test)]
mod tests;
mod witgen;

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
};

use anyhow::Result;
use cfg_if::cfg_if;
use risc0_core::scope;
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
use risc0_zkp::hal::webgpu::WebGpuHal;

use crate::execute::segment::Segment;

pub use witgen::PreflightResults;

const GLOBAL_MIX: usize = 0;
const GLOBAL_OUT: usize = 1;

pub type Seal = Vec<u32>;

pub trait SegmentProver {
    fn prove(&self, segment: &Segment) -> Result<Seal> {
        scope!("prove");
        let results = self.preflight(segment)?;
        self.prove_core(results)
    }

    fn preflight(&self, segment: &Segment) -> Result<PreflightResults>;

    fn prove_core(&self, preflight_results: PreflightResults) -> Result<Seal>;

    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    fn prove_core_async<'a>(
        &'a self,
        preflight_results: PreflightResults,
    ) -> Pin<Box<dyn Future<Output = Result<Seal>> + 'a>> {
        Box::pin(async move { self.prove_core(preflight_results) })
    }
}

pub fn segment_prover() -> Result<Box<dyn SegmentProver>> {
    cfg_if! {
        if #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))] {
            if let Some(hal) = current_webgpu_hal() {
                self::hal::webgpu::segment_prover(hal)
            } else {
                anyhow::bail!("segment_prover cannot synchronously initialize browser WebGPU")
            }
        } else if #[cfg(feature = "cuda")] {
            self::hal::cuda::segment_prover()
        // } else if #[cfg(any(all(target_os = "macos", target_arch = "aarch64"), target_os = "ios"))] {
        // self::hal::metal::segment_prover(hashfn)
        } else {
            self::hal::cpu::segment_prover()
        }
    }
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub fn segment_prover_with_hal(hal: Rc<WebGpuHal>) -> Result<Box<dyn SegmentProver>> {
    self::hal::webgpu::segment_prover(hal)
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
thread_local! {
    static WEBGPU_HAL: RefCell<Option<Rc<WebGpuHal>>> = RefCell::new(None);
    static WEBGPU_ASYNC_AUTHORITATIVE_SCOPES: Cell<WebGpuAsyncAuthoritativeScopes> =
        Cell::new(WebGpuAsyncAuthoritativeScopes::all_enabled());
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
#[derive(Clone, Copy)]
pub(crate) struct WebGpuAsyncAuthoritativeScopes {
    pub(crate) code_data: bool,
    pub(crate) accum_make_coeffs: bool,
    pub(crate) accum_poly_group: bool,
    pub(crate) accum_merkle: bool,
    pub(crate) finalize: bool,
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
impl WebGpuAsyncAuthoritativeScopes {
    const fn all_enabled() -> Self {
        Self {
            code_data: true,
            accum_make_coeffs: true,
            accum_poly_group: true,
            accum_merkle: true,
            finalize: true,
        }
    }
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub fn set_webgpu_async_authoritative_scopes(code_data: bool, accum_finalize: bool) {
    set_webgpu_async_authoritative_stages(code_data, accum_finalize, accum_finalize);
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub fn set_webgpu_async_authoritative_stages(code_data: bool, accum_commit: bool, finalize: bool) {
    WEBGPU_ASYNC_AUTHORITATIVE_SCOPES.with(|scopes| {
        scopes.set(WebGpuAsyncAuthoritativeScopes {
            code_data,
            accum_make_coeffs: accum_commit,
            accum_poly_group: accum_commit,
            accum_merkle: accum_commit,
            finalize,
        });
    });
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub fn set_webgpu_accum_commit_authoritative_stages(
    make_coeffs: bool,
    poly_group: bool,
    merkle: bool,
) {
    WEBGPU_ASYNC_AUTHORITATIVE_SCOPES.with(|scopes| {
        let mut current = scopes.get();
        current.accum_make_coeffs = make_coeffs;
        current.accum_poly_group = poly_group;
        current.accum_merkle = merkle;
        scopes.set(current);
    });
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub(crate) fn webgpu_async_authoritative_scopes() -> WebGpuAsyncAuthoritativeScopes {
    WEBGPU_ASYNC_AUTHORITATIVE_SCOPES.with(Cell::get)
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub fn with_webgpu_hal<T>(hal: Rc<WebGpuHal>, f: impl FnOnce() -> T) -> T {
    struct Reset(Option<Rc<WebGpuHal>>);

    impl Drop for Reset {
        fn drop(&mut self) {
            WEBGPU_HAL.with(|slot| {
                slot.replace(self.0.take());
            });
        }
    }

    WEBGPU_HAL.with(|slot| {
        let previous = slot.replace(Some(hal));
        let _reset = Reset(previous);
        f()
    })
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
fn current_webgpu_hal() -> Option<Rc<WebGpuHal>> {
    WEBGPU_HAL.with(|slot| slot.borrow().clone())
}
