# SP7bp - Sparse Zero-Default Accum Upload

## Goal

Reduce the dense RV32IM `source=accum` upload that happens before the WebGPU machine-column carry. SP7bo proved a direct MISC0 accumulator path, but xgboost still uploaded `1716518912` bytes from `source=accum`.

## RED

Added an e2e xgboost assertion bounding `source=accum` upload bytes to `1300000000`.

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: expected failure after proof generation.

- High WebGPU limits active.
- Receipt path completed far enough to emit full diagnostics.
- `source=accum uploads=119 upload_bytes=1716518912`.
- Assertion failed: `upload_bytes=1716518912 max_bytes=1300000000`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.

## Change

Added `WebGpuBuffer<BabyBearElem>::sync_cpu_to_gpu_zero_default_sparse_named(...)`.

Behavior:

- Scans the CPU shadow and packs only values that are neither zero nor `INVALID`.
- Leaves other GPU cells at WebGPU's zero-initialized default.
- Uses the existing sparse copy WGSL shape with separate value/range upload sources.
- Marks the CPU shadow as no longer dirty only after the sparse upload dispatch is queued.

The only production call site is RV32IM `dispatch_accum_machine_column_carry` for the `accum` buffer. This matches the accumulator's unchecked CPU semantics, where `INVALID` is consumed as zero before carry/commit.

## Compile

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
```

Result: PASS in `4m49s` with existing warnings only.

Post-format:

```bash
cargo fmt --manifest-path examples/browser-prove/Cargo.toml
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
git diff --check
```

Result: PASS in `2m17s`; `git diff --check` clean.

## GREEN - xgboost

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: PASS.

- High limits: `max_buffer_size=4294967292`, `max_storage_buffer_binding_size=2147483644`, `max_compute_workgroup_storage_size=49152`.
- `wall_ms=97684.0`.
- `gpu_active_ms=56627.0`.
- `gpu_idle_ratio=0.420`.
- `segments=11`.
- `user_cycles=2294867`.
- `total_cycles=2883584`.
- `gpu_dispatches=5260`.
- `raw_compute_dispatches=11391`.
- `queue_submits=2676`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.
- total `upload_bytes=5152115064`.
- total `readback_bytes=302380644`.
- `source=accum uploads=42 upload_bytes=528482304`.
- `source=webgpu_zero_default_sparse_values uploads=11 upload_bytes=418355904`.
- `source=webgpu_zero_default_sparse_ranges uploads=11 upload_bytes=31918832`.
- `source=recursion_data uploads=168 upload_bytes=2818572288`.
- `source=witgen_accum_shadow_rows readbacks=11 readback_bytes=294892052`.

Delta vs SP7bo xgboost:

- `source=accum`: `1716518912 -> 528482304` (`-1188036608`, about `-69.2%` for that source).
- total upload bytes: `5889877296 -> 5152115064` (`-737762232`, about `-12.5%`).
- wall: `98463.0 -> 97684.0` (`-779 ms`, single-run directional).

## GREEN - BusyLoop + KeccakUnion

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: PASS.

BusyLoop:

- `wall_ms=8734.0`.
- `gpu_active_ms=4026.0`.
- `gpu_idle_ratio=0.539`.
- `segments=1`.
- `user_cycles=202872`.
- `total_cycles=262144`.
- total `upload_bytes=326146312`.
- `source=accum uploads=2 upload_bytes=25165824`.
- `rv32im_witgen_accum_shadow_gpu_sync rows=68239`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.

KeccakUnion:

- `wall_ms=102927.0`.
- `gpu_active_ms=69622.0`.
- `gpu_idle_ratio=0.324`.
- `segments=4`.
- `pending_keccaks=9`.
- `assumptions=1`.
- total `upload_bytes=5162833428`.
- `source=accum uploads=50 upload_bytes=629145600`.
- `source=webgpu_zero_default_sparse_values uploads=4 upload_bytes=116570624`.
- `source=webgpu_zero_default_sparse_ranges uploads=4 upload_bytes=10483296`.
- `source=witgen_accum_shadow_rows readbacks=4 readback_bytes=48433004`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.

## Acceptance

Accepted as a deterministic data-movement reduction and a small directional wall improvement.

Do not overclaim the wall result from one run:

- xgboost observed wall gain vs SP7bo: `779 ms`.
- xgboost observed wall gain vs SP7bn latest pre-SP7bo accepted wall: `100003.0 -> 97684.0`, about `2319 ms`.
- KeccakUnion remained effectively flat.
- BusyLoop improved, but it is a single-segment short workload and more noise-sensitive.

Remaining dominant blockers:

- xgboost still reads back `294892052` bytes from `witgen_accum_shadow_rows`.
- xgboost still uploads `528482304` bytes from `source=accum` plus sparse value/range payloads.
- CPU `step_top_accum_direct_misc0` remains around `1.7s` per RV32IM segment.

Next work should continue toward GPU-resident accumulator consumption for the MISC0 replacement rows, not broader side paths.
