# SP7cs Compact Witgen Transactions Rejected

Date: 2026-05-20

## Scope

Tried to reduce MISC0 GPU-witgen replacement metadata upload by compacting the
per-arm memory transaction buffer to only cycles actually dispatched by the
replacement arm mask.

## Candidate

- Replaced full `iter6d_g_arm_txns` upload with a compact transaction stream.
- Kept `iter6d_g_arm_txn_start` indexed by original cycle so WGSL call sites did
  not need a new cycle remapping.
- Added `iter6d_g_compact_txns` diagnostics reporting selected cycles, compact
  transaction count, and full transaction count.

## Validation

Compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in `4m23s`.

BusyLoop + KeccakUnion browser e2e:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Log: `/tmp/sp7cs-busy-keccak-compact-txns.log`

Result:

```text
test tests::iter6d_g_replace_busy_loop_e2e_verify ... ok
BusyLoop wall_ms=7230 gpu_active_ms=3175 raw_compute_dispatches=609 queue_submits=173
BusyLoop compact_txns=261707 full_txns=832996 iter6d_g_arm_txns upload_bytes=5234140
KeccakUnion wall_ms=92193 gpu_active_ms=60188 raw_compute_dispatches=10660 queue_submits=3075
KeccakUnion iter6d_g_arm_txns upload_bytes=10047080
cpu_fallbacks=0 cpu_only_ops=0
```

xgboost browser e2e:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_xgboost -- --nocapture
```

Log: `/tmp/sp7cs-xgboost-compact-txns.log`

Result:

```text
test tests::iter6d_g_replace_xgboost ... ok
wall_ms=77836
segments=11
gpu_active_ms=44737
raw_compute_dispatches=9674
queue_submits=2718
upload_bytes=2982655888
readback_bytes=7488592
cpu_fallbacks=0
cpu_only_ops=0
iter6d_g_arm_txns upload_bytes=59175400
```

Compactness:

```text
xgboost compact_txns=2958770 full_txns=9520134
iter6d_g_arm_txns upload_bytes: SP7cq 190402680 -> SP7cs 59175400
```

Hot xgboost buckets:

```text
77836 ms prove_session_async
38413 ms composite_to_succinct_async
29139 ms finalize_async fri_prove
27415 ms fri_prove round=0 domain_in=1048576
20190 ms join_async
11474 ms rv32im_witgen_accum
11117 ms rv32im_accumulate step_top_accum_cpu_skip_replaced_misc0=true_major_mask=0x0006
 9609 ms finalize_async check_group
 6251 ms recursion_witgen_accum
```

## Decision

Rejected.

Correctness was clean and upload pressure improved materially, but this did not
meet the wall-time gate:

- BusyLoop was slightly positive vs SP7cq: `7265 ms -> 7230 ms`.
- KeccakUnion regressed slightly: `91943 ms -> 92193 ms`.
- xgboost regressed from SP7cq's `77417/77523 ms` range to `77836 ms`
  (`+313 to +419 ms`, about `+0.4%` to `+0.5%`).

Because the current priority is immediate wall-time reduction rather than
neutral data-movement cleanup, the candidate was reverted back to full
transaction buffers.

Post-revert checks:

```text
cargo fmt --check --manifest-path risc0/zkp/Cargo.toml
git diff --check
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Results: all passed; the no-run compile passed in `4m18s`.

Accepted wall-time gain: `0`.

Do not retry compact transaction uploads as a standalone optimization unless it
is paired with a mechanism that removes enough CPU/GPU work to produce a
representative xgboost wall-time win while keeping BusyLoop and KeccakUnion
within gate.
