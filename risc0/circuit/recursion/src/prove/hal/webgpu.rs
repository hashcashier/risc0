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

use std::{collections::BTreeMap, rc::Rc};

use anyhow::Result;
use risc0_circuit_recursion_sys::{RawPreflightTrace, StepMode};
use risc0_zkp::{
    field::baby_bear::{BabyBearElem, BabyBearExtElem},
    hal::{
        webgpu::{WebGpuBuffer, WebGpuHal},
        AccumPreflight, CircuitHal,
    },
};

use crate::{
    prove::{RecursionProver, RecursionProverImpl},
    CircuitImpl,
};

use super::{CircuitAccumulator, CircuitWitnessGenerator};

#[derive(Default)]
#[allow(dead_code)]
pub(crate) struct WebGpuCircuitHal;

impl CircuitWitnessGenerator<WebGpuHal> for WebGpuCircuitHal {
    fn generate_witness(
        &self,
        mode: StepMode,
        total_cycles: u32,
        preflight: &RawPreflightTrace,
        byte_reads: &BTreeMap<usize, Vec<u32>>,
        ctrl: &WebGpuBuffer<BabyBearElem>,
        data: &WebGpuBuffer<BabyBearElem>,
        global: &WebGpuBuffer<BabyBearElem>,
    ) -> Result<()> {
        super::rust_kernels::generate_witness(
            mode,
            total_cycles,
            preflight,
            byte_reads,
            ctrl,
            data,
            global,
        )
    }
}

impl CircuitAccumulator<WebGpuHal> for WebGpuCircuitHal {
    fn accumulate(
        &self,
        work_cycles: u32,
        total_cycles: u32,
        ctrl: &WebGpuBuffer<BabyBearElem>,
        global: &WebGpuBuffer<BabyBearElem>,
        data: &WebGpuBuffer<BabyBearElem>,
        mix: &WebGpuBuffer<BabyBearElem>,
        accum: &WebGpuBuffer<BabyBearElem>,
    ) -> Result<()> {
        super::rust_kernels::accumulate(
            work_cycles,
            total_cycles,
            ctrl,
            global,
            data,
            mix,
            accum,
        )
    }
}

impl CircuitHal<WebGpuHal> for WebGpuCircuitHal {
    fn eval_check(
        &self,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        globals: &[&WebGpuBuffer<BabyBearElem>],
        poly_mix: BabyBearExtElem,
        po2: usize,
        steps: usize,
    ) {
        risc0_zkp::hal::portable::eval_check::<WebGpuHal, CircuitImpl>(
            &CircuitImpl::new(),
            check,
            groups,
            globals,
            poly_mix,
            po2,
            steps,
        );
    }

    fn accumulate(
        &self,
        _preflight: &AccumPreflight,
        _ctrl: &WebGpuBuffer<BabyBearElem>,
        _io: &WebGpuBuffer<BabyBearElem>,
        _data: &WebGpuBuffer<BabyBearElem>,
        _mix: &WebGpuBuffer<BabyBearElem>,
        _accum: &WebGpuBuffer<BabyBearElem>,
        _steps: usize,
    ) {
        unimplemented!("browser WebGPU recursion accumulation kernel is not wired yet")
    }
}

pub fn recursion_prover(hal: Rc<WebGpuHal>) -> Result<Box<dyn RecursionProver>> {
    Ok(Box::new(RecursionProverImpl::new(
        hal,
        Rc::new(WebGpuCircuitHal),
    )))
}
