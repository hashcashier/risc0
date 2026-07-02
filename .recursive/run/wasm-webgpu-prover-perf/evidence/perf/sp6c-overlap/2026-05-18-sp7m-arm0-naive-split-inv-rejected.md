Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7m -- TopAccum arm0 naive split-inverse rejection
Date: 2026-05-18

## Headline

The next generated TopAccum target was major0 because the xgboost histogram
shows it is the largest arm (`768,461 / 2,883,584` cycles, 26.65%). A strict
RED/GREEN attempt added an opt-in authoritative arm0 path and representative
browser e2e test covering both BusyLoop and `KeccakUnion(1)`.

The compile GREEN succeeded, but the actual browser proof e2e failed on
BusyLoop before receipt verification. The failure is a real performance and
capacity blocker: the naive split-inverse design queued 1.93M independent
extension inversions for a single po2_18 BusyLoop proof and pushed the accum
commit to 45.951 s before Chrome reported WebGPU instance loss.

The arm0 production/test code was removed. Do not reintroduce arm0 using
per-denominator `ext_inv` unless a new e2e proof demonstrates correctness and
material wall-time improvement.

## RED Evidence

Initial RED compile after adding the representative arm0 e2e:

```text
error[E0432]: unresolved imports
`risc0_circuit_rv32im::prove::accum_gpu_arm0_authoritative_dispatches`,
`risc0_circuit_rv32im::prove::set_accum_gpu_arm0_authoritative_enabled`
```

The failing test intentionally used both workload classes:

- `BusyLoop { cycles: 200_000 }` with `segment_limit_po2(18)`
- `KeccakUnion(1)`

## Compile GREEN

After adding the arm0 API and generated-arm authoritative path:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm0_authoritative_e2e_verify --no-run

Finished `release` profile [optimized + debuginfo] target(s) in 4m 23s
```

## E2E Failure

Representative browser e2e command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm0_authoritative_e2e_verify -- --nocapture
```

Observed BusyLoop metrics before failure:

```text
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
browser-prove:stage start rv32im_accumulate topaccum_arm0_authoritative cycles=77196
browser-prove:metric rv32im_accumulate topaccum_arm0_authoritative dispatched cycles=77196 inv_items=1929900
browser-prove:stage done rv32im_accumulate topaccum_arm0_authoritative cycles=77196 elapsed_ms=35.000 gpu_active=true
browser-prove:stage start commit_group_async rv32im_accum
browser-prove:stage done commit_group_async rv32im_accum elapsed_ms=45951.000 gpu_active=true
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm0_authoritative: gpu_dispatches=79 cpu_mirrors=5 cpu_fallbacks=0 cpu_only_ops=0
browser-prove:async-prove-error name=multi_test/busy_loop_po2_18_topaccum_arm0_authoritative err=JsValue(AbortError: Failed to execute 'mapAsync' on 'GPUBuffer': A valid external Instance reference no longer exists.)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 121 filtered out; finished in 48.82s
```

Interpretation:

- The failure is not a CPU fallback; fallback counters stayed zero.
- The queued arm0 inverse workload is too large for the current
  per-denominator split-inverse strategy.
- Because BusyLoop failed before receipt verification, `KeccakUnion(1)` was not
  reached. This is enough to reject the path for correctness-first work.

## Cleanup Verification

The unvalidated arm0 runtime/test code was removed. The validated
representative arm5 authoritative e2e was rerun:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture

test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 116.52s
```

BusyLoop verified:

```text
browser-prove:metric prove_session_async wall_ms=10614.0 gpu_active_ms=7427.0 gpu_idle_ratio=0.300
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
```

KeccakUnion(1) verified:

```text
browser-prove:metric prove_session_async wall_ms=105675.0 gpu_active_ms=72351.0 gpu_idle_ratio=0.315
browser-prove:done multi_test/keccak_union_topaccum_arm5_authoritative: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_authoritative: gpu_dispatches=5904 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0
```

## Decision

Rejected: generated arm0 authoritative replacement using the current
per-denominator split-inverse kernel.

Required before retry:

- a correctness-preserving inverse strategy that does not scale as one full
  `ext_inv` per denominator, or
- a different generated TopAccum arm whose inverse item count is small enough
  to pass BusyLoop + `KeccakUnion(1)` e2e and xgboost A/B without device loss.

