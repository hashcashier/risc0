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

//! The async segment proving phases: witgen-phase and commit-phase
//! entry points, the segment job handoff between them, and the
//! browser prover constructors.

use super::*;
#[allow(unused_imports)]
use super::{accum_wgsl::*, kernels::*, phases::*, traits::*, witgen_wgsl::*};

/// Everything a segment's witgen phase produces, ready for the
/// transcript-bound commit phase. Opaque to callers; the zkvm prover
/// pipelines segment N+1's witgen phase (CPU-heavy) under segment N's
/// commit phase (GPU-heavy) — the phases only share the device queue.
/// Per-prove witgen state (the GPU-replacement arm mask) lives on the
/// job's private `WebGpuCircuitHal`, so in-flight jobs don't collide.
pub struct WebGpuSegmentJob {
    hal: Rc<WebGpuHal>,
    circuit_hal: Rc<WebGpuCircuitHal>,
    witgen: crate::prove::witgen::WitnessGenerator<WebGpuHal>,
    po2: u32,
}

/// Witgen phase of a segment prove — buffer setup, pre-witgen
/// GPU dispatches, and CPU witness generation (rayon-parallel). Touches
/// no Fiat-Shamir transcript state, so it may run while another
/// segment's commit phase is in flight on the same device.
pub async fn webgpu_segment_witgen_phase(
    hal: Rc<WebGpuHal>,
    preflight_results: PreflightResults,
) -> Result<WebGpuSegmentJob> {
    let circuit_hal = Rc::new(WebGpuCircuitHal::new(hal.clone()));
    // No-op when the witgen GPU probe is off; otherwise ensures the
    // per-arm kernels are compiled (they almost always already are).
    circuit_hal.prewarm_witgen_kernel();
    witgen_phase_inner(hal, circuit_hal, preflight_results).await
}

pub(crate) async fn witgen_phase_inner(
    hal_rc: Rc<WebGpuHal>,
    circuit_hal_rc: Rc<WebGpuCircuitHal>,
    preflight_results: PreflightResults,
) -> Result<WebGpuSegmentJob> {
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

    let hal = hal_rc.as_ref();
    let circuit_hal = circuit_hal_rc.as_ref();

    let po2 = preflight_results.po2();
    // Split WitnessGenerator
    // construction so we can insert an ASYNC pre-dispatch hook
    // (GPU shadow_init + per-arm chunks + sync_gpu_to_cpu) BEFORE
    // sync `generate_witness` runs. This is the fix for the
    // CPU/GPU shadow desync blocker that step 6.2.7 identified.
    let setup_timer = WebGpuStageTimer::new("rv32im_prove_setup");
    let (global_vec, injector, cycles, trace, _po2_inner) =
        crate::prove::witgen::WitnessGenerator::<WebGpuHal>::preflight_components(
            preflight_results,
        );
    let (global_buf, code_buf, data_buf) =
        crate::prove::witgen::WitnessGenerator::<WebGpuHal>::allocate_buffers(
            hal,
            &global_vec,
            cycles,
            &injector,
        );
    drop(setup_timer);
    {
        let _t = WebGpuStageTimer::new("rv32im_pre_witgen_dispatch");
        circuit_hal
            .pre_witgen_dispatch_async(&trace, &data_buf, &global_buf)
            .await?;
    }
    // If diff mode is on,
    // snapshot GPU result from CPU shadow, reset shadow to
    // INVALID, re-scatter injector, force mask=0 so rust_steps
    // can write cleanly, then diff BEFORE zeroize.
    //
    // Snapshot AFTER
    // generate_witness but BEFORE eltwise_zeroize_elem so we can
    // distinguish "INVALID (not written)" from "0 (actually
    // written zero)". The 6.2.13 filter conflated both, missing
    // mismatches where GPU wrote V != 0 but rust_steps wrote 0.
    let diff_mode = WITGEN_GPU_DIFF_ENABLED.load(Ordering::SeqCst);
    let replace_diff_mode = diff_mode && WITGEN_GPU_REPLACE_ENABLED.load(Ordering::SeqCst);
    let witgen = if replace_diff_mode {
        let replace_diff_segment =
            WITGEN_GPU_REPLACE_DIFF_SEEN_SEGMENTS.fetch_add(1, Ordering::SeqCst);
        let replace_diff_target = WITGEN_GPU_REPLACE_DIFF_TARGET_SEGMENT.load(Ordering::SeqCst);
        circuit_hal
            .generate_witness(mode, &trace, &global_buf, &data_buf)
            .context("witness generation failure (REPLACE DIFF mode)")?;
        if replace_diff_segment != replace_diff_target {
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "REPLACE_FINAL_SKIP segment_ord={} target={}",
                replace_diff_segment, replace_diff_target
            ));
            finish_witgen_from_populated_parts(hal, trace, cycles, global_buf, code_buf, data_buf)
        } else {
            hal.eltwise_zeroize_elem(&global_buf.buf);
            hal.eltwise_zeroize_elem(&data_buf.buf);
            let mut replace_snap = vec![0u32; data_buf.buf.size()];
            data_buf.buf.view(|slice: &[Val]| {
                let u32s: &[u32] = unsafe {
                    std::slice::from_raw_parts(slice.as_ptr() as *const u32, slice.len())
                };
                replace_snap.copy_from_slice(u32s);
            });

            data_buf.buf.view_mut(|slice: &mut [Val]| {
                for v in slice.iter_mut() {
                    *v = Val::INVALID;
                }
            });
            global_buf.buf.view_mut(|slice: &mut [Val]| {
                slice.copy_from_slice(&global_vec);
            });
            hal.scatter(
                &data_buf.buf,
                &injector.index,
                &injector.offsets,
                &injector.values,
            );
            circuit_hal.set_witgen_replace_arm_mask(0);
            circuit_hal
                .generate_witness(mode, &trace, &global_buf, &data_buf)
                .context("witness generation failure (REPLACE DIFF CPU mode)")?;
            hal.eltwise_zeroize_elem(&global_buf.buf);
            hal.eltwise_zeroize_elem(&data_buf.buf);
            let mut cpu_snap = vec![0u32; data_buf.buf.size()];
            data_buf.buf.view(|slice: &[Val]| {
                let u32s: &[u32] = unsafe {
                    std::slice::from_raw_parts(slice.as_ptr() as *const u32, slice.len())
                };
                cpu_snap.copy_from_slice(u32s);
            });
            let rows = data_buf.rows;
            let cols = data_buf.cols;
            let mut mismatches = 0usize;
            for i in 0..replace_snap.len() {
                let g = replace_snap[i];
                let c = cpu_snap[i];
                if g != c {
                    if mismatches < 20 {
                        let row = i % rows;
                        let col = i / rows;
                        let (major, minor) = trace
                            .cycles
                            .get(row)
                            .map(|cycle| (cycle.major, cycle.minor))
                            .unwrap_or((u8::MAX, u8::MAX));
                        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                            "REPLACE_FINAL_MISMATCH idx={} row={} col={} major={} minor={} replace=0x{:08x} cpu=0x{:08x}",
                            i, row, col, major, minor, g, c,
                        ));
                    }
                    mismatches += 1;
                }
            }
            risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                "REPLACE_FINAL_SUMMARY segment_ord={} total_cells={} mismatches={} rows={} cols={}",
                replace_diff_segment,
                replace_snap.len(),
                mismatches,
                rows,
                cols,
            ));
            anyhow::bail!(
                "REPLACE DIFF mode: {} final data mismatches; see REPLACE_FINAL_MISMATCH + REPLACE_FINAL_SUMMARY",
                mismatches
            );
        }
    } else if diff_mode {
        let mut snap = vec![0u32; data_buf.buf.size()];
        data_buf.buf.view(|slice: &[Val]| {
            let u32s: &[u32] =
                unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u32, slice.len()) };
            snap.copy_from_slice(u32s);
        });
        data_buf.buf.view_mut(|slice: &mut [Val]| {
            for v in slice.iter_mut() {
                *v = Val::INVALID;
            }
        });
        hal.scatter(
            &data_buf.buf,
            &injector.index,
            &injector.offsets,
            &injector.values,
        );
        circuit_hal.set_witgen_replace_arm_mask(0);
        // Run generate_witness only (no zeroize) so the CPU shadow
        // still has INVALID markers for unwritten cells.
        circuit_hal
            .generate_witness(mode, &trace, &global_buf, &data_buf)
            .context("witness generation failure (DIFF mode)")?;
        let cols = data_buf.cols;
        let rows = data_buf.rows;
        let mut cpu_snap = vec![0u32; data_buf.buf.size()];
        data_buf.buf.view(|slice: &[Val]| {
            let u32s: &[u32] =
                unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u32, slice.len()) };
            cpu_snap.copy_from_slice(u32s);
        });
        const INVALID_U32: u32 = 0xffffffffu32;
        let mut mismatches = 0usize;
        let mut gpu_wrote = 0usize;
        let mut cpu_wrote = 0usize;
        let mut both_wrote_match = 0usize;
        let mut gpu_only = 0usize;
        let mut cpu_only = 0usize;
        let mut candidate_cpu_only_nonzero = 0usize;
        for i in 0..snap.len() {
            let g = snap[i];
            let c = cpu_snap[i];
            let row = i % rows;
            let col = i / rows;
            let cycle = trace.cycles.get(row);
            let candidate_row = cycle
                .map(|cycle| is_witgen_diff_cycle(cycle.major, cycle.minor))
                .unwrap_or(false);
            let g_wrote = g != INVALID_U32;
            let c_wrote = c != INVALID_U32;
            if g_wrote {
                gpu_wrote += 1;
            }
            if c_wrote {
                cpu_wrote += 1;
            }
            if g_wrote && !c_wrote {
                gpu_only += 1;
            }
            if !g_wrote && c_wrote {
                cpu_only += 1;
                if candidate_row && c != 0 {
                    if candidate_cpu_only_nonzero < 20 {
                        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                            "DIFF_CANDIDATE_MISSING idx={} row={} col={} cpu=0x{:08x}",
                            i, row, col, c,
                        ));
                    }
                    candidate_cpu_only_nonzero += 1;
                }
            }
            if g_wrote && c_wrote && g != c {
                if mismatches < 20 {
                    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                        "DIFF_MISMATCH idx={} row={} col={} gpu=0x{:08x} cpu=0x{:08x}",
                        i, row, col, g, c,
                    ));
                }
                mismatches += 1;
            } else if g_wrote && c_wrote && g == c {
                both_wrote_match += 1;
            }
        }
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "DIFF_SUMMARY total_cells={} gpu_wrote={} cpu_wrote={} both_match={} mismatches={} gpu_only={} cpu_only={} candidate_cpu_only_nonzero={} rows={} cols={}",
            snap.len(),
            gpu_wrote,
            cpu_wrote,
            both_wrote_match,
            mismatches,
            gpu_only,
            cpu_only,
            candidate_cpu_only_nonzero,
            rows,
            cols,
        ));
        if mismatches != 0 || gpu_only != 0 || candidate_cpu_only_nonzero != 0 {
            anyhow::bail!(
                "DIFF mode: {} mismatches (gpu_only={} cpu_only={} candidate_cpu_only_nonzero={} both_match={}); see DIFF_MISMATCH + DIFF_CANDIDATE_MISSING + DIFF_SUMMARY",
                mismatches,
                gpu_only,
                cpu_only,
                candidate_cpu_only_nonzero,
                both_wrote_match
            );
        }

        hal.eltwise_zeroize_elem(&global_buf.buf);
        hal.eltwise_zeroize_elem(&data_buf.buf);
        let accum = MetaBuffer::new("accum", hal, cycles, REGCOUNT_ACCUM, true);
        crate::prove::witgen::WitnessGenerator::<WebGpuHal> {
            cycles,
            global: global_buf,
            code: code_buf,
            data: data_buf,
            accum,
            trace,
        }
    } else {
        // Production path offloads the witness CPU pass to a pool
        // worker so this wasm thread keeps servicing the other in-flight
        // segment's readback callbacks. Diff modes above keep the inline
        // blocking pass — they are single-segment diagnostics.
        let trace = circuit_hal
            .generate_witness_offloaded_async(mode, trace, &global_buf, &data_buf)
            .await?;
        crate::prove::witgen::WitnessGenerator::<WebGpuHal>::assemble_after_witgen(
            hal, trace, cycles, global_buf, code_buf, data_buf,
        )
    };

    Ok(WebGpuSegmentJob {
        hal: hal_rc.clone(),
        circuit_hal: circuit_hal_rc.clone(),
        witgen,
        po2,
    })
}

/// Transcript-bound commit phase — header commit, code/data/accum
/// group commits, accumulation, and finalize (eval_check + FRI).
/// Consumes the job; the seal is complete when this returns.
pub async fn webgpu_segment_commit_phase(job: WebGpuSegmentJob) -> Result<crate::prove::Seal> {
    let WebGpuSegmentJob {
        hal: hal_rc,
        circuit_hal: circuit_hal_rc,
        witgen,
        po2,
    } = job;
    let hal = hal_rc.as_ref();
    let circuit_hal = circuit_hal_rc.as_ref();

    let code = &witgen.code.buf;
    let data = &witgen.data.buf;
    let global = &witgen.global.buf;

    tracing::debug!("prove_inner");

    let mut prover = Prover::new(hal, TAPSET);
    let hashfn = &hal.get_hash_suite().hashfn;

    let header_timer = WebGpuStageTimer::new("rv32im_header_commit");
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
    drop(header_timer);

    let async_scopes = crate::prove::webgpu_async_authoritative_scopes();
    {
        let _gpu_scope = hal.gpu_authoritative_scope(async_scopes.code_data);
        {
            let _t = WebGpuStageTimer::new_active_for("commit_group_async rv32im_code", hal);
            prover
                .commit_group_async_in_place(REGISTER_GROUP_CODE, code.clone())
                .await?;
        }
        {
            let _t = WebGpuStageTimer::new_active_for("commit_group_async rv32im_data", hal);
            prover.commit_group_async(REGISTER_GROUP_DATA, data).await?;
        }
    }

    let accum_shadow_timer = WebGpuStageTimer::new("rv32im_accum_shadow_sync");
    let accum_shadow_rows = circuit_hal
        .sync_witgen_replace_accum_shadow_rows(&witgen.trace, &witgen.data)
        .await?;
    drop(accum_shadow_timer);
    if accum_shadow_rows != 0 {
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "rv32im_witgen_accum_shadow_gpu_sync rows={accum_shadow_rows}"
        ));
    }

    let mix: [Val; REGCOUNT_MIX] = std::array::from_fn(|_| prover.iop().random_elem());
    let mix = {
        let _t = WebGpuStageTimer::new("rv32im_witgen_accum");
        witgen.accum(hal, circuit_hal, &mix)?
    };
    {
        let _t = WebGpuStageTimer::new("rv32im_post_accum_sync");
        circuit_hal.post_accum_candidate_sync_async().await?;
    }

    let async_scopes = crate::prove::webgpu_async_authoritative_scopes();
    {
        let _t = WebGpuStageTimer::new_active_for("commit_group_async rv32im_accum", hal);
        prover
            .commit_group_async_in_place_scoped(
                REGISTER_GROUP_ACCUM,
                witgen.accum.buf.clone(),
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
}

pub fn prewarm_witgen_kernel_for_hal(hal: Rc<WebGpuHal>) {
    WebGpuCircuitHal::new(hal).prewarm_witgen_kernel();
}

pub fn enable_webgpu_witgen_accum_acceleration_for_hal(hal: Rc<WebGpuHal>) {
    set_witgen_gpu_probe_enabled(true);
    set_witgen_gpu_replace_enabled(true);
    set_witgen_gpu_replace_nonblocking_pending_enabled(true);
    set_accum_gpu_misc0_direct_enabled(true);
    set_accum_gpu_misc1_direct_enabled(true);
    set_accum_gpu_misc2_direct_enabled(true);
    set_witgen_gpu_mem0_replace_candidate_enabled(true);
    set_witgen_gpu_mem0_replace_minor_mask(1u16 << 2);
    set_accum_gpu_mem0_direct_enabled(true);
    set_accum_gpu_mem1_direct_enabled(true);
    set_accum_gpu_control0_direct_enabled(true);
    set_accum_gpu_poseidon1_direct_enabled(true);
    // Compile the POSEIDON1 direct-accum pipeline at init. Lazily it
    // lands on the FIRST segment's accum commit, where a serial
    // single-segment proof (BusyLoop) has nothing to hide it under
    // (+157 ms measured); pipelined multi-segment proofs hid it fully.
    if let Err(err) = WebGpuCircuitHal::new(hal.clone()).lookup_accum_poseidon1_direct_kernel() {
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "rv32im_accum_poseidon1_direct prewarm FAILED err={err:?}"
        ));
    }
    prewarm_witgen_kernel_for_hal(hal);
}

pub fn segment_prover(hal: Rc<WebGpuHal>) -> Result<Box<dyn SegmentProver>> {
    let circuit_hal = Rc::new(WebGpuCircuitHal::new(hal.clone()));
    // Kick off the witgen kernel Tint compile in the
    // background. No-op when WITGEN_GPU_PROBE_ENABLED is false.
    circuit_hal.prewarm_witgen_kernel();
    Ok(Box::new(WebGpuSegmentProver { hal, circuit_hal }))
}

pub(crate) fn step_mode_label(mode: StepMode) -> &'static str {
    match mode {
        StepMode::Parallel => "parallel",
        StepMode::SeqForward => "seq_forward",
        StepMode::SeqReverse => "seq_reverse",
    }
}
