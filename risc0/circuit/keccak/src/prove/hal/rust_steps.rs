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
    unused_macros,
    unused_mut,
    unused_parens,
    unused_variables
)]

use std::marker::PhantomData;

use anyhow::{ensure, Result};
use rayon::prelude::*;
use risc0_core::field::Elem as _;
use risc0_zkp::{hal::Buffer, layout::Reg};

use super::{MetaBuffer, PreflightCycleOrder, PreflightTrace, StepMode};
use crate::{prove::KeccakState, zirgen::circuit::*};

type Index = usize;

pub(crate) struct BufferRow<T> {
    buf: *mut T,
    rows: usize,
    cols: usize,
    checked: bool,
    is_global: bool,
    marker: PhantomData<T>,
}

impl<T> Clone for BufferRow<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for BufferRow<T> {}

// SAFETY: raw column-major view for the witness steppers; the parallel
// per-cycle protocol matches the C reference (keccak-sys ffi.cpp runs
// stepExec under poolstl::par capturing the same raw buffers) — each cycle's
// step writes only its own row, and `checked` set conflicts panic loudly.
unsafe impl<T: Send> Send for BufferRow<T> {}
unsafe impl<T: Sync> Sync for BufferRow<T> {}

impl BufferRow<Val> {
    fn mutable(slice: &mut [Val], rows: usize, cols: usize, checked: bool) -> Self {
        Self {
            buf: slice.as_mut_ptr(),
            rows,
            cols,
            checked,
            is_global: false,
            marker: PhantomData,
        }
    }

    fn immutable(slice: &[Val], rows: usize, cols: usize, checked: bool) -> Self {
        Self {
            buf: slice.as_ptr() as *mut Val,
            rows,
            cols,
            checked,
            is_global: false,
            marker: PhantomData,
        }
    }

    fn global(slice: &mut [Val], rows: usize, cols: usize, checked: bool) -> Self {
        Self {
            is_global: true,
            ..Self::mutable(slice, rows, cols, checked)
        }
    }

    fn load(&self, ctx: &ExecContext<'_>, col: usize, back: usize) -> Val {
        if self.is_global {
            debug_assert_eq!(back, 0);
            return self.get_at(0, col);
        }

        if back > ctx.cycle {
            return Val::ZERO;
        }
        self.get_at(ctx.cycle - back, col)
    }

    fn store(&self, ctx: &ExecContext<'_>, col: usize, val: Val) {
        let row = if self.is_global { 0 } else { ctx.cycle };
        self.set_at(row, col, val);
    }

    fn set_at(&self, row: usize, col: usize, val: Val) {
        let idx = col * self.rows + row;
        debug_assert!(col < self.cols);
        debug_assert!(row < self.rows);

        // SAFETY: BufferRow is scoped to a HAL buffer view and generated witness code
        // accesses column-major cells within the declared layout.
        let elem = unsafe { &mut *self.buf.add(idx) };
        if elem.is_valid() && *elem != val {
            panic!(
                "inconsistent set at row {row}, col {col}: new={} current={}",
                val.as_u32(),
                elem.as_u32()
            );
        }
        *elem = val;
    }

    fn get_at(&self, row: usize, col: usize) -> Val {
        let idx = col * self.rows + row;
        debug_assert!(col < self.cols);
        debug_assert!(row < self.rows);

        // SAFETY: Bounds are checked above and the buffer view outlives BufferRow.
        let val = unsafe { *self.buf.add(idx) };
        if self.checked && !val.is_valid() {
            panic!("read of unset value at row {row}, col {col}");
        }
        val
    }
}

pub(crate) struct BoundLayout<'a, C, T> {
    layout: &'a C,
    buffer: BufferRow<T>,
}

impl<'a, C, T> Clone for BoundLayout<'a, C, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, C, T> Copy for BoundLayout<'a, C, T> {}

impl<'a, C, T> BoundLayout<'a, C, T> {
    fn new(layout: &'a C, buffer: BufferRow<T>) -> Self {
        Self { layout, buffer }
    }

    fn map<D>(&self, f: impl FnOnce(&'a C) -> &'a D) -> BoundLayout<'a, D, T> {
        BoundLayout {
            layout: f(self.layout),
            buffer: self.buffer,
        }
    }
}

impl BoundLayout<'_, Reg, Val> {
    fn load(&self, ctx: &ExecContext<'_>, back: usize) -> Val {
        self.buffer.load(ctx, self.layout.offset, back)
    }

    fn store(&self, ctx: &ExecContext<'_>, val: Val) {
        self.buffer.store(ctx, self.layout.offset, val);
    }
}

macro_rules! bind_layout {
    ($layout:expr, $buffer:expr) => {
        BoundLayout::new($layout, $buffer)
    };
}

macro_rules! eqz {
    ($val:expr, $loc:expr) => {
        eqz($val, $loc)?
    };
}

macro_rules! invoke_extern {
    ($ctx:expr, is_first_cycle) => {
        $ctx.is_first_cycle()
    };
    ($ctx:expr, get_preimage, $idx:expr) => {
        $ctx.get_preimage($idx)?
    };
    ($ctx:expr, next_preimage) => {
        $ctx.next_preimage()
    };
    ($ctx:expr, log, $message:expr, [$($vals:expr),* $(,)?]) => {{
        let _ = $message;
        $(let _ = $vals;)*
    }};
}

pub(crate) struct ExecContext<'a> {
    preimages: &'a [KeccakState],
    preimage_idxs: &'a [u32],
    cycle: usize,
}

impl<'a> ExecContext<'a> {
    fn new(preimages: &'a [KeccakState], preimage_idxs: &'a [u32], cycle: usize) -> Self {
        Self {
            preimages,
            preimage_idxs,
            cycle,
        }
    }

    fn is_first_cycle(&self) -> Val {
        if self.cycle == 0 {
            Val::ONE
        } else {
            Val::ZERO
        }
    }

    fn get_cycle(&self) -> Val {
        Val::new(self.cycle as u32)
    }

    fn get_preimage(&self, idx: Val) -> Result<Val> {
        let idx = idx.as_u32();
        let idx_low = idx % 4;
        let idx_high = idx / 4;
        ensure!(idx_high < 25, "keccak preimage index out of range: {idx}");

        let preimage_idx = self.preimage_idxs[self.cycle] as usize;
        let preimage = self
            .preimages
            .get(preimage_idx)
            .ok_or_else(|| anyhow::anyhow!("keccak preimage {preimage_idx} out of range"))?;
        Ok(Val::new(
            ((preimage[idx_high as usize] >> (16 * idx_low)) & 0xffff) as u32,
        ))
    }

    fn next_preimage(&self) -> Val {
        if self.preimage_idxs[self.cycle] as usize != self.preimages.len() {
            Val::ONE
        } else {
            Val::ZERO
        }
    }
}

trait EqZero {
    fn ensure_zero(self, loc: &'static str) -> Result<()>;
}

impl EqZero for Val {
    fn ensure_zero(self, loc: &'static str) -> Result<()> {
        ensure!(self == Val::ZERO, "eqz failure at: {loc}");
        Ok(())
    }
}

fn eqz<T: EqZero>(val: T, loc: &'static str) -> Result<()> {
    val.ensure_zero(loc)
}

fn inv_0(val: Val) -> Val {
    val.inv()
}

fn isz(val: Val) -> Val {
    if val == Val::ZERO {
        Val::ONE
    } else {
        Val::ZERO
    }
}

fn bit_and(lhs: Val, rhs: Val) -> Val {
    Val::new(lhs.as_u32() & rhs.as_u32())
}

fn is_true(val: Val) -> bool {
    val != Val::ZERO
}

fn to_usize(val: Val) -> usize {
    val.as_u32() as usize
}

fn val(value: u32) -> Val {
    Val::new(value)
}

fn get(ctx: &ExecContext<'_>, buf: BufferRow<Val>, offset: usize, back: usize) -> Val {
    buf.load(ctx, offset, back)
}

fn set(ctx: &ExecContext<'_>, buf: BufferRow<Val>, offset: usize, value: Val) {
    buf.store(ctx, offset, value);
}

fn get_global(ctx: &ExecContext<'_>, buf: BufferRow<Val>, offset: usize) -> Val {
    buf.load(ctx, offset, 0)
}

fn set_global(ctx: &ExecContext<'_>, buf: BufferRow<Val>, offset: usize, value: Val) {
    buf.store(ctx, offset, value);
}

fn map_layout<A, L: 'static, R, F, const N: usize>(
    values: [A; N],
    layout: BoundLayout<'_, [&'static L; N], Val>,
    mut f: F,
) -> Result<[R; N]>
where
    F: FnMut(A, BoundLayout<'static, L, Val>) -> Result<R>,
{
    let mut out = Vec::with_capacity(N);
    for (idx, value) in values.into_iter().enumerate() {
        out.push(f(
            value,
            BoundLayout::new(layout.layout[idx], layout.buffer),
        )?);
    }
    match out.try_into() {
        Ok(out) => Ok(out),
        Err(_) => unreachable!("array length is fixed"),
    }
}

pub(crate) fn generate_witness<H, O>(
    mode: StepMode,
    preflight: &PreflightTrace<O>,
    global: &MetaBuffer<H>,
    data: &MetaBuffer<H>,
) -> Result<()>
where
    O: PreflightCycleOrder,
    H: risc0_zkp::hal::Hal<Field = CircuitField, Elem = Val, ExtElem = ExtVal>,
{
    let mut result = Ok(());
    data.buf.view_mut(|data_view| {
        global.buf.view_mut(|global_view| {
            let data = BufferRow::mutable(data_view, data.rows, data.cols, data.checked_reads);
            let global =
                BufferRow::global(global_view, global.rows, global.cols, global.checked_reads);
            result = run_witness_steps(mode, preflight, data, global);
        });
    });
    result
}

fn run_witness_steps<O>(
    mode: StepMode,
    preflight: &PreflightTrace<O>,
    data: BufferRow<Val>,
    global: BufferRow<Val>,
) -> Result<()>
where
    O: PreflightCycleOrder,
{
    let preimage_idxs: Vec<u32> = preflight
        .cycles
        .values()
        .flatten()
        .map(|cycle| cycle.preimage_idx)
        .collect();
    ensure!(
        preimage_idxs.len() == preflight.cycle,
        "keccak preflight cycle count mismatch"
    );

    match mode {
        StepMode::Parallel => {
            // C reference (keccak-sys ffi.cpp) runs stepExec under
            // poolstl::par with no split or leadership — keccak witgen
            // cycles are fully independent. Capture the concrete slices so
            // the generic preflight order type stays off the closure.
            let preimages = preflight.preimages.as_slice();
            let preimage_idxs = preimage_idxs.as_slice();
            (0..preflight.cycle)
                .into_par_iter()
                .try_for_each(|cycle| step_exec(preimages, preimage_idxs, cycle, data, global))?;
        }
        StepMode::SeqForward => {
            for cycle in 0..preflight.cycle {
                step_exec(
                    preflight.preimages.as_slice(),
                    preimage_idxs.as_slice(),
                    cycle,
                    data,
                    global,
                )?;
            }
        }
        StepMode::SeqReverse => {
            for cycle in (0..preflight.cycle).rev() {
                step_exec(
                    preflight.preimages.as_slice(),
                    preimage_idxs.as_slice(),
                    cycle,
                    data,
                    global,
                )?;
            }
        }
    }
    Ok(())
}

fn step_exec(
    preimages: &[KeccakState],
    preimage_idxs: &[u32],
    cycle: usize,
    data: BufferRow<Val>,
    global: BufferRow<Val>,
) -> Result<()> {
    let ctx = ExecContext::new(preimages, preimage_idxs, cycle);
    step_top(&ctx, data, global)
}

include!("rust_steps_generated.rs.inc");
