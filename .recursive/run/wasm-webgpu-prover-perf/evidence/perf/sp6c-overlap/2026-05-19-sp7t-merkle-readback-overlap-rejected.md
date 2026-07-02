Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7t -- Merkle readback overlap experiment
Date: 2026-05-19

## Headline

Attempted to overlap per-tree Merkle sample and sibling-node readbacks inside
`MerkleTreeProver<WebGpuHal>::prove_batch_async` by issuing both independent
buffer-index readbacks before awaiting either result.

Correctness held on representative browser proof e2e, including
`KeccakUnion(1)`, and on xgboost. Performance did not improve, and raw command
or readback counts did not change. The experiment was reverted.

Accepted wall-time reduction: `0 s`.

## Experiment

Changed `prove_batch_async` from sequential:

1. read sampled matrix values;
2. compute Merkle sibling indices;
3. read sibling nodes;
4. serialize proofs in the original transcript order.

to an overlapped version:

1. compute sampled matrix indices;
2. compute sibling indices and proof counts;
3. create `samples_fut` and `siblings_fut`;
4. `try_join` both futures before serializing proofs.

The proof transcript output order was unchanged.

## Representative E2E

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

Result:

```text
test tests::rv32im_accum_topaccum_arm5_authoritative_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 115.84s
```

BusyLoop:

```text
wall_ms=10995 gpu_active_ms=7614 gpu_idle_ratio=0.308
gpu_dispatches=328 raw_compute_dispatches=712 queue_submits=208
cpu_fallbacks=0 cpu_only_ops=0
```

KeccakUnion(1):

```text
wall_ms=104607 gpu_active_ms=71084 gpu_idle_ratio=0.320
segments=4
gpu_dispatches=5904 raw_compute_dispatches=12679 queue_submits=3805
cpu_fallbacks=0 cpu_only_ops=0
```

Comparison to SP7s accepted baseline:

```text
KeccakUnion wall_ms: 104059 -> 104607
KeccakUnion raw_compute_dispatches: 12679 -> 12679
KeccakUnion queue_submits: 3805 -> 3805
```

## xgboost E2E

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies -- --nocapture
```

Result:

```text
test tests::xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 102.70s
```

Key metrics:

```text
wall_ms=102470 gpu_active_ms=61730 gpu_idle_ratio=0.398
segments=11
gpu_dispatches=5258 raw_compute_dispatches=11382 queue_submits=3318
cpu_fallbacks=0 cpu_only_ops=0
uploads=3662 upload_bytes=8289600740
readbacks=1056 readback_bytes=7488592
```

Comparison to SP7s accepted baseline:

```text
xgboost wall_ms: 102145 -> 102470
xgboost raw_compute_dispatches: 11382 -> 11382
xgboost queue_submits: 3318 -> 3318
xgboost readbacks: 1056 -> 1056
xgboost readback_bytes: 7488592 -> 7488592
```

## Post-Revert Current-Tree Validation

After reverting the Merkle overlap patch, the current tree was revalidated with
the representative browser proof matrix. The sandboxed browser run could not
start the `wasm-bindgen-test-runner` server (`Operation not permitted`), so the
same commands were run outside the sandbox.

Representative BusyLoop + KeccakUnion e2e:

```text
test tests::rv32im_accum_topaccum_arm5_authoritative_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 115.23s
```

BusyLoop:

```text
wall_ms=10658 gpu_active_ms=7437 gpu_idle_ratio=0.302
gpu_dispatches=328 raw_compute_dispatches=712 queue_submits=208
cpu_fallbacks=0 cpu_only_ops=0
```

KeccakUnion(1):

```text
wall_ms=104339 gpu_active_ms=70664 gpu_idle_ratio=0.323
segments=4 pending_keccaks=9 assumptions=1
gpu_dispatches=5904 raw_compute_dispatches=12679 queue_submits=3805
cpu_fallbacks=0 cpu_only_ops=0
readbacks=1218 readback_bytes=10183944
```

xgboost e2e:

```text
test tests::xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 103.01s
```

Key metrics:

```text
wall_ms=102787 gpu_active_ms=61807 gpu_idle_ratio=0.399
segments=11
gpu_dispatches=5258 raw_compute_dispatches=11382 queue_submits=3318
cpu_fallbacks=0 cpu_only_ops=0
uploads=3662 upload_bytes=8289600740
readbacks=1056 readback_bytes=7488592
```

## Decision

Rejected and reverted.

The change was correctness-positive but performance-negative/noisy on both
representative workloads. It also did not reduce the measured bottleneck
counters: raw compute dispatches, queue submits, readback count, or readback
bytes.

Do not retry this exact overlap shape unless a new design reduces readback or
submit counts, not just the local await ordering.
