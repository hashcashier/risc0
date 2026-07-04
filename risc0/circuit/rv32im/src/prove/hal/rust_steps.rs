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
    sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicUsize, Ordering},
};

use rayon::prelude::*;

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

// SAFETY: BufferRow is a raw column-major view handed to the witness steppers
// for the duration of one witness-generation closure. The parallel stepper
// protocol is identical to the C reference (rv32im-sys ffi.cpp captures the
// same raw buffers across poolstl::par threads): each cycle's step writes
// only its own row, and cross-row `back` loads are confined by the protocol's
// phase structure (table split barrier in witgen; separate prefix pass in
// accum). `checked` set_at conflicts panic, which rayon propagates at join.
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
        if !eqz_elided() {
            eqz($val, $loc)?
        }
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
    // Relaxed atomics, mirroring the C reference's
    // `std::vector<std::atomic_uint32_t>` (tables.h): lookup counts are pure
    // associative increments accumulated across parallel cycle steps.
    table_u8: Vec<AtomicU32>,
    table_u16: Vec<AtomicU32>,
}

impl Default for LookupTables {
    fn default() -> Self {
        Self {
            table_u8: (0..(1 << 8)).map(|_| AtomicU32::new(0)).collect(),
            table_u16: (0..(1 << 16)).map(|_| AtomicU32::new(0)).collect(),
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
        cell.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn lookup_current(&self, table: Val, index: Val) -> Result<Val> {
        let table = table.as_u32();
        let index = index.as_u32() as usize;
        ensure!(table == 8 || table == 16, "invalid lookup table: {table}");
        Ok(Val::new(if table == 8 {
            self.table_u8[index].load(Ordering::Relaxed)
        } else {
            self.table_u16[index].load(Ordering::Relaxed)
        }))
    }
}

fn replay_arg_u16_lookup_delta(
    cycle: usize,
    tables: &LookupTables,
    data: BufferRow<Val>,
    arg: &ArgU16Layout,
) -> Result<()> {
    let index = data.get_at(cycle, arg.val._super.offset, false);
    tables.lookup_delta(cycle, Val::new(16), index)
}

fn replay_nondet_u16_lookup_delta(
    cycle: usize,
    tables: &LookupTables,
    data: BufferRow<Val>,
    layout: &NondetU16RegLayout,
) -> Result<()> {
    replay_arg_u16_lookup_delta(cycle, tables, data, layout.arg)
}

fn replay_normalize_u32_lookup_deltas(
    cycle: usize,
    tables: &LookupTables,
    data: BufferRow<Val>,
    layout: &NormalizeU32Layout,
) -> Result<()> {
    replay_nondet_u16_lookup_delta(cycle, tables, data, layout.low16)?;
    replay_nondet_u16_lookup_delta(cycle, tables, data, layout.high16)?;
    Ok(())
}

fn replay_get_sign_u32_lookup_deltas(
    cycle: usize,
    tables: &LookupTables,
    data: BufferRow<Val>,
    layout: &GetSignU32Layout,
) -> Result<()> {
    replay_nondet_u16_lookup_delta(cycle, tables, data, layout.rest_times_two)
}

fn replay_cmp_less_than_lookup_deltas(
    cycle: usize,
    tables: &LookupTables,
    data: BufferRow<Val>,
    layout: &CmpLessThanLayout,
) -> Result<()> {
    replay_normalize_u32_lookup_deltas(cycle, tables, data, layout.diff)?;
    replay_get_sign_u32_lookup_deltas(cycle, tables, data, layout.s1)?;
    replay_get_sign_u32_lookup_deltas(cycle, tables, data, layout.s2)?;
    replay_get_sign_u32_lookup_deltas(cycle, tables, data, layout.s3)?;
    Ok(())
}

fn replay_cmp_less_than_unsigned_lookup_deltas(
    cycle: usize,
    tables: &LookupTables,
    data: BufferRow<Val>,
    layout: &CmpLessThanUnsignedLayout,
) -> Result<()> {
    replay_normalize_u32_lookup_deltas(cycle, tables, data, layout.diff)
}

fn replay_u16_lookup_delta(cycle: usize, tables: &LookupTables, value: u32) -> Result<()> {
    ensure!(
        value < (1 << 16),
        "[{cycle}]: direct lookup replay value out of u16 range: {value}"
    );
    tables.lookup_delta(cycle, Val::new(16), Val::new(value))
}

fn replay_u8_lookup_delta(cycle: usize, tables: &LookupTables, value: u32) -> Result<()> {
    ensure!(
        value < (1 << 8),
        "[{cycle}]: direct lookup replay value out of u8 range: {value}"
    );
    tables.lookup_delta(cycle, Val::new(8), Val::new(value))
}

fn get_cycle_txn<'a>(
    preflight: &'a PreflightTrace,
    cycle: usize,
    txn_cursor: &mut usize,
) -> Result<&'a risc0_circuit_rv32im_sys::RawMemoryTransaction> {
    let txn = preflight
        .txns
        .get(*txn_cursor)
        .ok_or_else(|| anyhow::anyhow!("[{cycle}]: memory transaction index out of range"))?;
    ensure!(
        txn.cycle / 2 == cycle as u32,
        "[{cycle}]: memory transaction cycle mismatch: txn_cycle={}",
        txn.cycle
    );
    *txn_cursor += 1;
    Ok(txn)
}

fn current_pc_and_machine_mode(preflight: &PreflightTrace, cycle: usize) -> (u32, u8) {
    if cycle == 0 {
        (0, 1)
    } else {
        let prev = &preflight.cycles[cycle - 1];
        (prev.pc, prev.machine_mode)
    }
}

fn get_cycle_inst_and_sources(preflight: &PreflightTrace, cycle: usize) -> Result<(u32, u32, u32)> {
    let preflight_cycle = &preflight.cycles[cycle];
    let (pc, _) = current_pc_and_machine_mode(preflight, cycle);
    let mut txn_cursor = preflight_cycle.txn_idx as usize;
    let inst_txn = get_cycle_txn(preflight, cycle, &mut txn_cursor)?;
    ensure!(
        inst_txn.addr == pc / 4,
        "[{cycle}]: direct replay instruction txn addr mismatch: got={:#010x} expected={:#010x}",
        inst_txn.addr,
        pc / 4
    );
    let inst = inst_txn.word;
    let rs1 = (inst >> 15) & 0x1f;
    let rs2 = (inst >> 20) & 0x1f;
    let rs1_word = get_cycle_txn(preflight, cycle, &mut txn_cursor)?.word;
    let rs2_word = if rs1 == rs2 {
        rs1_word
    } else {
        get_cycle_txn(preflight, cycle, &mut txn_cursor)?.word
    };
    Ok((inst, rs1_word, rs2_word))
}

fn get_cycle_inst_rs1_and_load_word(
    preflight: &PreflightTrace,
    cycle: usize,
) -> Result<(u32, u32, u32)> {
    let preflight_cycle = &preflight.cycles[cycle];
    let (pc, _) = current_pc_and_machine_mode(preflight, cycle);
    let mut txn_cursor = preflight_cycle.txn_idx as usize;
    let inst_txn = get_cycle_txn(preflight, cycle, &mut txn_cursor)?;
    ensure!(
        inst_txn.addr == pc / 4,
        "[{cycle}]: MEM0 direct replay instruction txn addr mismatch: got={:#010x} expected={:#010x}",
        inst_txn.addr,
        pc / 4
    );
    let inst = inst_txn.word;
    let rs1_word = get_cycle_txn(preflight, cycle, &mut txn_cursor)?.word;
    let imm_i = ((inst as i32) >> 20) as u32;
    let load_addr = rs1_word.wrapping_add(imm_i);
    let load_txn = get_cycle_txn(preflight, cycle, &mut txn_cursor)?;
    ensure!(
        load_txn.addr == load_addr / 4,
        "[{cycle}]: MEM0 direct replay load txn addr mismatch: got={:#010x} expected={:#010x}",
        load_txn.addr,
        load_addr / 4
    );
    Ok((inst, rs1_word, load_txn.word))
}

fn get_cycle_store_context(
    preflight: &PreflightTrace,
    cycle: usize,
    minor: u8,
) -> Result<(u32, u32, u32, u32, u32, u32)> {
    let preflight_cycle = &preflight.cycles[cycle];
    let (pc, _) = current_pc_and_machine_mode(preflight, cycle);
    let mut txn_cursor = preflight_cycle.txn_idx as usize;
    let inst_txn = get_cycle_txn(preflight, cycle, &mut txn_cursor)?;
    ensure!(
        inst_txn.addr == pc / 4,
        "[{cycle}]: MEM1 direct replay instruction txn addr mismatch: got={:#010x} expected={:#010x}",
        inst_txn.addr,
        pc / 4
    );
    let inst = inst_txn.word;
    let rs1 = (inst >> 15) & 0x1f;
    let rs2 = (inst >> 20) & 0x1f;
    let rs1_word = get_cycle_txn(preflight, cycle, &mut txn_cursor)?.word;
    let rs2_word = if rs1 == rs2 {
        rs1_word
    } else {
        get_cycle_txn(preflight, cycle, &mut txn_cursor)?.word
    };
    let imm_s = ((((inst as i32) >> 25) << 5) as u32) | ((inst >> 7) & 0x1f);
    let store_addr = rs1_word.wrapping_add(imm_s);
    let read_txn = get_cycle_txn(preflight, cycle, &mut txn_cursor)?;
    ensure!(
        read_txn.addr == store_addr / 4,
        "[{cycle}]: MEM1 direct replay read txn addr mismatch: got={:#010x} expected={:#010x}",
        read_txn.addr,
        store_addr / 4
    );
    let write_txn = get_cycle_txn(preflight, cycle, &mut txn_cursor)?;
    ensure!(
        write_txn.addr == store_addr / 4,
        "[{cycle}]: MEM1 direct replay write txn addr mismatch: got={:#010x} expected={:#010x}",
        write_txn.addr,
        store_addr / 4
    );
    let expected_write = mem1_store_word(cycle, minor, store_addr, rs2_word, read_txn.word)?;
    ensure!(
        write_txn.word == expected_write,
        "[{cycle}]: MEM1 direct replay write word mismatch: got={:#010x} expected={:#010x}",
        write_txn.word,
        expected_write
    );
    Ok((
        inst,
        rs1_word,
        rs2_word,
        store_addr,
        read_txn.word,
        write_txn.word,
    ))
}

fn upper_base_for_mode(cycle: usize, mode: u8, context: &str) -> Result<u32> {
    match mode {
        0 => Ok(0xbfff),
        1 => Ok(0xffff),
        mode => anyhow::bail!("[{cycle}]: unsupported {context} direct replay machine_mode={mode}"),
    }
}

fn replay_addr_decompose_lookup_deltas(
    cycle: usize,
    tables: &LookupTables,
    addr: u32,
    mode: u8,
    context: &str,
) -> Result<()> {
    let addr_high = addr >> 16;
    let upper_base = upper_base_for_mode(cycle, mode, context)?;
    ensure!(
        addr_high <= upper_base,
        "[{cycle}]: {context} direct replay address high word exceeds mode range: addr_high={addr_high} upper_base={upper_base}"
    );
    replay_u16_lookup_delta(cycle, tables, upper_base - addr_high)?;
    replay_u16_lookup_delta(cycle, tables, (addr & 0xffff) >> 2)
}

fn misc0_simple_replay_values(preflight: &PreflightTrace, cycle: usize) -> Result<[u32; 6]> {
    let preflight_cycle = &preflight.cycles[cycle];
    let (pc, machine_mode) = current_pc_and_machine_mode(preflight, cycle);
    ensure!(
        pc & 3 == 0,
        "[{cycle}]: MISC0 direct replay expected word-aligned pc, got {pc:#010x}"
    );

    let pc_low = pc & 0xffff;
    let pc_high = pc >> 16;
    let upper_base = upper_base_for_mode(cycle, machine_mode, "MISC0")?;
    ensure!(
        pc_high <= upper_base,
        "[{cycle}]: MISC0 direct replay pc high word exceeds mode range: pc_high={pc_high} upper_base={upper_base}"
    );
    let pc_addr_upper_diff = upper_base - pc_high;
    let pc_addr_med14 = pc_low >> 2;

    let (inst, rs1_word, rs2_word) = get_cycle_inst_and_sources(preflight, cycle)?;
    let write_word = match preflight_cycle.minor {
        0 => rs1_word.wrapping_add(rs2_word),
        1 => rs1_word.wrapping_sub(rs2_word),
        2 => rs1_word ^ rs2_word,
        3 => rs1_word | rs2_word,
        4 => rs1_word & rs2_word,
        7 => {
            let imm_i = ((inst as i32) >> 20) as u32;
            rs1_word.wrapping_add(imm_i)
        }
        minor => anyhow::bail!("[{cycle}]: unsupported MISC0 direct replay minor={minor}"),
    };
    let next_pc = pc.wrapping_add(4);

    Ok([
        pc_addr_upper_diff,
        pc_addr_med14,
        write_word & 0xffff,
        write_word >> 16,
        next_pc & 0xffff,
        next_pc >> 16,
    ])
}

fn replay_misc0_simple_lookup_deltas(
    preflight: &PreflightTrace,
    cycle: usize,
    tables: &LookupTables,
) -> Result<()> {
    for value in misc0_simple_replay_values(preflight, cycle)? {
        replay_u16_lookup_delta(cycle, tables, value)?;
    }
    Ok(())
}

fn misc0_simple_short_circuit_minor(minor: u8) -> bool {
    matches!(minor, 0 | 1 | 2 | 3 | 4 | 7)
}

fn misc2_short_circuit_minor(minor: u8) -> bool {
    // Minor 1 still uses the generic chunk1 path whose nested
    // ReadSourceRegs mux is incomplete when rs1 == rs2. Keep it on CPU until
    // it has the same combined source-register treatment as minor 0/2+.
    matches!(minor, 0 | 2 | 3 | 4 | 5 | 6 | 7)
}

fn mem0_short_circuit_minor(minor: u8) -> bool {
    matches!(minor, 0 | 1 | 2 | 3 | 4)
        && (WITGEN_GPU_MEM0_REPLACE_MINOR_MASK.load(Ordering::Acquire) & (1u16 << minor)) != 0
}

fn mem1_short_circuit_minor(minor: u8) -> bool {
    matches!(minor, 0 | 1 | 2)
        && (WITGEN_GPU_MEM1_REPLACE_MINOR_MASK.load(Ordering::Acquire) & (1u16 << minor)) != 0
}

fn replay_normalized_word_lookup_deltas(
    cycle: usize,
    tables: &LookupTables,
    value: u32,
) -> Result<()> {
    replay_u16_lookup_delta(cycle, tables, value & 0xffff)?;
    replay_u16_lookup_delta(cycle, tables, value >> 16)
}

fn normalized_diff_lookup_values(lhs: u32, rhs: u32) -> (u32, u32) {
    let lhs_low = lhs & 0xffff;
    let lhs_high = lhs >> 16;
    let rhs_low = rhs & 0xffff;
    let rhs_high = rhs >> 16;
    let low = lhs_low + 0x1_0000 - rhs_low;
    let low_carry = (low >> 16) & 1;
    let high = lhs_high + 0xffff - rhs_high + low_carry;
    (low & 0xffff, high & 0xffff)
}

fn sign_rest_times_two(value: u32) -> u32 {
    ((value >> 16) & 0x7fff) * 2
}

fn replay_unsigned_cmp_lookup_deltas(
    cycle: usize,
    tables: &LookupTables,
    lhs: u32,
    rhs: u32,
) -> Result<()> {
    let (diff_low, diff_high) = normalized_diff_lookup_values(lhs, rhs);
    replay_u16_lookup_delta(cycle, tables, diff_low)?;
    replay_u16_lookup_delta(cycle, tables, diff_high)
}

fn replay_signed_cmp_lookup_deltas(
    cycle: usize,
    tables: &LookupTables,
    lhs: u32,
    rhs: u32,
) -> Result<()> {
    let (diff_low, diff_high) = normalized_diff_lookup_values(lhs, rhs);
    replay_u16_lookup_delta(cycle, tables, diff_low)?;
    replay_u16_lookup_delta(cycle, tables, diff_high)?;
    replay_u16_lookup_delta(cycle, tables, sign_rest_times_two(lhs))?;
    replay_u16_lookup_delta(cycle, tables, sign_rest_times_two(rhs))?;
    replay_u16_lookup_delta(cycle, tables, (diff_high & 0x7fff) * 2)
}

fn misc2_write_word(preflight: &PreflightTrace, cycle: usize, inst: u32, minor: u8) -> u32 {
    let (pc, _) = current_pc_and_machine_mode(preflight, cycle);
    match minor {
        3 | 4 => pc.wrapping_add(4),
        5 => inst & 0xffff_f000,
        6 => pc.wrapping_add(inst & 0xffff_f000),
        _ => 0,
    }
}

fn replay_misc2_lookup_deltas_from_preflight(
    preflight: &PreflightTrace,
    cycle: usize,
    tables: &LookupTables,
    minor: u8,
) -> Result<()> {
    let (pc, machine_mode) = current_pc_and_machine_mode(preflight, cycle);
    ensure!(
        pc & 3 == 0,
        "[{cycle}]: MISC2 direct replay expected word-aligned pc, got {pc:#010x}"
    );
    let pc_low = pc & 0xffff;
    let pc_high = pc >> 16;
    let upper_base = upper_base_for_mode(cycle, machine_mode, "MISC2")?;
    ensure!(
        pc_high <= upper_base,
        "[{cycle}]: MISC2 direct replay pc high word exceeds mode range: pc_high={pc_high} upper_base={upper_base}"
    );

    let (inst, rs1_word, rs2_word) = get_cycle_inst_and_sources(preflight, cycle)?;
    replay_u16_lookup_delta(cycle, tables, upper_base - pc_high)?;
    replay_u16_lookup_delta(cycle, tables, pc_low >> 2)?;
    replay_normalized_word_lookup_deltas(
        cycle,
        tables,
        misc2_write_word(preflight, cycle, inst, minor),
    )?;
    replay_normalized_word_lookup_deltas(cycle, tables, preflight.cycles[cycle].pc)?;
    match minor {
        0 => replay_signed_cmp_lookup_deltas(cycle, tables, rs1_word, rs2_word)?,
        2 => replay_unsigned_cmp_lookup_deltas(cycle, tables, rs1_word, rs2_word)?,
        _ => {}
    }
    Ok(())
}

fn replay_split_word_lookup_deltas(cycle: usize, tables: &LookupTables, word: u32) -> Result<()> {
    replay_u8_lookup_delta(cycle, tables, word & 0xff)?;
    replay_u8_lookup_delta(cycle, tables, (word >> 8) & 0xff)
}

fn replay_mem0_lookup_deltas_from_preflight(
    preflight: &PreflightTrace,
    cycle: usize,
    tables: &LookupTables,
    minor: u8,
) -> Result<()> {
    let (pc, machine_mode) = current_pc_and_machine_mode(preflight, cycle);
    ensure!(
        pc & 3 == 0,
        "[{cycle}]: MEM0 direct replay expected word-aligned pc, got {pc:#010x}"
    );

    let (inst, rs1_word, load_word) = get_cycle_inst_rs1_and_load_word(preflight, cycle)?;
    let imm_i = ((inst as i32) >> 20) as u32;
    let load_addr = rs1_word.wrapping_add(imm_i);
    let load_low2 = load_addr & 3;
    let halfword = if (load_low2 & 2) != 0 {
        load_word >> 16
    } else {
        load_word & 0xffff
    };

    replay_addr_decompose_lookup_deltas(cycle, tables, pc, machine_mode, "MEM0 pc")?;
    replay_normalized_word_lookup_deltas(cycle, tables, load_addr)?;
    replay_addr_decompose_lookup_deltas(cycle, tables, load_addr, machine_mode, "MEM0 load")?;

    match minor {
        0 => {
            replay_split_word_lookup_deltas(cycle, tables, halfword)?;
            let byte = if (load_low2 & 1) != 0 {
                (halfword >> 8) & 0xff
            } else {
                halfword & 0xff
            };
            replay_u8_lookup_delta(cycle, tables, (byte & 0x7f) * 2)?;
        }
        1 => {
            ensure!(
                (load_low2 & 1) == 0,
                "[{cycle}]: MEM0 LH direct replay expected halfword alignment, got addr={load_addr:#010x}"
            );
            replay_u16_lookup_delta(cycle, tables, (halfword & 0x7fff) * 2)?;
        }
        2 => {
            ensure!(
                load_low2 == 0,
                "[{cycle}]: MEM0 LW direct replay expected word alignment, got addr={load_addr:#010x}"
            );
        }
        3 => {
            replay_split_word_lookup_deltas(cycle, tables, halfword)?;
        }
        4 => {
            ensure!(
                (load_low2 & 1) == 0,
                "[{cycle}]: MEM0 LHU direct replay expected halfword alignment, got addr={load_addr:#010x}"
            );
        }
        _ => anyhow::bail!("[{cycle}]: unsupported MEM0 direct replay minor={minor}"),
    }

    replay_normalized_word_lookup_deltas(cycle, tables, pc.wrapping_add(4))
}

fn mem1_store_word(
    cycle: usize,
    minor: u8,
    store_addr: u32,
    rs2_word: u32,
    old_word: u32,
) -> Result<u32> {
    let low2 = store_addr & 3;
    match minor {
        0 => {
            let shift = low2 * 8;
            let mask = 0xffu32 << shift;
            Ok((old_word & !mask) | ((rs2_word & 0xff) << shift))
        }
        1 => {
            ensure!(
                (low2 & 1) == 0,
                "[{cycle}]: MEM1 SH direct replay expected halfword alignment, got addr={store_addr:#010x}"
            );
            let shift = (low2 & 2) * 8;
            let mask = 0xffffu32 << shift;
            Ok((old_word & !mask) | ((rs2_word & 0xffff) << shift))
        }
        2 => {
            ensure!(
                low2 == 0,
                "[{cycle}]: MEM1 SW direct replay expected word alignment, got addr={store_addr:#010x}"
            );
            Ok(rs2_word)
        }
        _ => anyhow::bail!("[{cycle}]: unsupported MEM1 direct replay minor={minor}"),
    }
}

fn replay_mem1_lookup_deltas_from_preflight(
    preflight: &PreflightTrace,
    cycle: usize,
    tables: &LookupTables,
    minor: u8,
) -> Result<()> {
    let (pc, machine_mode) = current_pc_and_machine_mode(preflight, cycle);
    ensure!(
        pc & 3 == 0,
        "[{cycle}]: MEM1 direct replay expected word-aligned pc, got {pc:#010x}"
    );

    let (_inst, _rs1_word, rs2_word, store_addr, old_word, _new_word) =
        get_cycle_store_context(preflight, cycle, minor)?;
    let halfword = if (store_addr & 2) != 0 {
        old_word >> 16
    } else {
        old_word & 0xffff
    };

    replay_addr_decompose_lookup_deltas(cycle, tables, pc, machine_mode, "MEM1 pc")?;
    replay_normalized_word_lookup_deltas(cycle, tables, store_addr)?;
    replay_addr_decompose_lookup_deltas(cycle, tables, store_addr, machine_mode, "MEM1 store")?;

    if minor == 0 {
        replay_split_word_lookup_deltas(cycle, tables, halfword)?;
        replay_split_word_lookup_deltas(cycle, tables, rs2_word & 0xffff)?;
    }

    replay_normalized_word_lookup_deltas(cycle, tables, pc.wrapping_add(4))
}

fn replay_short_circuit_side_effects(
    preflight: &PreflightTrace,
    tables: &LookupTables,
    cycle: usize,
    data: BufferRow<Val>,
) -> Result<()> {
    let preflight_cycle = &preflight.cycles[cycle];
    match (preflight_cycle.major, preflight_cycle.minor) {
        // Bounded replacement slice: MISC0 arithmetic/bitwise ops.
        // The GPU writes the witness cells, but later Control0 rows read the
        // mutable U16 lookup counts that step_Top would have advanced. Replay
        // the common decode/finalize table deltas directly from preflight.
        (0, minor) if misc0_simple_short_circuit_minor(minor) => {
            replay_misc0_simple_lookup_deltas(preflight, cycle, tables)
        }
        (2, minor) if misc2_short_circuit_minor(minor) => {
            replay_misc2_lookup_deltas_from_preflight(preflight, cycle, tables, minor)
        }
        (5, minor) if mem0_short_circuit_minor(minor) => {
            replay_mem0_lookup_deltas_from_preflight(preflight, cycle, tables, minor)
        }
        (6, minor) if mem1_short_circuit_minor(minor) => {
            replay_mem1_lookup_deltas_from_preflight(preflight, cycle, tables, minor)
        }
        _ => Ok(()),
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

static EQZ_ELIDED: AtomicBool = AtomicBool::new(false);

#[inline(always)]
fn eqz_elided() -> bool {
    EQZ_ELIDED.load(Ordering::Relaxed)
}

pub(crate) struct EqzElisionGuard {
    previous: bool,
}

impl Drop for EqzElisionGuard {
    fn drop(&mut self) {
        EQZ_ELIDED.store(self.previous, Ordering::Release);
    }
}

/// M6d: guard form of [`with_eqz_elided`] for the pool-offloaded witgen
/// pass, where elision must stay on across an `.await` rather than a
/// closure. Every taker sets the flag true and restores its previous
/// value, so overlapping guard lifetimes (an offloaded witgen spanning
/// another segment's blocking accum scope) compose correctly.
pub(crate) fn begin_eqz_elided() -> EqzElisionGuard {
    let previous = EQZ_ELIDED.swap(true, Ordering::AcqRel);
    EqzElisionGuard { previous }
}

pub(crate) fn with_eqz_elided<R>(f: impl FnOnce() -> R) -> R {
    let _guard = begin_eqz_elided();
    f()
}

pub(crate) fn with_accum_eqz_elided<R>(f: impl FnOnce() -> R) -> R {
    with_eqz_elided(f)
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
    replace_arm_mask: u16,
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
            result = run_witness_steps(mode, replace_arm_mask, preflight, data, global);
        });
    });
    result
}

/// M6d: [`generate_witness`] over bare `CpuBuffer` shadow handles, for the
/// pool-offloaded witgen pass. `CpuBuffer` is `Send + Sync`, so a rayon
/// worker can run this while the main wasm thread keeps servicing another
/// segment's readback callbacks. View discipline matches
/// `generate_witness`: mutable pass over `data`, read view of `global`;
/// the caller applies the `WebGpuBuffer` dirty-flag halves around it.
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_witness_on_shadows(
    mode: StepMode,
    replace_arm_mask: u16,
    preflight: &PreflightTrace,
    global: &risc0_zkp::hal::cpu::CpuBuffer<Val>,
    global_rows: usize,
    global_cols: usize,
    global_checked: bool,
    data: &risc0_zkp::hal::cpu::CpuBuffer<Val>,
    data_rows: usize,
    data_cols: usize,
    data_checked: bool,
) -> Result<()> {
    let mut result = Ok(());
    data.view_mut(|data_view| {
        global.view(|global_view| {
            let data = BufferRow::mutable(data_view, data_rows, data_cols, data_checked);
            let global = BufferRow::global(global_view, global_rows, global_cols, global_checked);
            result = run_witness_steps(mode, replace_arm_mask, preflight, data, global);
        });
    });
    result
}

pub(crate) fn step_accum<H>(
    replace_arm_mask: u16,
    preflight: &PreflightTrace,
    data: &MetaBuffer<H>,
    accum: &MetaBuffer<H>,
    global: &MetaBuffer<H>,
    mix: &MetaBuffer<H>,
) -> Result<()>
where
    H: risc0_zkp::hal::Hal<Field = CircuitField, Elem = Val, ExtElem = ExtVal>,
{
    step_accum_inner(replace_arm_mask, preflight, data, accum, global, mix, true)
}

pub(crate) fn repair_witgen_gpu_replace_shadow_for_accum<H>(
    replace_arm_mask: u16,
    preflight: &PreflightTrace,
    global: &MetaBuffer<H>,
    data: &MetaBuffer<H>,
) -> Result<usize>
where
    H: risc0_zkp::hal::Hal<Field = CircuitField, Elem = Val, ExtElem = ExtVal>,
{
    let mut result = Ok(0);
    data.buf.view_mut(|data_view| {
        global.buf.view(|global_view| {
            let data = BufferRow::mutable(data_view, data.rows, data.cols, false);
            let global = BufferRow::global(global_view, global.rows, global.cols, global.checked);
            let tables = LookupTables::default();
            let mut repaired = 0usize;

            #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
            let _timer = risc0_zkp::hal::webgpu::WebGpuStageTimer::new(format!(
                "rv32im_witgen_accum_shadow_replay cycles={}",
                preflight.cycles.len()
            ));

            for cycle in 0..preflight.cycles.len() {
                let major = preflight.cycles[cycle].major;
                let minor = preflight.cycles[cycle].minor;
                if !cycle_short_circuited(replace_arm_mask, major, minor) {
                    continue;
                }
                let ctx = ExecContext::new(preflight, &tables, cycle);
                if let Err(err) = step_Top(&ctx, data, global).map_err(|e| {
                    anyhow::anyhow!(
                        "step_Top shadow repair failed at cycle={cycle} major={major} minor={minor}: {e}"
                    )
                }) {
                    result = Err(err);
                    return;
                }
                repaired += 1;
            }
            if repaired != 0 {
                WITGEN_ACCUM_SHADOW_REPLAY_ROWS.fetch_add(repaired, Ordering::Relaxed);
            }
            result = Ok(repaired);
        });
    });
    result
}

pub(crate) fn step_accum_without_machine_column_carry<H>(
    replace_arm_mask: u16,
    preflight: &PreflightTrace,
    data: &MetaBuffer<H>,
    accum: &MetaBuffer<H>,
    global: &MetaBuffer<H>,
    mix: &MetaBuffer<H>,
) -> Result<()>
where
    H: risc0_zkp::hal::Hal<Field = CircuitField, Elem = Val, ExtElem = ExtVal>,
{
    step_accum_inner(replace_arm_mask, preflight, data, accum, global, mix, false)
}

pub(crate) fn step_accum_without_major_or_postprocess<H>(
    preflight: &PreflightTrace,
    data: &MetaBuffer<H>,
    accum: &MetaBuffer<H>,
    global: &MetaBuffer<H>,
    mix: &MetaBuffer<H>,
    skip_major: u8,
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
                    result = run_accum_raw_steps_skip_major(
                        preflight, data, accum, global, mix, skip_major,
                    );
                });
            });
        });
    });
    result
}

pub(crate) fn step_accum_without_replaced_misc0_or_postprocess<H>(
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
                    result = run_accum_raw_steps_skip_replaced_misc0_or_majors(
                        preflight, data, accum, global, mix, true, false, 0,
                    );
                });
            });
        });
    });
    result
}

pub(crate) fn step_accum_without_selected_majors_or_postprocess<H>(
    preflight: &PreflightTrace,
    data: &MetaBuffer<H>,
    accum: &MetaBuffer<H>,
    global: &MetaBuffer<H>,
    mix: &MetaBuffer<H>,
    skip_replaced_misc0: bool,
    skip_replaced_mem0: bool,
    skip_major_mask: u16,
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
                    result = run_accum_raw_steps_skip_replaced_misc0_or_majors(
                        preflight,
                        data,
                        accum,
                        global,
                        mix,
                        skip_replaced_misc0,
                        skip_replaced_mem0,
                        skip_major_mask,
                    );
                });
            });
        });
    });
    result
}

pub(crate) fn finish_accum_machine_column_carry<H>(accum: &MetaBuffer<H>)
where
    H: risc0_zkp::hal::Hal<Field = CircuitField, Elem = Val, ExtElem = ExtVal>,
{
    let last_cycle = accum.rows;
    accum.buf.view_mut(|accum_view| {
        let accum =
            BufferRow::mutable(accum_view, accum.rows, accum.cols, accum.checked).unchecked();
        apply_machine_column_carry(accum, last_cycle);
    });
}

fn step_accum_inner<H>(
    replace_arm_mask: u16,
    preflight: &PreflightTrace,
    data: &MetaBuffer<H>,
    accum: &MetaBuffer<H>,
    global: &MetaBuffer<H>,
    mix: &MetaBuffer<H>,
    run_machine_column_carry: bool,
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
                    result = run_accum_steps(
                        replace_arm_mask,
                        preflight,
                        data,
                        accum,
                        global,
                        mix,
                        run_machine_column_carry,
                    );
                });
            });
        });
    });
    result
}

fn run_witness_steps(
    mode: StepMode,
    replace_arm_mask: u16,
    preflight: &PreflightTrace,
    data: BufferRow<Val>,
    global: BufferRow<Val>,
) -> Result<()> {
    let tables = LookupTables::default();
    let split = preflight.table_split_cycle as usize;
    let last_cycle = preflight.cycles.len();

    match mode {
        StepMode::Parallel => {
            // 1:1 port of the C reference (rv32im-sys ffi.cpp
            // `risc0_circuit_rv32im_cpu_witgen`): `poolstl::par` for_each over
            // each side of the table split. Cycles within a phase are
            // independent — preflight pre-resolves cross-cycle data flow —
            // and the phase boundary is the barrier between lookup-count
            // accumulation and table-row emission reading final counts.
            (0..split).into_par_iter().try_for_each(|cycle| {
                step_exec(replace_arm_mask, preflight, &tables, cycle, data, global)
            })?;
            (split..last_cycle).into_par_iter().try_for_each(|cycle| {
                step_exec(replace_arm_mask, preflight, &tables, cycle, data, global)
            })?;
        }
        StepMode::SeqForward => {
            for cycle in 0..split {
                step_exec(replace_arm_mask, preflight, &tables, cycle, data, global)?;
            }
            for cycle in split..last_cycle {
                step_exec(replace_arm_mask, preflight, &tables, cycle, data, global)?;
            }
        }
        StepMode::SeqReverse => {
            for cycle in (0..split).rev() {
                step_exec(replace_arm_mask, preflight, &tables, cycle, data, global)?;
            }
            for cycle in (split..last_cycle).rev() {
                step_exec(replace_arm_mask, preflight, &tables, cycle, data, global)?;
            }
        }
    }
    Ok(())
}

fn run_accum_raw_steps_skip_major(
    preflight: &PreflightTrace,
    data: BufferRow<Val>,
    accum: BufferRow<Val>,
    global: BufferRow<Val>,
    mix: BufferRow<Val>,
    skip_major: u8,
) -> Result<()> {
    let tables = LookupTables::default();
    let last_cycle = preflight.cycles.len();

    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    let step_top_accum_timer = risc0_zkp::hal::webgpu::WebGpuStageTimer::new(format!(
        "rv32im_accumulate step_top_accum_cpu_skip_major{skip_major} cycles={last_cycle}"
    ));
    // Per-cycle accum steps are parallel in the C reference (cpu_accum
    // phase1 runs stepAccum under poolstl::par; the cross-cycle recurrences
    // live in the separate prefix passes).
    (0..last_cycle).into_par_iter().try_for_each(|cycle| {
        let major = preflight.cycles[cycle].major;
        let minor = preflight.cycles[cycle].minor;
        if major == skip_major {
            return Ok(());
        }
        let ctx = ExecContext::new(preflight, &tables, cycle);
        step_TopAccum(&ctx, accum, data, global, mix).map_err(|e| {
            anyhow::anyhow!(
                "step_TopAccum failed at cycle={cycle} major={major} minor={minor}: {e}"
            )
        })
    })?;
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    drop(step_top_accum_timer);

    Ok(())
}

fn run_accum_raw_steps_skip_replaced_misc0_or_majors(
    preflight: &PreflightTrace,
    data: BufferRow<Val>,
    accum: BufferRow<Val>,
    global: BufferRow<Val>,
    mix: BufferRow<Val>,
    skip_replaced_misc0: bool,
    skip_replaced_mem0: bool,
    skip_major_mask: u16,
) -> Result<()> {
    let tables = LookupTables::default();
    let last_cycle = preflight.cycles.len();

    let run_pass = |mask: u16| -> Result<()> {
        (0..last_cycle).into_par_iter().try_for_each(|cycle| {
            let major = preflight.cycles[cycle].major;
            let minor = preflight.cycles[cycle].minor;
            let skip_major = major < 16 && (mask & (1u16 << major)) != 0;
            if (skip_replaced_misc0 && major == 0 && misc0_simple_short_circuit_minor(minor))
                || (skip_replaced_mem0 && major == 5 && mem0_short_circuit_minor(minor))
                || skip_major
            {
                return Ok(());
            }
            let ctx = ExecContext::new(preflight, &tables, cycle);
            step_TopAccum(&ctx, accum, data, global, mix).map_err(|e| {
                anyhow::anyhow!(
                    "step_TopAccum failed at cycle={cycle} major={major} minor={minor}: {e}"
                )
            })
        })
    };

    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    let step_top_accum_timer = risc0_zkp::hal::webgpu::WebGpuStageTimer::new(format!(
        "rv32im_accumulate step_top_accum_cpu_skip_replaced_misc0={skip_replaced_misc0}_mem0={skip_replaced_mem0}_major_mask=0x{skip_major_mask:04x} cycles={last_cycle}"
    ));
    // Parallel per the C reference's phase1 (see run_accum_raw_steps_skip_major).
    run_pass(skip_major_mask)?;
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    drop(step_top_accum_timer);

    Ok(())
}

fn run_accum_steps(
    replace_arm_mask: u16,
    preflight: &PreflightTrace,
    data: BufferRow<Val>,
    accum: BufferRow<Val>,
    global: BufferRow<Val>,
    mix: BufferRow<Val>,
    run_machine_column_carry: bool,
) -> Result<()> {
    let tables = LookupTables::default();
    let last_cycle = preflight.cycles.len();
    let direct_misc0_enabled = WITGEN_GPU_DIRECT_MISC0_ACCUM_ENABLED.load(Ordering::Acquire);

    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    let step_top_accum_timer = {
        let label = if direct_misc0_enabled {
            "rv32im_accumulate step_top_accum_direct_misc0"
        } else {
            "rv32im_accumulate step_top_accum"
        };
        risc0_zkp::hal::webgpu::WebGpuStageTimer::new(format!("{label} cycles={last_cycle}"))
    };
    // Parallel per the C reference's phase1 (see run_accum_raw_steps_skip_major).
    let direct_misc0_rows = AtomicUsize::new(0);
    (0..last_cycle).into_par_iter().try_for_each(|cycle| {
        let major = preflight.cycles[cycle].major;
        let minor = preflight.cycles[cycle].minor;
        if direct_misc0_enabled && cycle_short_circuited(replace_arm_mask, major, minor) && major == 0 {
            direct_misc0_accum_step(cycle, data, accum, mix).map_err(|e| {
                anyhow::anyhow!(
                    "direct MISC0 step_TopAccum failed at cycle={cycle} major={major} minor={minor}: {e}"
                )
            })?;
            direct_misc0_rows.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        let ctx = ExecContext::new(preflight, &tables, cycle);
        step_TopAccum(&ctx, accum, data, global, mix).map_err(|e| {
            anyhow::anyhow!(
                "step_TopAccum failed at cycle={cycle} major={major} minor={minor}: {e}"
            )
        })
    })?;
    let direct_misc0_rows = direct_misc0_rows.into_inner();
    if direct_misc0_rows != 0 {
        WITGEN_GPU_DIRECT_MISC0_ACCUM_ROWS.fetch_add(direct_misc0_rows, Ordering::Relaxed);
    }
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    drop(step_top_accum_timer);

    let accum = accum.unchecked();

    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    let terminal_ext_prefix_timer = risc0_zkp::hal::webgpu::WebGpuStageTimer::new(format!(
        "rv32im_accumulate terminal_ext_prefix cycles={last_cycle}"
    ));
    for elem_idx in 0..ExtVal::EXT_SIZE {
        let col = accum.cols - ExtVal::EXT_SIZE + elem_idx;
        let mut running = Val::ZERO;
        for row in 0..last_cycle {
            let cur = accum.get_at(row, col, true);
            running += cur;
            accum.set_at(row, col, running);
        }
    }
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    drop(terminal_ext_prefix_timer);

    let split = LAYOUT_TOP_ACCUM.columns[0].offset;
    let machine_columns = (accum.cols - split) / ExtVal::EXT_SIZE;
    if run_machine_column_carry {
        apply_machine_column_carry(accum, last_cycle);
    }

    Ok(())
}

fn ext_from_val(val: Val) -> ExtVal {
    ExtVal::from_subfield(&val)
}

fn load_ext_at(buffer: BufferRow<Val>, row: usize, offset: usize) -> ExtVal {
    ExtVal::new(
        buffer.get_at(row, offset, false),
        buffer.get_at(row, offset + 1, false),
        buffer.get_at(row, offset + 2, false),
        buffer.get_at(row, offset + 3, false),
    )
}

fn store_ext_at(buffer: BufferRow<Val>, row: usize, offset: usize, value: ExtVal) {
    for (i, elem) in value.elems().iter().copied().enumerate() {
        buffer.set_at(row, offset + i, elem);
    }
}

fn load_nd_at(data: BufferRow<Val>, row: usize, layout: &'static NondetRegLayout) -> Val {
    data.get_at(row, layout._super.offset, false)
}

fn arg_u16_accum_term(
    data: BufferRow<Val>,
    mix: BufferRow<Val>,
    row: usize,
    arg: &'static ArgU16Layout,
) -> ExtVal {
    let randomness = LAYOUT_MIX.randomness;
    let count = load_nd_at(data, row, arg.count);
    let value = load_nd_at(data, row, arg.val);
    let denom = load_ext_at(mix, 0, randomness.arg_u16.val.offset) * ext_from_val(value)
        + load_ext_at(mix, 0, randomness._offset.offset);
    ext_from_val(count) * denom.inv()
}

fn memory_accum_term(
    data: BufferRow<Val>,
    mix: BufferRow<Val>,
    row: usize,
    arg: &'static MemoryArgLayout,
) -> ExtVal {
    let randomness = LAYOUT_MIX.randomness;
    let denom = load_ext_at(mix, 0, randomness.memory_arg.addr.offset)
        * ext_from_val(load_nd_at(data, row, arg.addr))
        + load_ext_at(mix, 0, randomness.memory_arg.cycle.offset)
            * ext_from_val(load_nd_at(data, row, arg.cycle))
        + load_ext_at(mix, 0, randomness.memory_arg.data_low.offset)
            * ext_from_val(load_nd_at(data, row, arg.data_low))
        + load_ext_at(mix, 0, randomness.memory_arg.data_high.offset)
            * ext_from_val(load_nd_at(data, row, arg.data_high))
        + load_ext_at(mix, 0, randomness._offset.offset);
    ext_from_val(load_nd_at(data, row, arg.count)) * denom.inv()
}

fn cycle_accum_term(
    data: BufferRow<Val>,
    mix: BufferRow<Val>,
    row: usize,
    arg: &'static CycleArgLayout,
) -> ExtVal {
    let randomness = LAYOUT_MIX.randomness;
    let denom = load_ext_at(mix, 0, randomness.cycle_arg.cycle.offset)
        * ext_from_val(load_nd_at(data, row, arg.cycle))
        + load_ext_at(mix, 0, randomness._offset.offset);
    ext_from_val(load_nd_at(data, row, arg.count)) * denom.inv()
}

fn store_direct_misc0_user_accum(accum: BufferRow<Val>, row: usize) {
    let user = LAYOUT_TOP_ACCUM.user._0;
    store_ext_at(accum, row, user.state.poly._super.offset, ExtVal::ZERO);
    store_ext_at(accum, row, user.state.term._super.offset, ExtVal::ONE);
    store_ext_at(accum, row, user.state.total._super.offset, ExtVal::ZERO);
    for (idx, bit) in user.poly_op._super.iter().enumerate() {
        accum.set_at(
            row,
            bit._super.offset,
            if idx == 0 { Val::ONE } else { Val::ZERO },
        );
    }
    store_ext_at(
        accum,
        row,
        user.state_redef.arm3.tmp._super.offset,
        ExtVal::ZERO,
    );
}

fn direct_misc0_accum_step(
    row: usize,
    data: BufferRow<Val>,
    accum: BufferRow<Val>,
    mix: BufferRow<Val>,
) -> Result<()> {
    let misc0 = LAYOUT_TOP.inst_result.arm0;
    let output_args = misc0._arguments_misc0_misc_output.arg_u16;
    let write_rd = misc0._super._0;
    let decoded = misc0.input.decoded;
    let source_args = misc0
        .input
        .source_regs
        ._arguments_read_source_regs_source_regs;

    store_direct_misc0_user_accum(accum, row);

    let mut cur = ExtVal::ZERO;

    cur += arg_u16_accum_term(data, mix, row, misc0._super.write_data.low16.arg);
    cur += arg_u16_accum_term(data, mix, row, misc0._super.write_data.high16.arg);
    cur += arg_u16_accum_term(data, mix, row, misc0._super.pc_norm.low16.arg);
    store_ext_at(accum, row, LAYOUT_TOP_ACCUM.columns[0].offset, cur);

    cur += arg_u16_accum_term(data, mix, row, misc0._super.pc_norm.high16.arg);
    cur += memory_accum_term(data, mix, row, write_rd._0.io.old_txn);
    cur += memory_accum_term(data, mix, row, write_rd._0.io.new_txn);
    store_ext_at(accum, row, LAYOUT_TOP_ACCUM.columns[1].offset, cur);

    cur += cycle_accum_term(data, mix, row, write_rd._0._0._0.arg);
    cur += cycle_accum_term(data, mix, row, misc0._0.arg1);
    cur += cycle_accum_term(data, mix, row, misc0._0.arg2);
    store_ext_at(accum, row, LAYOUT_TOP_ACCUM.columns[2].offset, cur);

    cur += arg_u16_accum_term(data, mix, row, decoded.pc_addr.upper_diff.arg);
    cur += arg_u16_accum_term(data, mix, row, decoded.pc_addr.med14.arg);
    cur += memory_accum_term(data, mix, row, decoded.load_inst.io.old_txn);
    store_ext_at(accum, row, LAYOUT_TOP_ACCUM.columns[3].offset, cur);

    cur += memory_accum_term(data, mix, row, decoded.load_inst.io.new_txn);
    cur += cycle_accum_term(data, mix, row, decoded.load_inst._0._0.arg);
    cur += memory_accum_term(data, mix, row, source_args.memory_arg[0]);
    store_ext_at(accum, row, LAYOUT_TOP_ACCUM.columns[4].offset, cur);

    cur += memory_accum_term(data, mix, row, source_args.memory_arg[1]);
    cur += memory_accum_term(data, mix, row, source_args.memory_arg[2]);
    cur += memory_accum_term(data, mix, row, source_args.memory_arg[3]);
    store_ext_at(accum, row, LAYOUT_TOP_ACCUM.columns[5].offset, cur);

    cur += cycle_accum_term(data, mix, row, source_args.cycle_arg[0]);
    cur += cycle_accum_term(data, mix, row, source_args.cycle_arg[1]);
    cur += arg_u16_accum_term(data, mix, row, output_args[0]);
    store_ext_at(accum, row, LAYOUT_TOP_ACCUM.columns[6].offset, cur);

    cur += arg_u16_accum_term(data, mix, row, output_args[1]);
    cur += arg_u16_accum_term(data, mix, row, output_args[2]);
    cur += arg_u16_accum_term(data, mix, row, output_args[3]);
    store_ext_at(accum, row, LAYOUT_TOP_ACCUM.columns[7].offset, cur);

    cur += arg_u16_accum_term(data, mix, row, output_args[4]);
    store_ext_at(accum, row, LAYOUT_TOP_ACCUM.columns[8].offset, cur);
    store_ext_at(accum, row, LAYOUT_TOP_ACCUM.columns[19].offset, cur);

    Ok(())
}

fn apply_machine_column_carry(accum: BufferRow<Val>, last_cycle: usize) {
    let split = LAYOUT_TOP_ACCUM.columns[0].offset;
    let machine_columns = (accum.cols - split) / ExtVal::EXT_SIZE;
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    let machine_column_carry_timer = risc0_zkp::hal::webgpu::WebGpuStageTimer::new(format!(
        "rv32im_accumulate machine_column_carry cycles={last_cycle} machine_columns={machine_columns}"
    ));
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
    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    drop(machine_column_carry_timer);
}

// SP7 iter-6d-g step 6.2.3: per-segment arm mask. Bit k set => major
// opcode k's cycles short-circuit step_Top. M6d: this global is a
// DIAGNOSTICS MIRROR only — it records the mask most recently computed by
// `pre_witgen_dispatch_async` so tests can assert which arms dispatched.
// The correctness-bearing copy lives per prove on `WebGpuCircuitHal`
// (`witgen_replace_arm_mask` Cell) and flows into rust_steps as an
// explicit parameter, because the M6d segment pipeline overlaps segment
// N+1's witgen (which computes ITS mask) with segment N's accum phase
// (which still reads N's mask). CPU HAL passes 0 (no short-circuit ever).
static WITGEN_GPU_REPLACE_ARM_MASK: AtomicU16 = AtomicU16::new(0);
const WITGEN_GPU_MEM0_REPLACE_MINOR_MASK_ALL: u16 = 0x001f;
static WITGEN_GPU_MEM0_REPLACE_MINOR_MASK: AtomicU16 =
    AtomicU16::new(WITGEN_GPU_MEM0_REPLACE_MINOR_MASK_ALL);
const WITGEN_GPU_MEM1_REPLACE_MINOR_MASK_ALL: u16 = 0x0007;
static WITGEN_GPU_MEM1_REPLACE_MINOR_MASK: AtomicU16 =
    AtomicU16::new(WITGEN_GPU_MEM1_REPLACE_MINOR_MASK_ALL);
// Legacy process-wide gate kept for the public setter so callers can
// flip the feature on/off; webgpu.rs reads this to decide whether to
// populate WITGEN_GPU_REPLACE_ARM_MASK each segment.
static WITGEN_GPU_REPLACE_ENABLED: AtomicBool = AtomicBool::new(false);
static WITGEN_GPU_SHORT_CIRCUIT_CYCLES: AtomicUsize = AtomicUsize::new(0);
static WITGEN_ACCUM_SHADOW_REPLAY_ROWS: AtomicUsize = AtomicUsize::new(0);
static WITGEN_GPU_DIRECT_MISC0_ACCUM_ENABLED: AtomicBool = AtomicBool::new(false);
static WITGEN_GPU_DIRECT_MISC0_ACCUM_ROWS: AtomicUsize = AtomicUsize::new(0);

pub fn set_witgen_gpu_replace_enabled(enabled: bool) {
    WITGEN_GPU_REPLACE_ENABLED.store(enabled, Ordering::Release);
    WITGEN_GPU_SHORT_CIRCUIT_CYCLES.store(0, Ordering::Release);
    WITGEN_ACCUM_SHADOW_REPLAY_ROWS.store(0, Ordering::Release);
    if !enabled {
        WITGEN_GPU_REPLACE_ARM_MASK.store(0, Ordering::Release);
    }
}

pub fn set_witgen_gpu_replace_arm_mask(mask: u16) {
    WITGEN_GPU_REPLACE_ARM_MASK.store(mask, Ordering::Release);
}

pub fn witgen_gpu_short_circuit_cycles() -> usize {
    WITGEN_GPU_SHORT_CIRCUIT_CYCLES.load(Ordering::Acquire)
}

pub fn witgen_accum_shadow_replay_rows() -> usize {
    WITGEN_ACCUM_SHADOW_REPLAY_ROWS.load(Ordering::Acquire)
}

pub fn witgen_gpu_replace_arm_mask() -> u16 {
    WITGEN_GPU_REPLACE_ARM_MASK.load(Ordering::Acquire)
}

pub fn set_witgen_gpu_mem0_replace_minor_mask(mask: u16) {
    WITGEN_GPU_MEM0_REPLACE_MINOR_MASK.store(
        mask & WITGEN_GPU_MEM0_REPLACE_MINOR_MASK_ALL,
        Ordering::Release,
    );
}

pub fn witgen_gpu_mem0_replace_minor_mask() -> u16 {
    WITGEN_GPU_MEM0_REPLACE_MINOR_MASK.load(Ordering::Acquire)
}

pub fn set_witgen_gpu_mem1_replace_minor_mask(mask: u16) {
    WITGEN_GPU_MEM1_REPLACE_MINOR_MASK.store(
        mask & WITGEN_GPU_MEM1_REPLACE_MINOR_MASK_ALL,
        Ordering::Release,
    );
}

pub fn witgen_gpu_mem1_replace_minor_mask() -> u16 {
    WITGEN_GPU_MEM1_REPLACE_MINOR_MASK.load(Ordering::Acquire)
}

pub fn set_witgen_gpu_direct_misc0_accum_enabled(enabled: bool) {
    WITGEN_GPU_DIRECT_MISC0_ACCUM_ENABLED.store(enabled, Ordering::Release);
    WITGEN_GPU_DIRECT_MISC0_ACCUM_ROWS.store(0, Ordering::Release);
}

pub fn witgen_gpu_direct_misc0_accum_rows() -> usize {
    WITGEN_GPU_DIRECT_MISC0_ACCUM_ROWS.load(Ordering::Acquire)
}

fn cycle_short_circuited(replace_arm_mask: u16, major: u8, minor: u8) -> bool {
    if major >= 13 {
        return false;
    }
    if (replace_arm_mask & (1u16 << major)) == 0 {
        return false;
    }
    (major == 0 && misc0_simple_short_circuit_minor(minor))
        || (major == 2 && misc2_short_circuit_minor(minor))
        || (major == 5 && mem0_short_circuit_minor(minor))
        || (major == 6 && mem1_short_circuit_minor(minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn misc0_arithmetic_and_bitwise_cycles_are_short_circuitable_when_arm_mask_enabled() {
        for minor in [0, 1, 2, 3, 4, 7] {
            assert!(
                cycle_short_circuited(1, 0, minor),
                "MISC0 minor {minor} should be covered by GPU-witgen replacement"
            );
        }
        for minor in [5, 6] {
            assert!(
                !cycle_short_circuited(1, 0, minor),
                "MISC0 compare minor {minor} remains CPU-covered because sparse shadow repair is wall-negative"
            );
        }
    }

    #[test]
    fn misc2_cycles_are_short_circuitable_when_arm_mask_enabled() {
        for minor in [0, 2, 3, 4, 5, 6, 7] {
            assert!(
                cycle_short_circuited(1u16 << 2, 2, minor),
                "MISC2 minor {minor} should be covered by GPU-witgen replacement"
            );
        }
        assert!(
            !cycle_short_circuited(1u16 << 2, 2, 1),
            "MISC2 minor 1 remains CPU-covered until its nested source-reg mux is complete"
        );
    }

    #[test]
    fn mem0_load_cycles_are_short_circuitable_when_arm_mask_enabled() {
        for minor in [0, 1, 2, 3, 4] {
            assert!(
                cycle_short_circuited(1u16 << 5, 5, minor),
                "MEM0 minor {minor} should be covered by GPU-witgen replacement"
            );
        }
        for minor in [5, 6, 7] {
            assert!(
                !cycle_short_circuited(1u16 << 5, 5, minor),
                "MEM0 minor {minor} should remain CPU-covered"
            );
        }
    }

    #[test]
    fn mem0_minor_mask_filters_short_circuitable_load_cycles() {
        set_witgen_gpu_mem0_replace_minor_mask(1u16 << 2);
        assert!(
            cycle_short_circuited(1u16 << 5, 5, 2),
            "MEM0 LW minor should remain short-circuitable when selected"
        );
        for minor in [0, 1, 3, 4, 5, 6, 7] {
            assert!(
                !cycle_short_circuited(1u16 << 5, 5, minor),
                "MEM0 minor {minor} should stay CPU-covered when the LW-only mask is active"
            );
        }
        set_witgen_gpu_mem0_replace_minor_mask(0x001f);
    }

    #[test]
    fn mem1_store_cycles_are_short_circuitable_when_arm_mask_enabled() {
        for minor in [0, 1, 2] {
            assert!(
                cycle_short_circuited(1u16 << 6, 6, minor),
                "MEM1 minor {minor} should be covered by GPU-witgen replacement"
            );
        }
        for minor in [3, 4, 5, 6, 7] {
            assert!(
                !cycle_short_circuited(1u16 << 6, 6, minor),
                "illegal MEM1 minor {minor} should remain CPU-covered"
            );
        }
    }

    #[test]
    fn mem1_store_byte_replay_restores_lookup_deltas() {
        use risc0_circuit_rv32im_sys::{RawMemoryTransaction, RawPreflightCycle};

        let pc = 0x1000;
        let rs1_word = 0x1000;
        let rs2_word = 0xaabb_ccdd;
        let old_store_word = 0x1122_3344;
        let inst = (2 << 20) | (1 << 15) | (4 << 7) | 0x23;
        let preflight = PreflightTrace {
            cycles: vec![RawPreflightCycle {
                state: 0,
                pc,
                major: 6,
                minor: 0,
                machine_mode: 1,
                padding: 0,
                user_cycle: 0,
                txn_idx: 0,
                paging_idx: 0,
                bigint_idx: 0,
                diff_count: [0, 0],
            }],
            txns: vec![
                RawMemoryTransaction {
                    addr: pc / 4,
                    cycle: 0,
                    word: inst,
                    prev_cycle: u32::MAX,
                    prev_word: inst,
                },
                RawMemoryTransaction {
                    addr: 1_073_725_440 + 1,
                    cycle: 0,
                    word: rs1_word,
                    prev_cycle: u32::MAX,
                    prev_word: rs1_word,
                },
                RawMemoryTransaction {
                    addr: 1_073_725_440 + 2,
                    cycle: 0,
                    word: rs2_word,
                    prev_cycle: u32::MAX,
                    prev_word: rs2_word,
                },
                RawMemoryTransaction {
                    addr: (rs1_word + 4) / 4,
                    cycle: 0,
                    word: old_store_word,
                    prev_cycle: u32::MAX,
                    prev_word: old_store_word,
                },
                RawMemoryTransaction {
                    addr: (rs1_word + 4) / 4,
                    cycle: 1,
                    word: 0x1122_33dd,
                    prev_cycle: 0,
                    prev_word: old_store_word,
                },
            ],
            ..Default::default()
        };
        let tables = LookupTables::default();

        replay_mem1_lookup_deltas_from_preflight(&preflight, 0, &tables, 0).unwrap();

        assert_eq!(
            tables
                .lookup_current(Val::new(16), Val::new(0xffff))
                .unwrap()
                .as_u32(),
            2
        );
        assert_eq!(
            tables
                .lookup_current(Val::new(16), Val::new(0x1004))
                .unwrap()
                .as_u32(),
            2
        );
        assert_eq!(
            tables
                .lookup_current(Val::new(16), Val::new(0))
                .unwrap()
                .as_u32(),
            2
        );
        for byte in [0x44, 0x33, 0xdd, 0xcc] {
            assert_eq!(
                tables
                    .lookup_current(Val::new(8), Val::new(byte))
                    .unwrap()
                    .as_u32(),
                1,
                "expected one replayed u8 lookup for byte {byte:#04x}"
            );
        }
    }
}

fn step_exec(
    replace_arm_mask: u16,
    preflight: &PreflightTrace,
    tables: &LookupTables,
    cycle: usize,
    data: BufferRow<Val>,
    global: BufferRow<Val>,
) -> Result<()> {
    let major = preflight.cycles[cycle].major;
    let minor = preflight.cycles[cycle].minor;
    if cycle_short_circuited(replace_arm_mask, major, minor) {
        replay_short_circuit_side_effects(preflight, tables, cycle, data)?;
        WITGEN_GPU_SHORT_CIRCUIT_CYCLES.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }
    let ctx = ExecContext::new(preflight, tables, cycle);
    // SP7 iter 6d-g step 6.2.5 diagnostic: tag the error with cycle + arm
    // info so a downstream bail localizes WHICH cycle's step_Top blew up.
    step_Top(&ctx, data, global).map_err(|e| {
        anyhow::anyhow!("step_Top failed at cycle={cycle} major={major} minor={minor}: {e}")
    })
}

// SP7 iter 6d-g step 6.2.7 (2026-05-16): shadow the anyhow::bail! macro
// inside the included steps.rs.inc so each "Reached unreachable mux arm"
// bail carries its line number. Lets us bisect down to a specific mux
// site without editing the generated file.
macro_rules! bail {
    ($msg:literal) => {
        return Err(::anyhow::anyhow!("{} (steps.rs.inc:{})", $msg, ::std::line!()));
    };
    ($fmt:literal, $($arg:tt)*) => {
        return Err(::anyhow::anyhow!(
            "{} (steps.rs.inc:{})",
            format!($fmt, $($arg)*),
            ::std::line!()
        ));
    };
}

include!("../../zirgen/steps.rs.inc");
