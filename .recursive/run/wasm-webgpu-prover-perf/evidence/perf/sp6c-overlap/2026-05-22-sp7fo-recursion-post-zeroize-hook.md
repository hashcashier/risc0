# SP7fo recursion post-zeroize witgen hook

## Purpose

Create and prove the production insertion point for the next GPU-resident recursion WOM / `verify_mem` candidate.

Prior rejected candidates regressed because they uploaded dense recursion data or sorted WOM rows. The safe next path must run after witness data/global zeroize, when `recursion_data` can be made GPU-current without recreating the SP7fd/SP7ff upload shape.

## Change

- Added `CircuitWitnessGenerator::post_witness_zeroize(...)` with a default no-op implementation.
- `WitnessGenerator::new` now calls that hook immediately after `data` and `global` zeroize.
- Added disabled-by-default WebGPU probe controls:
  - `set_recursion_witgen_post_zeroize_hook_probe_enabled(enabled)`
  - `recursion_witgen_post_zeroize_hook_calls()`
- The WebGPU recursion HAL increments the counter and emits `browser-prove:metric recursion_witgen_post_zeroize_hook ...` when the probe is enabled.
- The representative BusyLoop + KeccakUnion gate and xgboost gate now enable the probe and assert the hook runs during real proof generation.

Accepted wall-time gain: `0`. This is production wiring substrate, not the GPU `verify_mem` replacement itself.

## RED

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture
```

Expected failure: the representative browser e2e test imported missing post-zeroize hook probe APIs.

Observed failure:

```text
no `recursion_witgen_post_zeroize_hook_calls` in `prove`
no `set_recursion_witgen_post_zeroize_hook_probe_enabled` in `prove`
```

## GREEN / compile

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release --no-run
```

Result: passed.

```text
Finished `release` profile [optimized + debuginfo] target(s) in 4m 35s
```

## Representative e2e proof generation

Command:

```bash
script -q -e -c "env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=300 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture" .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7fo-default-representative-post-zeroize-hook.chrome.txt
```

Result:

- High WebGPU limits: `4294967292 / 2147483644 / 49152`.
- BusyLoop receipt verified: `wall_ms=5678`, `gpu_active_ms=4135`, `gpu_idle_ratio=0.272`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- KeccakUnion receipt verified: `wall_ms=85229`, `gpu_active_ms=63122`, `gpu_idle_ratio=0.259`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- Post-zeroize hook metrics: `26`.
- WOM profile remained unchanged in shape: `calls=26`, `rows=38678651`, `distinct_value_groups=0`, `distinct_value_rows=0`, `max_addr_group=317993`.
- Test passed: `1 passed; 0 failed; 164 filtered out; finished in 91.64s`.

Command:

```bash
script -q -e -c "env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=300 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture" .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7fo-xgboost-post-zeroize-hook.chrome.txt
```

Result:

- High WebGPU limits: `4294967292 / 2147483644 / 49152`.
- xgboost receipt verified: `wall_ms=64890`, `gpu_active_ms=48366`, `gpu_idle_ratio=0.255`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- Post-zeroize hook metrics: `21`.
- WOM profile remained unchanged in shape: `calls=21`, `rows=28520256`, `distinct_value_groups=0`, `distinct_value_rows=0`, `max_addr_group=317959`.
- Test passed: `1 passed; 0 failed; 164 filtered out; finished in 65.47s`.

## Decision

The post-zeroize recursion witgen hook is now e2e-proven on BusyLoop, KeccakUnion, and xgboost without changing default proof semantics or fallback behavior.

Next implementation step: use this hook for a guarded CPU-exec-only + GPU-resident row generation/scatter/backfill/`verify_mem` candidate. The candidate must keep WOM rows on GPU and must not reintroduce dense `recursion_data`, sorted-row, or offset upload behavior from SP7fd/SP7ff.
