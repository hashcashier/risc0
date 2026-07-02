# SP7ca: Poseidon2 hash_rows Workgroup 128 Rejected

Date: 2026-05-20
Status: rejected

## Candidate

Reduce only the Poseidon2 `hash_rows` WebGPU entry workgroup size from `256` to `128`, keeping `hash_fold` unchanged.

Reason for testing:

- Latest xgboost SP7bz profile spends `27569 ms` across 32 `fri_prove round=0 merkle_new rows=65536 cols=64` calls.
- `hash_rows` leaf hashing is the dominant work inside the large Merkle build, so this was a direct compute-path candidate rather than another submit-count-only change.

## Compile

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result with candidate: passed in `4m49s`.

## Representative E2E

Initial captured run failed before proof generation because the wasm-bindgen local server could not spawn in the sandbox:

```text
Error: failed to spawn server
Caused by:
    Operation not permitted (os error 1)
```

This was not a proof/verifier failure. The same command was rerun with escalation for the local browser test server.

Command:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: passed in `112.28s`, with real BusyLoop and KeccakUnion receipt verification and zero fallback/CPU-only.

BusyLoop:

- `wall_ms=9048`
- `gpu_active_ms=4036`
- `gpu_idle_ratio=0.554`
- `raw_compute_dispatches=721`
- `queue_submits=173`
- `upload_bytes=209750328`
- `readback_bytes=478272`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- large Merkle `fri_prove round=0 merkle_new`: `875 ms`, `858 ms`

KeccakUnion:

- `wall_ms=102917`
- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`
- `gpu_active_ms=69751`
- `gpu_idle_ratio=0.322`
- `raw_compute_dispatches=12716`
- `queue_submits=3075`
- `upload_bytes=2991671068`
- `readback_bytes=10183944`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- large Merkle `fri_prove round=0 merkle_new` samples stayed around `855-875 ms`

## Comparison

Accepted SP7bz baseline:

- BusyLoop `wall_ms=8305`, submits `173`
- KeccakUnion `wall_ms=101434`, submits `3075`
- large Merkle samples in the same range, about `860-875 ms`

Candidate:

- BusyLoop `8305 -> 9048` (`+743 ms`, `+8.9%`)
- KeccakUnion `101434 -> 102917` (`+1483 ms`, `+1.46%`)
- Queue submit and raw dispatch counts unchanged.
- Large Merkle round timings did not improve.

## Decision

Rejected before xgboost. The representative gate was correctness-clean, but the candidate moved wall time the wrong direction on both required workloads and did not improve the targeted `merkle_new` timing.

The code was reverted to `@workgroup_size(256)` for `poseidon2_rows`; post-revert compile passed:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in `4m51s`.

`git diff --check` is clean.

Follow-up: do not continue simple Poseidon2 workgroup-size tuning as an immediate lever. Merkle remains a major wall bucket, but future attempts need a real algorithmic/hash-kernel improvement rather than occupancy tuning.
