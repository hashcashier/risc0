# SP7eg - Sparse Zeroize Short-Gap Coalescing Rejected

Date: 2026-05-21

## Candidate

Coalesce one-cell zero/`INVALID` gaps inside sparse zeroize uploads by writing
explicit zero filler values. The goal was to reduce the large
`webgpu_zeroize_sparse_ranges` metadata and per-range GPU work seen on
KeccakUnion and xgboost without changing valid-or-zero semantics.

## RED Evidence

Focused browser HAL parity test:

```text
cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_core_gpu_results_match_cpu -- --nocapture
```

Log: `/tmp/sp7eg-sparse-short-gap-red.log`

Expected failure:

```text
webgpu_zeroize_sparse_upload name=data values=1536 ranges=96 sparse_bytes=6928 dense_bytes=8192
assertion failed: short-gap sparse zeroize should coalesce one-cell invalid gaps into a single range
left: 768
right: 8
```

## Focused GREEN

The broad implementation coalesced short gaps in the shared sparse upload plan.
Focused browser HAL parity passed:

Log: `/tmp/sp7eg-sparse-short-gap-green.log`

```text
webgpu_zeroize_sparse_upload name=data values=1631 ranges=1 sparse_bytes=6548 dense_bytes=8192
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
```

After the first representative failure, the candidate was narrowed so
zero-default upload and copy-slice priming kept the old exact sparse plan while
`eltwise_zeroize_elem` still coalesced short gaps. Focused parity still passed:

Log: `/tmp/sp7eg-sparse-short-gap-green2.log`

```text
webgpu_zeroize_sparse_upload name=data values=1631 ranges=1 sparse_bytes=6548 dense_bytes=8192
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
```

## Representative E2E Failure

The broad candidate failed before acceptance:

Log: `/tmp/sp7eg-sparse-short-gap-busy-keccak.log`

```text
browser-prove:async-prove-error name=multi_test/busy_loop_po2_18_default_representative err=verify segment index=0
Caused by:
    verification indicates proof is invalid
```

The narrowed candidate still failed the representative gate:

Log: `/tmp/sp7eg-sparse-short-gap-busy-keccak2.log`

```text
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
multi_test/busy_loop_po2_18_default_representative: wall_ms=6117.0 gpu_active_ms=3804.0
multi_test/busy_loop_po2_18_default_representative: cpu_fallbacks=0 cpu_only_ops=0
multi_test/keccak_union_default_representative: elapsed_ms=4223
multi_test/keccak_union_default_representative: cpu_fallbacks=0 cpu_only_ops=0
browser-prove:async-prove-error name=multi_test/keccak_union_default_representative err=verify segment index=0
Caused by:
    verification indicates proof is invalid
```

This is a correctness rejection, not a fallback issue.

## Post-Revert Verification

The candidate code and focused assertion were removed. Marker sweep for
`short-gap`, `short_gap`, `coalesce_short`, `SPARSE_ZERO_UPLOAD_COALESCE`, and
`sparse short` was clean. Formatting and diff hygiene passed:

```text
cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml
cargo fmt --check --manifest-path risc0/zkp/Cargo.toml
git diff --check -- examples/browser-prove/src/lib.rs risc0/zkp/src/hal/webgpu.rs
```

Post-revert representative proof gate:

Log: `/tmp/sp7eg-sparse-short-gap-post-revert-busy-keccak.log`

```text
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
multi_test/busy_loop_po2_18_default_representative: wall_ms=6074.0 gpu_active_ms=3830.0
multi_test/busy_loop_po2_18_default_representative: cpu_fallbacks=0 cpu_only_ops=0
multi_test/keccak_union_default_representative: wall_ms=90415.0 gpu_active_ms=59611.0
multi_test/keccak_union_default_representative: cpu_fallbacks=0 cpu_only_ops=0
test result: ok. 1 passed; 0 failed; 151 filtered out; finished in 97.22s
```

## Decision

Reject and revert.

Accepted wall-time gain: `0`.

Do not retry sparse zeroize short-gap coalescing by writing explicit zero
fillers across `INVALID` gaps. Focused HAL parity can pass, but representative
proof generation shows the change can alter proof semantics. Future sparse
upload work must preserve exact skipped-cell behavior or use a proof-level diff
gate before performance timing.
