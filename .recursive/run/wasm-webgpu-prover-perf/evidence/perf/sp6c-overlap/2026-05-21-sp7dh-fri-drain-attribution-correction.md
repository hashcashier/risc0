# SP7dh FRI Drain Attribution Correction

Date: 2026-05-21

## Intent

Correct SP7dg's FRI attribution. SP7dg drained inside Merkle construction, but FRI round 0 queues `batch_expand_into_evaluate_ntt_async` immediately before creating the Merkle tree. That meant `merkle fri_round0 drain_after_hash_rows` could still include the queued FRI NTT expansion.

This is diagnostic-only. The extra drains are controlled by the existing default-off `set_poly_group_drain_diagnostic_enabled(true)` flag.

## Change

Retained two additional default-off FRI drain labels:

- `fri_prove round=<n> drain_after_expand_evaluate_ntt domain=<domain>`
- `fri_prove round=<n> drain_after_fri_fold count_out=<count>`

The focused browser test `webgpu_prover_fri_drain_diagnostic_splits_round_work` proves these labels appear around a full browser proof and that the proof still verifies with zero CPU fallback/CPU-only counters.

## TDD Evidence

Corrected RED:

```text
/tmp/sp7dh-fri-drain-red2.log
webgpu_prover_fri_drain_diagnostic_splits_round_work
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
prove_session_async wall_ms=4279.0 gpu_active_ms=1741.0 gpu_idle_ratio=0.593
raw_compute_dispatches=548 queue_submits=167 cpu_fallbacks=0 cpu_only_ops=0
panicked at browser-prove/src/lib.rs:1610:9:
FRI drain diagnostics should split round-0 NTT work before Merkle hashing
```

GREEN:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_prover_fri_drain_diagnostic_splits_round_work --no-run
Finished release target in 4m 55s
```

Focused browser GREEN:

```text
/tmp/sp7dh-fri-drain-green.log
webgpu_prover_fri_drain_diagnostic_splits_round_work ... ok
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
prove_session_async wall_ms=4588.0 gpu_active_ms=1810.0 gpu_idle_ratio=0.605
raw_compute_dispatches=548 queue_submits=167 cpu_fallbacks=0 cpu_only_ops=0
fri_prove round=0 drain_after_expand_evaluate_ntt domain=1048576 elapsed_us=842000
merkle fri_round0 drain_after_hash_rows rows=65536 cols=64 elapsed_us=3000
fri_prove round=0 drain_after_fri_fold count_out=16384 elapsed_us=2000
```

## Attribution Run

For one xgboost attribution pass only, `xgboost_succinct_receipt_verifies` was temporarily wrapped with the diagnostic flag. That temporary enablement was removed after the run; the retained tests remain default-off unless they enable the flag locally.

Log: `/tmp/sp7dh-xgboost-fri-drain-attribution.log`

```text
xgboost_succinct_receipt_verifies ... ok
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
prove_session_async wall_ms=80107.0 gpu_active_ms=46985.0 gpu_idle_ratio=0.413
raw_compute_dispatches=9674 queue_submits=2718 cpu_fallbacks=0 cpu_only_ops=0
upload_bytes=3114030844 readback_bytes=7488592
```

Top attributed totals from that xgboost proof:

```text
30143 ms  32  finalize_async fri_prove
27231 ms  32  fri_prove round=0 drain_after_expand_evaluate_ntt domain=1048576
 9101 ms  32  poly_group check drain_after_batch_expand_into_evaluate_ntt count=16 size=262144 domain=1048576
  131 ms  32  merkle fri_round0 drain_after_hash_rows rows=65536 cols=64
   78 ms  32  merkle fri_round0 root_top_readback rows=65536 top_size=32
   76 ms  32  merkle check root_top_readback rows=1048576 top_size=32
   71 ms  32  fri_prove round=0 drain_after_fri_fold count_out=16384
```

## Default-Path Proof Gates

The first two default BusyLoop+KeccakUnion attempts were rejected before proof generation because they were launched from the worktree root and negotiated the invalid low-limit Chrome profile:

```text
/tmp/sp7dh-default-busy-keccak.log
/tmp/sp7dh-default-busy-keccak-retry.log
max_buffer_size=1073741824 max_storage_buffer_binding_size=1073741824 max_compute_workgroup_storage_size=32768
representative performance proof gates require high WebGPU limits
```

They are not counted as correctness or performance evidence. The valid default gates were rerun from `examples/browser-prove/`, so the local `webdriver.json` capabilities were applied.

BusyLoop + KeccakUnion log: `/tmp/sp7dh-default-busy-keccak.log`

```text
rv32im_default_representative_e2e_verify ... ok
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
BusyLoop: wall_ms=7217 gpu_active_ms=3198 raw_compute_dispatches=609 queue_submits=173 cpu_fallbacks=0 cpu_only_ops=0
KeccakUnion: wall_ms=91900 segments=4 pending_keccaks=9 assumptions=1 gpu_active_ms=60170 raw_compute_dispatches=10660 queue_submits=3075 cpu_fallbacks=0 cpu_only_ops=0
```

xgboost log: `/tmp/sp7dh-default-xgboost.log`

```text
xgboost_succinct_receipt_verifies ... ok
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
wall_ms=78096 segments=11 gpu_active_ms=45005 raw_compute_dispatches=9674 queue_submits=2718 cpu_fallbacks=0 cpu_only_ops=0
upload_bytes=3113998248 readback_bytes=7488592
```

Both valid default-path logs were checked for `drain_after` labels and contained none.

## Finding

SP7dg's apparent 27 s `fri_round0 hash_rows` wait was incomplete attribution. After draining the FRI NTT immediately before Merkle construction, the xgboost FRI round-0 `hash_rows rows=65536 cols=64` bucket falls to about 0.13 s total across 32 samples.

The dominant measured work is now both NTT expansion:

1. FRI round-0 `batch_expand_into_evaluate_ntt`, about 27.2 s xgboost ceiling.
2. Check-group `batch_expand_into_evaluate_ntt`, about 9.1 s xgboost ceiling.

## Decision

Accepted as diagnostic substrate. Accepted wall-time gain: 0.

Do not target Poseidon2 row hashing as the next immediate lever on the current evidence. Do not target iframe/multi-device workarounds as the next immediate lever. The next optimization work should target `batch_expand_into_evaluate_ntt` active time directly and must be retained only after representative browser proof gates verify BusyLoop, KeccakUnion, and xgboost correctness with zero fallback/CPU-only counters.
