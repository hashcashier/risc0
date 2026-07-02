# SP7ay: GPU-Witgen Candidate Screen

Date: 2026-05-19

## Scope

After the MISC1 replacement rejection, added a diff-only selector to screen
individual RV32IM witgen major arms without allowing an incomplete candidate
to affect proof output. The goal was to find an immediate, correctness-safe
GPU-witgen expansion target before spending implementation time on another
authoritative replacement.

This is diagnostic evidence only. These browser tests intentionally bail after
logging `DIFF_SUMMARY`; the useful signal is whether the candidate has:

- `mismatches = 0`
- `candidate_cpu_only_nonzero = 0`

Anything else is not safe to promote to proof generation.

## RED / GREEN

RED first imported a missing selector from the browser diagnostic test:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_diff_busy_loop_misc2 --no-run

error[E0432]: unresolved import
  `risc0_circuit_rv32im::prove::set_witgen_gpu_diff_major`
```

GREEN added:

- `set_witgen_gpu_diff_major(major: Option<u8>)`
- `is_witgen_diff_cycle`
- diff-only candidate filtering in `pre_witgen_dispatch_async`
- `candidate_cpu_only_nonzero` accounting in the final witness diff

The same no-run compile then passed. The expanded candidate-test compile also
passed:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_diff_busy_loop_mul0 --no-run

Finished `release` profile target(s)
wasm:
examples/target/wasm32-unknown-unknown/release/deps/browser_prove-d084df2f0d3bf6c6.wasm
```

## Diagnostic Matrix

All runs used BusyLoop po2=18, high-limit Chrome WebGPU, and the direct
`wasm-bindgen-test-runner` path from `examples/browser-prove` so
`webdriver.json` was applied:

```text
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152
```

| Candidate | Major | GPU wrote | Both match | Mismatches | Candidate CPU-only nonzero | Decision |
|---|---:|---:|---:|---:|---:|---|
| MISC2 | 2 | 10253350 | 10253350 | 0 | 463881 | Incomplete |
| MUL0 | 3 | 9187708 | 9187708 | 0 | 148 | Incomplete, low impact |
| DIV0 | 4 | 8533389 | 8533389 | 0 | 831 | Incomplete |
| MEM0 | 5 | 11605040 | 11605040 | 0 | 592025 | Incomplete |
| MEM1 | 6 | 11469798 | 11469570 | 228 | 349209 | Incorrect and incomplete |
| ECALL0 | 8 | 8928036 | 8905553 | 22483 | 356468 | Incorrect and incomplete |

The ECALL0 run also logged representative mismatches:

```text
DIFF_MISMATCH row=18521 col=128 gpu=0x38000001 cpu=0x52aaaaab
DIFF_SUMMARY total_cells=55312384 gpu_wrote=8928036 cpu_wrote=33114247
  both_match=8905553 mismatches=22483 gpu_only=0 cpu_only=24186211
  candidate_cpu_only_nonzero=356468 rows=262144 cols=211
```

The final panic in these diagnostic tests is the existing proof-path
diagnostic assertion:

```text
WebGPU proof path did not dispatch any GPU work
```

That assertion counts normal proof HAL ops and does not yet count the custom
pre-witgen diff dispatches. The `iter6d_g_per_arm_dispatch dispatched=1` and
`DIFF_SUMMARY` lines above are the relevant candidate-screen receipts.

## Decision

No candidate from this screen is safe to promote to authoritative replacement.
The important outcome is negative: broadening GPU witgen by another top-level
opcode chunk is not the next high-impact move.

MUL0 is closest, but it still misses 148 nonzero cells on BusyLoop and is a
small workload share. Fixing that path would be more likely to create another
correctness sidequest than a material wall-time reduction.

The next immediate performance work should target the larger remaining
bottleneck: the dense post-witgen `source=data` upload / CPU-shadow dependency,
or generate chunk-complete GPU-witgen kernels that include nested mux
dependencies rather than opportunistically selecting top-level chunks.

## Post-Screen Representative Proof Gates

The selector change touches pre-witgen dispatch plumbing, so the existing
MISC0 replacement proof gates were rerun after the diagnostic screen.

BusyLoop po2=18 plus `KeccakUnion(1)`:

```text
iter6d_g_replace_busy_loop_e2e_verify ... ok

BusyLoop:
  receipt verifies
  wall_ms=11587
  cpu_fallbacks=0
  cpu_only_ops=0
  source=data upload_bytes=355467264

KeccakUnion(1):
  receipt verifies
  wall_ms=102926
  segments=4
  pending_keccaks=9
  assumptions=1
  cpu_fallbacks=0
  cpu_only_ops=0
  source=data upload_bytes=4776263680
```

xgboost:

```text
iter6d_g_replace_xgboost ... ok

xgboost:
  receipt verifies
  wall_ms=103520
  segments=11
  cpu_fallbacks=0
  cpu_only_ops=0
  source=data upload_bytes=5252317184
  witgen_data_shadow_rows readback_bytes=406998464
```

## Wall-Time Assessment

Accepted wall-time gain from this step: 0.

This was a correctness gate that prevents bad runtime decisions. It preserves
the current MISC0-only opt-in replacement state and avoids investing in
incorrect or incomplete candidates before representative e2e proof gates.
