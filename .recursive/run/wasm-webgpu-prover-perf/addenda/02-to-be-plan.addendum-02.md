Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `02 TO-BE plan — Addendum 02`
Status: `DRAFT`
DraftedAt: `2026-05-13`
Workflow version: `recursive-mode-audit-v2`
Amends:
- `/.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- `/.recursive/run/wasm-webgpu-prover-perf/addenda/02-to-be-plan.addendum-01.md`
Inputs:
- SP3 retrospective (30 iters 7a–7bb): staged WGSL eval_check hits a Chrome/Dawn code-gen ceiling on production-scale DEFs — ~33× slower than the runtime interpreter; not addressable from the codegen layer. Recorded in memory note `project_sp3_staged_kernel_ceiling.md`.
- SP4 close-out: BufferPool + dispatch_gather_sample_tiled landed and the recursion-sized regression test passes in 0.92 s with cpu_fallbacks=0.
- Recursion lift profile captured 2026-05-13 (poseidon2_basic, post SP4-close-out, baseline 3.88 s): `lift_prove_async = 2730 ms` decomposes to
    - fri_prove: 911 ms (33%)
    - eval_u_groups: 668 ms (24%)
    - commit_group_async × 3 (CTRL + DATA + ACCUM, includes data-group upload + NTT + merkle): ~400 ms residual (~15%)
    - eval_u_check: 202 ms (7%)
    - recursion_witgen + recursion_accumulate: 407 ms combined (15%)
    - check_group: 105 ms (4%)
    - everything else: ~37 ms (1%)
Outputs:
- This file. Amends SP5/SP6 scope; introduces SP6a (`fri_prove` optimization) and SP6b (`eval_u_groups` optimization) as first-class sub-phases.

Scope note: SP3's empirical ceiling and SP4's measured recursion-lift breakdown together establish that the original plan's R3/R4 sequencing — "staged WGSL eval_check for recursion / keccak" — is unlikely to deliver the wall-time target on Chrome WebGPU. This addendum (a) splits SP5 into SP5a (BufferPool data group, real work) and SP5b (staged WGSL, shelved like SP3 unless paired with a separate kernel-restructure phase), and (b) elevates `fri_prove` and `eval_u_groups` from "expected to fall out of R5–R8 lever exhaustion" to dedicated sub-phases SP6a + SP6b, because the measured breakdown shows they are the dominant 57% of recursion lift time and SP5a alone cannot reach the R10 1.0× target on lift.

## TODO

- [x] Decompose SP5 into SP5a (tiled data group) + SP5b (staged WGSL, shelved per SP3 retrospective).
- [x] Insert SP6a (`fri_prove` optimization) and SP6b (`eval_u_groups` optimization) ahead of the existing SP6 (keccak staged WGSL).
- [x] Re-mark SP6 (keccak staged WGSL) and SP5b (recursion staged WGSL) as `shelved-pending-kernel-restructure` rather than `planned`.
- [x] Renumber SP7–SP11 explicitly so downstream artifacts can be traced.
- [x] Record the recursion-lift profile that motivated this addendum.
- [x] Coverage Gate / Approval Gate.

## SP5a — BufferPool-backed recursion data group (R3 prereq, narrowed scope)

Scope and purpose: Convert `commit_group_async(REGISTER_GROUP_DATA, &witgen.data)` for the recursion lift to allocate `witgen.data` as a `BufferPool` whenever its byte size exceeds `max_storage_binding_bytes()`. Per the SP4 BufferPool work, downstream gathers consume tiles natively; remaining downstream ops (NTT, bit-reverse, merkle) either fit in a single binding for their column-wise slices, or can be re-bound per-tile.

Implementation checklist:

- [ ] SP-CR regression gate (per Addendum 01): run xgboost + R1 smoke and confirm no correctness regression before committing.
- [ ] Add `BufferPool::from_cpu_buffer` helper in `risc0/zkp/src/hal/webgpu/buffer_pool.rs` that takes a `WebGpuBuffer<BabyBearElem>` plus `(stride, total_cols)` and copies CPU data into tile buffers.
- [ ] In `risc0/circuit/recursion/src/prove/hal/webgpu.rs:174`–249 `prove_async`, replace the single-buffer DATA-group upload with a BufferPool when the buffer would exceed `max_storage_binding_bytes()`. Add `upload_recursion_data_pool` helper.
- [ ] In `risc0/zkp/src/prove/prover.rs commit_group_async`, expose an opt-in pool-aware path that consumes a `BufferPool` for source data.
- [ ] Wire downstream NTT / merkle for the data group to operate over BufferPool tiles: add `batch_interpolate_ntt_pool_async`, `batch_bit_reverse_pool_async`, `hash_rows_pool_async` to operate per-tile.
- [ ] Add `recursion_lift_pool_upload_smoke` regression test in `examples/browser-prove/src/lib.rs` covering the case where data group exceeds 1 GiB binding (use a synthetic-size recursion DEF if poseidon2_basic doesn't reach that scale).

Tests:
- `cargo test --target wasm32-unknown-unknown --release -p browser-prove webgpu_hal_recursion_sized_gather_sample_uses_buffer_pool` (existing).
- `cargo test --target wasm32-unknown-unknown --release -p browser-prove recursion_lift_pool_upload_smoke` (new).
- `cargo test --target wasm32-unknown-unknown --release -p browser-prove native_poseidon2_basic_async_succinct_receipt_verify` — verifies no regression on the smoke fixture.

QA: rerun poseidon2_basic and re-capture `lift_prove_async` per-stage timings. SP5a's value is whatever `commit_group_async(DATA)` saves vs the existing single-binding path; expect ~100–200 ms on poseidon2_basic. Larger payoff on fixtures whose recursion lift's data group exceeds the 1 GiB binding limit.

## SP5b — Recursion staged WGSL eval_check (SHELVED)

Scope and purpose: same as the original SP5 staged-WGSL portion. Per the SP3 retrospective and the recursion-lift measurement, this is **shelved pending a separate kernel-restructure phase** (interpreter-style loop over compile-time-known op stream, or other approach that bypasses the Chrome/Dawn code-gen ceiling on 1.6 MB straight-line bodies). The `fp_slots=1001 / mix_slots=10` recursion DEF profile is in the same family as the rv32im DEF and is expected to hit the same ceiling.

Implementation checklist:
- [-] **Shelved**: do not attempt without a paired kernel-restructure design. Status to be revisited only if (a) Chrome/Dawn ships meaningfully better code-gen for large compute kernels, or (b) a dedicated kernel-restructure phase lands first.

## SP6a — `fri_prove` optimization (NEW, R5/R7 levers applied directly)

Scope and purpose: Reduce `finalize_async fri_prove` wall time from 911 ms toward something close to the native CUDA `fri_prove` ratio. This is the **largest single hotspot** in the recursion lift profile. The `fri_prove` stage runs a recursive FRI commitment with `fri_fold` reductions and intermediate Merkle commitments. Levers:

- Coalesced `fri_fold` reads (currently scalar; can be batched into vec4 if input is column-major).
- Pipeline reuse on the FRI Merkle hash stages (currently each fold creates fresh pipeline + bind groups; cache them across folds).
- Bounded readbacks at FRI layer roots (currently each layer reads back its full Merkle top; bounded readback per R6).
- For recursion-sized inputs: BufferPool-backed FRI input layer if it exceeds `max_storage_binding_bytes`.

Implementation checklist:
- [ ] SP-CR regression gate.
- [ ] Capture per-`fri_fold` and per-Merkle-layer timings via finer-grained `WebGpuStageTimer` scopes inside `fri_prove` (lives in `risc0/zkp/src/prove/fri.rs` or similar — TBD by reading the code).
- [ ] Identify the worst per-layer cost; attack that lever first.
- [ ] Add `fri_prove_pipeline_reuse_smoke` regression that asserts pipeline-creation count for `fri_fold` drops to a small constant across the layer chain.
- [ ] Per-R10 trend: rerun poseidon2_basic lift after each lever lands and confirm `fri_prove` decreases monotonically.

Tests:
- Smoke fixtures (R1) with refreshed lift trace.
- `cargo test --target wasm32-unknown-unknown --release -p browser-prove native_poseidon2_basic_async_succinct_receipt_verify`.

QA: `lift_prove_async fri_prove` < 300 ms on poseidon2_basic is the iter-by-iter close condition (≈3× speedup needed to hit a ~1.0× CUDA ratio assuming CUDA is in the ~100 ms range for the same work).

## SP6b — `eval_u_groups` optimization (NEW)

Scope and purpose: Reduce `finalize_async eval_u_groups` from 668 ms. This is `batch_evaluate_any` for each of the three groups at the FRI sample z points. With recursion's domain = 1M and 3 groups, this stage is heavy storage-buffer reads. Levers:

- Coalesced reads in `batch_evaluate_any` shader (verify current access pattern).
- Pipeline reuse across groups (currently each group dispatches a fresh pipeline; reuse).
- BufferPool-backed evaluation for groups that exceed `max_storage_binding_bytes`.
- Per-group dispatch parallelism on independent groups (CTRL + DATA + ACCUM evaluate independently).

Implementation checklist:
- [ ] SP-CR regression gate.
- [ ] Capture per-group `batch_evaluate_any` timings under `eval_u_groups`.
- [ ] Apply the lever that the per-group breakdown identifies as worst.
- [ ] `eval_u_groups < 200 ms` on poseidon2_basic is the iter close condition.

## SP6 — Keccak staged WGSL eval_check (SHELVED-PENDING-RESTRUCTURE)

The original SP6 staged-WGSL eval_check for keccak (R4) shares SP3's ceiling and at ~6741 fp_slots is the WORST-case scale for the Chrome/Dawn code-gen ceiling. Status: shelved pending kernel-restructure. Defer until SP5b's revisit conditions hold.

## SP7–SP11 (renumbered if applicable)

The original SP7 (GPU-resident witness/accumulate, R5), SP8 (pipeline + buffer reuse, R7), SP9 (bounded readbacks, R6), SP10 (parity matrix, R9), SP11 (closing audit, R10) remain in plan with their original scope. Their numbering is preserved; SP6a + SP6b slot in ahead of SP6 (now shelved). If a future addendum renumbers, this addendum is its precedent for the SP6a/SP6b naming.

## Trigger event

SP3's 30-iteration retrospective established that staged WGSL eval_check is not addressable from the codegen layer on Chrome WebGPU. SP4's recursion-lift profile (captured 2026-05-13 with the gated drain) showed the actual hotspots are `fri_prove` and `eval_u_groups`, not data group upload. The plan's original assumption ("R3 + R5 + R8 will reach the wall-time target via recursion-staged-WGSL + tiled data group") is empirically incomplete — `fri_prove` + `eval_u_groups` together are 57% of recursion lift time and need direct attention.

## Coverage Gate

Every R# from the run's requirements file (`00-requirements.md`) still has at least one sub-phase that covers it:

- R1 (frozen baselines) → SP1 (unchanged)
- R2 (rv32im staged WGSL) → SP2 + SP3 (SP3 shelved, scaffolding committed)
- R3 (recursion staged WGSL + tiled data group) → SP4 + SP5a (data group) + SP5b (staged, shelved). R3's "1.0× CUDA on recursion" gating moves to SP6a + SP6b which actually attack the dominant hotspots.
- R4 (keccak staged WGSL) → SP6 (shelved-pending-restructure)
- R5 (GPU-resident witness/accumulate) → SP7 (unchanged)
- R6 (bounded readbacks) → SP9 (unchanged), partially overlapped by SP6a Merkle bounded readback
- R7 (pipeline reuse) → SP8 (unchanged), partially overlapped by SP6a + SP6b pipeline reuse
- R8 (`gather_sample` tiled multi-buffer) → SP4 (committed, complete)
- R9 (deferred parity matrix) → SP10 (unchanged)
- R10 (wall-time parity) → SP11 (final audit) ← assessed against post-SP6a/SP6b numbers, not just SP5a
- R11 (forbidden regressions) → cross-cutting (per Addendum 01's SP-CR gate)
- R12 (telemetry) → cross-cutting
- R13 (correctness invariants) → cross-cutting

## Approval Gate

This addendum is `DRAFT` until the recursion-lift profile that motivates it is committed under `evidence/perf/r2-rv32im/` or a new `evidence/perf/r3-recursion/` sub-directory. Once recorded there, this addendum can be promoted to `LOCKED` with a content hash like Addendum 01.
