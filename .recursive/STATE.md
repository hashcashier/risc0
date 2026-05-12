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

- Run `wasm-webgpu-prover-perf`: Phase 0–4 LOCKED. SP1 baseline-capture recipe documented under `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r1-baselines/README.md`. SP2–SP11 production sub-phases deferred to follow-on runs per `02-to-be-plan.md`.
- Worktree `recursive/wasm-webgpu-prover-perf` at HEAD `454b3109b` (controller checkout `wasm` is at same commit).
- Diff basis for follow-on Phase 4 audits: `git diff --name-only d042da45c89a1cd5f9cf7c5eb962a754367b2118`.
