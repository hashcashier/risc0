# SP7ee - GPU-Witgen Transaction Compaction Rejected

Date: 2026-05-21

## Candidate

Compact the GPU-witgen replacement transaction payload. The default
replacement path dispatches MISC0 plus MEM0-LW rows, but
`dispatch_witgen_per_arm_probe` uploaded transaction records for every
preflight cycle. The candidate kept `txn_start` indexed by global cycle, while
packing only transactions for selected replacement cycles into
`iter6d_g_arm_txns`.

This targeted existing GPU-witgen overhead without enabling another replacement
arm that prior evidence had already rejected.

## RED

Temporary RED assertion added to `xgboost_succinct_receipt_verifies`:

```text
assert_upload_bytes_bounded("xgboost", ..., "iter6d_g_arm_txns", 120_000_000)
```

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture > /tmp/sp7ee-witgen-txn-compact-red.log 2>&1
```

Result: expected RED after successful proof generation. The run negotiated high
WebGPU limits, verified the xgboost proof/journal before the assertion, and
kept fallback counters clean.

Key lines:

```text
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
prove_session_async wall_ms=72210.0 gpu_active_ms=45116.0 gpu_idle_ratio=0.375
cpu_fallbacks=0 cpu_only_ops=0
source=iter6d_g_arm_txns uploads=10 upload_bytes=172397940
xgboost: WebGPU upload source `iter6d_g_arm_txns` exceeded bound: upload_bytes=172397940 max_bytes=120000000
```

## GREEN

Compile command:

```text
env CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies --no-run > /tmp/sp7ee-witgen-txn-compact-compile.log 2>&1
```

Result: PASS in `4m21s` with only pre-existing dead-code warnings.

xgboost GREEN command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture > /tmp/sp7ee-witgen-txn-compact-xgboost-green.log 2>&1
```

Result: PASS with verified xgboost journal, high WebGPU limits, and zero
fallback/CPU-only operations.

Key lines:

```text
iter6d_g_arm_txns compacted selected_cycles=150044 selected_txns=584866 full_txns=916707
...
prove_session_async wall_ms=72544.0 gpu_active_ms=45471.0 gpu_idle_ratio=0.373
cpu_fallbacks=0 cpu_only_ops=0
source=iter6d_g_arm_txns uploads=10 upload_bytes=104390820
test result: ok. 1 passed; 0 failed; 0 ignored; 151 filtered out; finished in 73.10s
```

Data movement improved:

```text
iter6d_g_arm_txns: 172397940 -> 104390820 bytes (-68007120 bytes, -39.4%)
total upload bytes: 2766605656 -> 2698458452 bytes (-68147204 bytes, -2.5%)
```

But wall regressed versus the fresh current default baseline:

```text
SP7ed fresh xgboost baseline: 72.76s
SP7ee xgboost candidate:     73.10s
Movement:                    +0.34s / +0.5%
```

## Representative BusyLoop + KeccakUnion

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture > /tmp/sp7ee-witgen-txn-compact-busy-keccak.log 2>&1
```

Result: PASS with verified BusyLoop and KeccakUnion receipts, high WebGPU
limits, zero fallback/CPU-only, and representative KeccakUnion shape.

Key lines:

```text
BusyLoop wall_ms=6094 gpu_active_ms=3833 cpu_fallbacks=0 cpu_only_ops=0
representative-keccak-union proof_count=1 segments=1 pending_keccaks=9 assumptions=1
KeccakUnion wall_ms=91400 gpu_active_ms=60209 cpu_fallbacks=0 cpu_only_ops=0
source=iter6d_g_arm_txns uploads=4 upload_bytes=19959620
test result: ok. 1 passed; 0 failed; 0 ignored; 151 filtered out; finished in 98.20s
```

Comparison:

```text
SP7dy accepted BusyLoop+KeccakUnion: 98.15s
SP7ee candidate:                    98.20s
Movement:                           +0.05s / noise-level slower
```

## Post-Revert Verification

Rejected code and the temporary xgboost upload assertion were removed.

Marker sweep:

```text
rg -n "arm_txns compacted|build_selected_preflight_txn_buffers|iter6d_g_arm_txns.*120_000_000|selected_txns|selected_cycles" risc0/circuit/rv32im/src/prove/hal/webgpu.rs examples/browser-prove/src/lib.rs
```

Result: clean.

Diff check:

```text
git diff --check -- examples/browser-prove/src/lib.rs risc0/circuit/rv32im/src/prove/hal/webgpu.rs
```

Result: PASS.

Post-revert compile:

```text
env CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies --no-run > /tmp/sp7ee-post-revert-xgboost-compile.log 2>&1
```

Result: PASS in `4m29s` with only pre-existing dead-code warnings.

## Decision

Reject and revert.

The candidate was correctness-clean and reduced GPU-witgen transaction upload
bytes by about 68 MB on xgboost, but it did not reduce representative proof wall
time. Under the current priority, a cleaner data-movement result is not enough
unless it preserves or improves wall time.

Accepted wall-time gain: `0`.

Do not retry transaction-payload compaction as an immediate lever unless it is
paired with a larger GPU-witgen change that removes dispatch/build overhead or
shows a representative wall-time win.
