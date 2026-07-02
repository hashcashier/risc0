# SP7ba: MISC2 sparse shadow prefix tightening

Date: 2026-05-19

## Scope

Continue GPU-witgen offload work by reducing the CPU-shadow repair cost introduced when MISC2 replacement was made authoritative in SP7az.

This does not broaden the set of GPU-owned witness rows. It only tightens the repair path for rows that already pass representative receipt verification.

## RED

The browser proof gate was tightened before the implementation:

- BusyLoop po2=18 `assert_witgen_shadow_readback_sparse` max: `80_000_000 -> 55_000_000`
- KeccakUnion(1) max: `120_000_000 -> 105_000_000`
- xgboost max: `800_000_000 -> 680_000_000`

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Expected failure:

- BusyLoop receipt generated and verified.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.
- `witgen_data_shadow_rows readbacks=3 readback_bytes=58_359_440`.
- Assertion failed because `58_359_440 > 55_000_000`.

## Implementation

Changed `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`:

- `witgen_replace_shadow_rows` now splits MISC2 into compare rows and branch-like rows.
- MISC2 minor `1` is excluded from sparse repair, matching the CPU short-circuit predicate.
- MISC2 compare rows, minors `0 | 2`, sync only prefix `139`.
- MISC2 branch-like rows, minors `3 | 4 | 5 | 6 | 7`, sync only prefix `132`.
- MISC0 arithmetic/bitwise prefixes remain unchanged at `132` and `196`.

Layout basis:

- Branch-like MISC2 rows need `MiscInput` plus source-register fields through column `131`.
- Compare rows need `NormalizeU32` carries and signed compare aux cells through column `138`.

## GREEN

Compile gate:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result:

- PASS, release wasm test built in `4m24s`.
- Only existing dead-code warnings in WebGPU experimental helpers.

Representative proof gates:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Filtered metrics capture result:

| Workload | wall_ms | segments | cpu_fallbacks | cpu_only_ops | source=data upload_bytes | witgen_data_shadow_rows |
|---|---:|---:|---:|---:|---:|---:|
| BusyLoop po2=18 | 17,405 | 1 | 0 | 0 | 355,467,264 | 49,265,896 |
| KeccakUnion(1) | 101,856 | 4 | 0 | 0 | 4,776,263,680 | 94,777,916 |

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result:

| Workload | wall_ms | segments | cpu_fallbacks | cpu_only_ops | source=data upload_bytes | witgen_data_shadow_rows |
|---|---:|---:|---:|---:|---:|---:|
| xgboost | 107,825 | 11 | 0 | 0 | 5,252,317,184 | 582,206,500 |

xgboost journal remained correct: `30.528042544062632`.

## Reduction

Compared with the pre-fix MISC2 replacement evidence:

| Workload | Before sparse rows | After sparse rows | Reduction |
|---|---:|---:|---:|
| BusyLoop po2=18 | 58,359,440 | 49,265,896 | 9,093,544 bytes / 15.58% |
| KeccakUnion(1) | 117,906,612 | 94,777,916 | 23,128,696 bytes / 19.62% |
| xgboost | 744,245,672 | 582,206,500 | 162,039,172 bytes / 21.77% |

## Wall-time assessment

Accepted wall-time gain: `0`.

The e2e proof gates are correctness-positive and the sparse readback reduction is real, but single-trial wall movement remains noise-level and first-segment MISC2 prewarm still dominates small-workload startup. The large structural blocker remains the final full `source=data` upload caused by CPU-dirty witness data being synced before later GPU operations consume or zeroize it.

## Next

Prioritize eliminating or avoiding the full `source=data` upload in the GPU-witgen replacement path before adding more opportunistic opcode slices.
