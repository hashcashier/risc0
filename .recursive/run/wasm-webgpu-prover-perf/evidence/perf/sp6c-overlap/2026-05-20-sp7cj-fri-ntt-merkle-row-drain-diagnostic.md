# SP7cj FRI round-0 NTT vs Merkle row drain diagnostic

Date: 2026-05-20

## Purpose

SP7ch attributed the dominant xgboost `fri_prove round=0 merkle_new rows=65536 cols=64` bucket to Merkle `hash_rows`, but that diagnostic did not drain immediately before Merkle construction. SP7cj inserted a controlled drain after `batch_expand_into_evaluate_ntt_async` and then repeated the Merkle row/fold drains to split queued FRI round-0 NTT work from actual Merkle hashing.

## Temporary Instrumentation

- `risc0/zkp/src/prove/fri.rs`: for `round_idx == 0 && domain == 1048576`, wrapped `hal.wait_idle().await?` in active timer `fri_round0_drain_diag expand_evaluate_ntt domain=1048576` immediately after `batch_expand_into_evaluate_ntt_async`.
- `risc0/zkp/src/prove/merkle.rs`: for `rows == 65536 && cols == 64`, wrapped waits after `hash_rows_async` and `hash_fold_chain_async` in active timers.

The instrumentation was diagnostic-only and was reverted after the run.

## Commands

Compile before e2e:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
```

Browser proof e2e:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture > /tmp/sp7cj-xgboost-fri-ntt-merkle-drain-diag.log 2>&1
```

Post-revert hygiene:

```bash
rg -n "fri_round0_drain_diag|merkle_drain_diag" risc0/zkp/src/prove/fri.rs risc0/zkp/src/prove/merkle.rs risc0
git diff --check
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
```

## E2E Result

Log: `/tmp/sp7cj-xgboost-fri-ntt-merkle-drain-diag.log`

- Test result: `ok. 1 passed; 0 failed; 0 ignored; 137 filtered out; finished in 90.43s`
- Proof session: `wall_ms=90196.0`, `gpu_active_ms=84508.0`, `gpu_idle_ratio=0.063`
- Counters: `raw_compute_dispatches=11466`, `queue_submits=2718`
- Correctness gate: `cpu_fallbacks=0`, `cpu_only_ops=0`
- Data movement: `upload_bytes=3104820276`, `readback_bytes=7488592`

## Drain Split

Aggregate over 32 xgboost round-0 calls:

```text
fri_round0_drain_diag expand_evaluate_ntt domain=1048576 sum_ms=27263 count=32
merkle_drain_diag hash_rows rows=65536 cols=64 sum_ms=107 count=32
merkle_drain_diag hash_fold_chain rows=65536 cols=64 sum_ms=140 count=32
```

Representative first call:

```text
fri_round0_drain_diag expand_evaluate_ntt domain=1048576 elapsed_ms=863
merkle_drain_diag hash_rows rows=65536 cols=64 elapsed_ms=3
merkle_drain_diag hash_fold_chain rows=65536 cols=64 elapsed_ms=4
fri_prove round=0 merkle_new rows=65536 cols=64 elapsed_ms=10
```

## Interpretation

SP7ch's `hash_rows` attribution included queued `batch_expand_into_evaluate_ntt_async` work. Once the NTT path is drained before Merkle construction, actual `rows=65536, cols=64` Merkle row hashing is only about `107 ms` total across 32 calls, and fold-chain is about `140 ms` total. The material xgboost blocker is therefore the WebGPU `batch_expand_into_evaluate_ntt` path, not Poseidon2 row hashing.

Near-term implication: do not spend more immediate work on Poseidon2 row-hash shader tweaks. The next wall-time lever should target WebGPU NTT/expand/evaluate work or another measured large bucket such as `finalize_async eval_u_readback` / `check_group`.

## Cleanup

- Temporary `fri_round0_drain_diag` and `merkle_drain_diag` instrumentation removed.
- `rg -n "fri_round0_drain_diag|merkle_drain_diag" risc0/zkp/src/prove/fri.rs risc0/zkp/src/prove/merkle.rs risc0` found no source matches.
- `git diff --check` passed.
- Post-revert compile passed in `4m50s`: `iter6d_g_replace_xgboost --no-run`.
