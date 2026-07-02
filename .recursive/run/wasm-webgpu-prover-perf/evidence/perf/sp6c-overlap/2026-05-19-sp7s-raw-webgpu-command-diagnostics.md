Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7s -- raw WebGPU command diagnostics
Date: 2026-05-19

## Headline

`WebGpuDiagnostics::gpu_dispatches` is operation-level accounting, not raw
WebGPU command accounting. SP7s adds:

- `raw_compute_dispatches`: increments at every actual WGSL
  `dispatch_workgroups` call.
- `queue_submits`: increments at every WebGPU queue submit.

This does not reduce wall time by itself. It prevents bad decisions when an
optimization changes the number of internal dispatches but leaves logical HAL
operation counts unchanged.

## RED

Extended focused browser HAL coverage for the chunked `batch_evaluate_any`
path:

- logical `gpu_dispatches` should remain `1`;
- raw compute dispatches should be `2` for the current partial + reduce path;
- queue submits should cover all raw compute dispatch command buffers.

RED compile failed because the diagnostics API did not exist:

```text
error[E0609]: no field `raw_compute_dispatches` on type `WebGpuDiagnostics`
error[E0609]: no field `queue_submits` on type `WebGpuDiagnostics`
```

## GREEN

Implemented:

- `WebGpuDiagnostics.raw_compute_dispatches`
- `WebGpuDiagnostics.queue_submits`
- low-level increments at all `dispatch_workgroups` sites in `WebGpuHal`
- low-level increment in `WebGpuHal::submit`
- pool diagnostics aggregation for the new fields
- browser diagnostics logging for single-prover and pool paths

Focused HAL browser test:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_core_gpu_results_match_cpu -- --nocapture
```

Result:

```text
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 0.14s
```

## Representative E2E

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

Result:

```text
test tests::rv32im_accum_topaccum_arm5_authoritative_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 115.00s
```

BusyLoop:

```text
browser-prove:metric prove_session_async wall_ms=10706.0 gpu_active_ms=7501.0 gpu_idle_ratio=0.299
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: gpu_dispatches=328 raw_compute_dispatches=712 queue_submits=208 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
```

KeccakUnion(1):

```text
browser-prove:metric prove_session_async wall_ms=104059.0 gpu_active_ms=70805.0 gpu_idle_ratio=0.320
browser-prove:done multi_test/keccak_union_topaccum_arm5_authoritative: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_authoritative: gpu_dispatches=5904 raw_compute_dispatches=12679 queue_submits=3805 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0 uploads=4085 upload_bytes=7166152108 device_copies=38 device_copy_bytes=38400 readbacks=1218 readback_bytes=10183944
```

## xgboost E2E

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies -- --nocapture
```

Result:

```text
test tests::xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 102.37s
```

Key metrics:

```text
browser-prove:metric prove_session_async wall_ms=102145.0 gpu_active_ms=61726.0 gpu_idle_ratio=0.396
browser-prove:done xgboost_topaccum_arm5_authoritative: segments=11 user_cycles=2294867 total_cycles=2883584
browser-prove:webgpu xgboost_topaccum_arm5_authoritative: gpu_dispatches=5258 raw_compute_dispatches=11382 queue_submits=3318 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=3662 upload_bytes=8289600740 device_copies=32 device_copy_bytes=32768 readbacks=1056 readback_bytes=7488592
```

## Interpretation

The new command counters reveal why operation-level `gpu_dispatches` was too
coarse for optimization decisions:

```text
BusyLoop:       328 logical GPU ops ->   712 raw compute dispatches,  208 queue submits
KeccakUnion(1): 5904 logical GPU ops -> 12679 raw compute dispatches, 3805 queue submits
xgboost:        5258 logical GPU ops -> 11382 raw compute dispatches, 3318 queue submits
```

This makes command batching a measurable target. For example, the current
chunked `batch_evaluate_any` path is one logical HAL op but two raw compute
dispatches per chunked call (partial + reduce), and future TopAccum candidates
can now be screened with command counts plus the candidate sync gate.

## Decision

Accepted as production diagnostics because focused HAL coverage, BusyLoop +
`KeccakUnion(1)`, and xgboost all verify receipts with zero CPU fallbacks and
zero CPU-only ops.

Accepted wall-time reduction: `0 s`. This is decision-quality instrumentation
for the next command-batching or generated-arm step.
