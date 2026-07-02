# SP7eo - Sparse upload staging-buffer reuse rejected

Date: 2026-05-21

## Candidate

Reuse HAL-local WebGPU staging buffers for sparse zeroize/default-zero upload
`values` and `ranges` buffers. The current path allocates fresh storage buffers
for every sparse upload, and SP7en showed sparse zeroize/default-zero upload as
the largest remaining data-movement source. The candidate intentionally did not
change sparse upload bytes or proof semantics; it only tried to reduce browser
buffer allocation churn on that hot path.

Touched during candidate, then reverted:

- `risc0/zkp/src/hal/webgpu.rs`
- `examples/browser-prove/src/lib.rs`

## Focused RED

Added a focused assertion to `webgpu_hal_core_gpu_results_match_cpu`:

- warm sparse zeroize once;
- reset diagnostics;
- run another sparse zeroize with the same shape;
- require `buffers_allocated <= 1`, allowing only the params UBO.

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
  cargo test --manifest-path Cargo.toml \
    --target wasm32-unknown-unknown --release \
    webgpu_hal_core_gpu_results_match_cpu -- --nocapture
```

Expected failure observed under high WebGPU limits:

```text
buffers_allocated: 3
bytes_allocated: 56
upload_sources:
  webgpu_zeroize_sparse_params  16 bytes
  webgpu_zeroize_sparse_ranges  24 bytes
  webgpu_zeroize_sparse_values  16 bytes
```

The failure proved the current warmed path still allocated fresh values/ranges
staging buffers.

## Focused GREEN

Implementation added per-HAL reusable `values` and `ranges` storage buffers
that grow to the largest requested sparse upload and are overwritten by later
`queue.writeBuffer` calls. Queue ordering preserves correctness: each later
write is ordered after earlier submitted dispatches that read the same scratch
buffer.

Same focused command passed:

```text
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 154 filtered out; finished in 0.17s
```

## Representative Gate

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
  cargo test --manifest-path Cargo.toml \
    --target wasm32-unknown-unknown --release \
    rv32im_default_representative_e2e_verify -- --nocapture
```

Result:

- high WebGPU limits negotiated;
- BusyLoop receipt verified;
- KeccakUnion receipt verified;
- CONTROL0/MISC/MEM direct accumulator assertions remained active;
- no proof failure;
- total test time: `97.28s`.

Comparison:

```text
SP7em accepted default BusyLoop+KeccakUnion: 95.73s
SP7eo candidate BusyLoop+KeccakUnion:        97.28s
Movement:                                   +1.55s / +1.6%
```

Because this is a shared-path allocation-churn candidate with no semantic byte
reduction, the wall-negative BusyLoop+KeccakUnion gate is enough to reject it
without spending another xgboost proof run.

## Revert Verification

Candidate code and test assertions were removed.

Marker sweep:

```bash
rg -n "sparse_upload_values_scratch|sparse_upload_ranges_scratch|ReusableStorageBuffer|warmed sparse zeroize|sparse_upload_reuse" \
  risc0/zkp/src/hal/webgpu.rs examples/browser-prove/src/lib.rs
```

Result: no matches.

Post-revert checks:

```text
cargo fmt --check --manifest-path risc0/zkp/Cargo.toml
cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml
git diff --check
```

All passed.

Post-revert focused browser log:

- `/tmp/sp7eo-sparse-staging-post-revert-focused.log`

Key lines:

```text
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
browser-prove:metric webgpu_zeroize_sparse_upload name=data values=4 ranges=3 sparse_bytes=56 dense_bytes=16384
browser-prove:metric webgpu_zeroize_sparse_upload name=data values=31 ranges=1 sparse_bytes=148 dense_bytes=512
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 154 filtered out; finished in 0.14s
```

## Decision

Rejected and reverted.

This candidate reduced a real allocation mechanism in the focused gate, but did
not improve representative wall time. Do not pursue sparse staging-buffer reuse
as an immediate performance lever unless a future browser profile shows buffer
allocation churn, not upload volume or NTT/FRI work, as the dominant wall-time
source.

Accepted wall-time gain: `0`.

