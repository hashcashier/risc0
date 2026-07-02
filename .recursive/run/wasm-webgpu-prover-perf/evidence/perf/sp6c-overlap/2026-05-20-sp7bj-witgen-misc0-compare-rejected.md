# SP7bj: MISC0 Compare GPU-Witgen Candidate Rejected

Date: 2026-05-20

## Scope

Candidate: extend opt-in RV32IM GPU-witgen replacement from profitable MISC0 arithmetic/bitwise minors `{0,1,2,3,4,7}` to compare minors `{5,6}` (`SLT`/`SLTU`).

Decision: reject and back out the candidate. It was correctness-positive, but wall-negative. The accepted path remains MISC0 `{0,1,2,3,4,7}` with no data-shadow readback and grouped accum-shadow row readback.

## RED / Candidate Signal

Representative browser e2e proof gate first proved the existing path missed compare minors:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

The BusyLoop receipt verified, then the new RED assertion failed:

```text
short_circuit_delta=68239
required >= 70000
wall_ms=8975.0
rv32im_witgen_accum_shadow_gpu_sync rows=68239
readbacks=23 readback_bytes=37670192
source=witgen_accum_shadow_rows readbacks=1 readback_bytes=37191920
cpu_fallbacks=0 cpu_only_ops=0
```

This showed compare minors were not covered, but did not prove they were profitable.

## Candidate GREEN But Wall-Negative

The candidate added MISC0 chunk5/chunk6 WGSL deltas and sparse compare-row CPU-shadow repair. It then passed receipt verification for BusyLoop and KeccakUnion, but regressed wall time:

```text
BusyLoop:
  wall_ms=11023.0
  rv32im_witgen_accum_shadow_gpu_sync rows=77196
  pre_witgen_dispatch_ms=3293
  prewarm_ms=3244
  readbacks=24 readback_bytes=47630376
  raw_compute_dispatches=717 queue_submits=173
  cpu_fallbacks=0 cpu_only_ops=0

KeccakUnion:
  wall_ms=104634.0
  segments=4 pending_keccaks=9 assumptions=1
  readbacks=417 readback_bytes=77218096
  source=witgen_accum_shadow_rows readbacks=4 readback_bytes=67019140
  source=witgen_data_shadow_rows readbacks=4 readback_bytes=15012
  raw_compute_dispatches=12658 queue_submits=3033
  cpu_fallbacks=0 cpu_only_ops=0
```

Compared with the SP7bi accepted path, the compare candidate adds two replacement kernels, sparse data-shadow row repair, extra queue submits, and a large BusyLoop wall regression. The extra short-circuited rows do not offset the browser-side compile/sync overhead.

## Backout Validation

Backed out:

- MISC0 `minor=5/6` from `is_witgen_replace_cycle` and `cycle_short_circuited`.
- MISC0 compare sparse data-shadow row repair.
- Generated `exec_misc0_chunk5_delta.wgsl` / `exec_misc0_chunk6_delta.wgsl`.
- Browser e2e gate now restores the invariant: no `witgen_data_shadow_rows` readback for replacement mode.

Compile gate:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run

result: pass
```

Representative browser e2e proof gate after backout:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Results:

```text
BusyLoop:
  wall_ms=8873.0
  rv32im_witgen_accum_shadow_gpu_sync rows=68239
  pre_witgen_dispatch_ms=1528
  prewarm_ms=2095
  readbacks=23 readback_bytes=37670192
  raw_compute_dispatches=715 queue_submits=170
  compute_pipeline_creations=29
  source=witgen_accum_shadow_rows readbacks=1 readback_bytes=37191920
  cpu_fallbacks=0 cpu_only_ops=0

KeccakUnion:
  wall_ms=104093.0
  segments=4 pending_keccaks=9 assumptions=1
  readbacks=413 readback_bytes=77188072
  raw_compute_dispatches=12650 queue_submits=3021
  source=witgen_accum_shadow_rows readbacks=4 readback_bytes=67004128
  no source=witgen_data_shadow_rows
  cpu_fallbacks=0 cpu_only_ops=0
```

xgboost e2e proof gate after backout:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_xgboost -- --nocapture
```

Results:

```text
xgboost:
  wall_ms=99736.0
  segments=11 user_cycles=2294890 total_cycles=2883584
  journal=30.528042544062632
  readbacks=363 readback_bytes=414487056
  source=witgen_accum_shadow_rows readbacks=11 readback_bytes=406998464
  raw_compute_dispatches=11380 queue_submits=2665
  compute_pipeline_creations=29
  cpu_fallbacks=0 cpu_only_ops=0
  no source=witgen_data_shadow_rows
```

## Conclusion

MISC0 compare offload is not an immediate performance win in the current architecture. Its required compare-row shadow repair and additional kernels increase wall time, especially on BusyLoop first-proof path. Do not continue same-pattern MISC0 expansion.

Next highest-value target remains the 407 MB xgboost `witgen_accum_shadow_rows` bridge. To produce material wall-time reduction, accumulation/TopAccum must consume GPU-owned witness data directly or move the relevant TopAccum work to WebGPU, instead of pulling the witness rows back to CPU.
