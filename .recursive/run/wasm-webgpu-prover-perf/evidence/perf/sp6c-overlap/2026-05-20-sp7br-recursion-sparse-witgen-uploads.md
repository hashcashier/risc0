Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7br - recursion sparse witgen uploads`
DraftedAt: `2026-05-20`
Status: `ACCEPTED`

## Summary

SP7br removes the remaining dense recursion `recursion_data` and `accum`
uploads from the GPU-witgen replacement path by sparse-priming the
destination before the recursion noise copy and by allowing recursion
`accum` zeroize to use the existing sparse zeroize upload path.

Correctness was validated by end-to-end browser proof generation on:

- `iter6d_g_replace_xgboost`
- `iter6d_g_replace_busy_loop_e2e_verify`, which covers BusyLoop and
  KeccakUnion in one fresh Chrome session

All accepted runs had `cpu_fallbacks=0`, `cpu_only_ops=0`, high WebGPU
limits, real receipt verification, and the xgboost journal value
`30.528042544062632`.

## Changes

- `risc0/zkp/src/hal/webgpu.rs`
  - `dispatch_eltwise_copy_elem_slice` now sparse-primes destination
    buffers named `recursion_data` or `accum` before recursion noise copy.
  - `try_sparse_zeroize_upload` now allows buffer name `accum` in addition
    to `data`, `recursion_data`, and `keccak_data`.
  - Existing safety gates remain: the sparse path refuses stale CPU shadows,
    non-dirty CPU shadows, sub-buffer views, non-sparse data, and oversized
    sparse uploads.
- `examples/browser-prove/src/lib.rs`
  - xgboost requires dense `source=accum <= 100000000` and dense
    `source=recursion_data <= 100000000`.
  - BusyLoop and KeccakUnion now also require dense `source=accum <=
    100000000` and dense `source=recursion_data <= 100000000`.

## TDD Evidence

### RED 1 - Dense `recursion_data`

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
    --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Expected failure after proof generation:

```text
xgboost_witgen_replace: WebGPU upload source `recursion_data` exceeded bound:
upload_bytes=2818572288 max_bytes=100000000
```

### GREEN 1 - Sparse-prime `recursion_data`

Compile:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
```

Result: PASS in `4m52s`.

xgboost e2e result after removing stale source-presence assertions:

```text
wall_ms=94841
gpu_active_ms=56551
gpu_idle_ratio=0.404
segments=11
cpu_fallbacks=0
cpu_only_ops=0
upload_bytes=3535161476
readback_bytes=7488592
source=recursion_data absent
source=accum upload_bytes=528482304
```

### RED 2 - Dense `accum`

Command: same xgboost e2e invocation above with dense `source=accum <=
100000000`.

Expected failure after proof generation:

```text
xgboost_witgen_replace: WebGPU upload source `accum` exceeded bound:
upload_bytes=528482304 max_bytes=100000000
```

### GREEN 2 - Sparse-prime and sparse-zeroize recursion `accum`

Compile:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
```

Result: PASS in `4m50s`.

xgboost e2e:

```text
wall_ms=94726
gpu_active_ms=56364
gpu_idle_ratio=0.405
segments=11
cpu_fallbacks=0
cpu_only_ops=0
upload_bytes=3193502120
readback_bytes=7488592
source=accum absent
source=recursion_data absent
webgpu_zero_default_sparse_values=287954624
webgpu_zero_default_sparse_ranges=160843856
webgpu_zeroize_sparse_values=1710456096
webgpu_zeroize_sparse_ranges=655448688
```

Representative BusyLoop + KeccakUnion e2e after adding the same dense
`accum` and `recursion_data` caps to both workloads:

```text
BusyLoop:
wall_ms=8805
gpu_active_ms=4305
gpu_idle_ratio=0.511
segments=1
cpu_fallbacks=0
cpu_only_ops=0
upload_bytes=213564800
readback_bytes=478272
dense source=accum <= 100000000 assertion passed
dense source=recursion_data <= 100000000 assertion passed

KeccakUnion:
wall_ms=101902
gpu_active_ms=68961
gpu_idle_ratio=0.323
segments=4
pending_keccaks=9
assumptions=1
cpu_fallbacks=0
cpu_only_ops=0
upload_bytes=3007508812
readback_bytes=10183944
dense source=accum <= 100000000 assertion passed
dense source=recursion_data <= 100000000 assertion passed
```

## Performance Delta

Compared with accepted SP7bq xgboost:

```text
wall_ms:      95324 -> 94726  (-598 ms, -0.6%, single browser trial)
upload_bytes: 5150602892 -> 3193502120  (-1957100772 bytes, -38.0%)
readback:     7488592 -> 7488592  (unchanged)
```

Compared with the intermediate sparse-`recursion_data` xgboost run:

```text
wall_ms:      94841 -> 94726  (-115 ms, noise-level)
upload_bytes: 3535161476 -> 3193502120  (-341659356 bytes, -9.7%)
source=accum: 528482304 -> 0 dense upload bytes
```

The wall-time movement is small and browser-noisy. The accepted win is a
large deterministic data-movement reduction with unchanged correctness and
zero CPU fallback.

## Commands

Successful compile gates:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run

cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Successful browser e2e gates:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
    --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture

env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
    --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Hygiene:

```bash
cargo fmt --manifest-path examples/browser-prove/Cargo.toml
git diff --check
```

`git diff --check` passed.

## Next

The next dominant xgboost upload sources are no longer dense
`recursion_data` or dense `accum`; they are the sparse value/range streams
themselves, especially `webgpu_zeroize_sparse_values` and
`webgpu_zeroize_sparse_ranges`. Further work should target reducing those
packed sparse payloads or eliminating the CPU-side sparse upload requirement
instead of pursuing iframe/multi-device side paths.
