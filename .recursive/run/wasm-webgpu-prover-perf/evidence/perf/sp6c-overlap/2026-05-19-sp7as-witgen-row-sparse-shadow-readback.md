# SP7as: Witgen Row-Sparse Shadow Readback

Date: 2026-05-19

## Scope

Continue the bounded RV32IM MISC0/ADD GPU-witgen replacement path by removing the dense CPU-shadow repair introduced in SP7aq/SP7ar.

SP7ar still read back the first 132 witness columns for every row after GPU replacement. That preserved correctness, but xgboost paid `1522532352` bytes of `witgen_data_shadow_columns` readback and remained slower than the default SP7an baseline.

## RED

Added representative proof gates requiring replacement mode to:

- reject all dense `witgen_data_shadow_columns` readbacks;
- require sparse `witgen_data_shadow_rows` readback;
- cap sparse readback bytes per workload.

The first BusyLoop proof generated and verified a receipt, then failed the new gate:

```text
iter6d_g_replace_busy_loop_e2e_verify
source=witgen_data_shadow_columns readbacks=1 readback_bytes=138412032
dense_readbacks must be 0
```

## GREEN

Implementation:

- Added a WebGPU pack kernel that copies selected row indices from a column prefix into a contiguous packed buffer.
- Added `WebGpuBuffer::sync_gpu_column_prefix_rows_to_cpu_unchecked`.
- Replaced dense MISC0/ADD shadow-column repair with row-sparse repair for `major=0, minor=0` replacement cycles only.

Representative browser proof generation passed with high WebGPU limits, real receipt verification, zero CPU fallback/CPU-only operations, and no dense shadow-column readback.

```text
BusyLoop po2=18:
  receipt verifies
  wall_ms=9719
  gpu_active_ms=4376
  gpu_idle_ratio=0.550
  readback_bytes=6436752
  source=witgen_data_shadow_columns readbacks=0
  source=witgen_data_shadow_rows asserted <= 80000000 bytes
  source=data upload_bytes=355467264
  cpu_fallbacks=0 cpu_only_ops=0

KeccakUnion(1):
  receipt verifies
  wall_ms=106368
  segments=4 pending_keccaks=9 assumptions=1
  readbacks=413
  readback_bytes=14198856
  source=witgen_data_shadow_columns readbacks=0
  source=witgen_data_shadow_rows readbacks=4 readback_bytes=4014912
  source=webgpu_sparse_column_prefix_rows uploads=4 upload_bytes=30416
  source=data upload_bytes=4776263680
  cpu_fallbacks=0 cpu_only_ops=0

xgboost:
  receipt verifies
  wall_ms=103134
  gpu_active_ms=58021
  gpu_idle_ratio=0.437
  segments=11
  journal=30.528042544062632
  source=witgen_data_shadow_columns readbacks=0
  source=witgen_data_shadow_rows readbacks=11 readback_bytes=41567328
  readbacks=363
  readback_bytes=49055920
  source=webgpu_sparse_column_prefix_rows uploads=11 upload_bytes=314904
  source=data upload_bytes=5252317184
  source=webgpu_scatter_offsets upload_bytes=93769148
  source=webgpu_scatter_values upload_bytes=93769148
  op=witgen_data_invalid_fill gpu_dispatches=11
  op=scatter gpu_dispatches=11
  cpu_fallbacks=0 cpu_only_ops=0
```

## Comparison

```text
SP7ar xgboost replacement:
  wall_ms=103220
  source=witgen_data_shadow_columns readback_bytes=1522532352
  total readback_bytes=1530020944

SP7as xgboost replacement:
  wall_ms=103134
  source=witgen_data_shadow_rows readback_bytes=41567328
  total readback_bytes=49055920

SP7an default xgboost mean:
  wall_ms=99692
```

SP7as removes about `1480955024` xgboost shadow-readback bytes versus SP7ar and reduces total xgboost readback bytes by about `1480965024`. The observed single-run wall movement versus SP7ar is only `-86 ms`, and replacement remains about `3442 ms` slower than the SP7an default xgboost mean.

## Decision

Correctness accepted for the bounded MISC0/ADD replacement path. Performance is not accepted for production enablement because representative xgboost wall time still loses to the default path.

This narrows the next immediate witgen target: remove the remaining replacement-only seeding/scatter overhead or avoid the final large `source=data` upload by making downstream accumulation consume GPU-authoritative witness data. Do not add more witgen arms until the bounded replacement path beats the default representative wall baseline.
