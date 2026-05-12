Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `01 AS-IS`
Status: `LOCKED`
LockedAt: `2026-05-12T05:07:52Z`
LockHash: `6ae676746bbd68fe6d9b724d493cdcfa968d04235ae8e40294c3324da67a743d`
Workflow version: `recursive-mode-audit-v2`
Inputs:
- `/.recursive/run/wasm-webgpu-prover-perf/00-requirements.md` (LOCKED 2026-05-12)
- `/.recursive/run/wasm-webgpu-prover-perf/00-worktree.md` (LOCKED 2026-05-12)
- `docs/requirements/wasm-webgpu-prover.md` (R1–R18 correctness seed, approved 2026-05-08)
- `docs/wasm-webgpu-prover-learnings.md` (pause handoff, failed-experiment ledger)
- `docs/wasm-webgpu-validation.md` (correctness + per-fixture native CUDA baselines)
- `docs/wasm-webgpu-cuda-comparison.md` (per-stage telemetry)
- `docs/wasm-webgpu-prover.md` (user-facing API + architecture description)
- `risc0/zkp/src/hal/webgpu.rs`
- `risc0/circuit/{rv32im,keccak,recursion}/src/prove/hal/webgpu.rs`
- `risc0/circuit/{rv32im-sys,keccak-sys,recursion-sys}/kernels/cuda/eval_check*.cu`
- `risc0/circuit/recursion-sys/kernels/metal/eval_check.metal`
- `risc0/circuit/{rv32im,keccak,recursion}/src/{zirgen/,}poly_ext.rs`
- `risc0/zkvm/src/host/server/prove/prover_impl.rs`
- `risc0/zkp/src/prove/{merkle.rs,fri.rs,poly_group.rs,prover.rs}`
- `risc0/zkp/src/adapter.rs:149`–255
- `examples/browser-prove/src/lib.rs:732`–741
Outputs:
- `/.recursive/run/wasm-webgpu-prover-perf/01-as-is.md`
Scope note: Captures the present-state substrate of the WebGPU browser prover and indexes every R# obligation against its current implementation evidence so Phase 2 can plan concrete changes per requirement.

## TODO

- [x] Reread Phase 0 artifacts and confirm worktree FF'd to controller HEAD `454b3109b`.
- [x] Reread `docs/wasm-webgpu-prover-learnings.md`, `docs/wasm-webgpu-validation.md`, `docs/wasm-webgpu-cuda-comparison.md`, `docs/requirements/wasm-webgpu-prover.md`.
- [x] Map WebGPU ZKP HAL (buffer model, op coverage, pipeline cache, eval_check hook, diagnostics).
- [x] Map browser CircuitHals (witness/accumulation/eval_check routing).
- [x] Map native CUDA reference `eval_check` kernels.
- [x] Map interpreted WGSL `eval_check` path and disabled split/straight-line generators.
- [x] Map async proof readback sites.
- [x] Index R1–R13 and inherited R18 obligations in the Source Requirement Inventory.
- [x] Enumerate Known Unknowns.
- [x] Complete required audited-phase sections.
- [x] Complete Coverage Gate / Approval Gate.

## Source Requirement Inventory

- R1 | Disposition: in-scope | Source Quote: "Rebuild the wasm harness" | Summary: Lock a six-fixture smoke suite (native CUDA + Chrome WebGPU) as the regression line before any optimization commit.
- R2 | Disposition: in-scope | Source Quote: "Circuit-specific staged WGSL `eval_check` for rv32im" | Summary: Replace the interpreted WGSL eval_check path for rv32im with circuit-specific generated WGSL staged into ≤ 8 kernels with explicit intermediate buffers.
- R3 | Disposition: in-scope | Source Quote: "Circuit-specific staged WGSL `eval_check` for recursion + tiled GPU-resident data group" | Summary: Generate recursion-specific staged WGSL and replace the per-proof chunk upload of the ~512 MiB recursion data group with a tiled multi-buffer GPU-resident representation.
- R4 | Disposition: in-scope | Source Quote: "Circuit-specific staged WGSL `eval_check` for keccak" | Summary: After R2+R3 stable, generate keccak-specific staged WGSL bounded to ≤ 12 stages so KeccakUnion(1) reports 0 eval_check CPU fallbacks and KeccakUnion(3) completes within the runner budget.
- R5 | Disposition: in-scope | Source Quote: "GPU-resident witness and accumulation for proof circuits" | Summary: Move recursion, then keccak, then rv32im witness generation + accumulation onto WebGPU so the Rust/WASM bridge stops dominating segment + recursion stage budgets.
- R6 | Disposition: in-scope | Source Quote: "Bounded transcript and Merkle readbacks" | Summary: Cut per-proof readback count + bytes by R6's listed thresholds without changing verifier-visible transcript semantics.
- R7 | Disposition: in-scope | Source Quote: "Pipeline and buffer reuse without correctness loss" | Summary: Achieve ≥ 50% pipeline-creation reduction and persistent bind-group/buffer reuse across stages with stable schemas.
- R8 | Disposition: in-scope | Source Quote: "Production `gather_sample` chunking with tiled multi-buffer source" | Summary: Replace the locked CPU fallback for oversized gather_sample sources with a tiled multi-buffer GPU path that handles recursion-sized matrices.
- R9 | Disposition: in-scope | Source Quote: "Restore deferred parity matrix" | Summary: Bring rsa_compat, KeccakUnion(3), groth16-verifier, xgboost, bn254, BLST, and verify back to passing succinct receipts under the new performance path.
- R10 | Disposition: in-scope | Source Quote: "Wall-time parity with native CUDA (1.0× ideal target)" | Summary: Drive wall-time toward 1.0× CUDA on every fixture class; close on lever exhaustion + documented structural residuals.
- R11 | Disposition: constraint | Source Quote: "Failure-mode hygiene and forbidden regressions" | Summary: Forbidden-regression flags and shelved experiments stay shelved unless dedicated evidence justifies a change.
- R12 | Disposition: in-scope | Source Quote: "Performance telemetry and CUDA-side baselines" | Summary: Record native CUDA + Chrome WebGPU measurements (HAL counters, stage timings, negotiated limits) as first-class run artifacts under `evidence/perf/`.
- R13 | Disposition: constraint | Source Quote: "Correctness invariants preserved end-to-end" | Summary: The R1–R18 acceptance gate from `docs/requirements/wasm-webgpu-prover.md` remains binding; no receipt-format, verifier, control ID, or claim change permitted.
- R18 | Disposition: out-of-scope | Source Quote: "(R1–R18) remains binding" | Summary: R14–R18 of the upstream correctness requirements (testing, documentation, telemetry baseline) are inherited as a constraint via R13; they are not new deliverables of this run.

## Audit Context

Audit Execution Mode: `self-audit`
Subagent Availability: `available`
Subagent Capability Probe: Explore subagents available via the controller's Agent tool with `subagent_type=Explore`. Five were dispatched in parallel for read-only substrate mapping during this phase. Router config (`.recursive/config/recursive-router.json`) is present in the worktree.
Delegation Decision Basis: AS-IS narrative requires synthesis across five disjoint code surfaces (HAL, CircuitHals, CUDA kernels, async proof, PolyExt metadata). Five Explore subagents were dispatched in parallel to keep controller context focused on synthesis. Delegated reads are bounded (file:line pointers; no edits).
Delegation Override Reason: None; subagents were used as documented in Subagent Contribution Verification.
Audit Inputs Provided:
- Phase 0 artifacts: `00-requirements.md` (LOCKED, hash `d98a93e75bc9122b110d42e7c33ecc66d60b438828e585babdf5a9b35ba042fc`), `00-worktree.md` (LOCKED, hash `71fbfa4cedfc54f30c9e59e28fd54a59a1197e7d1bdf29faeecc8ec029d8d536`)
- Diff basis: `git diff --name-only d042da45c89a1cd5f9cf7c5eb962a754367b2118` (executable from worktree)
- Worktree HEAD: `454b3109b recursive run 0` (post-Phase 0 commit FF'd into `recursive/wasm-webgpu-prover-perf`)
- Targeted code references: full list under `## Relevant Code Pointers`.

## Effective Inputs Re-read

- `00-requirements.md` (R1–R13 + constraints + OOS) → drove Source Requirement Inventory dispositions.
- `00-worktree.md` (Phase 0 setup, baseline build evidence, diff basis) → confirmed the smoke artifact path used in repro steps.
- `docs/wasm-webgpu-prover-learnings.md` → primary source for R2/R3/R4 disposition and R11's constraint encoding.
- `docs/wasm-webgpu-validation.md` → public example matrix and per-fixture native CUDA baselines.
- `docs/wasm-webgpu-cuda-comparison.md` → per-stage telemetry tables for `poseidon2_basic` and `libm`.
- `docs/requirements/wasm-webgpu-prover.md` → inherited R1–R18 correctness requirements (R18 reference indexed for traceability).

## Prior Recursive Evidence Reviewed

None applicable. Justification: this is the first run under the run-folder system, so no prior recursive runs, addenda, or memory shards reference this subsystem. The memory router at `.recursive/memory/MEMORY.md` was consulted and confirms no shards apply because the registry under `domains/`, `patterns/`, `incidents/`, `episodes/`, `skills/`, and `archive/` is empty for this subsystem. The reason no prior recursive evidence exists is that `.recursive/STATE.md` and `.recursive/DECISIONS.md` had only their scaffolded placeholder content at Phase 0 init.

## Earlier Phase Reconciliation

This run has only Phase 0 prior to AS-IS. Both Phase 0 artifacts are LOCKED and verify against their content hashes (verified via `verify-locks.py` after Phase 0 lock):
- `00-requirements.md`: LockHash `d98a93e75bc9122b110d42e7c33ecc66d60b438828e585babdf5a9b35ba042fc`, hash stable across the FF to controller HEAD.
- `00-worktree.md`: LockHash `71fbfa4cedfc54f30c9e59e28fd54a59a1197e7d1bdf29faeecc8ec029d8d536`, hash stable across the FF.

The Phase 0 diff basis (`d042da45c89a1cd5f9cf7c5eb962a754367b2118`) remains executable. After FF, the diff basis still resolves to the same commit; the Phase 0 artifacts now appear in `git diff --name-only` against the basis because they are committed in `454b3109b` (post-basis), which is expected and filtered out of drift checks by `filter_runtime_changed_files` for run-local paths.

The R10 target (1.0× ideal CUDA parity) is preserved verbatim from the locked requirements. No reinterpretation occurred.

## Reproduction Steps (Novice-Runnable)

All commands run from `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf/` unless noted.

### Wasm build gate (R1 substrate)

```bash
cd /home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release --no-run
```

Expected: exit 0 in ~13 min (clean) or ~2 min (incremental). Produces `examples/target/wasm32-unknown-unknown/release/deps/browser_prove-ea184bbfc45f01b0.wasm`.

### Native CUDA baseline for poseidon2_basic (R12 process)

```bash
RECURSION_SRC_PATH=$(pwd)/examples/target/release/build/risc0-circuit-recursion-*/out/recursion_zkr.zip \
RISC0_PROVER=local RISC0_EXECUTOR=local RISC0_INFO=1 RUST_LOG=info RISC0_PRINT_SEGMENTS=1 \
cargo test --manifest-path examples/browser-prove/Cargo.toml --release --features cuda \
  native_stats_tests::native_poseidon2_basic_prove_stats -- --ignored --nocapture
```

Expected: ~426 ms wall time, 1 segment, 3598 user cycles, 32768 total cycles.

### Chrome WebGPU baseline for poseidon2_basic (R1, R10)

```bash
cd /home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf/examples/browser-prove
WASM_BINDGEN_TEST_TIMEOUT=1200 \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
/home/rami/.cache/.wasm-pack/wasm-bindgen-c59d5019a2b42393/wasm-bindgen-test-runner \
  --nocapture \
  ../../examples/target/wasm32-unknown-unknown/release/deps/browser_prove-ea184bbfc45f01b0.wasm \
  native_poseidon2_basic_async_succinct_receipt_verify
```

Expected: succinct receipt in ~4.0 s with `eval_check gpu_dispatches=2 cpu_fallbacks=0`.

### Keccak eval_check WebGPU vs portable parity (R4 baseline)

```bash
cd /home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf/examples/browser-prove
WASM_BINDGEN_TEST_TIMEOUT=1200 \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
/home/rami/.cache/.wasm-pack/wasm-bindgen-c59d5019a2b42393/wasm-bindgen-test-runner \
  --nocapture \
  ../../examples/target/wasm32-unknown-unknown/release/deps/browser_prove-ea184bbfc45f01b0.wasm \
  keccak_eval_check_poly_ext_matches_cpu
```

Expected: pass in ~40 s for `po2 = 14`, `fp_slots = 6741`, workgroup_size = 1.

## Current Behavior by Requirement

### R1, R12 — Telemetry pipeline (PARTIAL)

Diagnostic pipeline emits per-stage timing (`browser-prove:stage start|done … elapsed_ms=…`) and per-op counters (`browser-prove:webgpu gpu_dispatches=… cpu_mirrors=… cpu_fallbacks=…`) automatically. Native CUDA baselines run with `RISC0_INFO=1` produce comparable `native_prove name=… elapsed=… segments=… user_cycles=… total_cycles=…` lines. Substrate is in place; the discipline (recording these under `evidence/perf/`) is the gap.

### R2 — rv32im eval_check (OPEN)

Today: rv32im `eval_check` hits the WebGPU interpreter (`EVAL_CHECK_BASE_INTERPRETER_WGSL`, dispatched via `dispatch_eval_check_poly_ext_interpreted_with_groups` at `risc0/zkp/src/hal/webgpu.rs:5138`–5250). Telemetry: 1 dispatch per call (1 ms for poseidon2_basic). For larger circuits (libm: 20,202 instructions, 927 fp_slots), interpreter takes 32.5 s because every lane re-evaluates the full PolyExtStepDef bytecode.

Native CUDA path: 4 staged kernels (`eval_check_{0,1,2,3}.cu`), each a fully unrolled constraint group function. ~26.7K lines total. 4 separate `<<<grid,block>>>` launches with explicit intermediate buffers.

### R3 — recursion eval_check + tiled data group (OPEN)

Today: recursion `eval_check` uses the same interpreter as rv32im. For poseidon2_basic, interpreter takes 1 ms; for libm, 119.9 s. Recursion data group (~512 MiB) is uploaded per proof through `dispatch_gather_sample_chunked` and bounded-chunk uploads in `mix_poly_coeffs`/`batch_evaluate_any`. Recursion-sized `gather_sample` fallback test (`webgpu_hal_recursion_sized_gather_sample_falls_back_to_cpu`) is locked at CPU fallback (`risc0/zkp/src/hal/webgpu.rs:6879`).

Native CUDA path: monolithic kernel `risc0/circuit/recursion-sys/kernels/cuda/eval_check.cu` (12,381 lines). Memory model: separate `const Fp*` for ctrl/data/accum/out/mix bound at launch.

### R4 — keccak eval_check (OPEN, blocked on R2/R3)

Today: keccak `eval_check` runs the interpreter in workgroup-shared mode because `fp_slots = 6741 > 1536`. Workgroup_size = 1 → severe under-utilization. Per `KeccakUnion(1)`: 9 CPU fallbacks, ~441 s vs 7.47 s native CUDA. Split path (`build_eval_check_split_wgsl` at `risc0/zkp/src/hal/webgpu.rs:2142`) is disabled (`WEBGPU_EVAL_CHECK_ENABLE_SPLIT = false`).

Native CUDA path: 5 files, 57 constraint functions, ≈91.1K lines. `kNumPolyMixPows = 2175` (4.7× rv32im's count).

### R5 — witness/accumulation (OPEN)

Today: every circuit's witness gen + accumulation routes through Rust/WASM bridges:
- rv32im witness: `risc0/circuit/rv32im/src/prove/hal/webgpu.rs:91` → `super::rust_steps::generate_witness()`
- rv32im accumulate: `risc0/circuit/rv32im/src/prove/hal/webgpu.rs:110` → `super::rust_steps::step_accum()`
- keccak witness: `risc0/circuit/keccak/src/prove/hal/webgpu.rs:131` → `super::rust_steps::generate_witness()`
- keccak accumulate: `risc0/circuit/keccak/src/prove/hal/webgpu.rs:136`–146 (no-op stub)
- recursion witness: `risc0/circuit/recursion/src/prove/hal/webgpu.rs:87` → `super::rust_kernels::generate_witness()`
- recursion accumulate: `risc0/circuit/recursion/src/prove/hal/webgpu.rs:114` → `super::rust_kernels::accumulate()`

Stage telemetry: `rv32im_witgen=86ms, rv32im_accumulate=241ms, recursion_witgen=203ms, recursion_accumulate=200ms`.

### R6 — readbacks (PARTIAL)

Today: Merkle root (1×, 32 B at `merkle.rs:272`); Merkle top (~3 KiB at `merkle.rs:296`); batched query openings in `prove_batch_async` (`merkle.rs:352`–422, all samples + all siblings in one indexed readback each); FRI final coefs (~2 KiB at `fri.rs:190`); DEEP outputs (~4 KiB × 2 at `prover.rs:625`, 667). Total for poseidon2_basic: ~62 readbacks / 431,840 bytes. R6 target: ≤ 30 readbacks / ≤ 1 MiB.

### R7 — pipeline/buffer reuse (PARTIAL)

Today: only eval_check interpreter pipelines cached (`eval_check_interpreter_pipelines: RefCell<BTreeMap<…>>` at `risc0/zkp/src/hal/webgpu.rs:854`–855). No LRU, per-HAL. No bind-group cache; bind groups created fresh per dispatch. NTT, FRI fold, Merkle hashing, combos_prepare/divide each create pipelines via own helpers.

### R8 — gather_sample tiling (PARTIAL)

Today: single-buffer chunked helper exists (`dispatch_gather_sample_chunked` at `risc0/zkp/src/hal/webgpu.rs:8014`; `debug_dispatch_gather_sample_chunked` at 6908). Source-size gate at `can_dispatch_gather_sample:6351`. Locked CPU fallback at `gather_sample_async:6879`. Multi-buffer source representation missing.

### R9 — deferred parity (OPEN)

Today: `multi_test/rsa_compat`, `KeccakUnion(3)`, `groth16-verifier`, `xgboost`, `bn254`, BLST, verify — all deferred/timing out per `docs/wasm-webgpu-validation.md:92`–94, 119, 131, 134–135.

### R10 — wall-time parity (OPEN)

Today: `poseidon2_basic` 9.4×, `libm` 491×, `KeccakUnion(1)` 59×, moderate examples ~10×.

### R11 — forbidden regressions (PARTIAL as constraint)

Today: guard flags at `risc0/zkp/src/hal/webgpu.rs:75`–81 — `WEBGPU_EVAL_CHECK_MAX_POLY_EXT_STEPS = 4096`, `WEBGPU_EVAL_CHECK_ENABLE_SPLIT = false`, `WEBGPU_EVAL_CHECK_SPLIT_TERMS_PER_SHADER = 64`, `WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS = 1536`.

### R13 — correctness invariants (IN_PLACE as constraint)

Today: 21 public examples + internal parity matrix verified per `docs/wasm-webgpu-validation.md`.

### R18 — inherited from `docs/requirements/wasm-webgpu-prover.md` (out-of-scope)

R18 in upstream requirements is "Documentation". Per R13, the original R1–R18 acceptance gate remains binding. R18 documentation updates land naturally as part of this run's documentation hygiene; it is not a separate deliverable of this run.

## Relevant Code Pointers

WebGPU HAL substrate (`risc0/zkp/src/hal/webgpu.rs`):
- Buffer model: `WebGpuBuffer<T>` 4621; `cpu_dirty` 4626; `cpu_stale` 4627; `mark_gpu_dirty()` 4659; `sync_cpu_to_gpu()` 4699; `mark_synced()` 4665; `cpu_is_current()` 4681; `gpu_authoritative_scope()` 4968.
- Op WGSL + dispatch: NTT_STEP 3691 / batch_interpolate_ntt 8980; NTT_NORMALIZE 3816; BATCH_BIT_REVERSE 3600 / 9110; BATCH_EXPAND 3651 / 8833; BATCH_EVALUATE_ANY 3883 / 9175; FRI_FOLD 2890 / 8156; MIX_POLY_COEFFS 3111 / 8286; COMBOS_PREPARE 3255 / 8547; COMBOS_DIVIDE 3447 / 8704; SCATTER_ELEM 2723 / 7826; GATHER_SAMPLE_ELEM 2756 / 7937; PREFIX_PRODUCTS_EXTELEM 2788 / 8121; POSEIDON2 4235 / 9290–9360; EVAL_CHECK_BASE_INTERPRETER_WGSL 1329–1596 / dispatch 5138–5250.
- Eval_check guards: lines 75–81.
- Straight-line generator: `build_eval_check_wgsl` 1907–2140.
- Split generator (disabled): `build_eval_check_split_wgsl` 2142.
- Pipeline cache: field 854–855; hit 5011; insert 5047; lookup helper 4996.
- Workgroup gates: `max_compute_workgroup_storage_size` 4853; `eval_check_base_workgroup_lanes()` 4982; private cap check 5175; `storage_binding_fits` 5091.
- Diagnostics: `WebGpuDiagnosticsState` 2430–2446; snapshot 2449.
- gather_sample fallback: gate 6351–6370; async entry 6844; fallback log 6879.

Browser CircuitHals:
- rv32im struct `risc0/circuit/rv32im/src/prove/hal/webgpu.rs:48`; `CircuitWitnessGenerator` 74–93; `CircuitAccumulator` 95–112; `CircuitHal` 114–153; `WebGpuCircuitEvalCheck` 50–72; `gpu_authoritative_scope` 246, 265.
- keccak struct `risc0/circuit/keccak/src/prove/hal/webgpu.rs:50`; `CircuitWitnessGenerator` 81–133; accumulate no-op 136–146; `CircuitHal` 135–173; `WebGpuCircuitEvalCheck` 57–79.
- recursion struct `risc0/circuit/recursion/src/prove/hal/webgpu.rs:42`; `CircuitWitnessGenerator` 68–97; `CircuitAccumulator` 99–116; `CircuitHal` 118–157; `WebGpuCircuitEvalCheck` 44–66.
- Scoped HAL binding: `risc0/zkvm/src/host/client/prove/webgpu.rs:34` `webgpu_prover`; `WebGpuProver::with_circuit_hal` 120–126; per-circuit thread-local `risc0/circuit/{circuit}/src/prove/mod.rs:with_webgpu_hal`.

Native CUDA references:
- rv32im: `risc0/circuit/rv32im-sys/kernels/cuda/eval_check_{0,1,2,3}.cu` (7858+6426+6260+6142 = 26,686 lines).
- keccak: `risc0/circuit/keccak-sys/kernels/cuda/eval_check_{0..4}.cu` (~91,121 lines, 57 constraint functions).
- recursion: `risc0/circuit/recursion-sys/kernels/cuda/eval_check.cu` (12,381 lines).
- recursion Metal: `risc0/circuit/recursion-sys/kernels/metal/eval_check.metal` (43 lines).

PolyExt metadata:
- Adapter types: `risc0/zkp/src/adapter.rs:149`–255 (`PolyExtStepDef`, `PolyExtStep`).
- rv32im DEF: `risc0/circuit/rv32im/src/zirgen/poly_ext.rs:26` (20,202 instructions, 927 fp_slots).
- keccak DEF: `risc0/circuit/keccak/src/zirgen/poly_ext.rs:26` (6741 fp_slots).
- recursion DEF: `risc0/circuit/recursion/src/poly_ext.rs:26` (1000 fp_slots).
- Portable CPU reference: `risc0/zkp/src/hal/portable.rs:27`.

Async proof readback substrate:
- Entry: `WebGpuProver::prove_with_opts_async` at `risc0/zkvm/src/host/client/prove/webgpu.rs:136`–143.
- Session: `prover_impl.rs:132`–154 `prove_with_ctx_async`; `prove_session_async` 193; `prove_segment_core_async` 353; `composite_to_succinct_async` 512.
- Stage timer: `WebGpuStageTimer` `prover_impl.rs:56`–83.
- Merkle: `commit_async` root readback `merkle.rs:272`; top-layer 296; `prove_async` 321, 339, 405; `prove_batch_async` 352–422.
- FRI: final coefs `fri.rs:190`.
- DEEP: `batch_evaluate_any_async` `prover.rs:625`, 667.
- Finalize: `finalize_async` `prover.rs:694`–773.

Helper tests:
- Keccak parity: `risc0/circuit/keccak/src/lib.rs:82`–157; harness `examples/browser-prove/src/lib.rs:732`–741.

## Subagent Contribution Verification

Five Explore subagents dispatched in parallel during this phase (no subagent action records under `subagents/` are required because no delegated production-edit work occurred; substrate mapping is read-only).

1. **WebGPU HAL surface** (`risc0/zkp/src/hal/webgpu.rs`): reported buffer model, op coverage with file:line, pipeline cache, workgroup-storage gates, gather_sample fallback, eval_check hook, diagnostics. Spot-checked line pointers (4621 buffer struct, 854 pipeline cache, 76 split flag, 81 private slot cap, 6879 gather fallback) align with repo content.
2. **Browser CircuitHals**: reported rust_steps/rust_kernels routing, no-op keccak accumulate stub, dual eval_check paths (WebGpuCircuitEvalCheck trait vs CircuitHal::eval_check synchronous fallback), scoped HAL binding. Spot-checked: rv32im struct 48, recursion struct 42, keccak accumulate stub 136–146.
3. **CUDA reference kernels**: reported per-circuit file/line counts (rv32im 4 / 26.7K; keccak 5 / 91.1K; recursion 1 / 12.4K), signatures, memory model, Metal wrapper. Spot-checked: recursion eval_check.cu line count and Metal file size match.
4. **Async proof readbacks**: reported entry chain, readback inventory by site, batched query openings, GPU-authoritative scopes, stage timer pattern. Spot-checked: `merkle.rs:380` (batched samples), `fri.rs:190` (final coefs) align.
5. **PolyExt metadata + WGSL interpreter**: reported PolyExtStep enum, interpreter opcode dispatch loop, disabled split path's design, tiny straight-line regression, private-scratch cap. Spot-checked: interpreter WGSL 1329–1596, build_eval_check_wgsl 1907–2140, ENABLE_SPLIT 76, MAX_POLY_EXT_STEPS 75.

No subagent claim contradicted another. The ambiguity around "WebGpuCircuitEvalCheck not invoked" was reconciled against docs telemetry ("eval_check gpu_dispatches=2") which shows the trait IS invoked via the HAL's `dispatch_eval_check_poly_ext` hook on the async path, distinct from the synchronous `CircuitHal::eval_check` fallback.

## Worktree Diff Audit

Baseline type: `local commit`
Baseline reference: `d042da45c89a1cd5f9cf7c5eb962a754367b2118`
Comparison reference: `working-tree`
Normalized baseline: `d042da45c89a1cd5f9cf7c5eb962a754367b2118`
Normalized comparison: `working-tree`
Normalized diff command: `git diff --name-only d042da45c89a1cd5f9cf7c5eb962a754367b2118`

Changed files reported by the diff basis (executed from the worktree at HEAD `454b3109b`):
- `.gitignore` — added `.worktrees/` pattern during Phase 0 (worktree isolation requirement)
- `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md` — Phase 0 artifact (LOCKED)
- `.recursive/run/wasm-webgpu-prover-perf/00-worktree.md` — Phase 0 artifact (LOCKED)

All three changes are Phase 0 deliverables and are filtered out of drift checks by `filter_runtime_changed_files` (run-local paths under `.recursive/run/wasm-webgpu-prover-perf/`). No product source files modified by this run yet. The diff basis is executable from the worktree's HEAD without modification.

## Gaps Found

None. All deltas between current state and R1–R13 obligations are captured under Known Unknowns (open structural questions to be resolved during Phase 2 planning) and Traceability (per-R# Phase 2 anchor file:line). No in-scope Phase 1 gap remains unaddressed by this artifact.

## Repair Work Performed

This phase introduced no product/worktree code changes. Worktree state changes during Phase 1:

1. Fast-forward of `recursive/wasm-webgpu-prover-perf` from `d042da45c` to `454b3109b` to pick up the controller's Phase 0 commit (`recursive run 0`). Non-product change; preserves the Phase 0 diff basis.
2. Recreation of empty run-folder subdirectories (`addenda/`, `subagents/`, `router-prompts/`, `evidence/{screenshots,logs,perf,traces,review-bundles,router,other}/`) that were not carried by the Phase 0 commit (git does not track empty directories).

No source files in `risc0/`, `examples/`, or other product trees were modified. No tests were added or changed. The diff basis remains executable.

## Requirement Completion Status

- R1 | Status: deferred | Rationale: Phase 1 is AS-IS analysis only; native CUDA + Chrome baseline capture is a Phase 2 plan + Phase 3 execution deliverable. The wasm build gate (substrate prerequisite) is already green per Phase 0. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R2 | Status: deferred | Rationale: Phase 1 only inventories the absence of circuit-specific WGSL for rv32im; Phase 2 plans the generator design and Phase 3 implements it. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R3 | Status: deferred | Rationale: Phase 1 inventories the interpreter + chunk-upload status of recursion eval_check and the locked CPU fallback for oversized gather_sample; design lands in Phase 2, implementation in Phase 3. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R4 | Status: deferred | Rationale: Keccak generator depends on R2 and R3 landing first per the spec's attack order; Phase 1 only inventories the 6741-slot interpreter underutilization. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R5 | Status: deferred | Rationale: Phase 1 inventories the Rust/WASM witness/accumulation bridges per circuit; Phase 2 plans the GPU-side replacement and Phase 3 implements it. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R6 | Status: deferred | Rationale: Phase 1 catalogs current readback sites and batching state; Phase 2 plans the per-fixture reduction targets and Phase 3 implements them. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R7 | Status: deferred | Rationale: Pipeline cache exists for eval_check interpreter only; Phase 2 plans the bind-group + cross-stage cache abstraction. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R8 | Status: deferred | Rationale: Single-buffer chunked path exists; multi-buffer tiling is a Phase 2 design + Phase 3 implementation deliverable. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R9 | Status: deferred | Rationale: Deferred parity fixtures are blocked on R2–R8 landing; Phase 2 orders them per fixture class. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R10 | Status: deferred | Rationale: Closing condition requires every lever in R2–R8 to land; Phase 2 plans the closing condition operationally and Phase 3+ measures. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R11 | Status: deferred | Rationale: Forbidden-regression guard flags are at conservative values in `risc0/zkp/src/hal/webgpu.rs:75`–81 today; Phase 1 inventories them, Phase 2 plans per-sub-phase compliance, Phase 3.5/4 verify no regression. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R12 | Status: deferred | Rationale: Diagnostic substrate exists; Phase 2 plans the per-fixture evidence directory layout under `evidence/perf/` and Phase 3+ records measurements. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R13 | Status: deferred | Rationale: Correctness invariants inherited from `docs/requirements/wasm-webgpu-prover.md` are in force in the current branch state per `docs/wasm-webgpu-validation.md`; Phase 4 verification re-checks them after Phase 3 lands. | Deferred By: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- R18 | Status: out-of-scope | Rationale: R18 is the upstream documentation requirement from `docs/requirements/wasm-webgpu-prover.md` and is inherited as a constraint via R13. It is not a separate deliverable of this performance-focused run; documentation hygiene happens naturally in Phase 8 memory updates. | Scope Decision: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`

## Audit Verdict

The AS-IS inventory is grounded in code pointers, not in design intent. Each R# has a Source Requirement Inventory entry with disposition, source quote, and summary. Each R# also has a current-state evidence pointer in `## Current Behavior by Requirement` and a forward-looking anchor in `## Traceability`. Failed-experiment ledger is incorporated as a Phase 2 design constraint via R11. Five Explore subagents corroborated the substrate map; no internal contradictions. No product changes were introduced.

Audit: PASS

## Known Unknowns

- Whether the CUDA kernels are regeneratable in-tree from `PolyExtStepDef::DEF` or must be hand-translated. The `.cu` files are committed but no in-tree generator is visible. R2/R3/R4 generators will need to operate on the in-tree DEF directly. Phase 2 must validate with a small prototype before committing the larger generator design.
- Whether Chrome's 1 GiB negotiated `maxBufferSize` is durable across hardware. Tiled multi-buffer R3/R8 design must be robust to smaller hardware ceilings.
- The exact device-loss threshold for staged WGSL. Failed experiments cite 128 s / 486 s / 148 s as the line where queued GPU work crashed Chrome's WebGPU instance. Phase 2 must default to per-stage `queue.submit()`.
- The exact cost split inside `lift_prove_async` (2.878 s for poseidon2_basic). Single timer today; finer subdivision required before R3 picks tiling parameters.
- Whether Keccak's no-op accumulate stub is correct or a TODO. Phase 2 must reconcile.
- Whether witness/accumulation can be overlapped with WebGPU dispatches before R5 lands as an independent micro-optimization.
- Whether the recursion data group is tile-stable across proofs. If yes, R3 can amortize the upload across many proofs.

## Evidence

### Failed-experiment ledger (from `docs/wasm-webgpu-prover-learnings.md`)

> "Eval-check interpreter pipeline caching was added … Keccak-small proof changed from about 125.398s to about 125.251s." (lines 488–494)

> "Increasing private scratch so Keccak could run a different path made the focused Keccak eval-check pass, but it was much slower: stable path about 40.6s, large private scratch experiment about 179.1s." (lines 712–719)

> "Some [split eval_check experiments] lost the WebGPU instance/device. … avoid huge shaders, avoid hundreds of pipelines per proof stage, avoid deferred giant GPU workloads that only fail at readback, prefer semantic staging with bounded resource usage." (lines 504–513)

### Per-stage telemetry (libm)

```
rv32im_witgen         85ms
rv32im_accumulate    309ms
rv32im_eval_check  32546ms   ← interpreter, 927 fp_slots, 20202 instructions
segment_prove_core 43592ms
recursion_witgen     207ms
recursion_accumulate 207ms
recursion_eval_check 119937ms  ← interpreter, 1000 fp_slots
lift_prove          170293ms
prove_session       213951ms
```

### Per-stage telemetry (poseidon2_basic post async/GPU-authoritative)

```
rv32im_witgen                 86ms
rv32im_accumulate            241ms
rv32im_eval_check_interp       1ms
segment_prove_core_async     969ms
recursion_witgen             203ms
recursion_accumulate         200ms
recursion_eval_check_interp    1ms
lift_prove_async            2878ms   ← dominates, not eval_check
prove_session_async         3913ms
```

### Op counters (poseidon2_basic)

```
gpu_dispatches=306 cpu_mirrors=11 cpu_fallbacks=1 cpu_only_ops=0
uploads=750 upload_bytes=444187604
device_copies=8 device_copy_bytes=212208640
readbacks=62 readback_bytes=431840
buffers=818 buffer_bytes=1738130508
```

R6 targets ≤ 30 readbacks / ≤ 1 MiB. R5 targets `cpu_mirrors` ↓. R3 targets `upload_bytes` ↓.

## Traceability

- R1 → Phase 2 plans `evidence/perf/r1-baselines/` capture commands. Anchors: `risc0/zkvm/src/host/server/prove/prover_impl.rs:56`–83 (timer); reproduction commands in this artifact.
- R2 → Phase 2 plans a circuit-specific staged WGSL generator for rv32im consuming `risc0/circuit/rv32im/src/zirgen/poly_ext.rs:26 DEF`; replaces interpreter dispatch at `risc0/zkp/src/hal/webgpu.rs:5138`–5250 for rv32im inputs.
- R3 → Phase 2 plans tiled GPU-resident buffers for the recursion data group (replacing chunk-upload paths in `risc0/zkp/src/prove/prover.rs` finalize and the locked CPU fallback at `risc0/zkp/src/hal/webgpu.rs:6879`) plus circuit-specific staged WGSL for `risc0/circuit/recursion/src/poly_ext.rs:26 DEF`.
- R4 → Phase 2 plans a keccak generator analogous to R2/R3, scoped after R2 and R3. Anchors: `risc0/circuit/keccak/src/zirgen/poly_ext.rs:26 DEF`; parity helper `risc0/circuit/keccak/src/lib.rs:82`–157.
- R5 → Phase 2 plans GPU-side witness/accumulation per circuit. Anchors: `risc0/circuit/{rv32im,keccak,recursion}/src/prove/hal/webgpu.rs` witness/accumulate entry lines; Keccak no-op accumulate stub at `risc0/circuit/keccak/src/prove/hal/webgpu.rs:136`–146.
- R6 → Phase 2 plans reduction targets at Merkle/FRI/DEEP boundaries. Anchors: `risc0/zkp/src/prove/merkle.rs:272, 296, 352`–422; `risc0/zkp/src/prove/fri.rs:190`; `risc0/zkp/src/prove/prover.rs:625, 667`.
- R7 → Phase 2 plans a bind-group + pipeline cache abstraction across NTT/FRI/Merkle/combos. Anchor: existing `eval_check_interpreter_pipelines` at `risc0/zkp/src/hal/webgpu.rs:854`–855.
- R8 → Phase 2 plans a tiled multi-buffer source representation. Anchors: `risc0/zkp/src/hal/webgpu.rs:6351` size gate, 6879 fallback, 8014 chunked dispatch, 6908 debug helper.
- R9 → Phase 2 orders the deferred fixtures so each becomes runnable after the relevant R2–R8 lever lands. No independent code anchor.
- R10 → Phase 2 defines per-fixture measurement format under `evidence/perf/` and the trend-tracking template. Anchors: `WebGpuStageTimer` at `risc0/zkvm/src/host/server/prove/prover_impl.rs:56`–83; `WebGpuDiagnostics` snapshot at `risc0/zkp/src/hal/webgpu.rs:2449`.
- R11 → Phase 2 declares which forbidden experiments each sub-phase avoids. Anchors: `risc0/zkp/src/hal/webgpu.rs:75`–81 guard flag block; `docs/wasm-webgpu-prover-learnings.md` failed-experiment ledger.
- R12 → Phase 2 defines the per-fixture evidence directory layout. Anchor: `evidence/perf/` subdir already created in Phase 0.
- R13 → Phase 2 includes a "regression check" post-condition. Anchor: `docs/wasm-webgpu-validation.md` passing matrix; `docs/requirements/wasm-webgpu-prover.md` R1–R18 acceptance gate.
- R18 → Inherited via R13. Documentation hygiene is covered by Phase 8 memory and `docs/requirements/wasm-webgpu-prover.md` already encodes the constraint.

## Coverage Gate

- [X] Every R# (R1–R13 + inherited R18) has a Source Requirement Inventory entry with disposition, source quote, and summary.
- [X] All required audited-phase sections are present (Audit Context, Effective Inputs Re-read, Earlier Phase Reconciliation, Prior Recursive Evidence Reviewed, Subagent Contribution Verification, Worktree Diff Audit, Gaps Found, Repair Work Performed, Requirement Completion Status, Audit Verdict).
- [X] Reproduction Steps cover wasm build, native CUDA baseline, Chrome browser baseline, Keccak parity.
- [X] Current Behavior by Requirement maps each R# to substrate evidence.
- [X] Relevant Code Pointers enumerate every file:line needed for Phase 2 planning.
- [X] Known Unknowns are enumerated with planned-resolution notes.
- [X] Evidence section captures failed-experiment ledger, per-stage telemetry, and op counters.
- [X] Traceability maps every R# (R1–R13, R18) to its Phase 2 anchor file:line.
- [X] Worktree Diff Audit includes the executable diff-basis fields.

Coverage: PASS

## Approval Gate

- [X] AS-IS is grounded in current-state code pointers, not design intent.
- [X] Substrate inventory is sufficient for Phase 2 to plan concrete edits per requirement.
- [X] Known Unknowns either have planned resolution in Phase 2 or are explicitly out-of-scope.
- [X] Failed-experiment ledger is incorporated as a constraint on Phase 2 design space.
- [X] No product/worktree code changes introduced by this phase.

Approval: PASS
