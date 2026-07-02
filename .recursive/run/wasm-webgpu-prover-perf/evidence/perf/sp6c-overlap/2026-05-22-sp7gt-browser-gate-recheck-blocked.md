# SP7gt Browser Gate Recheck Blocked

Date: 2026-05-22

## Purpose

Re-check whether the browser WebGPU proof gate is available after static preparation for the tiled NTT candidate completed.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Attempt 1: Missing Browser Runner

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_ntt_gpu_results_match_cpu -- --nocapture
```

Result:

```text
Finished `release` profile [optimized + debuginfo] target(s) in 4m 55s
Running unittests src/lib.rs (.../browser_prove-dd95d199bcf14693.wasm)
error: test failed, to rerun pass `--lib`

Caused by:
  could not execute process `...browser_prove-dd95d199bcf14693.wasm webgpu_hal_ntt_gpu_results_match_cpu --nocapture` (never executed)

Caused by:
  Exec format error (os error 8)
```

Interpretation: this only proved the command needed the wasm bindgen browser runner. It did not exercise browser WebGPU.

## Attempt 2: Browser Runner Inside Sandbox

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=120 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_ntt_gpu_results_match_cpu -- --nocapture
```

Result:

```text
Finished `release` profile [optimized + debuginfo] target(s) in 2m 18s
Running unittests src/lib.rs (.../browser_prove-dd95d199bcf14693.wasm)
Set timeout to 120 seconds...
Executing bindgen...
Error: failed to spawn server

Caused by:
    Operation not permitted (os error 1)
```

Interpretation: the focused browser gate reached the browser runner but cannot launch the required server/browser process inside the sandbox.

## Attempt 3: Required Escalated Browser Runner

The same browser-runner command was retried with required escalation.

Result:

```text
Rejected("This action was rejected due to unacceptable risk.
Reason: Automatic approval review failed: You've hit your usage limit. Visit https://chatgpt.com/codex/settings/usage to purchase more credits or try again at May 26th, 2026 9:53 PM.
The agent must not attempt to achieve the same outcome via workaround, indirect execution, or policy circumvention. Proceed only with a materially safer alternative, or if the user explicitly approves the action after being informed of the risk. Otherwise, stop and request user input.")
```

## Decision

The browser WebGPU validation gate remains blocked.

Allowed:

- static analysis;
- non-runtime evidence maintenance;
- local non-browser hygiene checks.

Blocked:

- implementing and retaining tiled NTT runtime code;
- accepting any new wall-time performance claim;
- attempting indirect browser execution workarounds.

Next runtime action remains SP7go only after browser proof validation is available.
