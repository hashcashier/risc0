# SP7et - recursion accumulator WGSL browser smoke

Date: 2026-05-22

## Purpose

Consume the SP7es-generated recursion accumulator WGSL artifacts from the RISC0 worktree and prove that Chrome's actual WebGPU/Tint path can compile and dispatch them before any proof-path wiring.

This is a runtime viability gate only. It does not replace recursion accumulation in proving yet.

## RISC0 changes

Generated WGSL artifacts were added under:

```text
risc0/circuit/recursion/src/prove/hal/webgpu_witgen_prelude.wgsl
risc0/circuit/recursion/src/prove/hal/webgpu_layout.wgsl.inc
risc0/circuit/recursion/src/prove/hal/webgpu_step_compute_accum.wgsl
risc0/circuit/recursion/src/prove/hal/webgpu_step_verify_accum.wgsl
```

Rust/browser harness changes:

```text
risc0/circuit/recursion/src/prove/hal/webgpu.rs
risc0/circuit/recursion/src/prove/mod.rs
examples/browser-prove/src/lib.rs
```

The WebGPU prelude now binds the accumulator stages with separate buffers:

```text
binding 0: ctrl
binding 1: global/out
binding 2: data
binding 3: mix
binding 4: wom/prefix-products buffer
binding 5: final accum
binding 6: params
```

The Plonk accumulator externs follow the native Metal dataflow:

- `extern_plonkWriteAccum` writes WOM.
- `extern_plonkReadAccum` reads WOM.
- `step_verify_accum` writes final accumulator columns through its `accum4` argument.

## Static validation

The RISC0-assembled browser modules were validated with `naga`:

```text
naga /tmp/sp7et-recursion-compute.wgsl /tmp/sp7et-recursion-compute.spv
naga /tmp/sp7et-recursion-verify.wgsl /tmp/sp7et-recursion-verify.spv
```

Result: both passed.

Assembled module sizes:

```text
317771 /tmp/sp7et-recursion-compute.wgsl
182497 /tmp/sp7et-recursion-verify.wgsl
500268 total
```

## Browser smoke

Command:

```text
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
  recursion_accum_wgsl_compiles_on_chrome -- --nocapture
```

Result: passed.

Key output:

```text
browser-prove:metric recursion_accum_wgsl compute_bytes=317780 verify_bytes=182506
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
browser-prove:metric recursion_accum_probe recursion_step_compute_accum_main module_bytes=317780 phase=OK
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
browser-prove:metric recursion_accum_probe recursion_step_verify_accum_main module_bytes=182506 phase=OK
test tests::recursion_accum_wgsl_compiles_on_chrome ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 155 filtered out; finished in 30.18s
```

## Interpretation

This advances recursion accumulator offload from generated-parser-valid WGSL to browser-compile-and-dispatch-valid WGSL in the RISC0 worktree.

No proof uses this path yet, no CPU/GPU parity has run, and no proving wall-time reduction is accepted. The next required step is an opt-in runtime path that allocates a separate WOM buffer, dispatches compute-accum, runs WebGPU prefix-products over WOM, dispatches verify-accum into final accum, and then compares against the CPU path before any e2e proof timing claim.

Accepted wall-time gain: 0.
