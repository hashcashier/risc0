Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7o -- TopAccum candidate sync gate
Date: 2026-05-18

## Headline

Arm10 proved that dispatch timing is not enough: `topaccum_arm10_authoritative`
reported only `34 ms` around enqueue, but the next GPU synchronization point
paid `45.528 s`. This slice adds a default-off sync gate that future generated
TopAccum candidates can enable during screening. The gate calls
`queue.onSubmittedWorkDone()` immediately after RV32IM accumulation and before
`commit_group_async rv32im_accum`, moving hidden queued GPU work into an
explicit `rv32im_accumulate candidate_sync_wait` timing bucket.

This is measurement/correctness infrastructure, not an accepted runtime speedup.
Default behavior is unchanged unless the browser test enables the flag.

## RED

The representative authoritative e2e was extended to enable the candidate sync
gate and assert that it ran for both representative workloads:

- BusyLoop `{ cycles: 200_000 }` at `segment_limit_po2(18)`
- `KeccakUnion(1)`

Initial compile failed because the public API did not exist:

```text
error[E0432]: unresolved imports `risc0_circuit_rv32im::prove::accum_gpu_candidate_sync_waits`, `risc0_circuit_rv32im::prove::set_accum_gpu_candidate_sync_enabled`
    --> browser-prove/src/lib.rs:4666:54
     |
4666 |             accum_gpu_arm5_authoritative_dispatches, accum_gpu_candidate_sync_waits,
     |                                                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `accum_gpu_candidate_sync_waits` in `prove`
4667 |             set_accum_gpu_arm5_authoritative_enabled, set_accum_gpu_candidate_sync_enabled,
     |                                                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `set_accum_gpu_candidate_sync_enabled` in `prove`
```

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify --no-run
```

## GREEN Compile

Added:

- `set_accum_gpu_candidate_sync_enabled`
- `accum_gpu_candidate_sync_waits`
- `WebGpuCircuitHal::post_accum_candidate_sync_async`
- async hook after `witgen.accum(...)` and before `commit_group_async rv32im_accum`

Compile passed:

```text
Finished `release` profile [optimized + debuginfo] target(s) in 4m 22s
Executable unittests src/lib.rs (examples/target/wasm32-unknown-unknown/release/deps/browser_prove-8b5b728d1fc35820.wasm)
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
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 117.15s
```

BusyLoop verified with high WebGPU limits and zero fallback:

```text
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
browser-prove:stage done rv32im_accumulate topaccum_arm5_authoritative cycles=40968 elapsed_ms=31.000 gpu_active=true
browser-prove:stage start rv32im_accumulate candidate_sync_wait
browser-prove:metric rv32im_accumulate candidate_sync_wait completed
browser-prove:stage done rv32im_accumulate candidate_sync_wait elapsed_ms=3351.000 gpu_active=true
browser-prove:stage start commit_group_async rv32im_accum
browser-prove:stage done commit_group_async rv32im_accum elapsed_ms=115.000 gpu_active=true
browser-prove:metric prove_session_async wall_ms=10715.0 gpu_active_ms=7521.0 gpu_idle_ratio=0.298
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
```

KeccakUnion(1) verified with zero fallback:

```text
browser-prove:metric prove_session_async wall_ms=106208.0 gpu_active_ms=72507.0 gpu_idle_ratio=0.317
browser-prove:done multi_test/keccak_union_topaccum_arm5_authoritative: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_authoritative: gpu_dispatches=5904 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0
```

The test assertion `accum_gpu_candidate_sync_waits() > 1` passed, proving that
the sync gate ran across both representative workloads.

## Hygiene

```text
git diff --check
# pass
```

## Decision

Accepted as default-off instrumentation for future generated TopAccum candidate
screening. It exposes hidden queued GPU time before a full candidate is accepted
or rejected.

Accepted wall-time reduction: `0 s`.
