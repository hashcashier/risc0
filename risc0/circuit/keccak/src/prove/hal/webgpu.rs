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

use std::{future::Future, pin::Pin, rc::Rc};

use anyhow::Result;
use risc0_core::{field::Elem as _, scope};
use risc0_zkp::{
    adapter::{CircuitInfo as _, PROOF_SYSTEM_INFO},
    core::digest::Digest,
    field::baby_bear::BabyBearExtElem,
    hal::{
        webgpu::{WebGpuBuffer, WebGpuCircuitEvalCheck, WebGpuHal, WebGpuStageTimer},
        AccumPreflight, Buffer, CircuitHal, Hal,
    },
    prove::Prover,
};

use crate::{
    prove::{KeccakProver, KeccakProverImpl, Seal},
    zirgen::{
        circuit::{
            CircuitField, ExtVal, Val, LAYOUT_GLOBAL, REGCOUNT_ACCUM, REGCOUNT_CODE, REGCOUNT_DATA,
            REGCOUNT_GLOBAL, REGCOUNT_MIX, REGISTER_GROUP_ACCUM, REGISTER_GROUP_CODE,
            REGISTER_GROUP_DATA,
        },
        taps::TAPSET,
        CircuitImpl,
    },
    KeccakState,
};

use super::{
    CircuitWitnessGenerator, ForwardPreflightOrder, MetaBuffer, PreflightCycleOrder,
    PreflightTrace, StepMode,
};

#[derive(Default)]
#[allow(dead_code)]
pub(crate) struct WebGpuCircuitHal;

struct WebGpuKeccakProver {
    hal: Rc<WebGpuHal>,
    circuit_hal: Rc<WebGpuCircuitHal>,
}

impl WebGpuKeccakProver {
    /// The keccak proof's phase-head CPU — preflight construction,
    /// scatter, and the witness pass — on a pool worker instead of this
    /// wasm thread. These blocks (~165 ms/proof) run while other proofs'
    /// readback chains are in flight (the keccak/union phase interleaves
    /// keccak proofs, union recursion proofs, and rv32im segment commits
    /// all day), and every millisecond spent inline on this thread starves
    /// those mapAsync callbacks. All three pieces are pre-transcript
    /// (phase-head), so the offload is clean — no
    /// mid-transcript foreground wait rides on the pool's injector queue.
    /// The preflight is built AND consumed on the worker; buffers travel
    /// as `Send` CPU-shadow handles (fresh buffers, shadows current by
    /// construction).
    async fn witgen_offloaded_async(
        &self,
        inputs: &[KeccakState],
        cycles: usize,
        global: &MetaBuffer<WebGpuHal>,
        data: &MetaBuffer<WebGpuHal>,
    ) -> Result<()> {
        let _timer = WebGpuStageTimer::new(format!(
            "keccak_witgen_phase inputs={} cycles={} offload=pool",
            inputs.len(),
            cycles
        ));
        let inputs = inputs.to_vec();
        let data_shadow = data.buf.begin_cpu_shadow_offload_mut();
        let (data_rows, data_cols, data_checked) = (data.rows, data.cols, data.checked_reads);
        let global_shadow = global.buf.begin_cpu_shadow_offload_mut();
        let (global_rows, global_cols, global_checked) =
            (global.rows, global.cols, global.checked_reads);
        let (tx, rx) = futures::channel::oneshot::channel();
        rayon::spawn(move || {
            type ShadowHal = risc0_zkp::hal::cpu::CpuHal<CircuitField>;
            let run = || -> Result<()> {
                let preflight = PreflightTrace::<ForwardPreflightOrder>::new(&inputs, cycles);
                let data_mb = MetaBuffer::<ShadowHal> {
                    buf: data_shadow,
                    rows: data_rows,
                    cols: data_cols,
                    checked_reads: data_checked,
                };
                let global_mb = MetaBuffer::<ShadowHal> {
                    buf: global_shadow,
                    rows: global_rows,
                    cols: global_cols,
                    checked_reads: global_checked,
                };
                scatter_preflight_into(&data_mb, &preflight.scatter, &preflight.data)?;
                super::rust_steps::generate_witness(
                    StepMode::Parallel,
                    &preflight,
                    &global_mb,
                    &data_mb,
                )
            };
            let _ = tx.send(run());
        });
        rx.await
            .map_err(|_| anyhow::anyhow!("offloaded keccak witgen worker dropped its result"))??;
        data.buf.finish_cpu_shadow_offload_mut();
        global.buf.finish_cpu_shadow_offload_mut();
        Ok(())
    }
}

impl WebGpuCircuitEvalCheck for WebGpuCircuitHal {
    fn eval_check_webgpu(
        &self,
        hal: &WebGpuHal,
        check: &WebGpuBuffer<Val>,
        groups: &[&WebGpuBuffer<Val>],
        globals: &[&WebGpuBuffer<Val>],
        poly_mix: BabyBearExtElem,
        po2: usize,
        steps: usize,
    ) -> Result<bool> {
        hal.dispatch_eval_check_poly_ext(
            check,
            groups,
            globals,
            crate::zirgen::taps::TAPSET,
            &crate::zirgen::poly_ext::DEF,
            poly_mix,
            po2,
            steps,
        )
    }
}

/// The scatter body, generic over the buffer HAL so the
/// pool-offloaded witgen phase can run it against bare `CpuBuffer`
/// shadow handles (`MetaBuffer<CpuHal>`), exactly like the generic
/// `rust_steps::generate_witness`.
fn scatter_preflight_into<H>(
    into: &MetaBuffer<H>,
    infos: &[risc0_circuit_keccak_sys::ScatterInfo],
    data: &[u32],
) -> Result<()>
where
    H: risc0_zkp::hal::Hal<Field = CircuitField, Elem = Val, ExtElem = ExtVal>,
{
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

impl CircuitWitnessGenerator<WebGpuHal> for WebGpuCircuitHal {
    type PreferredPreflightOrder = ForwardPreflightOrder;

    fn scatter_preflight(
        &self,
        into: &MetaBuffer<WebGpuHal>,
        infos: &[risc0_circuit_keccak_sys::ScatterInfo],
        data: &[u32],
    ) -> Result<()> {
        scope!("scatter");
        let _timer = WebGpuStageTimer::new(format!(
            "keccak_scatter_preflight rows={} cols={} scatter={} data_words={}",
            into.rows,
            into.cols,
            infos.len(),
            data.len()
        ));
        scatter_preflight_into(into, infos, data)
    }

    fn generate_witness<O: PreflightCycleOrder>(
        &self,
        mode: StepMode,
        preflight: &PreflightTrace<O>,
        global: &MetaBuffer<WebGpuHal>,
        data: &MetaBuffer<WebGpuHal>,
    ) -> Result<()> {
        let _timer = WebGpuStageTimer::new(format!(
            "keccak_witgen mode={} cycle={} preimages={} data_words={} cycles={}",
            step_mode_label(mode),
            preflight.cycle,
            preflight.preimages.len(),
            preflight.data.len(),
            preflight.cycles.len()
        ));
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
        let _timer = WebGpuStageTimer::new(format!(
            "keccak_eval_check po2={} steps={} domain={}",
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
}

impl KeccakProver for WebGpuKeccakProver {
    fn prove(&self, inputs: &[KeccakState], po2: usize) -> Result<Seal> {
        KeccakProverImpl {
            hal: self.hal.clone(),
            circuit_hal: self.circuit_hal.clone(),
        }
        .prove(inputs, po2)
    }

    fn prove_async<'a>(
        &'a self,
        inputs: &'a [KeccakState],
        po2: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Seal>> + 'a>> {
        Box::pin(async move {
            scope!("prove");

            let cycles: usize = 1 << po2;

            let mut global = vec![Val::INVALID; REGCOUNT_GLOBAL];
            global[LAYOUT_GLOBAL.total_cycles._super.offset] = Val::from_u64(1 << po2);

            let global = MetaBuffer {
                buf: self.hal.copy_from_elem("global", &global),
                rows: 1,
                cols: REGCOUNT_GLOBAL,
                checked_reads: true,
            };
            let code =
                MetaBuffer::new_zeroed("code", self.hal.as_ref(), cycles, REGCOUNT_CODE, true);
            let data = scope!(
                "alloc(keccak_data)",
                MetaBuffer::new(
                    "keccak_data",
                    self.hal.as_ref(),
                    cycles,
                    REGCOUNT_DATA,
                    true
                )
            );

            // Preflight + scatter + witness pass run on a pool worker
            // so this thread keeps observing the other in-flight proofs'
            // readback completions.
            self.witgen_offloaded_async(inputs, cycles, &global, &data)
                .await?;

            scope!("zeroize", {
                self.hal.eltwise_zeroize_elem(&data.buf);
            });

            let mut prover = Prover::new(self.hal.as_ref(), TAPSET);
            let hashfn = &self.hal.get_hash_suite().hashfn;

            prover
                .iop()
                .commit(&hashfn.hash_elem_slice(&PROOF_SYSTEM_INFO.encode()));
            prover
                .iop()
                .commit(&hashfn.hash_elem_slice(&CircuitImpl::CIRCUIT_INFO.encode()));

            global.buf.view(|slice| {
                let mut digest = Digest::ZERO;
                for (i, word) in digest.as_mut_words().iter_mut().enumerate() {
                    let low: u32 = slice[i * 2].into();
                    let high: u32 = slice[i * 2 + 1].into();
                    *word = low | (high << 16);
                }
                tracing::debug!("final digest: {digest}");

                let header: Vec<_> = slice
                    .iter()
                    .chain(Val::from_u32_slice(&[po2 as u32]))
                    .copied()
                    .collect();

                let header_digest = hashfn.hash_elem_slice(&header);
                prover.iop().commit(&header_digest);
                prover.iop().write_field_elem_slice(&header);
            });

            prover.set_po2(po2);

            {
                let _gpu_scope = self.hal.gpu_authoritative_scope(true);
                {
                    let _t = WebGpuStageTimer::new_active_for(
                        "commit_group_async keccak_code",
                        self.hal.as_ref(),
                    );
                    prover
                        .commit_group_async_in_place(REGISTER_GROUP_CODE, code.buf.clone())
                        .await?;
                }
                {
                    let _t = WebGpuStageTimer::new_active_for(
                        "commit_group_async keccak_data",
                        self.hal.as_ref(),
                    );
                    prover
                        .commit_group_async_in_place(REGISTER_GROUP_DATA, data.buf.clone())
                        .await?;
                }
            }

            let mix: [Val; REGCOUNT_MIX] = std::array::from_fn(|_| prover.iop().random_elem());
            let mix = self.hal.copy_from_elem("mix", mix.as_slice());

            let accum = self
                .hal
                .alloc_elem_init("accum", cycles * REGCOUNT_ACCUM, Val::ZERO);
            let seal = {
                let _gpu_scope = self.hal.gpu_authoritative_scope(true);
                {
                    let _t = WebGpuStageTimer::new_active_for(
                        "commit_group_async keccak_accum",
                        self.hal.as_ref(),
                    );
                    prover
                        .commit_group_async_in_place(REGISTER_GROUP_ACCUM, accum.clone())
                        .await?;
                }
                prover
                    .finalize_async(&[&mix, &global.buf], self.circuit_hal.as_ref())
                    .await?
            };

            Ok(seal)
        })
    }
}

pub fn keccak_prover(hal: Rc<WebGpuHal>) -> Result<Box<dyn KeccakProver>> {
    Ok(Box::new(WebGpuKeccakProver {
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
