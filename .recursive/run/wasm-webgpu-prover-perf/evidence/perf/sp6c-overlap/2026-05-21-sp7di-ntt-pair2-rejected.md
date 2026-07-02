Run: `.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7di - forward pair2 global NTT step`
Date: `2026-05-21`
Status: `REJECTED / REVERTED`

## Hypothesis

SP7dh identified xgboost's largest attributed buckets as FRI round-0
`batch_expand_into_evaluate_ntt` (~27.2 s with explicit drains) and
check-group `batch_expand_into_evaluate_ntt` (~9.1 s). This candidate
tested a narrow forward-only NTT step variant that processes two
adjacent global butterfly pairs per invocation after the accepted local
expand/NTT fused prefix. The goal was lower invocation/workgroup overhead
without changing the number of queue submits.

## RED / GREEN

RED added a focused browser HAL test requiring the proof-shaped
`batch_expand_into_evaluate_ntt` path to upload
`webgpu_ntt_step_pair2_params`. It failed as expected:

- Log: `/tmp/sp7di-pair2-ntt-red.log`
- High WebGPU limits were negotiated.
- CPU parity path reached.
- Failure: missing pair2 marker.
- Existing path had `raw_compute_dispatches=3`, `queue_submits=3`,
  `cpu_fallbacks=0`, `cpu_only_ops=0`.

GREEN added `NTT_STEP_PAIR2_WGSL` for forward NTT steps only and routed
`dispatch_batch_expand_into_evaluate_ntt` remaining stages through it.
The focused browser HAL parity test passed:

- Log: `/tmp/sp7di-pair2-ntt-green.log`
- `webgpu_hal_batch_expand_ntt_uses_pair2_global_steps ... ok`
- Limits:
  - `max_buffer_size=4294967292`
  - `max_storage_buffer_binding_size=2147483644`
  - `max_compute_workgroup_storage_size=49152`

## Representative Proof Gates

All representative gates were launched from `examples/browser-prove/`
with the high-limit Chrome profile. Receipts verified and no fallback or
CPU-only HAL operations were introduced.

### BusyLoop + KeccakUnion

Log: `/tmp/sp7di-busy-keccak-pair2-ntt.log`

BusyLoop:

- `wall_ms=7353`
- `gpu_active_ms=3222`
- `raw_compute_dispatches=609`
- `queue_submits=173`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- Pair2 marker: `source=webgpu_ntt_step_pair2_params uploads=14`

KeccakUnion:

- `wall_ms=92183`
- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`
- `gpu_active_ms=60172`
- `raw_compute_dispatches=10660`
- `queue_submits=3075`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- Pair2 marker: `source=webgpu_ntt_step_pair2_params uploads=257`

Comparison with SP7dh default baseline:

- BusyLoop: `7217 -> 7353 ms` (`+136 ms`, noise/worse)
- KeccakUnion: `91900 -> 92183 ms` (`+283 ms`, noise/worse)
- Dispatch and submit counts unchanged.

### xgboost

Log: `/tmp/sp7di-xgboost-pair2-ntt.log`

- `xgboost_succinct_receipt_verifies ... ok`
- `wall_ms=78056`
- `segments=11`
- `gpu_active_ms=44969`
- `gpu_idle_ratio=0.424`
- `raw_compute_dispatches=9674`
- `queue_submits=2718`
- `upload_bytes=3113932220`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- Pair2 marker: `source=webgpu_ntt_step_pair2_params uploads=224`

Comparison with SP7dh default baseline:

- xgboost: `78096 -> 78056 ms` (`-40 ms`, not material)
- `gpu_active_ms`: `45005 -> 44969` (`-36 ms`, not material)
- Dispatch and submit counts unchanged.

## Decision

Reject and revert. The candidate was correctness-clean, but the xgboost
movement was only `-40 ms` versus the latest valid default baseline and
did not materially reduce the attributed FRI/check NTT buckets. BusyLoop
and KeccakUnion were flat to slightly worse.

This also reinforces the SP7de/SP7cx finding: NTT changes that only
reshape invocation/dispatch mechanics without reducing the real active
work are not a reliable immediate wall-time lever.

## Revert / Hygiene

Reverted:

- Focused pair2 marker test in `examples/browser-prove/src/lib.rs`
- `NTT_STEP_PAIR2_WGSL`
- Pair2 routing and `webgpu_ntt_step_pair2_params` upload source in
  `risc0/zkp/src/hal/webgpu.rs`

Post-revert checks:

- `rg "pair2|webgpu_ntt_step_pair2|NTT_STEP_PAIR2" examples/browser-prove/src/lib.rs risc0/zkp/src/hal/webgpu.rs`: no matches
- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`: pass
- `cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml`: pass
- `git diff --check`: pass
- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies --no-run`: pass after sandbox escalation for Cargo target-lock writes, `4m52s`

Accepted wall-time gain: `0`.

Next target remains direct reduction of the `batch_expand_into_evaluate_ntt`
active work identified by SP7dh, not iframe/multi-device workarounds or
dispatch-count-only NTT reshaping.
