# SP7cf Poseidon2 Embedded WGSL Constants Rejected

Date: 2026-05-20

## Decision

Rejected and reverted before xgboost.

The candidate embedded Poseidon2 `ROUND_CONSTANTS` and `M_INT_DIAG_HZN` into the WGSL source as constant arrays and replaced shader storage-buffer reads with constant-array indexing. The existing bindings and buffers were left intact for a low-risk first pass; the intended change was only shader-side constant access in `hash_rows` and `hash_fold`.

This targeted the dominant Merkle bucket:

```text
SP7cc xgboost fri_prove round=0 merkle_new rows=65536 cols=64 = 27549 ms across 32 calls
```

## Validation

Candidate compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
Finished release profile in 4m53s
```

BusyLoop + KeccakUnion browser proof e2e:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
test tests::iter6d_g_replace_busy_loop_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 137 filtered out; finished in 109.91s
```

BusyLoop:

```text
wall_ms=9004
gpu_active_ms=4254
gpu_idle_ratio=0.528
raw_compute_dispatches=721
queue_submits=173
upload_bytes=209756624
readback_bytes=478272
cpu_fallbacks=0
cpu_only_ops=0
```

KeccakUnion:

```text
wall_ms=100601
segments=4
pending_keccaks=9
assumptions=1
gpu_active_ms=68794
gpu_idle_ratio=0.316
raw_compute_dispatches=12716
queue_submits=3075
upload_bytes=2991717028
readback_bytes=10183944
cpu_fallbacks=0
cpu_only_ops=0
```

## Performance

Compared with SP7cc:

- BusyLoop: `8159 -> 9004 ms` (`+845 ms`, about `+10.4%`).
- KeccakUnion: `101378 -> 100601 ms` (`-777 ms`, about `-0.8%`).
- Large KeccakUnion Merkle sample aggregate moved directionally down (`25815 -> 25697 ms` in the candidate log), but BusyLoop's first-proof regression fails the representative wall gate.

The most likely cause is larger Poseidon2 WGSL source / constant-array handling increasing first-proof shader/pipeline cost or reducing short-workload shader efficiency. The change might help a long Merkle-heavy workload, but the user-directed gate requires representative e2e workloads, and this fails BusyLoop before xgboost.

Post-revert compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
Finished release profile in 4m52s
```

## Conclusion

Do not retain embedded Poseidon2 WGSL constants in this shape. Any future version would need to avoid the BusyLoop first-proof regression, likely by separating compile-heavy experimental kernels from the default first-proof path or proving a much larger long-workload win while preserving the representative gate.
