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

use std::collections::BTreeMap;

use anyhow::Result;
use risc0_circuit_recursion_sys::{RawPreflightTrace, StepMode};
use risc0_zkp::hal::Hal;

pub(crate) mod cpu;
#[cfg(feature = "cuda")]
pub(crate) mod cuda;
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub(crate) mod rust_kernels;
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub(crate) mod webgpu;
// #[cfg(all(
//     feature = "prove",
//     any(all(target_os = "macos", target_arch = "aarch64"), target_os = "ios")
// ))]
// pub mod metal;

pub(crate) trait CircuitWitnessGenerator<H: Hal> {
    fn generate_witness(
        &self,
        mode: StepMode,
        total_cycles: u32,
        preflight: &RawPreflightTrace,
        byte_reads: &BTreeMap<usize, Vec<u32>>,
        ctrl: &H::Buffer<H::Elem>,
        data: &H::Buffer<H::Elem>,
        global: &H::Buffer<H::Elem>,
    ) -> Result<()>;

    #[allow(clippy::too_many_arguments)]
    fn post_witness_zeroize(
        &self,
        _hal: &H,
        _mode: StepMode,
        _total_cycles: u32,
        _preflight: &RawPreflightTrace,
        _byte_reads: &BTreeMap<usize, Vec<u32>>,
        _ctrl: &H::Buffer<H::Elem>,
        _data: &H::Buffer<H::Elem>,
        _global: &H::Buffer<H::Elem>,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CircuitAccumulationMode {
    Default,
    // Only the WebGPU HAL produces (and consumes) GPU-authoritative
    // accumulation; the variant exists exactly where that HAL compiles.
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    GpuAuthoritative,
}

pub(crate) trait CircuitAccumulator<H: Hal> {
    #[allow(clippy::too_many_arguments)]
    fn accumulate(
        &self,
        hal: &H,
        work_cycles: u32,
        total_cycles: u32,
        ctrl: &H::Buffer<H::Elem>,
        global: &H::Buffer<H::Elem>,
        data: &H::Buffer<H::Elem>,
        mix: &H::Buffer<H::Elem>,
        accum: &H::Buffer<H::Elem>,
    ) -> Result<CircuitAccumulationMode>;

    fn zeroize_after_accumulate(
        &self,
        hal: &H,
        _mode: CircuitAccumulationMode,
        accum: &H::Buffer<H::Elem>,
        global: &H::Buffer<H::Elem>,
    ) {
        hal.eltwise_zeroize_elem(accum);
        hal.eltwise_zeroize_elem(global);
    }
}
