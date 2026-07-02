# SP7ce Merkle Split Timers Reverted

Date: 2026-05-20

## Decision

Rejected as misleading diagnostic instrumentation and reverted.

The temporary patch added non-active `WebGpuStageTimer::new(...)` timers around:

- `MerkleTreeProver<WebGpuHal>::new_async` `hash_rows_async`
- `MerkleTreeProver<WebGpuHal>::new_async` `hash_fold_chain_async`
- `MerkleTreeProver<WebGpuHal>::new_committed_async` `hash_rows_async`
- `MerkleTreeProver<WebGpuHal>::new_committed_async` `hash_fold_chain_async`

The goal was to split the dominant xgboost bucket:

```text
fri_prove round=0 merkle_new rows=65536 cols=64 ~= 27549 ms across 32 calls
```

## Validation

Candidate compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
Finished release profile in 4m52s
```

xgboost browser proof e2e:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
test tests::iter6d_g_replace_xgboost ... ok
test result: ok. 1 passed; 0 failed; 137 filtered out; finished in 90.16s
```

xgboost correctness/perf summary:

```text
wall_ms=89923
gpu_active_ms=56685
gpu_idle_ratio=0.370
raw_compute_dispatches=11466
queue_submits=2718
upload_bytes=3104793948
readback_bytes=7488592
cpu_fallbacks=0
cpu_only_ops=0
```

Timer output was not useful:

```text
finalize_async fri_prove = 29409 ms
fri_prove round=0 merkle_new rows=65536 cols=64 = 27534 ms
merkle_new hash_rows rows=65536 cols=64 = 0 ms across 32 calls
merkle_new hash_fold_chain rows=65536 cols=64 layers=16 = 0 ms across 32 calls
```

## Finding

The added timers measured enqueue time, not GPU execution time. `hash_rows_async` and `hash_fold_chain_async` queue work and return before the GPU work is drained. The large `merkle_new` time is paid later when root/top readback forces the queued work to complete.

Retaining these timers would imply that Merkle hashing is free, which is false. Useful split profiling would need a different mechanism, such as a controlled diagnostic that intentionally drains after `hash_rows` and after `hash_fold_chain`, or actual GPU timestamps if available.

Post-revert compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
Finished release profile in 4m50s
```

## Conclusion

Do not repeat non-active enqueue timers for async Merkle substage profiling. They are correctness-clean but misleading for performance decisions.
