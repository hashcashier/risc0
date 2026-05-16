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
    cell::Cell,
    marker::PhantomData,
    sync::atomic::{AtomicBool, AtomicU16, Ordering},
};

use anyhow::{bail, ensure, Result};
use risc0_core::field::{
    baby_bear::{BabyBearElem, BabyBearExtElem},
    Elem as _, ExtElem as _,
};
use risc0_zkp::{hal::Buffer, layout::Reg};

use super::{MetaBuffer, StepMode};
use crate::{prove::witgen::preflight::PreflightTrace, zirgen::circuit::*};

type Index = usize;

pub(crate) struct BufferRow<T> {
    buf: *mut T,
    rows: usize,
    cols: usize,
    checked: bool,
    is_global: bool,
    zero_back_after: Option<usize>,
    marker: PhantomData<T>,
}

impl<T> Clone for BufferRow<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for BufferRow<T> {}

impl BufferRow<Val> {
    fn mutable(slice: &mut [Val], rows: usize, cols: usize, checked: bool) -> Self {
        Self {
            buf: slice.as_mut_ptr(),
            rows,
            cols,
            checked,
            is_global: false,
            zero_back_after: None,
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
            zero_back_after: None,
            marker: PhantomData,
        }
    }

    fn global(slice: &[Val], rows: usize, cols: usize, checked: bool) -> Self {
        Self {
            is_global: true,
            ..Self::immutable(slice, rows, cols, checked)
        }
    }

    fn with_zero_back_after(mut self, col: usize) -> Self {
        self.zero_back_after = Some(col);
        self
    }

    fn unchecked(mut self) -> Self {
        self.checked = false;
        self
    }

    fn load(&self, ctx: &ExecContext<'_>, col: usize, back: usize, unchecked: bool) -> Val {
        if self.is_global {
            debug_assert_eq!(back, 0);
            return self.get_at(0, col, unchecked);
        }

        if self
            .zero_back_after
            .is_some_and(|zero_back| col > zero_back && back > 0)
        {
            return Val::ZERO;
        }

        let row = (self.rows + ctx.cycle - back) % self.rows;
        self.get_at(row, col, unchecked)
    }

    fn store(&self, ctx: &ExecContext<'_>, col: usize, val: Val) {
        let row = if self.is_global { 0 } else { ctx.cycle };
        self.set_at(row, col, val);
    }

    fn set_at(&self, row: usize, col: usize, val: Val) {
        let idx = col * self.rows + row;
        debug_assert!(col < self.cols);
        debug_assert!(row < self.rows);

        // SAFETY: BufferRow is only constructed from HAL buffer views for the duration
        // of a witness-generation closure. The witness runtime writes column-major cells.
        let elem = unsafe { &mut *self.buf.add(idx) };
        if self.checked && elem.is_valid() && *elem != val {
            panic!(
                "inconsistent set at row {row}, col {col}: new={} current={}",
                val.as_u32(),
                elem.as_u32()
            );
        }
        *elem = val;
    }

    fn get_at(&self, row: usize, col: usize, unchecked: bool) -> Val {
        let idx = col * self.rows + row;
        debug_assert!(col < self.cols);
        debug_assert!(row < self.rows);

        // SAFETY: Bounds are checked above and buffers outlive BufferRow.
        let val = unsafe { *self.buf.add(idx) };
        if unchecked {
            return val.valid_or_zero();
        }
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
        self.buffer.load(ctx, self.layout.offset, back, false)
    }

    fn load_unchecked(&self, ctx: &ExecContext<'_>, back: usize) -> Val {
        self.buffer.load(ctx, self.layout.offset, back, true)
    }

    fn load_ext<E>(&self, ctx: &ExecContext<'_>, back: usize) -> ExtVal {
        let _ = PhantomData::<E>;
        ExtVal::new(
            self.buffer.load(ctx, self.layout.offset, back, false),
            self.buffer.load(ctx, self.layout.offset + 1, back, false),
            self.buffer.load(ctx, self.layout.offset + 2, back, false),
            self.buffer.load(ctx, self.layout.offset + 3, back, false),
        )
    }

    fn load_unchecked_ext<E>(&self, ctx: &ExecContext<'_>, back: usize) -> ExtVal {
        let _ = PhantomData::<E>;
        ExtVal::new(
            self.buffer.load(ctx, self.layout.offset, back, true),
            self.buffer.load(ctx, self.layout.offset + 1, back, true),
            self.buffer.load(ctx, self.layout.offset + 2, back, true),
            self.buffer.load(ctx, self.layout.offset + 3, back, true),
        )
    }

    fn store(&self, ctx: &ExecContext<'_>, val: Val) {
        self.buffer.store(ctx, self.layout.offset, val);
    }

    fn store_ext(&self, ctx: &ExecContext<'_>, val: ExtVal) {
        for (i, elem) in val.elems().iter().copied().enumerate() {
            self.buffer.store(ctx, self.layout.offset + i, elem);
        }
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
    ($ctx:expr, lookup_delta, $table:expr, $index:expr, $count:expr) => {
        $ctx.lookup_delta($table, $index, $count)?
    };
    ($ctx:expr, lookup_current, $table:expr, $index:expr) => {
        $ctx.lookup_current($table, $index)?
    };
    ($ctx:expr, memory_delta, $addr:expr, $cycle:expr, $data_low:expr, $data_high:expr, $count:expr) => {
        $ctx.memory_delta($addr, $cycle, $data_low, $data_high, $count)
    };
    ($ctx:expr, get_memory_txn, $addr:expr) => {
        $ctx.get_memory_txn($addr)?
    };
    ($ctx:expr, get_diff_count, $cycle:expr) => {
        $ctx.get_diff_count($cycle)
    };
    ($ctx:expr, is_first_cycle_0) => {
        $ctx.is_first_cycle_0()
    };
    ($ctx:expr, divide, $numer_low:expr, $numer_high:expr, $denom_low:expr, $denom_high:expr, $sign_type:expr) => {
        $ctx.divide($numer_low, $numer_high, $denom_low, $denom_high, $sign_type)
    };
    ($ctx:expr, get_major_minor) => {
        $ctx.get_major_minor()
    };
    ($ctx:expr, host_read_prepare, $fp:expr, $len:expr) => {
        $ctx.host_read_prepare($fp, $len)
    };
    ($ctx:expr, host_write, $fd:expr, $addr_low:expr, $addr_high:expr, $len:expr) => {
        $ctx.host_write($fd, $addr_low, $addr_high, $len)
    };
    ($ctx:expr, next_paging_idx) => {
        $ctx.next_paging_idx()
    };
    ($ctx:expr, big_int_extern) => {
        $ctx.big_int_extern()
    };
    ($ctx:expr, log, $message:expr, [$($vals:expr),* $(,)?]) => {{
        let _ = $message;
        $(let _ = $vals;)*
    }};
    ($ctx:expr, assert, $cond:expr, $message:expr) => {{
        let _ = ($ctx, $cond, $message);
    }};
    ($ctx:expr, print, $val:expr) => {{
        let _ = ($ctx, $val);
    }};
}

pub(crate) struct ExecContext<'a> {
    preflight: &'a PreflightTrace,
    tables: &'a LookupTables,
    cycle: usize,
    txn_idx: Cell<usize>,
}

impl<'a> ExecContext<'a> {
    fn new(preflight: &'a PreflightTrace, tables: &'a LookupTables, cycle: usize) -> Self {
        Self {
            preflight,
            tables,
            cycle,
            txn_idx: Cell::new(preflight.cycles[cycle].txn_idx as usize),
        }
    }

    fn get_memory_txn(&self, addr_elem: Val) -> Result<(Val, Val, Val, Val, Val)> {
        let txn_idx = self.txn_idx.get();
        self.txn_idx.set(txn_idx + 1);

        let txn =
            self.preflight.txns.get(txn_idx).ok_or_else(|| {
                anyhow::anyhow!("memory transaction index out of range: {txn_idx}")
            })?;
        ensure!(
            txn.cycle / 2 == self.cycle as u32,
            "txn cycle mismatch: txn {}, ctx {}",
            txn.cycle,
            self.cycle
        );
        ensure!(
            txn.addr == addr_elem.as_u32(),
            "memory peek not in preflight: txn addr {:#010x}, requested {:#010x}",
            txn.addr,
            addr_elem.as_u32()
        );
        Ok((
            Val::new(txn.prev_cycle),
            Val::new(txn.prev_word & 0xffff),
            Val::new(txn.prev_word >> 16),
            Val::new(txn.word & 0xffff),
            Val::new(txn.word >> 16),
        ))
    }

    fn lookup_delta(&self, table: Val, index: Val, _count: Val) -> Result<()> {
        self.tables.lookup_delta(self.cycle, table, index)
    }

    fn lookup_current(&self, table: Val, index: Val) -> Result<Val> {
        self.tables.lookup_current(table, index)
    }

    fn memory_delta(&self, _addr: Val, _cycle: Val, _data_low: Val, _data_high: Val, _count: Val) {}

    fn get_diff_count(&self, cycle: Val) -> Val {
        let cycle = cycle.as_u32();
        Val::new(self.preflight.cycles[(cycle / 2) as usize].diff_count[(cycle % 2) as usize])
    }

    fn is_first_cycle_0(&self) -> Val {
        if self.cycle == 0 {
            Val::ONE
        } else {
            Val::ZERO
        }
    }

    fn divide(
        &self,
        numer_low: Val,
        numer_high: Val,
        denom_low: Val,
        denom_high: Val,
        sign_type: Val,
    ) -> (Val, Val, Val, Val) {
        let numer = numer_low.as_u32() | (numer_high.as_u32() << 16);
        let denom = denom_low.as_u32() | (denom_high.as_u32() << 16);
        let (quot, rem) = divide_rv32im(numer, denom, sign_type.as_u32());
        (
            Val::new(quot & 0xffff),
            Val::new(quot >> 16),
            Val::new(rem & 0xffff),
            Val::new(rem >> 16),
        )
    }

    fn get_major_minor(&self) -> (Val, Val) {
        let cycle = &self.preflight.cycles[self.cycle];
        (Val::new(cycle.major as u32), Val::new(cycle.minor as u32))
    }

    fn host_read_prepare(&self, _fp: Val, _len: Val) -> Val {
        let txn_idx = self.txn_idx.get();
        Val::new(self.preflight.txns[txn_idx].word)
    }

    fn host_write(&self, _fd: Val, _addr_low: Val, _addr_high: Val, _len: Val) -> Val {
        let txn_idx = self.txn_idx.get();
        Val::new(self.preflight.txns[txn_idx].word)
    }

    fn next_paging_idx(&self) -> (Val, Val) {
        let cycle = &self.preflight.cycles[self.cycle];
        (
            Val::new(cycle.paging_idx),
            Val::new(cycle.machine_mode as u32),
        )
    }

    fn big_int_extern(
        &self,
    ) -> (
        Val,
        Val,
        Val,
        Val,
        Val,
        Val,
        Val,
        Val,
        Val,
        Val,
        Val,
        Val,
        Val,
        Val,
        Val,
        Val,
    ) {
        let idx = self.preflight.cycles[self.cycle].bigint_idx as usize;
        let bytes = &self.preflight.bigint_bytes[idx..idx + 16];
        (
            Val::new(bytes[0] as u32),
            Val::new(bytes[1] as u32),
            Val::new(bytes[2] as u32),
            Val::new(bytes[3] as u32),
            Val::new(bytes[4] as u32),
            Val::new(bytes[5] as u32),
            Val::new(bytes[6] as u32),
            Val::new(bytes[7] as u32),
            Val::new(bytes[8] as u32),
            Val::new(bytes[9] as u32),
            Val::new(bytes[10] as u32),
            Val::new(bytes[11] as u32),
            Val::new(bytes[12] as u32),
            Val::new(bytes[13] as u32),
            Val::new(bytes[14] as u32),
            Val::new(bytes[15] as u32),
        )
    }
}

pub(crate) struct LookupTables {
    table_u8: Vec<Cell<u32>>,
    table_u16: Vec<Cell<u32>>,
}

impl Default for LookupTables {
    fn default() -> Self {
        Self {
            table_u8: (0..(1 << 8)).map(|_| Cell::new(0)).collect(),
            table_u16: (0..(1 << 16)).map(|_| Cell::new(0)).collect(),
        }
    }
}

impl LookupTables {
    fn lookup_delta(&self, cycle: usize, table: Val, index: Val) -> Result<()> {
        let table = table.as_u32();
        let index = index.as_u32() as usize;
        if table == 0 {
            return Ok(());
        }
        ensure!(table == 8 || table == 16, "invalid lookup table: {table}");
        ensure!(
            index < (1usize << table),
            "[{cycle}]: lookup index out of range: table {table}, index {index}"
        );
        let cell = if table == 8 {
            &self.table_u8[index]
        } else {
            &self.table_u16[index]
        };
        cell.set(cell.get() + 1);
        Ok(())
    }

    fn lookup_current(&self, table: Val, index: Val) -> Result<Val> {
        let table = table.as_u32();
        let index = index.as_u32() as usize;
        ensure!(table == 8 || table == 16, "invalid lookup table: {table}");
        Ok(Val::new(if table == 8 {
            self.table_u8[index].get()
        } else {
            self.table_u16[index].get()
        }))
    }
}

trait EqZero {
    fn ensure_zero(self, loc: &'static str) -> Result<()>;
}

impl EqZero for BabyBearElem {
    fn ensure_zero(self, loc: &'static str) -> Result<()> {
        ensure!(self == Val::ZERO, "eqz failure at: {loc}");
        Ok(())
    }
}

impl EqZero for BabyBearExtElem {
    fn ensure_zero(self, loc: &'static str) -> Result<()> {
        for elem in self.elems().iter().copied() {
            elem.ensure_zero(loc)?;
        }
        Ok(())
    }
}

fn eqz<T: EqZero>(val: T, loc: &'static str) -> Result<()> {
    val.ensure_zero(loc)
}

trait Inv {
    fn inv_checked(self) -> Self
    where
        Self: Sized;
}

impl Inv for BabyBearElem {
    fn inv_checked(self) -> Self {
        self.inv()
    }
}

impl Inv for BabyBearExtElem {
    fn inv_checked(self) -> Self {
        self.inv()
    }
}

fn inv_0<T: Inv>(val: T) -> T {
    val.inv_checked()
}

fn isz(val: Val) -> Val {
    if val == Val::ZERO {
        Val::ONE
    } else {
        Val::ZERO
    }
}

fn neg_0(val: Val) -> Val {
    -val
}

fn bit_and(lhs: Val, rhs: Val) -> Val {
    Val::new(lhs.as_u32() & rhs.as_u32())
}

fn in_range(low: Val, mid: Val, high: Val) -> Val {
    assert!(low.as_u32() <= high.as_u32(), "invalid range");
    if low.as_u32() <= mid.as_u32() && mid.as_u32() < high.as_u32() {
        Val::ONE
    } else {
        Val::ZERO
    }
}

fn mod_0(lhs: Val, rhs: Val) -> Val {
    Val::new(lhs.as_u32() % rhs.as_u32())
}

fn is_true(val: Val) -> bool {
    val != Val::ZERO
}

fn to_usize(val: BabyBearElem) -> usize {
    val.as_u32() as usize
}

fn map_array<A, R, F, const N: usize>(values: [A; N], mut f: F) -> Result<[R; N]>
where
    F: FnMut(A) -> Result<R>,
{
    let mut out = Vec::with_capacity(N);
    for value in values {
        out.push(f(value)?);
    }
    match out.try_into() {
        Ok(out) => Ok(out),
        Err(_) => unreachable!("array length is fixed"),
    }
}

fn map_array2<A, B, R, F, const N: usize>(lhs: [A; N], rhs: [B; N], mut f: F) -> Result<[R; N]>
where
    F: FnMut(A, B) -> Result<R>,
{
    let mut out = Vec::with_capacity(N);
    for (lhs, rhs) in lhs.into_iter().zip(rhs) {
        out.push(f(lhs, rhs)?);
    }
    match out.try_into() {
        Ok(out) => Ok(out),
        Err(_) => unreachable!("array length is fixed"),
    }
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

fn reduce_array<A, R, F, const N: usize>(values: [A; N], start: R, mut f: F) -> Result<R>
where
    F: FnMut(R, A) -> Result<R>,
{
    let mut cur = start;
    for value in values {
        cur = f(cur, value)?;
    }
    Ok(cur)
}

fn reduce_layout<A, L: 'static, R, F, const N: usize>(
    values: [A; N],
    start: R,
    layout: BoundLayout<'_, [&'static L; N], Val>,
    mut f: F,
) -> Result<R>
where
    F: FnMut(R, A, BoundLayout<'static, L, Val>) -> Result<R>,
{
    let mut cur = start;
    for (idx, value) in values.into_iter().enumerate() {
        cur = f(
            cur,
            value,
            BoundLayout::new(layout.layout[idx], layout.buffer),
        )?;
    }
    Ok(cur)
}

fn divide_rv32im(mut numer: u32, mut denom: u32, sign_type: u32) -> (u32, u32) {
    let ones_comp = u32::from(sign_type == 2);
    let neg_numer = sign_type != 0 && (numer as i32) < 0;
    let neg_denom = sign_type == 1 && (denom as i32) < 0;
    if neg_numer {
        numer = numer.wrapping_neg().wrapping_sub(ones_comp);
    }
    if neg_denom {
        denom = denom.wrapping_neg().wrapping_sub(ones_comp);
    }

    let (mut quot, mut rem) = if denom == 0 {
        (0xffff_ffff, numer)
    } else {
        (numer / denom, numer % denom)
    };

    let quot_neg_out = u32::from(neg_numer ^ neg_denom) - u32::from(denom == 0 && neg_numer);
    let rem_neg_out = neg_numer;
    if quot_neg_out != 0 {
        quot = quot.wrapping_neg().wrapping_sub(ones_comp);
    }
    if rem_neg_out {
        rem = rem.wrapping_neg().wrapping_sub(ones_comp);
    }
    (quot, rem)
}

pub(crate) fn generate_witness<H>(
    mode: StepMode,
    preflight: &PreflightTrace,
    global: &MetaBuffer<H>,
    data: &MetaBuffer<H>,
) -> Result<()>
where
    H: risc0_zkp::hal::Hal<Field = CircuitField, Elem = Val, ExtElem = ExtVal>,
{
    let mut result = Ok(());
    data.buf.view_mut(|data_view| {
        global.buf.view(|global_view| {
            let data = BufferRow::mutable(data_view, data.rows, data.cols, data.checked);
            let global = BufferRow::global(global_view, global.rows, global.cols, global.checked);
            result = run_witness_steps(mode, preflight, data, global);
        });
    });
    result
}

pub(crate) fn step_accum<H>(
    preflight: &PreflightTrace,
    data: &MetaBuffer<H>,
    accum: &MetaBuffer<H>,
    global: &MetaBuffer<H>,
    mix: &MetaBuffer<H>,
) -> Result<()>
where
    H: risc0_zkp::hal::Hal<Field = CircuitField, Elem = Val, ExtElem = ExtVal>,
{
    let mut result = Ok(());
    accum.buf.view_mut(|accum_view| {
        data.buf.view(|data_view| {
            global.buf.view(|global_view| {
                mix.buf.view(|mix_view| {
                    let data = BufferRow::immutable(data_view, data.rows, data.cols, data.checked);
                    let split = LAYOUT_TOP_ACCUM.columns[0].offset;
                    let accum =
                        BufferRow::mutable(accum_view, accum.rows, accum.cols, accum.checked)
                            .with_zero_back_after(split);
                    let global =
                        BufferRow::global(global_view, global.rows, global.cols, global.checked);
                    let mix = BufferRow::global(mix_view, mix.rows, mix.cols, mix.checked);
                    result = run_accum_steps(preflight, data, accum, global, mix);
                });
            });
        });
    });
    result
}

fn run_witness_steps(
    mode: StepMode,
    preflight: &PreflightTrace,
    data: BufferRow<Val>,
    global: BufferRow<Val>,
) -> Result<()> {
    let tables = LookupTables::default();
    let split = preflight.table_split_cycle as usize;
    let last_cycle = preflight.cycles.len();

    match mode {
        StepMode::Parallel | StepMode::SeqForward => {
            for cycle in 0..split {
                step_exec(preflight, &tables, cycle, data, global)?;
            }
            for cycle in split..last_cycle {
                step_exec(preflight, &tables, cycle, data, global)?;
            }
        }
        StepMode::SeqReverse => {
            for cycle in (0..split).rev() {
                step_exec(preflight, &tables, cycle, data, global)?;
            }
            for cycle in (split..last_cycle).rev() {
                step_exec(preflight, &tables, cycle, data, global)?;
            }
        }
    }
    Ok(())
}

fn run_accum_steps(
    preflight: &PreflightTrace,
    data: BufferRow<Val>,
    accum: BufferRow<Val>,
    global: BufferRow<Val>,
    mix: BufferRow<Val>,
) -> Result<()> {
    let tables = LookupTables::default();
    let last_cycle = preflight.cycles.len();

    for cycle in 0..last_cycle {
        let ctx = ExecContext::new(preflight, &tables, cycle);
        step_TopAccum(&ctx, accum, data, global, mix)?;
    }

    let accum = accum.unchecked();

    for elem_idx in 0..ExtVal::EXT_SIZE {
        let col = accum.cols - ExtVal::EXT_SIZE + elem_idx;
        let mut running = Val::ZERO;
        for row in 0..last_cycle {
            let cur = accum.get_at(row, col, true);
            running += cur;
            accum.set_at(row, col, running);
        }
    }

    let split = LAYOUT_TOP_ACCUM.columns[0].offset;
    let machine_columns = (accum.cols - split) / ExtVal::EXT_SIZE;
    for row in 0..last_cycle {
        let back = (row + last_cycle - 1) % last_cycle;
        let prev: [Val; ExtVal::EXT_SIZE] =
            std::array::from_fn(|i| accum.get_at(back, accum.cols - ExtVal::EXT_SIZE + i, true));
        for j in 0..machine_columns - 1 {
            for (k, prev) in prev.iter().copied().enumerate() {
                let col = split + j * ExtVal::EXT_SIZE + k;
                let cur = accum.get_at(row, col, true);
                accum.set_at(row, col, cur + prev);
            }
        }
    }

    Ok(())
}

// SP7 iter-6d-g step 6.2.3: per-segment arm mask. Bit k set => major
// opcode k's cycles short-circuit step_Top. WebGPU HAL sets this before
// `generate_witness` runs based on which per-arm GPU kernels actually
// dispatched this segment (kernel ready + cycles > 0). CPU HAL leaves
// it 0 (no short-circuit ever).
static WITGEN_GPU_REPLACE_ARM_MASK: AtomicU16 = AtomicU16::new(0);
// Legacy process-wide gate kept for the public setter so callers can
// flip the feature on/off; webgpu.rs reads this to decide whether to
// populate WITGEN_GPU_REPLACE_ARM_MASK each segment.
static WITGEN_GPU_REPLACE_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_witgen_gpu_replace_enabled(enabled: bool) {
    WITGEN_GPU_REPLACE_ENABLED.store(enabled, Ordering::Release);
    if !enabled {
        WITGEN_GPU_REPLACE_ARM_MASK.store(0, Ordering::Release);
    }
}

pub fn set_witgen_gpu_replace_arm_mask(mask: u16) {
    WITGEN_GPU_REPLACE_ARM_MASK.store(mask, Ordering::Release);
}

fn cycle_short_circuited(major: u8) -> bool {
    if major >= 13 {
        return false;
    }
    let mask = WITGEN_GPU_REPLACE_ARM_MASK.load(Ordering::Acquire);
    (mask & (1u16 << major)) != 0
}

fn step_exec(
    preflight: &PreflightTrace,
    tables: &LookupTables,
    cycle: usize,
    data: BufferRow<Val>,
    global: BufferRow<Val>,
) -> Result<()> {
    let major = preflight.cycles[cycle].major;
    if cycle_short_circuited(major) {
        return Ok(());
    }
    let ctx = ExecContext::new(preflight, tables, cycle);
    step_Top(&ctx, data, global)
}

include!("../../zirgen/steps.rs.inc");
