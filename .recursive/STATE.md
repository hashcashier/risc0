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

### SP-CR (xgboost verify_lift) — RESOLVED via D14+D15+D16 (2026-05-12 17:01, GPU-only fix)

Trigger event was the xgboost browser proof's verify_lift failure (`evidence/perf/r9-deferred/xgboost.chrome.txt`). After D11's silent-fallback theory was falsified and D12's cpu_mirror workaround rejected (user directive 2026-05-12 15:30: "Falling back to CPU is not a fix under any circumstance"), the GPU-only investigation landed.

**Root cause (D14 evidence)**: Chrome WebGPU's Vulkan backend (Dawn) raised `VK_ERROR_OUT_OF_DEVICE_MEMORY` during `recursion_witgen`'s `witgen.data` allocation at segments 5-8. The buffer became `Invalid`; ~1235 cascading `GPUValidationError` events fired on subsequent operations. Without an `onuncapturederror` listener these were silent, and dispatches continued reporting `gpu_used=true` while writing to invalid buffers — producing zero Merkle roots → control_id mismatch.

**Cause of cumulative VRAM exhaustion**: `WebGpuBuffer` had no `Drop` impl calling `.destroy()`; JS GC of `GpuBuffer` handles is too slow for the recursion lift's allocation pattern. ~8-16 GiB cumulative VRAM held across 7-8 lifts exceeded Chrome Dawn's per-context budget.

**Fix (D14+D15+D16, all in `risc0/zkp/src/hal/webgpu.rs`)**:
- **D14**: `onuncapturederror` listener at `request_device()` logs `(type, msg)` of any future GPU error via `console.error`.
- **D15**: `BabyBearElem::ROU_FWD` and `ROU_REV` cached as `WebGpuHal.ntt_roots_fwd` / `ntt_roots_rev` at HAL init; dispatch sites consume cached buffers instead of allocating fresh.
- **D16**: `WebGpuBufferOwner { buffer: GpuBuffer }` with `Drop::drop` calling `buffer.destroy()`; `WebGpuBuffer.gpu` is `Option<Rc<WebGpuBufferOwner>>`. The last clone's drop releases GPU memory deterministically.

**Verified**: xgboost succinct receipt in **117.92 s** on the full GPU path (~21× native CUDA's 5.7 s) — vs the rejected D12 cpu_mirror's ~600×. R1 smoke poseidon2_basic in 3.93 s (matches baseline; no regression from D16's Drop work). Evidence: `evidence/logs/sp-cr-xgboost-d16-buffer-destroy-1778619000.txt`, `evidence/logs/sp-cr-r1-smoke-d16-poseidon2-1778619200.txt`.

Complementary defensive correctness fix retained: `can_dispatch_hash_rows` / `can_dispatch_hash_fold` check `round_constants` / `m_int_diag` raw_buffer presence (mirrors dispatch preconditions). xgboost reclassified to `verified` (full GPU path). SP10 unblocked. Performance follow-up: close the ~21× gap to 1.0× native CUDA via SP2–SP11.
