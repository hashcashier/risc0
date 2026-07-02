# SP7bo - Direct MISC0 Accum Stepping Stone

## Goal

Test whether the remaining CPU `step_TopAccum` work can be reduced for the rows already owned by the GPU-witgen MISC0 replacement path, without weakening e2e proof correctness.

This is not a full GPU accumulator offload. It is a default-off direct Rust accumulator path for GPU-short-circuited MISC0 rows, intended to prove the exact per-arm accumulator formula before moving it into WGSL/GPU-resident execution.

## RED

Added representative e2e assertions that enable direct MISC0 accum and require the row counter to increase for BusyLoop, KeccakUnion, and xgboost.

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: expected compile failure.

- `set_witgen_gpu_direct_misc0_accum_enabled` unresolved.
- `witgen_gpu_direct_misc0_accum_rows` unresolved.

## Change

Added an opt-in direct MISC0 accumulator path:

- `set_witgen_gpu_direct_misc0_accum_enabled(enabled: bool)`.
- `witgen_gpu_direct_misc0_accum_rows()`.
- `run_accum_steps` skips generated `step_TopAccum` only when direct accum is enabled, the cycle was short-circuited by GPU witgen, and `major == 0`.
- Direct path writes the exact MISC0 TopAccum machine-column terms plus the user-accum NOP state.
- Existing terminal prefix and machine-column carry postprocessing remain unchanged.

## Compile

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: PASS in `4m22s` with warnings only for existing unused probe/codegen fields.

Post-format command:

```bash
cargo fmt --manifest-path examples/browser-prove/Cargo.toml
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
```

Result: PASS in `2m15s` with the same warnings.

## GREEN - BusyLoop + KeccakUnion

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: PASS.

BusyLoop:

- `wall_ms=9055.0`.
- `gpu_active_ms=3999.0`.
- `gpu_idle_ratio=0.558`.
- `segments=1`.
- `user_cycles=202872`.
- `total_cycles=262144`.
- `rv32im_witgen_accum_shadow_gpu_sync rows=68239`.
- `rv32im_accumulate step_top_accum_direct_misc0 elapsed_ms=1823.000`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.

KeccakUnion:

- `wall_ms=102947.0`.
- `gpu_active_ms=69444.0`.
- `gpu_idle_ratio=0.325`.
- `segments=4`.
- `user_cycles=747265`.
- `total_cycles=917504`.
- `source=witgen_accum_shadow_rows readbacks=4 readback_bytes=48433004`.
- `accum uploads=75 upload_bytes=1007157248`.
- `recursion_data uploads=200 upload_bytes=3355443200`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.

## GREEN - xgboost

First retry failed before proof generation due to non-representative Chrome limits:

- `max_buffer_size=1073741824`.
- `max_storage_buffer_binding_size=1073741824`.
- `max_compute_workgroup_storage_size=32768`.
- Harness assertion failed at the representative WebGPU limit gate.

Successful command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: PASS.

- High limits: `max_buffer_size=4294967292`, `max_storage_buffer_binding_size=2147483644`, `max_compute_workgroup_storage_size=49152`.
- `wall_ms=98463.0`.
- `gpu_active_ms=56839.0`.
- `gpu_idle_ratio=0.423`.
- `segments=11`.
- `user_cycles=2294867`.
- `total_cycles=2883584`.
- `gpu_dispatches=5260`.
- `raw_compute_dispatches=11380`.
- `queue_submits=2665`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.
- `upload_bytes=5889877296`.
- `readback_bytes=302380644`.
- `source=accum uploads=119 upload_bytes=1716518912`.
- `source=recursion_data uploads=168 upload_bytes=2818572288`.
- `source=witgen_accum_shadow_rows readbacks=11 readback_bytes=294892052`.

## Delta

Compared with the latest accepted SP7bn xgboost run:

- Wall: `100003.0 -> 98463.0`, about `-1540ms` (`-1.54%`).
- `gpu_active_ms`: `56662.0 -> 56839.0`, effectively flat.
- `gpu_idle_ratio`: `0.433 -> 0.423`, slightly lower idle share.
- `witgen_accum_shadow_rows`: unchanged at about `294.9MB`.
- `source=accum upload_bytes`: unchanged at `1716518912`.
- CPU fallback: remains `0`.
- CPU-only HAL ops: remains `0`.

## Acceptance

Accepted as correctness-positive and a small wall-time improvement, but not as a major performance lever.

This does not remove the remaining bridge:

- `witgen_accum_shadow_rows` remains `294892052` bytes for xgboost.
- `source=accum upload_bytes` remains `1716518912` bytes for xgboost.
- The main remaining target is still moving the MISC0 accumulator consumer to GPU-resident execution so CPU `TopAccum` no longer needs GPU-owned witness rows.

Next work should focus on the GPU-resident accumulator path rather than broadening more opcode arms or pursuing iframe/multi-device side paths.
