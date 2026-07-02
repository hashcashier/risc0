# SP7cq: Batch Expand Local NTT Fusion

Date: 2026-05-20
Status: accepted as a small NTT/expand improvement

## Change

`batch_expand_into_evaluate_ntt` now runs a fused WebGPU kernel that:

- expands the coefficient rows into the evaluation-domain buffer;
- loads each 1024-element row block into workgroup memory;
- performs forward NTT stages through `s_bits <= 10` locally; and
- leaves the remaining larger stages on the existing cached-twiddle `NTT_STEP_WGSL` path.

This reduces global-memory NTT passes and raw compute dispatches while keeping queue-submit count unchanged. The old standalone `BATCH_EXPAND_WGSL` dispatch was removed because expansion is now handled by `BATCH_EXPAND_LOCAL_NTT_WGSL`.

Changed file:

- `risc0/zkp/src/hal/webgpu.rs`

## Validation

All proof runs below are browser WebGPU end-to-end proof generation with receipt verification, high Chrome WebGPU limits, and zero fallback/CPU-only counters.

### Compile / Hygiene

Compile gate:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in `2m18s` after final cleanup.

Hygiene:

```text
cargo fmt --check --manifest-path risc0/zkp/Cargo.toml
git diff --check
```

Both passed.

### BusyLoop + KeccakUnion

Log: `/tmp/sp7cq-busy-keccak-local-ntt.log`

Command:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: passed in `99.51s`.

BusyLoop:

- `wall_ms=7265`
- `gpu_active_ms=3216`
- `raw_compute_dispatches=609`
- `queue_submits=173`
- `upload_bytes=219420164`
- `readback_bytes=478272`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

KeccakUnion:

- `wall_ms=91943`
- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`
- `gpu_active_ms=60371`
- `raw_compute_dispatches=10660`
- `queue_submits=3075`
- `upload_bytes=2996450808`
- `readback_bytes=10183944`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

### xgboost

Logs:

- `/tmp/sp7cq-xgboost-local-ntt.log`
- `/tmp/sp7cq-xgboost-local-ntt-r2.log`

Command:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Trial 1:

- `wall_ms=77417`
- `gpu_active_ms=44742`
- `gpu_idle_ratio=0.422`
- `raw_compute_dispatches=9674`
- `queue_submits=2718`
- `upload_bytes=3113979572`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

Trial 2:

- `wall_ms=77523`
- `gpu_active_ms=44712`
- `gpu_idle_ratio=0.423`
- `raw_compute_dispatches=9674`
- `queue_submits=2718`
- `upload_bytes=3113943924`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

## Comparison

Against SP7cm final accepted baseline:

```text
BusyLoop    wall 7276 -> 7265  (-11 ms, -0.15%)   raw dispatches 721 -> 609
KeccakUnion wall 92303 -> 91943 (-360 ms, -0.39%) raw dispatches 12716 -> 10660
xgboost     wall 77800 -> 77470 mean (-330 ms, -0.42%) raw dispatches 11466 -> 9674
```

xgboost trial mean is `(77417 + 77523) / 2 = 77470 ms`.

The hot xgboost aggregate did not materially move:

```text
SP7cm  round0 FRI aggregate: 27383 ms
SP7cq  round0 FRI aggregate: 27409 / 27378 ms
SP7cm  check_group aggregate: 9571 ms
SP7cq  check_group aggregate: 9592 / 9586 ms
```

## Decision

Accepted as a small correctness-preserving NTT/expand improvement, not as the deeper NTT breakthrough.

Reasons:

- representative proof generation passed for BusyLoop, KeccakUnion, and two xgboost trials;
- `cpu_fallbacks=0` and `cpu_only_ops=0` in every gate;
- raw compute dispatches drop deterministically by removing the standalone expand dispatch and several early NTT-step dispatches per call;
- xgboost wall moved positively in both trials, but only by about `0.3s`, and the dominant drained FRI/check buckets remain flat.

Current accepted xgboost working-state estimate: about `77.5s` on this browser/NVIDIA setup.

Follow-up: do not count dispatch-count reduction alone as a significant lever. Further NTT work needs to reduce the dominant drained FRI/check buckets, likely with a larger tiled/Stockham-style design or another measured mechanism that moves xgboost wall by >1s.
