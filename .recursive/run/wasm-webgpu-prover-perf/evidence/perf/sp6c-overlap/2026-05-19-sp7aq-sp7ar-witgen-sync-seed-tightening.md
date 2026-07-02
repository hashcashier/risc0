# SP7aq/SP7ar: Witgen Replacement Sync/Seed Tightening

Date: 2026-05-19

## Scope

Continue the RV32IM GPU-witgen replacement path for the bounded MISC0/ADD slice. Goal is correctness-first reduction of the synchronization tax exposed by SP7ap before considering more arms.

## SP7aq: partial CPU-shadow repair

RED:

```text
iter6d_g_replace_busy_loop_e2e_verify
assert_no_witgen_data_readback failed after receipt verification:
data_readbacks=1 data_readback_bytes=221249536
```

Diagnostic:

```text
DIFF_SUMMARY total_cells=55312384 gpu_wrote=14231946 cpu_wrote=33114247 both_match=14231946 mismatches=0 gpu_only=0 cpu_only=18882301 replace_cpu_only_nonzero=0 rows=262144 cols=211
```

Interpretation: for the replacement rows, every nonzero CPU cell is already GPU-written. Normal replacement mode only needs to repair the CPU shadow for the MISC0/ADD columns consumed by lookup replay and downstream CPU accumulation.

GREEN:

```text
BusyLoop po2=18:
  receipt verifies
  wall_ms=9778
  no source=data readback
  readback_bytes=138890304
  source=witgen_data_shadow_columns readbacks=1 readback_bytes=138412032
  source=data upload_bytes=576716800
  cpu_fallbacks=0 cpu_only_ops=0

KeccakUnion(1):
  receipt verifies
  wall_ms=103681
  segments=4 pending_keccaks=9 assumptions=1
  no source=data readback
  readback_bytes=494626056
  source=witgen_data_shadow_columns readbacks=4 readback_bytes=484442112
  source=data upload_bytes=5550637056
  cpu_fallbacks=0 cpu_only_ops=0

xgboost:
  receipt verifies
  wall_ms=104650
  segments=11
  journal=30.528042544062632
  no source=data readback
  source=witgen_data_shadow_columns readbacks=11 readback_bytes=1522532352
  source=data upload_bytes=7686062080
  cpu_fallbacks=0 cpu_only_ops=0
```

Decision: correctness accepted as candidate/offload infrastructure, but not a production performance win. xgboost remains slower than SP7an default mean `99692 ms`.

## SP7ar: GPU INVALID-fill + sparse injector seed

RED:

```text
iter6d_g_replace_busy_loop_e2e_verify
receipt verifies, then seed-upload gate fails:
source=data upload_bytes=576716800
max_data_upload_bytes=400000000
```

GREEN:

```text
BusyLoop po2=18:
  receipt verifies
  wall_ms=10059
  source=data upload_bytes=355467264
  source=data readbacks=0
  source=witgen_data_shadow_columns readbacks=1 readback_bytes=138412032
  op=witgen_data_invalid_fill gpu_dispatches=1
  op=scatter gpu_dispatches=1
  cpu_fallbacks=0 cpu_only_ops=0

KeccakUnion(1):
  receipt verifies
  wall_ms=106939
  segments=4 pending_keccaks=9 assumptions=1
  source=data upload_bytes=4776263680
  source=data readbacks=0
  source=witgen_data_shadow_columns readbacks=4 readback_bytes=484442112
  op=witgen_data_invalid_fill gpu_dispatches=4
  op=scatter gpu_dispatches=4
  cpu_fallbacks=0 cpu_only_ops=0

xgboost:
  one low-limit Chrome session was rejected by the high-limit guard and not counted
  high-limit rerun receipt verifies
  wall_ms=103220
  segments=11
  journal=30.528042544062632
  source=data upload_bytes=5252317184
  source=webgpu_scatter_offsets upload_bytes=93769148
  source=webgpu_scatter_values upload_bytes=93769148
  source=data readbacks=0
  source=witgen_data_shadow_columns readbacks=11 readback_bytes=1522532352
  op=witgen_data_invalid_fill gpu_dispatches=11
  op=scatter gpu_dispatches=11
  cpu_fallbacks=0 cpu_only_ops=0
```

Comparison:

```text
SP7ap xgboost replacement: source=data upload_bytes=7686062080, wall_ms=104345
SP7aq xgboost partial shadow sync: source=data upload_bytes=7686062080, wall_ms=104650
SP7ar xgboost GPU seed: source=data upload_bytes=5252317184, wall_ms=103220
SP7an default xgboost mean: wall_ms=99692
```

Decision: candidate retained because it removes one full pre-witgen `data` upload per RV32IM segment and improves the replacement experiment versus SP7aq/SP7ap. It is still not production-enabled and does not count as accepted wall-time gain because xgboost remains about 3.5 s slower than the default SP7an baseline.

Next highest-value witgen target: stop repairing all rows in the first 132 data columns. A packed row-sparse GPU readback for only the replacement cycle list should remove most of the remaining `witgen_data_shadow_columns` readback and should also make the GPU injector scatter unnecessary for non-replaced rows.
