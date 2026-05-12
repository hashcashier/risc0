Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `00 Requirements`
Status: `LOCKED`
LockedAt: `2026-05-12T04:40:12Z`
LockHash: `d98a93e75bc9122b110d42e7c33ecc66d60b438828e585babdf5a9b35ba042fc`
Workflow version: `recursive-mode-audit-v2`
Inputs:
- `docs/requirements/wasm-webgpu-prover.md` (correctness seed, R1–R18, approved 2026-05-08, paused 2026-05-11)
- `docs/wasm-webgpu-prover-learnings.md` (pause handoff, executive summary, resume plan, failed-experiment ledger)
- `docs/wasm-webgpu-validation.md` (current correctness matrix and per-fixture native CUDA baselines)
- `docs/wasm-webgpu-cuda-comparison.md` (per-stage telemetry: poseidon2_basic 4.03 s vs 426 ms native CUDA, libm 214 s vs 436 ms, KeccakUnion(1) 441 s vs 7.47 s)
- `docs/wasm-webgpu-prover.md` (public-facing API + architecture description)
- Branch state: `wasm`, HEAD `d042da45c recursive mode`, ahead of `main` with WebGPU prover work; native CUDA prover stack unchanged.
Outputs:
- `/.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
Scope note: This document defines stable requirement identifiers and acceptance criteria for driving the WebGPU prover's wall-time toward native CUDA proving parity (1.0× as ideal target) while preserving the correctness gains locked in by the prior paused effort. (Template: feature)

## TODO

- [x] Elicit requirements from user/context (read `docs/` resume handoff + prior `docs/requirements/wasm-webgpu-prover.md`).
- [x] Define requirement identifiers (R1–R13).
- [x] Write acceptance criteria for each requirement.
- [x] Document out of scope items (OOS1–OOS7).
- [x] List constraints and assumptions.
- [x] Complete Coverage Gate checklist.
- [x] Complete Approval Gate checklist.

## Goal Prompt

Drive the WebGPU browser prover's wall-time toward native CUDA proving with 1.0× parity as the ideal target for every fixture class, while preserving the correctness, verifier compatibility, and parity matrix gained from the previously approved correctness run (`docs/requirements/wasm-webgpu-prover.md`). Continue extending the existing `risc0_zkp::hal::webgpu`, browser `CircuitHal`, and `WebGpuProver` paths rather than building a parallel browser proof system. The acceptance gate is two-fold: every receipt produced must still verify with the existing verifier and match native cycle/segment counts, and every lever in R2–R8 must have been applied so that any residual gap above 1.0× has an explicit, documented structural cause (workgroup-storage cap, async readback unavoidability, browser device-loss threshold) recorded in `docs/wasm-webgpu-cuda-comparison.md`.

## Requirements

### `R1` Frozen correctness smoke baseline

Description: Before any optimization, capture a green correctness smoke suite on the current branch state so every later performance change can be measured against a known-good regression line.

Acceptance criteria:
- Rebuild the wasm harness (`cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release --no-run`) and record the resulting `browser_prove-*.wasm` artifact path.
- Capture fresh native CUDA baselines (`--features cuda`) for: `risc0-zkvm-methods/cfg`, `hello-world`, `json`, `multi_test/poseidon2_basic`, `multi_test/libm`, `multi_test/keccak_union_small` — each with `segments`, `user_cycles`, `total_cycles`, wall time, and `RISC0_INFO=1`/`RISC0_PRINT_SEGMENTS=1` telemetry.
- Run the matching Chrome/WebGPU async tests, all using `prove_with_opts_async`/`compress_async`, and confirm each produces a succinct receipt that verifies through the existing verifier, with cycle/segment parity to the native baseline.
- The smoke suite results land in the run's evidence directory and become the regression gate; any later commit that breaks any of these fixtures must be reverted or fixed before locking the affected phase.
- `cpu_only_ops` stays at `0` on every run; any non-zero `cpu_fallbacks` must be named in the run notes.

### `R2` Circuit-specific staged WGSL `eval_check` for rv32im

Description: Replace the interpreted WGSL `eval_check` path for rv32im with circuit-specific generated WGSL kernels, staged into a bounded number of dispatches with explicit intermediate buffers, mirroring the structure of `risc0/circuit/rv32im-sys/kernels/cuda/eval_check_*.cu`.

Acceptance criteria:
- Generate (or translate) rv32im `eval_check` WGSL kernels driven by the existing PolyExt metadata, partitioned into a small bounded set of staged kernels (target: ≤ 8 stages per circuit, no hundreds-of-pipelines split path).
- Each generated kernel passes a CPU-parity unit test against the portable Rust `eval_check` for tiny (`po2 = 0..=3`), medium (recursion- and segment-sized), and production-shaped (Keccak-sized 6741 FP slots, rv32im 927 FP slots) domains.
- The rv32im browser segment proof (`multi_test/poseidon2_basic`, `hello-world`, `json`) consumes the new path and records `rv32im_eval_check` GPU dispatches ≥ 1 with `cpu_fallbacks = 0` and no device-loss events.
- Chrome's negotiated 1 GiB buffer / 49 KiB workgroup-storage limits remain respected: peak per-pipeline workgroup storage ≤ `max_compute_workgroup_storage_size`, peak storage-binding ≤ `max_storage_buffer_binding_size`.
- The replaced interpreted path stays in-tree as a documented correctness fallback, gated behind an explicit feature flag or env var, not silently triggered by the proof path.

### `R3` Circuit-specific staged WGSL `eval_check` for recursion + tiled GPU-resident data group

Description: Replace the interpreted recursion `eval_check` plus the per-proof chunk upload of the ~512 MiB recursion data group with circuit-specific staged WGSL and a tiled multi-buffer GPU-resident representation. This is the dominant remaining wall-time contributor for `lift_prove` (currently ~2.9 s in `poseidon2_basic`, ~170 s in `libm`).

Acceptance criteria:
- Generated recursion `eval_check` WGSL is staged into ≤ 8 kernels with explicit intermediate buffers; passes CPU-parity at the same tiny/medium/production domains as R2; survives Chrome `po2 = 0..=21` smoke tests without device loss.
- The recursion data group is materialized as a tiled multi-buffer GPU-resident structure: each tile ≤ Chrome's negotiated `max_buffer_size` (1 GiB) and ≤ `max_storage_buffer_binding_size`; tiles are bind-grouped per stage; total upload volume per proof drops by ≥ 50% versus the current chunk-upload path.
- `gather_sample` re-enables GPU execution against tiled sources at recursion shapes (`rows = 2^20`, `cols = 128`); the existing fallback regression `webgpu_hal_recursion_sized_gather_sample_falls_back_to_cpu` is replaced by a passing GPU-side regression at the same shape.
- `lift_prove_async` wall time for `multi_test/poseidon2_basic` drops to ≤ 1.0 s in Chrome (from current 2.878 s); `recursion_eval_check` for `multi_test/libm` drops below 10 s (from 119.937 s).
- All affected paths preserve cycle/segment parity and verifier compatibility; no introduction of split-shader regressions documented in `learnings.md` (monolithic real shader, 251-chunk split, large private scratch).

### `R4` Circuit-specific staged WGSL `eval_check` for keccak

Description: After R2 and R3 are stable, replace the Keccak `eval_check` portable WASM fallback (currently 9 fallbacks per `KeccakUnion(1)` because the generic interpreter needs 6741 FP slots) with a circuit-specific staged WGSL implementation.

Acceptance criteria:
- Generated keccak `eval_check` WGSL stages cover the 6741-slot circuit without exceeding Chrome's `max_compute_workgroup_storage_size` (49152 bytes) per pipeline; the stage count is bounded (target: ≤ 12 stages), and each stage passes CPU parity through `webgpu_testutil::eval_check_webgpu_matches_portable`.
- `multi_test/keccak_union_small` (`KeccakUnion(1)`) browser proof records 0 `eval_check` CPU fallbacks and produces a verified succinct receipt.
- `multi_test/keccak_union` full `KeccakUnion(3)` browser proof completes within `WASM_BINDGEN_TEST_TIMEOUT=7200` (no SIGKILL/timeout), drives the wall-time ratio toward the 1.0× target per R10, receipt verifies, and cycles match native.
- No `cpu_fallbacks` regression for non-keccak ZKP bulk ops on the keccak fixtures.

### `R5` GPU-resident witness and accumulation for proof circuits

Description: Move recursion (then keccak, then rv32im) witness generation and accumulation off the Rust/WASM bridge path onto WebGPU where the bridge dominates wall time. Native CUDA does this with `*_witgen.cu`/`*_accumulate.cu` kernels.

Acceptance criteria:
- Recursion `recursion_witgen` + `recursion_accumulate` combined wall time on `multi_test/poseidon2_basic` drops to ≤ 100 ms in Chrome (from current 203 ms + 200 ms ≈ 403 ms).
- Keccak browser witness/accumulation path uses WebGPU dispatches for the dominant computation; per-fixture CPU mirror count drops by ≥ 50% versus the current `KeccakUnion(1)` baseline (`cpu_mirrors=188`).
- rv32im `rv32im_witgen` + `rv32im_accumulate` combined wall time on `multi_test/poseidon2_basic` drops to ≤ 100 ms (from current 86 ms + 241 ms ≈ 327 ms).
- All affected paths preserve correctness through new CPU-vs-WebGPU `CircuitHal` parity tests for representative witness/accumulation buffers (tiny/medium/production shapes).
- No regression to `cpu_only_ops = 0` invariant.

### `R6` Bounded transcript and Merkle readbacks

Description: Continue narrowing CPU-host-visible readback work at transcript boundaries. The current async public proof path keeps STARK commit/finalize GPU-authoritative, but Merkle query openings, FRI final coefficients, and DEEP evaluations still drive readbacks. Reduce per-proof readback volume and count without breaking verifier compatibility.

Acceptance criteria:
- `multi_test/poseidon2_basic` browser proof reaches ≤ 30 readbacks and ≤ 1 MiB readback bytes (from current 62 readbacks / ~430 KB) without changing receipt semantics.
- `multi_test/keccak_union_small` browser proof readback volume drops by ≥ 50% versus the current 2598714552 bytes baseline.
- Merkle/FRI/DEEP query batching coalesces sibling and value readbacks per tree per round (no per-element host calls).
- Pipeline caching, bind-group reuse, and buffer reuse contribute additively; the focused `poseidon2_basic` proof's total `buffer_bytes` allocation drops by ≥ 30% from the current 1.74 GiB baseline.
- No verifier-visible change to transcript order, query order, or sampled openings.

### `R7` Pipeline and buffer reuse without correctness loss

Description: Push pipeline caching and buffer reuse aggressively for stages with stable schemas (commit/finalize, FRI rounds, Merkle row hashing, NTT/iNTT).

Acceptance criteria:
- Pipeline-creation count per browser proof drops by ≥ 50% on the smoke fixtures (R1) vs the current branch baseline; ratio measured from `webgpu` HAL diagnostics.
- Bind-group reuse covers FRI rounds (`fri_fold`, `fri_query`), Merkle row hashing, NTT/iNTT, and combo prepare/divide.
- Browser proof wall time on `multi_test/poseidon2_basic` does not regress from the R3/R5 floor when this optimization lands.
- Pipeline cache is correctness-safe: switching `ProverOpts`, segment dimensions, or recursion shape across proofs must not reuse a stale pipeline whose constants/binding layout no longer match.

### `R8` Production `gather_sample` chunking with tiled multi-buffer source

Description: Replace the CPU fallback gated by `GPUSupportedLimits.maxBufferSize` with a tiled multi-buffer GPU `gather_sample` path. This is a precondition for R3's recursion-sized GPU gather.

Acceptance criteria:
- `gather_sample` operates over a tiled `Vec<GPUBuffer>` representation; the recursion-sized regression (`rows = 2^20`, `cols = 128`, 512 MiB total) produces non-zero, CPU-parity-correct output in Chrome.
- The current locked fallback regression `webgpu_hal_recursion_sized_gather_sample_falls_back_to_cpu` is replaced by a passing GPU regression at the same dimensions.
- `multi_test/poseidon2_basic` and `multi_test/libm` `gather_sample` fallbacks drop to zero (`cpu_fallbacks=0` for `gather_sample`).
- HAL diagnostics report tile count, per-tile bytes, and per-tile binding usage in the standard `webgpu:` log line.

### `R9` Restore deferred parity matrix

Description: The correctness run paused leaving these fixtures deferred or timing out: `multi_test/rsa_compat`, `multi_test/keccak_union` (`KeccakUnion(3)`), `groth16-verifier`, `xgboost`, `bn254`, `risc0-zkvm-methods/blst`, `risc0-zkvm-methods/verify`. They must reach passing browser/WebGPU succinct receipts as part of this run.

Acceptance criteria:
- Each named fixture has a fresh native CUDA baseline (segments, user_cycles, total_cycles, wall time).
- Each named fixture's Chrome/WebGPU async browser test produces a succinct receipt, verifies with the existing verifier, and matches native cycle/segment counts.
- Each fixture's wall time satisfies the per-class ratio bounds in R10 below; any fixture that cannot meet the bound after R2–R8 lands must have a documented structural explanation in `docs/wasm-webgpu-cuda-comparison.md` (e.g., browser CI runner budget) before being marked accepted under this run.
- Validation matrix in `docs/wasm-webgpu-validation.md` is refreshed for each fixture.
- `RunUnconstrained { unconstrained: true }` remains classified as native-disabled (per existing exclusion rationale).

### `R10` Wall-time parity with native CUDA (1.0× ideal target)

Description: The ideal wall-time target for every fixture class is **1.0× native CUDA** on the user's RTX 5090 + Chrome/WebGPU hardware. The run drives the gap toward 1.0× by exhausting the levers in R2–R8, and documents any residual ratio above 1.0× with an explicit structural cause. There is no per-class permissive cutoff; the closing condition is lever exhaustion plus a monotone-decreasing trend on the smoke fixtures.

Acceptance criteria:
- For every fixture named in R1 (smoke) and R9 (deferred matrix), plus the full public example matrix and internal parity matrix already passing in `docs/wasm-webgpu-validation.md`, Chrome/WebGPU wall time is measured against a fresh native CUDA baseline and recorded as a ratio in `docs/wasm-webgpu-validation.md`.
- The closing condition is: **every lever in R2–R8 has landed**, AND **no further unimplemented lever in the run's lever ledger can be applied without violating R11 (forbidden regressions) or R13 (correctness invariants)**, AND **any residual ratio > 1.0× has a documented structural cause** (e.g., Chrome's 49 KiB workgroup-storage cap, async readback unavoidability at transcript boundaries, single-`GPUBuffer` size ceiling, async device-loss thresholds, browser CI runner overhead).
- No fixture regresses against its R1 smoke baseline; the trend across the implementation phase is **monotone-decreasing** on the focused smoke fixtures (`poseidon2_basic`, `libm`, `keccak_union_small`).
- The final run notes summarize the ratio per fixture, identify the dominant residual structural cost for each, and explicitly call out any fixture where 1.0× was effectively reached versus where structural residuals remain.
- All measurements use the user's RTX 5090 + Chrome/WebGPU with the negotiated 1 GiB `maxBufferSize` and `maxStorageBufferBindingSize`; ratios are not normalized or retried under degraded WebGPU limits.
- Each measurement records native CUDA wall time, browser wall time, ratio, segment count, cycle count, gpu_dispatches, cpu_mirrors, cpu_fallbacks, cpu_only_ops, upload_bytes, readback_bytes, and Chrome's negotiated limits.

### `R11` Failure-mode hygiene and forbidden regressions

Description: Carry forward the negative lessons from the paused run. The codebase has a documented ledger of failed experiments; this run must not silently re-enable them.

Acceptance criteria:
- `WEBGPU_EVAL_CHECK_ENABLE_SPLIT` remains `false` unless this run adds a circuit-specific staged path that demonstrates no device loss across all R10 fixtures over ≥ 3 consecutive Chrome runs.
- `WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS` does not exceed 1536 unless paired with measured performance evidence that the larger value beats the staged WGSL path on the affected fixture.
- No silent CPU fallback re-enabling: every existing `cpu_fallbacks` site that this run removes must stay at zero on the smoke suite, and any new fallback must be named in the HAL diagnostics with a typed reason.
- WebGPU device loss is treated as a first-class typed error (per existing R3 from the correctness run); no silent retry-with-shrunk-shader masking.
- No reintroduction of synchronous proof helpers as the public browser parity path; the harness keeps `prove_with_opts_async`, `compress_async`, and friends as the gate.
- No reintroduction of CPU shadow updates after every GPU dispatch on the focused async path (the current path uses dirty-bit tracking; reverting to per-dispatch mirror is a regression).

### `R12` Performance telemetry and CUDA-side baselines

Description: Make per-fixture native CUDA and Chrome/WebGPU measurements first-class artifacts under this run. The resume plan in `learnings.md` calls for native baseline before browser proof on every fixture; this requirement formalizes that.

Acceptance criteria:
- For each fixture in R9 and R10, the run records: native CUDA command + wall time, browser test command + wall time, ratio, `RISC0_INFO=1`/`RISC0_PRINT_SEGMENTS=1` telemetry, Chrome browser version + GPU adapter info, negotiated WebGPU limits, full HAL diagnostics counters.
- Measurements land under `/.recursive/run/wasm-webgpu-prover-perf/evidence/perf/` per the standard run artifact layout.
- `docs/wasm-webgpu-validation.md` and `docs/wasm-webgpu-cuda-comparison.md` are updated with refreshed numbers, replacing stale paused-state baselines per fixture.
- Per-stage telemetry (`segment_prove_core_async`, `lift_prove_async`, `recursion_eval_check`, `rv32im_eval_check`, etc.) is recorded for at least `poseidon2_basic`, `libm`, and `keccak_union_small`.

### `R13` Correctness invariants preserved end-to-end

Description: The acceptance gate from `docs/requirements/wasm-webgpu-prover.md` (R1–R18) remains binding. No optimization may relax it.

Acceptance criteria:
- All currently-passing fixtures in `docs/wasm-webgpu-validation.md` (the public example matrix and the internal parity matrix) continue to pass at the end of this run.
- All produced receipts remain non-dev-mode, succinct, and verify through the existing verifier without verifier branches.
- No changes to receipt formats, seal encoding, control IDs, claim semantics, or verifier behavior land in this run.
- The browser backend remains explicit (no silent fallback to Bonsai, CUDA, Metal, fake receipts, dev-mode); device loss and unsupported-limit cases return typed errors as before.
- `docs/wasm-webgpu-prover.md` user-facing API description stays accurate after each change.

## Out of Scope

- `OOS1`: Native `wgpu` (desktop) as the acceptance target. Browser `wasm32-unknown-unknown` + Chrome WebGPU remains the gate; desktop `wgpu` may exist only as a developer convenience if added orthogonally.
- `OOS2`: Hard blocking on exact 1.0× CUDA parity for every fixture. The run drives toward 1.0× as the ideal target (R10), but accepts documented structural residuals (workgroup-storage cap, async readback, single-`GPUBuffer` ceiling, async device-loss thresholds) as termination causes once every lever in R2–R8 has been applied.
- `OOS3`: Firefox + Safari automation and parity. Per `docs/requirements/wasm-webgpu-prover.md` R13, those are deferred until Chrome is stable and shader resource usage is bounded; this run does not change that.
- `OOS4`: Verifier, receipt format, control ID, or seal encoding changes.
- `OOS5`: JavaScript SDK or application framework. The Rust/WASM API surface is the contract.
- `OOS6`: Browser-side Groth16 proving or shrink-wrapping.
- `OOS7`: Executor-only examples (`bls12_381`, `c-kzg`, `profiling`, `browser-verify`) and native-disabled fixtures (`RunUnconstrained { unconstrained: true }`).

## Constraints

- Target: `wasm32-unknown-unknown` for browser builds; Chrome WebGPU with negotiated 1 GiB `maxBufferSize` and `maxStorageBufferBindingSize`, 49152-byte `maxComputeWorkgroupStorageSize`.
- Native CUDA baselines run on the user's RTX 5090 (32 GB VRAM, SM120/Blackwell). Local prover, `--features cuda`, `RISC0_PROVER=local`, `RISC0_EXECUTOR=local`.
- Reuse existing `risc0_zkp::hal::Hal`, `CircuitHal`, `WebGpuHal`, `WebGpuProver`, prover server, recursion, receipt, verifier abstractions. New abstractions only when WebGPU constraints force them.
- No fake receipts, no dev-mode, no Bonsai/CUDA/Metal silent fallback in browser acceptance.
- No verifier-visible change.
- No network access required by acceptance tests.
- No re-enabling of shelved experiments (monolithic generated `eval_check`, hundreds-of-pipelines split path, large private scratch as primary strategy) without dedicated evidence per R11.
- The run inherits and continues to honor the correctness requirements from `docs/requirements/wasm-webgpu-prover.md` (R1–R18).
- Browser CI test runner budget: per-fixture wall time should stay within `WASM_BINDGEN_TEST_TIMEOUT=7200`; targets in R10 are tighter than that ceiling.
- Treat `/.recursive/` control plane as the durable run home; this run is the first under `/.recursive/run/`.
- Per the recursive-mode workflow, isolate implementation in a worktree (e.g., `.worktrees/wasm-webgpu-prover-perf/`) before any Phase 3 code change; keep `main` and the `wasm` branch clean during exploratory Phase 1 work.

## Coverage Gate

- [X] Every requirement R1–R13 has at least one observable acceptance criterion.
- [X] All performance targets in R10 map to specific fixtures listed in `docs/wasm-webgpu-validation.md`.
- [X] Out-of-scope items OOS1–OOS7 reference the source policy doc that already excludes them (correctness requirements R1–R18 or pause handoff).
- [X] Constraints list reflects current Chrome/WebGPU negotiated limits, hardware, and verifier compatibility rules.
- [X] No requirement contradicts the correctness gate inherited from `docs/requirements/wasm-webgpu-prover.md`.

Coverage: PASS

## Approval Gate

- [X] User has reviewed the requirement set and confirmed scope.
- [X] User has confirmed the wall-time parity target in R10 (1.0× CUDA as ideal for every fixture, closing on lever exhaustion + documented structural residuals).
- [X] User has confirmed the order-of-attack implied by R2 → R3 → R5 → R8 → R4 → R6/R7 → R9.
- [X] User has confirmed that Firefox/Safari portability stays out of scope for this run.
- [X] User has confirmed run id `wasm-webgpu-prover-perf` and approves running `scripts/recursive-init.py --run-id wasm-webgpu-prover-perf --template feature`.

Approval: PASS
