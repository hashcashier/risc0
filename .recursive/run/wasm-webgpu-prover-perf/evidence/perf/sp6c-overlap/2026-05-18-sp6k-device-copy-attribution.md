# SP6k device-copy attribution

Date: 2026-05-18
Worktree: `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf`
Branch: `recursive/wasm-webgpu-prover-perf`
Baseline commit: `8998e0eea SP6j: treat empty scatter as no-op`

## Scope

SP6k attributes WebGPU device-to-device copy traffic by destination buffer
name. SP6j left xgboost with `device_copies=128` and
`device_copy_bytes=7222624256`, but the diagnostics only reported the
aggregate, making it unclear whether the traffic was from FRI finals,
coefficient materialization, or another copy path.

Changed files:

- `risc0/zkp/src/hal/webgpu.rs`
- `risc0/zkvm/src/host/client/prove/webgpu_pool.rs`
- `risc0/zkvm/src/lib.rs`
- `examples/browser-prove/src/lib.rs`

## RED

Test:

- `webgpu_hal_core_gpu_results_match_cpu`

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_core_gpu_results_match_cpu -- --nocapture
```

Expected RED failure:

```text
panicked at browser-prove/src/lib.rs:841:9:
device copy diagnostics should include the destination buffer name:
WebGpuDiagnostics { ... device_copies: 1, device_copy_bytes: 64, ... upload_sources: [], readback_sources: [...] }
```

RED verified: PASS. Device-copy totals existed, but the diagnostics did
not expose the destination buffer responsible for the copy.

## GREEN

Implementation:

- Added `WebGpuDeviceCopyDiagnostics` and
  `WebGpuDiagnostics.device_copy_sources`.
- Added per-source device-copy accounting to `WebGpuDiagnosticsState`.
- Added `WebGpuHal::copy_gpu_buffer_named` and routed
  `eltwise_copy_elem` through it using the output buffer name.
- Aggregated pool-level device-copy sources in `WebGpuProverPool`.
- Logged single-HAL and pool device-copy source lines in
  `examples/browser-prove`.

Focused Chrome test:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_core_gpu_results_match_cpu -- --nocapture
```

Result:

```text
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 111 filtered out; finished in 0.14s
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
browser-prove:metric pool_prove_scheduled_async receipt_kind=Succinct wall_ms=101794 segments=11 keccaks=0 pool_size=2
browser-prove:metric pool_xgboost_smoke wall_ms=101796
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5365 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4921 upload_bytes=10909213700 device_copies=128 device_copy_bytes=7222624256 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=37 bind_group_layout_cache_hits=1593 bind_group_creations=3358 compute_pipeline_creations=38 compute_pipeline_cache_hits=1592 buffers=6171 buffer_bytes=56466850780
browser-prove:webgpu-pool-device-copy pool_xgboost_smoke: source=coeffs device_copies=96 device_copy_bytes=7222591488
browser-prove:webgpu-pool-device-copy pool_xgboost_smoke: source=final_coeffs device_copies=32 device_copy_bytes=32768
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 111 filtered out; finished in 101.95s
```

## Interpretation

SP6k is attribution, not a wall-time optimization:

- Device copies remain 128 and 7.22 GB.
- `coeffs` accounts for 96 copies and 7,222,591,488 bytes.
- `final_coeffs` accounts for 32 copies and 32,768 bytes.
- Wall measured 101.796 s, within the SP6i-SP6j noise band.

The dominant remaining device-copy traffic is structural coefficient
materialization, not small FRI final copies. The next optimization target is
therefore the `make_coeffs` / `eltwise_copy_elem` path, but it cannot be
blindly converted to in-place mutation: some witness buffers are reused by
later proving stages, so ownership and lifecycle must be proven per group
before eliminating the copy.
