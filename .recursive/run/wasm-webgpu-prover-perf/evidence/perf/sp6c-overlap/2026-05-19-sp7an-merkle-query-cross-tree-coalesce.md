# SP7an: cross-tree Merkle query readback coalescing

Date: 2026-05-19

## Purpose

Reduce proving wall time on the production default path by collapsing FRI
query-phase Merkle readbacks across trees. Before this change,
`fri_prove_async` batched queries within each Merkle tree, but still mapped one
browser readback per tree. The production default matrix still showed this as a
large round-trip source:

- KeccakUnion default: `merkle_query readbacks=257`.
- xgboost default: `merkle_query readbacks=224`.

This is a direct wall-time target, not a diagnostic sidequest: it removes
browser map/submit round trips from every representative proof while preserving
the same proof bytes and transcript order.

## RED

Added `assert_merkle_query_readbacks_coalesced`, requiring
`merkle_query <= 2 * final_coeffs` so each proof may use at most one inner-tree
query readback and one FRI-round query readback.

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

Result: failed after real receipt generation and verification. BusyLoop
verified first, then KeccakUnion verified and failed the new assertion:

```text
merkle_query=257
final_coeffs=38
```

The failing run retained the representative KeccakUnion shape:

- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`

## Implementation

- Added `WebGpuIndexedReadbackGroup` and
  `WebGpuHal::read_buffer_index_groups_named`, which copies arbitrary indexed
  elements from multiple GPU buffers into one staging buffer and one browser
  map operation.
- Added `prove_batch_for_trees_async` in WebGPU Merkle proving. It preserves
  per-tree/per-query proof ordering while reading all inner-tree openings in one
  `merkle_query` map and all FRI-round openings in one `merkle_query` map.
- Changed `fri_prove_async` to call the cross-tree helper for both inner Merkle
  trees and FRI rounds.
- Tightened default representative and xgboost gates to enforce the new
  readback shape.

## GREEN: default representative e2e

First GREEN attempt hit the known low-limit Chrome profile and was rejected by
the high-limit guard:

```text
max_buffer_size=1073741824
max_storage_buffer_binding_size=1073741824
max_compute_workgroup_storage_size=32768
```

The rerun negotiated high limits and passed:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_default_representative_e2e_verify -- --nocapture
```

Result: passed in 110.49 s with receipt verification, high WebGPU limits, zero
CPU fallback/CPU-only ops, zero candidate-sync waits, zero opt-in arm5
dispatches, and zero `code` uploads.

| Workload | SP7am wall_ms | SP7an wall_ms | readbacks | merkle_query | queue submits |
|---|---:|---:|---:|---:|---:|
| BusyLoop default | 7542 | 7492 | 32 -> 22 | 14 -> 4 | 170 -> 160 |
| KeccakUnion(1) default | 103542 | 102726 | 590 -> 409 | 257 -> 76 | 3153 -> 2972 |

KeccakUnion retained representative shape:

- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`

## GREEN: xgboost e2e

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_succinct_receipt_verifies -- --nocapture
```

Result: passed in 99.76 s with high WebGPU limits, succinct receipt
verification, journal `30.528042544062632`, zero CPU fallback/CPU-only ops, zero
`code` uploads, and retained eval-u readback coalescing.

| Workload | SP7am wall_ms | SP7an wall_ms | segments | readbacks | merkle_query | queue submits |
|---|---:|---:|---:|---:|---:|---:|
| xgboost default | 100056 | 99537 | 11 | 512 -> 352 | 224 -> 64 | 2721 -> 2561 |

Raw compute dispatches and uploads were unchanged where expected:

- `raw_compute_dispatches=11287`
- `uploads=3315`
- `upload_bytes=7024315252`

## Repeat: xgboost e2e

The same xgboost proof gate was rerun before starting another optimization
candidate, to check whether the observed wall movement was a stable enough
signal to guide prioritization.

Result: passed with high WebGPU limits, succinct receipt verification, zero CPU
fallback/CPU-only ops, and the same command/readback shape:

- `wall_ms=99847`
- `gpu_active_ms=57092`
- `segments=11`
- `raw_compute_dispatches=11287`
- `queue_submits=2561`
- `uploads=3315`
- `upload_bytes=7024315252`
- `readbacks=352`
- `readback_bytes=7488592`
- `merkle_query=64`
- `nodes=224`
- `out=32`
- `final_coeffs=32`

Two SP7an xgboost trials are `99537` ms and `99847` ms, for a mean of
`99692` ms. Compared with the SP7am single-trial xgboost baseline of `100056`
ms, the repeated mean is `364` ms faster.

## Hygiene

```bash
cargo fmt --manifest-path examples/browser-prove/Cargo.toml
git diff --check
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_default_representative_e2e_verify --no-run
```

All passed. The post-format wasm compile produced the expected test binary.

## Decision

Accepted as a production default-path improvement. It is correctness-proven on
BusyLoop, KeccakUnion, and xgboost with real browser proof generation and
receipt verification.

Observed single-trial wall movement versus SP7am:

- BusyLoop: `7542 -> 7492` ms (-50 ms)
- KeccakUnion: `103542 -> 102726` ms (-816 ms)
- xgboost: `100056 -> 99537` ms (-519 ms)
- xgboost repeated mean: `100056 -> 99692` ms (-364 ms)

Accepted structural reduction:

- KeccakUnion `merkle_query`: `257 -> 76`
- xgboost `merkle_query`: `224 -> 64`
- KeccakUnion queue submits: `3153 -> 2972`
- xgboost queue submits: `2721 -> 2561`

Current accepted wall gain is modest: about 0.8 s on the single KeccakUnion
trial and 0.36 s on the repeated xgboost mean. The structural readback/query
reduction is strong, but this does not justify more small readback-only
sidequests without a clearer wall-time thesis.
