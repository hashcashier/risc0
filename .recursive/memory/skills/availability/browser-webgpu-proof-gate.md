Type: `availability`
Status: `CURRENT`
Scope: `Browser WebGPU proof validation availability for wasm-bindgen browser tests in this repository.`
Owns-Paths:
Watch-Paths:
- `/examples/browser-prove/`
- `/examples/browser-prove/src/lib.rs`
- `/examples/browser-prove/webdriver.json`
- `/risc0/zkp/src/hal/webgpu.rs`
Source-Runs:
- `wasm-webgpu-prover-perf`
Validated-At-Commit: `4b84f5e9159c7b042a173c98b07d74e47e048eaf`
Last-Validated: `2026-05-22T00:00:00Z`
Tags:
- `skills`
- `availability`
- `browser`
- `webgpu`
- `proof-validation`
- `sandbox`

# Browser WebGPU Proof Gate Availability

Browser WebGPU proof validation is a capability-sensitive gate. It cannot be replaced by native tests, static shader validation, Naga validation, or non-browser wasm compilation.

## Required Runner Shape

Focused and e2e browser proof tests need a wasm bindgen browser runner, Chrome/Chromedriver, and GPU environment variables similar to:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver
WASM_BINDGEN_TEST_TIMEOUT=120
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json
__VK_LAYER_NV_optimus=NVIDIA_only
__NV_PRIME_RENDER_OFFLOAD=1
__GLX_VENDOR_LIBRARY_NAME=nvidia
```

Plain `cargo test --target wasm32-unknown-unknown` is not enough. It can build the wasm test and then fail with `Exec format error` because it tries to execute the `.wasm` file directly.

## Sandbox Boundary

Inside the default sandbox, `wasm-bindgen-test-runner` may reach startup but fail to launch the browser/server:

```text
Error: failed to spawn server
Caused by:
    Operation not permitted (os error 1)
```

This is not a proof result and must not be treated as a candidate failure or success.

## Escalation Boundary

Browser execution requires approved escalation. If escalation is rejected by the approval system, especially due usage limits, do not attempt indirect browser execution, alternative launch paths, or policy workarounds.

Allowed while blocked:

- static analysis;
- Naga shader validation;
- arithmetic simulation;
- run evidence maintenance;
- local non-browser hygiene checks.

Blocked while unavailable:

- retaining runtime performance code;
- accepting wall-time gains;
- claiming browser correctness;
- substituting native or static tests for browser proof generation.

## Acceptance Rule

Performance changes to the browser WebGPU prover require browser proof evidence for the relevant gates. For the current `wasm-webgpu-prover-perf` run, that means focused HAL parity plus representative BusyLoop + KeccakUnion, xgboost, and drain-attribution proof gates before accepting runtime changes.
