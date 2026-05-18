# SP7a RV32IM accumulation substage profile

Date: 2026-05-18
Worktree: `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf`
Branch: `recursive/wasm-webgpu-prover-perf`
Baseline commit: `fd9e69662 SP6m: fuse coeff copy into interpolate`

## Scope

SP7a adds wasm-only timing around the three phases inside RV32IM
`run_accum_steps`:

- generated `step_TopAccum` loop
- terminal ExtVal prefix scan
- machine-column carry scan

This is diagnostic instrumentation only. TDD mode is pragmatic for this slice:
there is no new behavior assertion, and the compensating gate is the full
xgboost e2e proof receipt with unchanged verifier/journal expectations.

Changed file:

- `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs`

## Validation command

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=600 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_pool_xgboost_smoke -- --nocapture
```

Captured log:

- `/tmp/sp7a-accum-substage-profile.log`

Result:

```text
browser-prove:metric pool_xgboost_smoke wall_ms=102182
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5269 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4974 upload_bytes=10909201828 device_copies=32 device_copy_bytes=32768 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=39 bind_group_layout_cache_hits=1644 bind_group_creations=3411 compute_pipeline_creations=40 compute_pipeline_cache_hits=1643 buffers=6181 buffer_bytes=55003026812
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=accum uploads=119 upload_bytes=1716518912
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=combos uploads=64 upload_bytes=759169024
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=ctrl uploads=21 upload_bytes=506462208
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=data uploads=476 upload_bytes=7686062080
browser-prove:webgpu-pool-device-copy pool_xgboost_smoke: source=final_coeffs device_copies=32 device_copy_bytes=32768
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 113 filtered out; finished in 102.34s
```

## Stage aggregates

```text
rv32im_witgen count=11 sum_ms=5899.000 mean_ms=536.273 min_ms=507.000 max_ms=613.000
rv32im_accumulate_total count=11 sum_ms=22130.000 mean_ms=2011.818 min_ms=1843.000 max_ms=2081.000
rv32im_accumulate_step_top_accum count=11 sum_ms=20548.000 mean_ms=1868.000 min_ms=1702.000 max_ms=1939.000
rv32im_accumulate_terminal_ext_prefix count=11 sum_ms=10.000 mean_ms=0.909 min_ms=0.000 max_ms=1.000
rv32im_accumulate_machine_column_carry count=11 sum_ms=1570.000 mean_ms=142.727 min_ms=138.000 max_ms=156.000
recursion_witgen count=21 sum_ms=5138.000 mean_ms=244.667 min_ms=170.000 max_ms=331.000
recursion_accumulate count=21 sum_ms=5678.000 mean_ms=270.381 min_ms=194.000 max_ms=367.000
finalize_fri_prove count=32 sum_ms=50508.000 mean_ms=1578.375 min_ms=925.000 max_ms=2294.000
```

Top upload sources remain CPU-originated:

```text
data   upload_bytes=7686062080
accum  upload_bytes=1716518912
combos upload_bytes=759169024
ctrl   upload_bytes=506462208
```

## Interpretation

The RV32IM accumulation surface is real and stable:

- Total RV32IM accumulation: 22.13 s across 11 segments.
- Generated `step_TopAccum`: 20.55 s of that total.
- Machine-column carry scan: 1.57 s.
- Terminal ExtVal prefix scan: 0.01 s; not worth targeting.

The main 20.55 s lever remains blocked by the existing `TopAccum` /
`TopExtract` reachable-closure problem documented in SP7 iter 6a/6b.
The machine-column carry scan is a smaller but well-isolated target:
it runs after CPU `step_TopAccum`, uses only the terminal ExtVal prefix
columns, and occurs before the RV32IM `accum` commit. A WebGPU kernel for
this scan should be correctness-checkable against the current CPU result
and has an estimated xgboost wall ceiling of about 0.8-1.5 s after dispatch
and upload overlap costs.
