// Copyright 2025 RISC Zero, Inc.
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

//! Interface between the circuit and prover/verifier

use alloc::{str::from_utf8, vec::Vec};
use core::fmt;

use risc0_core::field::{Elem, ExtElem, Field};
use serde::{Deserialize, Serialize};

use crate::taps::TapSet;

// TODO: Remove references to these constants so we don't depend on a
// fixed set of register groups.
pub const REGISTER_GROUP_ACCUM: usize = 0;
pub const REGISTER_GROUP_CODE: usize = 1;
pub const REGISTER_GROUP_DATA: usize = 2;

// If true, enable tracing of adapter internals.
const ADAPTER_TRACE_ENABLED: bool = false;

macro_rules! trace_if_enabled {
    ($($args:tt)*) => {
        if ADAPTER_TRACE_ENABLED {
            tracing::trace!($($args)*)
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct MixState<EE: ExtElem> {
    pub tot: EE,
    pub mul: EE,
}

pub trait PolyFp<F: Field> {
    fn poly_fp(
        &self,
        cycle: usize,
        steps: usize,
        mix: &[F::ExtElem],
        args: &[&[F::Elem]],
    ) -> F::ExtElem;
}

pub trait PolyExt<F: Field> {
    fn poly_ext(
        &self,
        _mix: &F::ExtElem,
        u: &[F::ExtElem],
        args: &[&[F::Elem]],
    ) -> MixState<F::ExtElem>;

    fn poly_ext_scratch(&self) -> PolyExtScratch<F> {
        PolyExtScratch::default()
    }

    fn poly_ext_with_scratch(
        &self,
        scratch: &mut PolyExtScratch<F>,
        mix: &F::ExtElem,
        u: &[F::ExtElem],
        args: &[&[F::Elem]],
    ) -> MixState<F::ExtElem> {
        let _ = scratch;
        self.poly_ext(mix, u, args)
    }
}

pub trait TapsProvider {
    fn get_taps(&self) -> &'static TapSet<'static>;

    fn accum_size(&self) -> usize {
        self.get_taps().group_size(REGISTER_GROUP_ACCUM)
    }

    fn code_size(&self) -> usize {
        self.get_taps().group_size(REGISTER_GROUP_CODE)
    }

    fn ctrl_size(&self) -> usize {
        self.get_taps().group_size(REGISTER_GROUP_CODE)
    }

    fn data_size(&self) -> usize {
        self.get_taps().group_size(REGISTER_GROUP_DATA)
    }
}

/// A protocol info string for the proof system and circuits.
/// Used to seed the Fiat-Shamir transcript and provide domain separation between different
/// protocol and circuit versions.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolInfo(pub [u8; 16]);

impl ProtocolInfo {
    /// Encode a fixed context byte-string to elements, with one element per byte.
    // NOTE: This function is intended to be compatible with const, but is not const currently because
    // E::from_u64 is not const, as const functions on traits is not stable.
    pub fn encode<E: Elem>(&self) -> [E; 16] {
        let mut elems = [E::ZERO; 16];
        for (i, elem) in elems.iter_mut().enumerate().take(self.0.len()) {
            *elem = E::from_u64(self.0[i] as u64);
        }
        elems
    }
}

impl fmt::Display for ProtocolInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match from_utf8(&self.0) {
            Ok(s) => write!(f, "{s}"),
            Err(_) => write!(f, "0x{}", hex::encode(self.0)),
        }
    }
}

/// Versioned info string for the proof system.
///
/// NOTE: This string should be bumped with every change to the proof system, as defined by a
/// change to checks applied by the verifier.
pub const PROOF_SYSTEM_INFO: ProtocolInfo = ProtocolInfo(*b"RISC0_STARK:v1__");

pub trait CircuitInfo {
    const CIRCUIT_INFO: ProtocolInfo;
    const OUTPUT_SIZE: usize;
    const MIX_SIZE: usize;
}

/// traits implemented by generated rust code used in both prover and verifier
pub trait CircuitCoreDef<F: Field>: CircuitInfo + PolyExt<F> + TapsProvider {}

pub type Arg = usize;
pub type Var = usize;

pub struct PolyExtStepDef {
    pub block: &'static [PolyExtStep],
    pub ret: Var,
}

pub struct PolyExtScratch<F: Field> {
    fp_vars: Vec<F::ExtElem>,
    #[cfg(feature = "circuit_debug")]
    fp_index: Vec<usize>,
    mix_vars: Vec<MixState<F::ExtElem>>,
    #[cfg(feature = "circuit_debug")]
    mix_index: Vec<usize>,
    mix_exponents: Vec<usize>,
    mix_pows: Vec<F::ExtElem>,
    mix_pows_base: Option<F::ExtElem>,
}

impl<F: Field> Default for PolyExtScratch<F> {
    fn default() -> Self {
        Self {
            fp_vars: Vec::new(),
            #[cfg(feature = "circuit_debug")]
            fp_index: Vec::new(),
            mix_vars: Vec::new(),
            #[cfg(feature = "circuit_debug")]
            mix_index: Vec::new(),
            mix_exponents: Vec::new(),
            mix_pows: Vec::new(),
            mix_pows_base: None,
        }
    }
}

impl<F: Field> PolyExtScratch<F> {
    fn with_capacity(fp_expected: usize, mix_expected: usize) -> Self {
        Self {
            fp_vars: Vec::with_capacity(fp_expected),
            #[cfg(feature = "circuit_debug")]
            fp_index: Vec::with_capacity(fp_expected),
            mix_vars: Vec::with_capacity(mix_expected),
            #[cfg(feature = "circuit_debug")]
            mix_index: Vec::with_capacity(mix_expected),
            mix_exponents: Vec::new(),
            mix_pows: Vec::new(),
            mix_pows_base: None,
        }
    }

    fn prepare(&mut self, fp_expected: usize, mix_expected: usize) {
        self.fp_vars.clear();
        #[cfg(feature = "circuit_debug")]
        self.fp_index.clear();
        self.mix_vars.clear();
        #[cfg(feature = "circuit_debug")]
        self.mix_index.clear();

        if self.fp_vars.capacity() < fp_expected {
            self.fp_vars.reserve(fp_expected - self.fp_vars.capacity());
        }
        #[cfg(feature = "circuit_debug")]
        if self.fp_index.capacity() < fp_expected {
            self.fp_index
                .reserve(fp_expected - self.fp_index.capacity());
        }
        if self.mix_vars.capacity() < mix_expected {
            self.mix_vars
                .reserve(mix_expected - self.mix_vars.capacity());
        }
        #[cfg(feature = "circuit_debug")]
        if self.mix_index.capacity() < mix_expected {
            self.mix_index
                .reserve(mix_expected - self.mix_index.capacity());
        }
    }

    fn prepare_mix_pows(&mut self, mix: F::ExtElem) {
        if self.mix_pows_base == Some(mix) {
            return;
        }

        let max_exp = self.mix_exponents.iter().copied().max().unwrap_or(0);
        self.mix_pows.clear();
        self.mix_pows.reserve(max_exp + 1);

        let mut cur = F::ExtElem::ONE;
        for _ in 0..=max_exp {
            self.mix_pows.push(cur);
            cur *= mix;
        }
        self.mix_pows_base = Some(mix);
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum PolyExtStep {
    Const(u32),
    ConstExt(u32, u32, u32, u32),
    Get(usize),
    GetGlobal(Arg, usize),
    Add(Var, Var),
    Sub(Var, Var),
    Mul(Var, Var),
    True,
    AndEqz(Var, Var),
    AndCond(Var, Var, Var),
}

impl PolyExtStepDef {
    pub fn step<F: Field>(
        &self,
        mix: &F::ExtElem,
        u: &[F::ExtElem],
        args: &[&[F::Elem]],
    ) -> MixState<F::ExtElem> {
        let mut scratch = self.scratch::<F>();
        self.step_with_scratch(&mut scratch, mix, u, args)
    }

    pub fn scratch<F: Field>(&self) -> PolyExtScratch<F> {
        let fp_expected = self.fp_expected();
        let mix_expected = self.mix_expected();
        let mut scratch = PolyExtScratch::with_capacity(fp_expected, mix_expected);
        scratch.mix_exponents = self.mix_exponents();
        scratch
    }

    pub fn step_with_scratch<F: Field>(
        &self,
        scratch: &mut PolyExtScratch<F>,
        mix: &F::ExtElem,
        u: &[F::ExtElem],
        args: &[&[F::Elem]],
    ) -> MixState<F::ExtElem> {
        PolyExtExecutor::<F>::new(self, scratch).run(mix, u, args)
    }

    fn fp_expected(&self) -> usize {
        self.block.len() - self.mix_expected()
    }

    fn mix_expected(&self) -> usize {
        self.ret + 1
    }

    fn mix_exponents(&self) -> Vec<usize> {
        let mut exponents = Vec::with_capacity(self.mix_expected());
        for op in self.block {
            match op {
                PolyExtStep::True => exponents.push(0),
                PolyExtStep::AndEqz(chain, _) => exponents.push(exponents[*chain] + 1),
                PolyExtStep::AndCond(chain, _, inner) => {
                    exponents.push(exponents[*chain] + exponents[*inner])
                }
                _ => {}
            }
        }
        debug_assert_eq!(exponents.len(), self.mix_expected());
        exponents
    }
}

struct PolyExtExecutor<'a, 'scratch, F: Field> {
    def: &'a PolyExtStepDef,
    fp_expected: usize,
    mix_expected: usize,
    scratch: &'scratch mut PolyExtScratch<F>,
}

impl<'a, 'scratch, F: Field> PolyExtExecutor<'a, 'scratch, F> {
    pub fn new(def: &'a PolyExtStepDef, scratch: &'scratch mut PolyExtScratch<F>) -> Self {
        let fp_expected = def.fp_expected();
        let mix_expected = def.mix_expected();
        if scratch.mix_exponents.is_empty() {
            scratch.mix_exponents = def.mix_exponents();
        }
        scratch.prepare(fp_expected, mix_expected);
        Self {
            def,
            fp_expected,
            mix_expected,
            scratch,
        }
    }

    pub fn run(
        &mut self,
        mix: &F::ExtElem,
        u: &[F::ExtElem],
        args: &[&[F::Elem]],
    ) -> MixState<F::ExtElem> {
        self.scratch.prepare_mix_pows(*mix);
        for (idx, op) in self.def.block.iter().enumerate() {
            self.step(idx, op, mix, u, args);
        }
        assert_eq!(
            self.scratch.fp_vars.len(),
            self.fp_expected,
            "Miscalculated capacity for fp_vars"
        );
        assert_eq!(
            self.scratch.mix_vars.len(),
            self.mix_expected,
            "Miscalculated capacity for mix_vars"
        );

        #[cfg(feature = "circuit_debug")]
        self.debug(self.def.ret);

        self.scratch.mix_vars[self.def.ret]
    }

    #[cfg(feature = "circuit_debug")]
    fn debug(&mut self, next: Var) {
        let op_index = self.scratch.mix_index[next];
        let op = &self.def.block[op_index];
        tracing::debug!("chain: [m:{next}] {op:?}");
        match op {
            PolyExtStep::True => {
                tracing::debug!("PolyExtStep::True");
            }
            PolyExtStep::AndEqz(chain, inner) => {
                let inner_val = self.scratch.fp_vars[*inner];
                // inner should be zero
                if inner_val != F::ExtElem::ZERO {
                    // this is the first expression that is broken
                    tracing::debug!("expr: {}", self.debug_expr(*inner));
                    let inner_idx = self.scratch.fp_index[*inner];
                    let op = &self.def.block[inner_idx];
                    panic!("eqz failure: [f:{}] {op:?}", *inner);
                }
                self.debug(*chain);
            }
            PolyExtStep::AndCond(chain, cond, inner) => {
                let cond = self.scratch.fp_vars[*cond];
                if cond != F::ExtElem::ZERO {
                    tracing::debug!("true conditional");
                    // conditional is true
                    let inner_val = self.scratch.fp_vars[*inner];
                    // inner should be zero
                    if inner_val != F::ExtElem::ZERO {
                        tracing::debug!("inner != 0");
                        // follow inner to find out where it went bad
                        self.debug(*inner);
                    } else {
                        // follow chain
                        self.debug(*chain)
                    }
                } else {
                    // conditional is false, follow chain
                    self.debug(*chain)
                }
            }
            _ => unreachable!(),
        }
    }

    #[cfg(feature = "circuit_debug")]
    fn debug_expr(&self, next: Var) -> String {
        let op_index = self.scratch.fp_index[next];
        let op = &self.def.block[op_index];
        match op {
            PolyExtStep::Const(x) => format!("{x:?}"),
            PolyExtStep::ConstExt(x0, x1, x2, x3) => format!("({x0:?}, {x1:?}, {x2:?}, {x3:?})"),
            PolyExtStep::Get(x) => format!("Get({x})"),
            PolyExtStep::GetGlobal(arg, x) => format!("GetGlobal({arg}, {x})"),
            PolyExtStep::Add(x, y) => {
                format!("({} + {})", self.debug_expr(*x), self.debug_expr(*y))
            }
            PolyExtStep::Sub(x, y) => {
                format!("({} - {})", self.debug_expr(*x), self.debug_expr(*y))
            }
            PolyExtStep::Mul(x, y) => {
                format!("({} * {})", self.debug_expr(*x), self.debug_expr(*y))
            }
            _ => String::new(),
        }
    }

    fn fp_index(&self) -> usize {
        self.scratch.fp_vars.len()
    }

    fn mix_index(&self) -> usize {
        self.scratch.mix_vars.len()
    }

    fn push_fp(&mut self, _idx: usize, val: F::ExtElem) {
        #[cfg(feature = "circuit_debug")]
        self.scratch.fp_index.push(_idx);
        self.scratch.fp_vars.push(val);
    }

    fn push_mix(&mut self, _idx: usize, mix: MixState<F::ExtElem>) {
        #[cfg(feature = "circuit_debug")]
        self.scratch.mix_index.push(_idx);
        self.scratch.mix_vars.push(mix);
    }

    fn mix_power_for(&self, mix_idx: usize) -> F::ExtElem {
        self.scratch.mix_pows[self.scratch.mix_exponents[mix_idx]]
    }

    fn step(
        &mut self,
        idx: usize,
        op: &PolyExtStep,
        _mix: &F::ExtElem,
        u: &[F::ExtElem],
        args: &[&[F::Elem]],
    ) {
        match op {
            PolyExtStep::Const(value) => {
                let val = F::Elem::from_u64(*value as u64);
                trace_if_enabled!("[f:{}] {op:?} -> {val:?}", self.fp_index());
                self.push_fp(idx, val.into());
            }
            PolyExtStep::ConstExt(x0, x1, x2, x3) => {
                let val = F::ExtElem::from_subelems([
                    F::Elem::from_u64(*x0 as u64),
                    F::Elem::from_u64(*x1 as u64),
                    F::Elem::from_u64(*x2 as u64),
                    F::Elem::from_u64(*x3 as u64),
                ]);
                trace_if_enabled!("[f:{}] {op:?} -> {val:?}", self.fp_index());
                self.push_fp(idx, val);
            }
            PolyExtStep::Get(tap) => {
                let val = u[*tap];
                trace_if_enabled!("[f:{}] {op:?} -> {val:?}", self.fp_index());
                self.push_fp(idx, val);
            }
            PolyExtStep::GetGlobal(base, offset) => {
                let val = F::ExtElem::from_subfield(&args[*base][*offset]);
                trace_if_enabled!("[f:{}] {op:?} -> {val:?}", self.fp_index());
                self.push_fp(idx, val);
            }
            PolyExtStep::Add(x1, x2) => {
                let val = self.scratch.fp_vars[*x1] + self.scratch.fp_vars[*x2];
                trace_if_enabled!("[f:{}] {op:?} -> {val:?}", self.fp_index());
                self.push_fp(idx, val);
            }
            PolyExtStep::Sub(x1, x2) => {
                let val = self.scratch.fp_vars[*x1] - self.scratch.fp_vars[*x2];
                trace_if_enabled!("[f:{}] {op:?} -> {val:?}", self.fp_index());
                self.push_fp(idx, val);
            }
            PolyExtStep::Mul(x1, x2) => {
                let val = self.scratch.fp_vars[*x1] * self.scratch.fp_vars[*x2];
                trace_if_enabled!("[f:{}] {op:?} -> {val:?}", self.fp_index());
                self.push_fp(idx, val);
            }
            PolyExtStep::True => {
                let mix_val = MixState {
                    tot: F::ExtElem::ZERO,
                    mul: self.mix_power_for(self.mix_index()),
                };
                trace_if_enabled!("[m:{}] {op:?}", self.mix_index());
                self.push_mix(idx, mix_val);
            }
            PolyExtStep::AndEqz(chain, inner) => {
                let chain = self.scratch.mix_vars[*chain];
                let inner = self.scratch.fp_vars[*inner];
                let mix_val = MixState {
                    tot: chain.tot + chain.mul * inner,
                    mul: self.mix_power_for(self.mix_index()),
                };
                trace_if_enabled!("[m:{}] {op:?}, inner: {inner:?}", self.mix_index());
                self.push_mix(idx, mix_val);
            }
            PolyExtStep::AndCond(chain, cond, inner) => {
                let chain = self.scratch.mix_vars[*chain];
                let cond = self.scratch.fp_vars[*cond];
                let inner = self.scratch.mix_vars[*inner];
                let mix_val = MixState {
                    tot: chain.tot + cond * inner.tot * chain.mul,
                    mul: self.mix_power_for(self.mix_index()),
                };
                trace_if_enabled!(
                    "[m:{}] {op:?}, cond: {}, inner: {}",
                    self.mix_index(),
                    cond != F::ExtElem::ZERO,
                    inner.tot != F::ExtElem::ZERO,
                );
                self.push_mix(idx, mix_val);
            }
        }
    }
}
