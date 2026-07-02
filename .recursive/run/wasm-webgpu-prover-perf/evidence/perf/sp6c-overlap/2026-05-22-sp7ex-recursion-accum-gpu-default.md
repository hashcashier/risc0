# SP7ex - Recursion Accumulator GPU Default

Date: 2026-05-22

## Objective

Move recursion accumulation from CPU Rust kernels to generated WGSL on the
browser WebGPU path, while preserving the native accumulator dataflow:
compute-accum writes a separate WOM buffer, WebGPU prefix-products runs over
WOM, and verify-accum writes the final accumulator columns.

## Changed Files

Generated WGSL artifacts:

```text
risc0/circuit/recursion/src/prove/hal/webgpu_witgen_prelude.wgsl
risc0/circuit/recursion/src/prove/hal/webgpu_layout.wgsl.inc
risc0/circuit/recursion/src/prove/hal/webgpu_step_compute_accum.wgsl
risc0/circuit/recursion/src/prove/hal/webgpu_step_verify_accum.wgsl
```

Runtime and test wiring:

```text
risc0/circuit/recursion/src/prove/hal/mod.rs
risc0/circuit/recursion/src/prove/hal/cpu.rs
risc0/circuit/recursion/src/prove/hal/cuda.rs
risc0/circuit/recursion/src/prove/hal/webgpu.rs
risc0/circuit/recursion/src/prove/mod.rs
risc0/circuit/recursion/src/prove/witgen.rs
examples/browser-prove/src/lib.rs
```

## Implementation

`WebGpuCircuitHal::accumulate` now defaults to the generated WebGPU recursion
accumulator path when all storage buffers fit the device storage-binding
limit. Devices that cannot bind the required buffers keep the existing CPU
accumulator path instead of failing.

The GPU path:

- syncs `ctrl`, `global`, `data`, `mix`, and `accum` to GPU
- allocates a row-major `BabyBearExtElem` WOM buffer initialized to one
- dispatches `recursion_step_compute_accum_main`
- runs `hal.prefix_products(&wom)` under GPU-authoritative scope
- dispatches `recursion_step_verify_accum_main`
- marks final `accum` GPU-dirty
- zeroizes `accum` and `global` in GPU-authoritative mode

The default representative and xgboost tests now assert that the recursion
accumulator GPU dispatch counter increases.

## Browser Smoke

The SP7et Chrome smoke proved both generated WGSL modules compile and dispatch
on the representative high-limit WebGPU device. After the WOM layout correction
the assembled modules validated with `naga`:

```text
compute module: 318080 bytes
verify module: 182806 bytes
total: 500886 bytes
```

The browser smoke rerun passed with:

```text
compute_bytes=318089
verify_bytes=182815
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152
```

## Candidate E2E Gates

Raw logs:

```text
.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7eu-recursion-accum-gpu-candidate-e2e.chrome.txt
.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7ev-xgboost-recursion-accum-gpu-candidate.chrome.txt
```

BusyLoop + KeccakUnion candidate:

```text
BusyLoop wall_ms=5678 gpu_active_ms=4099
KeccakUnion wall_ms=85027 gpu_active_ms=62686
test result: ok, finished in 91.42s
cpu_fallbacks=0 cpu_only_ops=0
```

xgboost candidate:

```text
browser-prove:metric prove_session_async wall_ms=64290.0 gpu_active_ms=48164.0 gpu_idle_ratio=0.251
browser-prove:done xgboost_recursion_accum_gpu_candidate: segments=11 user_cycles=2294946 total_cycles=2883584
browser-prove:webgpu xgboost_recursion_accum_gpu_candidate: raw_compute_dispatches=9771 queue_submits=2782 cpu_fallbacks=0 cpu_only_ops=0 upload_bytes=2523497416 readback_bytes=7488592
test result: ok. 1 passed; 0 failed; 0 ignored; 157 filtered out; finished in 64.84s
```

## Default E2E Gates

Raw logs:

```text
.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7ew-xgboost-recursion-accum-default.chrome.txt
.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7ex-default-representative-recursion-accum-gpu.chrome.txt
```

### xgboost default

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
__VK_LAYER_NV_optimus=NVIDIA_only \
__NV_PRIME_RENDER_OFFLOAD=1 \
__GLX_VENDOR_LIBRARY_NAME=nvidia \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_succinct_receipt_verifies -- --nocapture
```

Result:

```text
browser-prove:metric prove_session_async wall_ms=64309.0 gpu_active_ms=47958.0 gpu_idle_ratio=0.254
browser-prove:done xgboost: segments=11 user_cycles=2294867 total_cycles=2883584
browser-prove:webgpu xgboost: raw_compute_dispatches=9771 queue_submits=2782 cpu_fallbacks=0 cpu_only_ops=0 upload_bytes=2523713172 readback_bytes=7488592
test result: ok. 1 passed; 0 failed; 0 ignored; 157 filtered out; finished in 64.84s
```

The xgboost journal assertion remained active and passed:
`30.528042544062632`.

### BusyLoop + KeccakUnion default

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
__VK_LAYER_NV_optimus=NVIDIA_only \
__NV_PRIME_RENDER_OFFLOAD=1 \
__GLX_VENDOR_LIBRARY_NAME=nvidia \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_default_representative_e2e_verify -- --nocapture
```

Result:

```text
BusyLoop wall_ms=5683 gpu_active_ms=4101 segments=1 total_cycles=262144
BusyLoop raw_compute_dispatches=606 queue_submits=167 cpu_fallbacks=0 cpu_only_ops=0
KeccakUnion wall_ms=85436 gpu_active_ms=63236 segments=4 total_cycles=917504
KeccakUnion raw_compute_dispatches=10725 queue_submits=3128 cpu_fallbacks=0 cpu_only_ops=0 upload_bytes=2657009332 readback_bytes=10183944
test result: ok. 1 passed; 0 failed; 0 ignored; 157 filtered out; finished in 91.83s
```

Both receipts were verified by `prove_succinct_info_async`.

## Performance Decision

Accept and enable by default on capable WebGPU devices.

Compared with the accepted SP7em default test runtimes:

```text
BusyLoop + KeccakUnion: 95.73s -> 91.83s  (-3.90s, -4.1%)
xgboost:                68.02s -> 64.84s  (-3.18s, -4.7%)
```

Using same-metric fresh proof-wall lines, xgboost moved from the SP7en profile
`68299 ms` to `64309 ms` (`-3990 ms`, `-5.8%`).

Compared with the SP7eq same-tree KeccakUnion profile, KeccakUnion proof wall
moved from `90796 ms` to `85436 ms` (`-5360 ms`, `-5.9%`).

Accepted current working state:

```text
xgboost default proof wall: 64.309s
xgboost default test runtime: 64.84s
BusyLoop + KeccakUnion default test runtime: 91.83s
```

## Follow-Up

The accepted gain confirms recursion accumulation was the right immediate
witness-generation target. The remaining large measured buckets are now
`fri_prove`, `check_group`, and recursion witness generation itself. Further
work should avoid one-off RV32IM side paths unless they remove multiple seconds
of full proof wall time under these same e2e gates.
