# SP7dj row4 vector NTT step - rejected

Run: `.recursive/run/wasm-webgpu-prover-perf/`
Date: 2026-05-21
Status: rejected and reverted

## Hypothesis

`batch_expand_into_evaluate_ntt` is still one of the dominant xgboost buckets after the accepted SP7cq local-NTT fusion. This candidate tried to reduce forward NTT active work for FRI/check shapes by processing four rows per invocation when `count % 4 == 0`, reusing the same twiddle across those rows and using WGSL vector field helpers.

Target shapes:

- FRI round-0 `count=4`
- check-group path `count=16`

## RED/GREEN

Temporary focused browser test:

- `webgpu_hal_batch_expand_ntt_uses_row4_vector_steps_for_count16`
- `count=16`, `in_size=1024`, `expand_bits=2`
- Compared WebGPU output against CPU and required marker upload source `webgpu_ntt_step_row4_vec_params`.

RED:

- Log: `/tmp/sp7dj-row4-vec-ntt-red.log`
- High WebGPU limits negotiated.
- CPU parity path reached.
- Failed only on the missing marker.
- Counters at failure: `raw_compute_dispatches=3`, `queue_submits=3`, `cpu_fallbacks=0`, `cpu_only_ops=0`.

GREEN:

- Log: `/tmp/sp7dj-row4-vec-ntt-green.log`
- Focused CPU parity test passed.
- High WebGPU limits negotiated.

Temporary implementation:

- Added `NTT_STEP_ROW4_VEC_WGSL` in `risc0/zkp/src/hal/webgpu.rs`.
- Routed forward `dispatch_batch_expand_into_evaluate_ntt` through row4 vector steps when `count % 4 == 0`.
- Used upload source marker `webgpu_ntt_step_row4_vec_params`.

Pre-e2e hygiene:

- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`
- `cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml`
- `git diff --check`

## Representative Proof Gates

BusyLoop + KeccakUnion:

- Log: `/tmp/sp7dj-busy-keccak-row4-vec-ntt.log`
- Test: `rv32im_default_representative_e2e_verify`
- High WebGPU limits negotiated.
- Verified receipts.
- BusyLoop: `wall_ms=7335`, `gpu_active_ms=3197`, `raw_compute_dispatches=609`, `queue_submits=173`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- KeccakUnion: `wall_ms=92073`, `segments=4`, `user_cycles=747265`, `total_cycles=917504`, `gpu_active_ms=60010`, `raw_compute_dispatches=10660`, `queue_submits=3075`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- Marker uploads: BusyLoop `uploads=10`, `upload_bytes=19456`; KeccakUnion `uploads=202`, `upload_bytes=380928`.

xgboost:

- Log: `/tmp/sp7dj-xgboost-row4-vec-ntt.log`
- Test: `xgboost_succinct_receipt_verifies`
- High WebGPU limits negotiated.
- Verified receipt and journal.
- `wall_ms=77892`
- `segments=11`
- `user_cycles=2294916`
- `total_cycles=2883584`
- `gpu_active_ms=44924`
- `gpu_idle_ratio=0.423`
- `raw_compute_dispatches=9674`
- `queue_submits=2718`
- `upload_bytes=3114113816`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- Marker uploads: `uploads=170`, `upload_bytes=336896`.

## Comparison

Versus SP7dh valid default baseline:

- BusyLoop: `7217 -> 7335` (`+118 ms`)
- KeccakUnion: `91900 -> 92073` (`+173 ms`)
- xgboost: `78096 -> 77892` (`-204 ms`)

The xgboost movement is noise-level and still worse than the accepted SP7cq focused estimate around `77470 ms`. BusyLoop and KeccakUnion were flat/slightly worse.

## Decision

Reject and revert. Correctness was clean, but this row-reuse/vectorization shape did not produce a material representative wall-time improvement.

Revert checks:

- Removed focused test, row4 shader, dispatch routing, and marker.
- Marker search clean: `row4_vec`, `ROW4_VEC`, `webgpu_ntt_step_row4_vec`, `uses_row4_vector`.
- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`
- `cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml`
- `git diff --check`
- Post-revert compile gate passed: `xgboost_succinct_receipt_verifies --no-run` in `4m53s` after sandbox escalation for Cargo target-lock writes.

Accepted wall-time gain: 0.

Do not continue row-reuse/vector/invocation reshaping for NTT without focused evidence that it reduces active time by more than about one second on representative proof generation. The next NTT attempt would need a stronger memory-pass-reducing design, or we should pivot away from this bucket.
