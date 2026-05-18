# SP6j empty scatter no-op cleanup

Date: 2026-05-18
Worktree: `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf`
Branch: `recursive/wasm-webgpu-prover-perf`
Baseline commit: `688c95b71 SP6i: skip hash rows output uploads`

## Scope

SP6j fixes misleading WebGPU scatter diagnostics for empty/no-op scatter
ranges. Empty scatter inputs are semantically no-ops; they should not
fall through to the CPU fallback path or mark the destination buffer as
CPU-dirty.

Changed files:

- `risc0/zkp/src/hal/webgpu.rs`
- `examples/browser-prove/src/lib.rs`

## RED

Test:

- `webgpu_hal_empty_scatter_is_noop_without_cpu_fallback`

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_empty_scatter_is_noop_without_cpu_fallback -- --nocapture
```

Expected RED failure:

```text
panicked at browser-prove/src/lib.rs:590:9:
assertion `left == right` failed
  left: 1
 right: 0
```

RED verified: PASS. The no-op scatter recorded one CPU fallback.

## GREEN

Implementation:

- `WebGpuHal::scatter` now returns before dispatch/fallback accounting
  when no adjacent index range has writes.
- Non-empty scatter inputs still use the existing WebGPU dispatch and CPU
  fallback behavior.

Focused Chrome test:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_empty_scatter_is_noop_without_cpu_fallback -- --nocapture
```

Result:

```text
Finished `release` profile [optimized + debuginfo] target(s) in 4m 41s
test tests::webgpu_hal_empty_scatter_is_noop_without_cpu_fallback ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 111 filtered out; finished in 0.07s
```

GREEN verified: PASS.

## Xgboost smoke

Command:

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
browser-prove:metric pool_prove_scheduled_async receipt_kind=Succinct wall_ms=103214 segments=11 keccaks=0 pool_size=2
browser-prove:metric pool_xgboost_smoke wall_ms=103216
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5365 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4921 upload_bytes=10909213700 device_copies=128 device_copy_bytes=7222624256 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=37 bind_group_layout_cache_hits=1593 bind_group_creations=3358 compute_pipeline_creations=38 compute_pipeline_cache_hits=1592 buffers=6171 buffer_bytes=56466850780
browser-prove:webgpu-pool-op pool_xgboost_smoke: op=scatter gpu_dispatches=11 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 111 filtered out; finished in 103.37s
```

## Interpretation

SP6j is primarily a diagnostics/control-flow fix:

- Overall xgboost `cpu_fallbacks`: 11 -> 0
- Scatter `cpu_fallbacks`: 11 -> 0
- Upload bytes: unchanged at 10.91 GB from SP6i
- Wall: noisy/flat at 103.216 s

The result removes a false "CPU fallback" signal from the xgboost proof.
The remaining `scatter` diagnostics are CPU mirrors for non-empty GPU
scatter calls, not fallbacks.
