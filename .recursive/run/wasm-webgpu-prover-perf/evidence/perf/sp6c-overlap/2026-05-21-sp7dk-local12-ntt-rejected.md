# SP7dk local-12 fused batch-expand NTT - rejected

Run: `.recursive/run/wasm-webgpu-prover-perf/`
Date: 2026-05-21
Status: rejected and reverted

## Hypothesis

SP7dh showed xgboost still spends large drain time in `batch_expand_into_evaluate_ntt`: roughly `27.2 s` in FRI round 0 and `9.1 s` in the check group. After row-reuse/invocation-shaping variants failed to move wall time materially, this candidate tried a stronger memory-pass reduction: for FRI/check shapes, fuse the first 12 NTT bits into local workgroup memory instead of the accepted 10-bit local path.

Target shapes:

- FRI round-0 `count=4`, `expand_bits=2`
- check-group `count=16`, `expand_bits=2`

The guard required `actual_expand_bits == 2`, `n_bits >= 12`, `count == 4 || count == 16`, and at least 16 KiB of workgroup storage for a 4096-u32 scratch block.

## RED/GREEN

Temporary focused browser HAL test:

- `webgpu_hal_batch_expand_ntt_uses_large_local_path_for_fri_shape`
- `count=4`, `in_size=1024`, `expand_bits=2`, `out_size=4096`
- Compared WebGPU output against CPU.
- Required marker upload source `webgpu_batch_expand_local_ntt12_params`.
- Required zero `webgpu_ntt_step_params` uploads for this `n_bits=12` shape.

RED:

- Log: `/tmp/sp7dk-local12-ntt-red.log`
- High WebGPU limits negotiated.
- CPU parity path reached.
- Failed only on the missing `webgpu_batch_expand_local_ntt12_params` marker.
- Counters at failure: `raw_compute_dispatches=3`, `queue_submits=3`, `cpu_fallbacks=0`, `cpu_only_ops=0`.

GREEN:

- Log: `/tmp/sp7dk-local12-ntt-green.log`
- Focused CPU parity test passed.
- High WebGPU limits negotiated.
- Compile/test wall: `4m50s` compile, browser test `0.11s`.

Temporary implementation:

- Added guarded local-12 path inside `risc0/zkp/src/hal/webgpu.rs::dispatch_batch_expand_into_evaluate_ntt`.
- Reused `BATCH_EXPAND_LOCAL_NTT_WGSL` by specializing `FUSED_BITS=12`, `FUSED_BLOCK_SIZE=4096`, and `scratch: array<u32, 4096>`.
- Used marker labels `webgpu_batch_expand_local_ntt12_*`.

Pre-e2e hygiene:

- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`
- `cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml`
- `git diff --check`

## Representative Proof Gates

BusyLoop + KeccakUnion:

- Log: `/tmp/sp7dk-busy-keccak-local12-ntt.log`
- Test: `rv32im_default_representative_e2e_verify`
- High WebGPU limits: `max_buffer_size=4294967292`, `max_storage_buffer_binding_size=2147483644`, `max_compute_workgroup_storage_size=49152`.
- Verified receipts.
- BusyLoop: `wall_ms=7265`, `gpu_active_ms=3191`, `raw_compute_dispatches=593`, `queue_submits=171`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- BusyLoop marker uploads: `webgpu_batch_expand_local_ntt12_params uploads=8`, `webgpu_ntt_step_params uploads=20`.
- KeccakUnion: `wall_ms=93058`, `segments=4`, `user_cycles=747265`, `total_cycles=917504`, `gpu_active_ms=60131`, `raw_compute_dispatches=10376`, `queue_submits=3038`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- KeccakUnion marker uploads: `webgpu_batch_expand_local_ntt12_params uploads=142`, `webgpu_ntt_step_params uploads=372`.

xgboost:

- Log: `/tmp/sp7dk-xgboost-local12-ntt.log`
- Test: `xgboost_succinct_receipt_verifies`
- High WebGPU limits negotiated.
- Verified receipt and journal.
- `wall_ms=78689`
- `segments=11`
- `user_cycles=2294916`
- `total_cycles=2883584`
- `gpu_active_ms=45017`
- `gpu_idle_ratio=0.428`
- `raw_compute_dispatches=9418`
- `queue_submits=2686`
- `upload_bytes=3113932868`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- Marker uploads: `webgpu_batch_expand_local_ntt12_params uploads=128`, `webgpu_ntt_step_params uploads=320`.

## Comparison

Versus SP7dh valid default baseline:

- BusyLoop: `7217 -> 7265` (`+48 ms`)
- KeccakUnion: `91900 -> 93058` (`+1158 ms`)
- xgboost: `78096 -> 78689` (`+593 ms`)
- xgboost raw dispatches: `9674 -> 9418` (`-256`)
- xgboost queue submits: `2718 -> 2686` (`-32`)

The candidate mechanically reduced dispatch/submission count, but did not reduce representative wall time. The likely lesson is that increasing the local scratch footprint to 4096 elements worsens occupancy/cache behavior enough to erase the two saved global stages.

## Decision

Reject and revert. Correctness was clean, but all representative wall measurements were flat or negative.

Revert checks:

- Removed focused test and guarded local-12 branch/labels.
- Marker search clean: `local_ntt12`, `LARGE_LOCAL_NTT`, `large_local`, `webgpu_batch_expand_local_ntt12`.
- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`
- `cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml`
- `git diff --check`
- Post-revert compile gate passed: `xgboost_succinct_receipt_verifies --no-run`.
- Captured post-revert compile log: `/tmp/sp7dk-post-revert-xgboost-no-run.log`, finished in `2m19s` after cache warm-up.

Accepted wall-time gain: 0.

Do not continue larger local-memory NTT block variants without focused evidence of active-time reduction. The recent NTT sequence now falsifies pair2, row4 vector, and local-12 memory-pass reduction; next work should pivot to a different high-ceiling bottleneck rather than more `batch_expand_into_evaluate_ntt` reshaping.
