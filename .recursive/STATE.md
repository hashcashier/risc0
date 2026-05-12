# STATE.md

## Current State

### WebGPU browser prover

Status: paused 2026-05-11 at user request after correctness substrate landed; performance follow-up run `wasm-webgpu-prover-perf` initiated 2026-05-12 to drive wall-time toward 1.0× native CUDA.

Substrate (per `docs/wasm-webgpu-prover-learnings.md` and `.recursive/run/wasm-webgpu-prover-perf/01-as-is.md`):
- `risc0_zkp::hal::webgpu::WebGpuHal` exposes ~20 WebGPU-backed bulk ops; correctness-positive on the smoke fixtures.
- Browser CircuitHals for rv32im/keccak/recursion route eval_check through a WGSL interpreter (`EVAL_CHECK_BASE_INTERPRETER_WGSL` at `risc0/zkp/src/hal/webgpu.rs:1329`–1596); witness + accumulation paths still on Rust/WASM bridges.
- 21 currently-passing public examples + internal parity matrix per `docs/wasm-webgpu-validation.md`.
- Performance ratios vs native CUDA (RTX 5090 + Chrome 1 GiB negotiated WebGPU limits): `poseidon2_basic ≈ 9.4×`, `libm ≈ 491×`, `KeccakUnion(1) ≈ 59×`.

Deferred fixtures (blocked on performance work): `multi_test/rsa_compat`, `multi_test/keccak_union` (KeccakUnion(3)), `groth16-verifier`, `xgboost`, `bn254`, `risc0-zkvm-methods/blst`, `risc0-zkvm-methods/verify`.

Failed-experiment ledger (do not re-enable without dedicated evidence): split-shader path (`WEBGPU_EVAL_CHECK_ENABLE_SPLIT = false` at `risc0/zkp/src/hal/webgpu.rs:76`); large private scratch (`WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS = 1536` at line 81); monolithic real-circuit shaders; per-dispatch CPU mirror updates on the focused async path.

### Recursive-mode work

- Run `wasm-webgpu-prover-perf`: Phase 0–8 LOCKED. SP1 baselines captured for all six R1 fixtures (`evidence/perf/r1-baselines/`); ratios 7.6×–8.8× for small fixtures, 15.7× for KeccakUnion(1). SP2 seed (webgpu_codegen module) landed with RED→GREEN unit tests. SP10 partial (xgboost) hit a `verify lift` regression — see Plan Addendum 01 below.
- Worktree `recursive/wasm-webgpu-prover-perf` at HEAD `240425ba9` (SP2 seed); five commits ahead of the Phase 0–8 lock commit `f7698e62a`.
- Diff basis for follow-on Phase 4 audits: `git diff --name-only d042da45c89a1cd5f9cf7c5eb962a754367b2118`.

### Correctness-First Discipline (Plan Addendum 01)

Plan amendment landed 2026-05-12 in `.recursive/run/wasm-webgpu-prover-perf/addenda/02-to-be-plan.addendum-01.md`. Rule: any correctness regression detected during SP1–SP11 (verifier rejection, panic before receipt, cycle drift, new cpu_only_ops/cpu_fallbacks, or Chrome WebGPU device loss) IMMEDIATELY halts all performance work and invokes the new sub-phase `SP-CR` (Correctness Regression triage). SP-CR runs to completion before any preempted SP resumes. Trigger event was the xgboost browser proof's verify_lift failure (`evidence/perf/r9-deferred/xgboost.chrome.txt`). xgboost is now `blocked` pending SP-CR. SP10 cannot proceed for any deferred fixture until xgboost's SP-CR closes.
