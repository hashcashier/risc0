# SP7x: zeroed ExtElem allocations skip host zero upload

Date: 2026-05-19

## Change

`WebGpuHal::alloc_extelem_zeroed` now marks the shadowed WebGPU buffer as synchronized immediately after allocation. The CPU shadow comes from `CpuHal::alloc_extelem_zeroed`, and WebGPU resource initialization is specified as zero-initialized, so first GPU use does not need a host zero upload.

Spec basis: <https://www.w3.org/TR/webgpu/#uninitialized-data>

Changed files:
- `risc0/zkp/src/hal/webgpu.rs`
- `examples/browser-prove/src/lib.rs`

## RED

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_alloc_extelem_zeroed_skips_host_zero_upload -- --nocapture
```

Result: failed after the focused GPU result matched the CPU expected output.

Key failing diagnostic:

```text
source=combos uploads=1 upload_bytes=768
cpu_fallbacks=0 cpu_only_ops=0
```

This proved the current path was still uploading host zeros for a zeroed ExtElem destination before `combos_prepare`.

## GREEN

Same command after implementation.

Result: passed in Chrome with high WebGPU limits:

```text
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152
test tests::webgpu_alloc_extelem_zeroed_skips_host_zero_upload ... ok
```

The test checks both output correctness and `combos_uploads == 0`.

## Representative e2e proof gate

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

Result: passed. This single browser e2e proves and verifies receipts for BusyLoop and `KeccakUnion(1)`.

BusyLoop:

```text
wall_ms=10750
gpu_active_ms=7479
gpu_idle_ratio=0.304
segments=1
queue_submits=174
readbacks=32
readback_bytes=478272
cpu_fallbacks=0
cpu_only_ops=0
```

KeccakUnion(1):

```text
wall_ms=104023
gpu_active_ms=69433
gpu_idle_ratio=0.333
segments=4
pending_keccaks=9
assumptions=1
queue_submits=3177
readbacks=590
readback_bytes=10183944
uploads=4019
upload_bytes=6451809708
readback_sources:
  final_coeffs=38
  merkle_query=257
  nodes=257
  out=38
cpu_fallbacks=0
cpu_only_ops=0
```

The KeccakUnion upload-source list did not include `combos`.

## xgboost e2e proof gate

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies -- --nocapture
```

Result: passed.

```text
wall_ms=101408
gpu_active_ms=60896
gpu_idle_ratio=0.399
segments=11
queue_submits=2774
readbacks=512
readback_bytes=7488592
uploads=3598
upload_bytes=7530431692
readback_sources:
  final_coeffs=32
  merkle_query=224
  nodes=224
  out=32
cpu_fallbacks=0
cpu_only_ops=0
```

Compared with SP7w xgboost (`uploads=3662`, `upload_bytes=8289600716`), this removes 64 uploads and 759,169,024 upload bytes. The `combos` upload source is absent from the xgboost upload-source list.

## Decision

Accept as a production data-movement reduction. The representative proof matrix now covers BusyLoop, `KeccakUnion(1)`, and xgboost with receipt verification and zero CPU fallback. Do not claim a material wall-time win from this single run; accepted wall-time reduction remains 0 s pending repeated A/B.
