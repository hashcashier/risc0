# SP7cx Expand/Local-NTT Plus Remaining NTT One-Submit Candidate - Rejected

Date: 2026-05-20

## Scope

Candidate changed `dispatch_batch_expand_into_evaluate_ntt` so the `BATCH_EXPAND_LOCAL_NTT_WGSL` dispatch and the remaining `NTT_STEP_WGSL` stage dispatches were enqueued into one compute pass and one queue submit when `n_bits > LOCAL_NTT_FUSED_BITS`.

Arithmetic, workgroup sizes, twiddle lookup, and raw dispatch counts were unchanged. Only the queue-submit topology changed.

## Validation

Compile/hygiene before e2e:

- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`: pass.
- `git diff --check`: pass.
- `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`: pass, 4m49s.

Representative BusyLoop + KeccakUnion browser proof generation:

- Log: `/tmp/sp7cx-expand-ntt-one-submit-busy-keccak.log`.
- Result: passed in 98.92s with verified receipts, high WebGPU limits, and zero fallback/CPU-only counters.
- BusyLoop: `wall_ms=7229`, `gpu_active_ms=3195`, `raw_compute_dispatches=609`, `queue_submits=159`, `upload_bytes=219417332`, `readback_bytes=478272`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- KeccakUnion: `wall_ms=91387`, `segments=4`, `pending_keccaks=9`, `assumptions=1`, `gpu_active_ms=59875`, `raw_compute_dispatches=10660`, `queue_submits=2818`, `upload_bytes=2996437536`, `readback_bytes=10183944`, `cpu_fallbacks=0`, `cpu_only_ops=0`.

Representative xgboost browser proof generation:

- Trial 1 log: `/tmp/sp7cx-expand-ntt-one-submit-xgboost.log`.
- Trial 1 result: verified receipt, high WebGPU limits, zero fallback/CPU-only counters, `wall_ms=78065`, `gpu_active_ms=45208`, `gpu_idle_ratio=0.421`, `raw_compute_dispatches=9674`, `queue_submits=2494`, `upload_bytes=3113827932`, `readback_bytes=7488592`.
- Trial 2 log: `/tmp/sp7cx-expand-ntt-one-submit-xgboost-r2.log`.
- Trial 2 result: verified receipt, high WebGPU limits, zero fallback/CPU-only counters, `wall_ms=77475`, `gpu_active_ms=44818`, `gpu_idle_ratio=0.422`, `raw_compute_dispatches=9674`, `queue_submits=2494`, `upload_bytes=3113991564`, `readback_bytes=7488592`.

## Comparison

Against accepted SP7cq:

- BusyLoop wall: `7265 -> 7229 ms`.
- KeccakUnion wall: `91943 -> 91387 ms`.
- xgboost mean wall: `77470 -> 77770 ms`, a regression of 300 ms, about 0.39%.
- Queue submits: BusyLoop `173 -> 159`, KeccakUnion `3075 -> 2818`, xgboost `2718 -> 2494`.
- Raw dispatches were unchanged: BusyLoop `609`, KeccakUnion `10660`, xgboost `9674`.

The xgboost hot FRI buckets were flat or slightly worse:

- `fri_prove round=0 domain_in`: SP7cq `27409/27378 ms`; candidate `27414/27402 ms`.
- `finalize_async fri_prove`: SP7cq about `29169 ms`; candidate `29224/29148 ms`.
- `check_group`: SP7cq `9592/9586 ms`; candidate `9693/9607 ms`.

## Decision

Rejected and reverted. The candidate produced deterministic submit-count reduction and preserved correctness, but it did not improve representative xgboost wall time. Under the current priority, command-submission cleanup is not retained without an end-to-end wall-time win on the representative workload set.

Post-revert validation:

- Marker search for the temporary expand/NTT one-submit code was clean.
- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`: pass.
- `git diff --check`: pass.
- `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`: pass, 4m50s.

Current accepted working-state estimate remains SP7cq: BusyLoop `7.265s`, KeccakUnion `91.943s`, xgboost about `77.5s`.
