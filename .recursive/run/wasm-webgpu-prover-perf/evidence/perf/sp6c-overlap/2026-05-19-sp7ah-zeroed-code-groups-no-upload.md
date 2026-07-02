# SP7ah zeroed code groups no upload

Date: 2026-05-19

## Change

Treat zero-initialized WebGPU `Elem` allocations as already GPU-current,
matching WebGPU's guaranteed zero-filled buffer allocation and CpuHal's zeroed
shadow. RV32IM and Keccak code groups are now allocated as zeroed buffers and no
longer run `eltwise_zeroize_elem` on the all-zero code group.

This keeps the earlier rejected SP7ag lesson: do not add a fill dispatch for a
tiny upload reduction. This accepted shape removes uploads and zeroize
dispatches without adding new GPU work.

## RED: focused HAL

Added `webgpu_alloc_elem_init_zeroed_skips_host_zero_upload`, which allocates a
zeroed Elem buffer, copies it on-GPU, verifies GPU/CPU equality, and asserts the
zeroed source was not uploaded.

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=180 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_alloc_elem_init_zeroed_skips_host_zero_upload -- --nocapture
```

Expected failure before implementation:

- high WebGPU limits:
  `max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152`
- assertion failed with `webgpu_hal_alloc_elem_zeroed uploads=1`,
  `upload_bytes=4096`

## RED: representative e2e assertion

Added retained proof-gate assertions that representative diagnostics contain no
`code` uploads. Before implementation, the BusyLoop + KeccakUnion proof test
generated and verified receipts, then failed the new assertion on KeccakUnion:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

Expected failure before implementation:

- BusyLoop receipt verified, zero CPU fallback/CPU-only, `code uploads=1`,
  `code upload_bytes=1048576`
- KeccakUnion receipt verified, zero CPU fallback/CPU-only, then assertion failed
  with `code uploads=13`, `code upload_bytes=4259840`

## Focused GREEN

Same focused command after implementation:

- high WebGPU limits:
  `max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152`
- `test tests::webgpu_alloc_elem_init_zeroed_skips_host_zero_upload ... ok`
- 1 passed, 126 filtered, finished 0.10s

## Representative e2e: BusyLoop + KeccakUnion

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

Result: pass, receipt verification, zero CPU fallback/CPU-only, retained
`code` upload assertion passed.

BusyLoop:

| metric | SP7af | SP7ah |
|---|---:|---:|
| wall_ms | 10592 | 10635 |
| gpu_active_ms | 7374 | 7420 |
| gpu_idle_ratio | 0.304 | 0.302 |
| raw_compute_dispatches | 711 | 710 |
| queue_submits | 175 | 174 |
| uploads | 218 | 217 |
| upload_bytes | 505197364 | 504148788 |
| code uploads / bytes | 1 / 1048576 | 0 / 0 |
| eltwise_zeroize_elem dispatches | 8 | 7 |
| cpu_fallbacks / cpu_only_ops | 0 / 0 | 0 / 0 |

KeccakUnion:

| metric | SP7af | SP7ah |
|---|---:|---:|
| wall_ms | 102563 | 102226 |
| gpu_active_ms | 69360 | 69157 |
| gpu_idle_ratio | 0.324 | 0.323 |
| segments | 4 | 4 |
| pending_keccaks | 9 | 9 |
| assumptions | 1 | 1 |
| raw_compute_dispatches | 12634 | 12621 |
| queue_submits | 3182 | 3169 |
| uploads | 3770 | 3748 |
| upload_bytes | 5915126028 | 5910276364 |
| code uploads / bytes | 13 / 4259840 | 0 / 0 |
| accum upload_bytes | 1007747072 | 1007157248 |
| eltwise_zeroize_elem dispatches | 134 | 121 |
| cpu_fallbacks / cpu_only_ops | 0 / 0 | 0 / 0 |

KeccakUnion deltas:

- `code` uploads: -13
- `code` upload bytes: -4259840
- `accum` upload bytes: -589824
- total upload bytes: -4849664
- raw dispatches: -13
- queue submits: -13
- observed wall: -337 ms single trial, directional/noisy

## Representative e2e: xgboost

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

Result: pass, succinct receipt verification, zero CPU fallback/CPU-only, retained
`code` upload assertion passed.

xgboost:

| metric | SP7af | SP7ah |
|---|---:|---:|
| wall_ms | 101212 | 100335 |
| gpu_active_ms | 60769 | 60659 |
| gpu_idle_ratio | 0.400 | 0.395 |
| segments | 11 | 11 |
| raw_compute_dispatches | 11342 | 11331 |
| queue_submits | 2776 | 2765 |
| uploads | 3370 | 3359 |
| upload_bytes | 7037573868 | 7026039532 |
| code uploads / bytes | 11 / 11534336 | 0 / 0 |
| data upload_bytes | 5252317184 | 5252317184 |
| accum upload_bytes | 1716518912 | 1716518912 |
| recursion_ctrl_compact upload_bytes | 37267544 | 37267544 |
| readback_bytes | 7488592 | 7488592 |
| cpu_fallbacks / cpu_only_ops | 0 / 0 | 0 / 0 |

xgboost deltas:

- `code` uploads: -11
- `code` upload bytes: -11534336
- total upload bytes: -11534336
- raw dispatches: -11
- queue submits: -11
- observed wall: -877 ms single trial, directional/noisy

## Decision

Accepted as a production data-movement and command-count reduction. This is the
acceptable version of the SP7ag idea: it removes code uploads and zeroize
dispatches without adding any replacement dispatches.

Accepted wall-time reduction remains 0 s pending repeated A/B because the
single-trial wall movement is positive but still browser-noisy. The structural
win is clear: fewer raw dispatches, fewer queue submits, fewer uploads, and no
CPU fallback/CPU-only ops across BusyLoop, KeccakUnion, and xgboost.
