# SP7bz: Grouped MISC Direct Accumulator Submit

Date: 2026-05-20
Status: accepted as small structural submit-count reduction

## Candidate

Batch the accepted MISC0/MISC1/MISC2 direct accumulator dispatches into one compute pass and one queue submit per RV32IM segment.

The arithmetic stays unchanged:

- MISC0/MISC1/MISC2 witness generation ownership is unchanged.
- GPU-witgen replacement mask remains `0x0001`.
- The same narrow direct accumulator kernels are used.
- CPU `TopAccum` still skips only the selected direct-accumulator rows/majors.

The intended effect is command submission reduction only: raw compute dispatch count should stay the same, while queue submissions drop by two per RV32IM segment when all three direct accumulator groups are present.

## Files

- `risc0/zkp/src/hal/webgpu.rs`
- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`

## Validation

All workload validation below is browser WebGPU end-to-end proof generation with receipt verification. No proxy-only benchmark was used.

### Compile Gate

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result before cleanup: passed in `4m24s`.

After removing the now-unused individual direct-dispatch wrappers and formatting, the same compile gate passed again:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in `4m24s`.

`git diff --check` is clean.

### BusyLoop + KeccakUnion

Command:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: passed in `110.05s`.

BusyLoop:

- `wall_ms=8305`
- `gpu_active_ms=4017`
- `gpu_idle_ratio=0.516`
- `raw_compute_dispatches=721`
- `queue_submits=173`
- `upload_bytes=209746548`
- `readback_bytes=478272`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- grouped direct rows: `MISC0=68239`, `MISC1=7111`, `MISC2=25080`

KeccakUnion:

- `wall_ms=101434`
- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`
- `gpu_active_ms=68763`
- `gpu_idle_ratio=0.322`
- `raw_compute_dispatches=12716`
- `queue_submits=3075`
- `upload_bytes=2991812084`
- `readback_bytes=10183944`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

### xgboost

Command:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

First run passed with verified receipt; the test runner reported `finished in 91.73s`, but the terminal output was too large to retain the exact metric line.

Captured repeat command:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture > /tmp/sp7bz-xgboost-repeat.log 2>&1
```

The first captured attempt failed before proof generation because `wasm-bindgen-test-runner` could not spawn its local server in the sandbox: `Operation not permitted (os error 1)`. This was not a proof/verifier failure. The same captured command then passed with escalation for the local browser test server.

Captured repeat result:

- test passed in `91.83s`
- `wall_ms=91598`
- `gpu_active_ms=56740`
- `gpu_idle_ratio=0.381`
- `raw_compute_dispatches=11466`
- `queue_submits=2718`
- `upload_bytes=3104692808`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

## Comparison

Accepted SP7bx baseline:

- BusyLoop: `8237` / `8230`, queue submits `175`
- KeccakUnion: `103559` / `101971`, queue submits `3083`
- xgboost: `91667` / `91215`, mean `91441.0`, queue submits `2740`

SP7bz:

- BusyLoop: `8305`, queue submits `173`
- KeccakUnion: `101434`, queue submits `3075`
- xgboost captured repeat: `91598`, queue submits `2718`

Movement:

- Queue submit reduction is deterministic: `-2` for one RV32IM segment, `-8` for four segments, `-22` for eleven segments.
- xgboost wall is effectively flat/slightly worse versus SP7bx mean: `91441.0 -> 91598` (`+157 ms`, about `+0.17%`).
- BusyLoop wall is slightly worse than the SP7bx pair, while KeccakUnion is inside the prior noisy range.

## Decision

Accept as a small structural cleanup, not as a counted wall-time win.

The change reduces WebGPU queue submissions without changing accumulator arithmetic and passed the representative proof-generation gate with zero fallback/CPU-only. The current xgboost working-state estimate remains about `91.4-91.6 s` on this browser/NVIDIA setup.

Do not use this result to justify another command-submission side path unless it shows clear wall movement. The next priority remains larger, xgboost-dominant fixed-work reductions in the prover path.
