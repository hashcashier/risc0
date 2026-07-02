# SP7cu MISC0 Filtered Replacement Dispatch - Rejected

Date: 2026-05-20

## Scope

Candidate filtered replacement-mode MISC0 dispatches so:

- main chunk0 ran only on MISC0 minor 0 rows;
- main chunk1 ran only on MISC0 minor 1 rows;
- extra MISC0 chunks ran only on their matching minor rows;
- MISC0 main chunk compile/readiness was skipped when the matching minor was absent.

The goal was to reduce wasted GPU-witgen work without changing receipt semantics.

## Validation

Compile/hygiene before e2e:

- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`: pass.
- `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`: pass, 4m24s.

Representative browser proof generation:

- BusyLoop: pass, receipt verified, `wall_ms=7221`, `gpu_active_ms=3208`, `raw_compute_dispatches=609`, `queue_submits=173`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- KeccakUnion: pass, receipt verified, `wall_ms=91636`, `segments=4`, `pending_keccaks=9`, `assumptions=1`, `gpu_active_ms=60037`, `raw_compute_dispatches=10660`, `queue_submits=3075`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- xgboost run 1: pass, receipt verified, journal `30.528042544062632`, `wall_ms=77331`, `gpu_active_ms=44757`, `gpu_idle_ratio=0.421`, `raw_compute_dispatches=9674`, `queue_submits=2718`, `upload_bytes=3113952132`, `readback_bytes=7488592`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- xgboost run 2: pass, receipt verified, journal `30.528042544062632`, `wall_ms=77538`, `gpu_active_ms=44743`, `gpu_idle_ratio=0.423`, `raw_compute_dispatches=9674`, `queue_submits=2718`, `upload_bytes=3113961280`, `readback_bytes=7488592`, `cpu_fallbacks=0`, `cpu_only_ops=0`.

One xgboost attempt was excluded because Chrome negotiated low WebGPU limits (`max_buffer_size=1073741824`, `max_storage_buffer_binding_size=1073741824`, `max_compute_workgroup_storage_size=32768`) and the representative guard failed before proof generation.

## Comparison

Accepted SP7cq baseline:

- BusyLoop: `wall_ms=7265`.
- KeccakUnion: `wall_ms=91943`.
- xgboost: `wall_ms=77417` / `77523`, mean `77470`.

SP7cu candidate:

- BusyLoop: `7265 -> 7221`, `-44 ms` / `-0.6%`.
- KeccakUnion: `91943 -> 91636`, `-307 ms` / `-0.3%`.
- xgboost mean: `77470 -> 77434.5`, `-35.5 ms` / `-0.05%`.

Dispatches and submits did not improve (`raw_compute_dispatches=9674`, `queue_submits=2718` on xgboost in both states). Cycle-list upload bytes were only split between `iter6d_g_arm_cycle_list` and `iter6d_g_arm_minor_cycle_list`; total was effectively unchanged.

## Decision

Rejected and reverted. The candidate was correctness-clean but the representative wall-time movement is noise-level and adds dispatch/readiness complexity without a significant performance gain. Current accepted working-state estimate remains SP7cq: BusyLoop `7.265s`, KeccakUnion `91.943s`, xgboost about `77.5s`.
