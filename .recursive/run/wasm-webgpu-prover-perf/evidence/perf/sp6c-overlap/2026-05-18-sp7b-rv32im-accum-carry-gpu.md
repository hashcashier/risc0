# SP7b RV32IM accumulation machine-column carry on WebGPU

Date: 2026-05-18
Worktree: `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf`
Branch: `recursive/wasm-webgpu-prover-perf`
Baseline commit: `5348461df SP7a: profile rv32im accumulation substages`

## Scope

SP7b offloads the isolated RV32IM accumulation machine-column carry scan to
WebGPU. CPU still runs generated `step_TopAccum` and the terminal ExtVal prefix
scan. The GPU carry is used only when the downstream RV32IM accum commit path
is GPU-authoritative, so CPU-authoritative diagnostic scopes keep the old CPU
path.

Changed files:

- `examples/browser-prove/src/lib.rs`
- `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs`
- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
- `risc0/circuit/rv32im/src/prove/mod.rs`

## RED

Focused test:

- `webgpu_rv32im_accum_machine_carry_matches_cpu`

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_rv32im_accum_machine_carry_matches_cpu -- --nocapture
```

Expected RED failure:

```text
error[E0432]: unresolved import `risc0_circuit_rv32im::prove::dispatch_webgpu_accum_machine_column_carry_for_test`
   --> browser-prove/src/lib.rs:981:13
    |
981 |         use risc0_circuit_rv32im::prove::dispatch_webgpu_accum_machine_column_carry_for_test;
    |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `dispatch_webgpu_accum_machine_column_carry_for_test` in `prove`
```

RED verified: PASS.

## GREEN

Implementation:

- Split CPU RV32IM accumulation so WebGPU can run generated `step_TopAccum`
  plus terminal ExtVal prefix, then stop before machine-column carry.
- Added `rv32im_accum_machine_column_carry` WGSL.
- Added a focused helper used by the browser test to compare GPU output with a
  CPU reference on an RV32IM-shaped column-major accum buffer.
- Production path runs the GPU carry only when all downstream accum commit
  stages are GPU-authoritative.

Focused GREEN command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_rv32im_accum_machine_carry_matches_cpu -- --nocapture
```

Result:

```text
test tests::webgpu_rv32im_accum_machine_carry_matches_cpu ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 114 filtered out; finished in 0.09s
```

GREEN verified: PASS.

## Correctness trap caught

An initial focused GREEN used row-major indexing in both the test reference and
the WGSL kernel. The focused test passed, but xgboost failed immediately:

```text
pool prove_with_ctx xgboost: prove segment 0

Caused by:
    0: verify segment
    1: verification indicates proof is invalid
browser-prove:stage done rv32im_accumulate machine_column_carry_gpu elapsed_ms=25.000 gpu_active=true
```

Root cause: RV32IM `BufferRow` uses column-major trace storage:
`idx = col * rows + row`. The test and kernel were corrected to use the same
layout before accepting the change.

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

- `/tmp/sp7b-accum-carry-xgboost-3.log`

Result:

```text
browser-prove:metric pool_xgboost_smoke wall_ms=101382
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5269 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4985 upload_bytes=10909202180 device_copies=32 device_copy_bytes=32768 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=40 bind_group_layout_cache_hits=1654 bind_group_creations=3422 compute_pipeline_creations=41 compute_pipeline_cache_hits=1653 buffers=6192 buffer_bytes=55003027164
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=accum uploads=119 upload_bytes=1716518912
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=combos uploads=64 upload_bytes=759169024
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=ctrl uploads=21 upload_bytes=506462208
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=data uploads=476 upload_bytes=7686062080
browser-prove:webgpu-pool-device-copy pool_xgboost_smoke: source=final_coeffs device_copies=32 device_copy_bytes=32768
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 114 filtered out; finished in 101.54s
```

Stage aggregates:

```text
rv32im_accumulate_step_top_accum count=11 sum_ms=20747.000 mean_ms=1886.091 min_ms=1716.000 max_ms=1961.000
rv32im_accumulate_machine_column_carry_cpu count=0 sum_ms=0.000 mean_ms=0.000 min_ms=0.000 max_ms=0.000
rv32im_accumulate_machine_column_carry_gpu count=11 sum_ms=274.000 mean_ms=24.909 min_ms=24.000 max_ms=26.000
rv32im_accumulate_total count=11 sum_ms=21033.000 mean_ms=1912.091 min_ms=1742.000 max_ms=1988.000
```

## Interpretation

Compared with SP7a:

- xgboost wall: 102.182 s -> 101.382 s
- RV32IM accumulation total: 22.130 s -> 21.033 s
- CPU machine-column carry: 1.570 s -> 0 s
- GPU machine-column carry: 0 s -> 0.274 s
- Uploads: effectively unchanged, because `accum` still originates on CPU
  before the carry kernel runs.

The measured wall win is modest but real enough to keep: about 0.8 s on this
xgboost run. The remaining large target is still generated `step_TopAccum`,
which was 20.747 s after this change and remains blocked on TopAccum/TopExtract
chunking.
