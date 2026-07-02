# SP7ac - use copy commands for eltwise_copy_elem_slice

Date: 2026-05-19

## Change

Replaced the WebGPU `eltwise_copy_elem_slice` compute-kernel path with a
row-wise `copy_buffer_to_buffer` command encoder. The source slice is still
uploaded once as before, and the destination buffer is still synchronized first
when needed, but this removes the per-call compute pipeline/bind group/params
buffer and raw compute dispatch.

This path is used by recursion noise insertion for `data` and `accum`.

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

Result: failed after the GPU output matched CPU output because the old path
used one raw compute dispatch and uploaded a params buffer:

```text
raw_compute_dispatches=1
upload_sources:
  webgpu_eltwise_copy_elem_slice_from uploads=1 upload_bytes=160
  webgpu_eltwise_copy_elem_slice_params uploads=1 upload_bytes=32
```

## GREEN - focused HAL

Same command after the implementation: passed with high WebGPU limits. The
focused assertion proves the copy-slice output still matches CPU and that the
copy-slice path uses zero raw compute dispatches and no
`webgpu_eltwise_copy_elem_slice_params` upload.

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
wall_ms=10680
gpu_active_ms=7461
gpu_idle_ratio=0.301
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

Compared with SP7ab BusyLoop (`uploads=219`, `upload_bytes=515931420`),
this removes 2 uploads and 64 upload bytes. Queue submits are unchanged; the
copy-command path replaces compute dispatches with WebGPU copy commands.

KeccakUnion:

```text
wall_ms=103251
gpu_active_ms=69668
gpu_idle_ratio=0.325
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

Compared with SP7ab KeccakUnion (`uploads=3835`, `upload_bytes=6415692924`),
this removes 50 uploads and 1,600 upload bytes. Raw compute dispatches drop by
50 relative to the prior copy-slice compute path. Device-copy diagnostics
increase because the same slice movement is now represented as explicit
buffer-copy commands.

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
wall_ms=101442
gpu_active_ms=60902
gpu_idle_ratio=0.400
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

Compared with SP7ab xgboost (`uploads=3429`, `upload_bytes=7506769904`),
this removes 42 uploads, 1,344 upload bytes, and 42 raw compute dispatches.
Queue submits remain unchanged. Observed wall movement (`101593 -> 101442`
ms) is directional but too small/noisy to claim as a material wall-time win.

## Decision

Accept as a small production command/allocation cleanup. It simplifies
`eltwise_copy_elem_slice` and removes per-call compute pipeline/bind-group
work while preserving full representative receipt verification. Accepted
wall-time reduction remains 0 s pending repeated A/B.
