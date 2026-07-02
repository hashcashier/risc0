# SP7dr - MEM1 Direct Accum Default

Date: 2026-05-21

## Scope

SP7dr adds a narrow direct accumulator for RV32IM MEM1/store rows (`major=6`). Unlike MEM0, MEM1 rows remain CPU-witgen-owned; only their lookup-accumulator contributions move to WebGPU, allowing the CPU TopAccum loop to skip another full major.

## Changed Surface

- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
  - Added MEM1 direct-accum flag/counter and WGSL kernel.
  - Extended grouped direct-accumulator dispatch to include MEM1.
  - Skips major `6` in the CPU TopAccum pass when MEM1 direct accumulation is enabled.
  - Promoted MEM1 direct accumulation in `enable_webgpu_witgen_accum_acceleration_for_hal`.
- `risc0/circuit/rv32im/src/prove/mod.rs`
  - Re-exported MEM1 direct-accum controls/counters for browser e2e assertions.
- `examples/browser-prove/src/lib.rs`
  - Added opt-in MEM1 candidate e2e gates for BusyLoop+KeccakUnion and xgboost.
  - Tightened default BusyLoop+KeccakUnion and xgboost gates to assert MEM1 direct rows are covered.

## RED/GREEN Evidence

- RED: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_mem1_direct_accum_candidate_e2e_verify --no-run`
  - Failed on missing `accum_gpu_mem1_direct_rows` and `set_accum_gpu_mem1_direct_enabled`.
- GREEN: same compile gate passed in `4m22s` after the minimal implementation.
- Promotion compile: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run` passed in `4m23s` after default enablement and default assertion updates.

Invalid low-limit Chrome sessions (`1073741824 / 1073741824 / 32768`) were rejected by `assert_representative_webgpu_limits` and are excluded from performance conclusions.

## Candidate E2E Results

High-limit candidate proof gates:

| Workload | Result | Notes |
|---|---:|---|
| BusyLoop+KeccakUnion candidate | `104.37s` | Correctness clean but noisy/worse; first proof showed a large accumulator Merkle root wait. |
| xgboost candidate | `74.47s` | Verified journal `30.528042544062632`; zero fallback assertions; `segments=11`; MEM1 direct rows active. |
| BusyLoop+KeccakUnion candidate repeat | `99.03s` | Verified receipts; zero fallback assertions; falsified the first run as a stable regression. |

Candidate logs showed `step_top_accum_cpu_skip_replaced_misc0=true_mem0=true_major_mask=0x0046`, proving major `6` was skipped by CPU TopAccum while MEM1 was written by the grouped direct-accumulator kernel.

## Default Promotion Results

Accepted latest-state default e2e proof gates:

| Workload | Prior accepted | Final | Movement | Notes |
|---|---:|---:|---:|---|
| xgboost | SP7dq `76.10s` | `74.29s` | `-1.81s` / `-2.4%` | Verified succinct receipt and journal; zero CPU fallback/CPU-only; MEM1 direct accumulation active. |
| xgboost | Fresh SP7dq revalidation `76.33s` | `74.29s` | `-2.04s` / `-2.7%` | Same default proof path. |
| BusyLoop+KeccakUnion | SP7dq `100.50s` | `99.49s` | `-1.01s` / `-1.0%` | Verified receipts; zero CPU fallback/CPU-only; MEM1 direct accumulation active. |
| BusyLoop+KeccakUnion | Candidate repeat `99.03s` | `99.49s` | `+0.46s` | Difference is within observed browser-run noise. |

Representative default logs:

- xgboost segment 0: `MEM1 direct_gpu dispatched rows=32927`; CPU TopAccum skip mask `0x0046`; RV32IM accumulate `613ms`.
- xgboost segment 1: `MEM1 direct_gpu dispatched rows=33684`.
- BusyLoop: `MEM1 direct_gpu dispatched rows=31917`; CPU TopAccum skip mask `0x0046`; RV32IM accumulate `808ms`.

## Decision

Accept MEM1 direct accumulation as default WebGPU acceleration.

Rationale:

- Correctness is covered by e2e proof generation on BusyLoop, KeccakUnion, and xgboost.
- The default gates assert high WebGPU limits and zero `cpu_fallbacks` / `cpu_only_ops`.
- xgboost improves by about `1.8s` from the prior accepted state.
- BusyLoop+KeccakUnion is not regressed in the promoted default run and is slightly better than SP7dq.
- The change is narrow: MEM1 witness generation remains CPU-owned; only accumulator lookup terms move to WebGPU.

Accepted production wall-time gain: about `1.8s` on xgboost (`~2.4%`) and about `1.0s` on BusyLoop+KeccakUnion (`~1%`, still treated as small/noisy).

## Hygiene

- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_mem1_direct_accum_candidate_e2e_verify --no-run`: PASS
- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`: PASS
- `cargo fmt --manifest-path risc0/circuit/rv32im/Cargo.toml --check`: PASS
- `cargo fmt --manifest-path examples/browser-prove/Cargo.toml --check`: PASS
- `git diff --check`: PASS

## Next Target

The remaining near-term RV32IM accumulator targets are smaller and riskier. The next candidate should either find another full-major direct accumulator with low inverse pressure or return to FRI/check only if there is a concrete memory-pass-reducing NTT design; dispatch-count-only NTT reshapes have already been rejected.
