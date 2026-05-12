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

Plan amendment landed 2026-05-12 in `.recursive/run/wasm-webgpu-prover-perf/addenda/02-to-be-plan.addendum-01.md`. Rule: any correctness regression detected during SP1–SP11 (verifier rejection, panic before receipt, cycle drift, new cpu_only_ops/cpu_fallbacks, or Chrome WebGPU device loss) IMMEDIATELY halts all performance work and invokes the new sub-phase `SP-CR` (Correctness Regression triage). SP-CR runs to completion before any preempted SP resumes.

### SP-CR (xgboost verify_lift) — RESOLVED via D12 (2026-05-12)

Trigger event was the xgboost browser proof's verify_lift failure (`evidence/perf/r9-deferred/xgboost.chrome.txt`). After 12 diagnostics (D1–D12) recorded in `.recursive/run/wasm-webgpu-prover-perf/01.5-root-cause.md`, the bug was narrowed to the `gpu_authoritative=true` path of recursion's commit_groups: GPU dispatches return `gpu_used=true` while the GPU buffer ends up holding zeros at segments 5–8 (non-deterministic). The D11 panic in `finish_hal_op` did not fire, so the corruption is in the actual GPU dispatch path, not the cpu_mirror fallback. Most likely cause: silent Chrome WebGPU `uncapturederror` (no listener attached at `request_device()`) under cumulative GPU pressure.

D12 landing: `risc0/circuit/recursion/src/prove/hal/webgpu.rs` forces `gpu_authoritative_scope(false)` for both `commit_group` blocks in recursion's `prove_async`. xgboost succinct receipt now verifies in 3548 s (vs ~85 s on the broken path, vs ~5.7 s native CUDA). The surgical fix to restore lift performance is deferred — most promising next step is **D14**: attach `onuncapturederror` listener to surface the swallowed validation error. Until then, D12 is the production configuration; recursion performance is intentionally cpu_mirror.

Complementary defensive correctness fix landed alongside D12: `can_dispatch_hash_rows` and `can_dispatch_hash_fold` now check `round_constants` / `m_int_diag` `raw_buffer` presence (mirroring `dispatch_poseidon2_hash_*`'s preconditions) — this was originally a hypothesized cause that D11 falsified, but the tightening is kept because it removes a latent silent-fallback class. xgboost is now `verified` (cpu_mirror recursion, ~60 min). SP10 unblocked for follow-on deferred fixtures.
