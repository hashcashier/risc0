# SP7y: skip fri_fold destination upload

Date: 2026-05-19

## Change

`WebGpuHal::dispatch_fri_fold` no longer uploads the destination buffer before dispatch. The WGSL kernel writes all four extension-element lanes for every `idx < count` and never reads `output`, so the previous `output.sync_cpu_to_gpu` was dead data movement for newly allocated `out_coeffs` buffers.

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
  webgpu_fri_fold_skips_output_upload -- --nocapture
```

Result: failed after the focused GPU result matched expected CPU output.

Key failing diagnostic:

```text
source=out_coeffs uploads=1 upload_bytes=128
cpu_fallbacks=0 cpu_only_ops=0
```

This proved `fri_fold` was uploading the destination before a full overwrite.

## GREEN

Same command after implementation.

Result: passed. The small focused run negotiated low Chrome adapter limits (`1073741824` / `1073741824` / `32768`), so it is used only as correctness and counter-shape evidence. Representative performance evidence below uses high WebGPU limits.

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

Result: passed with high WebGPU limits (`4294967292` / `2147483644` / `49152`). This single browser e2e proves and verifies receipts for BusyLoop and `KeccakUnion(1)`.

BusyLoop:

```text
wall_ms=10614
gpu_active_ms=7420
gpu_idle_ratio=0.301
segments=1
queue_submits=174
readbacks=32
readback_bytes=478272
uploads=222
upload_bytes=515953708
cpu_fallbacks=0
cpu_only_ops=0
```

KeccakUnion(1):

```text
wall_ms=103666
gpu_active_ms=69509
gpu_idle_ratio=0.329
segments=4
pending_keccaks=9
assumptions=1
queue_submits=3177
readbacks=590
readback_bytes=10183944
uploads=3914
upload_bytes=6443685804
readback_sources:
  final_coeffs=38
  merkle_query=257
  nodes=257
  out=38
cpu_fallbacks=0
cpu_only_ops=0
```

Compared with SP7x KeccakUnion (`uploads=4019`, `upload_bytes=6451809708`), this removes 105 uploads and 8,123,904 upload bytes. The `out_coeffs` upload source is absent.

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

Result: passed with high WebGPU limits.

```text
wall_ms=101700
gpu_active_ms=60775
gpu_idle_ratio=0.402
segments=11
queue_submits=2774
readbacks=512
readback_bytes=7488592
uploads=3502
upload_bytes=7521486028
readback_sources:
  final_coeffs=32
  merkle_query=224
  nodes=224
  out=32
cpu_fallbacks=0
cpu_only_ops=0
```

Compared with SP7x xgboost (`uploads=3598`, `upload_bytes=7530431692`), this removes 96 uploads and 8,945,664 upload bytes. The `out_coeffs` upload source is absent.

## Decision

Accept as a production data-movement reduction. The representative proof matrix covers BusyLoop, `KeccakUnion(1)`, and xgboost with receipt verification and zero CPU fallback. Do not claim a material wall-time win from this single run; xgboost wall moved `101408 -> 101700` ms, so accepted wall-time reduction remains 0 s pending repeated A/B.
