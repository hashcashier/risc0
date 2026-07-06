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

//! Phase-level orchestration on the HAL: the `eval_check` hook,
//! grouped direct-accum dispatch, CPU-shadow row synchronization, and
//! the pre-witgen GPU dispatch decision.

use super::*;
#[allow(unused_imports)]
use super::{accum_wgsl::*, kernels::*, session::*, traits::*, witgen_wgsl::*};

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

impl WebGpuCircuitHal {
    pub(crate) fn witgen_replace_shadow_rows(
        preflight: &PreflightTrace,
        mask: u16,
    ) -> (Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>) {
        let arithmetic_rows = Vec::new();
        let bitwise_rows = Vec::new();
        let mut misc2_compare_rows = Vec::new();
        let mut misc2_branch_rows = Vec::new();
        for (cycle_idx, cycle) in preflight.cycles.iter().enumerate() {
            match cycle.major {
                // MISC0 lookup-table side effects are replayed directly from
                // preflight in rust_steps, so the CPU shadow does not need the
                // GPU-authored arithmetic/bitwise rows before generate_witness.
                0 if (mask & 0x0001) != 0 => {}
                2 if (mask & (1u16 << 2)) != 0 => match cycle.minor {
                    0 | 2 => misc2_compare_rows.push(cycle_idx as u32),
                    3 | 4 | 5 | 6 | 7 => misc2_branch_rows.push(cycle_idx as u32),
                    _ => {}
                },
                _ => {}
            }
        }
        (
            arithmetic_rows,
            bitwise_rows,
            misc2_compare_rows,
            misc2_branch_rows,
        )
    }

    pub(crate) fn witgen_replace_accum_shadow_rows_by_minor(
        preflight: &PreflightTrace,
        mask: u16,
    ) -> ([Vec<u32>; 8], Vec<u32>, Vec<u32>, Vec<u32>) {
        let mut misc0_rows: [Vec<u32>; 8] = std::array::from_fn(|_| Vec::new());
        let mut misc2_compare_rows = Vec::new();
        let mut misc2_branch_rows = Vec::new();
        let mut mem0_rows = Vec::new();
        for (cycle_idx, cycle) in preflight.cycles.iter().enumerate() {
            match cycle.major {
                0 if (mask & 0x0001) != 0 => match cycle.minor {
                    0 | 1 | 2 | 3 | 4 | 7 => {
                        misc0_rows[cycle.minor as usize].push(cycle_idx as u32)
                    }
                    _ => {}
                },
                2 if (mask & (1u16 << 2)) != 0 => match cycle.minor {
                    0 | 2 => misc2_compare_rows.push(cycle_idx as u32),
                    3 | 4 | 5 | 6 | 7 => misc2_branch_rows.push(cycle_idx as u32),
                    _ => {}
                },
                5 if (mask & (1u16 << 5)) != 0
                    && witgen_mem0_replace_minor_enabled(cycle.minor) =>
                {
                    mem0_rows.push(cycle_idx as u32);
                }
                _ => {}
            }
        }
        (misc0_rows, misc2_compare_rows, misc2_branch_rows, mem0_rows)
    }

    pub(crate) fn top_accum_major0_shadow_columns(prefix: usize, data_cols: usize) -> Vec<u32> {
        let capped = prefix.min(data_cols);
        (0..capped)
            .filter(|&col| col <= 1 || col >= 14)
            .map(|col| col as u32)
            .collect()
    }

    pub(crate) fn prefix_shadow_columns(prefix: usize, data_cols: usize) -> Vec<u32> {
        (0..prefix.min(data_cols)).map(|col| col as u32).collect()
    }

    pub(crate) fn push_shadow_column(columns: &mut Vec<u32>, data_cols: usize, col: u32) {
        if (col as usize) < data_cols {
            columns.push(col);
        }
    }

    pub(crate) fn push_shadow_column_range(
        columns: &mut Vec<u32>,
        data_cols: usize,
        start: u32,
        end_inclusive: u32,
    ) {
        for col in start..=end_inclusive {
            Self::push_shadow_column(columns, data_cols, col);
        }
    }

    pub(crate) fn misc0_accum_shadow_columns(minor: u8, data_cols: usize) -> Vec<u32> {
        let mut columns = Vec::with_capacity(160);
        for col in [0, 1, 14, 15, 17, 18, 21] {
            Self::push_shadow_column(&mut columns, data_cols, col);
        }
        match minor {
            1 => Self::push_shadow_column(&mut columns, data_cols, 22),
            2 => Self::push_shadow_column_range(&mut columns, data_cols, 22, 23),
            3 => Self::push_shadow_column_range(&mut columns, data_cols, 22, 24),
            4 => Self::push_shadow_column_range(&mut columns, data_cols, 22, 25),
            7 => Self::push_shadow_column_range(&mut columns, data_cols, 22, 28),
            _ => {}
        }
        Self::push_shadow_column_range(&mut columns, data_cols, 29, 40);
        Self::push_shadow_column_range(&mut columns, data_cols, 42, 43);
        Self::push_shadow_column_range(&mut columns, data_cols, 45, 46);
        Self::push_shadow_column_range(&mut columns, data_cols, 48, 49);
        Self::push_shadow_column_range(&mut columns, data_cols, 54, 75);
        Self::push_shadow_column_range(&mut columns, data_cols, 86, 87);
        Self::push_shadow_column_range(&mut columns, data_cols, 90, 125);
        Self::push_shadow_column_range(&mut columns, data_cols, 128, 131);
        if matches!(minor, 2 | 3 | 4) {
            Self::push_shadow_column_range(&mut columns, data_cols, 132, 195);
        }
        columns
    }

    pub(crate) async fn sync_witgen_replace_shadow_rows(
        &self,
        data: &MetaBuffer<WebGpuHal>,
        arithmetic_rows: &[u32],
        bitwise_rows: &[u32],
        misc2_compare_rows: &[u32],
        misc2_branch_rows: &[u32],
        source: &'static str,
    ) -> Result<()> {
        // The authoritative slices are MISC0 arithmetic/bitwise ops and
        // MISC2 rows where rust_steps short-circuits CPU writes. Keep the
        // row classes separate so branch-like MISC2 rows do not pay for the
        // signed-compare aux columns, and minor 1 remains CPU-owned.
        // Diff evidence shows `replace_cpu_only_nonzero=0` for this slice:
        // all nonzero CPU cells are already written by the GPU. The arithmetic
        // subset lives within [0, 132); bitwise rows additionally write their
        // shared ToBits_16 decomposition columns through [132, 196). Keep the
        // row groups separate so common ADD/SUB/ADDI rows do not pay for the
        // wider bitwise shadow readback. For MISC0 accum shadow repair, column
        // 1 is the active major-0 selector; columns 2..13 are inactive
        // top-level selectors and are not read once the major-0 branch is
        // selected. Omitting them avoids transferring 12 dead cells per
        // GPU-owned row while preserving the current CPU TopAccum path.
        const MISC0_ARITH_SHADOW_COLS: usize = 132;
        const MISC0_BITWISE_SHADOW_COLS: usize = 196;
        // MISC2 branch-like rows (minors 3..=7) need MiscInput + source-reg
        // fields through column 131. Compare rows (minors 0 and 2) also need
        // NormalizeU32 carries plus signed compare aux cells through column
        // 138.
        const MISC2_BRANCH_SHADOW_COLS: usize = 132;
        const MISC2_COMPARE_SHADOW_COLS: usize = 139;
        let misc0_arith_columns =
            Self::top_accum_major0_shadow_columns(MISC0_ARITH_SHADOW_COLS, data.cols);
        let misc0_bitwise_columns =
            Self::top_accum_major0_shadow_columns(MISC0_BITWISE_SHADOW_COLS, data.cols);
        let misc2_compare_columns =
            Self::prefix_shadow_columns(MISC2_COMPARE_SHADOW_COLS, data.cols);
        let misc2_branch_columns = Self::prefix_shadow_columns(MISC2_BRANCH_SHADOW_COLS, data.cols);
        let groups = [
            (misc0_arith_columns.as_slice(), arithmetic_rows),
            (misc0_bitwise_columns.as_slice(), bitwise_rows),
            (misc2_compare_columns.as_slice(), misc2_compare_rows),
            (misc2_branch_columns.as_slice(), misc2_branch_rows),
        ];
        if groups
            .iter()
            .all(|(columns, rows)| columns.is_empty() || rows.is_empty())
        {
            // The subsequent synchronous rust_steps pass only needs the CPU
            // shadow for non-replaced rows. Keep the mixed CPU/GPU ownership
            // intentional: MISC0 cells remain GPU-owned, CPU-written cells are
            // uploaded sparsely by eltwise_zeroize_elem before commitment.
            data.buf
                .sync_gpu_ranges_to_cpu_unchecked(self.hal.as_ref(), &[], source)
                .await?;
        } else {
            data.buf
                .sync_gpu_column_set_row_groups_to_cpu_unchecked(
                    self.hal.as_ref(),
                    data.rows,
                    groups.as_slice(),
                    source,
                )
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn sync_witgen_replace_accum_shadow_rows(
        &self,
        preflight: &PreflightTrace,
        data: &MetaBuffer<WebGpuHal>,
    ) -> Result<usize> {
        let mask = self.witgen_replace_arm_mask.get();
        if mask == 0 {
            return Ok(0);
        }
        let (misc0_rows, misc2_compare_rows, misc2_branch_rows, mem0_rows) =
            Self::witgen_replace_accum_shadow_rows_by_minor(preflight, mask);
        let gpu_direct_misc0 =
            ACCUM_GPU_MISC0_DIRECT_ENABLED.load(Ordering::SeqCst) && (mask & 0x0001) != 0;
        let gpu_direct_mem0 =
            ACCUM_GPU_MEM0_DIRECT_ENABLED.load(Ordering::SeqCst) && (mask & (1u16 << 5)) != 0;
        let misc0_read_rows = if gpu_direct_misc0 {
            0
        } else {
            [0usize, 1, 2, 3, 4, 7]
                .into_iter()
                .try_fold(0usize, |total, minor| {
                    total.checked_add(misc0_rows[minor].len())
                })
                .ok_or_else(|| anyhow::anyhow!("witgen accum shadow row count overflow"))?
        };
        let mem0_read_rows = if gpu_direct_mem0 { 0 } else { mem0_rows.len() };
        let rows = misc0_read_rows
            .checked_add(misc2_compare_rows.len())
            .and_then(|n| n.checked_add(misc2_branch_rows.len()))
            .and_then(|n| n.checked_add(mem0_read_rows))
            .ok_or_else(|| anyhow::anyhow!("witgen accum shadow row count overflow"))?;
        if rows == 0 {
            return Ok(0);
        }
        // Accumulation only reads a sparse subset of each MISC0 minor's GPU
        // witness cells. Keep these row classes exact instead of using the old
        // broad arithmetic/bitwise prefixes; xgboost evidence shows the common
        // rows need 90-97 columns and bitwise rows need 153-155 columns, not
        // the previous 120/184 column transfer.
        let misc0_columns: [Vec<u32>; 8] =
            std::array::from_fn(|minor| Self::misc0_accum_shadow_columns(minor as u8, data.cols));
        let misc2_compare_columns = Self::prefix_shadow_columns(139, data.cols);
        let misc2_branch_columns = Self::prefix_shadow_columns(132, data.cols);
        let mem0_columns = Self::prefix_shadow_columns(data.cols, data.cols);
        let empty_rows: &[u32] = &[];
        let groups = [
            (
                misc0_columns[0].as_slice(),
                if gpu_direct_misc0 {
                    empty_rows
                } else {
                    misc0_rows[0].as_slice()
                },
            ),
            (
                misc0_columns[1].as_slice(),
                if gpu_direct_misc0 {
                    empty_rows
                } else {
                    misc0_rows[1].as_slice()
                },
            ),
            (
                misc0_columns[2].as_slice(),
                if gpu_direct_misc0 {
                    empty_rows
                } else {
                    misc0_rows[2].as_slice()
                },
            ),
            (
                misc0_columns[3].as_slice(),
                if gpu_direct_misc0 {
                    empty_rows
                } else {
                    misc0_rows[3].as_slice()
                },
            ),
            (
                misc0_columns[4].as_slice(),
                if gpu_direct_misc0 {
                    empty_rows
                } else {
                    misc0_rows[4].as_slice()
                },
            ),
            (
                misc0_columns[7].as_slice(),
                if gpu_direct_misc0 {
                    empty_rows
                } else {
                    misc0_rows[7].as_slice()
                },
            ),
            (
                misc2_compare_columns.as_slice(),
                misc2_compare_rows.as_slice(),
            ),
            (
                misc2_branch_columns.as_slice(),
                misc2_branch_rows.as_slice(),
            ),
            (
                mem0_columns.as_slice(),
                if gpu_direct_mem0 {
                    empty_rows
                } else {
                    mem0_rows.as_slice()
                },
            ),
        ];
        data.buf
            .sync_gpu_column_set_row_groups_to_cpu_unchecked(
                self.hal.as_ref(),
                data.rows,
                groups.as_slice(),
                "witgen_accum_shadow_rows",
            )
            .await?;
        Ok(rows)
    }

    /// Async pre-dispatch hook.
    /// Does the pre-witgen GPU work (shadow_init + per-arm chunks) and then
    /// `sync_gpu_to_cpu` on the data buffer so rust_steps' subsequent
    /// view_mut sees GPU writes in the CPU shadow. Sets the short-circuit
    /// mask so rust_steps skips arms covered by GPU dispatch.
    ///
    /// Called from `prove_core_async` BEFORE `WitnessGenerator::populate_from_parts`
    /// (which runs the sync `generate_witness` / rust_steps path). No-op when
    /// the probe flag is off (legacy sync path still works).
    pub async fn pre_witgen_dispatch_async(
        &self,
        preflight: &PreflightTrace,
        data: &MetaBuffer<WebGpuHal>,
        global: &MetaBuffer<WebGpuHal>,
    ) -> Result<()> {
        let diff_mode = WITGEN_GPU_DIFF_ENABLED.load(Ordering::SeqCst);
        if !WITGEN_GPU_PROBE_ENABLED.load(Ordering::SeqCst) && !diff_mode {
            self.set_witgen_replace_arm_mask(0);
            return Ok(());
        }
        // Only run the GPU
        // dispatches + sync if we're actually going to short-circuit
        // (replace flag on). For probe-only mode, the dispatches'
        // partial cell writes would conflict with rust_steps' full
        // writes via set_at's "inconsistent set" check. Probe-only
        // exists to measure dispatch cost; with the async refactor it's
        // moot since we'd be syncing back values that rust_steps
        // overwrites anyway.
        //
        // Diff mode bypasses
        // the replace-off gate so we can run GPU writes for snapshot
        // purposes even when not short-circuiting. Caller in
        // `prove_core_async` will reset the CPU shadow + re-scatter
        // injector + force mask=0 after snapshot to keep rust_steps'
        // set_at consistency check satisfied.
        let replace_enabled = WITGEN_GPU_REPLACE_ENABLED.load(Ordering::SeqCst);
        if !replace_enabled && !diff_mode {
            self.set_witgen_replace_arm_mask(0);
            return Ok(());
        }
        let replace_mode = replace_enabled;
        let _timer = WebGpuStageTimer::new(format!(
            "witgen_arm_pre_witgen_dispatch_async cycles={}",
            preflight.cycles.len()
        ));

        let diff_only_mode = diff_mode && !replace_mode;
        let nonblocking_pending = WITGEN_GPU_REPLACE_NONBLOCKING_PENDING_ENABLED
            .load(Ordering::SeqCst)
            && replace_mode
            && !diff_only_mode;
        let mut needed_replacement_arm = false;
        if replace_mode || diff_mode {
            let mut needed_arms = [false; 13];
            for cycle in &preflight.cycles {
                let arm = cycle.major as usize;
                let needed = if diff_only_mode {
                    is_witgen_diff_cycle(cycle.major, cycle.minor)
                } else {
                    is_witgen_replace_cycle(cycle.major, cycle.minor)
                        && is_witgen_replace_supported_arm(arm)
                };
                if arm < needed_arms.len() && needed {
                    needed_arms[arm] = true;
                }
            }
            for (arm_idx, needed) in needed_arms.iter().enumerate() {
                if *needed {
                    if replace_mode {
                        needed_replacement_arm = true;
                    }
                    if nonblocking_pending {
                        continue;
                    }
                    if let Err(err) = self
                        .ensure_witgen_replace_arm_ready_async(arm_idx, preflight, diff_only_mode)
                        .await
                    {
                        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                            "witgen_arm_on_demand arm{arm_idx} FAILED err={err:?}"
                        ));
                    }
                }
            }
        }

        let desired_mask = if replace_mode {
            self.ready_witgen_replace_mask(preflight) & witgen_replace_supported_arm_mask()
        } else {
            0
        };
        if replace_mode && desired_mask == 0 {
            self.set_witgen_replace_arm_mask(0);
            if nonblocking_pending && needed_replacement_arm {
                WITGEN_GPU_REPLACE_NONBLOCKING_PENDING_SKIPS.fetch_add(1, Ordering::SeqCst);
                risc0_zkp::hal::webgpu::log_webgpu_metric(
                    "witgen_arm_pre_witgen_dispatch_async nonblocking_pending_skip",
                );
            }
            risc0_zkp::hal::webgpu::log_webgpu_metric(
                "witgen_arm_pre_witgen_dispatch_async mask=0x0000 no_ready_replacement no_sync",
            );
            return Ok(());
        }

        // Keep unwritten GPU witness cells at INVALID. In replacement mode
        // seed the browser buffer directly with INVALID; uploading the full
        // CPU shadow here costs one full data matrix per segment before any
        // GPU witgen work can run.
        let seeded_on_gpu = if replace_mode && !diff_mode {
            self.hal.init_invalid_elem(&data.buf).unwrap_or_else(|err| {
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_gpu_seed FAILED err={err:?}"
                ));
                false
            })
        } else {
            false
        };
        if seeded_on_gpu {
            risc0_zkp::hal::webgpu::log_webgpu_metric("witgen_arm_gpu_seed invalid_fill");
        } else {
            data.buf.sync_cpu_to_gpu(self.hal.as_ref())?;
        }

        // 1. Per-arm chunk0+chunk1 dispatches. Returns the arm_idx values
        //    actually dispatched (kernel ready + cycles > 0).
        //    The dispatch path also runs shadow_init using the same uploaded
        //    preflight metadata buffer.
        let dispatched_arms = match self.dispatch_witgen_per_arm_probe(
            data,
            global,
            preflight,
            replace_mode.then_some(desired_mask),
            diff_only_mode,
        ) {
            Ok(arms) => arms,
            Err(err) => {
                risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
                    "witgen_arm_per_arm_dispatch FAILED err={err:?}"
                ));
                Vec::new()
            }
        };
        // 2. Compute the short-circuit mask (MISC0-only for the initial
        //    bring-up; expand once verify passes).
        let mut mask: u16 = 0;
        if replace_mode {
            for arm_idx in &dispatched_arms {
                if is_zero_back_reg_arm(*arm_idx) && *arm_idx < 13 {
                    mask |= 1u16 << arm_idx;
                }
            }
            mask &= desired_mask;
        }
        self.set_witgen_replace_arm_mask(mask);
        risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
            "witgen_arm_pre_witgen_dispatch_async mask=0x{:04x} dispatched_arms={:?}",
            mask, dispatched_arms,
        ));
        if replace_mode && mask == 0 {
            return Ok(());
        }
        let (arithmetic_rows, bitwise_rows, misc2_compare_rows, misc2_branch_rows) =
            if replace_mode && !diff_mode {
                Self::witgen_replace_shadow_rows(preflight, mask)
            } else {
                (Vec::new(), Vec::new(), Vec::new(), Vec::new())
            };
        // 3. Mark GPU writes as authoritative + sync GPU -> CPU shadow.
        //    In normal replacement mode, repair only the columns needed by
        //    the current MISC0/ADD slice. Explicit diff modes keep the full
        //    unchecked readback so diagnostics still compare every cell.
        data.buf.mark_gpu_dirty();
        if replace_mode && !diff_mode {
            self.sync_witgen_replace_shadow_rows(
                data,
                arithmetic_rows.as_slice(),
                bitwise_rows.as_slice(),
                misc2_compare_rows.as_slice(),
                misc2_branch_rows.as_slice(),
                "witgen_data_shadow_rows",
            )
            .await?;
        } else {
            data.buf
                .sync_gpu_to_cpu_unchecked(self.hal.as_ref())
                .await?;
        }
        Ok(())
    }

    /// The production witgen CPU pass, run on a rayon pool worker
    /// instead of this wasm thread. Measurement showed why this is
    /// load-bearing: with witgen inline, segment N+1's ~100-200 ms
    /// rayon-joins starve segment N's readback callbacks (the merkle code
    /// root readback summed 338 -> 3000 ms across xgboost), and the
    /// pipeline overlap buys nothing. `CpuBuffer` shadow handles are
    /// `Send + Sync`; the trace travels through the worker and back so
    /// nothing is cloned. Eqz elision is held as a guard across the await
    /// (all takers set-true/restore, so overlap with another segment's
    /// blocking accum scope composes).
    pub(crate) async fn generate_witness_offloaded_async(
        &self,
        mode: StepMode,
        trace: PreflightTrace,
        global: &MetaBuffer<WebGpuHal>,
        data: &MetaBuffer<WebGpuHal>,
    ) -> Result<PreflightTrace> {
        let _timer = WebGpuStageTimer::new(format!(
            "rv32im_witgen mode={} cycles={} txns={} bigint_bytes={} data_rows={} data_cols={} offload=pool",
            step_mode_label(mode),
            trace.cycles.len(),
            trace.txns.len(),
            trace.bigint_bytes.len(),
            data.rows,
            data.cols
        ));
        let replace_arm_mask = self.witgen_replace_arm_mask.get();
        let data_shadow = data.buf.begin_cpu_shadow_offload_mut();
        let global_shadow = global.buf.begin_cpu_shadow_offload();
        let (global_rows, global_cols, global_checked) = (global.rows, global.cols, global.checked);
        let (data_rows, data_cols, data_checked) = (data.rows, data.cols, data.checked);
        let eqz_guard = crate::prove::hal::rust_steps::begin_eqz_elided();
        let (tx, rx) = futures::channel::oneshot::channel();
        rayon::spawn(move || {
            let result = crate::prove::hal::rust_steps::generate_witness_on_shadows(
                mode,
                replace_arm_mask,
                &trace,
                &global_shadow,
                global_rows,
                global_cols,
                global_checked,
                &data_shadow,
                data_rows,
                data_cols,
                data_checked,
            );
            let _ = tx.send((result, trace));
        });
        let (result, trace) = rx
            .await
            .map_err(|_| anyhow::anyhow!("offloaded rv32im witgen worker dropped its result"))?;
        drop(eqz_guard);
        result.context("witness generation failure")?;
        data.buf.finish_cpu_shadow_offload_mut();
        Ok(trace)
    }

    pub async fn post_accum_candidate_sync_async(&self) -> Result<()> {
        if !ACCUM_GPU_CANDIDATE_SYNC_ENABLED.load(Ordering::SeqCst) {
            return Ok(());
        }
        let _timer = WebGpuStageTimer::new_active_for(
            "rv32im_accumulate candidate_sync_wait",
            self.hal.as_ref(),
        );
        self.hal
            .wait_idle()
            .await
            .context("TopAccum candidate sync wait failed")?;
        ACCUM_GPU_CANDIDATE_SYNC_WAITS.fetch_add(1, Ordering::SeqCst);
        risc0_zkp::hal::webgpu::log_webgpu_metric(
            "rv32im_accumulate candidate_sync_wait completed",
        );
        Ok(())
    }
}
