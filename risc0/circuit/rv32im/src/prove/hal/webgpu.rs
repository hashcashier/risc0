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
use risc0_zkp::{
    adapter::{CircuitInfo as _, PROOF_SYSTEM_INFO},
    field::Elem as _,
    hal::{
        webgpu::{WebGpuBuffer, WebGpuCircuitEvalCheck, WebGpuHal, WebGpuStageTimer},
        AccumPreflight, Buffer, CircuitHal, Hal,
    },
    prove::Prover,
};

use super::{
    CircuitAccumulator, CircuitWitnessGenerator, MetaBuffer, PreflightResults, SegmentProver,
    SegmentProverImpl, StepMode,
};
use crate::{
    prove::witgen::preflight::PreflightTrace,
    zirgen::{
        circuit::{
            ExtVal, Val, REGCOUNT_MIX, REGISTER_GROUP_ACCUM, REGISTER_GROUP_CODE,
            REGISTER_GROUP_DATA,
        },
        taps::TAPSET,
        CircuitImpl,
    },
    RV32IM_SEAL_VERSION,
};

#[derive(Default)]
#[allow(dead_code)]
pub(crate) struct WebGpuCircuitHal;

impl WebGpuCircuitEvalCheck for WebGpuCircuitHal {
    fn eval_check_webgpu(
        &self,
        hal: &WebGpuHal,
        check: &WebGpuBuffer<Val>,
        groups: &[&WebGpuBuffer<Val>],
        globals: &[&WebGpuBuffer<Val>],
        poly_mix: ExtVal,
        po2: usize,
        steps: usize,
    ) -> Result<bool> {
        hal.dispatch_eval_check_poly_ext(
            check,
            groups,
            globals,
            TAPSET,
            &crate::zirgen::poly_ext::DEF,
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
        preflight: &PreflightTrace,
        global: &MetaBuffer<WebGpuHal>,
        data: &MetaBuffer<WebGpuHal>,
    ) -> Result<()> {
        let _timer = WebGpuStageTimer::new(format!(
            "rv32im_witgen mode={} cycles={} txns={} bigint_bytes={} data_rows={} data_cols={}",
            step_mode_label(mode),
            preflight.cycles.len(),
            preflight.txns.len(),
            preflight.bigint_bytes.len(),
            data.rows,
            data.cols
        ));
        super::rust_steps::generate_witness(mode, preflight, global, data)
    }
}

impl CircuitAccumulator<WebGpuHal> for WebGpuCircuitHal {
    fn step_accum(
        &self,
        preflight: &PreflightTrace,
        data: &MetaBuffer<WebGpuHal>,
        accum: &MetaBuffer<WebGpuHal>,
        global: &MetaBuffer<WebGpuHal>,
        mix: &MetaBuffer<WebGpuHal>,
    ) -> Result<()> {
        let _timer = WebGpuStageTimer::new(format!(
            "rv32im_accumulate cycles={} data_rows={} accum_rows={}",
            preflight.cycles.len(),
            data.rows,
            accum.rows
        ));
        super::rust_steps::step_accum(preflight, data, accum, global, mix)
    }
}

impl CircuitHal<WebGpuHal> for WebGpuCircuitHal {
    fn eval_check(
        &self,
        check: &WebGpuBuffer<Val>,
        groups: &[&WebGpuBuffer<Val>],
        globals: &[&WebGpuBuffer<Val>],
        poly_mix: ExtVal,
        po2: usize,
        steps: usize,
    ) {
        let _timer = WebGpuStageTimer::new(format!(
            "rv32im_eval_check po2={} steps={} domain={}",
            po2,
            steps,
            steps * risc0_zkp::INV_RATE
        ));
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

    fn accumulate(
        &self,
        _preflight: &AccumPreflight,
        _ctrl: &WebGpuBuffer<Val>,
        _global: &WebGpuBuffer<Val>,
        _data: &WebGpuBuffer<Val>,
        _mix: &WebGpuBuffer<Val>,
        _accum: &WebGpuBuffer<Val>,
        _steps: usize,
    ) {
        unimplemented!("browser WebGPU rv32im accumulation kernel is not wired yet")
    }
}

struct WebGpuSegmentProver {
    hal: Rc<WebGpuHal>,
    circuit_hal: Rc<WebGpuCircuitHal>,
}

impl SegmentProver for WebGpuSegmentProver {
    fn preflight(&self, segment: &crate::execute::segment::Segment) -> Result<PreflightResults> {
        scope!("preflight");

        cfg_if::cfg_if! {
            if #[cfg(feature = "witgen_debug")] {
                let rand_z = ExtVal::ONE;
            } else {
                let mut rng = rand::rng();
                let rand_z = ExtVal::random(&mut rng);
            }
        }
        PreflightResults::new(segment, rand_z)
    }

    fn prove_core(&self, preflight_results: PreflightResults) -> Result<crate::prove::Seal> {
        let hal = self.hal.clone();
        let circuit_hal = self.circuit_hal.clone();
        let delegate = SegmentProverImpl::new(move || (hal.clone(), circuit_hal.clone()));
        delegate.prove_core(preflight_results)
    }

    fn prove_core_async<'a>(
        &'a self,
        preflight_results: PreflightResults,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<crate::prove::Seal>> + 'a>> {
        Box::pin(async move {
            scope!("prove_core");

            cfg_if::cfg_if! {
                if #[cfg(feature = "witgen_debug")] {
                    let mode = if std::env::var_os("RISC0_WITGEN_DEBUG").is_some() {
                        StepMode::SeqForward
                    } else {
                        StepMode::Parallel
                    };
                } else {
                    let mode = StepMode::Parallel;
                }
            }

            let hal = self.hal.as_ref();
            let circuit_hal = self.circuit_hal.as_ref();

            let po2 = preflight_results.po2();
            let witgen = super::super::witgen::WitnessGenerator::new(
                hal,
                circuit_hal,
                preflight_results,
                mode,
            )?;

            let code = &witgen.code.buf;
            let data = &witgen.data.buf;
            let global = &witgen.global.buf;

            tracing::debug!("prove_inner");

            let mut prover = Prover::new(hal, TAPSET);
            let hashfn = &hal.get_hash_suite().hashfn;

            prover.iop().write_u32_slice(&[RV32IM_SEAL_VERSION]);
            prover
                .iop()
                .commit(&hashfn.hash_elem_slice(&PROOF_SYSTEM_INFO.encode()));
            prover
                .iop()
                .commit(&hashfn.hash_elem_slice(&CircuitImpl::CIRCUIT_INFO.encode()));

            let global_len = global.size();
            let mut header = vec![Val::ZERO; global_len + 1];
            global.view_mut(|view| {
                for (i, elem) in view.iter_mut().enumerate() {
                    *elem = elem.valid_or_zero();
                    header[i] = *elem;
                }
                header[global_len] = Val::new_raw(po2);
            });

            let header_digest = hashfn.hash_elem_slice(&header);
            prover.iop().commit(&header_digest);
            prover.iop().write_field_elem_slice(header.as_slice());
            prover.set_po2(po2 as usize);

            let async_scopes = crate::prove::webgpu_async_authoritative_scopes();
            {
                let _gpu_scope = hal.gpu_authoritative_scope(async_scopes.code_data);
                {
                    let _t = WebGpuStageTimer::new_active("commit_group_async rv32im_code");
                    prover.commit_group_async(REGISTER_GROUP_CODE, code).await?;
                }
                {
                    let _t = WebGpuStageTimer::new_active("commit_group_async rv32im_data");
                    prover.commit_group_async(REGISTER_GROUP_DATA, data).await?;
                }
            }

            let mix: [Val; REGCOUNT_MIX] = std::array::from_fn(|_| prover.iop().random_elem());
            let mix = {
                let _t = WebGpuStageTimer::new("rv32im_witgen_accum");
                witgen.accum(hal, circuit_hal, &mix)?
            };

            let async_scopes = crate::prove::webgpu_async_authoritative_scopes();
            {
                let _t = WebGpuStageTimer::new_active("commit_group_async rv32im_accum");
                prover
                    .commit_group_async_scoped(
                        REGISTER_GROUP_ACCUM,
                        &witgen.accum.buf,
                        async_scopes.accum_make_coeffs,
                        async_scopes.accum_poly_group,
                        async_scopes.accum_merkle,
                    )
                    .await?;
            }
            {
                let _gpu_scope = hal.gpu_authoritative_scope(async_scopes.finalize);
                prover
                    .finalize_async(&[&mix.buf, global], circuit_hal)
                    .await
            }
        })
    }
}

pub fn segment_prover(hal: Rc<WebGpuHal>) -> Result<Box<dyn SegmentProver>> {
    Ok(Box::new(WebGpuSegmentProver {
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
