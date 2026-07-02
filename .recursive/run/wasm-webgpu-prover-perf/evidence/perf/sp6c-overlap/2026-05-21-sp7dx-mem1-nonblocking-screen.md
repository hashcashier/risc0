# SP7dx - MEM1 Nonblocking GPU-Witgen Screen

Date: 2026-05-21

## Candidate

Re-screen the SP7dw MEM1 GPU-witgen replacement path with the accepted
nonblocking prewarm behavior enabled, so the first cold segment skips pending
MEM1 replacement instead of waiting for compile/predispatch and later segments
can use the hot arm-6 replacement.

This is distinct from SP7dw's forced-cold production candidate.

Opt-in settings:

- `set_witgen_gpu_probe_enabled(true)`
- `set_witgen_gpu_replace_enabled(true)`
- `set_witgen_gpu_replace_nonblocking_pending_enabled(true)`
- `set_accum_gpu_mem1_direct_enabled(true)`
- `set_witgen_gpu_mem1_replace_minor_mask(0x0007)`
- `set_witgen_gpu_mem1_replace_candidate_enabled(true)`

## Evidence

### xgboost

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --target wasm32-unknown-unknown --release iter6d_g_mem1_witgen_replace_nonblocking_candidate_xgboost_e2e_verify -- --nocapture
```

Result: PASS, high WebGPU limits, journal `30.528042544062632`, zero
`cpu_fallbacks`, zero `cpu_only_ops`, total test time `72.67s`.

Key lines:

```text
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
segment 0: nonblocking_pending_skip, mask=0x0000, pre_witgen_dispatch_async=3ms, rv32im_witgen=507ms
segment 1: iter6d_g_minor_dispatch arm=6 minor=2 cycles=32153, mask=0x0061, pre_witgen_dispatch_async=16ms, rv32im_witgen=217ms
prove_session_async wall_ms=72093.0 gpu_active_ms=45824.0 gpu_idle_ratio=0.364
gpu_dispatches=5259 raw_compute_dispatches=9749 queue_submits=2771 cpu_fallbacks=0 cpu_only_ops=0 uploads=3312 upload_bytes=2613694224 readbacks=352 readback_bytes=7488592
```

Comparison:

```text
SP7dt accepted xgboost:                       73.39s
SP7dx MEM1 nonblocking xgboost screen:        72.67s
Movement vs SP7dt:                            -0.72s / -1.0%
Same-tree fresh default xgboost revalidation: 72.77s
Movement vs same-tree fresh default:          -0.10s / noise-level
```

### BusyLoop + KeccakUnion

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --target wasm32-unknown-unknown --release iter6d_g_mem1_witgen_replace_nonblocking_candidate_e2e_verify -- --nocapture
```

One invalid low-limit Chrome attempt was rejected by the representative-limit
guard and excluded:

```text
max_buffer_size=1073741824 max_storage_buffer_binding_size=1073741824 max_compute_workgroup_storage_size=32768
```

The accepted rerun negotiated the required limits:

```text
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
```

Result: PASS, verified BusyLoop and KeccakUnion receipts, zero
`cpu_fallbacks`, zero `cpu_only_ops`, total test time `98.99s`.

Key BusyLoop lines:

```text
nonblocking_pending_skip
mask=0x0000 no_ready_replacement no_sync
pre_witgen_dispatch_async elapsed_ms=2
rv32im_witgen elapsed_ms=520
rv32im_accumulate MEM1 direct_gpu dispatched rows=31917
prove_session_async wall_ms=6768.0 gpu_active_ms=4472.0 gpu_idle_ratio=0.339
```

Key KeccakUnion lines:

```text
representative-keccak-union proof_count=1 segments=1 pending_keccaks=9 assumptions=1
segments=4 pending_keccaks=9 assumptions=1
iter6d_g_minor_dispatch arm=6 minor=2 cycles=11173
mask=0x0061 dispatched_arms=[0, 5, 6]
pre_witgen_dispatch_async elapsed_ms=13
rv32im_witgen elapsed_ms=983
rv32im_accumulate MEM1 direct_gpu dispatched rows=11256
prove_session_async wall_ms=91472.0 gpu_active_ms=60399.0 gpu_idle_ratio=0.340
```

Aggregate diagnostics for KeccakUnion:

```text
gpu_dispatches=5900 raw_compute_dispatches=10692 queue_submits=3099 cpu_mirrors=175 cpu_fallbacks=0 cpu_only_ops=0 uploads=3663 upload_bytes=2923955584 readbacks=409 readback_bytes=10183944
```

Large upload buckets remain:

```text
webgpu_zeroize_sparse_values uploads=63 upload_bytes=1964226448
webgpu_zeroize_sparse_ranges uploads=63 upload_bytes=664360240
webgpu_zero_default_sparse_values uploads=4 upload_bytes=60936792
webgpu_zero_default_sparse_ranges uploads=4 upload_bytes=18498256
```

Comparison:

```text
SP7dt accepted BusyLoop+KeccakUnion:        99.31s
SP7dx MEM1 nonblocking BusyLoop+KeccakUnion: 98.99s
Movement:                                  -0.32s / -0.3%
```

## Decision

Do not promote MEM1 witness replacement by default from this screen alone.

The nonblocking form fixes the SP7dw cold-predispatch regression and is
correctness-clean across xgboost, BusyLoop, and KeccakUnion. However, the wall
movement is small and partly noise-level, especially against a same-tree xgboost
default rerun.

Accepted production wall-time gain: `0`.

Keep the opt-in nonblocking MEM1 screen as evidence that the avenue is viable,
but prioritize the larger measured bucket next: sparse zeroize/default-zero
uploads, which still move about `2.6GB` on KeccakUnion and `>2GB` on xgboost
candidate logs.
