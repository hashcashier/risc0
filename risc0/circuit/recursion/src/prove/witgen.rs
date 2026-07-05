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

use anyhow::{Context as _, Result};
use risc0_circuit_recursion_sys::{RawPreflightTrace, StepMode};
use risc0_core::scope;
use risc0_zkp::{
    adapter::{CircuitInfo as _, TapsProvider as _},
    core::digest::Digest,
    field::{
        baby_bear::{BabyBear, BabyBearElem, BabyBearExtElem},
        Elem as _,
    },
    hal::Hal,
};

use crate::{CircuitImpl, CIRCUIT};

use super::{preflight::Preflight, CircuitAccumulator, CircuitWitnessGenerator, Program};

pub(crate) struct WitnessGenerator<H: Hal> {
    work_cycles: u32,
    total_cycles: u32,
    pub global: H::Buffer<H::Elem>,
    pub ctrl: H::Buffer<H::Elem>,
    pub data: H::Buffer<H::Elem>,
    pub accum: H::Buffer<H::Elem>,
}

/// Buffers allocated for one recursion witness generation, before the
/// witness pass has filled them. [`WitnessGenerator::new`] splits at
/// this point so the browser WebGPU path can run the CPU witness pass on a
/// pool worker between [`WitnessGenerator::alloc_buffers`] and
/// [`WitnessGenerator::finish_after_generate`].
pub(crate) struct WitgenBuffers<H: Hal> {
    pub work_cycles: u32,
    pub total_cycles: u32,
    pub global: H::Buffer<H::Elem>,
    pub ctrl: H::Buffer<H::Elem>,
    pub data: H::Buffer<H::Elem>,
    pub accum: H::Buffer<H::Elem>,
}

impl<H> WitnessGenerator<H>
where
    H: Hal<Field = BabyBear, Elem = BabyBearElem, ExtElem = BabyBearExtElem>,
{
    pub fn alloc_buffers(
        hal: &H,
        zkr: &Program,
        ctrl_cache_key: Option<Digest>,
    ) -> Result<WitgenBuffers<H>> {
        let total_cycles = 1 << zkr.po2;

        let global = vec![BabyBearElem::INVALID; CircuitImpl::OUTPUT_SIZE];
        let global = hal.copy_from_elem("global", &global);

        // populate the ctrl buffer
        let ctrl_size = CIRCUIT.ctrl_size();
        assert_eq!(ctrl_size, zkr.code_size);
        let ctrl = hal
            .copy_from_elem_transpose_zero_pad(
                "ctrl",
                "recursion_ctrl_compact",
                &zkr.code,
                zkr.code_rows(),
                ctrl_size,
                total_cycles,
                ctrl_cache_key,
            )
            .context("recursion ctrl construction failure")?;

        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let alloc_timer =
            risc0_zkp::hal::webgpu::WebGpuStageTimer::new("recursion_witgen_alloc_init");
        let data = hal.alloc_elem_init(
            "recursion_data",
            total_cycles * CIRCUIT.data_size(),
            BabyBearElem::INVALID,
        );
        let accum = hal.alloc_elem_init(
            "accum",
            total_cycles * CIRCUIT.accum_size(),
            BabyBearElem::INVALID,
        );
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        drop(alloc_timer);

        Ok(WitgenBuffers {
            work_cycles: zkr.code_rows() as u32,
            total_cycles: total_cycles as u32,
            global,
            ctrl,
            data,
            accum,
        })
    }

    pub fn raw_trace(preflight: &Preflight, work_cycles: u32) -> RawPreflightTrace {
        RawPreflightTrace {
            wom: preflight.trace.wom.as_ptr(),
            cycles: preflight.trace.cycles.as_ptr(),
            iops: preflight.trace.iops.as_ptr(),
            num_woms: preflight.trace.wom.len() as u32,
            num_cycles: work_cycles,
            num_iops: preflight.trace.iops.len() as u32,
        }
    }

    /// Noise, zeroize, and the deferred GPU witness dispatches — everything
    /// after the CPU witness pass has filled `bufs`.
    pub fn finish_after_generate<C: CircuitWitnessGenerator<H>>(
        hal: &H,
        circuit_hal: &C,
        bufs: WitgenBuffers<H>,
        preflight: &Preflight,
        witness_mode: StepMode,
    ) -> Result<Self> {
        let WitgenBuffers {
            work_cycles,
            total_cycles,
            global,
            ctrl,
            data,
            accum,
        } = bufs;
        let total_cycles = total_cycles as usize;
        let raw_trace = Self::raw_trace(preflight, work_cycles);

        // Add random noise to end of the data columns
        scope!("noise", {
            use risc0_zkp::ZK_CYCLES;
            let mut rng = rand::rng();
            let noise = vec![BabyBearElem::random(&mut rng); ZK_CYCLES * CIRCUIT.data_size()];
            hal.eltwise_copy_elem_slice(
                &data,
                &noise,
                CIRCUIT.data_size(),      // from_rows
                ZK_CYCLES,                // from_cols
                0,                        // from_offset
                ZK_CYCLES,                // from_stride
                total_cycles - ZK_CYCLES, // into_offset
                total_cycles,             // into_stride
            );
        });

        // Zero out 'invalid' entries in data and output.
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let zeroize_timer =
            risc0_zkp::hal::webgpu::WebGpuStageTimer::new("recursion_witgen_zeroize");
        scope!("zeroize", {
            hal.eltwise_zeroize_elem(&data);
            hal.eltwise_zeroize_elem(&global);
        });
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        drop(zeroize_timer);

        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let post_zeroize_timer =
            risc0_zkp::hal::webgpu::WebGpuStageTimer::new("recursion_witgen_post_zeroize");
        circuit_hal
            .post_witness_zeroize(
                hal,
                witness_mode,
                total_cycles as u32,
                &raw_trace,
                preflight.byte_reads(),
                &ctrl,
                &data,
                &global,
            )
            .context("post-zeroize witness generation failure")?;
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        drop(post_zeroize_timer);

        Ok(Self {
            work_cycles,
            total_cycles: total_cycles as u32,
            global,
            ctrl,
            data,
            accum,
        })
    }

    pub fn new<C: CircuitWitnessGenerator<H>>(
        hal: &H,
        circuit_hal: &C,
        zkr: &Program,
        preflight: &Preflight,
        ctrl_cache_key: Option<Digest>,
    ) -> Result<Self> {
        scope!("witgen");

        let bufs = Self::alloc_buffers(hal, zkr, ctrl_cache_key)?;
        let raw_trace = Self::raw_trace(preflight, bufs.work_cycles);

        let witness_mode = StepMode::Parallel;
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        let generate_timer =
            risc0_zkp::hal::webgpu::WebGpuStageTimer::new("recursion_witgen_generate");
        circuit_hal
            .generate_witness(
                witness_mode,
                bufs.total_cycles,
                &raw_trace,
                preflight.byte_reads(),
                &bufs.ctrl,
                &bufs.data,
                &bufs.global,
            )
            .context("witness generation failure")?;
        #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
        drop(generate_timer);

        Self::finish_after_generate(hal, circuit_hal, bufs, preflight, witness_mode)
    }

    pub fn accum<C: CircuitAccumulator<H>>(
        &self,
        hal: &H,
        circuit_hal: &C,
        mix: &[BabyBearElem],
    ) -> Result<H::Buffer<H::Elem>> {
        scope!("accum");

        let mix = hal.copy_from_elem("mix", mix);

        // Add random noise to end of accum
        scope!("noise", {
            use risc0_zkp::ZK_CYCLES;
            let mut rng = rand::rng();
            let total_cycles = self.total_cycles as usize;
            let noise = vec![BabyBearElem::random(&mut rng); ZK_CYCLES * CIRCUIT.accum_size()];
            hal.eltwise_copy_elem_slice(
                &self.accum,
                &noise,
                CIRCUIT.accum_size(),     // from_rows
                ZK_CYCLES,                // from_cols
                0,                        // from_offset
                ZK_CYCLES,                // from_stride
                total_cycles - ZK_CYCLES, // into_offset
                total_cycles,             // into_stride
            );
        });

        let accumulation_mode = circuit_hal.accumulate(
            hal,
            self.work_cycles,
            self.total_cycles,
            &self.ctrl,
            &self.global,
            &self.data,
            &mix,
            &self.accum,
        )?;

        scope!("zeroize", {
            circuit_hal.zeroize_after_accumulate(hal, accumulation_mode, &self.accum, &self.global);
        });

        Ok(mix)
    }
}
