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

//! Portable circuit-HAL helpers.

use risc0_core::field::{Elem as _, ExtElem as _, RootsOfUnity as _};

use super::{Buffer as _, Hal};
use crate::{adapter::CircuitCoreDef, core::log2_ceil, INV_RATE};

/// Compute the validity/check polynomial from generated Rust circuit metadata.
///
/// This mirrors the native circuit kernels by evaluating each tap over the
/// expanded domain, invoking the generated `poly_ext` constraint program, and
/// dividing by the zerofier used by the prover.
pub fn eval_check<H, C>(
    circuit: &C,
    check: &H::Buffer<H::Elem>,
    groups: &[&H::Buffer<H::Elem>],
    globals: &[&H::Buffer<H::Elem>],
    poly_mix: H::ExtElem,
    po2: usize,
    steps: usize,
) where
    H: Hal,
    C: CircuitCoreDef<H::Field>,
{
    let taps = circuit.get_taps();
    let domain = steps * INV_RATE;
    let ext_size = H::ExtElem::EXT_SIZE;
    assert_eq!(check.size(), ext_size * domain);
    assert_eq!(groups.len(), taps.num_groups());

    let group_values: Vec<_> = groups.iter().map(|group| group.to_vec()).collect();
    for (group_id, group) in group_values.iter().enumerate() {
        assert_eq!(group.len(), taps.group_size(group_id) * domain);
    }

    let global_values: Vec<_> = globals.iter().map(|global| global.to_vec()).collect();
    let global_refs: Vec<_> = match global_values.as_slice() {
        // CircuitHal convention is [mix, out]. Generated poly_ext programs
        // reference globals as [out, mix].
        [mix, out] => vec![out.as_slice(), mix.as_slice()],
        _ => global_values.iter().map(Vec::as_slice).collect(),
    };

    let exp_po2 = log2_ceil(INV_RATE);
    let rou = H::Elem::ROU_FWD[po2 + exp_po2];
    let three = H::Elem::from_u64(3);
    let mut tap_values = vec![H::ExtElem::ZERO; taps.tap_size()];

    check.view_mut(|check| {
        for cycle in 0..domain {
            for (tap_idx, tap) in taps.taps().enumerate() {
                let back = (tap.back() * INV_RATE) % domain;
                let row = (cycle + domain - back) % domain;
                let group = &group_values[tap.group()];
                let value = group[tap.offset() * domain + row];
                tap_values[tap_idx] = H::ExtElem::from_subfield(&value);
            }

            let total = circuit
                .poly_ext(&poly_mix, &tap_values, global_refs.as_slice())
                .tot;
            let x = rou.pow(cycle);
            let zerofier = (three * x).pow(steps) - H::Elem::ONE;
            let result = total * zerofier.inv();
            for (idx, elem) in result.subelems().iter().enumerate() {
                check[idx * domain + cycle] = *elem;
            }
        }
    });
}
