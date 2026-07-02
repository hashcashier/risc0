# SP7aa: cache eval-check interpreter instruction buffers

Date: 2026-05-19

## Change

`WebGpuHal` now caches eval-check interpreter instruction buffers for the lifetime of a HAL. The cache key is exact instruction words plus base-field mode, not a hash-only key, so different programs cannot alias through a digest collision. Repeated segment/lift eval-check dispatches bind the cached GPU buffer instead of allocating and uploading the same immutable interpreter program again.

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
  webgpu_eval_check_reuses_instruction_upload -- --nocapture
```

Result: failed after two recursion eval-check interpreter dispatches matched the portable output.

Key failing diagnostic:

```text
source=webgpu_eval_check_base_interpreter_instructions uploads=2 upload_bytes=790976
cpu_fallbacks=0 cpu_only_ops=0
```

This proved the same immutable recursion eval-check interpreter program was uploaded once per dispatch.

## GREEN

Same command after implementation.

Result: passed with high WebGPU limits (`4294967292` / `2147483644` / `49152`). The test dispatches the same recursion eval-check twice, verifies both outputs against the portable implementation, and asserts `webgpu_eval_check_base_interpreter_instructions uploads == 1`.

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
wall_ms=10585
gpu_active_ms=7387
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

BusyLoop has one RV32IM segment and one recursion lift, so it does not repeat either interpreter program within the workload. No data-movement reduction is expected there.

KeccakUnion(1):

```text
wall_ms=103054
gpu_active_ms=69417
gpu_idle_ratio=0.326
segments=4
pending_keccaks=9
assumptions=1
queue_submits=3177
readbacks=590
readback_bytes=10183944
uploads=3835
upload_bytes=6415692924
readback_sources:
  final_coeffs=38
  merkle_query=257
  nodes=257
  out=38
cpu_fallbacks=0
cpu_only_ops=0
```

Compared with SP7z KeccakUnion (`uploads=3872`, `upload_bytes=6442871132`), this removes 37 uploads and 27,178,208 upload bytes. In the combined test, BusyLoop runs first and warms the same HAL cache, so KeccakUnion reports only one base-interpreter instruction upload and no repeated RV32IM interpreter instruction upload source.

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
wall_ms=101522
gpu_active_ms=60809
gpu_idle_ratio=0.401
segments=11
queue_submits=2774
readbacks=512
readback_bytes=7488592
uploads=3429
upload_bytes=7506769928
readback_sources:
  final_coeffs=32
  merkle_query=224
  nodes=224
  out=32
cpu_fallbacks=0
cpu_only_ops=0
```

Interpreter instruction upload sources:

```text
webgpu_eval_check_base_interpreter_instructions uploads=1 upload_bytes=395488
webgpu_eval_check_interpreter_instructions uploads=1 upload_bytes=646464
```

Compared with SP7z xgboost (`uploads=3459`, `upload_bytes=7521144244`), this removes 30 uploads and 14,374,316 total upload bytes. The exact instruction-source deltas are:

```text
base interpreter instructions: 21 uploads / 8305248 bytes -> 1 upload / 395488 bytes
rv32im interpreter instructions: 11 uploads / 7111104 bytes -> 1 upload / 646464 bytes
```

## Decision

Accept as a production data-movement and allocation-pressure reduction. The representative proof matrix covers BusyLoop, `KeccakUnion(1)`, and xgboost with receipt verification and zero CPU fallback. Do not claim a material wall-time win from this single run; xgboost wall moved `101317 -> 101522` ms, so accepted wall-time reduction remains 0 s pending repeated A/B.
