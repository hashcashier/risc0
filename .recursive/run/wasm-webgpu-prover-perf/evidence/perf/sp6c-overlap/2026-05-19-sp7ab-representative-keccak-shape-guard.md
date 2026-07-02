# SP7ab - Representative KeccakUnion workload shape guard

Date: 2026-05-19

## Change

Tightened the canonical browser representative proof gate
`rv32im_accum_topaccum_arm5_authoritative_e2e_verify` so `KeccakUnion(1)`
is not only proved and verified, but also preflighted through the executor.
The guard now asserts that the fixture produces a top-level session segment,
pending keccak proof requests, and assumption resolution work before the
succinct receipt proof runs.

This makes the current representative browser matrix explicit:

- BusyLoop: plain RV32IM proof at po2=18.
- KeccakUnion(1): pending keccak proofs plus assumption resolution.
- xgboost: multi-segment RV32IM workload.

## RED / invalid environment check

Command without `VK_ICD_FILENAMES`:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

Result: invalid for performance/correctness evidence. Chrome negotiated low
WebGPU limits and the BusyLoop proof lost the WebGPU device before receipt
verification:

```text
max_buffer_size=1073741824
max_storage_buffer_binding_size=1073741824
max_compute_workgroup_storage_size=32768
TopAccum candidate sync wait failed
OperationError: A valid external Instance reference no longer exists.
```

Fast probe with the NVIDIA Vulkan ICD pinned passed and restored the intended
high-limit device:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  sp7_ext_inv_formula_matches_baby_bear_on_chrome -- --nocapture
```

```text
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152
test result: ok
```

## RED - first shape assertion

The first guard incorrectly asserted that `KeccakUnion(1)` must have multiple
top-level executor segments. The high-limit e2e proved BusyLoop, then failed
at the new guard before proving KeccakUnion:

```text
browser-prove:representative-keccak-union proof_count=1 segments=1 pending_keccaks=9 assumptions=1
KeccakUnion representative workload should exercise multi-segment proving
```

This clarified the workload split: `KeccakUnion(1)` is representative because
it produces pending keccak proof requests and assumption resolution work.
xgboost remains the multi-segment representative gate.

## GREEN - BusyLoop + KeccakUnion proof gate

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

Result: passed with high WebGPU limits, workload-shape guard active, succinct
receipt verification, and zero CPU fallback.

BusyLoop:

```text
wall_ms=10621
gpu_active_ms=7385
gpu_idle_ratio=0.305
segments=1
queue_submits=174
readbacks=32
readback_bytes=478272
uploads=219
upload_bytes=515931420
cpu_fallbacks=0
cpu_only_ops=0
```

KeccakUnion preflight:

```text
proof_count=1
top_level_session_segments=1
pending_keccaks=9
assumptions=1
```

KeccakUnion proof:

```text
wall_ms=103210
gpu_active_ms=69290
gpu_idle_ratio=0.329
segments=4
pending_keccaks=9
assumptions=1
queue_submits=3177
readbacks=590
readback_bytes=10183944
uploads=3835
upload_bytes=6415692924
readback_sources:
  final_coeffs=38
  merkle_query=257
  nodes=257
  out=38
cpu_fallbacks=0
cpu_only_ops=0
```

## GREEN - xgboost proof gate

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

Result: passed with high WebGPU limits, succinct receipt verification, and
zero CPU fallback.

```text
wall_ms=101593
gpu_active_ms=60858
gpu_idle_ratio=0.401
segments=11
queue_submits=2774
readbacks=512
readback_bytes=7488592
uploads=3429
upload_bytes=7506769904
readback_sources:
  final_coeffs=32
  merkle_query=224
  nodes=224
  out=32
cpu_fallbacks=0
cpu_only_ops=0
```

## Decision

Keep the KeccakUnion shape guard. It does not reduce wall time directly, but
it prevents future e2e runs from silently losing the pending-keccak /
assumption-resolution workload class. Require `VK_ICD_FILENAMES` in browser
performance runs unless a fast WebGPU probe has already confirmed high
limits.
