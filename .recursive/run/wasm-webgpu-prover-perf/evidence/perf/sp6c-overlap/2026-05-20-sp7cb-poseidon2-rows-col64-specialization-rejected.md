# SP7cb: Poseidon2 hash_rows cols=64 Specialization Rejected

Date: 2026-05-20
Status: rejected

## Candidate

Add a specialized WebGPU entry point for `poseidon2_rows` when `col_size == 64`.

Rationale:

- SP7bz xgboost spends `27569 ms` across 32 `fri_prove round=0 merkle_new rows=65536 cols=64` calls.
- SP7ca showed workgroup-size tuning does not help this bucket.
- This candidate removed the generic `used` counter / dynamic absorb loop for the dominant 64-column leaf hash shape while leaving the generic path unchanged for other column counts.

## Compile

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result with candidate: passed in `4m53s`.

## Representative E2E

Command:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: passed in `111.73s`, with real BusyLoop and KeccakUnion receipt verification and zero fallback/CPU-only.

BusyLoop:

- `wall_ms=8780`
- `gpu_active_ms=4032`
- `gpu_idle_ratio=0.541`
- `raw_compute_dispatches=721`
- `queue_submits=173`
- `upload_bytes=209721344`
- `readback_bytes=478272`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- large Merkle samples stayed in the prior band: `875 ms` and `857 ms`

KeccakUnion:

- `wall_ms=102650`
- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`
- `gpu_active_ms=69692`
- `gpu_idle_ratio=0.321`
- `raw_compute_dispatches=12716`
- `queue_submits=3075`
- `upload_bytes=2991920632`
- `readback_bytes=10183944`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- large Merkle samples stayed around `855-875 ms`

## Comparison

Accepted SP7bz baseline:

- BusyLoop `wall_ms=8305`
- KeccakUnion `wall_ms=101434`
- same raw dispatch / queue submit counts
- large Merkle samples around `855-875 ms`

Candidate:

- BusyLoop `8305 -> 8780` (`+475 ms`, about `+5.7%`)
- KeccakUnion `101434 -> 102650` (`+1216 ms`, about `+1.2%`)
- `raw_compute_dispatches` and `queue_submits` unchanged.
- Targeted `merkle_new rows=65536 cols=64` timings did not improve.

## Decision

Rejected before xgboost. Correctness was clean, but the candidate moved wall time the wrong direction on both representative workloads and did not improve the targeted Merkle leaf-hash timing.

The code was reverted to the generic `poseidon2_rows` entry point for all column sizes.

Post-revert compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in `4m51s`.

`git diff --check` is clean.

Follow-up: do not continue small Poseidon2 leaf-kernel surface rewrites as immediate levers. The Merkle bucket remains large, but meaningful wall improvement likely requires a deeper Poseidon2 parallelization or different hash/Merkle strategy rather than loop-shape specialization.
