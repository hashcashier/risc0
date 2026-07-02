# SP7ag alloc elem init GPU fill rejected

Date: 2026-05-19

## Candidate

Initialize small nonzero `alloc_elem_init` WebGPU buffers with a GPU fill kernel
instead of uploading the host-filled shadow. The candidate capped GPU fills at
4 MiB and relied on WebGPU zero-initialization for zero-filled buffers.

## RED

Focused browser HAL coverage added
`webgpu_alloc_elem_init_fills_on_gpu_without_host_upload`, which allocated
`BabyBearElem::INVALID` and compared raw GPU contents with the CPU shadow before
checking that the allocation source had zero uploads.

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=180 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_alloc_elem_init_fills_on_gpu_without_host_upload -- --nocapture
```

Expected failure before implementation:

```text
alloc_elem_init_invalid: GPU/CPU mismatch
```

The GPU buffer still contained WebGPU zero-initialized contents while the CPU
shadow contained `0xffffffff`.

## Focused GREEN

The same focused browser test passed after adding the GPU fill path:

- high WebGPU limits:
  `max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152`
- `test tests::webgpu_alloc_elem_init_fills_on_gpu_without_host_upload ... ok`
- 1 passed, 126 filtered, finished 0.09s

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

Result: pass, receipt verification, zero CPU fallback/CPU-only.

BusyLoop:

| metric | SP7af | candidate |
|---|---:|---:|
| wall_ms | 10592 | 10624 |
| gpu_active_ms | 7374 | 7365 |
| gpu_idle_ratio | 0.304 | 0.307 |
| raw_compute_dispatches | 711 | 712 |
| queue_submits | 175 | 176 |
| uploads | 218 | 218 |
| upload_bytes | 505197364 | 504148804 |
| alloc_elem_init dispatches | 0 | 1 |
| cpu_fallbacks / cpu_only_ops | 0 / 0 | 0 / 0 |

KeccakUnion:

| metric | SP7af | candidate |
|---|---:|---:|
| wall_ms | 102563 | 103179 |
| gpu_active_ms | 69360 | 69126 |
| gpu_idle_ratio | 0.324 | 0.330 |
| segments | 4 | 4 |
| pending_keccaks | 9 | 9 |
| assumptions | 1 | 1 |
| raw_compute_dispatches | 12634 | 12647 |
| queue_submits | 3182 | 3195 |
| uploads | 3770 | 3761 |
| upload_bytes | 5915126028 | 5910276572 |
| alloc_elem_init dispatches | 0 | 13 |
| fill-param uploads | 0 | 13 |
| fill-param bytes | 0 | 208 |
| cpu_fallbacks / cpu_only_ops | 0 / 0 | 0 / 0 |

KeccakUnion deltas:

- upload bytes: -4849456
- raw dispatches: +13
- queue submits: +13
- observed wall: +616 ms in one browser trial

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

Result: pass, succinct receipt verification, zero CPU fallback/CPU-only.

xgboost:

| metric | SP7af | candidate |
|---|---:|---:|
| wall_ms | 101212 | 100935 |
| gpu_active_ms | 60769 | 60585 |
| gpu_idle_ratio | 0.400 | 0.400 |
| segments | 11 | 11 |
| raw_compute_dispatches | 11342 | 11353 |
| queue_submits | 2776 | 2787 |
| uploads | 3370 | 3370 |
| upload_bytes | 7037573868 | 7026039708 |
| data upload_bytes | 5252317184 | 5252317184 |
| accum upload_bytes | 1716518912 | 1716518912 |
| code upload_bytes | 11534336 | 0 |
| alloc_elem_init dispatches | 0 | 11 |
| fill-param uploads | 0 | 11 |
| fill-param bytes | 0 | 176 |
| readback_bytes | 7488592 | 7488592 |
| cpu_fallbacks / cpu_only_ops | 0 / 0 | 0 / 0 |

xgboost deltas:

- upload bytes: -11534160
- raw dispatches: +11
- queue submits: +11
- observed wall: -277 ms in one browser trial, noisy/directional only

## Post-revert current-tree verification

After reverting the candidate runtime/test code, reran the representative gate
on the retained tree.

BusyLoop + KeccakUnion command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

Result: pass, receipt verification, zero CPU fallback/CPU-only.

| workload | wall_ms | gpu_active_ms | raw_dispatches | queue_submits | uploads | upload_bytes | cpu_fallbacks / cpu_only_ops |
|---|---:|---:|---:|---:|---:|---:|---:|
| BusyLoop | 10714 | 7478 | 711 | 175 | 218 | 505197364 | 0 / 0 |
| KeccakUnion(1) | 102769 | 69459 | 12634 | 3182 | 3770 | 5915126028 | 0 / 0 |

KeccakUnion retained the representative shape guard:

- top-level proof `segments=4`
- pending keccak proofs `pending_keccaks=9`
- assumption resolution `assumptions=1`

xgboost command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies -- --nocapture
```

Result: pass, succinct receipt verification, zero CPU fallback/CPU-only.

| workload | wall_ms | gpu_active_ms | segments | raw_dispatches | queue_submits | uploads | upload_bytes | readback_bytes | cpu_fallbacks / cpu_only_ops |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| xgboost | 101832 | 60672 | 11 | 11342 | 2776 | 3370 | 7037573868 | 7488592 | 0 / 0 |

## Decision

Rejected and reverted.

The candidate was correctness-positive across the full representative matrix,
including KeccakUnion, but it traded a tiny upload-byte reduction for additional
raw dispatches and queue submits on a browser workload that is already
submission-sensitive. KeccakUnion caught the mixed behavior: wall time regressed
directionally while xgboost improved only within single-trial noise.

Accepted wall-time reduction: 0 s.

Rule retained for follow-up candidates: no accepted runtime change without
receipt-verified BusyLoop, KeccakUnion, and xgboost evidence with zero CPU
fallback/CPU-only ops. KeccakUnion remains required because it exercises pending
keccak proofs and assumption resolution, not just a single top-level RV32IM
segment.
