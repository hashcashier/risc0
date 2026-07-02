# SP7cc Accumulator `eqz!` Elision In WebGPU Direct-Accum Path

Date: 2026-05-20

## Decision

Accepted.

This candidate elides evaluation of generated `eqz!` assertion expressions only while the WebGPU direct-accumulator path is running the remaining CPU `step_TopAccum` rows. The arithmetic-producing accumulator work is unchanged; final correctness is still proven by real browser receipt generation and verification. The guard restores the previous mode after the wrapped call.

This is not a multi-device, iframe, or broad generated-arm path. It targets the confirmed xgboost blocker:

- SP7bz `rv32im_accumulate step_top_accum_cpu_skip_replaced_misc0=true_major_mask=0x0006`: `12680 ms` across 11 segments.

## Code Shape

- `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs`
  - Added `ACCUM_EQZ_ELIDED` plus `with_accum_eqz_elided(...)`.
  - Changed the generated `eqz!` macro so elided assertions do not evaluate `$val`.
- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
  - Wrapped the WebGPU direct-accum CPU raw-step calls in `with_accum_eqz_elided(...)`.

## Validation

Compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
Finished release profile in 4m25s
```

Representative browser proof e2e:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
test tests::iter6d_g_replace_busy_loop_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 137 filtered out; finished in 109.84s
```

BusyLoop:

```text
wall_ms=8159
gpu_active_ms=3985
gpu_idle_ratio=0.512
raw_compute_dispatches=721
queue_submits=173
upload_bytes=209744064
readback_bytes=478272
cpu_fallbacks=0
cpu_only_ops=0
```

KeccakUnion:

```text
wall_ms=101378
segments=4
pending_keccaks=9
assumptions=1
gpu_active_ms=69097
gpu_idle_ratio=0.318
raw_compute_dispatches=12716
queue_submits=3075
upload_bytes=2991670004
readback_bytes=10183944
cpu_fallbacks=0
cpu_only_ops=0
```

xgboost browser proof e2e:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
test tests::iter6d_g_replace_xgboost ... ok
test result: ok. 1 passed; 0 failed; 137 filtered out; finished in 90.01s
```

xgboost metrics:

```text
wall_ms=89769
gpu_active_ms=56510
gpu_idle_ratio=0.370
segments=11
user_cycles=2294946
total_cycles=2883584
raw_compute_dispatches=11466
queue_submits=2718
upload_bytes=3104584972
readback_bytes=7488592
cpu_fallbacks=0
cpu_only_ops=0
```

xgboost targeted bucket:

```text
rv32im_accumulate step_top_accum_cpu_skip_replaced_misc0=true_major_mask=0x0006:
1021 + 979 + 1016 + 998 + 1008 + 983 + 994 + 981 + 958 + 987 + 1331 = 11256 ms
```

## Performance

Compared with SP7bz:

- BusyLoop: `8305 -> 8159 ms` (`-146 ms`, `-1.8%`).
- KeccakUnion: `101434 -> 101378 ms` (`-56 ms`, effectively flat).
- xgboost repeat: `91598 -> 89769 ms` (`-1829 ms`, about `-2.0%`).
- xgboost current estimate vs SP7bx mean: `91441.0 -> 89769 ms` (`-1672 ms`, about `-1.8%`).
- xgboost targeted `step_top_accum` bucket: `12680 -> 11256 ms` (`-1424 ms`, about `-11.2%`).

Dispatch/submission counts did not change, so this is a pure CPU-side generated-assertion-evaluation reduction in the remaining direct-accum raw-step work.

## Notes

Correctness risk is bounded by scope and receipt verification:

- The elision guard is only used around WebGPU direct-accum raw-step calls.
- The generated expression passed to `eqz!` is not evaluated while elided.
- Any accumulator arithmetic error still invalidates the proof and fails receipt verification.
- All accepted representative workloads generated and verified receipts with `cpu_fallbacks=0` and `cpu_only_ops=0`.
