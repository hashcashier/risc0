# SP7c recursion accumulation substage profile

Date: 2026-05-18
Worktree: `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf`
Branch: `recursive/wasm-webgpu-prover-perf`
Baseline commit: `fbd1d34c1 SP7b: offload rv32im accum carry`

## Scope

SP7c adds wasm-only timing around recursion accumulation substages:

- generated `compute_accum`
- `calc_prefix_products`
- generated `verify_accum`

This is diagnostic instrumentation only. The compensating validation gate is
the full xgboost e2e proof receipt.

Changed file:

- `risc0/circuit/recursion/src/prove/hal/rust_kernels.rs`

## Xgboost e2e proof

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=600 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_pool_xgboost_smoke -- --nocapture
```

Captured log:

- `/tmp/sp7c-recursion-accum-profile.log`

Result:

```text
browser-prove:metric pool_xgboost_smoke wall_ms=101254
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5269 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4985 upload_bytes=10909202180 device_copies=32 device_copy_bytes=32768 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=40 bind_group_layout_cache_hits=1654 bind_group_creations=3422 compute_pipeline_creations=41 compute_pipeline_cache_hits=1653 buffers=6192 buffer_bytes=55003027164
browser-prove:webgpu-pool-device-copy pool_xgboost_smoke: source=final_coeffs device_copies=32 device_copy_bytes=32768
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 114 filtered out; finished in 101.42s
```

Stage aggregates:

```text
recursion_accumulate_total count=21 sum_ms=5724.000 mean_ms=272.571 min_ms=196.000 max_ms=355.000
recursion_accumulate_compute_accum count=21 sum_ms=3359.000 mean_ms=159.952 min_ms=114.000 max_ms=208.000
recursion_accumulate_prefix_products count=21 sum_ms=59.000 mean_ms=2.810 min_ms=2.000 max_ms=4.000
recursion_accumulate_verify_accum count=21 sum_ms=2298.000 mean_ms=109.429 min_ms=78.000 max_ms=143.000
```

## Interpretation

Recursion accumulation does not have an SP7b-style cheap post-scan target:

- Prefix products are only 59 ms across the whole xgboost run.
- The measurable surface is generated circuit work:
  - `compute_accum`: 3.36 s
  - `verify_accum`: 2.30 s

So the next material wall-time target is still generated circuit execution on
GPU: RV32IM `step_TopAccum` first because it is about 20.7 s, then recursion
compute/verify accumulation and witness generation.
