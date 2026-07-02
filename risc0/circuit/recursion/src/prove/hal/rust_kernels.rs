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

#![allow(
    dead_code,
    non_snake_case,
    unused_assignments,
    unused_mut,
    unused_parens,
    unused_variables
)]

use std::{
    cmp::Ordering,
    collections::BTreeMap,
    marker::PhantomData,
    slice,
    sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering},
};

use anyhow::{ensure, Result};
use risc0_circuit_recursion_sys::{RawPreflightCycle, RawPreflightTrace, StepMode};
use risc0_core::field::{
    baby_bear::{BabyBearElem, BabyBearExtElem},
    Elem as _,
};
use risc0_zkp::hal::{
    webgpu::{WebGpuBuffer, WebGpuStageTimer},
    Buffer,
};

type Fp = BabyBearElem;
type FpExt = BabyBearExtElem;

const K_MAX_WOM_ROWS_PER_CYCLE: usize = 9;
const K_INVALID_PATTERN: u32 = 0xffff_ffff;

static WOM_SORT_PROFILE_ENABLED: AtomicBool = AtomicBool::new(false);
static WOM_SORT_PROFILE_CALLS: AtomicU64 = AtomicU64::new(0);
static WOM_SORT_PROFILE_ROWS: AtomicU64 = AtomicU64::new(0);
static WOM_SORT_PROFILE_ADDR_GROUPS: AtomicU64 = AtomicU64::new(0);
static WOM_SORT_PROFILE_REPEATED_ADDR_GROUPS: AtomicU64 = AtomicU64::new(0);
static WOM_SORT_PROFILE_DISTINCT_VALUE_GROUPS: AtomicU64 = AtomicU64::new(0);
static WOM_SORT_PROFILE_DISTINCT_VALUE_ROWS: AtomicU64 = AtomicU64::new(0);
static WOM_SORT_PROFILE_MAX_ADDR_GROUP: AtomicU64 = AtomicU64::new(0);

pub(crate) fn set_wom_sort_profile_enabled(enabled: bool) {
    if enabled {
        WOM_SORT_PROFILE_CALLS.store(0, AtomicOrdering::SeqCst);
        WOM_SORT_PROFILE_ROWS.store(0, AtomicOrdering::SeqCst);
        WOM_SORT_PROFILE_ADDR_GROUPS.store(0, AtomicOrdering::SeqCst);
        WOM_SORT_PROFILE_REPEATED_ADDR_GROUPS.store(0, AtomicOrdering::SeqCst);
        WOM_SORT_PROFILE_DISTINCT_VALUE_GROUPS.store(0, AtomicOrdering::SeqCst);
        WOM_SORT_PROFILE_DISTINCT_VALUE_ROWS.store(0, AtomicOrdering::SeqCst);
        WOM_SORT_PROFILE_MAX_ADDR_GROUP.store(0, AtomicOrdering::SeqCst);
    }
    WOM_SORT_PROFILE_ENABLED.store(enabled, AtomicOrdering::SeqCst);
}

pub(crate) fn wom_sort_profile_snapshot() -> [u64; 7] {
    [
        WOM_SORT_PROFILE_CALLS.load(AtomicOrdering::SeqCst),
        WOM_SORT_PROFILE_ROWS.load(AtomicOrdering::SeqCst),
        WOM_SORT_PROFILE_ADDR_GROUPS.load(AtomicOrdering::SeqCst),
        WOM_SORT_PROFILE_REPEATED_ADDR_GROUPS.load(AtomicOrdering::SeqCst),
        WOM_SORT_PROFILE_DISTINCT_VALUE_GROUPS.load(AtomicOrdering::SeqCst),
        WOM_SORT_PROFILE_DISTINCT_VALUE_ROWS.load(AtomicOrdering::SeqCst),
        WOM_SORT_PROFILE_MAX_ADDR_GROUP.load(AtomicOrdering::SeqCst),
    ]
}

#[derive(Clone, Copy)]
struct WomArgumentRow {
    addr: u32,
    value: FpExt,
}

impl WomArgumentRow {
    fn invalid() -> Self {
        Self {
            addr: K_INVALID_PATTERN,
            value: FpExt::INVALID,
        }
    }
}

impl Eq for WomArgumentRow {}

impl PartialEq for WomArgumentRow {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
            && (self.addr == K_INVALID_PATTERN || self.value.elems() == other.value.elems())
    }
}

impl Ord for WomArgumentRow {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.addr.cmp(&other.addr) {
            Ordering::Equal if self.addr == K_INVALID_PATTERN => Ordering::Equal,
            Ordering::Equal => self.value.elems().cmp(other.value.elems()),
            ordering => ordering,
        }
    }
}

impl PartialOrd for WomArgumentRow {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct KernelArgs {
    ptrs: [*mut Fp; 5],
    lens: [usize; 5],
    marker: PhantomData<Fp>,
}

#[derive(Clone, Debug)]
pub(crate) struct WomGpuVerifyPlan {
    pub(crate) work_cycles: u32,
    pub(crate) total_cycles: u32,
    pub(crate) valid_rows: u32,
    pub(crate) cycle_prefixes: Vec<u32>,
    pub(crate) bucket_bases: Vec<u32>,
}

impl KernelArgs {
    fn exec(ctrl: &[Fp], global: &mut [Fp], data: &mut [Fp]) -> Self {
        Self {
            ptrs: [
                ctrl.as_ptr() as *mut Fp,
                global.as_mut_ptr(),
                data.as_mut_ptr(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            ],
            lens: [ctrl.len(), global.len(), data.len(), 0, 0],
            marker: PhantomData,
        }
    }

    fn accum(ctrl: &[Fp], global: &[Fp], data: &[Fp], mix: &[Fp], accum: &mut [Fp]) -> Self {
        Self {
            ptrs: [
                ctrl.as_ptr() as *mut Fp,
                global.as_ptr() as *mut Fp,
                data.as_ptr() as *mut Fp,
                mix.as_ptr() as *mut Fp,
                accum.as_mut_ptr(),
            ],
            lens: [ctrl.len(), global.len(), data.len(), mix.len(), accum.len()],
            marker: PhantomData,
        }
    }

    fn get(&self, group: usize, idx: usize) -> Fp {
        assert!(idx < self.lens[group], "kernel read out of bounds");
        // SAFETY: Bounds are checked above; pointers come from live HAL buffer views.
        unsafe { *self.ptrs[group].add(idx) }
    }

    fn set(&mut self, group: usize, idx: usize, value: Fp) -> Result<()> {
        ensure!(idx < self.lens[group], "kernel write out of bounds");
        // SAFETY: Bounds are checked above; pointers come from live mutable HAL buffer views.
        let elem = unsafe { &mut *self.ptrs[group].add(idx) };
        ensure!(
            !elem.is_valid() || *elem == value,
            "inconsistent generated recursion register write"
        );
        *elem = value;
        Ok(())
    }
}

struct MachineContext<'a> {
    wom: &'a [FpExt],
    cycles: Vec<RawPreflightCycle>,
    iops: &'a [FpExt],
    byte_reads: &'a BTreeMap<usize, Vec<u32>>,
    wom_rows: Vec<WomArgumentRow>,
    wom_index: Vec<u32>,
}

impl<'a> MachineContext<'a> {
    fn new(preflight: &'a RawPreflightTrace, byte_reads: &'a BTreeMap<usize, Vec<u32>>) -> Self {
        let cycles = unsafe {
            slice::from_raw_parts(preflight.cycles, preflight.num_cycles as usize).to_vec()
        };
        let wom = unsafe { slice::from_raw_parts(preflight.wom, preflight.num_woms as usize) };
        let iops = unsafe { slice::from_raw_parts(preflight.iops, preflight.num_iops as usize) };
        let wom_rows = vec![WomArgumentRow::invalid(); cycles.len() * K_MAX_WOM_ROWS_PER_CYCLE];
        let wom_index = vec![0; cycles.len()];
        Self {
            wom,
            cycles,
            iops,
            byte_reads,
            wom_rows,
            wom_index,
        }
    }

    fn do_step_exec(&mut self, mode: StepMode, steps: usize, args: &mut KernelArgs) -> Result<()> {
        match mode {
            StepMode::Parallel | StepMode::SeqForward => {
                for cycle in 0..self.cycles.len() {
                    step_exec(self, steps, cycle, args)?;
                }
            }
            StepMode::SeqReverse => {
                for cycle in (0..self.cycles.len()).rev() {
                    self.par_step_exec(steps, cycle, args)?;
                }
            }
        }
        Ok(())
    }

    fn par_step_exec(
        &mut self,
        steps: usize,
        mut cycle: usize,
        args: &mut KernelArgs,
    ) -> Result<()> {
        if cycle == 0 || self.cycles[cycle].is_par_safe != 0 {
            step_exec(self, steps, cycle, args)?;
            cycle += 1;
            while cycle < self.cycles.len() && self.cycles[cycle].is_par_safe == 0 {
                step_exec(self, steps, cycle, args)?;
                cycle += 1;
            }
        }
        Ok(())
    }

    fn verify_wom(&mut self, mode: StepMode, steps: usize, args: &mut KernelArgs) -> Result<()> {
        self.prepare_wom_for_verify(steps, args)?;
        self.do_step_verify_wom(mode, steps, args)
    }

    fn gpu_verify_plan(&self, total_cycles: usize) -> Result<WomGpuVerifyPlan> {
        let mut valid_rows = 0u32;
        let mut cycle_prefixes = Vec::with_capacity(self.wom_index.len());
        let mut bucket_counts = Vec::<u32>::new();
        bucket_counts.push(0);

        for (cycle, &count) in self.wom_index.iter().enumerate() {
            cycle_prefixes.push(valid_rows);
            valid_rows = valid_rows
                .checked_add(count)
                .ok_or_else(|| anyhow::anyhow!("recursion WOM row count overflow"))?;

            let count = count as usize;
            ensure!(
                count <= K_MAX_WOM_ROWS_PER_CYCLE,
                "too many recursion WOM rows per cycle"
            );
            let base = cycle
                .checked_mul(K_MAX_WOM_ROWS_PER_CYCLE)
                .ok_or_else(|| anyhow::anyhow!("recursion WOM row base overflow"))?;
            for row in &self.wom_rows[base..base + count] {
                ensure!(
                    row.addr != K_INVALID_PATTERN,
                    "valid recursion WOM row left invalid"
                );
                let elems = row.value.elems();
                let bucket = if row.addr == 0 && elems.iter().all(|&elem| elem == Fp::ZERO) {
                    0usize
                } else {
                    row.addr as usize + 1
                };
                if bucket >= bucket_counts.len() {
                    bucket_counts.resize(bucket + 1, 0);
                }
                bucket_counts[bucket] = bucket_counts[bucket]
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("recursion WOM bucket count overflow"))?;
            }
        }

        let mut running = 0u32;
        let mut bucket_bases = Vec::with_capacity(bucket_counts.len());
        for count in bucket_counts {
            bucket_bases.push(running);
            running = running
                .checked_add(count)
                .ok_or_else(|| anyhow::anyhow!("recursion WOM bucket base overflow"))?;
        }
        ensure!(
            running == valid_rows,
            "recursion WOM bucket counts disagree with cycle counts"
        );

        Ok(WomGpuVerifyPlan {
            work_cycles: self.cycles.len() as u32,
            total_cycles: total_cycles as u32,
            valid_rows,
            cycle_prefixes,
            bucket_bases,
        })
    }

    fn prepare_wom_for_verify(&mut self, steps: usize, args: &mut KernelArgs) -> Result<u32> {
        let valid_rows = self
            .wom_index
            .iter()
            .try_fold(0usize, |acc, &count| acc.checked_add(count as usize))
            .expect("recursion WOM row count overflow");
        self.wom_rows.sort();
        record_wom_sort_profile(&self.wom_rows, valid_rows);

        let mut running = 0u32;
        for idx in &mut self.wom_index {
            let cur = *idx;
            *idx = running;
            running += cur;
        }

        self.inject_wom_backs(steps, args)?;
        Ok(running)
    }

    fn do_step_verify_wom(
        &mut self,
        mode: StepMode,
        steps: usize,
        args: &mut KernelArgs,
    ) -> Result<()> {
        match mode {
            StepMode::Parallel | StepMode::SeqForward => {
                for cycle in 0..self.cycles.len() {
                    step_verify_mem(self, steps, cycle, args)?;
                }
            }
            StepMode::SeqReverse => {
                for cycle in (0..self.cycles.len()).rev() {
                    step_verify_mem(self, steps, cycle, args)?;
                }
            }
        }
        Ok(())
    }

    fn inject_wom_backs(&mut self, steps: usize, args: &mut KernelArgs) -> Result<()> {
        for cycle in 1..self.cycles.len() {
            let idx = self.wom_index[cycle] as usize;
            if idx != 0 {
                let prev = self.wom_rows[idx - 1];
                args.set(2, cycle - 1, Fp::new(prev.addr))?;
                for (i, elem) in prev.value.elems().iter().copied().enumerate() {
                    args.set(2, (i + 1) * steps + cycle - 1, elem)?;
                }
            } else {
                for i in 0..5 {
                    args.set(2, i * steps + cycle - 1, Fp::ZERO)?;
                }
            }
        }
        Ok(())
    }

    fn read_iop_header(&mut self, _cycle: usize, _args: [Fp; 2]) -> Result<()> {
        Ok(())
    }

    fn read_iop_body(&mut self, cycle: usize, _args: [Fp; 3]) -> Result<[Fp; 4]> {
        let iop_idx = self.cycles[cycle].iop_idx as usize;
        ensure!(
            iop_idx < self.iops.len(),
            "recursion IOP read out of bounds"
        );
        self.cycles[cycle].iop_idx += 1;
        let elems = self.iops[iop_idx].elems();
        Ok([elems[0], elems[1], elems[2], elems[3]])
    }

    fn wom_read(&mut self, _cycle: usize, args: [Fp; 1]) -> Result<[Fp; 4]> {
        let addr = args[0].as_u32() as usize;
        ensure!(addr < self.wom.len(), "recursion WOM read out of bounds");
        let elems = self.wom[addr].elems();
        Ok([elems[0], elems[1], elems[2], elems[3]])
    }

    fn wom_write(&mut self, _cycle: usize, _args: [Fp; 5]) -> Result<()> {
        Ok(())
    }

    fn plonk_write_wom(&mut self, cycle: usize, args: [Fp; 5]) -> Result<()> {
        let idx = self.wom_index[cycle] as usize;
        ensure!(
            idx < K_MAX_WOM_ROWS_PER_CYCLE,
            "too many WOM rows per cycle"
        );
        self.wom_index[cycle] += 1;
        self.wom_rows[cycle * K_MAX_WOM_ROWS_PER_CYCLE + idx] = WomArgumentRow {
            addr: args[0].as_u32(),
            value: FpExt::new(args[1], args[2], args[3], args[4]),
        };
        Ok(())
    }

    fn plonk_read_wom(&mut self, cycle: usize) -> Result<[Fp; 5]> {
        let idx = self.wom_index[cycle] as usize;
        ensure!(idx < self.wom_rows.len(), "WOM plonk read out of bounds");
        self.wom_index[cycle] += 1;
        let row = self.wom_rows[idx];
        let elems = row.value.elems();
        Ok([Fp::new(row.addr), elems[0], elems[1], elems[2], elems[3]])
    }

    fn read_coefficients(&mut self, cycle: usize) -> Result<[Fp; 16]> {
        let coeffs = self
            .byte_reads
            .get(&cycle)
            .ok_or_else(|| anyhow::anyhow!("missing checked-byte reads for cycle {cycle}"))?;
        ensure!(
            coeffs.len() * 4 == 16,
            "unexpected checked-byte coefficient count"
        );
        let mut out = [Fp::ZERO; 16];
        for (out, coeff) in out.chunks_mut(4).zip(coeffs.iter()) {
            let bytes = coeff.to_le_bytes();
            for i in 0..4 {
                out[i] = Fp::new(bytes[i] as u32);
            }
        }
        Ok(out)
    }

    fn log(&mut self, _cycle: usize, _args: &[Fp]) -> Result<()> {
        Ok(())
    }
}

struct AccumContext {
    accum: Vec<FpExt>,
}

impl AccumContext {
    fn new(work_cycles: usize) -> Self {
        Self {
            accum: vec![FpExt::ONE; work_cycles],
        }
    }

    fn compute_accum(
        &mut self,
        work_cycles: usize,
        total_cycles: usize,
        args: &mut KernelArgs,
    ) -> Result<()> {
        for cycle in 0..work_cycles {
            step_compute_accum(self, total_cycles, cycle, args)?;
        }
        Ok(())
    }

    fn calc_prefix_products(&mut self) {
        let mut running = FpExt::ONE;
        for value in &mut self.accum {
            running *= *value;
            *value = running;
        }
    }

    fn verify_accum(
        &mut self,
        work_cycles: usize,
        total_cycles: usize,
        args: &mut KernelArgs,
    ) -> Result<()> {
        for cycle in 0..work_cycles {
            step_verify_accum(self, total_cycles, cycle, args)?;
        }
        Ok(())
    }

    fn plonk_write_accum_wom(&mut self, cycle: usize, args: [Fp; 4]) -> Result<()> {
        ensure!(cycle < self.accum.len(), "accum write out of bounds");
        self.accum[cycle] = FpExt::new(args[0], args[1], args[2], args[3]);
        Ok(())
    }

    fn plonk_read_accum_wom(&mut self, cycle: usize) -> Result<[Fp; 4]> {
        ensure!(cycle < self.accum.len(), "accum read out of bounds");
        let elems = self.accum[cycle].elems();
        Ok([elems[0], elems[1], elems[2], elems[3]])
    }
}

fn record_wom_sort_profile(sorted_rows: &[WomArgumentRow], valid_rows: usize) {
    if !WOM_SORT_PROFILE_ENABLED.load(AtomicOrdering::SeqCst) {
        return;
    }

    let rows = &sorted_rows[..valid_rows.min(sorted_rows.len())];
    let mut addr_groups = 0u64;
    let mut repeated_addr_groups = 0u64;
    let mut distinct_value_groups = 0u64;
    let mut distinct_value_rows = 0u64;
    let mut max_addr_group = 0u64;

    let mut idx = 0usize;
    while idx < rows.len() {
        let addr = rows[idx].addr;
        let first_value = rows[idx].value.elems();
        let group_start = idx;
        let mut all_values_same = true;
        idx += 1;
        while idx < rows.len() && rows[idx].addr == addr {
            if rows[idx].value.elems() != first_value {
                all_values_same = false;
            }
            idx += 1;
        }

        let group_len = (idx - group_start) as u64;
        addr_groups += 1;
        max_addr_group = max_addr_group.max(group_len);
        if group_len > 1 {
            repeated_addr_groups += 1;
            if !all_values_same {
                distinct_value_groups += 1;
                distinct_value_rows += group_len;
            }
        }
    }

    WOM_SORT_PROFILE_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
    WOM_SORT_PROFILE_ROWS.fetch_add(rows.len() as u64, AtomicOrdering::SeqCst);
    WOM_SORT_PROFILE_ADDR_GROUPS.fetch_add(addr_groups, AtomicOrdering::SeqCst);
    WOM_SORT_PROFILE_REPEATED_ADDR_GROUPS.fetch_add(repeated_addr_groups, AtomicOrdering::SeqCst);
    WOM_SORT_PROFILE_DISTINCT_VALUE_GROUPS.fetch_add(distinct_value_groups, AtomicOrdering::SeqCst);
    WOM_SORT_PROFILE_DISTINCT_VALUE_ROWS.fetch_add(distinct_value_rows, AtomicOrdering::SeqCst);
    WOM_SORT_PROFILE_MAX_ADDR_GROUP.fetch_max(max_addr_group, AtomicOrdering::SeqCst);

    risc0_zkp::hal::webgpu::log_webgpu_metric(&format!(
        "recursion_wom_sort_profile rows={} addr_groups={} repeated_addr_groups={} distinct_value_groups={} distinct_value_rows={} max_addr_group={}",
        rows.len(),
        addr_groups,
        repeated_addr_groups,
        distinct_value_groups,
        distinct_value_rows,
        max_addr_group,
    ));
}

pub(crate) fn generate_witness(
    mode: StepMode,
    total_cycles: u32,
    preflight: &RawPreflightTrace,
    byte_reads: &BTreeMap<usize, Vec<u32>>,
    ctrl: &WebGpuBuffer<Fp>,
    data: &WebGpuBuffer<Fp>,
    global: &WebGpuBuffer<Fp>,
) -> Result<()> {
    let total_cycles = total_cycles as usize;
    let mut result = Ok(());
    ctrl.view(|ctrl| {
        global.view_mut(|global| {
            data.view_mut(|data| {
                let mut ctx = MachineContext::new(preflight, byte_reads);
                let mut args = KernelArgs::exec(ctrl, global, data);
                result = ctx
                    .do_step_exec(mode, total_cycles, &mut args)
                    .and_then(|_| ctx.verify_wom(mode, total_cycles, &mut args));
            });
        });
    });
    result
}

pub(crate) fn generate_witness_exec_plan(
    mode: StepMode,
    total_cycles: u32,
    preflight: &RawPreflightTrace,
    byte_reads: &BTreeMap<usize, Vec<u32>>,
    ctrl: &WebGpuBuffer<Fp>,
    data: &WebGpuBuffer<Fp>,
    global: &WebGpuBuffer<Fp>,
) -> Result<WomGpuVerifyPlan> {
    let total_cycles = total_cycles as usize;
    let mut result = None;
    ctrl.view(|ctrl| {
        global.view_mut(|global| {
            data.view_mut(|data| {
                let mut ctx = MachineContext::new(preflight, byte_reads);
                let mut args = KernelArgs::exec(ctrl, global, data);
                result = Some(
                    ctx.do_step_exec(mode, total_cycles, &mut args)
                        .and_then(|_| ctx.gpu_verify_plan(total_cycles)),
                );
            });
        });
    });
    result.expect("recursion exec-plan witness generation did not run")
}

pub(crate) fn accumulate(
    work_cycles: u32,
    total_cycles: u32,
    ctrl: &WebGpuBuffer<Fp>,
    global: &WebGpuBuffer<Fp>,
    data: &WebGpuBuffer<Fp>,
    mix: &WebGpuBuffer<Fp>,
    accum: &WebGpuBuffer<Fp>,
) -> Result<()> {
    let work_cycles = work_cycles as usize;
    let total_cycles = total_cycles as usize;
    let mut result = Ok(());
    ctrl.view(|ctrl| {
        global.view(|global| {
            data.view(|data| {
                mix.view(|mix| {
                    accum.view_mut(|accum| {
                        let mut ctx = AccumContext::new(work_cycles);
                        let mut args = KernelArgs::accum(ctrl, global, data, mix, accum);
                        result = {
                            let _timer = WebGpuStageTimer::new(format!(
                                "recursion_accumulate compute_accum work_cycles={} total_cycles={}",
                                work_cycles, total_cycles
                            ));
                            ctx.compute_accum(work_cycles, total_cycles, &mut args)
                        }
                        .map(|_| {
                            let _timer = WebGpuStageTimer::new(format!(
                                "recursion_accumulate prefix_products work_cycles={work_cycles}"
                            ));
                            ctx.calc_prefix_products()
                        })
                        .and_then(|_| {
                            let _timer = WebGpuStageTimer::new(format!(
                                "recursion_accumulate verify_accum work_cycles={} total_cycles={}",
                                work_cycles, total_cycles
                            ));
                            ctx.verify_accum(work_cycles, total_cycles, &mut args)
                        });
                    });
                });
            });
        });
    });
    result
}

include!("rust_kernels_generated.rs.inc");
