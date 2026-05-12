Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `07 State Update`
Status: `LOCKED`
Workflow version: `recursive-mode-audit-v2`
Inputs:
- `/.recursive/run/wasm-webgpu-prover-perf/06-decisions-update.md` (LOCKED, hash `9e82d9b337e0…`)
- `/.recursive/STATE.md`
- `/.recursive/run/wasm-webgpu-prover-perf/01-as-is.md` (LOCKED, hash `6ae676746bbd…`)
- `/.recursive/run/wasm-webgpu-prover-perf/03-implementation-summary.md` (LOCKED, hash `a6399a2c1295…`)
- `/.recursive/run/wasm-webgpu-prover-perf/04-test-summary.md` (LOCKED, hash `3082c7320caf…`)
Outputs:
- `/.recursive/run/wasm-webgpu-prover-perf/07-state-update.md`
- `/.recursive/STATE.md`
Scope note: Updates `.recursive/STATE.md` with the current state of the WebGPU browser prover and the recursive-mode work, replacing the scaffolded "Initial state not documented yet." placeholder with substantive current-state truths grounded in Phase 1/3/4 evidence.

## TODO

- [x] Reread Phase 6 lock; confirm DECISIONS.md update applied.
- [x] Update `.recursive/STATE.md` with WebGPU prover substrate + recursive-mode run state.
- [x] Rationale section covers why the placeholder is replaced and what the new state represents.
- [x] Resulting State Summary block shows post-edit state.
- [x] Audit + Coverage + Approval gates.

## Audit Context

Audit Execution Mode: `self-audit`
Subagent Availability: `available`
Subagent Capability Probe: Subagents available but not invoked; the state update is a controller-owned ledger edit.
Delegation Decision Basis: State update is a narrow ledger edit grounded in Phase 1 AS-IS substrate map and Phase 4 test status.
Delegation Override Reason: None.
Audit Inputs Provided:
- Phase 6 lock (hash `9e82d9b337e0…`)
- Phase 1, 3, 4 locks for substrate ground-truth.
- Diff basis: `git diff --name-only d042da45c89a1cd5f9cf7c5eb962a754367b2118`.

## Effective Inputs Re-read

- `06-decisions-update.md` → confirms DECISIONS.md is current; STATE.md update follows naturally.
- `01-as-is.md ## Current Behavior by Requirement` → ground truth for the substrate description in STATE.md.
- `04-test-summary.md ## Results Summary` → ground truth for "Phase 0–4 LOCKED" claim and SP2–SP11 deferral.
- `.recursive/STATE.md` → confirmed as single-line placeholder pre-edit; first run state update replaces it.

## Earlier Phase Reconciliation

Phase 0–4 + Phase 6 lock chain verifies intact. Phase 7 makes no Phase 0–6 retroactive changes; only `.recursive/STATE.md` is edited.

## Prior Recursive Evidence Reviewed

None applicable. Justification: first run; this is the first STATE.md update.

## Subagent Contribution Verification

No subagents dispatched in Phase 7.

## Worktree Diff Audit

Baseline type: `local commit`
Baseline reference: `d042da45c89a1cd5f9cf7c5eb962a754367b2118`
Comparison reference: `working-tree`
Normalized baseline: `d042da45c89a1cd5f9cf7c5eb962a754367b2118`
Normalized comparison: `working-tree`
Normalized diff command: `git diff --name-only d042da45c89a1cd5f9cf7c5eb962a754367b2118`

Changed files (Phase 7 entry):
- `.gitignore` (Phase 0)
- `.recursive/DECISIONS.md` (Phase 6)
- `.recursive/STATE.md` (Phase 7: this update)
- All run-local artifacts under `.recursive/run/wasm-webgpu-prover-perf/`
- `evidence/perf/r1-baselines/README.md`

`.recursive/STATE.md` is the only non-run-local non-DECISIONS file edited in Phase 7.

## Gaps Found

None.

## Repair Work Performed

Added the current-state summary to `.recursive/STATE.md`. No other product or worktree edits.

## State Changes Applied

Pre-edit `.recursive/STATE.md` body (under `## Current State`):

```
- Initial state not documented yet.
```

Post-edit body covers two sections — "WebGPU browser prover" (substrate, performance ratios, deferred fixtures, failed-experiment ledger) and "Recursive-mode work" (run lock-chain status, worktree branch, diff basis). The full post-edit text is at `.recursive/STATE.md` and was applied in this phase.

## Rationale

The placeholder "Initial state not documented yet." was the scaffolded default. Replacing it with substantive current-state truths makes `.recursive/STATE.md` a useful entry point for future contributors. The WebGPU prover section indexes the substrate (HAL ops, CircuitHal routing, interpreter usage) and the performance baseline (per-fixture ratios from `docs/wasm-webgpu-cuda-comparison.md`). The recursive-mode section records Phase 0–4 lock status, worktree branch, and diff basis so any future run can pick up the SP2–SP11 work without re-deriving Phase 0 setup. The failed-experiment ledger is repeated here (rather than only in `01-as-is.md`) so a contributor consulting STATE.md gets the prohibitive constraints at a glance.

## Resulting State Summary

```
### WebGPU browser prover

Status: paused 2026-05-11 at user request after correctness substrate landed; performance follow-up run `wasm-webgpu-prover-perf` initiated 2026-05-12 to drive wall-time toward 1.0× native CUDA.
LockedAt: `2026-05-12T05:21:59Z`
LockHash: `60efaf09c8b74512e8d98a1f105edbdd7b9634f3c8cee6efa01df063f1562795`

Substrate (per `docs/wasm-webgpu-prover-learnings.md` and `.recursive/run/wasm-webgpu-prover-perf/01-as-is.md`):
- `risc0_zkp::hal::webgpu::WebGpuHal` exposes ~20 WebGPU-backed bulk ops; correctness-positive on the smoke fixtures.
- Browser CircuitHals for rv32im/keccak/recursion route eval_check through a WGSL interpreter (`EVAL_CHECK_BASE_INTERPRETER_WGSL` at `risc0/zkp/src/hal/webgpu.rs:1329`–1596); witness + accumulation paths still on Rust/WASM bridges.
- 21 currently-passing public examples + internal parity matrix per `docs/wasm-webgpu-validation.md`.
- Performance ratios vs native CUDA (RTX 5090 + Chrome 1 GiB negotiated WebGPU limits): poseidon2_basic ≈ 9.4×, libm ≈ 491×, KeccakUnion(1) ≈ 59×.

Deferred fixtures (blocked on performance work): multi_test/rsa_compat, KeccakUnion(3), groth16-verifier, xgboost, bn254, risc0-zkvm-methods/blst, risc0-zkvm-methods/verify.

Failed-experiment ledger (do not re-enable without dedicated evidence): split-shader path (WEBGPU_EVAL_CHECK_ENABLE_SPLIT = false at risc0/zkp/src/hal/webgpu.rs:76); large private scratch (WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS = 1536 at line 81); monolithic real-circuit shaders; per-dispatch CPU mirror updates on the focused async path.

### Recursive-mode work

- Run `wasm-webgpu-prover-perf`: Phase 0–4 LOCKED. SP1 baseline-capture recipe documented under `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r1-baselines/README.md`. SP2–SP11 production sub-phases deferred to follow-on runs per `02-to-be-plan.md`.
- Worktree `recursive/wasm-webgpu-prover-perf` at HEAD `454b3109b` (controller checkout `wasm` is at same commit).
- Diff basis for follow-on Phase 4 audits: git diff --name-only d042da45c89a1cd5f9cf7c5eb962a754367b2118.
```

## Requirement Completion Status

- R1 | Status: deferred | Rationale: SP1 recipe canonical via `evidence/perf/r1-baselines/README.md`; measurement runs deferred. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R2 | Status: deferred | Rationale: SP2/SP3 implementation deferred. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R3 | Status: deferred | Rationale: SP4/SP5 implementation deferred. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R4 | Status: deferred | Rationale: SP6 implementation deferred. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R5 | Status: deferred | Rationale: SP7 implementation deferred. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R6 | Status: deferred | Rationale: SP8 implementation deferred. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R7 | Status: deferred | Rationale: SP9 implementation deferred. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R8 | Status: deferred | Rationale: SP4 implementation deferred. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R9 | Status: deferred | Rationale: SP10 deferred matrix bring-up depends on SP2–SP9. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R10 | Status: deferred | Rationale: SP11 closing-condition audit depends on SP2–SP9. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R11 | Status: deferred | Rationale: Guard-flag invariance preserved; SP11 formally verifies. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R12 | Status: deferred | Rationale: Evidence layout established; measurement deferred. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R13 | Status: deferred | Rationale: Verifier paths preserved; SP11 verifies regression matrix. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- R18 | Status: out-of-scope | Rationale: Inherited via R13. | Scope Decision: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`

## Audit Verdict

STATE.md update accurately reflects the current substrate + run state. No retroactive changes to Phase 0–6 locked artifacts. The two-section structure (WebGPU prover substrate + recursive-mode work) makes the run's progress legible at a glance.

Audit: PASS

## Traceability

- R1 → STATE.md "WebGPU browser prover" + "Recursive-mode work" sections (SP1 reference)
- R2 → STATE.md "WebGPU browser prover" substrate description (interpreter usage as the rv32im baseline that R2 replaces)
- R3 → STATE.md "WebGPU browser prover" substrate description (interpreter usage + per-proof recursion-data uploads as the recursion baseline that R3 replaces)
- R4 → STATE.md "WebGPU browser prover" substrate description (Keccak interpreter underutilization that R4 replaces)
- R5 → STATE.md "WebGPU browser prover" substrate description (witness/accumulation on Rust/WASM bridges that R5 replaces)
- R6 → STATE.md "WebGPU browser prover" — performance ratios reflect current readback cost; R6 will reduce
- R7 → STATE.md "WebGPU browser prover" substrate description (per-stage pipelines without cross-stage cache; R7 introduces cache)
- R8 → STATE.md "WebGPU browser prover" substrate description (locked CPU fallback for oversized gather_sample; R8 replaces with tiled multi-buffer)
- R9 → STATE.md "WebGPU browser prover" Deferred fixtures list (R9 brings each back to passing)
- R10 → STATE.md "WebGPU browser prover" performance ratios (current 9.4×/491×/59× drive toward 1.0× under R10)
- R11 → STATE.md "WebGPU browser prover" Failed-experiment ledger (R11 forbids re-enabling)
- R12 → STATE.md "Recursive-mode work" (SP1 evidence-capture layout established)
- R13 → STATE.md "WebGPU browser prover" (21 public examples + parity matrix regression line)
- R18 → Inherited via R13 in STATE.md

## Coverage Gate

- [X] STATE.md updated with substantive current-state content.
- [X] Rationale section explains the replacement.
- [X] Resulting State Summary block shows post-edit state.
- [X] Traceability covers R# scope.

Coverage: PASS

## Approval Gate

- [X] STATE.md edit applied cleanly.
- [X] No retroactive changes to Phase 0–6 locked artifacts.
- [X] Substrate + run state are both legible.

Approval: PASS
