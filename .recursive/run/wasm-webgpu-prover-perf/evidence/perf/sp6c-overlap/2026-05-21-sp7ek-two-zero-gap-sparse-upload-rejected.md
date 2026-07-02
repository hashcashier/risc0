# SP7ek Two-Zero-Gap Sparse Upload Extension Rejected

Date: 2026-05-21

## Objective

Test whether extending SP7ej sparse zeroize upload coalescing from one raw-zero cell to two raw-zero cells gives a worthwhile immediate performance win without weakening proof correctness.

## Candidate

The candidate changed `build_sparse_zero_upload_plan` to merge up to two raw-zero cells between nonzero/non-`INVALID` runs, while keeping `INVALID` and longer zero runs as hard sparse range boundaries.

Touched files during the candidate:

- `risc0/zkp/src/hal/webgpu.rs`
- `examples/browser-prove/src/lib.rs`

## Focused RED

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_core_gpu_results_match_cpu -- --nocapture
```

Log: `/tmp/sp7ek-zero-gap2-red.log`

Result: failed as expected before the production change. The accepted SP7ej single-gap planner emitted `values=5`, `ranges=3`, `sparse_bytes=60`, and the new assertion expected `16` range bytes instead of `24`.

## Focused GREEN

Same command after the candidate passed.

Log: `/tmp/sp7ek-zero-gap2-green.log`

Key log:

```text
browser-prove:metric webgpu_zeroize_sparse_upload name=data values=7 ranges=2 sparse_bytes=60 dense_bytes=512
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
```

## Representative E2E Proof Gates

### BusyLoop + KeccakUnion

Command:

```bash
env ... cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture
```

Log: `/tmp/sp7ek-zero-gap2-busy-keccak.log`

Result:

- high WebGPU limits: `4294967292 / 2147483644 / 49152`
- verified receipts
- test total: `97.92s`
- BusyLoop: `prove_session_async wall_ms=6374`, `gpu_active_ms=4044`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- KeccakUnion: `prove_session_async wall_ms=90833`, `gpu_active_ms=60077`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- KeccakUnion retained representative shape: `pending_keccaks=9`, `assumptions=1`

Compared with accepted SP7ej single-gap (`96.96s` total, BusyLoop `6090ms`, KeccakUnion `90151ms`), this trial was slower.

### xgboost

Command:

```bash
env ... cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture
```

Log: `/tmp/sp7ek-zero-gap2-xgboost.log`

Result:

- high WebGPU limits: `4294967292 / 2147483644 / 49152`
- `prove_session_async wall_ms=72535`
- total test: `73.06s`
- `gpu_active_ms=45529`
- `gpu_idle_ratio=0.372`
- `raw_compute_dispatches=9718`
- `queue_submits=2740`
- `upload_bytes=2674318020`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

Sparse upload movement versus accepted SP7ej single-gap xgboost:

- total upload bytes: `2674553136 -> 2674318020` (`-235116` bytes)
- `webgpu_zeroize_sparse_ranges`: `540031960 -> 500263080` (`-39768880` bytes)
- `webgpu_zeroize_sparse_values`: `1673685344 -> 1713206180` (`+39520836` bytes)
- `webgpu_zero_default_sparse_ranges`: `75594704 -> 75064944` (`-529760` bytes)
- `webgpu_zero_default_sparse_values`: `60793024 -> 61325624` (`+532600` bytes)

## Revert Validation

The two-gap behavior was removed and the focused browser HAL test was rerun.

Log: `/tmp/sp7ek-zero-gap2-revert-focused.log`

Key log:

```text
browser-prove:metric webgpu_zeroize_sparse_upload name=data values=31 ranges=1 sparse_bytes=148 dense_bytes=512
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
```

This returns the code to the SP7ej single-gap behavior, which already has representative BusyLoop+KeccakUnion and xgboost proof validation.

## Decision

Reject and revert the two-zero-gap extension.

The candidate was correctness-positive on the representative proof gates, but it does not meet the immediate significant performance bar:

- BusyLoop+KeccakUnion was slower in the candidate trial (`96.96s -> 97.92s`).
- xgboost wall movement was noise-level (`72834ms -> 72535ms`).
- xgboost total upload movement was only `-235116` bytes because the range-metadata reduction was almost entirely offset by explicit zero values.

Accepted wall-time gain: `0`.

Next work should return to larger buckets: FRI/check `batch_expand_into_evaluate_ntt`, queue/idle reduction, or a chunk-complete witness/accumulator design with a large CPU-owned surface.
