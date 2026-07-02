# SP7af recursion ctrl control-id cache

Date: 2026-05-19

## Change

Cache WebGPU recursion `ctrl` buffers produced by `copy_from_elem_transpose_zero_pad`
when the caller supplies the recursion program control ID. The cache key is:

- `control_id`
- compact row count
- column count
- padded total row count

The host recursion prover now threads the known control ID into the circuit
recursion prover, and `WitnessGenerator` passes it to the HAL. Generic HAL
implementations ignore the key and preserve the existing CPU construction path.

## RED

The focused browser HAL test was extended to call
`copy_from_elem_transpose_zero_pad(..., Some(Digest::new([0xace5; 8])))` twice
and assert that the second cached call has zero host-to-GPU uploads and zero raw
compute dispatches.

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_transpose_zero_pad_uploads_only_compact_rows --no-run
```

Expected failure before implementation:

```text
error[E0061]: this method takes 6 arguments but 7 arguments were supplied
```

## Focused GREEN

Compile:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_transpose_zero_pad_uploads_only_compact_rows --no-run
```

Result: pass, `Finished release profile`, executable produced.

Browser:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=180 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_transpose_zero_pad_uploads_only_compact_rows -- --nocapture
```

Result:

- high WebGPU limits:
  `max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152`
- `test tests::webgpu_hal_transpose_zero_pad_uploads_only_compact_rows ... ok`
- focused cached second call proves GPU buffer equality, zero host-to-GPU upload,
  and zero raw compute dispatch.

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

| metric | SP7ae | SP7af |
|---|---:|---:|
| wall_ms | 10641 | 10592 |
| gpu_active_ms | 7399 | 7374 |
| gpu_idle_ratio | 0.305 | 0.304 |
| segments | 1 | 1 |
| raw_compute_dispatches | 711 | 711 |
| queue_submits | 175 | 175 |
| uploads | 218 | 218 |
| upload_bytes | 505197364 | 505197364 |
| recursion_ctrl_compact upload_bytes | 13383240 | 13383240 |
| cpu_fallbacks / cpu_only_ops | 0 / 0 | 0 / 0 |

KeccakUnion:

| metric | SP7ae | SP7af |
|---|---:|---:|
| wall_ms | 103410 | 102563 |
| gpu_active_ms | 69317 | 69360 |
| gpu_idle_ratio | 0.330 | 0.324 |
| segments | 4 | 4 |
| pending_keccaks | 9 | 9 |
| assumptions | 1 | 1 |
| raw_compute_dispatches | 12654 | 12634 |
| queue_submits | 3202 | 3182 |
| uploads | 3810 | 3770 |
| upload_bytes | 6315322116 | 5915126028 |
| recursion_ctrl_compact upload_bytes | 502561592 | 102365824 |
| copy_from_elem_transpose_zero_pad dispatches | 25 | 5 |
| cpu_fallbacks / cpu_only_ops | 0 / 0 | 0 / 0 |

KeccakUnion deltas:

- `recursion_ctrl_compact`: -400195768 bytes
- total `upload_bytes`: -400196088 bytes
- `raw_compute_dispatches`: -20
- `queue_submits`: -20
- observed `wall_ms`: -847 ms single trial, noisy/directional only

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

| metric | SP7ae | SP7af |
|---|---:|---:|
| wall_ms | 100994 | 101212 |
| gpu_active_ms | 60388 | 60769 |
| gpu_idle_ratio | 0.402 | 0.400 |
| segments | 11 | 11 |
| raw_compute_dispatches | 11361 | 11342 |
| queue_submits | 2795 | 2776 |
| uploads | 3408 | 3370 |
| upload_bytes | 7386365368 | 7037573868 |
| recursion_ctrl_compact upload_bytes | 386058680 | 37267544 |
| copy_from_elem_transpose_zero_pad dispatches | 21 | 2 |
| readbacks | 512 | 512 |
| readback_bytes | 7488592 | 7488592 |
| cpu_fallbacks / cpu_only_ops | 0 / 0 | 0 / 0 |

xgboost deltas:

- `recursion_ctrl_compact`: -348791136 bytes
- total `upload_bytes`: -348791500 bytes
- `raw_compute_dispatches`: -19
- `queue_submits`: -19
- observed `wall_ms`: +218 ms single trial, noisy

## Decision

Accepted as a production data-movement and command-count reduction. Accepted
wall-time reduction remains 0 s pending repeated A/B because the single-trial
wall movement is mixed: KeccakUnion improved directionally while xgboost was
flat/slightly slower within expected browser noise.
