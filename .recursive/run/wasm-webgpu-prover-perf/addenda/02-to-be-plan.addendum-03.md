Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `02 TO-BE plan — Addendum 03`
Status: `DRAFT`
DraftedAt: `2026-05-13`
Workflow version: `recursive-mode-audit-v2`
Amends:
- `/.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- `/.recursive/run/wasm-webgpu-prover-perf/addenda/02-to-be-plan.addendum-01.md`
- `/.recursive/run/wasm-webgpu-prover-perf/addenda/02-to-be-plan.addendum-02.md`
Inputs:
- Concurrency analysis (this session, 2026-05-13): current `prove_session_async` / `composite_to_succinct_async` / `lift_async` / `join_async` chains are **strictly serial**. Each segment awaits the previous segment's full prove; each lift awaits the previous segment's lift; each join awaits its two operand receipts. Trace evidence: every `*_async elapsed_ms` in the smoke logs is contiguous-sequential.
- WebGPU device queue semantics: one `GPUQueue` per `WebGpuHal`. Concurrent `queue.submit()` calls serialize through that queue, but **CPU-side bookkeeping (preflight, witgen, IOP commits, hash-chain construction) does not need to wait on GPU drains** as long as the data lifetimes are preserved.
- User directive 2026-05-13: "Let integrate that practical roadmap into our plan. We should aim to maximize GPU utilization at all time."
Outputs:
- This file. Introduces `SP6c` (CPU/GPU overlap + segment/lift pipelining) and amends every existing SP's "Implementation checklist" with a `GPU-utilization regression gate` mirroring Addendum 01's SP-CR gate — every commit that touches the prove pipeline must surface a measured `gpu_idle_ratio` and not regress it without an explicit reason.

Scope note: SP3's WGSL code-gen ceiling, SP6a's Poseidon2 compute floor, and SP6b's per-evaluation thread underutilization together leave significant **GPU idle time during CPU-bound stages and between-segment transitions**. Even on poseidon2_basic (1 segment) we have `recursion_witgen` (202 ms) + `recursion_accumulate` (205 ms) + IOP commits that ARE CPU-bound while the GPU sits idle. On multi-segment fixtures (xgboost has many; xgboost's 117 s baseline) the win compounds: every segment-to-segment transition wastes GPU capacity. This addendum makes "minimize `gpu_idle_ratio`" a first-class lever alongside R2–R8.

## TODO

- [x] Codify the `gpu_idle_ratio` metric definition.
- [x] Define SP6c: CPU/GPU overlap + segment/lift pipelining.
- [x] Amend each remaining SP (SP6a/SP6b/SP7/SP8/SP9/SP10/SP11) with the `gpu_idle_ratio` regression gate.
- [x] Coverage Gate / Approval Gate.

## `gpu_idle_ratio` metric

For each prove run, capture:
- `wall_time_ms` — `prove_session_async` end-to-end elapsed
- `gpu_active_ms` — sum of all timer scopes whose body issued at least one GPU dispatch and ended with a drain (queue.onSubmittedWorkDone or mapAsync). Identified by stage label prefix or by an explicit annotation we add to WebGpuStageTimer.

Then `gpu_idle_ratio = 1.0 - gpu_active_ms / wall_time_ms`.

On the current poseidon2_basic post-SP6a smoke (lift_prove_async = 2730 ms, prove_session = 3739 ms):
- Approximate `gpu_active_ms` for rv32im finalize stages alone: ~250 ms (eval_check + check_group + NTT pieces).
- Plus recursion finalize: ~1900 ms (eval_check_drain-equivalent + check_group + eval_u_groups + eval_u_check + fri_prove). Most of these ARE drains, so they ARE active GPU time.
- CPU-bound pieces: recursion_witgen (202 ms), recursion_accumulate (205 ms), commit_group_async wrappers (~400 ms residual which is half CPU bookkeeping, half NTT GPU work), ~407 ms total CPU-bound on poseidon2_basic recursion lift.
- Rough current `gpu_idle_ratio` ≈ 407 / 3739 ≈ **11%**. Not huge on this fixture, but on multi-segment xgboost it scales by segment count.

Target: `gpu_idle_ratio < 5%` on every R1 smoke fixture, `< 10%` on the R9 deferred matrix.

## SP6c — CPU/GPU overlap + segment/lift pipelining (NEW)

Scope and purpose: Restructure `prove_session_async` and `composite_to_succinct_async` so CPU-bound stages (preflight, witgen, IOP bookkeeping) for segment / lift N+1 overlap GPU-bound stages (commit_group_async finalize_async fri_prove) for segment / lift N. The GPU queue stays saturated; CPU work piggybacks.

Implementation checklist:

- [ ] SP-CR regression gate (Addendum 01) AND `gpu_idle_ratio` regression gate.
- [ ] Add `WebGpuStageTimer::new_active` constructor that explicitly marks a stage as "GPU active" (issues dispatches and ends with a drain). Existing `new` becomes "neutral" (CPU bookkeeping). Update existing timers to tag themselves correctly so the metric can be computed automatically.
- [ ] In `risc0/zkvm/src/host/server/prove/prover_impl.rs prove_session_async` (line 193), pipeline the segment loop: start segment N+1's preflight + witgen as a separate async task while segment N's finalize is awaiting. Use `wasm_bindgen_futures::spawn_local` or `futures::join!` style as appropriate. Ensure the prover state is correctly threaded (each segment owns its own `Prover`).
- [ ] In `composite_to_succinct_async`, pipeline lifts: lift N+1 can start as soon as segment N+1's receipt is ready, without waiting for lift N. The current synchronous loop blocks unnecessarily.
- [ ] In the join tree, parallelize independent joins at the same tree level: e.g., join(A, B) and join(C, D) at level 0 don't depend on each other.
- [ ] Add a regression test `webgpu_multi_segment_gpu_idle_ratio_smoke` that runs a synthetic multi-segment proof and asserts the measured `gpu_idle_ratio` is below the per-fixture target.

Tests:
- Smoke fixtures (R1) — confirm `gpu_idle_ratio` improves and no correctness regression.
- xgboost (R9 deferred) — multi-segment; expect the biggest absolute win here.
- `cargo test --target wasm32-unknown-unknown --release -p browser-prove webgpu_multi_segment_gpu_idle_ratio_smoke`.

QA: visual inspection of the trace — interleaved `recursion_witgen` and `finalize_async` stages from different segments should overlap by their `elapsed_ms` boundaries. `gpu_idle_ratio` reported in `evidence/perf/sp6c-overlap/` per fixture.

## Amendments to existing SPs

Every remaining SP (SP6a, SP6b, SP7, SP8, SP9, SP10, SP11) gets one additional checklist item:

- [ ] **GPU-utilization regression gate**: after the SP's iter(s) commit, measure `gpu_idle_ratio` on poseidon2_basic + libm + keccak_union_small. Compare to the SP-entry baseline. If `gpu_idle_ratio` grows by more than 2 percentage points, root-cause before declaring the SP done. Reason: each lever should EITHER reduce per-stage wall time OR overlap better; it should not make the GPU sit idle more than before.

This composes with the `feedback_full_benchmarks_at_phase_end` discipline.

## Trigger event

User directive 2026-05-13: "We should aim to maximize GPU utilization at all time." The original plan structured wins per-stage; concurrency / overlap was implicit and unattributed. Making `gpu_idle_ratio` a first-class metric and adding SP6c as a dedicated overlap phase ensures the lever is exercised.

## Coverage Gate

R10 (wall-time parity) implicitly benefits from `gpu_idle_ratio` reduction — any CPU-bound stage that can overlap GPU work shrinks total wall time. SP11's closing audit now considers `gpu_idle_ratio` per-fixture as part of the "every lever exhausted" criterion.

## Approval Gate

DRAFT until the first `gpu_idle_ratio` measurement is recorded under `evidence/perf/sp6c-overlap/<fixture>.md`.
