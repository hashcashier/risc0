# SP7bv MISC2 Recheck Rejected

Date: 2026-05-20

## Candidate

Recheck MISC2 GPU-witgen replacement in the current tree after the later sparse
upload/readback reductions. Earlier evidence proved the MISC2 slice could be
made correctness-positive, but it was wall-negative because it required sparse
CPU-shadow repair. This candidate tested whether the newer MISC0 direct
accumulator and sparse upload work changed that decision.

Candidate edit:

- Temporarily set `WITGEN_REPLACE_SUPPORTED_ARM_MASK` to `(1 << 0) | (1 << 2)`.
- Temporarily set `WITGEN_REPLACE_SUPPORTED_ARMS` to `[0, 2]`.
- Changed the representative BusyLoop gate to assert the MISC2 bit was active.

## Compile Gate

Post-candidate compile passed:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run

Finished `release` profile ... in 4m23s
```

## Representative E2E Result

BusyLoop + KeccakUnion browser e2e was started first. BusyLoop generated and
verified the proof path, then failed the representative gate before KeccakUnion
because MISC2 reintroduced sparse shadow readbacks.

BusyLoop candidate metrics:

- high WebGPU limits negotiated:
  - `max_buffer_size=4294967292`
  - `max_storage_buffer_binding_size=2147483644`
  - `max_compute_workgroup_storage_size=49152`
- `mask=0x0005`, `dispatched_arms=[0, 2]`
- on-demand MISC2 compiles: `misc2_chunk2..misc2_chunk6`, count `5`
- `iter6d_g_pre_witgen_dispatch_async elapsed_ms=5506`
- `prove_session_async wall_ms=12499`
- `gpu_active_ms=3983`
- `gpu_idle_ratio=0.681`
- `raw_compute_dispatches=728`
- `queue_submits=184`
- `compute_pipeline_creations=39`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `rv32im_witgen_accum_shadow_gpu_sync rows=22747`
- `witgen_data_shadow_rows readbacks=1 readback_bytes=12073976`
- `witgen_accum_shadow_rows readbacks=1 readback_bytes=12073976`
- total `readback_bytes=24626224`

The failing assertion was the existing representative gate:

```text
GPU-witgen replacement must not repair the CPU shadow by reading sparse rows
left: 1
right: 0
```

## Decision

Rejected without running xgboost.

The current accepted BusyLoop post-revert run is `wall_ms=8432` with no
`witgen_data_shadow_rows` or `witgen_accum_shadow_rows` readback. MISC2 moved
the first representative workload to `12499 ms` and reintroduced two 12.1 MB
shadow readbacks. That is a large enough regression on the smallest gate that
xgboost would not be a responsible use of time.

This also clarifies the blocker: MISC2 is not being held back only by old dense
upload behavior. It still requires CPU-shadow repair that violates the current
proof-performance gate.

## Revert Validation

Candidate code/test changes were reverted to the MISC0-only replacement mask.

Post-revert compile passed:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run

Finished `release` profile ... in 4m22s
```

The exact MISC0-only working state had already passed representative e2e
immediately before this candidate:

- BusyLoop + KeccakUnion browser e2e:
  - BusyLoop `wall_ms=8432`, zero fallback/CPU-only.
  - KeccakUnion `wall_ms=102209`, zero fallback/CPU-only.
- xgboost browser e2e:
  - `wall_ms=95680`, `segments=11`, journal `30.528042544062632`,
    zero fallback/CPU-only.

## Follow-Up

Do not retry MISC2 as a broad replacement arm until its witness/accum shadow
repair is eliminated or made GPU-resident. The next useful work should either
remove that repair dependency directly or choose a different candidate with no
shadow-readback requirement and enough xgboost row volume to matter.
