# SP7z: skip direct batch_evaluate_any destination upload

Date: 2026-05-19

## Change

The direct `WebGpuHal::dispatch_batch_evaluate_any` path no longer uploads the destination buffer before dispatch. `BATCH_EVALUATE_ANY_WGSL` writes all four extension-element lanes for every `eval_idx < count` through `store_output(eval_idx, total)` and never reads `out`, so the previous `out.sync_cpu_to_gpu` was dead data movement for fully overwritten output buffers.

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
  webgpu_batch_evaluate_any_skips_output_upload -- --nocapture
```

Result: failed after the focused GPU result matched expected CPU output.

Key failing diagnostic:

```text
source=out uploads=1 upload_bytes=80
webgpu_batch_evaluate_any_params uploads=1 upload_bytes=32
cpu_fallbacks=0 cpu_only_ops=0
```

This proved the direct `batch_evaluate_any` path was uploading the destination before a full overwrite.

## GREEN

Same command after implementation.

Result: passed with high WebGPU limits (`4294967292` / `2147483644` / `49152`). The `out` upload source is absent.

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

Result: passed with high WebGPU limits and receipt verification. This single browser e2e proves and verifies receipts for BusyLoop and `KeccakUnion(1)`.

BusyLoop:

```text
wall_ms=10684
gpu_active_ms=7459
gpu_idle_ratio=0.302
segments=1
queue_submits=174
readbacks=32
readback_bytes=478272
uploads=219
upload_bytes=515931420
cpu_fallbacks=0
cpu_only_ops=0
```

Compared with SP7y BusyLoop (`uploads=222`, `upload_bytes=515953708`), this removes 3 uploads and 22,288 upload bytes. The `out` upload source is absent.

KeccakUnion(1):

```text
wall_ms=104966
gpu_active_ms=69961
gpu_idle_ratio=0.333
segments=4
pending_keccaks=9
assumptions=1
queue_submits=3177
readbacks=590
readback_bytes=10183944
uploads=3872
upload_bytes=6442871132
readback_sources:
  final_coeffs=38
  merkle_query=257
  nodes=257
  out=38
cpu_fallbacks=0
cpu_only_ops=0
```

Compared with SP7y KeccakUnion (`uploads=3914`, `upload_bytes=6443685804`), this removes 42 uploads and 814,672 upload bytes. The `out` upload source is absent; `out` remains as an intentional coalesced readback source.

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

Result: passed with high WebGPU limits and receipt verification.

```text
wall_ms=101317
gpu_active_ms=60767
gpu_idle_ratio=0.400
segments=11
queue_submits=2774
readbacks=512
readback_bytes=7488592
uploads=3459
upload_bytes=7521144244
readback_sources:
  final_coeffs=32
  merkle_query=224
  nodes=224
  out=32
cpu_fallbacks=0
cpu_only_ops=0
```

Compared with SP7y xgboost (`uploads=3502`, `upload_bytes=7521486028`), this removes 43 uploads and 341,784 upload bytes. The `out` upload source is absent; `out` remains as an intentional coalesced readback source.

## Decision

Accept as a production data-movement reduction. The representative proof matrix covers BusyLoop, `KeccakUnion(1)`, and xgboost with receipt verification and zero CPU fallback. Do not claim a material wall-time win from this single run; observed wall movement remains noisy, so accepted wall-time reduction remains 0 s pending repeated A/B.
