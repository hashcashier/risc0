# SP7bg - GPU-Witgen Replacement Prewarm Pending Promises

Date: 2026-05-20

## Purpose

Continue the GPU-witgen replacement path without expanding opcode scope. The target was the first-segment compile wall that still appeared in representative e2e proofs after MISC0 shadow readback elision.

## RED

Added an e2e gate for `witgen_gpu_replace_on_demand_kernel_compiles()` and required it to stay unchanged across BusyLoop, KeccakUnion, and xgboost replacement proofs.

The first RED run generated and verified the BusyLoop receipt, then failed the new assertion:

- `iter6d_g_on_demand_compile_count`: 5
- on-demand labels: `misc0_chunk0` chunk0, `misc0_chunk0` chunk1, `misc0_chunk3`, `misc0_chunk4`, `misc0_chunk7`
- BusyLoop `iter6d_g_pre_witgen_dispatch_async`: 3279 ms
- BusyLoop `iter6d_d_witgen_prewarm_async`: 3838 ms
- BusyLoop `prove_session_async wall_ms`: 10858

This proved the issue was real: correctness was intact, but the first segment was still making replacement kernels compile on the proof critical path.

## GREEN

Implementation changes:

- Added `WebGpuStartedComputeKernel` plus `WebGpuHal::start_compute_kernel_async` / `finish_compute_kernel_async`, so `createComputePipelineAsync` promises start immediately instead of only when a Rust future is first polled.
- Started RV32IM replacement prewarm from `WebGpuProver::from_hal`, before the first proof's execute/preflight work.
- Stored pending replacement compile handles in RV32IM thread-local maps.
- Taught `ensure_witgen_replace_arm_ready_async` to await pending prewarm promises instead of launching duplicate on-demand compiles.
- Skipped the old full top-chunk prewarm when replacement mode is enabled; the replacement path does not consume those kernels.
- Representative e2e tests now assert zero replacement on-demand compiles for BusyLoop, KeccakUnion, and xgboost.

## Verification

Commands:

- `cargo fmt`
- `git diff --check`
- `env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture`
- `env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture`

BusyLoop + KeccakUnion e2e passed:

- BusyLoop receipt verified.
- BusyLoop `wall_ms=9039`, `iter6d_g_pre_witgen_dispatch_async=1518 ms`, prewarm completed in `2086 ms`.
- BusyLoop on-demand replacement compile count stayed `0`.
- BusyLoop `cpu_fallbacks=0`, `cpu_only_ops=0`, no `witgen_data_shadow_rows`.
- KeccakUnion receipt verified with `segments=4`, `pending_keccaks=9`, `assumptions=1`.
- KeccakUnion `wall_ms=103665`, on-demand replacement compile count stayed `0`.
- KeccakUnion `cpu_fallbacks=0`, `cpu_only_ops=0`, no `witgen_data_shadow_rows`.

xgboost e2e passed:

- Receipt journal: `30.528042544062632`.
- `segments=11`, `wall_ms=101512`.
- First segment `iter6d_g_pre_witgen_dispatch_async=1432 ms`; later hot segments were ~12 ms.
- On-demand replacement compile count stayed `0`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`, no `witgen_data_shadow_rows`.
- `recursion_data upload_bytes=2818572288`.

## Wall-Time Comparison

Compared with SP7be/SP7bf:

- BusyLoop: `10663 -> 9039 ms`, `-1624 ms` (`-15.2%`).
- KeccakUnion: `103174 -> 103665 ms`, `+491 ms` (`+0.5%`, treated as flat/noisy).
- xgboost: `102938 -> 101512 ms`, `-1426 ms` (`-1.4%`).

Accepted as a replacement-path improvement because it removes duplicate first-segment replacement compiles and improves two of the representative walls without changing proof semantics. It is not enough to promote replacement over the best default xgboost baseline yet; the remaining material blocker is still the CPU accumulation dependency / host shadow replay for GPU-owned MISC0 rows.
