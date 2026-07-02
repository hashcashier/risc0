# SP7ch - Merkle row/fold drain diagnostic

Date: 2026-05-20

## Purpose

SP7ce showed that simple timers around WebGPU Merkle substages measured enqueue time, not GPU execution. The current xgboost profile after SP7cg shows the largest remaining bucket is:

- `fri_prove round=0 merkle_new rows=65536 cols=64 = 27512 ms` across 32 calls.

This diagnostic temporarily inserted controlled `hal.wait_idle().await?` drains for exactly that hot shape inside `MerkleTreeProver::new_committed_async`:

- after `hash_rows_async`
- after `hash_fold_chain_async`

The instrumentation was diagnostic-only and was reverted after the run.

## Compile

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
```

Result:

- Passed.
- Elapsed: `4m55s`.

## xgboost e2e diagnostic

Command output: `/tmp/sp7ch-xgboost-merkle-drain-diag.log`

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result:

- `test tests::iter6d_g_replace_xgboost ... ok`
- `test result: ok. 1 passed; 0 failed; 137 filtered out; finished in 89.42s`
- `wall_ms=89183`
- `gpu_active_ms=84002`
- `gpu_idle_ratio=0.058` because the diagnostic waits intentionally classified most Merkle work as active drained GPU time.
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `raw_compute_dispatches=11466`
- `queue_submits=2718`

## Split

Controlled-drain totals for the hot shape:

- `merkle_drain_diag hash_rows rows=65536 cols=64`: `27360 ms` across 32 calls.
- `merkle_drain_diag hash_fold_chain rows=65536 cols=64`: `177 ms` across 32 calls.
- Enclosing `fri_prove round=0 merkle_new rows=65536 cols=64`: `27632 ms` across 32 calls.

Per-call row hashing is roughly `855 ms`; fold-chain is roughly `5.5 ms`.

## Decision

Diagnostic accepted and reverted. The next material implementation target is Poseidon2 row hashing for `rows=65536 cols=64`, not fold-chain batching, fold constants, or submit-count work.

Implication: a row-hash implementation that improves this kernel by:

- `10%` should reduce xgboost wall by about `2.7 s`.
- `20%` should reduce xgboost wall by about `5.5 s`.
- `40%` should reduce xgboost wall by about `10.9 s`.

The likely productive direction is a new specialized row-hash kernel for the `cols=64` Merkle leaf shape. Any retained version must still pass BusyLoop, KeccakUnion, and xgboost browser proof e2e with zero fallback/CPU-only ops.
