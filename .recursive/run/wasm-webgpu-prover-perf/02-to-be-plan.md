Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `02 TO-BE plan`
Status: `LOCKED`
LockedAt: `2026-05-12T05:14:03Z`
LockHash: `25aced2df2cb0420801faa3e0e6b0666aeedf1f5cd09803df25180d77a146a92`
Workflow version: `recursive-mode-audit-v2`
Inputs:
- `/.recursive/run/wasm-webgpu-prover-perf/00-requirements.md` (LOCKED)
- `/.recursive/run/wasm-webgpu-prover-perf/01-as-is.md` (LOCKED, hash `6ae676746bbd68fe6d9b724d493cdcfa968d04235ae8e40294c3324da67a743d`)
- `docs/requirements/wasm-webgpu-prover.md`
- `docs/wasm-webgpu-prover-learnings.md`
- `docs/wasm-webgpu-validation.md`
- `docs/wasm-webgpu-cuda-comparison.md`
- `docs/wasm-webgpu-prover.md`
- `risc0/zkp/src/hal/webgpu.rs`
- `risc0/circuit/{rv32im,keccak,recursion}/src/prove/hal/webgpu.rs`
- `risc0/circuit/{rv32im-sys,keccak-sys,recursion-sys}/kernels/cuda/eval_check*.cu`
- `risc0/circuit/{rv32im,keccak,recursion}/src/{zirgen/,}poly_ext.rs`
- `risc0/zkp/src/adapter.rs`
- `risc0/zkvm/src/host/server/prove/prover_impl.rs`
- `risc0/zkp/src/prove/{merkle.rs,fri.rs,prover.rs,poly_group.rs}`
Outputs:
- `/.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
Scope note: ExecPlan-grade plan that breaks R1–R13 into 11 ordered sub-phases (SP1–SP11) with concrete edits by file path, tests, manual QA scenarios, idempotence guidance, and traceability so Phase 3 can implement incrementally and Phase 3.5/4/5 can audit each sub-phase against its planned change surface.

## TODO

- [x] Reread Phase 0 + Phase 1 artifacts and confirm locks intact.
- [x] Define ordered sub-phases (SP1–SP11) covering R1–R13 + inherited R18.
- [x] Map each R# to one or more sub-phases via Requirement Mapping.
- [x] Enumerate Planned Changes by File scoped to each sub-phase.
- [x] Define Testing Strategy (TDD discipline applies in Phase 3).
- [x] Define Manual QA Scenarios.
- [x] Document Idempotence and Recovery for partial implementations.
- [x] Record Plan Drift Check (no in-scope drift from Phase 1 inventory).
- [x] Complete required audited-phase sections.
- [x] Complete Coverage Gate / Approval Gate.

## Audit Context

Audit Execution Mode: `self-audit`
Subagent Availability: `available`
Subagent Capability Probe: Explore subagents remain available; no new subagent dispatches required for Phase 2 because the substrate inventory from Phase 1 is sufficient for planning. Phase 3.5 code review may delegate via the router when implementation lands.
Delegation Decision Basis: Phase 2 is synthesis-and-sequencing work; the AS-IS phase already paid the substrate-mapping cost. Self-audit is appropriate because the plan composition itself benefits from the controller's full Phase 0 + Phase 1 context.
Delegation Override Reason: None; subagents are available but unnecessary for plan composition.
Audit Inputs Provided:
- Phase 0 artifacts: `00-requirements.md` (LOCKED, `d98a93e75bc9…`), `00-worktree.md` (LOCKED, `71fbfa4cedfc…`)
- Phase 1 artifact: `01-as-is.md` (LOCKED, `6ae676746bbd…`)
- Diff basis: `git diff --name-only d042da45c89a1cd5f9cf7c5eb962a754367b2118` (executable from worktree at HEAD `454b3109b`)
- Targeted code references inherited from `01-as-is.md ## Relevant Code Pointers`

## Effective Inputs Re-read

- `00-requirements.md` (R1–R13 + constraints) → maps to Requirement Mapping coverages below.
- `01-as-is.md` (Source Requirement Inventory + Current Behavior by Requirement + Traceability) → drives sub-phase ordering and per-R# anchors.
- `docs/wasm-webgpu-prover-learnings.md` (Recommended Resume Plan + failed-experiment ledger) → drives SP2–SP6 design constraints.
- `docs/wasm-webgpu-validation.md` (per-fixture native CUDA baselines + deferred fixture matrix) → drives SP10 ordering.
- `docs/wasm-webgpu-cuda-comparison.md` (per-stage telemetry tables) → identifies which timer scopes need subdivision under SP1.
- `docs/requirements/wasm-webgpu-prover.md` (inherited R1–R18 correctness gate) → preserved as constraint in SP11.

## Earlier Phase Reconciliation

Phase 0 locks (`00-requirements.md` and `00-worktree.md`) and Phase 1 lock (`01-as-is.md`) verify against their content hashes. The Phase 0 diff basis (`d042da45c…`) remains executable; the Phase 1 Worktree Diff Audit lists only the three Phase 0 deliverables as changed files, which are filtered by run-local path. Phase 2 introduces no product code changes; the only worktree state added by this phase is `02-to-be-plan.md` itself. The Phase 1 Source Requirement Inventory is preserved verbatim in this phase's Requirement Mapping below.

## Prior Recursive Evidence Reviewed

None applicable. Justification: this is the first run; no prior runs exist under `.recursive/run/`. The memory router `.recursive/memory/MEMORY.md` was re-consulted at Phase 2 entry — no memory shards relevant to the WebGPU prover subsystem exist yet. The reason is that the registry under `domains/`, `patterns/`, `incidents/`, `episodes/`, `skills/`, and `archive/` remains empty for this subsystem as established in Phase 1.

## Planned Changes by File

This section enumerates the planned change surface per sub-phase. Each Phase 3 sub-phase MUST cite its target files from this list.

### Substrate additions (new files)

- `risc0/zkp/src/hal/webgpu/eval_check_codegen/mod.rs` — new module entry. Re-exports per-circuit staged WGSL generators (`rv32im_gen`, `keccak_gen`, `recursion_gen`) and a shared `StagedKernel` description type.
- `risc0/zkp/src/hal/webgpu/eval_check_codegen/staged_kernel.rs` — `StagedKernel { id, dispatch_shape, workgroup_size, bind_groups, wgsl_source }` types + serialization for diagnostic logging.
- `risc0/zkp/src/hal/webgpu/eval_check_codegen/rv32im_gen.rs` — rv32im-specific staged generator. Consumes `risc0_circuit_rv32im::zirgen::poly_ext::DEF` and emits a fixed bounded set of WGSL kernels (≤ 8) with explicit intermediate buffers.
- `risc0/zkp/src/hal/webgpu/eval_check_codegen/recursion_gen.rs` — recursion-specific staged generator. Consumes `risc0_circuit_recursion::poly_ext::DEF`.
- `risc0/zkp/src/hal/webgpu/eval_check_codegen/keccak_gen.rs` — keccak-specific staged generator. Consumes `risc0_circuit_keccak::zirgen::poly_ext::DEF`.
- `risc0/zkp/src/hal/webgpu/buffer_pool.rs` — new module. `BufferPool { buffers: Vec<GPUBuffer>, layout: TileLayout }` for tiled multi-buffer source representations (R3, R8 dependency).
- `risc0/zkp/src/hal/webgpu/pipeline_cache.rs` — new module. Generalizes the existing `eval_check_interpreter_pipelines` cache pattern into a cross-stage `PipelineCache` keyed by `(stage_name, layout_hash, constants_hash)`. Adds a `BindGroupCache` companion keyed by `(pipeline_id, buffer_identities)`.

### Existing-file edits

- `risc0/zkp/src/hal/webgpu.rs`:
  - `dispatch_eval_check_poly_ext` (line ~5855) — add fast-path that, when the requested circuit has a generated WGSL kernel set available, dispatches through the staged generator instead of the interpreter. Interpreter remains as fallback behind an env var (`RISC0_WEBGPU_EVAL_CHECK_FORCE_INTERPRETER=1`).
  - `dispatch_gather_sample_chunked` (line 8014) — add a tiled-source variant `dispatch_gather_sample_tiled` operating over `BufferPool` from `buffer_pool.rs`.
  - `gather_sample_async` (line 6844) — replace the locked CPU-fallback path (line 6879) with a dispatch to `dispatch_gather_sample_tiled` when the source is `BufferPool`-backed.
  - Diagnostics counters (line 2430–2446) — add `pipeline_creations`, `bind_group_creations`, `pipeline_cache_hits`, `bind_group_cache_hits`, `tile_count`, `tile_bytes_max` so R7 + R8 telemetry can assert thresholds.
- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`:
  - `impl WebGpuCircuitEvalCheck` (50–72) — route through the new rv32im generator via the HAL fast-path (no signature change; the implementation passes `CircuitKind::Rv32im` so the HAL can pick the right kernel set).
  - `CircuitWitnessGenerator::generate_witness` (74–93) — Phase 3 sub-phase SP7 replaces the `super::rust_steps::generate_witness()` call with a WebGPU dispatch path; until SP7 lands, this remains as-is.
  - `CircuitAccumulator::step_accum` (95–112) — Phase 3 sub-phase SP7 replaces with a WebGPU dispatch path.
- `risc0/circuit/keccak/src/prove/hal/webgpu.rs`:
  - `impl WebGpuCircuitEvalCheck` (57–79) — route through keccak generator.
  - `CircuitWitnessGenerator::generate_witness` (81–133) — SP7 replaces with WebGPU.
  - Accumulate no-op stub (136–146) — SP7 either confirms intentional no-op (move comment to a `// Keccak accumulate is empty by construction` annotation) or implements GPU-side accumulate.
- `risc0/circuit/recursion/src/prove/hal/webgpu.rs`:
  - `impl WebGpuCircuitEvalCheck` (44–66) — route through recursion generator.
  - `CircuitWitnessGenerator::generate_witness` (68–97) — SP7 replaces with WebGPU.
  - `CircuitAccumulator::accumulate` (99–116) — SP7 replaces with WebGPU.
- `risc0/zkp/src/prove/prover.rs`:
  - `finalize_async` (694–773) — wire `combos_prepare_async` / `combos_divide_async` to consume tiled buffer-pool inputs.
  - `batch_evaluate_any_async` (625, 667) — defer per-PolyGroup readback until both DEEP groups complete (R6 reduction).
- `risc0/zkp/src/prove/merkle.rs`:
  - `prove_batch_async` (352–422) — share readback buffer allocation across trees within one segment to reduce `buffer_bytes` (R6, R7 dependency).
- `risc0/zkvm/src/host/server/prove/prover_impl.rs`:
  - `WebGpuStageTimer` (56–83) — add finer-grained timer scopes inside `lift_prove_async`: `recursion_data_group_upload`, `recursion_commit`, `recursion_finalize`, `recursion_merkle` (resolves Known Unknown from Phase 1).
- `risc0/circuit/recursion/src/prove/hal/webgpu.rs` and `risc0/circuit/recursion/src/prove/mod.rs`:
  - Replace per-proof chunk upload of the recursion data group with `BufferPool`-backed tiles. New helper `upload_recursion_data_pool` that constructs a `BufferPool` once and reuses across stages within a proof.

### Test additions (new files)

- `risc0/zkp/src/hal/webgpu/eval_check_codegen/tests/rv32im_parity.rs` — CPU-vs-WebGPU parity for rv32im at `po2 = 0..=3` (tiny), `po2 = 16` (segment-sized), and `po2 = 21` (production-shaped). Uses `risc0_zkp::hal::portable::eval_check` as ground truth.
- `risc0/zkp/src/hal/webgpu/eval_check_codegen/tests/recursion_parity.rs` — same pattern for recursion.
- `risc0/zkp/src/hal/webgpu/eval_check_codegen/tests/keccak_parity.rs` — same pattern for keccak; extends existing `eval_check_webgpu_matches_portable` helper.
- `risc0/zkp/src/hal/webgpu/buffer_pool/tests.rs` — `BufferPool` allocation, tile addressing, and round-trip dispatch tests.
- `risc0/zkp/src/hal/webgpu/pipeline_cache/tests.rs` — pipeline + bind-group cache hit/miss invariants, lifetime safety, and layout-key correctness.

### Documentation additions / edits

- `docs/wasm-webgpu-validation.md` — refresh per-fixture wall-time and ratio columns at the end of each sub-phase that affects a fixture's measurement.
- `docs/wasm-webgpu-cuda-comparison.md` — refresh per-stage telemetry for the smoke fixtures + libm + KeccakUnion(1) at sub-phase boundaries.
- `docs/wasm-webgpu-prover.md` — update the "Performance Debt" section as bottlenecks move.
- `docs/wasm-webgpu-prover-learnings.md` — append per-sub-phase lessons (e.g., generator-specific Chrome device-loss thresholds discovered) as the run progresses.

## Requirement Mapping

Each R# is mapped to one or more sub-phases. Source Quotes are preserved verbatim from `01-as-is.md ## Source Requirement Inventory` (per lint rule).

- R1 | Coverage: direct | Source Quote: "Rebuild the wasm harness" | Implementation Surface: `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r1-baselines/` (new fixture-baseline subdirectory); commands captured via `risc0/zkvm/src/host/server/prove/prover_impl.rs` timers | Verification Surface: SP1's `06-decisions-update.md` summary references and `04-test-summary.md` smoke results | QA Surface: SP1 manual scenario "rerun the six smoke fixtures and confirm `cpu_only_ops=0` plus matching cycles"
- R2 | Coverage: direct | Source Quote: "Circuit-specific staged WGSL `eval_check` for rv32im" | Implementation Surface: `risc0/zkp/src/hal/webgpu/eval_check_codegen/rv32im_gen.rs`, `risc0/zkp/src/hal/webgpu.rs:5855` (fast-path), `risc0/circuit/rv32im/src/prove/hal/webgpu.rs:50`–72 (trait routing) | Verification Surface: `risc0/zkp/src/hal/webgpu/eval_check_codegen/tests/rv32im_parity.rs`, Chrome browser proof for `poseidon2_basic`, `hello-world`, `json` | QA Surface: manual rerun of rv32im acceptance fixtures + diff diagnostics (`rv32im_eval_check gpu_dispatches`, `cpu_fallbacks`)
- R3 | Coverage: direct | Source Quote: "Circuit-specific staged WGSL `eval_check` for recursion + tiled GPU-resident data group" | Implementation Surface: `risc0/zkp/src/hal/webgpu/eval_check_codegen/recursion_gen.rs`, `risc0/zkp/src/hal/webgpu/buffer_pool.rs`, `risc0/circuit/recursion/src/prove/hal/webgpu.rs:44`–66, `risc0/zkp/src/hal/webgpu.rs:6844` (gather_sample tiled), `risc0/zkp/src/prove/prover.rs:694`–773 (finalize pool consumption) | Verification Surface: `risc0/zkp/src/hal/webgpu/eval_check_codegen/tests/recursion_parity.rs`, recursion-sized `BufferPool` regression at `rows = 2^20, cols = 128` | QA Surface: Chrome proof of `multi_test/libm`; `lift_prove_async` wall-time on `poseidon2_basic`
- R4 | Coverage: direct | Source Quote: "Circuit-specific staged WGSL `eval_check` for keccak" | Implementation Surface: `risc0/zkp/src/hal/webgpu/eval_check_codegen/keccak_gen.rs`, `risc0/circuit/keccak/src/prove/hal/webgpu.rs:57`–79 | Verification Surface: `risc0/zkp/src/hal/webgpu/eval_check_codegen/tests/keccak_parity.rs` extends `risc0/circuit/keccak/src/lib.rs:82`–157 | QA Surface: Chrome proof of `multi_test/keccak_union_small` (assert 0 eval_check fallbacks) and `multi_test/keccak_union` (assert KeccakUnion(3) completes within `WASM_BINDGEN_TEST_TIMEOUT=7200`)
- R5 | Coverage: direct | Source Quote: "GPU-resident witness and accumulation for proof circuits" | Implementation Surface: `risc0/circuit/recursion/src/prove/hal/webgpu.rs:68`–116 (witness + accumulate), `risc0/circuit/keccak/src/prove/hal/webgpu.rs:81`–146 (witness + accumulate stub reconciliation), `risc0/circuit/rv32im/src/prove/hal/webgpu.rs:74`–112 (witness + accumulate), new WGSL kernels under `risc0/zkp/src/hal/webgpu.rs` | Verification Surface: new CPU-vs-WebGPU CircuitHal parity tests under each circuit crate | QA Surface: poseidon2_basic stage timers showing `recursion_witgen + recursion_accumulate ≤ 100 ms`
- R6 | Coverage: direct | Source Quote: "Bounded transcript and Merkle readbacks" | Implementation Surface: `risc0/zkp/src/prove/prover.rs:625`, 667 (DEEP coalescing), `risc0/zkp/src/prove/merkle.rs:352`–422 (cross-tree readback batching), new diagnostic assertions in `risc0/zkp/src/hal/webgpu.rs:2430`–2446 | Verification Surface: existing browser parity harness reports `readbacks` and `readback_bytes` per proof; new assertion test in `examples/browser-prove/src/lib.rs` for `poseidon2_basic ≤ 30 readbacks / ≤ 1 MiB` | QA Surface: poseidon2_basic + keccak_union_small browser proof diagnostics
- R7 | Coverage: direct | Source Quote: "Pipeline and buffer reuse without correctness loss" | Implementation Surface: `risc0/zkp/src/hal/webgpu/pipeline_cache.rs` (new), `risc0/zkp/src/hal/webgpu.rs` (NTT/FRI/Merkle/combos pipeline-creation paths), diagnostic counters at lines 2430–2446 | Verification Surface: `risc0/zkp/src/hal/webgpu/pipeline_cache/tests.rs`; new browser harness assertion `pipeline_creations` drops ≥ 50% on smoke suite | QA Surface: rerun smoke suite and inspect HAL diagnostic deltas
- R8 | Coverage: direct | Source Quote: "Production `gather_sample` chunking with tiled multi-buffer source" | Implementation Surface: `risc0/zkp/src/hal/webgpu/buffer_pool.rs` (new), `risc0/zkp/src/hal/webgpu.rs:6844`–6879 (tiled gather fallback replacement), `risc0/zkp/src/hal/webgpu.rs:8014` (tiled-source dispatch variant) | Verification Surface: `risc0/zkp/src/hal/webgpu/buffer_pool/tests.rs` recursion-sized regression at `rows = 2^20, cols = 128`; replaces the existing `webgpu_hal_recursion_sized_gather_sample_falls_back_to_cpu` test | QA Surface: rerun `multi_test/libm` browser proof; assert `gather_sample cpu_fallbacks = 0`
- R9 | Coverage: indirect | Source Quote: "Restore deferred parity matrix" | Implementation Surface: emergent from SP2–SP8; no R9-specific code surface. The seven deferred fixtures (`rsa_compat`, `KeccakUnion(3)`, `groth16-verifier`, `xgboost`, `bn254`, BLST, `verify`) become runnable as their gating R# implementations land. | Verification Surface: per-fixture browser proof entries in `docs/wasm-webgpu-validation.md` matrix | QA Surface: SP10 manual scenarios | Rationale: R9 is a per-fixture acceptance batch; the actual implementation surface is the SP2–SP8 levers that unblock each fixture. SP10 sequences and records the per-fixture browser runs.
- R10 | Coverage: indirect | Source Quote: "Wall-time parity with native CUDA (1.0× ideal target)" | Implementation Surface: emergent from SP2–SP8; no R10-specific code surface. R10 measurement and closing-condition assessment live in SP11 + `docs/wasm-webgpu-cuda-comparison.md` ratio refreshes. | Verification Surface: `docs/wasm-webgpu-validation.md` ratio column refreshed at each sub-phase boundary | QA Surface: SP11 final ratio review | Rationale: R10 is a measurement aggregate over R2–R8 levers. The "closing condition" check belongs in SP11 after SP2–SP8 have landed; no separate code surface owns R10.
- R11 | Coverage: direct | Source Quote: "Failure-mode hygiene and forbidden regressions" | Implementation Surface: `risc0/zkp/src/hal/webgpu.rs:75`–81 (guard flags stay at current values; any change requires evidence + Phase 3.5 review) | Verification Surface: `04-test-summary.md` Phase 4 explicitly checks that the four guard flags retain their AS-IS values unless a sub-phase added evidence; new lint assertion in `risc0/zkp/src/hal/webgpu/tests.rs` | QA Surface: Phase 3.5 code review explicitly inspects the guard-flag block on each implementation diff
- R12 | Coverage: direct | Source Quote: "Performance telemetry and CUDA-side baselines" | Implementation Surface: `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/` subdirectories per fixture; `risc0/zkvm/src/host/server/prove/prover_impl.rs:56`–83 timer additions under `lift_prove_async` | Verification Surface: Phase 4 `04-test-summary.md` references the per-fixture evidence files | QA Surface: SP1 captures the initial six-fixture baseline; subsequent sub-phases refresh affected fixtures
- R13 | Coverage: direct | Source Quote: "Correctness invariants preserved end-to-end" | Implementation Surface: no new code; constraint enforces no changes to `risc0/zkp/src/verify/` or receipt format/control ID/seal encoding files | Verification Surface: full browser harness pass over `docs/wasm-webgpu-validation.md` matrix in Phase 4 | QA Surface: SP11 manual scenario "rerun the 21 currently-passing examples and confirm all still pass"
- R18 | Coverage: out-of-scope | Source Quote: "(R1–R18) remains binding" | Rationale: R14–R18 of the upstream correctness requirements are inherited as a constraint via R13. They are not new deliverables of this performance-focused run; documentation hygiene happens naturally in Phase 8 memory updates.

## Implementation Steps

The sub-phase ordering follows the attack order encoded in `00-requirements.md ## Approval Gate` (R2 → R3 → R5 → R8 → R4 → R6/R7 → R9) plus front-loaded baseline capture (SP1) and a final closing-condition audit (SP11).

1. **SP1 — Baseline capture** (R1, R12): Run native CUDA + Chrome WebGPU pairs for the six smoke fixtures. Record under `evidence/perf/r1-baselines/<fixture>.md`. Refresh `docs/wasm-webgpu-validation.md` and `docs/wasm-webgpu-cuda-comparison.md` ratio columns.
2. **SP2 — Codegen skeleton + tiny rv32im prototype** (R2 prerequisite, R11 constraint): Add `eval_check_codegen/` module, `StagedKernel` type, `rv32im_gen` generator emitting WGSL for `po2 = 0..=3` only. Add `rv32im_parity.rs` tiny test. Do NOT wire into prove path yet.
3. **SP3 — rv32im production staged WGSL** (R2): Extend `rv32im_gen` to handle production `po2 = 16` (segment-sized) and `po2 = 21` (production-shaped). Wire into `dispatch_eval_check_poly_ext` fast-path. Validate parity at tiny/medium/production domains. Add browser harness assertion.
4. **SP4 — Buffer pool + tiled gather_sample** (R8, R3 prerequisite): Add `buffer_pool.rs`, `dispatch_gather_sample_tiled`. Replace locked CPU fallback at `risc0/zkp/src/hal/webgpu.rs:6879` with tiled path. Add recursion-sized regression at `rows = 2^20, cols = 128`. Retire the existing CPU-fallback regression.
5. **SP5 — Recursion staged WGSL + tiled data group** (R3): Extend `recursion_gen` to handle production recursion size. Replace per-proof chunk upload with `BufferPool`-backed tiles. Add `recursion_parity.rs` tests. Wire into prove path. Validate `lift_prove_async ≤ 1.0 s` on poseidon2_basic and `recursion_eval_check < 10 s` on libm.
6. **SP6 — Keccak staged WGSL** (R4, blocked on SP3 + SP5): Extend `keccak_gen` for the 6741-slot circuit, bounded to ≤ 12 stages. Wire into prove path. Add `keccak_parity.rs` tests. Validate KeccakUnion(1) reports 0 eval_check fallbacks and KeccakUnion(3) completes within the runner budget.
7. **SP7 — GPU-resident witness and accumulation** (R5, blocked on SP6): Move recursion, then keccak, then rv32im witness + accumulate onto WebGPU. Reconcile keccak's no-op accumulate stub. Validate per-fixture telemetry targets.
8. **SP8 — Bounded readbacks** (R6, blocked on SP7): Coalesce DEEP readbacks across PolyGroup calls; share cross-tree Merkle readback buffer allocation. Add browser harness assertion `poseidon2_basic ≤ 30 readbacks / ≤ 1 MiB`.
9. **SP9 — Pipeline + bind-group cache** (R7, blocked on SP8): Add `PipelineCache` and `BindGroupCache` covering NTT/FRI/Merkle/combos stages. Add `pipeline_creations` diagnostic + smoke suite assertion (≥ 50% reduction). Validate no correctness regression.
10. **SP10 — Deferred matrix bring-up** (R9, blocked on SP9): Bring up the seven deferred fixtures in order: `multi_test/rsa_compat`, `multi_test/keccak_union` (KeccakUnion(3)), then large fixtures (`groth16-verifier`, `xgboost`, `bn254`, BLST, `verify`). Capture native CUDA + browser pairs under `evidence/perf/r9-deferred/<fixture>.md`. Update validation matrix.
11. **SP11 — Closing-condition audit** (R10, R11, R13, R18): Run the full smoke + acceptance matrix end-to-end. Confirm guard flags at AS-IS values (R11). Confirm 21 public examples + internal parity matrix still pass (R13). Refresh `docs/wasm-webgpu-cuda-comparison.md` with final per-fixture ratios. Identify any residual ratio > 1.0× with a documented structural cause.

Each sub-phase MUST pass these gates before proceeding:
- All CPU-vs-WebGPU parity tests for the affected circuit pass.
- The relevant focused browser fixture (poseidon2_basic for rv32im/recursion; keccak_union_small for keccak) verifies a succinct receipt and matches native cycle counts.
- `cpu_only_ops` stays at 0 on the smoke suite.
- Diagnostic counters fall within R6/R7/R8 thresholds where applicable.
- Guard flags (R11) unchanged unless paired with explicit evidence.

## Testing Strategy

Phase 3 follows TDD with `TDD Mode: strict` for all generator/codegen and HAL-cache work. The discipline is RED → GREEN → REFACTOR:

- **RED**: Add the CPU-parity unit test before any generator code. The test fails because the WGSL kernel does not exist or returns zeros.
- **GREEN**: Implement the generator emitting WGSL. Test passes.
- **REFACTOR**: Once parity holds, optimize the generator (slot reuse, kernel staging, bind-group reuse) while keeping the test green.

Test layers (top-to-bottom):

1. **HAL op equivalence** (`risc0/zkp/src/hal/webgpu/eval_check_codegen/tests/*_parity.rs`): CPU-vs-WebGPU eval_check output at three domain shapes per circuit (tiny / medium / production).
2. **Circuit HAL equivalence** (extends each circuit's existing test module): CPU-vs-WebGPU witness/accumulation buffers for representative preflight traces.
3. **Focused proof receipts** (`examples/browser-prove/src/lib.rs`): per-circuit smoke fixture (`poseidon2_basic`, `multi_test/keccak_union_small`) — assert receipt verifies, cycles match, diagnostic counters within R6 thresholds.
4. **Composite-to-succinct compression** (existing harness `prove_with_opts_async` + `compress_async`): unchanged surface; coverage maintained.
5. **Full example matrix** (existing harness): the 21 currently-passing public examples must continue to pass at SP11.
6. **Native CUDA parity baselines**: per fixture; recorded under `evidence/perf/`.

Manual QA scenarios are listed in the next section and are required at sub-phase boundaries that affect user-facing behavior.

Pragmatic exceptions to strict TDD (must be documented in `03-implementation-summary.md`'s TDD Compliance Log):
- SP1 (baseline capture) is process work, not new product code. No RED/GREEN cycle applies; the "test" is reproducible commands captured under `evidence/perf/`.
- SP11 (closing-condition audit) runs existing tests; no new RED/GREEN cycle.
- Diagnostic counter additions in `risc0/zkp/src/hal/webgpu.rs:2430`–2446 may be added concurrently with the consuming test if the counter is purely additive and does not change correctness.

## Playwright Plan (if applicable)

Not applicable. Browser test orchestration is via `wasm-bindgen-test-runner` + ChromeDriver (existing path documented in `01-as-is.md ## Reproduction Steps`). Playwright is not part of the test stack.

## Manual QA Scenarios

Per-sub-phase manual scenarios (executed by the contributor at the end of each sub-phase):

- **SP1 QA**: Run `native_poseidon2_basic_prove_stats` on RTX 5090 + the same fixture's Chrome WebGPU run; visually confirm cycles match, `cpu_only_ops=0`, and the recorded values match the AS-IS baselines (4.03 s / 426 ms for poseidon2_basic).
- **SP2 QA**: Run `cargo test --target wasm32-unknown-unknown --release --no-run` and `cargo test rv32im_parity_tiny`; confirm builds without compile errors and the tiny po2 parity test passes.
- **SP3 QA**: Chrome browser run of `native_poseidon2_basic_async_succinct_receipt_verify`; assert `rv32im_eval_check gpu_dispatches ≥ 1, cpu_fallbacks = 0`; assert succinct receipt verifies; cycles match native (1 seg, 3598 user, 32768 total). Compare wall time to the SP1 baseline; expect improvement.
- **SP4 QA**: Chrome browser run of the new recursion-sized BufferPool regression at `rows = 2^20, cols = 128`; assert non-zero CPU-parity-correct output; assert `gather_sample cpu_fallbacks = 0`.
- **SP5 QA**: Chrome browser run of `native_libm_async_succinct_receipt_verify`; assert `lift_prove_async ≤ 1.0 s` for poseidon2_basic; `recursion_eval_check < 10 s` for libm; receipt verifies.
- **SP6 QA**: Chrome browser run of `native_keccak_union_small_succinct_receipt_verify` and `native_keccak_union_succinct_receipt_verify`; assert KeccakUnion(1) reports `eval_check cpu_fallbacks = 0`; assert KeccakUnion(3) completes within `WASM_BINDGEN_TEST_TIMEOUT=7200`; both receipts verify.
- **SP7 QA**: Chrome browser run of the smoke suite; assert `recursion_witgen + recursion_accumulate ≤ 100 ms` for poseidon2_basic; assert no `cpu_only_ops > 0` regression.
- **SP8 QA**: Chrome browser run of poseidon2_basic; assert `readbacks ≤ 30` and `readback_bytes ≤ 1048576`.
- **SP9 QA**: Run smoke suite twice; capture `pipeline_creations` from each; assert ≥ 50% reduction from the SP1 baseline.
- **SP10 QA**: Per-fixture: run the deferred fixture; assert succinct receipt; assert cycles match native; assert ratio recorded.
- **SP11 QA**: Full example matrix run (21 fixtures + internal parity matrix); assert all pass. Visually inspect refreshed `docs/wasm-webgpu-cuda-comparison.md` and `docs/wasm-webgpu-validation.md` for sanity.

## Idempotence and Recovery

Each sub-phase is structured to be idempotent and partially recoverable:

- **Worktree branch `recursive/wasm-webgpu-prover-perf`**: all commits live here; the controller `wasm` branch is unaffected. A failed sub-phase can be reset via `git reset --hard <pre-sub-phase-sha>` without affecting Phase 0/1 locks.
- **Generator skeleton (SP2)**: behind an env var (`RISC0_WEBGPU_EVAL_CHECK_FORCE_INTERPRETER=1`) so the new code path can be disabled at runtime if a Chrome regression appears. Default behavior remains the interpreter until SP3 flips the fast-path default.
- **BufferPool (SP4)**: introduced as a new type alongside the existing `WebGpuBuffer`. Old single-buffer code paths remain functional; the tiled-source variants are opt-in until SP5 wires them into the recursion finalize path.
- **Pipeline cache (SP9)**: introduced as a new field on `WebGpuHal` with an opt-out env var (`RISC0_WEBGPU_DISABLE_PIPELINE_CACHE=1`) so a correctness regression can be diagnosed by disabling reuse.
- **R11 guard flags**: any flip requires explicit evidence in the sub-phase artifact + Phase 3.5 review. If a sub-phase lands a flag flip without that evidence, the lock fails and the sub-phase is reverted.
- **Diagnostic counters**: purely additive; absence of a counter results in `0` in JSON output, not a crash. Old browser-prove harness assertions on absent counters short-circuit on `Option::unwrap_or(0)`.
- **Documentation refresh**: deferred to the end of each sub-phase; if a sub-phase's measurement work fails mid-way, the docs remain at the previous sub-phase's state and can be re-derived from the per-fixture evidence files.

If Chrome devices a WebGPU device loss during a sub-phase (mirroring the historical 128 s / 486 s / 148 s ledger entries), the sub-phase MUST: (a) capture the failing pipeline shape under `evidence/logs/<sub-phase>-deviceloss-<timestamp>.txt`, (b) reduce per-stage queued work below the failure point, (c) re-run, (d) only proceed once the run is stable across ≥ 3 consecutive attempts.

## Implementation Sub-phases

### SP1 — Baseline capture (R1, R12)

Scope and purpose: At the end of SP1, the run has six fixture baselines recorded as canonical evidence (native CUDA + Chrome WebGPU pairs) plus refreshed validation/comparison docs. This is the regression line for all subsequent sub-phases.

Implementation checklist:
- [ ] Create `evidence/perf/r1-baselines/` (subdir).
- [ ] For each of `risc0-zkvm-methods/cfg`, `hello-world`, `json`, `multi_test/poseidon2_basic`, `multi_test/libm`, `multi_test/keccak_union_small`: run native CUDA baseline command from `01-as-is.md ## Reproduction Steps`; save stdout + telemetry to `evidence/perf/r1-baselines/<fixture>.native.txt`.
- [ ] For each fixture: run the matching Chrome WebGPU async test; save stdout + diagnostic counters to `evidence/perf/r1-baselines/<fixture>.chrome.txt`.
- [ ] Append a per-fixture summary table to `evidence/perf/r1-baselines/summary.md`.
- [ ] Refresh `docs/wasm-webgpu-validation.md` ratio columns.
- [ ] Confirm `cpu_only_ops = 0` on every Chrome run.

Tests for this sub-phase:
- (no new product code; the "tests" are the rerunnable commands and their captured outputs)

### SP2 — Codegen skeleton + tiny rv32im prototype (R2, R11)

Scope and purpose: At end of SP2, the run has a new `eval_check_codegen` module with rv32im-specific generator emitting WGSL for tiny po2 (0..=3), validated against portable, but NOT yet wired into the prove path.

Implementation checklist:
- [ ] Add `risc0/zkp/src/hal/webgpu/eval_check_codegen/mod.rs`, `staged_kernel.rs`, `rv32im_gen.rs`.
- [ ] Add `risc0/zkp/src/hal/webgpu/eval_check_codegen/tests/rv32im_parity.rs` with three tiny po2 cases.
- [ ] Confirm new tests run RED first (before generator emits real WGSL).
- [ ] Implement `rv32im_gen` minimally to GREEN.
- [ ] Confirm `WEBGPU_EVAL_CHECK_ENABLE_SPLIT` and other R11 flags remain at AS-IS values.
- [ ] Do NOT modify `dispatch_eval_check_poly_ext` fast-path yet.

Tests for this sub-phase:
- `cargo test --target wasm32-unknown-unknown --release rv32im_parity_tiny` (3 tests, po2 = 0/1/2/3).

### SP3 — rv32im production staged WGSL (R2)

Scope and purpose: rv32im eval_check on Chrome now runs through the staged WGSL generator for production-shape circuits. Browser `rv32im_eval_check_codegen` dispatches with `cpu_fallbacks = 0` on the smoke fixtures.

Implementation checklist:
- [ ] Extend `rv32im_gen` to handle segment-sized po2 (16) and production-shaped po2 (21).
- [ ] Add segment-sized + production-shaped parity tests.
- [ ] Wire into `risc0/zkp/src/hal/webgpu.rs:dispatch_eval_check_poly_ext` fast-path.
- [ ] Route `risc0/circuit/rv32im/src/prove/hal/webgpu.rs:impl WebGpuCircuitEvalCheck` through the generator.
- [ ] Add browser harness assertion: `rv32im_eval_check gpu_dispatches ≥ 1, cpu_fallbacks = 0` for poseidon2_basic.
- [ ] Refresh poseidon2_basic + hello-world + json measurements under `evidence/perf/r2-rv32im/`.

Tests for this sub-phase:
- `cargo test --target wasm32-unknown-unknown --release rv32im_parity` (full suite).
- `cargo test --target wasm32-unknown-unknown --release native_poseidon2_basic_async_succinct_receipt_verify`.

### SP4 — Buffer pool + tiled gather_sample (R8)

Scope and purpose: `WebGpuHal` can represent oversized sources as a tiled `BufferPool` and `gather_sample` operates over the pool without CPU fallback at recursion sizes.

Implementation checklist:
- [ ] Add `risc0/zkp/src/hal/webgpu/buffer_pool.rs` with `BufferPool` + `TileLayout`.
- [ ] Add `dispatch_gather_sample_tiled` in `risc0/zkp/src/hal/webgpu.rs`.
- [ ] Replace locked CPU fallback at `risc0/zkp/src/hal/webgpu.rs:6879` with `dispatch_gather_sample_tiled` for `BufferPool`-backed sources.
- [ ] Add recursion-sized regression `webgpu_hal_recursion_sized_gather_sample_uses_buffer_pool` in `risc0/zkp/src/hal/webgpu/buffer_pool/tests.rs`.
- [ ] Retire the existing `webgpu_hal_recursion_sized_gather_sample_falls_back_to_cpu` regression.
- [ ] Refresh measurements under `evidence/perf/r8-gather-tile/`.

Tests for this sub-phase:
- `cargo test buffer_pool` (unit tests).
- `cargo test --target wasm32-unknown-unknown --release webgpu_hal_recursion_sized_gather_sample_uses_buffer_pool`.

### SP5 — Recursion staged WGSL + tiled data group (R3)

Scope and purpose: Recursion eval_check on Chrome runs through staged WGSL with the data group held as a `BufferPool`. `lift_prove_async ≤ 1.0 s` for poseidon2_basic; `recursion_eval_check < 10 s` for libm.

Implementation checklist:
- [ ] Add `risc0/zkp/src/hal/webgpu/eval_check_codegen/recursion_gen.rs`.
- [ ] Add `recursion_parity` tests at tiny/medium/production domains.
- [ ] Replace per-proof chunk upload of the recursion data group with `BufferPool`-backed tiles (`upload_recursion_data_pool` helper in `risc0/circuit/recursion/src/prove/mod.rs`).
- [ ] Route `risc0/circuit/recursion/src/prove/hal/webgpu.rs:impl WebGpuCircuitEvalCheck` through the generator.
- [ ] Add finer-grained `WebGpuStageTimer` scopes in `risc0/zkvm/src/host/server/prove/prover_impl.rs:56`–83: `recursion_data_group_upload`, `recursion_commit`, `recursion_finalize`, `recursion_merkle`.
- [ ] Refresh poseidon2_basic + libm measurements under `evidence/perf/r3-recursion/`.

Tests for this sub-phase:
- `cargo test recursion_parity` (full suite).
- Chrome harness: `native_poseidon2_basic_async_succinct_receipt_verify` (assert lift_prove_async ≤ 1.0 s).
- Chrome harness: `native_libm_async_succinct_receipt_verify` (assert recursion_eval_check < 10 s).

### SP6 — Keccak staged WGSL (R4)

Scope and purpose: Keccak eval_check on Chrome runs through staged WGSL bounded to ≤ 12 kernels. KeccakUnion(1) reports 0 eval_check fallbacks; KeccakUnion(3) completes within the runner budget.

Implementation checklist:
- [ ] Add `risc0/zkp/src/hal/webgpu/eval_check_codegen/keccak_gen.rs`.
- [ ] Add `keccak_parity` tests at tiny/medium/production domains (extending `risc0/circuit/keccak/src/lib.rs:82`–157).
- [ ] Route `risc0/circuit/keccak/src/prove/hal/webgpu.rs:impl WebGpuCircuitEvalCheck` through the generator.
- [ ] Add browser harness assertion: KeccakUnion(1) `eval_check cpu_fallbacks = 0`.
- [ ] Add browser harness assertion: KeccakUnion(3) completes within `WASM_BINDGEN_TEST_TIMEOUT=7200`.
- [ ] Refresh KeccakUnion(1) + KeccakUnion(3) measurements under `evidence/perf/r4-keccak/`.

Tests for this sub-phase:
- `cargo test keccak_parity` (full suite).
- Chrome harness: `native_keccak_union_small_succinct_receipt_verify`.
- Chrome harness: `native_keccak_union_succinct_receipt_verify`.

### SP7 — GPU-resident witness + accumulate (R5)

Scope and purpose: Recursion (then keccak, then rv32im) witness gen + accumulate run on WebGPU. Per-fixture stage targets met.

Implementation checklist:
- [ ] For recursion: implement WebGPU witness + accumulate kernels; replace `super::rust_kernels::{generate_witness, accumulate}` calls in `risc0/circuit/recursion/src/prove/hal/webgpu.rs:68`–116 with WebGPU dispatches.
- [ ] For keccak: implement WebGPU witness; reconcile the no-op accumulate stub at `risc0/circuit/keccak/src/prove/hal/webgpu.rs:136`–146 (either confirm intentional or implement).
- [ ] For rv32im: implement WebGPU witness + accumulate; replace `super::rust_steps::{generate_witness, step_accum}` calls in `risc0/circuit/rv32im/src/prove/hal/webgpu.rs:74`–112.
- [ ] Add per-circuit CPU-vs-WebGPU CircuitHal parity tests.
- [ ] Assert per-fixture telemetry: `recursion_witgen + recursion_accumulate ≤ 100 ms`; `rv32im_witgen + rv32im_accumulate ≤ 100 ms`; keccak `cpu_mirrors` drops ≥ 50%.
- [ ] Refresh measurements under `evidence/perf/r5-witness-accumulate/`.

Tests for this sub-phase:
- `cargo test recursion_circuit_hal_parity`, `keccak_circuit_hal_parity`, `rv32im_circuit_hal_parity`.
- Chrome harness: poseidon2_basic with stage timer assertions.

### SP8 — Bounded readbacks (R6)

Scope and purpose: poseidon2_basic browser proof reaches ≤ 30 readbacks / ≤ 1 MiB readback bytes; keccak_union_small drops ≥ 50% from current baseline.

Implementation checklist:
- [ ] Coalesce DEEP `batch_evaluate_any_async` readbacks across the two PolyGroup calls in `risc0/zkp/src/prove/prover.rs:625`, 667.
- [ ] Share cross-tree Merkle readback buffer allocation in `risc0/zkp/src/prove/merkle.rs:352`–422.
- [ ] Add browser harness assertions for `readbacks ≤ 30` and `readback_bytes ≤ 1048576` on poseidon2_basic.
- [ ] Add browser harness assertion for ≥ 50% `readback_bytes` reduction on keccak_union_small.
- [ ] Refresh measurements under `evidence/perf/r6-readbacks/`.

Tests for this sub-phase:
- Chrome harness: poseidon2_basic with readback assertions.
- Chrome harness: keccak_union_small with readback assertions.

### SP9 — Pipeline + bind-group cache (R7)

Scope and purpose: ≥ 50% pipeline-creation reduction on smoke suite; bind-group reuse across NTT/FRI/Merkle/combos stages.

Implementation checklist:
- [ ] Add `risc0/zkp/src/hal/webgpu/pipeline_cache.rs` with `PipelineCache` and `BindGroupCache`.
- [ ] Generalize the existing `eval_check_interpreter_pipelines` cache into the new abstraction.
- [ ] Route NTT, FRI, Merkle hashing, combos_prepare/divide through the cache.
- [ ] Add diagnostic counters: `pipeline_creations`, `bind_group_creations`, `pipeline_cache_hits`, `bind_group_cache_hits`.
- [ ] Add `risc0/zkp/src/hal/webgpu/pipeline_cache/tests.rs`.
- [ ] Add browser harness assertion: ≥ 50% reduction in `pipeline_creations` on smoke suite vs SP1 baseline.
- [ ] Refresh measurements under `evidence/perf/r7-cache/`.

Tests for this sub-phase:
- `cargo test pipeline_cache`.
- Chrome harness: poseidon2_basic with cache assertions.

### SP10 — Deferred matrix bring-up (R9)

Scope and purpose: All seven deferred fixtures produce succinct receipts in Chrome with matching cycles.

Implementation checklist:
- [ ] Run native CUDA baselines for `multi_test/rsa_compat`, `multi_test/keccak_union` (KeccakUnion(3)), `groth16-verifier`, `xgboost`, `bn254`, `risc0-zkvm-methods/blst`, `risc0-zkvm-methods/verify`.
- [ ] Run matching Chrome WebGPU proofs.
- [ ] Update `docs/wasm-webgpu-validation.md` matrix rows for each fixture.
- [ ] Capture per-fixture evidence under `evidence/perf/r9-deferred/<fixture>.md`.
- [ ] For any fixture that fails or times out: document the structural cause (workgroup-storage, async readback, runner budget) and either lift through another R# lever or scope to a follow-on run.

Tests for this sub-phase:
- Per-fixture Chrome harness invocations (existing test names).

### SP11 — Closing-condition audit (R10, R11, R13)

Scope and purpose: Final ratio table; guard-flag verification; full regression check; documented structural residuals.

Implementation checklist:
- [ ] Refresh `docs/wasm-webgpu-cuda-comparison.md` with final per-fixture ratios for all measured fixtures.
- [ ] Run the full 21-public-example matrix + internal parity matrix from `docs/wasm-webgpu-validation.md`. Assert all pass.
- [ ] Confirm `WEBGPU_EVAL_CHECK_ENABLE_SPLIT`, `WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS`, `WEBGPU_EVAL_CHECK_SPLIT_TERMS_PER_SHADER`, `WEBGPU_EVAL_CHECK_MAX_POLY_EXT_STEPS` remain at AS-IS values (or document evidence for any change).
- [ ] For every fixture with residual ratio > 1.0×: write a one-sentence structural-cause note in `docs/wasm-webgpu-cuda-comparison.md`.
- [ ] Capture closing-condition evidence under `evidence/perf/r10-r11-r13-closeout/`.

Tests for this sub-phase:
- Full Chrome harness pass (21 examples + internal parity matrix).

## Plan Drift Check

No in-scope drift between Phase 1 inventory and this plan. Coverage map:

- R1 → SP1 (direct)
- R2 → SP2 + SP3 (direct, split into prerequisite + production)
- R3 → SP4 + SP5 (direct, split into tiling-prereq + recursion-codegen)
- R4 → SP6 (direct)
- R5 → SP7 (direct)
- R6 → SP8 (direct)
- R7 → SP9 (direct)
- R8 → SP4 (direct, shared with R3-prereq)
- R9 → SP10 (indirect; the implementation surface for R9 is the lever sum of SP2–SP9)
- R10 → SP11 (indirect; R10 is a measurement aggregate)
- R11 → SP2 + SP11 (direct, as constraint at sub-phase entry + closeout audit)
- R12 → SP1 (direct; evidence-collection process)
- R13 → SP11 (direct; full regression check)
- R18 → out-of-scope (inherited via R13)

Phase 2 introduces only one structural change beyond Phase 1's inventory: it splits R2 across SP2 (tiny prototype) + SP3 (production wiring) and splits R3 across SP4 (tiling prereq) + SP5 (recursion codegen). Rationale: SP4 is a hard prerequisite for SP5 (recursion data group must be tiled before staged WGSL can address it); SP2 is a structural prerequisite for SP3 (the generator skeleton + parity test infrastructure must land before production wiring). Both splits are coverage-neutral.

## Subagent Contribution Verification

No subagents dispatched in Phase 2. Plan composition was done by the controller using Phase 0 + Phase 1 substrate. Subagents will return in Phase 3.5 (code review) and Phase 4 (test summary) when delegated review value is highest.

## Worktree Diff Audit

Baseline type: `local commit`
Baseline reference: `d042da45c89a1cd5f9cf7c5eb962a754367b2118`
Comparison reference: `working-tree`
Normalized baseline: `d042da45c89a1cd5f9cf7c5eb962a754367b2118`
Normalized comparison: `working-tree`
Normalized diff command: `git diff --name-only d042da45c89a1cd5f9cf7c5eb962a754367b2118`

Changed files reported (from worktree at HEAD `454b3109b` + uncommitted Phase 1 & Phase 2 artifacts):
- `.gitignore` (Phase 0)
- `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md` (Phase 0)
- `.recursive/run/wasm-webgpu-prover-perf/00-worktree.md` (Phase 0)
- `.recursive/run/wasm-webgpu-prover-perf/01-as-is.md` (Phase 1; LOCKED hash `6ae676746bbd…`)
- `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md` (Phase 2; this file)

All five are run-local artifacts filtered out of drift checks by `filter_runtime_changed_files`. No product source files modified yet.

## Gaps Found

None. Every R# has a sub-phase mapping; every Phase 1 Known Unknown has a planned resolution in either SP1 (cost subdivision in `lift_prove_async`) or SP5 (recursion data group tile stability) or SP6 (Keccak no-op accumulate reconciliation) or SP9 (pipeline + bind-group cache lifecycle) or is acknowledged as a per-environment unknown (Chrome 1 GiB `maxBufferSize` durability; device-loss threshold).

## Repair Work Performed

No product code changes. This phase's worktree state changes are: addition of `02-to-be-plan.md` only.

## Requirement Completion Status

- R1 | Status: planned | Implementation Surface: `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r1-baselines/` and `risc0/zkvm/src/host/server/prove/prover_impl.rs` (existing timer scaffolding) | Verification Surface: SP1 manual QA + per-fixture evidence files | QA Surface: SP1 manual scenario
- R2 | Status: planned | Implementation Surface: `risc0/zkp/src/hal/webgpu/eval_check_codegen/rv32im_gen.rs`, `risc0/zkp/src/hal/webgpu.rs`, `risc0/circuit/rv32im/src/prove/hal/webgpu.rs` | Verification Surface: SP2 + SP3 parity tests + browser harness assertions | QA Surface: SP3 manual scenario
- R3 | Status: planned | Implementation Surface: `risc0/zkp/src/hal/webgpu/eval_check_codegen/recursion_gen.rs`, `risc0/zkp/src/hal/webgpu/buffer_pool.rs`, `risc0/circuit/recursion/src/prove/hal/webgpu.rs`, `risc0/zkp/src/prove/prover.rs` | Verification Surface: SP4 + SP5 parity + recursion-sized regression | QA Surface: SP5 manual scenario
- R4 | Status: planned | Implementation Surface: `risc0/zkp/src/hal/webgpu/eval_check_codegen/keccak_gen.rs`, `risc0/circuit/keccak/src/prove/hal/webgpu.rs` | Verification Surface: SP6 parity tests + KeccakUnion(1) + KeccakUnion(3) browser assertions | QA Surface: SP6 manual scenario
- R5 | Status: planned | Implementation Surface: `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`, `risc0/circuit/keccak/src/prove/hal/webgpu.rs`, `risc0/circuit/recursion/src/prove/hal/webgpu.rs`, new WGSL kernels in `risc0/zkp/src/hal/webgpu.rs` | Verification Surface: SP7 CircuitHal parity tests + stage timer assertions | QA Surface: SP7 manual scenario
- R6 | Status: planned | Implementation Surface: `risc0/zkp/src/prove/prover.rs`, `risc0/zkp/src/prove/merkle.rs`, `risc0/zkp/src/hal/webgpu.rs` (diagnostics) | Verification Surface: SP8 browser harness readback assertions | QA Surface: SP8 manual scenario
- R7 | Status: planned | Implementation Surface: `risc0/zkp/src/hal/webgpu/pipeline_cache.rs`, `risc0/zkp/src/hal/webgpu.rs` | Verification Surface: SP9 cache tests + pipeline_creations assertion | QA Surface: SP9 manual scenario
- R8 | Status: planned | Implementation Surface: `risc0/zkp/src/hal/webgpu/buffer_pool.rs`, `risc0/zkp/src/hal/webgpu.rs` | Verification Surface: SP4 buffer_pool tests + recursion-sized regression | QA Surface: SP4 manual scenario
- R9 | Status: planned-indirectly | Implementation Surface: emergent from SP2–SP9; recorded under `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r9-deferred/` | Verification Surface: per-fixture browser proof entries in `docs/wasm-webgpu-validation.md` | QA Surface: SP10 manual scenarios | Rationale: R9 is a per-fixture acceptance batch whose code surface is shared with R2–R8. SP10 sequences the per-fixture runs after the gating R# levers land.
- R10 | Status: planned-indirectly | Implementation Surface: emergent from SP2–SP9; closing-condition assessment under `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r10-r11-r13-closeout/` and `docs/wasm-webgpu-cuda-comparison.md` | Verification Surface: SP11 ratio table | QA Surface: SP11 manual scenario | Rationale: R10 is a measurement aggregate over R2–R8 levers; the closing condition is naturally checked in SP11 after the levers land.
- R11 | Status: planned | Implementation Surface: `risc0/zkp/src/hal/webgpu.rs:75`–81 (constraint; no flag changes unless evidence) | Verification Surface: SP11 closing audit + Phase 3.5 review on each sub-phase | QA Surface: SP11 manual scenario
- R12 | Status: planned | Implementation Surface: `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/`, `risc0/zkvm/src/host/server/prove/prover_impl.rs` (timer additions in SP5) | Verification Surface: Phase 4 references the per-fixture evidence files | QA Surface: SP1 + SP5 manual scenarios
- R13 | Status: planned | Implementation Surface: `docs/requirements/wasm-webgpu-prover.md`, `docs/wasm-webgpu-validation.md` (regression line; no new code change planned beyond per-sub-phase regression checks) | Verification Surface: full 21-example + internal parity matrix in SP11 | QA Surface: SP11 manual scenario
- R18 | Status: out-of-scope | Rationale: R14–R18 of `docs/requirements/wasm-webgpu-prover.md` are inherited as a constraint via R13. They are not new deliverables; documentation hygiene happens naturally in Phase 8 memory updates. | Scope Decision: `.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`

## Audit Verdict

Phase 2 produces an ExecPlan-grade plan with 11 ordered sub-phases (SP1–SP11) covering every in-scope R# and every Phase 1 inventory item. Each sub-phase has scope, implementation checklist, planned files, tests, and manual QA scenarios. The Requirement Mapping preserves Phase 1 source quotes verbatim. R9 and R10 are coverage=indirect with explicit rationale because their implementation surface is shared with R2–R8. R18 is coverage=out-of-scope inherited via R13. No drift from Phase 1 inventory; only Phase 2-specific decomposition splits (R2 → SP2+SP3; R3 → SP4+SP5).

Audit: PASS

## Traceability

- R1 → SP1 → `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r1-baselines/`
- R2 → SP2 + SP3 → `risc0/zkp/src/hal/webgpu/eval_check_codegen/rv32im_gen.rs`, `risc0/zkp/src/hal/webgpu.rs:5855` fast-path, `risc0/circuit/rv32im/src/prove/hal/webgpu.rs:50`–72
- R3 → SP4 + SP5 → `risc0/zkp/src/hal/webgpu/eval_check_codegen/recursion_gen.rs`, `risc0/zkp/src/hal/webgpu/buffer_pool.rs`, `risc0/circuit/recursion/src/prove/hal/webgpu.rs:44`–66, `risc0/zkp/src/prove/prover.rs:694`–773
- R4 → SP6 → `risc0/zkp/src/hal/webgpu/eval_check_codegen/keccak_gen.rs`, `risc0/circuit/keccak/src/prove/hal/webgpu.rs:57`–79
- R5 → SP7 → `risc0/circuit/{rv32im,keccak,recursion}/src/prove/hal/webgpu.rs` witness/accumulate entry lines
- R6 → SP8 → `risc0/zkp/src/prove/prover.rs:625`, 667 (DEEP coalescing), `risc0/zkp/src/prove/merkle.rs:352`–422 (cross-tree readback)
- R7 → SP9 → `risc0/zkp/src/hal/webgpu/pipeline_cache.rs` + diagnostic counters at `risc0/zkp/src/hal/webgpu.rs:2430`–2446
- R8 → SP4 → `risc0/zkp/src/hal/webgpu/buffer_pool.rs`, `risc0/zkp/src/hal/webgpu.rs:6844`–6879
- R9 → SP10 → `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r9-deferred/`
- R10 → SP11 → `docs/wasm-webgpu-cuda-comparison.md` (refreshed ratios)
- R11 → SP2 + SP11 → `risc0/zkp/src/hal/webgpu.rs:75`–81 (constraint)
- R12 → SP1 + SP5 → `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/`, `risc0/zkvm/src/host/server/prove/prover_impl.rs:56`–83
- R13 → SP11 → `docs/wasm-webgpu-validation.md` (full matrix regression check)
- R18 → out-of-scope via R13

## Coverage Gate

- [X] Every R# (R1–R13 + R18 inherited) is mapped to one or more sub-phases.
- [X] Every sub-phase (SP1–SP11) has scope, implementation checklist, planned files, tests, and manual QA.
- [X] Planned Changes by File enumerates concrete paths for every sub-phase.
- [X] Requirement Mapping preserves Source Quotes verbatim from Phase 1 inventory.
- [X] Plan Drift Check enumerates the SP-specific decomposition splits (R2, R3) with rationale.
- [X] Idempotence and Recovery covers worktree branch hygiene, sub-phase-local env-var opt-outs, and device-loss handling.
- [X] Required audited-phase sections all present.

Coverage: PASS

## Approval Gate

- [X] ExecPlan is sufficient for Phase 3 to implement incrementally without ambiguity.
- [X] Sub-phase ordering matches the spec's R2 → R3 → R5 → R8 → R4 → R6/R7 → R9 attack order (with SP1 baseline capture front-loaded and SP11 closing-condition audit appended).
- [X] No Phase 1 Known Unknown is left without a planned resolution.
- [X] Failed-experiment ledger constraints from `01-as-is.md` are encoded into SP2–SP6 design constraints (per-stage `queue.submit()`, no monolithic shaders, no large private scratch as primary strategy).
- [X] No verifier-visible change planned (R13 constraint preserved).

Approval: PASS
