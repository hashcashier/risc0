Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7v -- coalesce Merkle query sample/sibling readback
Date: 2026-05-19

## Headline

Merkle query openings used two independent indexed readbacks when the sampled
matrix values and sibling nodes were both GPU-resident: one readback for
`evaluated` values and one readback for `nodes`. SP7v adds one two-buffer
indexed WebGPU readback and routes GPU-resident Merkle query openings through
`merkle_query`, preserving proof bytes while removing one browser map/readback
and one queue submit per opened Merkle tree.

Accepted as a production browser round-trip reduction:

- one fewer readback per GPU-resident Merkle query tree;
- one fewer queue submit per GPU-resident Merkle query tree;
- unchanged readback bytes, because the same sampled values and sibling nodes
  are still copied;
- receipt verification on the representative matrix: BusyLoop,
  `KeccakUnion(1)`, and xgboost;
- no CPU fallback or CPU-only ops in proof e2e.

Accepted wall-time reduction: `0 s` for now. The single-run wall movement is
small/noisy, so the accepted claim is structural command/readback reduction.

## RED And Invalid Attempt

The RED intent was to make representative proof e2e require `merkle_query`
readbacks during KeccakUnion. Pre-implementation SP7u diagnostics had no
`merkle_query` source and still showed separate query-phase sources:

```text
KeccakUnion SP7u:
queue_submits=3548
readbacks=961
nodes readbacks=514
evaluated readbacks=257
```

A first browser run after adding the assertion negotiated low WebGPU limits and
failed before reaching the intended `merkle_query` assertion:

```text
max_buffer_size=1073741824
max_storage_buffer_binding_size=1073741824
max_compute_workgroup_storage_size=32768
TopAccum candidate sync wait failed / device loss
```

This was discarded as invalid environment evidence, not treated as a
correctness RED. The harness must negotiate the high-limit device before the
run is counted.

## GREEN

Implemented:

- `WebGpuHal::read_two_buffer_indices_named`, which copies fixed-size elements
  from two arbitrary indexed GPU buffers into one MAP_READ staging buffer and
  maps it once.
- `read_webgpu_merkle_query`, which reads sampled `BabyBearElem` values and
  sibling `Digest`s together under the readback source name `merkle_query`.
- `MerkleTreeProver<WebGpuHal>::prove_batch_async` now takes the combined
  readback path when matrix and nodes are both GPU-resident, retaining the old
  separate paths for CPU-current or empty-proof cases.
- The representative authoritative test asserts that KeccakUnion produces at
  least one `merkle_query` readback.

Compile/no-run:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify --no-run

result: passed
```

Fast high-limit sanity:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_prover_commit_group_fuses_copy_into_interpolate -- --nocapture

webgpu-limits max_buffer_size=4294967292 \
  max_storage_buffer_binding_size=2147483644 \
  max_compute_workgroup_storage_size=49152
test result: ok
```

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
webgpu-limits max_buffer_size=4294967292 \
  max_storage_buffer_binding_size=2147483644 \
  max_compute_workgroup_storage_size=49152
test tests::rv32im_accum_topaccum_arm5_authoritative_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 114.32s
```

BusyLoop:

```text
wall_ms=10689 gpu_active_ms=7461 gpu_idle_ratio=0.302
gpu_dispatches=328 raw_compute_dispatches=712 queue_submits=180
readbacks=38 readback_bytes=478272
cpu_fallbacks=0 cpu_only_ops=0
```

KeccakUnion(1):

```text
wall_ms=103398 gpu_active_ms=69905 gpu_idle_ratio=0.324
segments=4 pending_keccaks=9 assumptions=1
gpu_dispatches=5904 raw_compute_dispatches=12679 queue_submits=3291
readbacks=704 readback_bytes=10183944
final_coeffs readbacks=38 readback_bytes=38400
merkle_query readbacks=257 readback_bytes=9033800
nodes readbacks=257 readback_bytes=271392
out readbacks=152 readback_bytes=840352
cpu_fallbacks=0 cpu_only_ops=0
```

Comparison to SP7u:

```text
BusyLoop queue_submits: 194 -> 180
BusyLoop readbacks: 52 -> 38

KeccakUnion wall_ms: 103636 -> 103398
KeccakUnion queue_submits: 3548 -> 3291
KeccakUnion readbacks: 961 -> 704
KeccakUnion nodes readbacks: 514 -> 257
KeccakUnion merkle_query readbacks: 0 -> 257
KeccakUnion readback_bytes: 10183944 -> 10183944
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
webgpu-limits max_buffer_size=4294967292 \
  max_storage_buffer_binding_size=2147483644 \
  max_compute_workgroup_storage_size=49152
test tests::xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 102.27s
```

Key metrics:

```text
wall_ms=102044 gpu_active_ms=61256 gpu_idle_ratio=0.400
segments=11
gpu_dispatches=5258 raw_compute_dispatches=11382 queue_submits=2870
readbacks=608 readback_bytes=7488592
final_coeffs readbacks=32 readback_bytes=32768
merkle_query readbacks=224 readback_bytes=6856000
nodes readbacks=224 readback_bytes=236544
out readbacks=128 readback_bytes=363280
cpu_fallbacks=0 cpu_only_ops=0
```

Comparison to SP7u:

```text
xgboost wall_ms: 102728 -> 102044
xgboost queue_submits: 3094 -> 2870
xgboost readbacks: 832 -> 608
xgboost nodes readbacks: 448 -> 224
xgboost merkle_query readbacks: 0 -> 224
xgboost readback_bytes: 7488592 -> 7488592
```

## Decision

Accepted. This removes one browser readback/map/submit per GPU-resident Merkle
query tree and is verified by real browser proof generation plus receipt
verification on BusyLoop, `KeccakUnion(1)`, and xgboost.

The structural reduction is large and deterministic:

```text
KeccakUnion readbacks: 961 -> 704 (-257)
KeccakUnion queue_submits: 3548 -> 3291 (-257)
xgboost readbacks: 832 -> 608 (-224)
xgboost queue_submits: 3094 -> 2870 (-224)
```

Wall movement is directionally positive but not claimed as material without
repeat A/B. Next likely target: reduce remaining per-tree query readbacks, or
batch more Merkle proof bytes into existing command buffers without changing
transcript semantics.
