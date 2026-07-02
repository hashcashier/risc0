# SP7ae - build recursion ctrl on GPU from compact rows

Date: 2026-05-19

## Change

`recursion::WitnessGenerator` no longer builds the padded column-major `ctrl`
group by uploading the full padded buffer on WebGPU. It now calls
`Hal::copy_from_elem_transpose_zero_pad`. The generic fallback preserves the
old CPU behavior; the WebGPU override:

- builds the same CPU shadow for synchronous correctness;
- uploads only compact row-major program code as `recursion_ctrl_compact`;
- dispatches one small transpose/zero-pad kernel into the destination `ctrl`
  GPU buffer;
- relies on WebGPU zero initialization for padded rows.

This targets the recurring `ctrl` upload source seen in recursion lift/join
proofs.

## RED

Focused browser HAL coverage was added first:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_transpose_zero_pad_uploads_only_compact_rows --no-run
```

Result: failed at compile time because the new HAL method did not exist:

```text
error[E0599]: no method named `copy_from_elem_transpose_zero_pad` found for struct `WebGpuHal`
```

## GREEN - focused browser HAL

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=180 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_transpose_zero_pad_uploads_only_compact_rows -- --nocapture
```

Result: passed in Chrome with high WebGPU limits. The test proves the GPU
destination matches the CPU shadow, no full destination upload source is
recorded, only the compact source rows are uploaded, and the path uses one raw
compute dispatch.

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
fallback / zero CPU-only.

BusyLoop:

```text
wall_ms=10641
gpu_active_ms=7399
gpu_idle_ratio=0.305
segments=1
queue_submits=175
raw_compute_dispatches=711
uploads=218
upload_bytes=505197364
recursion_ctrl_compact upload_bytes=13383240
cpu_fallbacks=0
cpu_only_ops=0
```

KeccakUnion:

```text
wall_ms=103410
gpu_active_ms=69317
gpu_idle_ratio=0.330
segments=4
pending_keccaks=9
assumptions=1
queue_submits=3202
raw_compute_dispatches=12654
uploads=3810
upload_bytes=6315322116
recursion_ctrl_compact upload_bytes=502561592
cpu_fallbacks=0
cpu_only_ops=0
```

Compared with SP7ad:

```text
BusyLoop upload_bytes: 515931356 -> 505197364 (-10733992)
KeccakUnion upload_bytes: 6415691324 -> 6315322116 (-100369208)
KeccakUnion raw_compute_dispatches: 12629 -> 12654 (+25)
KeccakUnion queue_submits: 3177 -> 3202 (+25)
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
fallback / zero CPU-only.

```text
wall_ms=100994
gpu_active_ms=60388
gpu_idle_ratio=0.402
segments=11
queue_submits=2795
raw_compute_dispatches=11361
uploads=3408
upload_bytes=7386365368
recursion_ctrl_compact upload_bytes=386058680
readbacks=512
readback_bytes=7488592
cpu_fallbacks=0
cpu_only_ops=0
```

Compared with SP7ad xgboost:

```text
upload_bytes: 7506768560 -> 7386365368 (-120403192)
raw_compute_dispatches: 11340 -> 11361 (+21)
queue_submits: 2774 -> 2795 (+21)
wall_ms: 101393 -> 100994 (-399 ms, single trial/noisy)
```

## Hygiene

`git diff --check` passed. A narrow `rustfmt --edition 2021 --check` invocation
still reports the broad pre-existing formatting drift in `risc0/zkp/src/hal/webgpu.rs`,
`risc0/zkp/src/hal/webgpu/buffer_pool.rs`, and `risc0/zkp/src/hal/webgpu_codegen.rs`;
no repo-wide formatting pass was applied.

## Decision

Accept as a production data-movement cleanup. The current representative
programs have compact recursion code close to their padded `ctrl` shape, so the
byte reduction is useful but not large enough to claim a material wall-time
win. Accepted wall-time reduction remains 0 s pending repeated A/B.
