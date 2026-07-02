# SP7ct MISC1 Combined Source Regs Rejected

Date: 2026-05-20

## Scope

Revisited MISC1 GPU-witgen replacement after the earlier SP7ax rejection
identified a concrete missing nested source-register mux rather than a measured
wall-time loss.

## Candidate

- Added combined MISC1 source-register selection to the MISC1 delta WGSL.
- Tried a full MISC1 XORI/ORI shape with MISC1 replacement arm `1` enabled.
- Replayed MISC1 lookup side effects from preflight so CPU `step_Top` could be
  short-circuited for replaced rows.
- Added browser assertions that MISC1 replacement was active.
- After the full XORI/ORI shape showed high first-proof cost, narrowed the
  candidate to XORI-only and removed the chunk1/ORI path.

## Validation

Compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed after integration fixes; final successful candidate compile took
about `6m00s`.

Full MISC1 XORI/ORI browser e2e:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result:

```text
BusyLoop wall_ms=10815 gpu_active_ms=3181 raw_compute_dispatches=611 queue_submits=175
KeccakUnion wall_ms=91555 gpu_active_ms=60187 raw_compute_dispatches=10668 queue_submits=3083
replacement_mask=0x0003
cpu_fallbacks=0 cpu_only_ops=0
receipts verified
```

Narrowed XORI-only browser e2e:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result:

```text
BusyLoop wall_ms=10376 gpu_active_ms=3176 raw_compute_dispatches=610 queue_submits=174
KeccakUnion wall_ms=92040 gpu_active_ms=60408 raw_compute_dispatches=10664 queue_submits=3079
replacement_mask=0x0003
dispatched_arms=[0, 1]
cpu_fallbacks=0 cpu_only_ops=0
receipts verified
```

Notable timer evidence on the XORI-only run:

```text
iter6d_d_witgen_prewarm_async elapsed_ms=5212
iter6d_g_pre_witgen_dispatch_async elapsed_ms=4705
```

## Decision

Rejected and reverted.

Correctness was clean in both candidate shapes, but the representative wall gate
failed:

- Full MISC1 XORI/ORI regressed BusyLoop from SP7cq `7265 ms` to `10815 ms`
  (`+49%`), while KeccakUnion was slightly positive/noisy
  (`91943 ms -> 91555 ms`).
- Narrowed XORI-only still regressed BusyLoop from `7265 ms` to `10376 ms`
  (`+42.8%`) and left KeccakUnion effectively flat/slightly worse
  (`91943 ms -> 92040 ms`).

No xgboost run was retained for this candidate because BusyLoop and KeccakUnion
already failed the representative wall gate. Accepted wall-time gain: `0`.

Post-revert checks:

```text
cargo fmt --check --manifest-path risc0/zkp/Cargo.toml
git diff --check
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Results: all passed; no-run compile passed in `4m23s`.

Do not retry MISC1 replacement in this per-proof prewarm shape. It is only worth
revisiting if replacement kernels are compiled/cached ahead of proof time, or if
the row-list/prewarm overhead is removed enough to avoid first-proof BusyLoop
regression while preserving KeccakUnion and xgboost correctness.
