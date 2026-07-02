# SP7cd Batch Evaluate Chunk Size 2048 Rejected

Date: 2026-05-20

## Decision

Rejected and reverted.

The candidate changed only:

```rust
const WEBGPU_BATCH_EVALUATE_CHUNK_SIZE: usize = 1024 -> 2048;
```

The target was the xgboost `finalize_async eval_u_readback` drain bucket, which includes the outstanding chunked `batch_evaluate_any` GPU work before the coalesced `out` readback.

## Validation

Candidate compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
Finished release profile in 4m51s
```

BusyLoop + KeccakUnion browser proof e2e:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
test tests::iter6d_g_replace_busy_loop_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 137 filtered out; finished in 109.90s
```

BusyLoop:

```text
wall_ms=8109
gpu_active_ms=3991
gpu_idle_ratio=0.508
raw_compute_dispatches=721
queue_submits=173
upload_bytes=209732396
readback_bytes=478272
cpu_fallbacks=0
cpu_only_ops=0
```

KeccakUnion:

```text
wall_ms=101474
segments=4
gpu_active_ms=69054
gpu_idle_ratio=0.319
raw_compute_dispatches=12716
queue_submits=3075
upload_bytes=2991823252
readback_bytes=10183944
cpu_fallbacks=0
cpu_only_ops=0
```

xgboost browser proof e2e:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
test tests::iter6d_g_replace_xgboost ... ok
test result: ok. 1 passed; 0 failed; 137 filtered out; finished in 89.88s
```

xgboost:

```text
wall_ms=89645
gpu_active_ms=56513
gpu_idle_ratio=0.370
raw_compute_dispatches=11466
queue_submits=2718
upload_bytes=3104689764
readback_bytes=7488592
cpu_fallbacks=0
cpu_only_ops=0
```

Targeted bucket comparison against SP7cc:

```text
finalize_async eval_u_readback: 10731 -> 10772 ms
```

Total xgboost wall moved only:

```text
89769 -> 89645 ms
```

That `-124 ms` single-run wall movement is noise-level and not supported by the targeted bucket, which slightly regressed.

Post-revert compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
Finished release profile in 4m51s
```

## Conclusion

Do not retain the `2048` chunk size. The candidate was correctness-clean, but it did not improve the measured `batch_evaluate_any` drain path and did not provide meaningful wall-time evidence. The accepted value remains `1024`.
