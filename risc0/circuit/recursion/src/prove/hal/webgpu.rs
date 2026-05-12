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
    adapter::{CircuitInfo as _, PROOF_SYSTEM_INFO},
    field::{
        baby_bear::{BabyBearElem, BabyBearExtElem},
        Elem as _,
    },
    hal::{
        webgpu::{WebGpuBuffer, WebGpuCircuitEvalCheck, WebGpuHal, WebGpuStageTimer},
        AccumPreflight, Buffer, CircuitHal, Hal,
    },
    prove::Prover,
};

use crate::{
    prove::{preflight::Preflight, RecursionProver, RecursionProverImpl, RecursionReceipt},
    taps::TAPSET,
    CircuitImpl, REGISTER_GROUP_ACCUM, REGISTER_GROUP_CTRL, REGISTER_GROUP_DATA,
};

use super::{CircuitAccumulator, CircuitWitnessGenerator};

#[derive(Default)]
#[allow(dead_code)]
pub(crate) struct WebGpuCircuitHal;

impl WebGpuCircuitEvalCheck for WebGpuCircuitHal {
    fn eval_check_webgpu(
        &self,
        hal: &WebGpuHal,
        check: &WebGpuBuffer<BabyBearElem>,
        groups: &[&WebGpuBuffer<BabyBearElem>],
        globals: &[&WebGpuBuffer<BabyBearElem>],
        poly_mix: BabyBearExtElem,
        po2: usize,
        steps: usize,
    ) -> Result<bool> {
        hal.dispatch_eval_check_poly_ext(
            check,
            groups,
            globals,
            TAPSET,
            &crate::poly_ext::DEF,
            poly_mix,
            po2,
            steps,
        )
    }
}

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
        let _timer = WebGpuStageTimer::new(format!(
            "recursion_witgen mode={} total_cycles={} preflight_cycles={} wom={} iops={}",
            step_mode_label(mode),
            total_cycles,
            preflight.num_cycles,
            preflight.num_woms,
            preflight.num_iops
        ));
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
        let _timer = WebGpuStageTimer::new(format!(
            "recursion_accumulate work_cycles={} total_cycles={}",
            work_cycles, total_cycles
        ));
        super::rust_kernels::accumulate(work_cycles, total_cycles, ctrl, global, data, mix, accum)
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
        let _timer = WebGpuStageTimer::new(format!(
            "recursion_eval_check po2={} steps={} domain={}",
            po2,
            steps,
            steps * risc0_zkp::INV_RATE
        ));
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

struct WebGpuRecursionProver {
    hal: Rc<WebGpuHal>,
    circuit_hal: Rc<WebGpuCircuitHal>,
}

impl RecursionProver for WebGpuRecursionProver {
    fn prove(
        &self,
        program: crate::prove::Program,
        input: std::collections::VecDeque<u32>,
    ) -> Result<RecursionReceipt> {
        let delegate = RecursionProverImpl::new(self.hal.clone(), self.circuit_hal.clone());
        delegate.prove(program, input)
    }

    fn prove_async<'a>(
        &'a self,
        program: crate::prove::Program,
        input: std::collections::VecDeque<u32>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<RecursionReceipt>> + 'a>> {
        Box::pin(async move {
            risc0_core::scope!("prove");

            let mut preflight = Preflight::new(input);
            for (cycle, row) in program.code_by_row().enumerate() {
                preflight.step(cycle, row)?;
            }

            let witgen = crate::prove::witgen::WitnessGenerator::new(
                self.hal.as_ref(),
                self.circuit_hal.as_ref(),
                &program,
                &preflight,
            )?;

            let global = &witgen.global;
            let hashfn = &self.hal.get_hash_suite().hashfn;
            let mut prover = Prover::new(self.hal.as_ref(), TAPSET);

            prover
                .iop()
                .commit(&hashfn.hash_elem_slice(&PROOF_SYSTEM_INFO.encode()));
            prover
                .iop()
                .commit(&hashfn.hash_elem_slice(&CircuitImpl::CIRCUIT_INFO.encode()));

            let global_len = global.size();
            let mut header = vec![BabyBearElem::ZERO; global_len + 1];
            global.view_mut(|view| {
                for (i, elem) in view.iter_mut().enumerate() {
                    *elem = elem.valid_or_zero();
                    header[i] = *elem;
                }
                header[global_len] = BabyBearElem::new_raw(program.po2 as u32);
            });

            let header_digest = hashfn.hash_elem_slice(&header);
            prover.iop().commit(&header_digest);
            prover.iop().write_field_elem_slice(header.as_slice());
            prover.set_po2(program.po2);

            // SP-CR fix 2026-05-12: force `gpu_authoritative=false` for ALL
            // recursion prove ops (CTRL/DATA/ACCUM commit_groups + finalize).
            // Under `gpu_authoritative=true`, multi-segment xgboost's lift at
            // segments 5-8 silently produces all-zero Merkle roots for all
            // three commit groups, breaking `verify_integrity` with a control_id
            // mismatch. D11's `finish_hal_op` panic-on-fallback did NOT fire,
            // meaning the dispatches reported gpu_used=true while the GPU
            // buffer actually held zeros. The narrow surgical fix (likely a
            // missing onuncapturederror listener and/or a buffer "lost" via
            // silent OOM under Chrome WebGPU pressure) is deferred; D12 keeps
            // the prover correct at the cost of ~60× lift latency (xgboost
            // succinct: 3548s vs ~85s on the broken path, vs ~5.7s native
            // CUDA). See `.recursive/run/wasm-webgpu-prover-perf/01.5-root-cause.md`
            // for the full investigation; SP10-deferred fixtures unblocked by
            // this fix.
            {
                let _gpu_scope = self.hal.gpu_authoritative_scope(false);
                prover
                    .commit_group_async(REGISTER_GROUP_CTRL, &witgen.ctrl)
                    .await?;
                prover
                    .commit_group_async(REGISTER_GROUP_DATA, &witgen.data)
                    .await?;
            }

            let mix: [BabyBearElem; CircuitImpl::MIX_SIZE] =
                std::array::from_fn(|_| prover.iop().random_elem());
            let mix = witgen.accum(self.hal.as_ref(), self.circuit_hal.as_ref(), &mix)?;

            let seal = {
                let _gpu_scope = self.hal.gpu_authoritative_scope(false);
                prover
                    .commit_group_async(REGISTER_GROUP_ACCUM, &witgen.accum)
                    .await?;
                prover
                    .finalize_async(&[&mix, global], self.circuit_hal.as_ref())
                    .await?
            };

            Ok(RecursionReceipt {
                seal,
                output: preflight.output,
            })
        })
    }
}

pub fn recursion_prover(hal: Rc<WebGpuHal>) -> Result<Box<dyn RecursionProver>> {
    Ok(Box::new(WebGpuRecursionProver {
        hal,
        circuit_hal: Rc::new(WebGpuCircuitHal),
    }))
}

fn step_mode_label(mode: StepMode) -> &'static str {
    match mode {
        StepMode::Parallel => "parallel",
        StepMode::SeqForward => "seq_forward",
        StepMode::SeqReverse => "seq_reverse",
    }
}
