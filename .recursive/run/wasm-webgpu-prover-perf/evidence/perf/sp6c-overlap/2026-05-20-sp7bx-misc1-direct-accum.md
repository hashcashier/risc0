# SP7bx: MISC1 Direct Accumulator Coverage

Date: 2026-05-20
Status: accepted as default-off direct-accum capability

## Change

Add accumulator-only direct WebGPU coverage for MISC1 rows, reusing the narrow direct accumulator kernel family already used for MISC0 and MISC2.

This does not enable MISC1 GPU-witgen replacement. The GPU-witgen replacement mask remains `0x0001` with `dispatched_arms=[0]`; MISC1 witness generation stays on CPU. The change only skips CPU `TopAccum` for major-1 rows when the runtime flag is enabled, then writes the direct accumulator contribution on WebGPU.

## Files

- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
- `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs`
- `risc0/circuit/rv32im/src/prove/mod.rs`
- `examples/browser-prove/src/lib.rs`

## Validation

All validation was browser WebGPU end-to-end proof generation with receipt verification. No proxy-only timing was used.

### Compile Gate

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in `4m22s`.

### BusyLoop + KeccakUnion Trial 1

Command:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: passed in `112.10s`.

BusyLoop:

- `wall_ms=8237`
- `gpu_active_ms=3987`
- `gpu_idle_ratio=0.516`
- `raw_compute_dispatches=721`
- `queue_submits=175`
- `upload_bytes=209753384`
- `readback_bytes=478272`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- first segment direct rows: `MISC0=68239`, `MISC1=7111`, `MISC2=25080`

KeccakUnion:

- `wall_ms=103559`
- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`
- `gpu_active_ms=69530`
- `gpu_idle_ratio=0.329`
- `raw_compute_dispatches=12716`
- `queue_submits=3083`
- `upload_bytes=2991845620`
- `readback_bytes=10183944`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `rv32im_accum_misc1_direct_rows=69056` bytes across 4 segments

### xgboost Trial 1

Command:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: passed in `91.91s`; receipt journal verified as `30.528042544062632`.

- `segments=11`
- `wall_ms=91667`
- `gpu_active_ms=56893`
- `gpu_idle_ratio=0.379`
- `raw_compute_dispatches=11466`
- `queue_submits=2740`
- `upload_bytes=3104811460`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `rv32im_accum_misc1_direct_rows=695512` bytes across 11 segments

### xgboost Trial 2

Command:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: passed in `91.45s`; receipt journal verified as `30.528042544062632`.

- `segments=11`
- `wall_ms=91215`
- `gpu_active_ms=56570`
- `gpu_idle_ratio=0.380`
- `raw_compute_dispatches=11466`
- `queue_submits=2740`
- `upload_bytes=3104677492`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `rv32im_accum_misc1_direct_rows=695512` bytes across 11 segments

### BusyLoop + KeccakUnion Trial 2

Command:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: passed in `110.50s`.

BusyLoop:

- `wall_ms=8230`
- `gpu_active_ms=3994`
- `gpu_idle_ratio=0.515`
- `raw_compute_dispatches=721`
- `queue_submits=175`
- `upload_bytes=209726676`
- `readback_bytes=478272`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

KeccakUnion:

- `wall_ms=101971`
- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`
- `gpu_active_ms=69389`
- `gpu_idle_ratio=0.320`
- `raw_compute_dispatches=12716`
- `queue_submits=3083`
- `upload_bytes=2991672468`
- `readback_bytes=10183944`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `rv32im_accum_misc1_direct_rows=69056` bytes across 4 segments

## Comparison

SP7bw accepted baseline:

- BusyLoop: `8326 ms`
- KeccakUnion: `102019 ms`
- xgboost: `92843` / `93262 ms`, mean `93052.5 ms`

SP7bx:

- BusyLoop: `8237` / `8230 ms`, mean `8233.5 ms`
- KeccakUnion: `103559` / `101971 ms`; one noisy regression, one flat/slightly positive against SP7bw
- xgboost: `91667` / `91215 ms`, mean `91441.0 ms`

xgboost movement:

- vs SP7bw mean: `93052.5 -> 91441.0 ms`, `-1611.5 ms`, about `-1.73%`
- vs recent post-revert accepted mean before SP7bw (`94959.25 ms`): `-3518.25 ms`, about `-3.70%`

BusyLoop movement:

- vs SP7bw: `8326 -> 8233.5 ms`, `-92.5 ms`, about `-1.1%`

KeccakUnion movement:

- representative e2e correctness is clean, but wall is noisy: `103559 ms` first trial, `101971 ms` repeat versus SP7bw `102019 ms`.
- Do not count this as a KeccakUnion wall-time win; keep KeccakUnion in the gate for future candidates.

## Decision

Accept the MISC1 direct accumulator path as a small xgboost-positive, correctness-preserving, default-off capability. The current xgboost working-state estimate with MISC0+MISC1+MISC2 direct accumulator coverage enabled is about `91.4 s` on this browser/NVIDIA setup.

This remains a local accumulator-side improvement. It does not change the conclusion that iframe/multi-device orchestration is lower priority than reducing per-proof fixed work inside the single WebGPU prover pipeline.
