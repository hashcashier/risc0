# SP7bd sparse zeroize range metadata compression

Date: 2026-05-19

## Goal

Reduce host-to-GPU upload bytes on the GPU-witgen data repair path by removing dead metadata from the sparse zeroize upload range table.

The existing sparse zeroize WGSL only used `src_start` and `dst_start` for each range. The uploaded `len` and padding words doubled range-table bytes without affecting semantics.

## RED

Test change:

- Expanded `webgpu_hal_data_zeroize_sparse_uploads_only_nonzero_values` with many singleton nonzero runs.
- Added an assertion that `webgpu_zeroize_sparse_ranges` must be at most `2x` the value payload, rejecting the old four-u32 range encoding.

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=180 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_data_zeroize_sparse_uploads_only_nonzero_values -- --nocapture
```

Expected failure observed:

- GPU zeroize executed and readback matched up to the new assertion point.
- `values=213`, `ranges=131`.
- `sparse_bytes=2964`, `dense_bytes=16384`.
- `webgpu_zeroize_sparse_values=852`.
- `webgpu_zeroize_sparse_ranges=2096`, failing the `<= 1704` cap.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.

## GREEN implementation

Changed `risc0/zkp/src/hal/webgpu.rs`:

- Changed sparse zeroize range records from four u32s to two u32s: `[src_start, dst_start]`.
- Updated `ZEROIZE_SPARSE_UPLOAD_ELEM_WGSL` binary-search stride from `4u` to `2u`.
- Updated range-count diagnostics and params from `ranges.len() / 4` to `ranges.len() / 2`.

The shader already derives the active range from `src_start` ordering and `params.value_count`, so `len` was not needed.

## Verification

Focused HAL GREEN:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=180 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_data_zeroize_sparse_uploads_only_nonzero_values -- --nocapture
```

Result: PASS, `test result: ok`, finished in `0.09s` after rebuild.

- `values=213`, `ranges=131`.
- `sparse_bytes=1916`, down from RED `2964`.
- Implied `webgpu_zeroize_sparse_ranges=1048`, down from RED `2096`.
- GPU buffer matched CPU after zeroize.

Representative BusyLoop + KeccakUnion browser proof:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: PASS, `test result: ok`, finished in `113.77s`.

BusyLoop po2=18:

- Receipt verified.
- `wall_ms=10600`, `gpu_active_ms=3989`, `gpu_idle_ratio=0.624`.
- `mask=0x0001`, `dispatched_arms=[0]`.
- `webgpu_zeroize_sparse_upload name=data values=17580007 ranges=3063676 sparse_bytes=94829452 dense_bytes=221249536`.
- Previous SP7bc same-path first BusyLoop sparse payload was about `119338860`, so this removes about `24.5 MB`.
- `upload_bytes=406305016`, down from SP7bc `430814424`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.

KeccakUnion(1):

- Receipt path verified.
- Representative shape retained: `segments=4`, `pending_keccaks=9`, `assumptions=1`.
- `wall_ms=102887`, `gpu_active_ms=69530`, `gpu_idle_ratio=0.324`.
- `upload_bytes=5440853300`, down from SP7bc `5688957092`.
- `source=data upload_bytes=3355443200`, down from SP7bc `3930062848`.
- `webgpu_zeroize_sparse_ranges=341161744`.
- `webgpu_zeroize_sparse_values=527532452`.
- `witgen_data_shadow_rows=67004128`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.

Representative xgboost browser proof:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: PASS, `test result: ok`, finished in `102.14s`.

xgboost:

- Receipt journal assertion passed: `30.528042544062632`.
- `wall_ms=101918`, `gpu_active_ms=56617`, `gpu_idle_ratio=0.444`.
- `segments=11`, `user_cycles=2294946`, `total_cycles=2883584`.
- `upload_bytes=6015038844`, down from SP7bc `6314431520`.
- `webgpu_zeroize_sparse_ranges=299394824`, down from SP7bc `598787472`.
- `webgpu_zeroize_sparse_values=801748256`, unchanged within measurement noise.
- `source=data upload_bytes=2818572288`, unchanged.
- `witgen_data_shadow_rows=407004560`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.

## Interpretation

Correctness:

- Focused HAL buffer-equivalence test passed.
- Representative e2e proof generation passed for BusyLoop po2=18, KeccakUnion(1), and xgboost.
- Receipt verification/journal assertions passed.
- No CPU fallback or CPU-only operations were introduced.

Performance:

- Deterministic upload-byte reduction: range metadata is halved.
- xgboost total upload bytes moved `6314431520 -> 6015038844` (`-299392676`, about `-4.7%` total upload).
- xgboost wall moved `101985 -> 101918` (`-67 ms`), which is noise-level and not a material wall-time gain.
- KeccakUnion total upload moved `5688957092 -> 5440853300` (`-248103792`), while wall moved `103241 -> 102887` (`-354 ms`), also not enough for a production wall claim.
- Accepted production wall-time gain remains `0`; accepted deterministic gain is lower upload pressure on the opt-in replacement path.

## Next target

The remaining high-value GPU-witgen blocker is not range metadata. It is still the large `source=data` upload / CPU-shadow dependency. Future work should either:

- make more of witgen GPU-resident so less CPU-produced witness data needs upload, or
- move downstream accumulation/commit consumption to use GPU-owned witness data without requiring dense CPU repair.
