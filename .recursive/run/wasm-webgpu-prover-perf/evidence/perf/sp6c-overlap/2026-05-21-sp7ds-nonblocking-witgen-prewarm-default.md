# SP7ds - Nonblocking Witgen Prewarm Default

Date: 2026-05-21

## Scope

SP7ds promotes the nonblocking replacement-kernel prewarm behavior to the default browser WebGPU prover path. When a segment needs GPU-witgen replacement but the replacement kernels are still compiling, the prover now skips replacement for that cold segment instead of blocking on the pending Tint/kernel work. The background prewarm still continues, and later segments use the normal GPU-witgen replacement path once the kernels are ready.

## Changed Surface

- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
  - Added default enablement via `set_witgen_gpu_replace_nonblocking_pending_enabled(true)` in `enable_webgpu_witgen_accum_acceleration_for_hal`.
  - Retained the candidate implementation that skips only when replacement arms are needed but no replacement arm is ready.
- `examples/browser-prove/src/lib.rs`
  - Updated the default BusyLoop+KeccakUnion gate so a cold BusyLoop segment may skip pending replacement without weakening the ready-kernel assertions.
  - Kept the KeccakUnion and xgboost gates as the proof that replacement resumes after prewarm catches up.

## RED/GREEN Evidence

- RED: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_nonblocking_witgen_prewarm_candidate_xgboost_e2e_verify --no-run`
  - Failed on missing `set_witgen_gpu_replace_nonblocking_pending_enabled` and `witgen_gpu_replace_nonblocking_pending_skips`.
- GREEN: same compile gate passed in `4m19s` after adding the flag/counter and nonblocking skip logic.
- Candidate BusyLoop+KeccakUnion compile gate passed in `2m18s`.
- Promotion compile: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run` passed in `4m26s`.

Invalid low-limit Chrome sessions (`1073741824 / 1073741824 / 32768`) remain excluded by `assert_representative_webgpu_limits`. Accepted runs below negotiated `4294967292 / 2147483644 / 49152`.

## Candidate E2E Results

High-limit candidate proof gates before promotion:

| Workload | Result | Notes |
|---|---:|---|
| xgboost candidate | `73.19s` | Verified journal `30.528042544062632`; zero fallback assertions; first segment skipped pending replacement (`pre_witgen=2ms`), later segments used replacement (`mask=0x0021`). |
| BusyLoop+KeccakUnion candidate | `99.37s` | Verified receipts; zero fallback assertions; BusyLoop skipped pending replacement; KeccakUnion used replacement after prewarm. |

Fresh default xgboost before promotion was `74.78s` with segment-0 `pre_witgen_dispatch_async=2123ms` waiting for prewarm. Candidate xgboost reduced that cold wait to `2ms`; the skipped cold segment then paid CPU witness/accum cost, but still improved wall time by `1.59s` versus that fresh default run.

## Default Promotion Results

Accepted latest-state default e2e proof gates:

| Workload | Prior accepted | Final | Movement | Notes |
|---|---:|---:|---:|---|
| xgboost | SP7dr `74.29s` | `73.74s` | `-0.55s` / `-0.7%` | Verified succinct receipt and journal; zero fallback/CPU-only assertions; segment 0 skipped cold wait; segment 1 used replacement. |
| xgboost | Fresh pre-promotion default `74.78s` | `73.74s` | `-1.04s` / `-1.4%` | Same default proof path in the current tree. |
| BusyLoop+KeccakUnion | SP7dr `99.49s` | `98.82s` | `-0.67s` / `-0.7%` | Verified receipts; zero fallback/CPU-only assertions; BusyLoop skipped cold wait and KeccakUnion used replacement after prewarm. |

Representative default logs:

- xgboost segment 0: `nonblocking_pending_skip`, `mask=0x0000 no_ready_replacement no_sync`, `pre_witgen_dispatch_async=2ms`, `rv32im_witgen=506ms`, `rv32im_witgen_accum=1271ms`.
- xgboost prewarm completed during segment 0: `iter6d_d_witgen_prewarm_async elapsed_ms=2226`.
- xgboost segment 1: `mask=0x0021 dispatched_arms=[0, 5]`, `pre_witgen_dispatch_async=16ms`, `rv32im_witgen=279ms`, `rv32im_witgen_accum=578ms`.
- BusyLoop segment: `nonblocking_pending_skip`, `pre_witgen_dispatch_async=2ms`, `rv32im_witgen=540ms`, CPU TopAccum `step_top_accum_cpu_skip_replaced_misc0=false_mem0=false_major_mask=0x0046` in `1371ms`, `rv32im_witgen_accum=1412ms`.
- BusyLoop prewarm completed during the proof: `iter6d_d_witgen_prewarm_async elapsed_ms=2437`; KeccakUnion then passed the replacement assertions.

## Decision

Accept nonblocking replacement-kernel prewarm as default.

Rationale:

- Correctness is covered by e2e proof generation on BusyLoop, KeccakUnion, and xgboost.
- The default gates assert representative high WebGPU limits and zero `cpu_fallbacks` / `cpu_only_ops`.
- The change avoids a multi-second cold wait on the first segment while preserving the existing GPU-witgen replacement path for subsequent segments.
- The wall-time movement is positive on both representative gates in the promoted default run.

Accepted production wall-time gain: about `0.55s` on xgboost versus SP7dr (`~0.7%`), or `1.04s` versus the same-day fresh pre-promotion default; about `0.67s` on BusyLoop+KeccakUnion (`~0.7%`, still small/noisy).

## Hygiene

- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_nonblocking_witgen_prewarm_candidate_xgboost_e2e_verify --no-run`: RED then GREEN as above.
- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_nonblocking_witgen_prewarm_candidate_xgboost_e2e_verify -- --nocapture`: PASS, `73.19s`.
- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_nonblocking_witgen_prewarm_candidate_e2e_verify -- --nocapture`: PASS, `99.37s`.
- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`: PASS, `4m26s`.
- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture`: PASS, `98.82s`.
- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture`: PASS, `73.74s`.
- `cargo fmt --manifest-path examples/browser-prove/Cargo.toml --check`: PASS.
- `cargo fmt --manifest-path risc0/circuit/rv32im/Cargo.toml --check`: PASS.
- `git diff --check`: PASS.

## Next Target

Move back to the primary requested lever: more witness-generation work on GPU, not scheduler sidequests. The current profile shows steady-state replacement segments at about `279ms` witness + `578ms` witness-accum, while a cold skipped segment pays about `506ms` witness + `1271ms` witness-accum. The next useful work should target remaining CPU-owned witgen/TopAccum rows only when a candidate can be screened against BusyLoop+KeccakUnion before xgboost.
