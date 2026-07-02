# SP7fg - Recursion exec Poseidon2-chain capacity gate

Date: 2026-05-22

## Purpose

SP7fe selected the chunk-complete Poseidon2 exec chain as the smallest recursion witness slice worth wiring next:

`poseidon2_load -> poseidon2_full -> poseidon2_partial -> poseidon2_store`

This gate confirms that the combined chain is browser-capacity safe before runtime wiring or correctness claims.

## Artifact

Added generated/pruned WGSL:

- `risc0/circuit/recursion/src/prove/hal/webgpu_step_exec_poseidon2_chain.wgsl`

The source is the SP7fa generated recursion `step_exec.wgsl`, pruned to:

- `code[3]` / `x2594`: `poseidon2_load`
- `code[4]`: `poseidon2_full`
- `code[5]`: `poseidon2_partial`
- `code[6]` / `x3780`: `poseidon2_store`
- required cleanup and Plonk write blocks for `x2594` and `x3780`

Sizes:

- pruned exec chunk: `117255` bytes
- assembled module with prelude, layout, and test extern stubs: `169012` bytes in the Rust/browser test

## Static validation

Temporary assembled module:

- `/tmp/sp7fg-step-exec-poseidon2-chain-assembled.wgsl`

Command:

```bash
naga /tmp/sp7fg-step-exec-poseidon2-chain-assembled.wgsl
```

Result:

- `Validation successful`

## Browser capacity validation

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
__VK_LAYER_NV_optimus=NVIDIA_only \
__NV_PRIME_RENDER_OFFLOAD=1 \
__GLX_VENDOR_LIBRARY_NAME=nvidia \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  recursion_exec_poseidon2_chain_wgsl_compiles_on_chrome \
  -- --nocapture
```

Log:

- `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7fg-recursion-exec-poseidon2-chain-capacity-probe.chrome.txt`

Result:

- high WebGPU limits negotiated
- `recursion_exec_poseidon2_chain_wgsl module_bytes=169012`
- `recursion_accum_probe recursion_step_exec_poseidon2_chain_main module_bytes=169012 phase=OK`
- test passed in `5.78s`

## Decision

Accept as a capacity/probe artifact only. This is not a performance win and does not prove witness correctness.

The next runtime step is to wire real GPU-resident externs for this chunk:

- `extern_womRead`
- `extern_womWrite`
- `extern_plonkWrite`

Then run focused CPU/GPU parity before any representative proof gate.
