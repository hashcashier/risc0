# SP7cr MISC2 No-Shadow Replacement Rejected

Date: 2026-05-20

## Scope

Revisited MISC2 GPU-witgen replacement after SP7bw's MISC2 direct accumulator
path. The candidate was distinct from SP7bv: it replayed MISC2 lookup side
effects from preflight and skipped MISC2 witness/accum shadow repair when direct
accum was active.

## Candidate

- Enabled replacement arms `0` and `2`.
- Replayed MISC2 lookup deltas from preflight instead of GPU-authored witness
  shadow rows.
- Required zero `witgen_data_shadow_rows` and zero
  `witgen_accum_shadow_rows` readbacks in representative tests.
- Added MISC2 extra replacement chunks to prewarm after the first run exposed
  five on-demand compiles (`misc2_chunk2..6`).

## Validation

Compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in `5m50s` before browser e2e and `4m30s` after the MISC2
prewarm repair.

BusyLoop + KeccakUnion browser e2e:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Log: `/tmp/sp7cr-busy-keccak-misc2-prewarm.log`

Result:

```text
test tests::iter6d_g_replace_busy_loop_e2e_verify ... ok
BusyLoop wall_ms=8693 gpu_active_ms=3219 raw_compute_dispatches=616 queue_submits=180
KeccakUnion wall_ms=91930 gpu_active_ms=60261 raw_compute_dispatches=10688 queue_submits=3103
cpu_fallbacks=0 cpu_only_ops=0
no witgen_data_shadow_rows/witgen_accum_shadow_rows readback sources
```

xgboost browser e2e:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_xgboost -- --nocapture
```

Log: `/tmp/sp7cr-xgboost-misc2-prewarm.log`

Result:

```text
test tests::iter6d_g_replace_xgboost ... ok
wall_ms=78328
segments=11
gpu_active_ms=44685
raw_compute_dispatches=9751
queue_submits=2795
upload_bytes=2987889616
readback_bytes=7488592
cpu_fallbacks=0
cpu_only_ops=0
```

Hot xgboost buckets:

```text
37944 ms composite_to_succinct_async
29123 ms finalize_async fri_prove
27370 ms fri_prove round=0 domain_in=1048576
19882 ms join_async
11529 ms rv32im_witgen_accum
11165 ms rv32im_accumulate step_top_accum_cpu_skip_replaced_misc0=true_major_mask=0x0006
 9583 ms finalize_async check_group
```

## Decision

Rejected.

Correctness was clean, and the no-shadow repair path worked, but performance
failed the representative wall gate:

- BusyLoop regressed from SP7cq `7265 ms` to `8693 ms` (`+1428 ms`, `+19.7%`).
- KeccakUnion was flat: SP7cq `91943 ms` vs candidate `91930 ms`.
- xgboost regressed from SP7cq `77417/77523 ms` to `78328 ms`
  (`+805 to +911 ms`, about `+1.0%` to `+1.2%`).

The candidate was reverted back to MISC0-only replacement. Post-revert checks:

```text
cargo fmt --check --manifest-path risc0/zkp/Cargo.toml
git diff --check
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Results: all passed; the final no-run compile passed in `4m21s`.

Accepted wall-time gain: `0`.

Do not retry MISC2 replacement as a broad arm unless the implementation removes
the extra row-list uploads/dispatch work or proves a material xgboost win while
keeping BusyLoop and KeccakUnion within the representative wall gate.
