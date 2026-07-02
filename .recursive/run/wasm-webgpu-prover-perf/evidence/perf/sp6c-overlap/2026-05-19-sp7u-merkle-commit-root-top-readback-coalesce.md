Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7u -- coalesce committed Merkle root/top readback
Date: 2026-05-19

## Headline

Committed WebGPU Merkle trees used to read `nodes[1]` for the root during
construction, then immediately read the top layer during `commit_async`. SP7u
adds a committed-constructor path that reads root and top layer with one indexed
`nodes` readback, then writes the same transcript bytes in the same order.

Accepted as a production browser round-trip reduction:

- one fewer `nodes` readback per committed Merkle tree;
- one fewer queue submit per committed Merkle tree;
- no change to proof transcript semantics;
- no CPU fallback or CPU-only ops in proof e2e.

Accepted wall-time reduction: `0 s` for now. Single-run wall moved in the right
direction on KeccakUnion and was basically flat on xgboost; treat that as
directional/noisy, not a material wall claim.

## RED

Extended the focused browser commit-group test to require committed WebGPU
Merkle construction to read root and top layer in one `nodes` readback.

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_prover_commit_group_fuses_copy_into_interpolate -- --nocapture
```

Result:

```text
test tests::webgpu_prover_commit_group_fuses_copy_into_interpolate ... FAIL
assertion `left == right` failed: committed WebGPU Merkle construction should read root and top layer in one nodes readback
left: 2
right: 1
```

RED diagnostics:

```text
readbacks=2 readback_bytes=544
readback_sources=[nodes readbacks=2 readback_bytes=544]
gpu_dispatches=10 raw_compute_dispatches=16 queue_submits=10
cpu_fallbacks=0 cpu_only_ops=0
```

## GREEN

Implemented:

- `MerkleTreeProver<WebGpuHal>::new_committed_async`
- `PolyGroup<WebGpuHal>::new_committed_async`
- WebGPU `Prover::commit_group_async{,_in_place}` now use the committed
  PolyGroup constructor.
- WebGPU `finalize_async` check group uses the committed constructor.
- WebGPU FRI rounds use the committed Merkle constructor.

Focused GREEN:

```text
test tests::webgpu_prover_commit_group_fuses_copy_into_interpolate ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 0.12s
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
test tests::rv32im_accum_topaccum_arm5_authoritative_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 114.58s
```

BusyLoop:

```text
wall_ms=10713 gpu_active_ms=7484 gpu_idle_ratio=0.301
gpu_dispatches=328 raw_compute_dispatches=712 queue_submits=194
readbacks=52 readback_bytes=478272
cpu_fallbacks=0 cpu_only_ops=0
```

KeccakUnion(1):

```text
wall_ms=103636 gpu_active_ms=70563 gpu_idle_ratio=0.319
segments=4 pending_keccaks=9 assumptions=1
gpu_dispatches=5904 raw_compute_dispatches=12679 queue_submits=3548
readbacks=961 readback_bytes=10183944
nodes readbacks=514 readback_bytes=4796192
cpu_fallbacks=0 cpu_only_ops=0
```

Comparison to post-SP7t current-tree baseline:

```text
BusyLoop queue_submits: 208 -> 194
BusyLoop readbacks: 66 -> 52

KeccakUnion wall_ms: 104339 -> 103636
KeccakUnion queue_submits: 3805 -> 3548
KeccakUnion readbacks: 1218 -> 961
KeccakUnion nodes readbacks: 771 -> 514
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
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 102.97s
```

Key metrics:

```text
wall_ms=102728 gpu_active_ms=61672 gpu_idle_ratio=0.400
segments=11
gpu_dispatches=5258 raw_compute_dispatches=11382 queue_submits=3094
readbacks=832 readback_bytes=7488592
nodes readbacks=448 readback_bytes=4383744
cpu_fallbacks=0 cpu_only_ops=0
```

Comparison to post-SP7t current-tree baseline:

```text
xgboost wall_ms: 102787 -> 102728
xgboost queue_submits: 3318 -> 3094
xgboost readbacks: 1056 -> 832
xgboost nodes readbacks: 672 -> 448
```

## Decision

Accepted as a structural readback/submit reduction. It removes one browser
round trip per committed Merkle tree and preserves receipt verification on the
current representative matrix: BusyLoop, `KeccakUnion(1)`, and xgboost.

The wall movement is not large enough to claim as material without repeated
A/B. The next likely target is to reduce remaining query-phase Merkle proof
readbacks (`nodes` and `evaluated`) or reduce raw hash-fold dispatch count.
