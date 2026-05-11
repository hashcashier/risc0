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

use risc0_core::{
    field::{Elem, ExtElem, RootsOfUnity},
    scope, scope_with,
};

use crate::{
    core::poly::poly_interpolate,
    hal::{Buffer, CircuitHal, Hal},
    prove::{fri::fri_prove, poly_group::PolyGroup, write_iop::WriteIOP},
    taps::TapSet,
    INV_RATE,
};

/// Object to generate a zero-knowledge proof of the execution of some circuit.
pub struct Prover<'a, H: Hal> {
    hal: &'a H,
    taps: &'a TapSet<'a>,
    iop: WriteIOP<H::Field>,
    groups: Vec<Option<PolyGroup<H>>>,
    cycles: usize,
    po2: usize,
}

fn make_coeffs<H: Hal>(hal: &H, witness: &H::Buffer<H::Elem>, count: usize) -> H::Buffer<H::Elem> {
    scope!("make_coeffs");
    let coeffs = hal.alloc_elem("coeffs", witness.size());
    hal.eltwise_copy_elem(&coeffs, witness);
    // Do interpolate
    hal.batch_interpolate_ntt(&coeffs, count);
    // Convert f(x) -> f(3x), which effective multiplies coefficients c_i by 3^i.
    #[cfg(not(feature = "circuit_debug"))]
    hal.zk_shift(&coeffs, count);
    coeffs
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
async fn make_coeffs_async(
    hal: &crate::hal::webgpu::WebGpuHal,
    witness: &crate::hal::webgpu::WebGpuBuffer<risc0_core::field::baby_bear::BabyBearElem>,
    count: usize,
) -> anyhow::Result<crate::hal::webgpu::WebGpuBuffer<risc0_core::field::baby_bear::BabyBearElem>> {
    scope!("make_coeffs");
    let coeffs = hal.alloc_elem("coeffs", witness.size());
    if hal.can_dispatch_batch_interpolate_ntt(&coeffs) && hal.can_dispatch_zk_shift(&coeffs) {
        hal.eltwise_copy_elem(&coeffs, witness);
    } else {
        // Large recursion groups can exceed WebGPU's safe storage binding
        // size. Keep the CPU mirror current for the initial witness copy so
        // the CPU NTT fallback does not have to read back raw witness cells,
        // which may still contain invalid sentinels before interpolation.
        let _cpu_mirror_scope = hal.gpu_authoritative_scope(false);
        hal.eltwise_copy_elem(&coeffs, witness);
    }
    hal.batch_interpolate_ntt_async(&coeffs, count).await?;
    #[cfg(not(feature = "circuit_debug"))]
    hal.zk_shift_async(&coeffs, count).await?;
    Ok(coeffs)
}

impl<'a, H: Hal> Prover<'a, H> {
    /// Creates a new prover.
    pub fn new(hal: &'a H, taps: &'a TapSet) -> Self {
        Self {
            hal,
            taps,
            iop: WriteIOP::new(hal.get_hash_suite().rng.as_ref()),
            groups: std::iter::repeat_with(|| None)
                .take(taps.num_groups())
                .collect(),
            cycles: 0,
            po2: usize::MAX,
        }
    }

    /// Accesses the prover's IOP to commit or read random data.
    pub fn iop(&mut self) -> &mut WriteIOP<H::Field> {
        &mut self.iop
    }

    /// Sets the number of cycles to 2^po2.  This must be called
    /// once after new() before any commit_group() calls.
    pub fn set_po2(&mut self, po2: usize) {
        assert_eq!(self.po2, usize::MAX);
        assert_eq!(self.cycles, 0);
        self.po2 = po2;
        self.cycles = 1 << po2;
    }

    /// Commits a given buffer to the IOP; the values must not subsequently
    /// change.
    pub fn commit_group(&mut self, tap_group_index: usize, witness: &H::Buffer<H::Elem>) {
        scope_with!("commit_group({})", witness.name());
        let group_size = self.taps.group_size(tap_group_index);
        assert_eq!(witness.size() % group_size, 0);
        assert_eq!(witness.size() / group_size, self.cycles);
        assert!(
            self.groups[tap_group_index].is_none(),
            "Attempted to commit group {} more than once",
            self.taps.group_name(tap_group_index)
        );

        let coeffs = make_coeffs(self.hal, witness, group_size);
        let group_ref = self.groups[tap_group_index].insert(PolyGroup::new(
            self.hal,
            coeffs,
            group_size,
            self.cycles,
            witness.name(),
        ));

        group_ref.merkle.commit(&mut self.iop);

        tracing::debug!(
            "{} group root: {}",
            self.taps.group_name(tap_group_index),
            group_ref.merkle.root()
        );
    }

    /// Generates the proof and returns the seal.
    pub fn finalize<C>(mut self, globals: &[&H::Buffer<H::Elem>], circuit_hal: &C) -> Vec<u32>
    where
        C: CircuitHal<H>,
    {
        scope!("finalize");

        // Set the poly mix value, which is used for constraint compression in the
        // DEEP-ALI protocol.
        let poly_mix = self.iop.random_ext_elem();
        let domain = self.cycles * INV_RATE;
        let ext_size = H::ExtElem::EXT_SIZE;

        // Now generate the check polynomial.
        // The check polynomial is the core of the STARK: if the constraints are
        // satisfied, the check polynomial will be a low-degree polynomial. See
        // DEEP-ALI paper for details on the construction of the check_poly.
        let check_poly = self.hal.alloc_elem("check_poly", ext_size * domain);

        let groups: Vec<&_> = self
            .groups
            .iter()
            .map(|pg| &pg.as_ref().unwrap().evaluated)
            .collect();
        circuit_hal.eval_check(
            &check_poly,
            groups.as_slice(),
            globals,
            poly_mix,
            self.po2,
            self.cycles,
        );

        #[cfg(feature = "circuit_debug")]
        let mut bad_z = None;

        #[cfg(feature = "circuit_debug")]
        check_poly.view(|check_out| {
            for i in (0..domain).step_by(4) {
                if check_out[i] != H::Elem::ZERO {
                    tracing::debug!("check[{i}] = 0x{:08x?}", check_out[i].to_u32_words()[0]);
                    bad_z.get_or_insert(H::ExtElem::from_subfield(
                        &H::Elem::ROU_FWD[self.po2].pow(i / 4),
                    ));
                }
            }
            // assert!(bad_z.is_none());
        });

        // Convert to coefficients.  Some tricky business here with the fact that
        // checkPoly is really an FpExt polynomial.  Nicely for us, since all the
        // roots of unity (which are the only thing that and values get multiplied
        // by) are in Fp, FpExt values act like simple vectors of Fp for the
        // purposes of interpolate/evaluate.
        self.hal.batch_interpolate_ntt(&check_poly, ext_size);

        // The next step is to convert the degree 4*n check polynomial into 4 degree n
        // polynomials so that f(x) = g0(x^4) + g1(x^4) x + g2(x^4) x^2 + g3(x^4)
        // x^3.  To do this, we normally would grab all the coefficients of f(x) =
        // sum_i c_i x^i where i % 4 == 0 and put them into a new polynomial g0(x) =
        // sum_i d0_i*x^i, where d0_i = c_(i*4).
        //
        // Amazingly, since the coefficients are bit reversed, the coefficients of g0
        // are all already next to each other and in bit-reversed for g0, as are
        // the coefficients of g1, etc. So really, we can just reinterpret 4 polys of
        // invRate*size to 16 polys of size, without actually doing anything.

        // Make the PolyGroup + add it to the IOP;
        let check_group = PolyGroup::new(self.hal, check_poly, H::CHECK_SIZE, self.cycles, "check");
        check_group.merkle.commit(&mut self.iop);
        tracing::debug!("checkGroup: {}", check_group.merkle.root());

        // Now pick a value for Z, which is used as the DEEP-ALI query point.
        cfg_if::cfg_if! {
            if #[cfg(feature = "circuit_debug")] {
                let z = if let Some(bad_z) = bad_z {
                    self.iop.write_field_elem_slice(bad_z.subelems());
                    bad_z
                } else {
                    self.iop.random_ext_elem()
                };
            } else {
                let z = self.iop.random_ext_elem();
            }
        }
        tracing::debug!("Z = {z:?}");

        // Get rev rou for size
        let back_one = H::ExtElem::from_subfield(&H::Elem::ROU_REV[self.po2]);
        let mut all_xs = Vec::new();

        // Now, we evaluate each group at the appropriate points (relative to Z).
        // From here on out, we always process groups in accum, code, data order,
        // since this is the order used by the codegen system (alphabetical).
        // Sometimes it's a requirement for matching generated code, but even when
        // it's not we keep the order for consistency.

        let mut eval_u: Vec<H::ExtElem> = Vec::new();
        scope!("eval_u", {
            for (id, pg) in self.groups.iter().enumerate() {
                let pg = pg.as_ref().unwrap();

                let mut which = Vec::new();
                let mut xs = Vec::new();
                for tap in self.taps.group_taps(id) {
                    which.push(tap.offset() as u32);
                    let x = back_one.pow(tap.back()) * z;
                    xs.push(x);
                    all_xs.push(x);
                }
                let which = self.hal.copy_from_u32("which", which.as_slice());
                let xs = self.hal.copy_from_extelem("xs", xs.as_slice());
                let out = self.hal.alloc_extelem("out", which.size());
                self.hal
                    .batch_evaluate_any(&pg.coeffs, pg.count, &which, &xs, &out);
                out.view(|view| {
                    eval_u.extend(view);
                });
            }
        });

        // Now, convert the values to coefficients via interpolation
        let mut coeff_u = vec![H::ExtElem::ZERO; eval_u.len()];
        scope!("poly_interpolate", {
            let mut pos = 0;
            for reg in self.taps.regs() {
                poly_interpolate(
                    &mut coeff_u[pos..],
                    &all_xs[pos..],
                    &eval_u[pos..],
                    reg.size(),
                );
                pos += reg.size();
            }
        });

        // Add in the coeffs of the check polynomials.
        let z_pow = z.pow(ext_size);
        scope!("misc", {
            let which = Vec::from_iter(0u32..H::CHECK_SIZE as u32);
            let xs = vec![z_pow; H::CHECK_SIZE];
            let out = self.hal.alloc_extelem("out", H::CHECK_SIZE);
            let which = self.hal.copy_from_u32("which", which.as_slice());
            let xs = self.hal.copy_from_extelem("xs", xs.as_slice());
            self.hal
                .batch_evaluate_any(&check_group.coeffs, H::CHECK_SIZE, &which, &xs, &out);
            out.view(|view| {
                coeff_u.extend(view);
            });

            tracing::debug!("Size of U = {}", coeff_u.len());
            self.iop.write_field_elem_slice(&coeff_u);
            let hash_u = self
                .hal
                .get_hash_suite()
                .hashfn
                .hash_ext_elem_slice(coeff_u.as_slice());
            self.iop.commit(&hash_u);

            // Set the mix value, which is used for FRI batching.
        });

        let mix = self.iop.random_ext_elem();
        tracing::debug!("Mix = {mix:?}");

        // Do the coefficient mixing
        // Begin by making a zeroed output buffer
        let combo_count = self.taps.combos_size();
        let combos = scope!(
            "alloc(combos)",
            self.hal
                .alloc_extelem_zeroed("combos", self.cycles * (combo_count + 1))
        );

        scope!("mix_poly_coeffs", {
            let mut cur_mix = H::ExtElem::ONE;

            for (id, pg) in self.groups.iter().enumerate() {
                let pg = pg.as_ref().unwrap();

                let group_size = self.taps.group_size(id);
                let mut which = Vec::with_capacity(group_size);
                for reg in self.taps.group_regs(id) {
                    which.push(reg.combo_id() as u32);
                }
                let which = self.hal.copy_from_u32("which", which.as_slice());
                self.hal.mix_poly_coeffs(
                    &combos,
                    &cur_mix,
                    &mix,
                    &pg.coeffs,
                    &which,
                    group_size,
                    self.cycles,
                );
                cur_mix *= mix.pow(group_size);
            }

            let which = vec![combo_count as u32; H::CHECK_SIZE];
            let which_buf = self.hal.copy_from_u32("which", which.as_slice());
            self.hal.mix_poly_coeffs(
                &combos,
                &cur_mix,
                &mix,
                &check_group.coeffs,
                &which_buf,
                H::CHECK_SIZE,
                self.cycles,
            );
        });

        scope!("load_combos", {
            let reg_sizes: Vec<_> = self.taps.regs().map(|x| x.size() as u32).collect();
            let reg_combo_ids: Vec<_> = self.taps.regs().map(|x| x.combo_id() as u32).collect();

            scope!("prepare", {
                self.hal.combos_prepare(
                    &combos,
                    &coeff_u,
                    combo_count,
                    self.cycles,
                    &reg_sizes,
                    &reg_combo_ids,
                    &mix,
                );
            });

            scope!("divide", {
                let mut chunks = vec![];

                // Divide each element by (x - Z * back1^back) for each back
                for i in 0..combo_count {
                    let mut pows = vec![];
                    for &back in self.taps.get_combo(i).slice() {
                        pows.push(z * back_one.pow(back.into()));
                    }
                    chunks.push((i, pows));
                }

                // Divide check polys by z^EXT_SIZE
                chunks.push((combo_count, vec![z_pow]));

                self.hal.combos_divide(&combos, chunks, self.cycles);
            });
        });

        // Sum the combos up into one final polynomial + make it into 4 Fp polys.
        // Additionally, it needs to be bit reversed to make everyone happy
        let final_poly_coeffs = scope!("sum", {
            let final_poly_coeffs = self
                .hal
                .alloc_elem("final_poly_coeffs", self.cycles * ext_size);
            self.hal.eltwise_sum_extelem(&final_poly_coeffs, &combos);
            final_poly_coeffs
        });

        // Finally do the FRI protocol to prove the degree of the polynomial
        scope!(
            "bit_rev",
            self.hal.batch_bit_reverse(&final_poly_coeffs, ext_size)
        );
        tracing::debug!("FRI-proof, size = {}", final_poly_coeffs.size() / ext_size);

        fri_prove(self.hal, &mut self.iop, &final_poly_coeffs, |iop, idx| {
            for pg in self.groups.iter() {
                let pg = pg.as_ref().unwrap();
                pg.merkle.prove(self.hal, iop, idx);
            }
            check_group.merkle.prove(self.hal, iop, idx);
        });

        let proven_soundness_error =
            super::soundness::proven::<H>(self.taps, final_poly_coeffs.size());
        tracing::debug!("proven_soundness_error: {proven_soundness_error:?}");

        let conjectured_security =
            super::soundness::toy_model_security::<H>(self.taps, final_poly_coeffs.size());
        tracing::debug!("conjectured_security: {conjectured_security:?}");

        // Return final proof
        let proof = self.iop.proof;
        tracing::debug!("Proof size = {}", proof.len());
        proof
    }
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
impl<'a> Prover<'a, crate::hal::webgpu::WebGpuHal> {
    /// Async WebGPU variant of [`Self::commit_group`].
    pub async fn commit_group_async(
        &mut self,
        tap_group_index: usize,
        witness: &crate::hal::webgpu::WebGpuBuffer<risc0_core::field::baby_bear::BabyBearElem>,
    ) -> anyhow::Result<()> {
        let authoritative = self.hal.gpu_authoritative();
        self.commit_group_async_scoped(
            tap_group_index,
            witness,
            authoritative,
            authoritative,
            authoritative,
        )
        .await
    }

    /// Async WebGPU variant of [`Self::commit_group`] with diagnostic ownership
    /// controls for the three commit sub-stages.
    #[doc(hidden)]
    pub async fn commit_group_async_scoped(
        &mut self,
        tap_group_index: usize,
        witness: &crate::hal::webgpu::WebGpuBuffer<risc0_core::field::baby_bear::BabyBearElem>,
        make_coeffs_authoritative: bool,
        poly_group_authoritative: bool,
        merkle_authoritative: bool,
    ) -> anyhow::Result<()> {
        scope_with!("commit_group({})", witness.name());
        let group_size = self.taps.group_size(tap_group_index);
        assert_eq!(witness.size() % group_size, 0);
        assert_eq!(witness.size() / group_size, self.cycles);
        assert!(
            self.groups[tap_group_index].is_none(),
            "Attempted to commit group {} more than once",
            self.taps.group_name(tap_group_index)
        );

        let coeffs = {
            let _gpu_scope = self.hal.gpu_authoritative_scope(make_coeffs_authoritative);
            make_coeffs_async(self.hal, witness, group_size).await?
        };
        let group = {
            let _gpu_scope = self.hal.gpu_authoritative_scope(poly_group_authoritative);
            PolyGroup::new_async(self.hal, coeffs, group_size, self.cycles, witness.name()).await?
        };
        let group_ref = self.groups[tap_group_index].insert(group);

        {
            let _gpu_scope = self.hal.gpu_authoritative_scope(merkle_authoritative);
            group_ref
                .merkle
                .commit_async(self.hal, &mut self.iop)
                .await?;
        }

        tracing::debug!(
            "{} group root: {}",
            self.taps.group_name(tap_group_index),
            group_ref.merkle.root()
        );
        Ok(())
    }

    /// Async WebGPU variant of [`Self::finalize`].
    pub async fn finalize_async<C>(
        mut self,
        globals: &[&crate::hal::webgpu::WebGpuBuffer<risc0_core::field::baby_bear::BabyBearElem>],
        circuit_hal: &C,
    ) -> anyhow::Result<Vec<u32>>
    where
        C: CircuitHal<crate::hal::webgpu::WebGpuHal> + crate::hal::webgpu::WebGpuCircuitEvalCheck,
    {
        scope!("finalize");

        let poly_mix = self.iop.random_ext_elem();
        let domain = self.cycles * INV_RATE;
        let ext_size = <crate::hal::webgpu::WebGpuHal as Hal>::ExtElem::EXT_SIZE;
        let check_poly = self.hal.alloc_elem("check_poly", ext_size * domain);

        let groups: Vec<&_> = self
            .groups
            .iter()
            .map(|pg| &pg.as_ref().unwrap().evaluated)
            .collect();

        let eval_check_on_gpu = self.hal.gpu_authoritative()
            && circuit_hal.eval_check_webgpu(
                self.hal,
                &check_poly,
                groups.as_slice(),
                globals,
                poly_mix,
                self.po2,
                self.cycles,
            )?;
        if !eval_check_on_gpu {
            // Current circuit eval_check fallbacks are synchronous CPU portable
            // code. Keep this as the explicit compatibility barrier when a
            // circuit-specific WebGPU kernel cannot own this stage yet.
            for group in &groups {
                group.sync_gpu_to_cpu(self.hal).await?;
            }
            for global in globals {
                global.sync_gpu_to_cpu(self.hal).await?;
            }

            circuit_hal.eval_check(
                &check_poly,
                groups.as_slice(),
                globals,
                poly_mix,
                self.po2,
                self.cycles,
            );
        }

        #[cfg(feature = "circuit_debug")]
        let mut bad_z = None;

        #[cfg(feature = "circuit_debug")]
        {
            check_poly.sync_gpu_to_cpu(self.hal).await?;
            check_poly.view(|check_out| {
                for i in (0..domain).step_by(4) {
                    if check_out[i] != <crate::hal::webgpu::WebGpuHal as Hal>::Elem::ZERO {
                        tracing::debug!("check[{i}] = 0x{:08x?}", check_out[i].to_u32_words()[0]);
                        bad_z.get_or_insert(
                            <crate::hal::webgpu::WebGpuHal as Hal>::ExtElem::from_subfield(
                                &<crate::hal::webgpu::WebGpuHal as Hal>::Elem::ROU_FWD[self.po2]
                                    .pow(i / 4),
                            ),
                        );
                    }
                }
            });
        }

        {
            let _timer =
                crate::hal::webgpu::WebGpuStageTimer::new("finalize_async check_interpolate");
            self.hal
                .batch_interpolate_ntt_async(&check_poly, ext_size)
                .await?;
        }
        let check_group = {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new("finalize_async check_group");
            PolyGroup::new_async(
                self.hal,
                check_poly,
                <crate::hal::webgpu::WebGpuHal as Hal>::CHECK_SIZE,
                self.cycles,
                "check",
            )
            .await?
        };
        {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new("finalize_async check_commit");
            check_group
                .merkle
                .commit_async(self.hal, &mut self.iop)
                .await?;
        }
        tracing::debug!("checkGroup: {}", check_group.merkle.root());

        cfg_if::cfg_if! {
            if #[cfg(feature = "circuit_debug")] {
                let z = if let Some(bad_z) = bad_z {
                    self.iop.write_field_elem_slice(bad_z.subelems());
                    bad_z
                } else {
                    self.iop.random_ext_elem()
                };
            } else {
                let z = self.iop.random_ext_elem();
            }
        }
        tracing::debug!("Z = {z:?}");

        let back_one = <crate::hal::webgpu::WebGpuHal as Hal>::ExtElem::from_subfield(
            &<crate::hal::webgpu::WebGpuHal as Hal>::Elem::ROU_REV[self.po2],
        );
        let mut all_xs = Vec::new();
        let mut eval_u: Vec<<crate::hal::webgpu::WebGpuHal as Hal>::ExtElem> = Vec::new();
        {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new("finalize_async eval_u_groups");
            for (id, pg) in self.groups.iter().enumerate() {
                let pg = pg.as_ref().unwrap();

                let mut which = Vec::new();
                let mut xs = Vec::new();
                for tap in self.taps.group_taps(id) {
                    which.push(tap.offset() as u32);
                    let x = back_one.pow(tap.back()) * z;
                    xs.push(x);
                    all_xs.push(x);
                }
                let which = self.hal.copy_from_u32("which", which.as_slice());
                let xs = self.hal.copy_from_extelem("xs", xs.as_slice());
                let out = self.hal.alloc_extelem("out", which.size());
                self.hal
                    .batch_evaluate_any_async(&pg.coeffs, pg.count, &which, &xs, &out)
                    .await?;
                out.sync_gpu_to_cpu(self.hal).await?;
                out.view(|view| {
                    eval_u.extend(view);
                });
            }
        }

        let mut coeff_u = vec![<crate::hal::webgpu::WebGpuHal as Hal>::ExtElem::ZERO; eval_u.len()];
        let _timer = crate::hal::webgpu::WebGpuStageTimer::new("finalize_async poly_interpolate");
        scope!("poly_interpolate", {
            let mut pos = 0;
            for reg in self.taps.regs() {
                poly_interpolate(
                    &mut coeff_u[pos..],
                    &all_xs[pos..],
                    &eval_u[pos..],
                    reg.size(),
                );
                pos += reg.size();
            }
        });
        drop(_timer);

        let z_pow = z.pow(ext_size);
        let which = Vec::from_iter(0u32..<crate::hal::webgpu::WebGpuHal as Hal>::CHECK_SIZE as u32);
        let xs = vec![z_pow; <crate::hal::webgpu::WebGpuHal as Hal>::CHECK_SIZE];
        let out = self
            .hal
            .alloc_extelem("out", <crate::hal::webgpu::WebGpuHal as Hal>::CHECK_SIZE);
        let which = self.hal.copy_from_u32("which", which.as_slice());
        let xs = self.hal.copy_from_extelem("xs", xs.as_slice());
        {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new("finalize_async eval_u_check");
            self.hal
                .batch_evaluate_any_async(
                    &check_group.coeffs,
                    <crate::hal::webgpu::WebGpuHal as Hal>::CHECK_SIZE,
                    &which,
                    &xs,
                    &out,
                )
                .await?;
            out.sync_gpu_to_cpu(self.hal).await?;
            out.view(|view| {
                coeff_u.extend(view);
            });
        }

        tracing::debug!("Size of U = {}", coeff_u.len());
        {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new("finalize_async write_u_commit");
            self.iop.write_field_elem_slice(&coeff_u);
            let hash_u = self
                .hal
                .get_hash_suite()
                .hashfn
                .hash_ext_elem_slice(coeff_u.as_slice());
            self.iop.commit(&hash_u);
        }

        let mix = self.iop.random_ext_elem();
        tracing::debug!("Mix = {mix:?}");
        let combo_count = self.taps.combos_size();
        let combos = scope!(
            "alloc(combos)",
            self.hal
                .alloc_extelem_zeroed("combos", self.cycles * (combo_count + 1))
        );

        {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new("finalize_async coeff_sync");
            for pg in self.groups.iter() {
                pg.as_ref()
                    .unwrap()
                    .coeffs
                    .sync_gpu_to_cpu(self.hal)
                    .await?;
            }
            check_group.coeffs.sync_gpu_to_cpu(self.hal).await?;
        }

        scope!("mix_poly_coeffs", {
            let _timer =
                crate::hal::webgpu::WebGpuStageTimer::new("finalize_async mix_poly_coeffs");
            let mut cur_mix = <crate::hal::webgpu::WebGpuHal as Hal>::ExtElem::ONE;

            for (id, pg) in self.groups.iter().enumerate() {
                let pg = pg.as_ref().unwrap();

                let group_size = self.taps.group_size(id);
                let mut which = Vec::with_capacity(group_size);
                for reg in self.taps.group_regs(id) {
                    which.push(reg.combo_id() as u32);
                }
                let which = self.hal.copy_from_u32("which", which.as_slice());
                self.hal
                    .mix_poly_coeffs_async(
                        &combos,
                        &cur_mix,
                        &mix,
                        &pg.coeffs,
                        &which,
                        group_size,
                        self.cycles,
                    )
                    .await?;
                cur_mix *= mix.pow(group_size);
            }

            let which =
                vec![combo_count as u32; <crate::hal::webgpu::WebGpuHal as Hal>::CHECK_SIZE];
            let which_buf = self.hal.copy_from_u32("which", which.as_slice());
            self.hal
                .mix_poly_coeffs_async(
                    &combos,
                    &cur_mix,
                    &mix,
                    &check_group.coeffs,
                    &which_buf,
                    <crate::hal::webgpu::WebGpuHal as Hal>::CHECK_SIZE,
                    self.cycles,
                )
                .await?;
        });

        scope!("load_combos", {
            let reg_sizes: Vec<_> = self.taps.regs().map(|x| x.size() as u32).collect();
            let reg_combo_ids: Vec<_> = self.taps.regs().map(|x| x.combo_id() as u32).collect();

            scope!("prepare", {
                let _timer =
                    crate::hal::webgpu::WebGpuStageTimer::new("finalize_async combos_prepare");
                self.hal
                    .combos_prepare_async(
                        &combos,
                        &coeff_u,
                        combo_count,
                        self.cycles,
                        &reg_sizes,
                        &reg_combo_ids,
                        &mix,
                    )
                    .await?;
            });

            scope!("divide", {
                let _timer =
                    crate::hal::webgpu::WebGpuStageTimer::new("finalize_async combos_divide");
                let mut chunks = vec![];

                for i in 0..combo_count {
                    let mut pows = Vec::new();
                    for &back in self.taps.get_combo(i).slice() {
                        pows.push(z * back_one.pow(back.into()));
                    }
                    chunks.push((i, pows));
                }

                chunks.push((combo_count, vec![z_pow]));
                self.hal
                    .combos_divide_async(&combos, chunks, self.cycles)
                    .await?;
            });
        });

        let final_poly_coeffs = scope!("sum", {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new("finalize_async sum");
            let final_poly_coeffs = self
                .hal
                .alloc_elem("final_poly_coeffs", self.cycles * ext_size);
            self.hal
                .eltwise_sum_extelem_async(&final_poly_coeffs, &combos)
                .await?;
            final_poly_coeffs
        });

        scope!("bit_rev", {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new("finalize_async bit_rev");
            self.hal
                .batch_bit_reverse_async(&final_poly_coeffs, ext_size)
                .await?;
        });
        tracing::debug!("FRI-proof, size = {}", final_poly_coeffs.size() / ext_size);

        let mut inner_merkles = self
            .groups
            .iter()
            .map(|pg| &pg.as_ref().unwrap().merkle)
            .collect::<Vec<_>>();
        inner_merkles.push(&check_group.merkle);
        {
            let _timer = crate::hal::webgpu::WebGpuStageTimer::new("finalize_async fri_prove");
            crate::prove::fri::fri_prove_async(
                self.hal,
                &mut self.iop,
                &final_poly_coeffs,
                inner_merkles.as_slice(),
            )
            .await?;
        }

        let proven_soundness_error = super::soundness::proven::<crate::hal::webgpu::WebGpuHal>(
            self.taps,
            final_poly_coeffs.size(),
        );
        tracing::debug!("proven_soundness_error: {proven_soundness_error:?}");

        let conjectured_security = super::soundness::toy_model_security::<
            crate::hal::webgpu::WebGpuHal,
        >(self.taps, final_poly_coeffs.size());
        tracing::debug!("conjectured_security: {conjectured_security:?}");

        let proof = self.iop.proof;
        tracing::debug!("Proof size = {}", proof.len());
        Ok(proof)
    }
}
