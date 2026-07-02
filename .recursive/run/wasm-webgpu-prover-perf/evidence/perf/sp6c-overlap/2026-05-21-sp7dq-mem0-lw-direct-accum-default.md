# SP7dq - MEM0 LW Direct Accum Default

Date: 2026-05-21

## Scope

SP7dp proved the all-minor MEM0 direct-accumulator candidate was correctness-clean and xgboost-positive, but BusyLoop+KeccakUnion-negative. SP7dq narrows MEM0 replacement to the dominant load-word minor (`minor=2`) and promotes that shape to the default browser WebGPU acceleration setup only after representative e2e proof generation.

## Changed Surface

- `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs`
  - Added MEM0 minor-mask state and test coverage.
  - `mem0_short_circuit_minor` now respects the selected minor mask.
- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
  - Added MEM0 minor-mask state, getter/setter, and prewarm counter.
  - MEM0 replacement dispatch and direct-accum row selection now respect the minor mask.
  - MEM0 extra prewarm now requests only selected MEM0 extra minors.
  - Default `enable_webgpu_witgen_accum_acceleration_for_hal` now enables MEM0 LW replacement plus MEM0 direct accumulation before prewarm.
- `risc0/circuit/rv32im/src/prove/mod.rs`
  - Re-exported the new MEM0 controls/counters for browser e2e assertions.
- `examples/browser-prove/src/lib.rs`
  - Added MEM0 LW candidate e2e gates for BusyLoop+KeccakUnion and xgboost.
  - Strengthened default xgboost to assert MEM0 LW direct accumulation is active.
  - Updated the representative BusyLoop+KeccakUnion default gate to accept only the LW direct-accum MEM0 path, not the rejected all-minor production path.
  - Hardened shared browser proof diagnostics so any `cpu_fallbacks > 0` fails the e2e gate.

## RED/GREEN Evidence

### MEM0 Minor Mask

- RED: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_mem0_lw_direct_accum_candidate_e2e_verify --no-run`
  - Failed on missing `set_witgen_gpu_mem0_replace_minor_mask` and `witgen_gpu_mem0_replace_minor_mask`.
- GREEN: same compile command passed in `4m25s`.

### MEM0 Prewarm Filter

- RED: candidate xgboost compile failed on missing `witgen_gpu_mem0_extra_prewarm_requests`.
- GREEN: same compile command passed in `4m26s`.
- Candidate e2e after the filter:
  - BusyLoop+KeccakUnion candidate passed in `99.78s`; high limits; `mask=0x0021`; only MEM0 minor `2`; `MEM0 direct_gpu dispatched rows=40795`; `mem0_extra requested=1`.
  - xgboost candidate passed in `76.63s`; high limits; journal `30.528042544062632`; segment 0 MEM0 minor `2` rows `40301`; segment 1 `41337`; `mem0_extra requested=1`.

### Default Promotion

- RED: default xgboost e2e under high limits failed before proving because default MEM0 minor mask was still all-minor (`left: 31`, `right: 4`).
- GREEN: default xgboost e2e passed after promotion in `76.23s`; high limits; verified journal; `mask=0x0021`; MEM0 minor `2`; `MEM0 direct_gpu dispatched rows=40301` in segment 0 and `41337` in segment 1; `mem0_extra requested=1`.
- RED after promotion: default BusyLoop+KeccakUnion e2e under high limits reached proof generation, then failed on the stale assertion forbidding any MEM0 production replacement. Logs showed the intended LW-only path was active (`mask=0x0021`, MEM0 minor `2`, direct MEM0 rows).
- GREEN after updating representative assertions: default BusyLoop+KeccakUnion e2e passed in `99.71s`; high limits; verified receipts.
- Final hardened GREEN: default BusyLoop+KeccakUnion e2e passed in `100.50s` after shared `cpu_fallbacks == 0` assertion was added.
- Final hardened GREEN: default xgboost e2e passed in `76.10s` after shared `cpu_fallbacks == 0` assertion was added.

Invalid low-limit Chrome sessions (`1073741824 / 1073741824 / 32768`) were rejected by `assert_representative_webgpu_limits` and are excluded from performance conclusions.

## Final Representative Results

Accepted latest-state e2e proof gates:

| Workload | Prior reference | Final | Movement | Notes |
|---|---:|---:|---:|---|
| xgboost | SP7do post-revert `78.62s` | `76.10s` | `-2.52s` / `-3.2%` | Verified succinct receipt and journal; zero CPU fallback/CPU-only; MEM0 LW direct accumulation active. |
| xgboost | SP7dp default sanity `78.33s` | `76.10s` | `-2.23s` / `-2.8%` | Same default proof path. |
| BusyLoop+KeccakUnion | SP7do post-revert `99.56s` | `100.50s` | `+0.94s` / `+0.9%` | Verified receipts; zero CPU fallback/CPU-only; considered flat/noisy rather than a material regression. |
| BusyLoop+KeccakUnion | SP7dq first GREEN `99.71s` | `100.50s` | `+0.79s` | Difference is within observed browser-run noise. |

The default path now gets the xgboost-positive MEM0 LW direct-accum win without the broad MEM0 shadow repair that made SP7do's all-minor production attempt wall-negative.

## Decision

Accept MEM0 LW direct accumulation as default WebGPU acceleration.

Rationale:

- Correctness is covered by receipt verification on BusyLoop, KeccakUnion, and xgboost.
- Representative high-limit gates enforce zero `cpu_fallbacks` and zero `cpu_only_ops`.
- xgboost improves by about `2.5s` from the SP7do post-revert reference.
- BusyLoop+KeccakUnion remains effectively flat, with no material wall improvement and no correctness/fallback regression.
- MEM0 prewarm is narrowed to one extra minor instead of the prior three-minor all-MEM0 prewarm.

Accepted production wall-time gain: about `2.5s` on xgboost (`~3%`). No accepted BusyLoop+KeccakUnion wall gain.

## Hygiene

- `cargo fmt --manifest-path risc0/circuit/rv32im/Cargo.toml`: PASS
- `cargo fmt --manifest-path examples/browser-prove/Cargo.toml`: PASS
- `cargo fmt --check --manifest-path risc0/circuit/rv32im/Cargo.toml`: PASS
- `cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml`: PASS
- `git diff --check`: PASS

## Next Target

The next immediate material target remains the measured FRI/check `batch_expand_into_evaluate_ntt` bucket. Earlier dispatch-count-only or branch-only NTT reshaping attempts were rejected, so the next attempt needs a memory-pass-reducing design with focused active-time evidence before full e2e proof generation.
