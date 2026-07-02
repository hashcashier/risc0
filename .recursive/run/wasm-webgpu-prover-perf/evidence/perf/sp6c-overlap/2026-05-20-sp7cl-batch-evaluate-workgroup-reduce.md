# SP7cl batch_evaluate_any workgroup reduction

Date: 2026-05-20

## Purpose

SP7cj corrected the hot-path attribution: much of the remaining xgboost time was queued GPU work draining at synchronization points. After SP7ck rejected a grouped NTT-step loop, SP7cl targeted the `finalize_async eval_u_readback` drain bucket. The direct WebGPU `batch_evaluate_any` shader used one invocation per evaluation and scanned the entire polynomial serially in that invocation, unlike CUDA's one-block-per-evaluation reduction.

## Change

File changed:

- `risc0/zkp/src/hal/webgpu.rs`

Implementation:

- Reworked `BATCH_EVALUATE_ANY_WGSL` to use one 256-lane workgroup per evaluation.
- Each lane evaluates every 256th coefficient using `step_x = x^256`.
- Partial extension-field sums reduce through `var<workgroup> partials: array<vec4<u32>, 256>`.
- The host dispatch for the direct path now launches `eval_count` workgroups instead of `ceil(eval_count / 256)`.
- Chunked/small-eval fallback paths are unchanged.

This preserves the same output values and transcript data; it only changes the parallelization of direct `batch_evaluate_any`.

## Commands

Compile:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in `4m50s`.

BusyLoop + KeccakUnion browser proof e2e:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture > /tmp/sp7cl-busy-keccak-batch-eval-reduce.log 2>&1
```

xgboost browser proof e2e:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture > /tmp/sp7cl-xgboost-batch-eval-reduce.log 2>&1
```

## Representative Results

All proof e2e runs verified receipts and reported `cpu_fallbacks=0`, `cpu_only_ops=0`.

Against SP7cg accepted baseline:

```text
BusyLoop    8072  -> 7485  ms  -587 ms   -7.3%
KeccakUnion 100588 -> 92641 ms  -7947 ms  -7.9%
xgboost     89418 -> 78509 ms  -10909 ms -12.2%
```

xgboost details:

```text
wall_ms=78509
gpu_active_ms=45925
gpu_idle_ratio=0.415
raw_compute_dispatches=11466
queue_submits=2718
upload_bytes=3104639804
readback_bytes=7488592
```

Target bucket movement on xgboost:

```text
finalize_async eval_u_readback 10712 -> 343 ms across 32 calls
finalize_async check_group      9728 -> 9784 ms across 32 calls
finalize_async fri_prove       29306 -> 29146 ms across 32 calls
```

The large wall-time gain comes from making the direct `batch_evaluate_any` work finish before the `out` readback drain. The readback bytes and dispatch/submit counts are effectively unchanged; the work inside each direct evaluate dispatch became much more parallel.

## Decision

Accepted. This is a correctness-clean, representative e2e-proven wall-time reduction across BusyLoop, KeccakUnion, and xgboost, with the largest xgboost improvement since the direct accumulator/eqz work.

Current accepted xgboost working-state estimate: about `78.5 s` on this browser/NVIDIA setup.

