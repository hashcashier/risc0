# DECISIONS.md

## Recursive Run Index

- `wasm-webgpu-prover-perf` (started 2026-05-12) — performance follow-up to the paused `docs/requirements/wasm-webgpu-prover.md` correctness run. Drives WebGPU browser prover wall-time toward 1.0× native CUDA via 11 ordered sub-phases (SP1–SP11) covering circuit-specific staged WGSL eval_check for rv32im/recursion/keccak, tiled GPU-resident recursion data group, GPU-side witness+accumulation, bounded transcript readbacks, pipeline+bind-group cache, and tiled gather_sample. Phase 0–4 LOCKED in this run iteration; SP2–SP11 production implementation deferred to follow-on runs.
  - Phase 0 Requirements LockHash: `d98a93e75bc9122b110d42e7c33ecc66d60b438828e585babdf5a9b35ba042fc`
  - Phase 0 Worktree LockHash: `71fbfa4cedfc54f30c9e59e28fd54a59a1197e7d1bdf29faeecc8ec029d8d536`
  - Phase 1 AS-IS LockHash: `6ae676746bbd68fe6d9b724d493cdcfa968d04235ae8e40294c3324da67a743d`
  - Phase 2 TO-BE LockHash: `25aced2df2cb0420801faa3e0e6b0666aeedf1f5cd09803df25180d77a146a92`
  - Phase 3 Implementation LockHash: `a6399a2c129508d9eda16edc82aa1c857c689bd21f38d207fabb80fe5fc3a485`
  - Phase 4 Test Summary LockHash: `3082c7320caf49e1ab282fbef37601f408397e17666131b2db4512bda06bbf69` (later re-locked to `2b29bce7686b7bc8039af6c43b61451e45ecd7265bbfb17564f6dde27e5e9099` after R11 `.gitignore` accounting fix)
  - Phase 3 Implementation: re-locked to `0bea5e92363f0488c5099bb873ac61250d668c46638cd58d3c65d62e14b25117` after same fix
  - **Plan Addendum 01** (Correctness-First Discipline, 2026-05-12): `.recursive/run/wasm-webgpu-prover-perf/addenda/02-to-be-plan.addendum-01.md`. Codifies that any correctness regression detected during SP1–SP11 (verifier rejection, panic before receipt, cycle drift, new `cpu_only_ops`/`cpu_fallbacks`, or Chrome WebGPU device loss) IMMEDIATELY invokes the new `SP-CR` (Correctness Regression triage) sub-phase, which preempts ALL performance work and runs to completion before any preempted SP resumes. Trigger event: xgboost browser proof's verify_lift failure (`evidence/perf/r9-deferred/xgboost.chrome.txt`). xgboost is classified `blocked` pending SP-CR completion. SP10 (deferred matrix bring-up) blocked on xgboost SP-CR.
  - Diff basis (Phase 0): `git diff --name-only d042da45c89a1cd5f9cf7c5eb962a754367b2118`
  - Worktree: `.worktrees/wasm-webgpu-prover-perf/` on branch `recursive/wasm-webgpu-prover-perf`
  - Post-lock work (commits on `recursive/wasm-webgpu-prover-perf` after Phase 0–8 closeout): SP1 baselines (`c68bd6cc7`), docs refresh (`a0a64d492`), SP10 partial / xgboost regression (`1d1d9ef1e`), SP2 seed module + tests (`240425ba9`), Plan Addendum 01 + control plane (this commit).
