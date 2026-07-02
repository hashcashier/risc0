# SP7at: Witgen Invalid-Fill-Only Seed

Date: 2026-05-19

## Scope

Continue the bounded RV32IM MISC0/ADD GPU-witgen replacement path after SP7as row-sparse shadow repair.

SP7as proved the CPU shadow only needs replacement rows from the first 132 witness columns. That makes the replacement seed scatter suspect: the CPU shadow is already seeded by `allocate_buffers`, while the GPU MISC0/ADD replacement kernels should only need invalid-filled witness storage plus `shadow_init` and preflight buffers.

## RED

Added representative proof gates requiring replacement mode to elide:

- `webgpu_scatter_offsets` upload bytes;
- `webgpu_scatter_values` upload bytes;
- GPU `scatter` dispatches during replacement seed.

The first BusyLoop proof generated and verified a receipt, then failed the new gate:

```text
iter6d_g_replace_busy_loop_e2e_verify
source=webgpu_scatter_offsets upload_bytes=8963712
source=webgpu_scatter_values upload_bytes=8963712
op=scatter gpu_dispatches=1
```

## GREEN

Implementation:

- Replaced `WebGpuHal::init_invalid_and_scatter_elem` with `WebGpuHal::init_invalid_elem`.
- Changed `WebGpuCircuitHal::pre_witgen_dispatch_async` to seed replacement-mode GPU witness data with INVALID fill only.
- Kept the normal CPU-side injector scatter in `WitnessGenerator::allocate_buffers`; only the replacement GPU seed path changed.

Representative browser proof generation passed with high WebGPU limits, real receipt verification, zero CPU fallback/CPU-only operations, no dense shadow-column readback, and no replacement seed GPU scatter.

```text
BusyLoop po2=18:
  receipt verifies
  wall_ms=9393
  gpu_active_ms=4329
  gpu_idle_ratio=0.539
  source=data upload_bytes=355467264
  readback_bytes=6436752
  op=scatter gpu_dispatches=0 cpu_mirrors=1
  op=witgen_data_invalid_fill gpu_dispatches=1
  no webgpu_scatter_offsets upload source
  no webgpu_scatter_values upload source
  cpu_fallbacks=0 cpu_only_ops=0

KeccakUnion(1):
  receipt verifies
  wall_ms=103939
  segments=4 pending_keccaks=9 assumptions=1
  source=data upload_bytes=4776263680
  source=witgen_data_shadow_rows readbacks=4 readback_bytes=4014912
  readbacks=413
  readback_bytes=14198856
  op=scatter gpu_dispatches=0 cpu_mirrors=4
  op=witgen_data_invalid_fill gpu_dispatches=4
  no webgpu_scatter_offsets upload source
  no webgpu_scatter_values upload source
  cpu_fallbacks=0 cpu_only_ops=0

xgboost:
  receipt verifies
  wall_ms=103480
  gpu_active_ms=58140
  gpu_idle_ratio=0.438
  segments=11
  journal=30.528042544062632
  uploads=3425
  upload_bytes=7344983580
  raw_compute_dispatches=11342
  queue_submits=2627
  source=data upload_bytes=5252317184
  source=witgen_data_shadow_rows readbacks=11 readback_bytes=41568912
  readbacks=363
  readback_bytes=49057504
  op=scatter gpu_dispatches=0 cpu_mirrors=11
  op=witgen_data_invalid_fill gpu_dispatches=11
  no webgpu_scatter_offsets upload source
  no webgpu_scatter_values upload source
  cpu_fallbacks=0 cpu_only_ops=0
```

## Comparison

```text
SP7as xgboost replacement:
  wall_ms=103134
  uploads=3458
  upload_bytes=7532518804
  raw_compute_dispatches=11353
  queue_submits=2638
  webgpu_scatter_offsets upload_bytes=93769148
  webgpu_scatter_values upload_bytes=93769148

SP7at xgboost replacement:
  wall_ms=103480
  uploads=3425
  upload_bytes=7344983580
  raw_compute_dispatches=11342
  queue_submits=2627
  webgpu_scatter_offsets upload_bytes=0
  webgpu_scatter_values upload_bytes=0

SP7an default xgboost mean:
  wall_ms=99692
```

SP7at removes `187535224` total xgboost upload bytes versus SP7as, along with 33 uploads, 11 raw dispatches, and 11 queue submits. The single-run xgboost wall moved the wrong direction by `+346 ms`, and replacement remains about `3788 ms` slower than the SP7an default mean.

## Decision

Correctness accepted for the opt-in replacement path. Performance is not accepted for production enablement because representative xgboost wall time still loses to the default path.

Do not spend more time on replacement seed micro-optimizations. The remaining immediate witgen bottleneck is the large final `source=data` upload (`5252317184` bytes on xgboost) or the CPU accumulation dependency that forces it.
