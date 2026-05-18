# SP6d iter 12 - xgboost default scheduled pool smoke

Date: 2026-05-18
Baseline commit before evidence: `9d49afa9a`

## Purpose

After SP6d iter 10 made `WebGpuProverPool::prove_with_ctx_async` route through the dependency-graph scheduler by default, rerun the canonical xgboost pool smoke. This is the run's best real-world multi-segment proxy without Keccak assumptions, so it checks whether the scheduler default helps or hurts a lift/join-heavy workload.

## Command

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=600 \
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_pool_xgboost_smoke
```

## Result

```text
running 1 test
test tests::webgpu_pool_xgboost_smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 108 filtered out; finished in 102.82s
```

The test also verifies the xgboost journal output equals `30.528042544062632`.

## Interpretation

The default scheduled pool path is safe on xgboost, but it is not a meaningful performance win for this workload. The 102.82s result is effectively flat against the current validation anchor for xgboost single-slot WebGPU (`102.7s`, about `18.0x` native CUDA).

This matches the SP6d iter 8/9 boundary:

- Homogeneous or lift/join-heavy work on one physical GPU stays flat.
- Heterogeneous KeccakUnion-style work can see a modest scheduler win because keccak proofs, union nodes, lifts, joins, and resolves expose different CPU/GPU phase shapes.
- The dominant remaining xgboost lever is still SP7 GPU-resident witness/accum coverage, not more same-GPU queue fan-out.
