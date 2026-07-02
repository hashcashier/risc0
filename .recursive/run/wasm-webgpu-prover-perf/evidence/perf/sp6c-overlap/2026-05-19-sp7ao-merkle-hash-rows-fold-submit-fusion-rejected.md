# SP7ao: Merkle hash_rows + fold-chain submit fusion rejected

Date: 2026-05-19

## Purpose

Test whether Merkle tree construction could reduce browser proving wall time by
dispatching `hash_rows` and the full `hash_fold_chain` in one WebGPU submit.
This targeted a real repeated bucket: FRI `merkle_new` remains about 0.85-0.9 s
for the large round in each RV32IM/recursion proof, and KeccakUnion/xgboost
construct hundreds of Merkle trees.

## RED

Added focused browser HAL coverage requiring a fused helper to:

- produce byte-identical Merkle nodes versus the existing separate
  `hash_rows` + `hash_fold` sequence,
- use one `queue_submit`,
- keep the same four raw dispatches for an 8-row toy tree.

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_core_gpu_results_match_cpu -- --nocapture
```

Result: compile RED failed on the missing
`WebGpuHal::debug_hash_rows_fold_chain` method.

## GREEN Prototype

Added a prototype `hash_rows_fold_chain_async` path that submitted one compute
pass containing:

1. `poseidon2_rows` for leaves,
2. the existing dynamic-uniform `poseidon2_fold` dispatches for each parent
   layer.

The first focused run exposed a test-scope issue: without a GPU-authoritative
scope, the helper correctly fell back to the old path and reported
`queue_submits=4`. After tightening the focused test to match production async
scope, the focused browser HAL test passed with high WebGPU limits.

## Representative E2E

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_default_representative_e2e_verify -- --nocapture
```

Result: passed with high WebGPU limits, real receipt verification, zero CPU
fallback/CPU-only ops, zero candidate-sync waits, zero arm5 dispatches, and
zero `code` uploads.

| Workload | Baseline wall_ms | Prototype wall_ms | queue submits | raw dispatches | readbacks |
|---|---:|---:|---:|---:|---:|
| BusyLoop default | 7473 | 7605 | 160 -> 146 | 706 -> 706 | 22 -> 22 |
| KeccakUnion(1) default | 102622 | 103090 | 2972 -> 2715 | 12605 -> 12605 | 409 -> 409 |

KeccakUnion retained representative shape:

- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`

## Decision

Rejected and removed. The prototype was correctness-positive and structurally
reduced submits, but wall time moved the wrong direction on the representative
BusyLoop + KeccakUnion gate. xgboost was not run because KeccakUnion already
failed the wall-time acceptance criterion.

Do not pursue submit-only Merkle construction fusion again unless it is paired
with a real hash-compute improvement or repeated e2e wall evidence.
