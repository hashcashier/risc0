Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6d iter 10 -- default pool route uses dependency scheduler`
Date: 2026-05-18
Status: `IMPLEMENTED`

## Headline

`WebGpuProverPool::prove_with_ctx_async` now routes through
`prove_with_ctx_scheduled_async` by default. The old phased pool path
is preserved as `prove_with_ctx_sequential_async` for A/B comparison
and fallback. A public `webgpu_prover_pool(slots)` constructor and
`prove_async` / `prove_with_opts_async` convenience methods let browser
callers use the scheduled pool with the same async method shape as
`WebGpuProver`.

This wires the measured-positive SP6d scheduler path into the public
pool entrypoint. Iter 9 measured the scheduler at ~8% wall-time savings
on the mixed `KeccakUnion(3)` workload versus the same scheduler with a
1-slot serial baseline.

## Why this change

Before this step, callers had to opt into the only SP6d path with a
positive wall-time result by calling `prove_with_ctx_scheduled_async`
directly. The normal pool entrypoint still used the older phased flow:
segments, then keccaks, then union, then compression. That meant the
validated heterogeneous-overlap scheduler was not the default for pool
users.

## TDD Evidence

TDD Mode: `strict`

RED command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_pool_default_uses_scheduled_path_smoke --no-run
```

RED result:

- Failed with `E0599`: no method named
  `last_prove_strategy_for_diagnostics` on `WebGpuProverPool`.
- This was the expected failure for the route-selection assertion.

GREEN compile command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_pool_default_uses_scheduled_path_smoke --no-run
```

GREEN compile result:

- Passed; produced
  `examples/target/wasm32-unknown-unknown/release/deps/browser_prove-8b5b728d1fc35820.wasm`.

GREEN browser command 1:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_pool_default_uses_scheduled_path_smoke -- --nocapture
```

GREEN browser result 1:

- Passed in Chrome/WebGPU.
- Chrome limits in this run:
  `max_buffer_size=4294967292`,
  `max_storage_buffer_binding_size=2147483644`,
  `max_compute_workgroup_storage_size=49152`.
- Test result: `1 passed; 0 failed; 107 filtered out; finished in 0.07s`.

Second RED command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_prover_pool_convenience_uses_scheduled_path_smoke --no-run
```

Second RED result:

- Failed with `E0432`: unresolved import `risc0_zkvm::webgpu_prover_pool`.
- This was the expected failure for the missing pooled convenience
  constructor.

Second GREEN compile command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_prover_pool_convenience_uses_scheduled_path_smoke --no-run
```

Second GREEN compile result:

- Passed; produced the browser-prove wasm test artifact.

GREEN browser command 2:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_prover_pool_convenience_uses_scheduled_path_smoke -- --nocapture
```

GREEN browser result 2:

- Passed in Chrome/WebGPU.
- Chrome limits in this run:
  `max_buffer_size=4294967292`,
  `max_storage_buffer_binding_size=2147483644`,
  `max_compute_workgroup_storage_size=49152`.
- Test result: `1 passed; 0 failed; 108 filtered out; finished in 0.08s`.

## Failed test design discarded

The first GREEN browser attempt used `BusyLoop { cycles: 1 }` and tried
to verify a real succinct receipt through the default pool path. In
this environment that still produced a full po2=15 segment plus
recursion lift and timed out at the runner's 600-second cap. The output
showed extremely slow stage timings, including:

- `finalize_async check_group` for the rv32im segment: `214742 ms`.
- `commit_group_async recursion_data`: `73281 ms`.
- The run was still inside recursion `finalize_async check_group` when
  ChromeDriver was killed.

That was not a route-selection test; it was an accidental full proving
benchmark. The final test uses the scheduler's early dev-mode rejection
path instead, proving route selection without spending minutes on a
receipt.

## Files changed

- `risc0/zkvm/src/host/client/prove/webgpu_pool.rs`
  - Added public `webgpu_prover_pool(slots)` constructor.
  - Added `prove_async` and `prove_with_opts_async` convenience methods
    to mirror `WebGpuProver`'s async API shape.
  - Added diagnostics-only `last_prove_strategy_for_diagnostics`.
  - Preserved old phased route as `prove_with_ctx_sequential_async`.
  - Changed `prove_with_ctx_async` to call
    `prove_with_ctx_scheduled_async`.
- `risc0/zkvm/src/lib.rs`
  - Re-exported `webgpu_prover_pool` for browser WebGPU client builds.
- `examples/browser-prove/src/lib.rs`
  - Added `webgpu_pool_default_uses_scheduled_path_smoke`.
  - Added `webgpu_prover_pool_convenience_uses_scheduled_path_smoke`.

## Follow-up

The pool scheduler is now reachable through `webgpu_prover_pool(slots)`
and the pool's convenience async methods. `webgpu_prover()` still
constructs a single `WebGpuProver`; switching that default would be a
larger API/behavior decision because it changes device allocation count
for existing browser users.
