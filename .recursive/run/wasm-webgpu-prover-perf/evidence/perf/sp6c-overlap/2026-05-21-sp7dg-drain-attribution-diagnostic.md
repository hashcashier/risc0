# SP7dg Drain Attribution Diagnostic

Date: 2026-05-21

## Intent

Attribute the large `root_top_readback` waits identified in SP7df to the queued GPU work that they were actually draining. This is diagnostic-only; explicit drains intentionally perturb wall time and must stay default-off.

## Change

Retained a default-off browser/WebGPU diagnostic flag:

- `set_poly_group_drain_diagnostic_enabled(bool)` toggles forced `hal.wait_idle().await?` drains.
- `PolyGroup::new_async` / `new_committed_async` can emit and wait at:
  - `drain_after_batch_expand_into_evaluate_ntt`
  - `drain_after_batch_bit_reverse`
- `MerkleTreeProver::new_async` / `new_committed_async` can emit and wait at:
  - `drain_after_hash_rows`
  - `drain_after_hash_fold`

Default behavior is unchanged unless the flag is enabled by a focused test or temporary attribution run.

## TDD Evidence

RED:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_prover_commit_group_drain_diagnostic_splits_queue_waits --no-run
error[E0432]: unresolved import `risc0_zkp::hal::webgpu::set_poly_group_drain_diagnostic_enabled`
```

GREEN:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_prover_commit_group_drain_diagnostic_splits_queue_waits --no-run
Finished release target in 4m 50s
```

Focused browser GREEN:

```text
webgpu_prover_commit_group_drain_diagnostic_splits_queue_waits ... ok
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
poly_group webgpu_drain_diag_commit_witness drain_after_batch_expand_into_evaluate_ntt ... elapsed_ms=25
poly_group webgpu_drain_diag_commit_witness drain_after_batch_bit_reverse ... elapsed_ms=4
merkle webgpu_drain_diag_commit_witness drain_after_hash_rows ... elapsed_ms=2
merkle webgpu_drain_diag_commit_witness drain_after_hash_fold ... elapsed_ms=2
```

## Attribution Run

For one xgboost attribution pass only, `xgboost_succinct_receipt_verifies` was temporarily wrapped with the diagnostic flag. That temporary enablement was removed after the run; the retained test remains on the default path.

Log: `/tmp/sp7dg-xgboost-drain-attribution.log`

```text
xgboost_succinct_receipt_verifies ... ok
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
prove_session_async wall_ms=79997.0 gpu_active_ms=46831.0 gpu_idle_ratio=0.415
segments=11 user_cycles=2294916 total_cycles=2883584
raw_compute_dispatches=9674 queue_submits=2718 cpu_fallbacks=0 cpu_only_ops=0
upload_bytes=3113946408 readback_bytes=7488592
```

## Default-Path Proof Gates

After removing the temporary xgboost diagnostic enablement, the retained default-off code was revalidated on the representative browser proof gates.

BusyLoop + KeccakUnion log: `/tmp/sp7dg-default-busy-keccak.log`

```text
rv32im_default_representative_e2e_verify ... ok
BusyLoop: wall_ms=7327 gpu_active_ms=3205 raw_compute_dispatches=609 queue_submits=173 cpu_fallbacks=0 cpu_only_ops=0
KeccakUnion: wall_ms=91790 segments=4 pending_keccaks=9 assumptions=1 gpu_active_ms=59919 raw_compute_dispatches=10660 queue_submits=3075 cpu_fallbacks=0 cpu_only_ops=0
```

xgboost log: `/tmp/sp7dg-default-xgboost.log`

```text
xgboost_succinct_receipt_verifies ... ok
wall_ms=78033 segments=11 gpu_active_ms=45142 raw_compute_dispatches=9674 queue_submits=2718 cpu_fallbacks=0 cpu_only_ops=0
upload_bytes=3113829152 readback_bytes=7488592
```

Both default-path logs were checked for `drain_after` labels and contained none.

## Finding

The SP7df "readback wait" is mostly prior GPU work becoming visible at the first synchronization point.

Top attributed totals from the xgboost proof:

```text
29738 ms  32  finalize_async fri_prove
27263 ms  32  merkle fri_round0 drain_after_hash_rows rows=65536 cols=64
 9983 ms  32  finalize_async check_group
 9187 ms  32  poly_group check drain_after_batch_expand_into_evaluate_ntt count=16 size=262144 domain=1048576
```

The readback payloads themselves are small once the queue is explicitly drained earlier:

```text
  88 ms  32  merkle check root_top_readback rows=1048576 top_size=32
  82 ms  32  merkle fri_round0 root_top_readback rows=65536 top_size=32
```

The `check_group` split now points at NTT expansion, not Merkle root readback:

```text
9187 ms  poly_group check drain_after_batch_expand_into_evaluate_ntt
  74 ms  poly_group check drain_after_batch_bit_reverse
 161 ms  merkle check drain_after_hash_rows
 226 ms  merkle check drain_after_hash_fold
  88 ms  merkle check root_top_readback
```

The FRI round-0 split points at 64-column row hashing:

```text
27263 ms  merkle fri_round0 drain_after_hash_rows rows=65536 cols=64
  153 ms  merkle fri_round0 drain_after_hash_fold rows=65536 layers=16
   82 ms  merkle fri_round0 root_top_readback rows=65536 top_size=32
```

## Decision

Accepted as diagnostic substrate. Accepted wall-time gain: 0.

Immediate optimization targets should now be:

1. `merkle fri_round0 hash_rows rows=65536 cols=64`, with a measured ceiling around 27 s in xgboost.
2. `poly_group check batch_expand_into_evaluate_ntt count=16`, with a measured ceiling around 9 s in xgboost.

Do not spend near-term effort on root/top readback payload mechanics, iframe workarounds, or dispatch-count-only NTT reductions unless a new e2e proof run shows they reduce one of the attributed buckets.
