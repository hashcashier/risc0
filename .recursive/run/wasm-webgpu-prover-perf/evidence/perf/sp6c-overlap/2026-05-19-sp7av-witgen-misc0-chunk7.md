# SP7av: MISC0 Chunk7 GPU-Witgen Replacement

Date: 2026-05-19

## Scope

Broaden the opt-in RV32IM GPU-witgen replacement path from MISC0
`minor=0` to MISC0 `minor in {0,1,7}`. This adds ADDI (`chunk7`) while
keeping correctness gated by browser WebGPU receipt generation.

## RED

The first broad gate required more than 20 MB of sparse replacement-row
shadow repair. Current MISC0 ADD-only replacement generated and verified a
BusyLoop receipt, then failed:

```text
source=witgen_data_shadow_rows readback_bytes=5958480
min_sparse_readback_bytes=20000000
```

Enabling `minor=7` without a matching kernel failed before receipt
generation:

```text
rv32im_witgen panic: read of unset value at row 18320, col 40
```

Root cause: only chunk0/chunk1 kernels were dispatched; ADDI needs the
MISC0 chunk7 witness cells.

## GREEN

Implementation:

- generated and vendored a small `exec_misc0_chunk7_delta.wgsl`;
- added an arm0 chunk7 WebGPU kernel cache and dispatch;
- short-circuits only MISC0 `minor in {0,1,7}`;
- filters replacement-mode per-arm cycle lists to replacement rows only;
- joins on-demand arm0 chunk0/chunk1/chunk7 pipeline compiles instead of
  awaiting them serially.

## Browser E2E Proofs

All runs used high WebGPU limits:

```text
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152
```

BusyLoop + KeccakUnion representative gate:

```text
iter6d_g_replace_busy_loop_e2e_verify ... ok

BusyLoop po2=18:
  receipt verifies
  wall_ms=8942
  pre_witgen_dispatch_ms=1135
  rv32im_witgen_ms=444
  source=witgen_data_shadow_rows readback_bytes=33634128
  source=iter6d_g_arm_cycle_list upload_bytes=254804
  source=data upload_bytes=355467264
  cpu_fallbacks=0 cpu_only_ops=0

KeccakUnion(1):
  receipt verifies
  wall_ms=103877
  segments=4 pending_keccaks=9 assumptions=1
  source=witgen_data_shadow_rows readback_bytes=66163680
  source=data upload_bytes=4776263680
  cpu_fallbacks=0 cpu_only_ops=0
```

xgboost representative gate:

```text
iter6d_g_replace_xgboost ... ok

xgboost:
  receipt verifies
  wall_ms=101668
  segments=11
  journal=30.528042544062632
  source=witgen_data_shadow_rows readback_bytes=382183296
  source=iter6d_g_arm_cycle_list upload_bytes=2895328
  source=data upload_bytes=5252317184
  raw_compute_dispatches=11353
  queue_submits=2638
  cpu_fallbacks=0 cpu_only_ops=0
```

## Comparison

```text
SP7at replacement:
  BusyLoop wall_ms=9393
  KeccakUnion wall_ms=103939
  xgboost wall_ms=103480
  xgboost witgen_data_shadow_rows=41568912

SP7av replacement:
  BusyLoop wall_ms=8942  (-451 ms)
  KeccakUnion wall_ms=103877 (-62 ms)
  xgboost wall_ms=101668 (-1812 ms)
  xgboost witgen_data_shadow_rows=382183296

SP7an default xgboost mean:
  wall_ms=99692
```

## Decision

Correctness accepted for the opt-in replacement path. Performance is a
real representative improvement versus SP7at replacement, especially
xgboost (`-1.812 s`), but it still does not beat the SP7an default mean.
Keep replacement opt-in; do not default-enable until the final
`source=data` upload or CPU accumulation dependency is removed.
