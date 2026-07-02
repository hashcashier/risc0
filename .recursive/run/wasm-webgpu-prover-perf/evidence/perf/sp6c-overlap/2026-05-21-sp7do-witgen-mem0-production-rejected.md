# SP7do MEM0 GPU-Witgen Production Rejection

Date: 2026-05-21

## Candidate

Enable production GPU-witgen replacement for MEM0 after the MEM0 diff path proved cell-complete.

The candidate kept the current MISC0 production replacement and added MEM0 short-circuit replay for the lookup-table side effects driven by MEM0 minor rows.

## TDD Evidence

RED:

- Tightened `iter6d_g_replace_busy_loop_e2e_verify` to require MEM0 production replacement after the focused diff path was cell-complete.
- Command: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture`
- Expected failure observed after BusyLoop proof generation: `MEM0 diff is now complete; replacement should short-circuit BusyLoop MEM0 rows`.
- The failure showed production replacement still used `mask=0x0001`, so MEM0 was not enabled.

GREEN candidate:

- Added direct MEM0 lookup side-effect replay helpers in `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs`.
- Added MEM0 replacement prewarm/on-demand plumbing in `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`.
- Temporarily enabled production replacement for arms `[0, 5]`.
- `cargo fmt --check --manifest-path risc0/circuit/rv32im/Cargo.toml`: passed.
- `cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml`: passed.
- `git diff --check`: passed.
- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`: passed; compile took `4m22s`.

Focused MEM0 diff basis:

- Prior focused diff evidence showed MEM0 mismatches cleared: `DIFF_SUMMARY ... mismatches=0 ... candidate_cpu_only_nonzero=0`.
- BusyLoop MEM0 minor dispatch coverage included `arm=5 minor=2 cycles=40795` and `arm=5 minor=3 cycles=171`.

## Representative E2E Evidence

BusyLoop + KeccakUnion with MEM0 production enabled:

- Command: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture`
- Result: passed with verified receipts and high WebGPU limits.
- Total test time: `100.90s`.
- Limits: `max_buffer_size=4294967292`, `max_storage_buffer_binding_size=2147483644`, `max_compute_workgroup_storage_size=49152`.
- BusyLoop used `mask=0x0021`, `dispatched_arms=[0, 5]`.
- MEM0 replacement fired: `iter6d_g_minor_dispatch arm=5 minor=2 cycles=40795`, `iter6d_g_minor_dispatch arm=5 minor=3 cycles=171`.
- New repair cost appeared: `rv32im_witgen_accum_shadow_gpu_sync rows=40968`.
- BusyLoop `pre_witgen_dispatch_async=2519 ms`, `rv32im_witgen=299 ms`, `segment_prove_core_async=6305 ms`.

xgboost with MEM0 production enabled:

- Command: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture`
- Result: passed with verified receipt/journal and high WebGPU limits.
- Total test time: `79.29s`.
- `segments=11`.
- Segment 0 used `mask=0x0021`, `dispatched_arms=[0, 5]`.
- MEM0 replacement fired: `iter6d_g_minor_dispatch arm=5 minor=2 cycles=40301`, `iter6d_g_minor_dispatch arm=5 minor=3 cycles=2`.
- New repair cost appeared: `rv32im_witgen_accum_shadow_gpu_sync rows=40303`.
- Prewarm included MEM0 extras: `iter6d_g_prewarm mem0_extra requested=3`.

## Decision

Rejected for production/default. The candidate was correctness-clean, but wall-negative against the accepted working state. The MEM0 CPU-witgen saving was erased by broad sparse accumulator-shadow repair and extra replacement prewarm work.

Accepted wall-time gain: `0`.

Keep the MEM0 diff/replay substrate available, but leave production replacement MISC0-only until MEM0 can update or repair the accumulator without a broad `rv32im_witgen_accum_shadow_gpu_sync` readback/repair path.

## Post-Revert Accepted State

Production replacement was reverted to MISC0-only:

- `WITGEN_REPLACE_SUPPORTED_ARM_MASK = 1u16 << 0`.
- `WITGEN_REPLACE_SUPPORTED_ARMS = &[0]`.
- MEM0 prewarm remains gated behind `is_witgen_replace_supported_arm(5)`, so it is not requested in accepted/default mode.
- `rust_steps::cycle_short_circuited` excludes MEM0.
- `iter6d_g_replace_busy_loop_e2e_verify` now asserts that MEM0 remains excluded until the accumulator repair cost is fixed.

Post-revert verification:

- `cargo fmt --check --manifest-path risc0/circuit/rv32im/Cargo.toml`: passed.
- `cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml`: passed.
- `git diff --check`: passed.
- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture`: passed; total `78.62s`, `segments=11`, segment 0 `mask=0x0001`, `dispatched_arms=[0]`, no MEM0 prewarm.
- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture`: passed; total `99.56s`, BusyLoop `mask=0x0001`, `dispatched_arms=[0]`, no MEM0 prewarm, `rv32im_witgen=363 ms`, `segment_prove_core_async=5390 ms`.

Latest accepted xgboost working state remains noise-level around the SP7dh/SP7cy band (`78.62s` in this post-revert sample; prior best estimate about `77.5s`).

