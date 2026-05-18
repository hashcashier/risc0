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

use std::{cmp::Ordering, collections::BTreeMap, marker::PhantomData, slice};

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
        self.wom_rows.sort();

        let mut running = 0u32;
        for idx in &mut self.wom_index {
            let cur = *idx;
            *idx = running;
            running += cur;
        }

        self.inject_wom_backs(steps, args)?;
        self.do_step_verify_wom(mode, steps, args)
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
