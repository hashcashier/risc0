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

mod hal;
mod preflight;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub mod testutil;
pub mod zkr;

use std::rc::Rc;
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
use std::{cell::RefCell, future::Future, pin::Pin};

use anyhow::Result;
use cfg_if::cfg_if;
use risc0_core::{field::Elem, scope};
use risc0_zkp::{
    adapter::{CircuitInfo as _, PROOF_SYSTEM_INFO},
    core::{digest::Digest, hash::poseidon2::Poseidon2HashSuite},
    hal::{Buffer, CircuitHal, Hal},
};

use self::{
    hal::{CircuitWitnessGenerator, MetaBuffer, StepMode},
    preflight::PreflightTrace,
};
use crate::{
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

const GLOBAL_MIX: usize = 0;
const GLOBAL_OUT: usize = 1;

pub type Seal = Vec<u32>;

pub trait KeccakProver {
    fn prove(&self, inputs: &[KeccakState], po2: usize) -> Result<Seal>;

    #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
    fn prove_async<'a>(
        &'a self,
        inputs: &'a [KeccakState],
        po2: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Seal>> + 'a>> {
        Box::pin(async move { self.prove(inputs, po2) })
    }

    fn verify(&self, seal: &Seal) -> Result<()> {
        let hash_suite = Poseidon2HashSuite::new_suite();

        // We don't have a `code' buffer to verify.
        let check_code_fn = |_: u32, _: &Digest| Ok(());

        Ok(risc0_zkp::verify::verify(
            &CircuitImpl,
            &hash_suite,
            seal,
            check_code_fn,
        )?)
    }
}

pub fn keccak_prover() -> Result<Box<dyn KeccakProver>> {
    cfg_if! {
        if #[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))] {
            if let Some(hal) = current_webgpu_hal() {
                self::hal::webgpu::keccak_prover(hal)
            } else {
                anyhow::bail!("keccak_prover cannot synchronously initialize browser WebGPU")
            }
        } else if #[cfg(feature = "cuda")] {
            self::hal::cuda::keccak_prover()
        // } else if #[cfg(any(all(target_os = "macos", target_arch = "aarch64"), target_os = "ios"))] {
        //     self::metal::keccak_prover()
        } else {
            self::hal::cpu::keccak_prover()
        }
    }
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub fn keccak_prover_with_hal(
    hal: Rc<risc0_zkp::hal::webgpu::WebGpuHal>,
) -> Result<Box<dyn KeccakProver>> {
    self::hal::webgpu::keccak_prover(hal)
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
thread_local! {
    static WEBGPU_HAL: RefCell<Option<Rc<risc0_zkp::hal::webgpu::WebGpuHal>>> = RefCell::new(None);
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub fn with_webgpu_hal<T>(hal: Rc<risc0_zkp::hal::webgpu::WebGpuHal>, f: impl FnOnce() -> T) -> T {
    struct Reset(Option<Rc<risc0_zkp::hal::webgpu::WebGpuHal>>);

    impl Drop for Reset {
        fn drop(&mut self) {
            WEBGPU_HAL.with(|slot| {
                slot.replace(self.0.take());
            });
        }
    }

    WEBGPU_HAL.with(|slot| {
        let previous = slot.replace(Some(hal));
        let _reset = Reset(previous);
        f()
    })
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
fn current_webgpu_hal() -> Option<Rc<risc0_zkp::hal::webgpu::WebGpuHal>> {
    WEBGPU_HAL.with(|slot| slot.borrow().clone())
}

pub(crate) struct KeccakProverImpl<H, C>
where
    H: Hal<Field = CircuitField, Elem = Val, ExtElem = ExtVal>,
    C: CircuitHal<H> + CircuitWitnessGenerator<H>,
{
    hal: Rc<H>,
    circuit_hal: Rc<C>,
}

impl<H, C> KeccakProver for KeccakProverImpl<H, C>
where
    H: Hal<Field = CircuitField, Elem = Val, ExtElem = ExtVal>,
    C: CircuitHal<H> + CircuitWitnessGenerator<H>,
{
    fn prove(&self, inputs: &[KeccakState], po2: usize) -> Result<Seal> {
        scope!("prove");

        let cycles: usize = 1 << po2;
        let preflight = PreflightTrace::<C::PreferredPreflightOrder>::new(inputs, cycles);

        let mut global = vec![Val::INVALID; REGCOUNT_GLOBAL];
        global[LAYOUT_GLOBAL.total_cycles._super.offset] = Elem::from_u64(1 << po2);

        let global = MetaBuffer {
            buf: self.hal.copy_from_elem("global", &global),
            rows: 1,
            cols: REGCOUNT_GLOBAL,
            checked_reads: true,
        };
        let code = MetaBuffer::new_zeroed("code", self.hal.as_ref(), cycles, REGCOUNT_CODE, true);
        let data = scope!(
            "alloc(data)",
            MetaBuffer::new("data", self.hal.as_ref(), cycles, REGCOUNT_DATA, true)
        );

        self.circuit_hal
            .scatter_preflight(&data, &preflight.scatter, &preflight.data)?;

        self.circuit_hal
            .generate_witness(StepMode::Parallel, &preflight, &global, &data)?;

        // Zero out invalid data entries. The code group is allocated zeroed.
        scope!("zeroize", {
            self.hal.eltwise_zeroize_elem(&data.buf);
        });

        let mut prover = risc0_zkp::prove::Prover::new(self.hal.as_ref(), TAPSET);

        // At the start of the protocol, seed the Fiat-Shamir transcript with context information
        // about the proof system and circuit.
        let hashfn = &self.hal.get_hash_suite().hashfn;
        prover
            .iop()
            .commit(&hashfn.hash_elem_slice(&PROOF_SYSTEM_INFO.encode()));
        prover
            .iop()
            .commit(&hashfn.hash_elem_slice(&CircuitImpl::CIRCUIT_INFO.encode()));

        // Concat io (i.e. globals) and po2 into a vector.
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
                .chain(Elem::from_u32_slice(&[po2 as u32]))
                .copied()
                .collect();

            let header_digest = hashfn.hash_elem_slice(&header);
            prover.iop().commit(&header_digest);
            prover.iop().write_field_elem_slice(&header);
        });

        prover.set_po2(po2);

        prover.commit_group(REGISTER_GROUP_CODE, &code.buf);
        prover.commit_group(REGISTER_GROUP_DATA, &data.buf);

        let mix: [Val; REGCOUNT_MIX] = std::array::from_fn(|_| prover.iop().random_elem());
        let mix = self.hal.copy_from_elem("mix", mix.as_slice());

        let accum = self
            .hal
            .alloc_elem_init("accum", cycles * REGCOUNT_ACCUM, H::Elem::ZERO);
        prover.commit_group(REGISTER_GROUP_ACCUM, &accum);

        let seal = prover.finalize(&[&mix, &global.buf], self.circuit_hal.as_ref());

        Ok(seal)
    }
}
