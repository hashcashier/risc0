# SP6d iter 11 - public pool diagnostics + Keccak smoke

Date: 2026-05-18
Baseline commit before change: `5a3499bb7d8441a5231ca493605ea02b0b7c5f4c`

## Purpose

SP6d iter 10 made `WebGpuProverPool::prove_with_ctx_async` route through the dependency-graph scheduler by default and added the public `webgpu_prover_pool(slots)` constructor plus async convenience methods. This iteration removes the next harness/API gap: pooled acceptance fixtures can now reset and assert aggregate WebGPU diagnostics across all pool slots, then exercise the public pooled convenience path directly.

## Changed surface

- `risc0/zkvm/src/host/client/prove/webgpu_pool.rs`
  - Added `WebGpuProverPool::diagnostics() -> WebGpuDiagnostics`.
  - Added `WebGpuProverPool::reset_diagnostics()`.
  - Aggregation sums scalar counters and merges per-op/upload/readback rows by name with saturating arithmetic.
- `examples/browser-prove/src/lib.rs`
  - Added `log_webgpu_pool_diagnostics`, mirroring the single-prover diagnostics assertion style.
  - Strengthened `webgpu_prover_pool_convenience_uses_scheduled_path_smoke` to reset/read pool diagnostics and assert dev-mode rejection happens before GPU dispatches.
  - Routed `webgpu_pool_prove_session_keccak_union_smoke` through `webgpu_prover_pool(2).prove_with_opts_async(...)` instead of constructing `WebGpuProverPool` directly and calling `prove_with_ctx_async(...)`.

## TDD evidence

### RED

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_prover_pool_convenience_uses_scheduled_path_smoke --no-run
```

Expected failure: browser test code asks the pool to reset and read aggregate diagnostics before the production API exists.

Observed failure:

- `error[E0599]: no method named reset_diagnostics found for struct Rc<WebGpuProverPool>`
- `error[E0599]: no method named diagnostics found for struct Rc<WebGpuProverPool>`
- `error[E0599]: no method named diagnostics found for reference &WebGpuProverPool`

RED verified: PASS.

### GREEN

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_prover_pool_convenience_uses_scheduled_path_smoke --no-run
```

Observed result after implementation:

- Finished release profile.
- Produced `examples/target/wasm32-unknown-unknown/release/deps/browser_prove-8b5b728d1fc35820.wasm`.

GREEN compile verified: PASS.

## Browser verification

Fast public API route check:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_prover_pool_convenience_uses_scheduled_path_smoke
```

Result:

- `1 passed; 0 failed; 108 filtered out; finished in 0.07s`.
- The test verified `prove_with_opts_async` records `last_prove_strategy_for_diagnostics() == Some("scheduled")`.
- The test verified dev-mode rejection happened before aggregate pool GPU dispatches.

Full public pooled Keccak smoke:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=600 \
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_pool_prove_session_keccak_union_smoke
```

Result:

- `1 passed; 0 failed; 108 filtered out; finished in 194.34s`.
- Receipt verified against `MULTI_TEST_ID`.
- The smoke now uses the public pooled constructor and async options API.
- Passing `log_webgpu_pool_diagnostics` asserts aggregate `gpu_dispatches > 0` and aggregate `cpu_only_ops == 0` across pool slots.

Note: `wasm-bindgen-test-runner` suppresses scraped `console.log` diagnostics on successful headless browser runs, so the command output does not include the numeric aggregate diagnostic rows. The pass still covers the aggregate assertions above.

## Additional checks

```bash
git diff --check
```

Result: PASS.

## Interpretation

This is not a new wall-time breakthrough by itself. It makes the SP6d iter 10 scheduler default consumable by the same public API style users should call in browser code, and it gives pooled browser fixtures a correctness/performance guard equivalent to the single-prover `WebGpuProver::diagnostics()` assertions.

The full Keccak pooled smoke remains minutes-scale; the observed 194.34s browser test duration should be treated as an acceptance smoke timing, not a CUDA-parity benchmark. The run objective remains active because SP7 GPU-resident witness coverage is still the dominant unresolved path to materially closing the CUDA gap.
