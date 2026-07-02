# SP7al: canonical production-wall proofs keep candidate sync off

Date: 2026-05-19

## Problem

The SP7o/SP7p candidate-sync gate is useful instrumentation for generated
TopAccum candidates: it exposes hidden queued GPU work before
`commit_group_async rv32im_accum`.

The canonical representative wall-time proof gates had drifted into always
enabling that diagnostic sync. That made the wall tests non-production-shaped
and could lead to bad decisions by serializing the proof path around a
diagnostic wait.

## RED

Changed the retained representative assertions to require zero candidate-sync
waits while the tests still enabled the gate. The BusyLoop and KeccakUnion proof
receipts were generated and verified first, then the new assertion failed as
intended.

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

Primary failure:

```text
canonical production-wall representative workloads should not force candidate sync waits
```

Key RED metrics:

| Workload | wall_ms | raw dispatches | queue submits | uploads | upload_bytes | Notes |
|---|---:|---:|---:|---:|---:|---|
| BusyLoop po2_18 | 10622 | 710 | 174 | 217 | 504148788 | `candidate_sync_wait=3363 ms` |
| KeccakUnion(1) | 103226 | 12621 | 3169 | 3748 | 5910276364 | `pending_keccaks=9`, `assumptions=1` |

CPU fallback and CPU-only op counters stayed at zero.

## GREEN

The canonical BusyLoop + KeccakUnion and xgboost representative wall tests now
call `set_accum_gpu_candidate_sync_enabled(false)`, and the setter resets the
wait counter on both enable and disable so prior diagnostic runs cannot leak
into production-wall assertions.

## Representative e2e: BusyLoop + KeccakUnion

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

Result: passed in 113.65 s with high WebGPU limits, receipt verification, zero
CPU fallback/CPU-only ops, zero candidate-sync waits, zero `code` uploads.

| Workload | SP7ah wall_ms | SP7al wall_ms | raw dispatches | queue submits | uploads | upload_bytes |
|---|---:|---:|---:|---:|---:|---:|
| BusyLoop po2_18 | 10635 | 10473 | 710 | 174 | 217 | 504148788 |
| KeccakUnion(1) | 102226 | 102905 | 12621 | 3169 | 3748 | 5910276364 |

KeccakUnion retained representative shape:

- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`

The explicit `candidate_sync_wait` bucket disappeared. The same hidden cold work
is now visible in the first `commit_group_async rv32im_accum` timing bucket
(`3403 ms` on BusyLoop), so this is a measurement-shape correction, not removal
of that work.

## Representative e2e: xgboost

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies -- --nocapture
```

Result: passed in 100.70 s with high WebGPU limits, succinct receipt
verification, zero CPU fallback/CPU-only ops, zero candidate-sync waits, zero
`code` uploads.

| Workload | SP7ah wall_ms | SP7al wall_ms | segments | raw dispatches | queue submits | uploads | upload_bytes |
|---|---:|---:|---:|---:|---:|---:|---:|
| xgboost | 100335 | 100474 | 11 | 11331 | 2765 | 3359 | 7026039532 |

## Decision

Accepted as representative-test correction only.

Canonical wall tests now measure the production proof path with candidate sync
off. The diagnostic sync gate remains available for future generated TopAccum
candidate screening, where it should be enabled deliberately to expose hidden
queued work before deciding whether a candidate is viable.

Observed wall movement was mixed and within single-run browser noise:

- BusyLoop: -162 ms
- KeccakUnion: +679 ms
- xgboost: +139 ms

Accepted wall-time reduction: 0 s.

Immediate follow-up: do not chase submit-count-only or data-movement sidequests
without a credible e2e wall signal. The next high-impact target is the real
~3.4 s cold/hidden work now attributed to first-segment
`commit_group_async rv32im_accum`, plus broader generated-witgen/TopAccum work
only after that hidden work is bounded.
