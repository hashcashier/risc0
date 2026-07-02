# SP7ej Zero-Gap Sparse Upload Coalescing

Date: 2026-05-21

## Objective

Reduce the large sparse zeroize upload metadata bucket without repeating the rejected SP7eg behavior that wrote explicit zeros across `INVALID` gaps and broke representative proof verification.

## Change

`build_sparse_zero_upload_plan` now coalesces only a single raw-zero cell between two nonzero/non-`INVALID` runs. `INVALID` cells remain hard range boundaries, and trailing zeros before `INVALID` or end-of-buffer are not uploaded.

Touched files:

- `risc0/zkp/src/hal/webgpu.rs`
- `examples/browser-prove/src/lib.rs`

## Focused RED

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_core_gpu_results_match_cpu -- --nocapture
```

Result: failed before production code change on the new sparse range assertion.

Key log:

```text
browser-prove:metric webgpu_zeroize_sparse_upload name=data values=16 ranges=16 sparse_bytes=208 dense_bytes=512
assertion `left == right` failed: single-cell zero gaps should coalesce into one sparse zeroize range while INVALID gaps remain boundaries
left: 128
right: 8
```

## Focused GREEN

Same command after the planner change passed.

Key log:

```text
browser-prove:metric webgpu_zeroize_sparse_upload name=data values=31 ranges=1 sparse_bytes=148 dense_bytes=512
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
```

This verifies the focused CPU/GPU parity path and the intended exact behavior: one range, explicit zero fillers inside the merged run, no trailing zero before `INVALID`.

## Representative E2E Proof Gates

### BusyLoop + KeccakUnion

Command:

```bash
env ... cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture
```

Log: `/tmp/sp7ej-zero-gap-busy-keccak.log`

Result:

- high WebGPU limits: `4294967292 / 2147483644 / 49152`
- verified receipts
- test total: `96.96s`
- BusyLoop: `prove_session_async wall_ms=6090`, `gpu_active_ms=3835`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- KeccakUnion: `prove_session_async wall_ms=90151`, `gpu_active_ms=59681`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- KeccakUnion retained representative shape and proof path: `pending_keccaks=9`, `assumptions=1`

### xgboost

Command:

```bash
env ... cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture
```

Log: `/tmp/sp7ej-zero-gap-xgboost.log`

Result:

- high WebGPU limits: `4294967292 / 2147483644 / 49152`
- verified journal: `30.528042544062632`
- `prove_session_async wall_ms=72834`
- total test: `73.38s`
- `gpu_active_ms=46156`
- `gpu_idle_ratio=0.366`
- `raw_compute_dispatches=9718`
- `queue_submits=2740`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

Sparse upload movement versus SP7eh captured xgboost:

- total upload bytes: `2724587656 -> 2674553136` (`-50034520` bytes)
- `webgpu_zeroize_sparse_ranges`: `638976408 -> 540031960` (`-98944448` bytes)
- `webgpu_zeroize_sparse_values`: `1623965848 -> 1673685344` (`+49719496` bytes)
- `webgpu_zero_default_sparse_ranges`: `77223216 -> 75594704` (`-1628512` bytes)
- `webgpu_zero_default_sparse_values`: `59974080 -> 60793024` (`+818944` bytes)

## Decision

Accept the change as a correctness-proven sparse upload simplification.

Accepted wall-time gain for planning: `0`. The representative wall movement is small/noise-level: BusyLoop + KeccakUnion improved versus SP7eh total (`97.73s -> 96.96s`), and xgboost improved versus the captured SP7eh rerun (`73307ms -> 72834ms` prove wall), but this is not a significant improvement and should not distract from the larger remaining buckets.

Next material targets remain FRI/check `batch_expand_into_evaluate_ntt` or a chunk-complete witness/accumulator design with a larger CPU-owned surface.
