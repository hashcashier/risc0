Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7e -- TopAccum arm5 real-buffer e2e probe
Date: 2026-05-18

## Headline

TopAccum arm5 now has an opt-in browser proof probe that compiles and executes
the generated `step_TopAccumArm5` WGSL against real proof buffers during an e2e
succinct proof, while preserving receipt correctness by binding a scratch copy
of `accum`.

This is not a wall-time speedup yet. It is the correctness gate before any
authoritative TopAccum replacement.

## TDD Evidence

RED:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_real_buffer_probe_e2e_verify --no-run

error[E0432]: unresolved imports
  accum_gpu_arm5_probe_dispatches
  set_accum_gpu_arm5_probe_enabled
```

GREEN compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_real_buffer_probe_e2e_verify --no-run

Finished release profile ... browser_prove-8b5b728d1fc35820.wasm
```

## Negative Authoritative Probe

First implementation bound the generated arm directly to authoritative
`accum`. Browser e2e proof reached the probe and dispatched:

```text
rv32im_accumulate topaccum_arm5_probe sample_cycle=18334 available_cycles=40968
rv32im_accumulate topaccum_arm5_probe dispatched sample_cycle=18334 available_cycles=40968
```

The segment verifier then rejected the proof:

```text
async prove failed: verify segment
Caused by:
    verification indicates proof is invalid
```

Interpretation: generated arm5 execution over real buffers is device-safe, but
its writes are not yet safe to use as authoritative `accum` output. Any
replacement path now requires cell/row diff evidence before it may write the
real `accum` buffer.

## Scratch Probe GREEN

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=180 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_real_buffer_probe_e2e_verify -- --nocapture
```

Result:

```text
test tests::rv32im_accum_topaccum_arm5_real_buffer_probe_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 117 filtered out; finished in 8.79s
```

Key metrics:

```text
prove_session_async wall_ms=8644.0 gpu_active_ms=5212.0 gpu_idle_ratio=0.397
rv32im_accumulate step_top_accum elapsed_ms=1966.000
rv32im_accumulate topaccum_arm5_probe elapsed_ms=27.000 gpu_active=true
rv32im_accumulate topaccum_arm5_probe dispatched sample_cycle=18334 available_cycles=40968
cpu_fallbacks=0 cpu_only_ops=0
device_copy source=rv32im_accum_topaccum_arm5_probe_accum_scratch bytes=108003328
```

## Scratch-vs-CPU Row Diff

Follow-up diagnostic added a GPU row-compare kernel for the sampled row. The
same focused e2e proof still verifies:

```text
test tests::rv32im_accum_topaccum_arm5_real_buffer_probe_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 117 filtered out; finished in 8.80s
```

Key metrics:

```text
prove_session_async wall_ms=8637.0 gpu_active_ms=5244.0 gpu_idle_ratio=0.393
rv32im_accumulate step_top_accum elapsed_ms=1918.000
rv32im_accumulate topaccum_arm5_probe elapsed_ms=27.000 gpu_active=true
rv32im_accumulate topaccum_arm5_probe dispatched sample_cycle=18334 available_cycles=40968
topaccum_arm5_probe mismatch_count=40 first_mismatch_col=23
cpu_fallbacks=0 cpu_only_ops=0
```

Interpretation: the generated arm5 body is executable on real buffers, but it
does not match CPU `step_TopAccum` for the sampled row. The mismatch is now
bounded: 40 accum columns differ, with first mismatch at column 23. Authoritative
replacement remains blocked until those columns are explained and fixed.

## Default-Off Regression

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=180 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  native_busy_loop_po2_18_async_succinct_receipt_verify -- --nocapture
```

Result:

```text
test tests::native_busy_loop_po2_18_async_succinct_receipt_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 117 filtered out; finished in 8.02s
```

Key metrics:

```text
prove_session_async wall_ms=7871.0 gpu_active_ms=4459.0 gpu_idle_ratio=0.433
rv32im_accumulate step_top_accum elapsed_ms=1936.000
cpu_fallbacks=0 cpu_only_ops=0
device_copy_bytes=2048
```

## Wall-Time Impact

Current accepted wall-time reduction: **0 s**. The probe is opt-in and uses a
scratch `accum` copy, so it intentionally adds overhead when enabled and has no
effect when disabled.

The useful result is that Chrome executes the split arm5 TopAccum body on real
proof buffers without device loss, and the scratch row diff now explains why the
authoritative attempt failed. The next performance step is to map mismatch
columns 23+ back to TopAccum layout fields and fix the generated arm or wrapper
until the sampled row reaches `mismatch_count=0`.
