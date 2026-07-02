# SP7aw: MISC0 Bitwise GPU-Witgen Replacement

Date: 2026-05-19

## Scope

Broaden the opt-in RV32IM GPU-witgen replacement path from MISC0
`minor in {0,1,7}` to `minor in {0,1,2,3,4,7}` by adding XOR, OR, and
AND chunk kernels. This remains correctness-gated by browser WebGPU e2e
proof generation.

## RED

The first generated `exec_misc0_chunk{2,3,4}_delta.wgsl` files included
baseline field helpers and failed WebGPU module validation with duplicate
definitions. Regenerating deltas as baseline-excluding closures fixed the
Tint validation issue.

The first e2e proof with bitwise replacement compiled and dispatched but
failed BusyLoop segment verification. A focused replace-diff run showed:

```text
REPLACE_FINAL_SUMMARY total_cells=55312384 mismatches=0 rows=262144 cols=211
```

That ruled out incorrect GPU witness cells and localized the failure to
normal replacement-mode sparse shadow sync. The old sparse sync read only
the arithmetic-safe prefix `[0,132)`, but bitwise rows also write shared
`ToBits_16` columns through `[132,196)`. Normal replacement then uploaded
the stale CPU shadow and overwrote correct GPU cells.

## GREEN

Implementation:

- generated and vendored `exec_misc0_chunk2_delta.wgsl`,
  `exec_misc0_chunk3_delta.wgsl`, and `exec_misc0_chunk4_delta.wgsl`;
- generalized MISC0 extra-kernel caching/dispatch from chunk7 to chunks
  `{2,3,4,7}`;
- short-circuits only MISC0 `minor in {0,1,2,3,4,7}`;
- splits replacement-row sparse readback into arithmetic rows with prefix
  `132` and bitwise rows with prefix `196`.

The intermediate broad `196`-column readback was correctness-positive but
performance-negative on xgboost:

```text
xgboost wall_ms=103526
source=witgen_data_shadow_rows readback_bytes=592299456
```

The final split readback reduced xgboost shadow readback to `406998464`
bytes while preserving receipt verification.

## Browser E2E Proofs

All accepted runs used high WebGPU limits:

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
  wall_ms=11671
  source=witgen_data_shadow_rows readback_bytes≈36030192
  cpu_fallbacks=0 cpu_only_ops=0

KeccakUnion(1):
  receipt verifies
  wall_ms=103915
  segments=4 pending_keccaks=9 assumptions=1
  source=witgen_data_shadow_rows readback_bytes=67004128
  source=data upload_bytes=4776263680
  cpu_fallbacks=0 cpu_only_ops=0
```

xgboost representative gate:

```text
iter6d_g_replace_xgboost ... ok

xgboost:
  receipt verifies
  wall_ms=103320
  segments=11
  journal=30.528042544062632
  source=witgen_data_shadow_rows readback_bytes=406998464
  source=iter6d_g_arm_cycle_list upload_bytes=3021936
  source=data upload_bytes=5252317184
  raw_compute_dispatches=11380
  queue_submits=2676
  cpu_fallbacks=0 cpu_only_ops=0
```

## Comparison

```text
SP7av replacement:
  BusyLoop wall_ms=8942
  KeccakUnion wall_ms=103877
  xgboost wall_ms=101668
  xgboost witgen_data_shadow_rows=382183296

SP7aw bitwise replacement:
  BusyLoop wall_ms=11671 (+2729 ms)
  KeccakUnion wall_ms=103915 (+38 ms)
  xgboost wall_ms=103320 (+1652 ms)
  xgboost witgen_data_shadow_rows=406998464 (+24815168 bytes)

SP7an default xgboost mean:
  wall_ms=99692
```

## Decision

Correctness accepted for the opt-in replacement path: all representative
browser WebGPU proof-generation gates pass, including BusyLoop,
KeccakUnion(1), and xgboost with journal verification.

Performance is rejected for default promotion. Adding more MISC0 chunk
coverage does not beat SP7av or the SP7an default mean because the path is
still dominated by full `source=data` upload and CPU accumulation
dependencies. Do not keep expanding same-pattern MISC0 chunks as the next
priority; the immediate material target remains eliminating the dense
post-witgen `source=data` upload or making more of the downstream
accumulation path consume GPU-resident data directly.
