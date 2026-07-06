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

//! The synchronous circuit-HAL trait implementations
//! (`CircuitWitnessGenerator`, `CircuitAccumulator`, `CircuitHal`)
//! and the `SegmentProver` wrapper.

use super::*;
#[allow(unused_imports)]
use super::{accum_wgsl::*, kernels::*, phases::*, session::*, witgen_wgsl::*};

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
        // GPU dispatches now happen
        // in `pre_witgen_dispatch_async` (called from the async prove path
        // BEFORE this sync `generate_witness`). The mask and CPU shadow
        // are already in sync by the time we get here, so rust_steps
        // reads correct values for short-circuited cycles.
        //
        // For the sync prove path (no pre_dispatch hook), mask was reset
        // to 0 by pre_witgen_dispatch_async's no-op branch, OR if that
        // wasn't called, the legacy state stands (still 0 by default).
        crate::prove::hal::rust_steps::with_eqz_elided(|| {
            crate::prove::hal::rust_steps::generate_witness(
                mode,
                self.witgen_replace_arm_mask.get(),
                preflight,
                global,
                data,
            )
        })
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
        log_accum_major_histogram(preflight);
        let scopes = crate::prove::webgpu_async_authoritative_scopes();
        if scopes.accum_make_coeffs && scopes.accum_poly_group && scopes.accum_merkle {
            let mask = self.witgen_replace_arm_mask.get();
            let misc0_direct_enabled = ACCUM_GPU_MISC0_DIRECT_ENABLED.load(Ordering::SeqCst);
            let mem0_direct_enabled = ACCUM_GPU_MEM0_DIRECT_ENABLED.load(Ordering::SeqCst);
            let direct_row_mask = mask
                | if misc0_direct_enabled { 0x0001 } else { 0 }
                | if mem0_direct_enabled { 1u16 << 5 } else { 0 };
            let (misc0_rows_by_minor, _, _, mem0_rows_by_minor) =
                Self::witgen_replace_accum_shadow_rows_by_minor(preflight, direct_row_mask);
            let misc0_rows = if misc0_direct_enabled {
                [0usize, 1, 2, 3, 4, 7]
                    .into_iter()
                    .flat_map(|minor| misc0_rows_by_minor[minor].iter().copied())
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let misc1_rows = if ACCUM_GPU_MISC1_DIRECT_ENABLED.load(Ordering::SeqCst) {
                preflight
                    .cycles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, cycle)| {
                        (cycle.major == 1).then(|| {
                            u32::try_from(idx)
                                .context("MISC1 direct accumulator cycle index exceeds u32")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            let misc2_rows = if ACCUM_GPU_MISC2_DIRECT_ENABLED.load(Ordering::SeqCst) {
                preflight
                    .cycles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, cycle)| {
                        (cycle.major == 2).then(|| {
                            u32::try_from(idx)
                                .context("MISC2 direct accumulator cycle index exceeds u32")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            let mem0_rows = if mem0_direct_enabled {
                mem0_rows_by_minor
            } else {
                Vec::new()
            };
            let mem1_rows = if ACCUM_GPU_MEM1_DIRECT_ENABLED.load(Ordering::SeqCst) {
                preflight
                    .cycles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, cycle)| {
                        (cycle.major == 6).then(|| {
                            u32::try_from(idx)
                                .context("MEM1 direct accumulator cycle index exceeds u32")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            let control0_rows = if ACCUM_GPU_CONTROL0_DIRECT_ENABLED.load(Ordering::SeqCst) {
                preflight
                    .cycles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, cycle)| {
                        (cycle.major == 7).then(|| {
                            u32::try_from(idx)
                                .context("CONTROL0 direct accumulator cycle index exceeds u32")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            let poseidon1_rows = if ACCUM_GPU_POSEIDON1_DIRECT_ENABLED.load(Ordering::SeqCst) {
                preflight
                    .cycles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, cycle)| {
                        (cycle.major == 10).then(|| {
                            u32::try_from(idx)
                                .context("POSEIDON1 direct accumulator cycle index exceeds u32")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            if !misc0_rows.is_empty()
                || !misc1_rows.is_empty()
                || !misc2_rows.is_empty()
                || !mem0_rows.is_empty()
                || !mem1_rows.is_empty()
                || !control0_rows.is_empty()
                || !poseidon1_rows.is_empty()
            {
                let direct_major_mask = if misc1_rows.is_empty() { 0 } else { 1u16 << 1 }
                    | if misc2_rows.is_empty() { 0 } else { 1u16 << 2 }
                    | if mem1_rows.is_empty() { 0 } else { 1u16 << 6 }
                    | if control0_rows.is_empty() {
                        0
                    } else {
                        1u16 << 7
                    }
                    | if poseidon1_rows.is_empty() {
                        0
                    } else {
                        1u16 << 10
                    };
                crate::prove::hal::rust_steps::with_accum_eqz_elided(|| {
                    crate::prove::hal::rust_steps::step_accum_without_selected_majors_or_postprocess(
                        preflight,
                        data,
                        accum,
                        global,
                        mix,
                        !misc0_rows.is_empty(),
                        !mem0_rows.is_empty(),
                        direct_major_mask,
                    )
                })?;
                anyhow::ensure!(
                    self.dispatch_accum_misc_direct_grouped(
                        &misc0_rows,
                        &misc1_rows,
                        &misc2_rows,
                        &mem0_rows,
                        &mem1_rows,
                        &control0_rows,
                        &poseidon1_rows,
                        data,
                        accum,
                        mix
                    )?,
                    "RV32IM grouped MISC direct accumulator GPU dispatch unavailable"
                );
                {
                    let _timer = WebGpuStageTimer::new_active_for(
                        "rv32im_accumulate terminal_ext_prefix_gpu",
                        self.hal.as_ref(),
                    );
                    anyhow::ensure!(
                        dispatch_accum_terminal_ext_prefix(
                            self.hal.as_ref(),
                            &accum.buf,
                            accum.rows,
                            accum.cols,
                        )?,
                        "RV32IM accum terminal prefix GPU dispatch unavailable in MISC0 direct mode"
                    );
                }
                let split = LAYOUT_TOP_ACCUM.columns[0].offset;
                {
                    let _timer = WebGpuStageTimer::new_active_for(
                        "rv32im_accumulate machine_column_carry_gpu",
                        self.hal.as_ref(),
                    );
                    anyhow::ensure!(
                        dispatch_accum_machine_column_carry(
                            self.hal.as_ref(),
                            &accum.buf,
                            accum.rows,
                            accum.cols,
                            split,
                        )?,
                        "RV32IM accum machine-column carry GPU dispatch unavailable in MISC0 direct mode"
                    );
                }
                return Ok(());
            }
            if ACCUM_GPU_ARM5_AUTHORITATIVE_ENABLED.load(Ordering::SeqCst) {
                let arm5_cycles = preflight
                    .cycles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, cycle)| {
                        (cycle.major == 5).then(|| {
                            u32::try_from(idx)
                                .context("TopAccum arm5 authoritative cycle index exceeds u32")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                if !arm5_cycles.is_empty() {
                    crate::prove::hal::rust_steps::with_accum_eqz_elided(|| {
                        crate::prove::hal::rust_steps::step_accum_without_major_or_postprocess(
                            preflight, data, accum, global, mix, 5,
                        )
                    })?;
                    self.dispatch_topaccum_arm5_authoritative(
                        &arm5_cycles,
                        data,
                        accum,
                        global,
                        mix,
                    )?;
                    {
                        let _timer = WebGpuStageTimer::new_active_for(
                            "rv32im_accumulate terminal_ext_prefix_gpu",
                            self.hal.as_ref(),
                        );
                        anyhow::ensure!(
                            dispatch_accum_terminal_ext_prefix(
                                self.hal.as_ref(),
                                &accum.buf,
                                accum.rows,
                                accum.cols,
                            )?,
                            "RV32IM accum terminal prefix GPU dispatch unavailable in authoritative arm5 mode"
                        );
                    }
                    let split = LAYOUT_TOP_ACCUM.columns[0].offset;
                    {
                        let _timer = WebGpuStageTimer::new_active_for(
                            "rv32im_accumulate machine_column_carry_gpu",
                            self.hal.as_ref(),
                        );
                        anyhow::ensure!(
                            dispatch_accum_machine_column_carry(
                                self.hal.as_ref(),
                                &accum.buf,
                                accum.rows,
                                accum.cols,
                                split,
                            )?,
                            "RV32IM accum machine-column carry GPU dispatch unavailable in authoritative arm5 mode"
                        );
                    }
                    return Ok(());
                }
            }
            crate::prove::hal::rust_steps::step_accum_without_machine_column_carry(
                self.witgen_replace_arm_mask.get(),
                preflight,
                data,
                accum,
                global,
                mix,
            )?;
            self.dispatch_topaccum_arm5_real_buffer_probe(preflight, data, accum, global, mix)?;
            let split = LAYOUT_TOP_ACCUM.columns[0].offset;
            {
                let _timer = WebGpuStageTimer::new_active_for(
                    "rv32im_accumulate machine_column_carry_gpu",
                    self.hal.as_ref(),
                );
                if dispatch_accum_machine_column_carry(
                    self.hal.as_ref(),
                    &accum.buf,
                    accum.rows,
                    accum.cols,
                    split,
                )? {
                    return Ok(());
                }
            }
            crate::prove::hal::rust_steps::finish_accum_machine_column_carry(accum);
            return Ok(());
        }
        crate::prove::hal::rust_steps::step_accum(
            self.witgen_replace_arm_mask.get(),
            preflight,
            data,
            accum,
            global,
            mix,
        )
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

pub(crate) struct WebGpuSegmentProver {
    pub(crate) hal: Rc<WebGpuHal>,
    pub(crate) circuit_hal: Rc<WebGpuCircuitHal>,
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
            let job = witgen_phase_inner(
                self.hal.clone(),
                self.circuit_hal.clone(),
                preflight_results,
            )
            .await?;
            webgpu_segment_commit_phase(job).await
        })
    }
}
