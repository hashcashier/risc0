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
use risc0_core::scope;
use risc0_zkp::hal::{
    webgpu::{WebGpuBuffer, WebGpuHal},
    AccumPreflight, Buffer, CircuitHal,
};

use crate::{
    prove::{KeccakProver, KeccakProverImpl},
    zirgen::{circuit::Val, CircuitImpl},
};

use super::{
    CircuitWitnessGenerator, ForwardPreflightOrder, MetaBuffer, PreflightCycleOrder,
    PreflightTrace, StepMode,
};

#[derive(Default)]
#[allow(dead_code)]
pub(crate) struct WebGpuCircuitHal;

impl CircuitWitnessGenerator<WebGpuHal> for WebGpuCircuitHal {
    type PreferredPreflightOrder = ForwardPreflightOrder;

    fn scatter_preflight(
        &self,
        into: &MetaBuffer<WebGpuHal>,
        infos: &[risc0_circuit_keccak_sys::ScatterInfo],
        data: &[u32],
    ) -> Result<()> {
        scope!("scatter");
        into.buf.view_mut(|into_slice| {
            for info in infos {
                let inner_count = 32 / info.bits;
                let mask: u32 = (1 << info.bits) - 1;
                for i in 0..info.count as u32 {
                    let from_idx = info.offset + (i / inner_count);
                    let word = data[from_idx as usize];
                    let j = i % inner_count;
                    let val = (word >> (j * info.bits)) & mask;
                    let col = info.col as u32 + i;
                    let into_idx = col as usize * into.rows + info.row as usize;
                    into_slice[into_idx] = val.into();
                }
            }
        });
        Ok(())
    }

    fn generate_witness<O: PreflightCycleOrder>(
        &self,
        mode: StepMode,
        preflight: &PreflightTrace<O>,
        global: &MetaBuffer<WebGpuHal>,
        data: &MetaBuffer<WebGpuHal>,
    ) -> Result<()> {
        super::rust_steps::generate_witness(mode, preflight, global, data)
    }
}

impl CircuitHal<WebGpuHal> for WebGpuCircuitHal {
    fn accumulate(
        &self,
        _preflight: &AccumPreflight,
        _ctrl: &WebGpuBuffer<Val>,
        _io: &WebGpuBuffer<Val>,
        _data: &WebGpuBuffer<Val>,
        _mix: &WebGpuBuffer<Val>,
        _accum: &WebGpuBuffer<Val>,
        _steps: usize,
    ) {
    }

    fn eval_check(
        &self,
        check: &WebGpuBuffer<Val>,
        groups: &[&WebGpuBuffer<Val>],
        globals: &[&WebGpuBuffer<Val>],
        poly_mix: <WebGpuHal as risc0_zkp::hal::Hal>::ExtElem,
        po2: usize,
        steps: usize,
    ) {
        risc0_zkp::hal::portable::eval_check::<WebGpuHal, CircuitImpl>(
            &CircuitImpl,
            check,
            groups,
            globals,
            poly_mix,
            po2,
            steps,
        );
    }
}

pub fn keccak_prover(hal: Rc<WebGpuHal>) -> Result<Box<dyn KeccakProver>> {
    Ok(Box::new(KeccakProverImpl {
        hal,
        circuit_hal: Rc::new(WebGpuCircuitHal),
    }))
}
