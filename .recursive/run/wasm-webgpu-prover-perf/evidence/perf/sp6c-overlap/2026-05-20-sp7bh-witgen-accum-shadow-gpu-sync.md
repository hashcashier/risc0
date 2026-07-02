# SP7bh - GPU-Witgen Accum Shadow Sync

Date: 2026-05-20

## Purpose

Move the opt-in GPU-witgen replacement path further off CPU by removing the post-data-commit CPU `step_Top` replay that repaired the CPU shadow for `TopAccum`.

This is a bridge, not the final architecture: CPU `TopAccum` still needs a coherent host shadow, so the replacement path now syncs only the row prefixes needed for GPU-owned replacement rows instead of rerunning CPU witness generation for those rows.

## RED

Added `witgen_accum_shadow_replay_rows()` instrumentation and e2e gates requiring the counter to stay unchanged across BusyLoop, KeccakUnion, and xgboost replacement proofs.

The first RED BusyLoop run generated and verified the receipt, then failed the new assertion:

- `rv32im_witgen_accum_shadow_replay cycles=262144 elapsed_ms=173`
- `rv32im_witgen_accum_shadow_replay rows=68239`
- assertion: `GPU-witgen replacement must not rerun CPU step_Top for BusyLoop accum shadow repair`
- BusyLoop `wall_ms=9077`

This proved the remaining CPU witness work was real and on the representative proof path.

## GREEN

Implementation changes:

- Added an explicit replay-row counter and public wasm test getter.
- Added `WebGpuCircuitHal::witgen_replace_accum_shadow_rows` to classify GPU-owned replacement rows by the data prefixes needed by CPU accumulation.
- Generalized sparse row-prefix sync attribution so the pre-accum bridge reports as `witgen_accum_shadow_rows`.
- Replaced the post-data-commit CPU `repair_witgen_gpu_replace_shadow_for_accum(...)` call with `sync_witgen_replace_accum_shadow_rows(...)`.
- Kept the existing CPU repair helper present but removed it from the active browser replacement path.
- Tightened BusyLoop, KeccakUnion, and xgboost e2e replacement tests so receipt verification must complete and the CPU replay counter must not advance.

## Verification

Commands:

- `cargo fmt`
- `git diff --check -- examples/browser-prove/src/lib.rs risc0/circuit/rv32im/src/prove/hal/rust_steps.rs risc0/circuit/rv32im/src/prove/hal/webgpu.rs risc0/circuit/rv32im/src/prove/mod.rs`
- `env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture`
- `env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture`

BusyLoop + KeccakUnion e2e passed:

- BusyLoop receipt verified.
- BusyLoop `wall_ms=9145`.
- BusyLoop first `iter6d_g_pre_witgen_dispatch_async=1546 ms`.
- BusyLoop `rv32im_witgen=401 ms`.
- BusyLoop `rv32im_witgen_accum_shadow_gpu_sync rows=68239`.
- BusyLoop `rv32im_accumulate step_top_accum=1987 ms`, `rv32im_witgen_accum=2033 ms`.
- BusyLoop `cpu_fallbacks=0`, `cpu_only_ops=0`.
- BusyLoop `readbacks=24`, `readback_bytes=37670192`.
- KeccakUnion receipt verified with `segments=4`, `pending_keccaks=9`, `assumptions=1`.
- KeccakUnion `wall_ms=103079`.
- KeccakUnion `cpu_fallbacks=0`, `cpu_only_ops=0`.
- KeccakUnion `readbacks=417`, `readback_bytes=77188072`.
- KeccakUnion `source=witgen_accum_shadow_rows readbacks=8 readback_bytes=67004128`.

xgboost e2e passed:

- Receipt journal: `30.528042544062632`.
- `segments=11`, `user_cycles=2294890`, `total_cycles=2883584`.
- `wall_ms=99541`.
- `gpu_active_ms=56631`, `gpu_idle_ratio=0.431`.
- First segment `iter6d_g_pre_witgen_dispatch_async=1560 ms`.
- First segment `rv32im_witgen=397 ms`.
- First segment `rv32im_witgen_accum_shadow_gpu_sync rows=73215`.
- First segment `rv32im_accumulate step_top_accum=1911 ms`, `rv32im_witgen_accum=1956 ms`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.
- `readbacks=374`, `readback_bytes=414487056`.
- `source=witgen_accum_shadow_rows readbacks=22 readback_bytes=406998464`.
- `recursion_data upload_bytes=2818572288`.

## Wall-Time Comparison

Compared with SP7bg:

- BusyLoop: `9039 -> 9145 ms`, `+106 ms` (`+1.2%`, slight regression/noise).
- KeccakUnion: `103665 -> 103079 ms`, `-586 ms` (`-0.6%`, slight improvement/noise).
- xgboost: `101512 -> 99541 ms`, `-1971 ms` (`-1.94%`).

Compared with SP7be/SP7bf xgboost:

- xgboost: `102938 -> 99541 ms`, `-3397 ms` (`-3.3%`).

This is accepted as a correctness-preserving GPU-witgen path improvement for multi-segment xgboost because it removes about 170 ms/segment of CPU replay and all representative proofs verify with zero CPU fallback/CPU-only ops. The tradeoff is explicit: xgboost now pays about 407 MB of sparse `witgen_accum_shadow_rows` readback. The next material target is to remove that bridge by making accumulation consume the GPU-owned witness directly, or by moving the relevant `TopAccum` work to WebGPU, instead of expanding more top-level witgen opcode arms.
