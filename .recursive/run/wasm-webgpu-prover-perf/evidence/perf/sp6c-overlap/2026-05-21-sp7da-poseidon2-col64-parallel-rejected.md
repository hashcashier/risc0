# SP7da - Poseidon2 cols=64 parallel row-hash rejected

Date: 2026-05-21
Status: rejected and reverted

## Candidate

SP7ch showed the large Merkle leaf-hash bucket is dominated by
`fri_prove round=0 merkle_new rows=65536 cols=64`. Several small Poseidon2
tuning attempts had already failed to move this bucket, so this candidate tried
a deeper WGSL specialization for exactly `cols=64`:

- one workgroup processed eight rows;
- each row used 32 lanes;
- Poseidon2 full rounds, partial rounds, and M-ext/M-int work were split across
  workgroup memory instead of keeping the existing one-thread-per-row shape.

The candidate was intentionally tested with a hidden focused hook first, then
with representative proof generation.

## RED

A focused browser-prove test was added for the missing hook:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_hash_rows_col64_parallel_matches_cpu --no-run
```

Expected failure:

```text
error[E0599]: no method named `debug_hash_rows_col64_parallel` found for struct `WebGpuHal`
```

This confirmed the test started red before production code was added.

## GREEN

After adding the specialized kernel and hook, the no-run compile passed:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_hash_rows_col64_parallel_matches_cpu --no-run
```

Result:

- passed;
- elapsed: `4m48s`.

The first focused Chrome run exposed a WGSL compile error because `active` is a
reserved WGSL word. The variable was renamed to `is_active`.

Focused Chrome validation then passed:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
    --target wasm32-unknown-unknown --release \
    webgpu_hal_hash_rows_col64_parallel_matches_cpu -- --nocapture
```

Result:

- passed;
- high WebGPU limits;
- `1 passed`;
- finished in `0.16s`.

## Representative e2e: BusyLoop and KeccakUnion

Command output: `/tmp/sp7da-busy-keccak-col64-parallel.log`

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
    --target wasm32-unknown-unknown --release \
    rv32im_default_representative_e2e_verify -- --nocapture
```

Result:

- `test tests::rv32im_default_representative_e2e_verify ... ok`
- `test result: ok. 1 passed; 0 failed; 138 filtered out; finished in 100.08s`
- BusyLoop and KeccakUnion proof generation and verification succeeded.
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

BusyLoop:

- `wall_ms=7270`
- `gpu_active_ms=3219`
- `gpu_idle_ratio=0.557`
- `raw_compute_dispatches=609`
- `queue_submits=173`
- `upload_bytes=219455316`
- `readback_bytes=478272`

KeccakUnion:

- `wall_ms=92516`
- `gpu_active_ms=59843`
- `gpu_idle_ratio=0.353`
- `raw_compute_dispatches=10660`
- `queue_submits=3075`
- `upload_bytes=2996370184`
- `readback_bytes=10183944`

Hot Merkle bucket:

- `fri_prove round=0 merkle_new rows=65536 cols=64`
- 30 samples across BusyLoop+KeccakUnion;
- sum: `25686 ms`;
- mean: `856.2 ms`.

## Representative e2e: xgboost

Command output: `/tmp/sp7da-xgboost-col64-parallel.log`

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
    --target wasm32-unknown-unknown --release \
    xgboost_succinct_receipt_verifies -- --nocapture
```

Result:

- `test tests::xgboost_succinct_receipt_verifies ... ok`
- `test result: ok. 1 passed; 0 failed; 138 filtered out; finished in 78.03s`
- `wall_ms=77790`
- `segments=11`
- `gpu_active_ms=44975`
- `gpu_idle_ratio=0.422`
- `raw_compute_dispatches=9674`
- `queue_submits=2718`
- `upload_bytes=3113980108`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

Hot Merkle bucket:

- `fri_prove round=0 merkle_new rows=65536 cols=64`
- 32 samples;
- sum: `27507 ms`;
- mean: `859.6 ms`.

## Comparison

Against latest accepted default SP7cy:

- BusyLoop: `7507 -> 7270 ms`, apparently faster but below the target workload
  noise floor and not enough to accept with the larger-workload regression.
- KeccakUnion: `91537 -> 92516 ms`, `+979 ms`.
- xgboost: `78466 -> 77790 ms`, `-676 ms`, but the focused accepted SP7cq
  estimate remains about `77470 ms`.

Against the measured hot Merkle bucket:

- BusyLoop+KeccakUnion hot samples stayed in the same band as prior rejected
  Poseidon2 attempts: about `856 ms` per large `cols=64` leaf-hash build.
- xgboost hot samples were `27507 ms` total, matching the prior SP7ch/SP7cc band
  of roughly `27360-27549 ms`.

## Decision

Rejected and reverted.

Correctness was clean, including the focused hash-row CPU comparison and all
representative proof/verifier gates. The change still did not improve the
targeted large Merkle bucket, regressed KeccakUnion relative to SP7cy, and did
not beat the accepted focused xgboost state. Keeping it would add substantial
specialized shader complexity for no reliable wall-time gain.

Follow-up: do not spend more immediate work on `cols=64` row-hash thread-level
parallelization in this shape. A future Merkle attempt needs either a different
leaf-hash algorithm shape, fewer synchronization drains around Merkle work, or
evidence from a shader-level microbenchmark before another representative e2e
run.
