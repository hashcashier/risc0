# SP7ad - skip gather_sample destination upload on full overwrite

Date: 2026-05-19

## Change

Skipped `gather_sample` destination synchronization when the gather writes the
entire logical destination buffer (`size == dst.size()`). Partial gathers still
synchronize the destination first so untouched elements remain preserved.

The same full-overwrite guard was applied to the chunked/tiled gather test
hooks.

## RED

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=180 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_core_gpu_results_match_cpu -- --nocapture
```

First sandboxed attempt failed to spawn the wasm-bindgen local browser server
(`Operation not permitted`). The same command was rerun with approval.

Result: failed after the GPU output matched CPU output because the old path
uploaded the fully overwritten destination:

```text
upload_sources:
  webgpu_gather_sample_params uploads=1 upload_bytes=32
  webgpu_hal_gather_dst uploads=1 upload_bytes=40
```

## GREEN - focused HAL

Same command after the implementation: passed with high WebGPU limits. The
focused assertion proves `gather_sample` still matches CPU output and no longer
uploads `webgpu_hal_gather_dst` when the gather fully overwrites the
destination.

## GREEN - BusyLoop + KeccakUnion proof gate

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

Result: passed with high WebGPU limits, receipt verification, and zero CPU
fallback.

BusyLoop:

```text
wall_ms=10916
gpu_active_ms=7650
gpu_idle_ratio=0.299
segments=1
queue_submits=174
raw_compute_dispatches=710
readbacks=32
readback_bytes=478272
uploads=217
upload_bytes=515931356
device_copies=4
device_copy_bytes=575488
cpu_fallbacks=0
cpu_only_ops=0
```

KeccakUnion:

```text
wall_ms=103014
gpu_active_ms=68948
gpu_idle_ratio=0.331
segments=4
pending_keccaks=9
assumptions=1
queue_submits=3177
raw_compute_dispatches=12629
readbacks=590
readback_bytes=10183944
uploads=3785
upload_bytes=6415691324
device_copies=88
device_copy_bytes=14374400
readback_sources:
  final_coeffs=38
  merkle_query=257
  nodes=257
  out=38
cpu_fallbacks=0
cpu_only_ops=0
```

## GREEN - xgboost proof gate

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies -- --nocapture
```

Result: passed with high WebGPU limits, receipt verification, and zero CPU
fallback.

```text
wall_ms=101393
gpu_active_ms=60983
gpu_idle_ratio=0.399
segments=11
queue_submits=2774
raw_compute_dispatches=11340
readbacks=512
readback_bytes=7488592
uploads=3387
upload_bytes=7506768560
device_copies=74
device_copy_bytes=12075008
readback_sources:
  final_coeffs=32
  merkle_query=224
  nodes=224
  out=32
cpu_fallbacks=0
cpu_only_ops=0
```

## Decision

Accept as a focused production data-flow cleanup. Current representative
workloads do not show a changed aggregate upload count because their hot gather
paths already avoid this direct full-destination upload shape. Accepted
wall-time reduction remains 0 s.
