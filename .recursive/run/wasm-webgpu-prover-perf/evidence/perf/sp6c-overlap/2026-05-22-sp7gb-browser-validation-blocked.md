# SP7gb Browser Validation Blocked

Date: 2026-05-22

## Purpose

Before making another runtime performance change, check whether the browser WebGPU validation gate is available in this session.

The user has made e2e proof-generation validation mandatory for performance changes, with representative BusyLoop + KeccakUnion and xgboost coverage before accepting any runtime win.

## Attempted Gate

Focused browser WebGPU validation command from `examples/browser-prove`:

```text
script -q -e -c "env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=120 cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release recursion_wom_generated_row_coverage_is_stable -- --nocapture" ../../.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7ga-validation-availability.chrome.txt
```

## Result

The approval system rejected the escalated browser command before execution because this session has hit its usage limit. No Chrome/browser log was produced.

This is the same validation blocker that prevented accepting SP7fz.

## Decision

Do not make or retain runtime performance changes while this gate is unavailable.

Allowed follow-up while blocked:

- static analysis;
- evidence documentation;
- planning against existing accepted/rejected proof logs;
- local non-GPU hygiene checks.

Blocked follow-up while unavailable:

- accepting a performance change;
- leaving speculative runtime code in the worktree;
- attempting indirect/workaround execution of the rejected browser command.

Current accepted working state remains SP7fr.

Accepted wall-time gain: 0.
