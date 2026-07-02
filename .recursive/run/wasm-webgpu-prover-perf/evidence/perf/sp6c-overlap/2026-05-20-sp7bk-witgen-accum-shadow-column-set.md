# SP7bk - Witgen Accum-Shadow Column-Set Sync

## Goal

Reduce the remaining GPU-witgen `witgen_accum_shadow_rows` bridge without changing proof semantics or reintroducing CPU fallback.

The previous accepted path packed row groups, but still read broad prefixes:

- BusyLoop: `37191920` bytes.
- KeccakUnion: `67004128` bytes.
- xgboost: `406998464` bytes.

## RED

Added an xgboost e2e gate requiring the GPU-witgen accum-shadow sync to stay below `390000000` bytes.

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: expected failure after proof generation.

- `source=witgen_accum_shadow_rows readbacks=11 readback_bytes=406990528`.
- New assertion failed: `readback_bytes=406990528 max_bytes=390000000`.
- Diagnostics still showed `cpu_fallbacks=0`, `cpu_only_ops=0`.

## Change

Added `WebGpuBuffer::sync_gpu_column_set_row_groups_to_cpu_unchecked(...)` plus `PACK_COLUMN_SET_ROW_GROUPS_ELEM_WGSL`.

RV32IM GPU-witgen accum-shadow repair now uses explicit column sets for MISC0 replacement rows:

- MISC0 arithmetic: `0, 1, 14..132`.
- MISC0 bitwise: `0, 1, 14..196`.
- MISC2 row groups remain prefix-shaped for future opt-in paths, but production replacement still excludes MISC2.

Rationale: for current MISC0 major-0 rows, column `1` selects the major-0 branch. Top-level selector columns `2..13` are inactive once that branch is selected, so broad prefix sync was transferring 12 dead cells for every GPU-owned MISC0 row.

## Compile

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
```

Result: PASS in `4m50s`.

## GREEN - xgboost

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: PASS in `99.73s`.

- `wall_ms=99493`.
- `segments=11`.
- `user_cycles=2294867`.
- `total_cycles=2883584`.
- Journal: `30.528042544062632`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.
- `readbacks=363`.
- `readback_bytes=378216656`.
- `source=witgen_accum_shadow_rows readbacks=11 readback_bytes=370728064`.
- `raw_compute_dispatches=11380`.
- `queue_submits=2665`.
- `compute_pipeline_creations=29`.

Delta vs SP7bj xgboost:

- `witgen_accum_shadow_rows`: `406998464 -> 370728064` (`-36270400` bytes, about `-8.9%` for that source).
- total readback bytes: `414487056 -> 378216656` (`-36270400` bytes).
- wall: `99736 -> 99493` (`-243 ms`, noise-level).

## GREEN - BusyLoop + KeccakUnion

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: PASS in `112.41s`.

BusyLoop:

- `wall_ms=9144`.
- `segments=1`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.
- `source=witgen_accum_shadow_rows readbacks=1 readback_bytes=34394720`.
- `raw_compute_dispatches=715`.
- `queue_submits=170`.

KeccakUnion:

- `wall_ms=102952`.
- `segments=4`.
- `pending_keccaks=9`.
- `assumptions=1`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.
- `source=witgen_accum_shadow_rows readbacks=4 readback_bytes=60937792`.
- `raw_compute_dispatches=12650`.
- `queue_submits=3021`.

Delta vs SP7bj:

- BusyLoop source bytes: `37191920 -> 34394720` (`-2797200`).
- KeccakUnion source bytes: `67004128 -> 60937792` (`-6066336`).
- BusyLoop wall moved `8873 -> 9144` (`+271 ms`, noisy).
- KeccakUnion wall moved `104093 -> 102952` (`-1141 ms`, noisy).

## Acceptance

Accepted as a correctness-preserving data-movement reduction. It does not eliminate the bridge; xgboost still reads back `370728064` bytes of GPU-owned witness data for CPU `TopAccum`.

Next material target remains making accumulation/TopAccum consume GPU-owned witness data directly, not expanding more same-pattern MISC0 witgen minors.
