# SP7ai recursion accum noise CPU-shadow rejected

Date: 2026-05-19

## Candidate

The recursion accumulator is still CPU-authoritative. The candidate wrote
recursion accumulator ZK noise directly into the WebGPU CPU shadow instead of
using `eltwise_copy_elem_slice`, avoiding a GPU copy/upload before the CPU
accumulation loop dirtied the same buffer again.

## RED

Added retained proof-gate assertions on aggregate `accum` upload bytes.

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

Expected failure before implementation:

- BusyLoop receipt verified first, zero CPU fallback/CPU-only.
- KeccakUnion receipt verified first, zero CPU fallback/CPU-only.
- Assertion then failed with KeccakUnion `accum upload_bytes=1007157248`,
  above the temporary threshold.

## GREEN

Same BusyLoop + KeccakUnion command after implementation passed:

- BusyLoop receipt verified, zero CPU fallback/CPU-only.
- KeccakUnion receipt verified, zero CPU fallback/CPU-only.
- KeccakUnion `accum upload_bytes 1007157248 -> 692584448`.
- KeccakUnion `uploads 3748 -> 3698`, `queue_submits 3169 -> 3144`.
- KeccakUnion wall moved the wrong direction in the single trial:
  `102226 -> 103757` ms versus SP7ah.

xgboost proof gate:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies -- --nocapture
```

Result:

- xgboost receipt verified, zero CPU fallback/CPU-only.
- `segments=11`.
- `accum upload_bytes 1716518912 -> 1452277760`.
- total upload bytes `7026039532 -> 6760766224`.
- `uploads 3359 -> 3317`, `queue_submits 2765 -> 2744`.
- wall moved the wrong direction in the single trial:
  `100335 -> 100964` ms versus SP7ah.

## Rejection

Rejected and reverted. Although the candidate removed 21-25 submits and
~265-315 MB of uploads on representative workloads, both KeccakUnion and
xgboost moved the wrong way in single-trial wall time. Under the updated
priority, keep work focused on immediate significant wall-time reductions, not
data-movement sidequests without a wall signal.

