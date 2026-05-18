# SP6f compute-pipeline cache

Date: 2026-05-18

Purpose: reduce repeated browser compute-pipeline construction after SP6e
showed layout caching is already effective and xgboost still performs thousands
of object creations. Unlike bind groups, compute pipelines do not retain
per-proof buffers, so caching them is safe when their bind-group layout
identity is known to the same HAL.

## RED

Test-first change: extend
`webgpu_hal_reports_layout_and_bind_group_diagnostics` to create the same
compute kernel twice and assert:

```text
compute_pipeline_creations == 1
compute_pipeline_cache_hits == 1
```

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_reports_layout_and_bind_group_diagnostics --no-run
```

Expected failure:

```text
error[E0609]: no field `compute_pipeline_creations` on type `WebGpuDiagnostics`
error[E0609]: no field `compute_pipeline_cache_hits` on type `WebGpuDiagnostics`
```

## GREEN implementation

- Added `compute_pipeline_creations` and `compute_pipeline_cache_hits` to
  `WebGpuDiagnostics`, reset/snapshot accounting, pool aggregation, and browser
  diagnostic logs.
- Added a per-HAL `js_sys::WeakMap` from HAL-created `GpuBindGroupLayout`
  objects to their structural layout cache key.
- Added a per-HAL compute-kernel cache keyed by static kernel label, WGSL
  content, entry point, and layout keys. The cache is used only when every
  supplied layout was created by the same HAL; otherwise the existing uncached
  creation path remains in effect.
- Applied the cache to both `create_compute_kernel` and
  `create_compute_kernel_async`.

## Verification

Focused no-run compile:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_reports_layout_and_bind_group_diagnostics --no-run
```

Result:

```text
Finished `release` profile [optimized + debuginfo] target(s) in 4m 38s
```

Focused browser test:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_reports_layout_and_bind_group_diagnostics
```

Result:

```text
test tests::webgpu_hal_reports_layout_and_bind_group_diagnostics ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 109 filtered out; finished in 0.16s
```

Full xgboost pooled smoke with diagnostics:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=600 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_pool_xgboost_smoke -- --nocapture
```

Result:

```text
browser-prove:metric pool_prove_scheduled_async receipt_kind=Succinct wall_ms=103346 segments=11 keccaks=0 pool_size=2
browser-prove:metric pool_xgboost_smoke wall_ms=103352
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5365 cpu_mirrors=181 cpu_fallbacks=11 cpu_only_ops=0 uploads=14425 upload_bytes=15273567236 device_copies=128 device_copy_bytes=7222624256 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=35 bind_group_layout_cache_hits=1593 bind_group_creations=12510 compute_pipeline_creations=36 compute_pipeline_cache_hits=1592 buffers=15323 buffer_bytes=56464671708
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 109 filtered out; finished in 103.51s
```

## Interpretation

- The cache is functionally effective: xgboost records 36 compute-pipeline
  creations and 1,592 cache hits.
- The proof still verifies on the default scheduled pool path, with
  `cpu_only_ops=0`.
- Wall time is effectively flat/slightly noisy relative to SP6d iter 12
  (102.82s) and the SP6e diagnostic run (102.859s). Chrome/Dawn either already
  deduplicates most synchronous pipeline creation cost internally, or the
  remaining xgboost wall is dominated elsewhere.
- The bind-group count remains high at 12,510. A bind-group cache is still a
  plausible object-churn target, but it needs explicit buffer lifetime and
  invalidation semantics before it is safe.
