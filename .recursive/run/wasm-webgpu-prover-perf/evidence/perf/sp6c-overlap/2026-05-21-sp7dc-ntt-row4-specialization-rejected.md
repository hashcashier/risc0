# SP7dc: NTT Row4 Specialization Rejected

Date: 2026-05-21
Status: rejected and reverted

## Candidate

Specialized the remaining forward `batch_expand_into_evaluate_ntt` NTT stages for the FRI-shaped `count == 4` case. The candidate added `NTT_STEP_ROW4_WGSL`, which processed the same butterfly pair across all four extension-field lanes in one invocation and reused the twiddle load across those lanes.

Files temporarily changed:

- `risc0/zkp/src/hal/webgpu.rs`
- `examples/browser-prove/src/lib.rs`

## TDD

RED:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_fri_shape_batch_expand_ntt_uses_row4_path -- --nocapture
```

Result: failed as intended. GPU output matched the CPU mirror, but diagnostics only showed `webgpu_ntt_step_params`; the expected `webgpu_ntt_step_row4_params` path marker was absent.

GREEN:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_fri_shape_batch_expand_ntt_uses_row4_path -- --nocapture
```

Result: passed in Chrome with high WebGPU limits and CPU parity.

## Representative Proof E2E

All proof runs below used high Chrome WebGPU limits and real receipt verification.

BusyLoop + KeccakUnion:

- Log: `/tmp/sp7dc-busy-keccak-row4-ntt.log`
- Result: passed, zero fallback/CPU-only.
- BusyLoop: `wall_ms=7251`, `gpu_active_ms=3239`, `raw_compute_dispatches=609`, `queue_submits=173`, `upload_bytes=219427600`, `readback_bytes=478272`.
- KeccakUnion: `wall_ms=91625`, `segments=4`, `pending_keccaks=9`, `assumptions=1`, `gpu_active_ms=59866`, `raw_compute_dispatches=10660`, `queue_submits=3075`, `upload_bytes=2996636400`, `readback_bytes=10183944`.

xgboost:

- Log: `/tmp/sp7dc-xgboost-row4-ntt.log`
- Result: passed, verified journal `30.528042544062632`, zero fallback/CPU-only.
- `wall_ms=77959`
- `segments=11`
- `gpu_active_ms=44961`
- `gpu_idle_ratio=0.423`
- `raw_compute_dispatches=9674`
- `queue_submits=2718`
- `upload_bytes=3113948644`
- `readback_bytes=7488592`
- `webgpu_ntt_step_row4_params uploads=96 upload_bytes=147456`

Target aggregates from the xgboost log:

```text
fri_prove round=0 merkle_new rows=65536 cols=64: sum_ms=27415 count=32 mean_ms=856.7
finalize_async fri_prove: sum_ms=29199 count=32 mean_ms=912.5
finalize_async check_group: sum_ms=9633 count=32 mean_ms=301.0
recursion_witgen: sum_ms=5226 count=21
recursion_accumulate: sum_ms=5855 count=21
recursion_witgen_accum: sum_ms=5969 count=21
```

## Decision

Rejected and reverted.

The candidate was correctness-clean, but it did not provide a significant wall-time improvement:

- KeccakUnion was flat against SP7cy (`91537 -> 91625 ms`, +88 ms).
- xgboost improved only `78466 -> 77959 ms` against the latest canonical default sample, about `0.5 s`, and did not beat the accepted focused SP7cq estimate of about `77.5 s`.
- The target round-0 FRI bucket stayed in the same band (`27415 ms` across 32 samples).

Given the user priority to avoid sidequests and keep only immediate significant performance wins, the extra shader path is not worth retaining.

## Cleanup

Reverted the row4 shader, host dispatch routing, and focused test.

Post-revert checks:

```text
rg -n "row4|NTT_STEP_ROW4|webgpu_ntt_step_row4|fri_shape_batch_expand_ntt" risc0/zkp/src/hal/webgpu.rs examples/browser-prove/src/lib.rs
git diff --check
cargo fmt --check --manifest-path risc0/zkp/Cargo.toml
cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies --no-run
```

Results: marker search found no matches; all checks passed. The no-run compile passed in `4m54s`.

