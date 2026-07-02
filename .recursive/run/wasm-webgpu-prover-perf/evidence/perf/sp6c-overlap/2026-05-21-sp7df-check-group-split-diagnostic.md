# SP7df Check-Group Split Diagnostic

Date: 2026-05-21

## Intent

Decompose the previously opaque `finalize_async check_group` browser WebGPU timer before attempting another optimization. Prior xgboost evidence showed about 7-10 s in `check_group`, but the timer wrapped `PolyGroup::new_committed_async` as one bucket.

## Change

Retained diagnostic-only stage accounting:

- `WebGpuDiagnostics.stages` now records HAL-scoped `WebGpuStageTimer::new_for` / `new_active_for` labels with elapsed microseconds.
- `PolyGroup::new_async` / `new_committed_async` now emit split labels for `batch_expand_into_evaluate_ntt` and `batch_bit_reverse`.
- `MerkleTreeProver::new_async` / `new_committed_async` now emit split labels for `hash_rows`, `hash_fold`, and `root[_top]_readback`.
- Browser diagnostics now print `browser-prove:webgpu-stage ...` lines.

The split timers use `new_for` for sub-stage diagnostics, so they do not add to aggregate `gpu_active_ms` and do not double-count existing outer active timers.

## TDD Evidence

RED:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_prover_commit_group_fuses_copy_into_interpolate --no-run
error[E0609]: no field `stages` on type `WebGpuDiagnostics`
```

GREEN:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_prover_commit_group_fuses_copy_into_interpolate --no-run
Finished release target in 4m 53s
```

Focused browser GREEN:

```text
webgpu_prover_commit_group_fuses_copy_into_interpolate ... ok
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
poly_group webgpu_fused_commit_witness batch_expand_into_evaluate_ntt ... elapsed_ms=1
merkle webgpu_fused_commit_witness root_top_readback ... elapsed_ms=27
```

## Representative Proof Gates

BusyLoop + KeccakUnion log: `/tmp/sp7df-busy-keccak-checkgroup-split.log`

```text
rv32im_default_representative_e2e_verify ... ok
BusyLoop: wall_ms=7269 gpu_active_ms=3214 raw_compute_dispatches=609 queue_submits=173 cpu_fallbacks=0 cpu_only_ops=0
KeccakUnion: wall_ms=92136 segments=4 gpu_active_ms=60094 raw_compute_dispatches=10660 queue_submits=3075 cpu_fallbacks=0 cpu_only_ops=0
```

xgboost log: `/tmp/sp7df-xgboost-checkgroup-split.log`

```text
xgboost_succinct_receipt_verifies ... ok
wall_ms=78258 segments=11 gpu_active_ms=45000 raw_compute_dispatches=9674 queue_submits=2718 cpu_fallbacks=0 cpu_only_ops=0
```

## Finding

`check_group` is not primarily synchronous CPU work, nor a visible readback-byte problem. The root/top readback is acting as the queue drain for the asynchronous check-group construction.

Observed split totals:

```text
BusyLoop check_group: samples=2 sum_ms=786; merkle check root_top_readback sum_ms=760
KeccakUnion check_group: samples=38 sum_ms=27755; merkle check root_top_readback sum_ms=27539
xgboost check_group: samples=32 sum_ms=9611; merkle check root_top_readback sum_ms=9389
```

Other xgboost check-group split labels were effectively zero in the synchronous timer:

```text
poly_group check batch_expand_into_evaluate_ntt sum_ms=2
poly_group check batch_bit_reverse sum_ms=2
merkle check hash_rows sum_ms=3
merkle check hash_fold sum_ms=0
```

The same pattern explains the old "FRI round0 drain" bucket:

```text
xgboost merkle fri_round0 root_top_readback: samples=32 sum_ms=27417
```

## Decision

Accepted as diagnostic substrate. Accepted wall-time gain: 0.

Next optimization work should not target the tiny root/top payload itself. The profitable next step is an opt-in drain attribution run that inserts explicit queue drains after NTT expansion, bit reversal, row hashing, and fold-chain submission, then reverts or leaves it default-off. That will identify whether the hidden queue work is still NTT/expand traffic, Merkle hashing, or a broader queued-work ordering issue.
