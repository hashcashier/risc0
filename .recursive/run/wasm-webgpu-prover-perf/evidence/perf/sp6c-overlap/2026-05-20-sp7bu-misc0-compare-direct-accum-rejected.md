# SP7bu MISC0 Compare Direct Accumulator Rejected

Date: 2026-05-20

## Candidate

Extend the accepted direct MISC0 accumulator path to MISC0 compare rows
(`major=0`, `minor=5|6`) without GPU-witgen replacement. The goal was to skip
a small additional slice of CPU `step_TopAccum` while preserving CPU witness
generation for compare rows.

Implementation sketch:

- Add opt-in `set_accum_gpu_misc0_compare_direct_enabled`.
- Collect compare-row cycle indices separately from GPU-witgen-owned MISC0 rows.
- Run CPU accumulation with both replaced MISC0 rows and compare rows skipped.
- Reuse the narrow MISC0 direct accumulator kernel for compare rows.

## RED / GREEN

RED compile failed as expected after the representative tests imported missing
APIs:

```text
unresolved imports `accum_gpu_misc0_compare_direct_rows`,
`set_accum_gpu_misc0_compare_direct_enabled`
```

GREEN compile passed:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run

Finished `release` profile ... in 4m23s
```

## Candidate E2E Results

BusyLoop + KeccakUnion browser e2e passed with receipt verification and zero
fallback/CPU-only ops.

BusyLoop:

- `wall_ms=8369`
- `gpu_active_ms=4004`
- `gpu_idle_ratio=0.522`
- `raw_compute_dispatches=720`
- `queue_submits=174`
- `upload_bytes=210569440`
- `readback_bytes=478272`
- regular MISC0 direct rows: `68239`
- compare-direct rows: `8957`

KeccakUnion:

- `wall_ms=102643`
- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`
- `gpu_active_ms=69322`
- `gpu_idle_ratio=0.325`
- `raw_compute_dispatches=12712`
- `queue_submits=3079`
- `upload_bytes=3007518544`
- `readback_bytes=10183944`
- compare-row-list upload: `27` rows / `108` bytes

xgboost first attempt was an environment gate failure, not proof evidence:
Chrome negotiated low WebGPU limits (`max_buffer_size=1073741824`,
`max_storage_buffer_binding_size=1073741824`,
`max_compute_workgroup_storage_size=32768`) and the high-limit guard halted the
run before proof generation.

xgboost rerun passed with receipt verification and zero fallback/CPU-only ops:

- `wall_ms=95279`
- `segments=11`
- journal `30.528042544062632`
- `gpu_active_ms=56906`
- `gpu_idle_ratio=0.403`
- `raw_compute_dispatches=11454`
- `queue_submits=2728`
- `upload_bytes=3187327764`
- `readback_bytes=7488592`
- compare-row-list upload: `10` uploads / `51932` bytes

The latest fresh pre-candidate xgboost rerun was `wall_ms=94792`. The candidate
therefore moved xgboost wall by `+487 ms` in the direct A/B window while adding
`+10` raw dispatches and `+10` queue submits. It saved only about `5.9 MB` of
upload and covered about `13k` compare rows across xgboost.

## Decision

Rejected.

The project priority is e2e proving wall time with absolute correctness. This
candidate was correctness-clean, but the deterministic work removed was too
small, and the extra dispatch/submission overhead failed the immediate
significant-wall-improvement bar on xgboost.

## Revert Validation

Candidate code/test changes were removed. The retained working state keeps the
accepted direct MISC0 accumulator path and removes only the compare-direct
extension.

Post-revert gates:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run

Finished `release` profile ... in 4m19s
```

BusyLoop + KeccakUnion browser e2e: PASS.

- BusyLoop: `wall_ms=8432`, `gpu_active_ms=4012`, `gpu_idle_ratio=0.524`,
  `raw_compute_dispatches=719`, `queue_submits=173`, `upload_bytes=213565180`,
  `readback_bytes=478272`, zero fallback/CPU-only.
- KeccakUnion: `wall_ms=102209`, `segments=4`, `pending_keccaks=9`,
  `assumptions=1`, `gpu_active_ms=69056`, `gpu_idle_ratio=0.324`,
  `raw_compute_dispatches=12708`, `queue_submits=3075`,
  `upload_bytes=3007441216`, `readback_bytes=10183944`, zero fallback/CPU-only.

xgboost browser e2e: PASS.

- `wall_ms=95680`
- `segments=11`
- journal `30.528042544062632`
- `gpu_active_ms=56999`
- `gpu_idle_ratio=0.404`
- `raw_compute_dispatches=11444`
- `queue_submits=2718`
- `upload_bytes=3193303564`
- `readback_bytes=7488592`
- zero fallback/CPU-only

The post-revert xgboost wall trial was noisy relative to the prior post-revert
series (`94385`, `94980`, `94792`), but the structural counters returned to the
lower-dispatch/lower-submit working path. This does not justify retaining a
candidate whose direct pre/post xgboost run was wall-negative and below the
priority threshold.

## Follow-Up

Do not continue accumulator-only compare-row slicing as an immediate wall-time
lever. Remaining effort should target larger xgboost-dominant costs, especially
the sparse upload streams and the still-visible RV32IM CPU accumulation slice,
but only with a mechanism large enough to clear representative e2e wall gates.
