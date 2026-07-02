# SP7ed - Current Baseline + Exp2 Local NTT Stage-Skip Rejected

Date: 2026-05-21

## Current Post-Revert Baseline

The post-SP7ec reverted tree was revalidated before testing the next
candidate. The representative default path is still correctness-clean with
high Chrome WebGPU limits and zero CPU fallback/CPU-only operations.

BusyLoop + KeccakUnion command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture
```

Result: PASS, verified BusyLoop and KeccakUnion receipts, high WebGPU limits,
`cpu_fallbacks=0`, `cpu_only_ops=0`, finished in `98.12s`.

xgboost command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture > /tmp/sp7ed-current-xgboost-baseline.log 2>&1
```

Key lines:

```text
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
prove_session_async wall_ms=72220.0 gpu_active_ms=45211.0 gpu_idle_ratio=0.374
test result: ok. 1 passed; 0 failed; 0 ignored; 154 filtered out; finished in 72.76s
segments=11 user_cycles=2294890 total_cycles=2883584
cpu_fallbacks=0 cpu_only_ops=0
raw_compute_dispatches=9718 queue_submits=2740
upload_count=3292 upload_bytes=2766499944
readback_count=352 readback_bytes=7488592
batch_expand_into_evaluate_ntt gpu_dispatches=224
```

Current xgboost stage aggregate from `/tmp/sp7ed-current-xgboost-baseline.log`:

```text
rv32im_witgen_ms 3632
rv32im_step_top_accum_ms 6790
finalize_check_group_ms 9619 count 32
finalize_fri_prove_ms 29199 count 32
recursion_witgen_ms 5229 count 21
recursion_accumulate_ms 6131 count 21
```

Sparse upload aggregate:

```text
sparse_bytes 2400108060
dense_bytes 6968836096
values 420970803
ranges 89527978
ratio 0.344406
```

## Candidate

Try an `expand_bits=2` specialization in `BATCH_EXPAND_LOCAL_NTT_WGSL` that
pre-applies the first two local NTT stages and starts from the stage-3
boundary, avoiding the existing repeated-input scratch fill.

## RED Focused Gate

Temporary RED test:

```text
webgpu_hal_batch_expand_ntt_uses_exp2_stage_skip_path
```

The test used a proof-shaped HAL case:

```text
count=4, in_size=1024, expand_bits=2, out_size=4096
```

RED command:

```text
env CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_batch_expand_ntt_uses_exp2_stage_skip_path -- --nocapture > /tmp/sp7ed-exp2-stage-skip-red.log 2>&1
```

Result: expected RED. CPU/GPU parity passed, but the new marker
`webgpu_batch_expand_local_ntt_exp2_stage_skip_params` was absent and the
generic `webgpu_batch_expand_local_ntt_params` marker was present. Fallback
counters stayed zero.

## GREEN Attempt

Implementation attempted:

- For `params.expand_bits == 2u`, fill scratch with `4 * input[col >> 2]` at
  `i % 4 == 0` and zero elsewhere.
- Use a distinct upload marker
  `webgpu_batch_expand_local_ntt_exp2_stage_skip_params`.
- Keep the NTT loop start at stage 3.

GREEN command:

```text
env CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_batch_expand_ntt_uses_exp2_stage_skip_path -- --nocapture > /tmp/sp7ed-exp2-stage-skip-green.log 2>&1
```

Result: FAIL with CPU/GPU output mismatch before any representative proof run.

## Finding

The candidate premise was wrong. The existing WebGPU kernel already starts the
local NTT at:

```text
stage = params.expand_bits + 1
```

For `expand_bits=2`, stages 1 and 2 are already skipped. The repeated scratch
fill is the correct representation at that stage boundary. Pre-applying the
first two stages in the scratch buffer double-applies the transform and breaks
CPU/GPU parity.

## Post-Revert Verification

The temporary focused test and runtime marker/code path were removed.

Marker sweep:

```text
rg -n "exp2_stage_skip|stage_skip local|webgpu_hal_batch_expand_ntt_uses_exp2_stage_skip_path" examples/browser-prove/src/lib.rs risc0/zkp/src/hal/webgpu.rs
```

Result: clean.

Diff check:

```text
git diff --check -- examples/browser-prove/src/lib.rs risc0/zkp/src/hal/webgpu.rs
```

Result: PASS.

## Decision

Reject before representative e2e. Accepted wall-time gain: `0`.

Do not retry "pre-apply the first `expand_bits` local NTT stages" in
`BATCH_EXPAND_LOCAL_NTT_WGSL`. Future `expand_bits=2` local-NTT work must
preserve the repeated scratch representation or prove a mathematically
equivalent transform with focused CPU/GPU parity before e2e proof timing.
