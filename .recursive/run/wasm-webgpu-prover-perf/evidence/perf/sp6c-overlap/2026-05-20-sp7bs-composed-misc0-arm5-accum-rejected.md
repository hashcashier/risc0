Run: `.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7bs - composed MISC0 direct + arm5 authoritative accumulator`
DraftedAt: `2026-05-20`
Status: `REJECTED`

## Summary

SP7bs tested whether the already-correct TopAccum arm5 authoritative path
could compose with the GPU-owned MISC0 accumulator path. The correctness
result was positive, but the performance result was not: xgboost regressed
from the accepted SP7br wall `94726 ms` to `95342 ms`, and BusyLoop regressed
from `8805 ms` to `11920 ms`. The candidate was reverted.

The accepted working state remains SP7br. A post-revert xgboost proof
rerun passed at `94385 ms` with zero fallback/CPU-only ops, confirming the
revert restored the latest working path.

## RED

Representative proof tests were first tightened to require arm5
authoritative accumulator dispatches while MISC0 direct accumulator
replacement was enabled:

- `iter6d_g_replace_busy_loop_e2e_verify`
- `iter6d_g_replace_xgboost`

The RED run failed after a valid BusyLoop proof and receipt verification
because the existing `step_accum` selected the MISC0-direct branch and
returned before arm5 authoritative dispatch:

```text
GPU authoritative arm5 accumulator must compose with BusyLoop GPU-owned MISC0 rows
```

## Candidate

The candidate:

- added a CPU raw-step helper that skipped both replaced MISC0 rows and
  major 5
- dispatched GPU MISC0 direct and TopAccum arm5 authoritative in one
  accumulator path
- sparse-uploaded `accum` before arm5 dispatch to avoid overwriting
  GPU-written MISC0 rows with CPU invalid/zero cells

## Candidate Validation

Compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run

PASS in 4m22s
```

BusyLoop + KeccakUnion browser proof e2e:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture

PASS
BusyLoop: wall_ms=11920, gpu_active_ms=7436, gpu_idle_ratio=0.376,
  segments=1, cpu_fallbacks=0, cpu_only_ops=0,
  upload_bytes=209129624, readback_bytes=478272
KeccakUnion: wall_ms=101405, gpu_active_ms=68934, gpu_idle_ratio=0.320,
  segments=4, pending_keccaks=9, assumptions=1,
  upload_bytes=3006713624, readback_bytes=10183944,
  cpu_fallbacks=0, cpu_only_ops=0
```

xgboost browser proof e2e:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_xgboost -- --nocapture

PASS
xgboost: wall_ms=95342, segments=11, gpu_active_ms=59827,
  gpu_idle_ratio=0.373, cpu_fallbacks=0, cpu_only_ops=0,
  upload_bytes=3148014452, readback_bytes=7488592,
  journal=30.528042544062632
```

## Rejection Basis

Compared with SP7br:

- xgboost wall regressed `94726 -> 95342` (`+616 ms`, `+0.65%`)
- BusyLoop wall regressed `8805 -> 11920` (`+3115 ms`, `+35.4%`)
- KeccakUnion was essentially flat (`101902 -> 101405`, single-run noise)
- xgboost upload moved only `3193502120 -> 3148014452`
  (`-45486668`, `-1.4%`)
- raw GPU work increased (`gpu_active_ms 56364 -> 59827`)

Correctness was not the blocker. The blocker is that the extra arm5
GPU work and sparse-bridge overhead do not produce a representative wall
win. This violates the current prioritization rule: accept only immediate,
meaningful wall-time improvements or deterministic data-movement cuts that
do not materially regress wall time.

## Revert Validation

The candidate runtime/test changes were manually reverted. Post-revert
compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run

PASS in 4m26s
```

Post-revert xgboost browser proof e2e:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_xgboost -- --nocapture

PASS
xgboost: wall_ms=94385, segments=11, gpu_active_ms=56094,
  gpu_idle_ratio=0.406, cpu_fallbacks=0, cpu_only_ops=0,
  upload_bytes=3193577424, readback_bytes=7488592,
  journal=30.528042544062632
```

This restored the accepted SP7br shape:

- dense `source=accum` absent
- dense `source=recursion_data` absent
- `witgen_accum_shadow_rows` readback absent
- zero CPU fallback and zero CPU-only ops

## Decision

Reject SP7bs. Do not retry arm5 composition with MISC0 direct unless a
new profile shows a concrete wall-time mechanism that outweighs the added
GPU/sparse-bridge work. The next useful target should be the confirmed
large remaining sparse upload streams or another xgboost-dominant blocker,
not broader TopAccum-arm composition.
