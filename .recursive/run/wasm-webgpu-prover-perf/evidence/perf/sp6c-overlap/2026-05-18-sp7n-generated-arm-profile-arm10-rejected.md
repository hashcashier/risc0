Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7n -- Generated TopAccum arm profile and arm10 rejection
Date: 2026-05-18

## Headline

The generated TopAccum arm profiler was added after arm0 failed. It shows why
selecting the next arm by xgboost cycle count alone is too blunt: each arm has
different generated body size and inverse pressure.

Arm10 looked attractive by inverse pressure (`2` `ext_inv` calls, `389,142`
xgboost inverse items), so it was tested as an opt-in authoritative path. It
passed receipt verification on both BusyLoop and `KeccakUnion(1)`, but was a
severe BusyLoop performance regression: `commit_group_async rv32im_accum` hid
`45.528 s` of queued GPU work and BusyLoop wall grew to `53.034 s`.

The arm10 runtime/test path was removed. Current-tree validation after cleanup
uses the retained arm5 authoritative representative e2e, which covers BusyLoop
and `KeccakUnion(1)`.

## Generator Profile

Command:

```text
rustc --edition=2021 --test risc0/circuit/rv32im/src/prove/wgsl_pruner.rs -o /tmp/wgsl_pruner_test
/tmp/wgsl_pruner_test topaccum_arm_generator_reproduces_vendored_arm5 topaccum_arm_generator_profiles_all_arms --nocapture
```

Result:

```text
running 2 tests
arm generated_bytes nonblank_lines ext_inv_calls xgboost_cycles xgboost_inv_items
test tests::topaccum_arm_generator_reproduces_vendored_arm5 ... ok
0 207447 1308 25 768461 19211525
1 212711 1359 25 173854 4346350
2 126936 1197 25 399587 9989675
3 194314 1708 37 65151 2410587
4 282275 2398 47 5203 244541
5 112962 1114 26 430888 11203088
6 126258 1163 29 351857 10203853
7 201625 1625 58 388742 22547036
8 125791 1197 26 74670 1941420
9 741353 3593 52 30308 1576016
10 157201 1205 2 194571 389142
11 423126 2015 17 292 4964
12 267381 1902 42 0 0
test tests::topaccum_arm_generator_profiles_all_arms ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.17s
```

## Arm10 RED/GREEN/E2E

RED compile after adding the representative arm10 e2e:

```text
error[E0432]: unresolved imports
risc0_circuit_rv32im::prove::accum_gpu_arm10_authoritative_dispatches,
risc0_circuit_rv32im::prove::set_accum_gpu_arm10_authoritative_enabled
```

GREEN compile after adding the opt-in arm10 runtime path:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm10_authoritative_e2e_verify --no-run

Finished `release` profile [optimized + debuginfo] target(s) in 4m 21s
```

Representative browser e2e passed correctness but failed the performance gate:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 121 filtered out; finished in 160.00s
```

BusyLoop metrics:

```text
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
browser-prove:stage start rv32im_accumulate topaccum_arm10_authoritative cycles=19152
browser-prove:metric rv32im_accumulate topaccum_arm10_authoritative dispatched cycles=19152 inv_items=38304
browser-prove:stage done rv32im_accumulate topaccum_arm10_authoritative cycles=19152 elapsed_ms=34.000 gpu_active=true
browser-prove:stage done commit_group_async rv32im_accum elapsed_ms=45528.000 gpu_active=true
browser-prove:metric prove_session_async wall_ms=53034.0 gpu_active_ms=49569.0 gpu_idle_ratio=0.065
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm10_authoritative: cpu_fallbacks=0 cpu_only_ops=0
```

KeccakUnion(1) metrics:

```text
browser-prove:metric prove_session_async wall_ms=106729.0 gpu_active_ms=72584.0 gpu_idle_ratio=0.320
browser-prove:done multi_test/keccak_union_topaccum_arm10_authoritative: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm10_authoritative: cpu_fallbacks=0 cpu_only_ops=0
```

Interpretation:

- The path was correctness-positive; both receipts verified and no fallback
  counters moved.
- The path was performance-negative; arm10 reduced inverse count but generated
  body cost dominated and surfaced at the next GPU synchronization point.
- `WebGpuStageTimer` around dispatch measures enqueue time. The actual GPU
  cost for generated arms must be measured by forcing a sync/readback or by the
  following proof-stage sync, not by dispatch elapsed time alone.

## Cleanup Verification

Arm10 runtime/test references were removed:

```text
rg -n "arm10|ARM10|TopAccum arm10|topaccum_arm10" \
  risc0/circuit/rv32im/src/prove/hal/webgpu.rs \
  risc0/circuit/rv32im/src/prove/mod.rs \
  examples/browser-prove/src/lib.rs

# no matches
```

The current-tree representative arm5 authoritative browser e2e was rerun with
the correct package-scoped command:

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
Finished `release` profile [optimized + debuginfo] target(s) in 4m 20s
Running unittests src/lib.rs (examples/target/wasm32-unknown-unknown/release/deps/browser_prove-8b5b728d1fc35820.wasm)
test tests::rv32im_accum_topaccum_arm5_authoritative_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 116.81s
```

BusyLoop verified:

```text
browser-prove:metric prove_session_async wall_ms=10759.0 gpu_active_ms=7556.0 gpu_idle_ratio=0.298
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
```

KeccakUnion(1) verified:

```text
browser-prove:metric prove_session_async wall_ms=105801.0 gpu_active_ms=72474.0 gpu_idle_ratio=0.315
browser-prove:done multi_test/keccak_union_topaccum_arm5_authoritative: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_authoritative: gpu_dispatches=5904 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0
```

## Decision

Rejected: arm10 authoritative replacement using the current generated-arm,
per-row split-inverse design.

Accepted: keep the generator/reproducer/profile tests as durable guardrails.

Next requirement before trying another generated arm: add a micro gate that
dispatches the candidate on real cycle lists and forces a small GPU sync so
hidden queued work is measured before a full proof e2e. Inverse count alone is
not a safe selector.

Accepted wall-time reduction from arm10: `0 s`.
