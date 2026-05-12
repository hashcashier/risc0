Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `02 TO-BE plan — Addendum 01`
Status: `LOCKED`
LockedAt: `2026-05-12T06:43:01Z`
LockHash: `406d4123d961bded1438b939936c38790be868abf712f624ff4d2df2e15ed5d6`
Workflow version: `recursive-mode-audit-v2`
Amends:
- `/.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md` (LOCKED, hash `25aced2df2cb0420801faa3e0e6b0666aeedf1f5cd09803df25180d77a146a92`)
Inputs:
- `/.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- `/.recursive/run/wasm-webgpu-prover-perf/01-as-is.md`
- `/.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r9-deferred/xgboost.chrome.txt` (xgboost verify_lift failure)
Outputs:
- `/.recursive/run/wasm-webgpu-prover-perf/addenda/02-to-be-plan.addendum-01.md`
Scope note: Codifies the **Correctness-First Discipline**. Any correctness regression detected during SP1–SP11 (e.g., the xgboost `verify lift` failure observed 2026-05-12) immediately halts all performance work on this run until the regression is fixed. This addendum amends every SP's "Implementation checklist" with a regression-triage gate and adds a new sub-phase `SP-CR` (Correctness Regression triage) that takes priority over every other SP whenever invoked.

## TODO

- [x] Define the Correctness Regression criteria (1)–(6).
- [x] Define the SP-CR sub-phase scope, checklist, tests, QA surface, idempotence guidance.
- [x] Amend every SP1–SP11 Implementation checklist with the regression-gate item.
- [x] Mark xgboost as `blocked` pending SP-CR.
- [x] Block SP10 on xgboost SP-CR completion.
- [x] Document that this addendum exists outside the locked Phase 2 artifact (per recursive-mode addendum convention).
- [x] Update `.recursive/STATE.md`, `.recursive/DECISIONS.md`, `docs/wasm-webgpu-prover.md`, `docs/wasm-webgpu-validation.md`, `docs/wasm-webgpu-prover-learnings.md` to surface the rule.
- [x] Coverage Gate / Approval Gate.

## Trigger event

The xgboost browser Chrome run on 2026-05-12 (after SP1 docs refresh) completed `prove_session_async` across all 11 segments in 103.585 s but the subsequent `composite_to_succinct_async` rejected with `panicked at browser-prove/src/lib.rs:338:17: xgboost: async prove failed: verify lift`. Per-segment telemetry shows segment 8's `lift_prove_async = 642 ms` (vs ~2570 ms for segments 0–7) and `verify_lift = 1 ms` (vs ~14 ms for peers), suggesting a transcript-corruption or buffer-staleness regression at a segment-to-segment boundary. The proof was rejected before succinct compression. Evidence: `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r9-deferred/xgboost.chrome.txt`.

The amended `docs/wasm-webgpu-validation.md` row for xgboost records the failure. This addendum makes the response to such failures a hard rule.

## Correctness-First Discipline (new rule)

**Correctness regression** is any of:
1. A Chrome WebGPU receipt that previously verified, now fails to verify with the existing verifier.
2. A `prove_with_opts_async` or `compress_async` call that previously succeeded, now panics or returns an error before producing a receipt.
3. Cycle-count or segment-count drift between native CUDA and Chrome WebGPU for the same fixture.
4. `cpu_only_ops > 0` on any run that previously reported `cpu_only_ops = 0`.
5. Any new `cpu_fallbacks` site on an existing fixture beyond the documented Phase 1 ledger entries (today: `scatter` on small fixtures; `scatter + 3 keccak eval_check` on `KeccakUnion(1)`).
6. Any Chrome WebGPU device-loss event that did not occur in the AS-IS baseline.

When any of (1)–(6) is observed:

- **All in-progress and planned performance work on this run halts immediately.** SP2–SP11 are paused.
- The triage of the regression takes priority and is tracked under the new sub-phase `SP-CR` defined below.
- Performance work resumes only after the regression is fixed and verified by re-running the affected fixture (and the R1 smoke suite) end-to-end with a verified receipt.

## SP-CR — Correctness Regression Triage (new)

`SP-CR` is a non-numbered sub-phase that can be invoked at any point during SP1–SP11. It preempts whichever SP is in flight and runs to completion before the preempted SP resumes.

Scope and purpose: Reproduce the regression, isolate its root cause, land a fix, and re-verify the affected fixture plus the R1 smoke suite. Document the root cause and the fix in `01.5-root-cause.md` (created or appended to) under the run.

Implementation checklist:
- [ ] Capture the failing run's full output under `.recursive/run/wasm-webgpu-prover-perf/evidence/logs/` with a timestamped filename.
- [ ] Reproduce the failure deterministically (assert the same panic/error message + stage telemetry shape on a second run).
- [ ] Identify the minimal fixture or focused test that reproduces the failure.
- [ ] Bisect against the worktree's commit history if the regression appeared between commits.
- [ ] Form a root-cause hypothesis. Confirm via instrumentation (additional `WebGpuStageTimer` scopes, GPU-state assertions, or targeted CPU-vs-WebGPU comparisons in the regression area).
- [ ] Land the fix. Add a focused regression test that would catch the same failure if it recurred.
- [ ] Re-run the affected fixture end-to-end on Chrome WebGPU. Confirm: (a) succinct receipt verifies; (b) cycle/segment counts match native; (c) `cpu_only_ops = 0`; (d) no new `cpu_fallbacks` sites.
- [ ] Re-run the R1 smoke suite (six fixtures). Confirm no other fixture regressed.
- [ ] Update `docs/wasm-webgpu-validation.md` with the fix and the verified row.
- [ ] Append a root-cause + fix entry to `01.5-root-cause.md` (or create that artifact if it doesn't yet exist for this run).
- [ ] Mark the affected fixture's row in `docs/wasm-webgpu-validation.md` as verified again.

Tests for this sub-phase:
- The affected fixture's existing browser test (e.g., `xgboost_succinct_receipt_verifies` for the trigger event).
- The R1 smoke suite (six fixtures).
- The new focused regression test added during the fix.

QA Surface: rerun the affected fixture + R1 smoke; visually inspect refreshed validation matrix.

Idempotence: SP-CR can be invoked repeatedly; each invocation produces its own `01.5-root-cause.md` entry, evidence files, and regression-test addition. Multiple SP-CR cycles per run are allowed if multiple regressions are observed.

## Amendments to SP1–SP11

Every SP's "Implementation checklist" is extended with this final gate item (effective immediately):

- [ ] Run the R1 smoke suite plus the SP's directly affected fixture(s). Confirm no correctness regression per the (1)–(6) definitions above. If any regression is detected, immediately invoke `SP-CR` before continuing or committing the SP work.

Specific SP amendments:

- **SP1** (baseline capture): If any of the six R1 fixtures fails its succinct-receipt-verify on Chrome during the initial capture, treat as a correctness regression (because the AS-IS state has these fixtures passing per `docs/wasm-webgpu-validation.md`). Invoke `SP-CR` before proceeding to SP2.
- **SP2 / SP3** (rv32im staged WGSL): Any small-fixture regression after wiring the generator into the prove path invokes `SP-CR`. The generator's CPU-parity unit tests stay GREEN as a precondition; if they regress, SP-CR applies.
- **SP4 / SP5** (BufferPool + GPU witness/accumulate): Touching the buffer model and witness/accumulate paths is the highest-risk regression surface. Every commit in these SPs MUST be preceded by a clean R1 smoke run; any regression invokes `SP-CR`.
- **SP6** (keccak staged WGSL): `KeccakUnion(1)` and `KeccakUnion(3)` receipts MUST verify before SP6 lock. Any fallback/dispatch correctness drift invokes `SP-CR`.
- **SP7** (GPU witness/accumulate): the no-op keccak accumulate reconciliation MUST not change `KeccakUnion(1)` receipt verification; if it does, SP-CR.
- **SP8** (bounded readbacks): coalescing readbacks must not change transcript order or sampled openings; if a receipt fails to verify after a coalescing change, SP-CR.
- **SP9** (pipeline + bind-group cache): the cache MUST be correctness-safe (stale-pipeline reuse is a correctness regression); any receipt failure after a cache-hit invokes SP-CR.
- **SP10** (deferred matrix bring-up): xgboost's current verify_lift failure (the trigger event for this addendum) means SP10 cannot proceed for any deferred fixture before xgboost is fixed under SP-CR. The xgboost verify_lift root cause MUST be resolved before any other deferred fixture is attempted via SP10.
- **SP11** (closing-condition audit): explicitly verifies no fixture's row in `docs/wasm-webgpu-validation.md` regressed from its AS-IS verified state. Any regression at SP11 invokes SP-CR before the run can be marked closed.

## Updated sub-phase ordering

The SP1 → SP2 → ... → SP11 attack order from `02-to-be-plan.md ## Implementation Steps` is preserved with one insertion:

`SP10` (deferred matrix bring-up) cannot start for any fixture until the xgboost verify_lift regression has been resolved under SP-CR. The current SP10 partial result (xgboost attempt 2026-05-12) is the trigger event; the rest of SP10 is blocked on SP-CR completion for that fixture.

## Updated Requirement Completion Status conventions

For any future phase artifact (Phase 4 test summary in a follow-on run, etc.):

- A fixture marked `verified` in any Requirement Completion Status MUST satisfy the (1)–(6) definitions above. If a fixture previously marked `verified` later regresses, downgrade to `blocked` until SP-CR completes.
- xgboost is marked `blocked` (status: blocked) until SP-CR resolves the verify_lift root cause. Blocking Evidence: `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r9-deferred/xgboost.chrome.txt`.

## Plan Drift Check (amended)

This addendum adds:
- A new sub-phase (`SP-CR`) that can preempt any SP1–SP11.
- An extra checklist item to every SP1–SP11's Implementation checklist (regression gate).
- A blocking precondition on SP10 (xgboost must be SP-CR'd first).

Coverage neutrality: SP-CR is not tied to a specific R#. It is a process discipline that supports R13 (correctness invariants preserved end-to-end) and R11 (failure-mode hygiene). It does not introduce new R# scope.

## Why this addendum exists outside the locked Phase 2 artifact

`02-to-be-plan.md` is LOCKED with content hash `25aced2df2cb…`. Edits to its body would break the lock seal. The recursive-mode workflow handles post-lock plan amendments via `addenda/<artifact>.addendum-NN.md` files; this is that mechanism. Future phase artifacts in follow-on runs MUST include this addendum's path in their `Inputs` and `Effective Inputs Re-read` sections.

## Acknowledgement of session scope

This addendum was written after observing the xgboost verify_lift failure during SP10 partial execution in the same session that locked Phase 0–8. It does not retroactively invalidate any locked Phase 0–8 artifact; it amends the plan going forward.

## Traceability

- xgboost verify_lift failure → triggers SP-CR before any further SP10 fixture is attempted
- R11 (failure-mode hygiene) → reinforced: any new `cpu_fallbacks` site or device-loss event invokes SP-CR
- R13 (correctness invariants preserved end-to-end) → operationalized as the SP-CR gate on every SP
- Phase 2 `## Implementation Sub-phases` SP10 checklist → blocked on xgboost SP-CR completion
- Phase 2 `## Implementation Steps` SP11 closing-condition audit → must verify the SP-CR gate held across the run

## Coverage Gate

- [X] Correctness Regression definition (1)–(6) recorded.
- [X] SP-CR sub-phase defined with scope, purpose, implementation checklist, tests, QA surface, idempotence guidance.
- [X] All eleven existing sub-phases (SP1–SP11) amended with a regression-gate checklist item.
- [X] SP10 explicitly blocked on xgboost SP-CR completion.
- [X] xgboost classified as `blocked` with Blocking Evidence path.
- [X] No R# scope drift introduced by this addendum (process discipline only).

Coverage: PASS

## Approval Gate

- [X] Addendum amends the LOCKED Phase 2 plan without modifying its body (lock seal preserved).
- [X] The correctness-first rule is a strict superset of the existing R11/R13 constraints — it does not relax any prior discipline.
- [X] SP-CR is invokable from any SP; preemption discipline is explicit.
- [X] No existing receipt-format, verifier, control ID, or claim-semantics change introduced (R13 invariant preserved).

Approval: PASS
