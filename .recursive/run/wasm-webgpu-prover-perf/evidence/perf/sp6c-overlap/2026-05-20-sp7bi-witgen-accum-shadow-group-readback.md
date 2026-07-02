# SP7bi - GPU-Witgen Accum Shadow Group Readback

Date: 2026-05-20

## Purpose

Coalesce the opt-in GPU-witgen replacement path's sparse accum-shadow row-prefix sync into one packed readback per segment instead of one readback per row group.

This is not the final witgen architecture. It reduces readback count, queue submits, temporary buffers, and readback calls, but it does not reduce the large `witgen_accum_shadow_rows` byte volume. The next material target remains removing the bridge by making accumulation consume GPU-owned witness data directly, or moving the relevant `TopAccum` work to WebGPU.

## RED

Added e2e gates requiring `witgen_accum_shadow_rows` readbacks to be coalesced:

- BusyLoop replacement proof: at most 1 accum-shadow readback.
- KeccakUnion replacement proof: at most 4 accum-shadow readbacks.
- xgboost replacement proof: at most 11 accum-shadow readbacks.

The RED BusyLoop run generated and verified a receipt, then failed the new assertion:

- BusyLoop `wall_ms=8796`.
- `source=witgen_accum_shadow_rows readbacks=2 readback_bytes=37191920`.
- Assertion: `readbacks=2 max_readbacks=1`.
- Message: `GPU-witgen accum shadow sync should coalesce row groups to at most one readback per segment`.

This proved the old path was still doing two readbacks per segment for the MISC0 arithmetic and bitwise row groups.

## GREEN

Implementation changes:

- Added `PACK_COLUMN_PREFIX_ROW_GROUPS_ELEM_WGSL`, a packed sparse column-prefix row-group kernel.
- Added `WebGpuBuffer::sync_gpu_column_prefix_row_groups_to_cpu_unchecked(...)`, which packs multiple `(column_prefix, rows)` groups into one GPU buffer and one named readback, then scatters the packed result back into the CPU shadow.
- Changed RV32IM `sync_witgen_replace_shadow_rows(...)` to sync MISC0 arithmetic, MISC0 bitwise, MISC2 compare, and MISC2 branch groups through the grouped helper.
- Kept the no-row path intact so empty groups still only mark the shadow range coherent.

## Verification

Commands:

- `cargo fmt`
- `git diff --check -- examples/browser-prove/src/lib.rs risc0/zkp/src/hal/webgpu.rs risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
- `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`
- `env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture`
- `env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture`

BusyLoop + KeccakUnion e2e passed with receipt verification and zero CPU fallback/CPU-only ops:

- BusyLoop `wall_ms=8953`.
- BusyLoop `gpu_active_ms=4023`, `gpu_idle_ratio=0.551`.
- BusyLoop `readbacks=23`, `readback_bytes=37670192`.
- BusyLoop `source=witgen_accum_shadow_rows readbacks=1 readback_bytes=37191920`.
- BusyLoop `queue_submits=170`, `raw_compute_dispatches=715`.
- BusyLoop grouped uploads: params `1/16 bytes`, rows `1/272956 bytes`, specs `1/32 bytes`.
- KeccakUnion `wall_ms=102416`.
- KeccakUnion `segments=4`, `pending_keccaks=9`, `assumptions=1`.
- KeccakUnion `readbacks=413`, `readback_bytes=77188072`.
- KeccakUnion `source=witgen_accum_shadow_rows readbacks=4 readback_bytes=67004128`.
- KeccakUnion `queue_submits=3021`, `raw_compute_dispatches=12650`.

xgboost e2e passed with receipt verification and zero CPU fallback/CPU-only ops:

- Receipt journal: `30.528042544062632`.
- `segments=11`, `user_cycles=2294890`, `total_cycles=2883584`.
- `wall_ms=99330`.
- `gpu_active_ms=56536`, `gpu_idle_ratio=0.431`.
- `readbacks=363`, `readback_bytes=414487056`.
- `source=witgen_accum_shadow_rows readbacks=11 readback_bytes=406998464`.
- `queue_submits=2665`, `raw_compute_dispatches=11380`.
- Grouped upload sources: params `11/176 bytes`, rows `11/3021936 bytes`, specs `11/352 bytes`.
- First segment `iter6d_g_pre_witgen_dispatch_async=1452 ms`.
- First segment `rv32im_witgen=398 ms`.
- First segment `rv32im_witgen_accum_shadow_gpu_sync rows=73215`.
- First segment `rv32im_accumulate step_top_accum=1922 ms`, `rv32im_witgen_accum=1970 ms`.

## Wall-Time Comparison

Compared with SP7bh:

- BusyLoop: `9145 -> 8953 ms`, `-192 ms` (`-2.1%`, directional/noisy).
- KeccakUnion: `103079 -> 102416 ms`, `-663 ms` (`-0.6%`, directional/noisy).
- xgboost: `99541 -> 99330 ms`, `-211 ms` (`-0.2%`, noise-level).

The deterministic gain is the readback shape:

- BusyLoop accum-shadow source: `2 -> 1` readbacks.
- KeccakUnion accum-shadow source: `8 -> 4` readbacks.
- xgboost accum-shadow source: `22 -> 11` readbacks.
- xgboost total readbacks: `374 -> 363`.
- xgboost accum-shadow bytes stayed `406998464`, so the main data-movement bottleneck remains.

Accepted as a small production bridge improvement. It does not change the priority: move accumulation/TopAccum over GPU-owned witgen data so the 407 MB shadow transfer disappears instead of being better packaged.
