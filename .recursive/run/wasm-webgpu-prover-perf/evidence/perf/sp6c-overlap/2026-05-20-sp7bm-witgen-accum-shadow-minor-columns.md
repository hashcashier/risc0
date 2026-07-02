# SP7bm - Witgen Accum-Shadow Per-Minor Column Sync

## Goal

Reduce the remaining GPU-witgen `witgen_accum_shadow_rows` bridge without weakening proof correctness. The previous accepted SP7bk path reduced xgboost from `406998464` to `370728064` bytes, but still read broad per-class column sets for every GPU-owned MISC0 row.

## RED

Lowered the xgboost e2e cap from `390000000` to `340000000` bytes.

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: expected failure after proof generation.

- `source=witgen_accum_shadow_rows readbacks=11 readback_bytes=370728064`.
- New assertion failed: `readback_bytes=370728064 max_bytes=340000000`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.
- High WebGPU limits were active.

## Diagnostic

Added temporary CPU `step_TopAccum` load tracing for GPU-owned MISC0 rows, then removed it before acceptance.

Trace result for data columns read by `step_TopAccum`:

- Minor 0: `0,1,14,15,17,18,21,29..40,42,43,45,46,48,49,54..75,86,87,90..125,128..131`.
- Minor 1: minor 0 plus `22`.
- Minor 7: minor 0 plus `22..28`.
- Minor 2: minor 0 plus `22,23` and `132..195`.
- Minor 3: minor 0 plus `22..24` and `132..195`.
- Minor 4: minor 0 plus `22..25` and `132..195`.

The temporary trace was removed. Hygiene check:

```bash
rg -n "debug_accum_data_load_cols|AccumData|accum_data\\(|reset_accum_data_load_trace|record_accum_data_load" risc0/circuit/rv32im/src/prove/hal/rust_steps.rs risc0/circuit/rv32im/src/prove/hal/webgpu.rs examples/browser-prove/src/lib.rs
```

Result: no matches.

## Change

Replaced the broad MISC0 accum-shadow row sync with per-minor column groups:

- MISC0 minor 0: exact common column set.
- MISC0 minor 1: common plus `22`.
- MISC0 minor 7: common plus `22..28`.
- MISC0 minors 2, 3, 4: common plus the minor-specific compare/bit columns and `132..195`.
- MISC2 compare/branch groups remain preserved for future diff/probe work, but production replacement still keeps MISC2 disabled.

This keeps CPU `TopAccum` correct while reducing the GPU-owned witness cells read back to CPU shadow memory.

## Compile

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
```

Result: PASS in `4m27s` with warnings only for existing unused probe/codegen fields.

## GREEN - xgboost

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: PASS.

- `wall_ms=99832`.
- `segments=11`.
- `user_cycles=2294867`.
- `total_cycles=2883584`.
- Journal: `30.528042544062632`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.
- `gpu_dispatches=5260`.
- `raw_compute_dispatches=11380`.
- `queue_submits=2665`.
- total `readback_bytes=302380644`.
- `source=witgen_accum_shadow_rows readbacks=11 readback_bytes=294892052`.

Delta vs SP7bk/SP7bl xgboost:

- `witgen_accum_shadow_rows`: `370728064 -> 294892052` (`-75836012` bytes, about `-20.45%` for that source).
- total readback bytes: `378216656 -> 302380644` (`-75836012` bytes).
- wall: `99800 -> 99832` vs the immediate SP7bl cleanup run, noise-level.

## GREEN - BusyLoop + KeccakUnion

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: PASS.

BusyLoop:

- `wall_ms=8952`.
- `segments=1`.
- `user_cycles=202872`.
- `total_cycles=262144`.
- `rv32im_witgen_accum_shadow_gpu_sync rows=68239`.
- total `readback_bytes=27257264`.
- `gpu_dispatches=329`.
- `raw_compute_dispatches=715`.
- `queue_submits=170`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.

KeccakUnion:

- `wall_ms=102731`.
- `segments=4`.
- `pending_keccaks=9`.
- `assumptions=1`.
- `source=witgen_accum_shadow_rows readbacks=4 readback_bytes=48433004`.
- `gpu_dispatches=5900`.
- `raw_compute_dispatches=12650`.
- `queue_submits=3021`.
- `cpu_fallbacks=0`.
- `cpu_only_ops=0`.

## Acceptance

Accepted deterministic data-movement gain:

- xgboost `witgen_accum_shadow_rows` readback bytes: `370728064 -> 294892052`.
- Reduction: `75836012` bytes, about `20.45%` for that source.

Accepted wall-time gain: `0`. The xgboost wall moved from `99800` to `99832` against the immediate post-cleanup run, so this is not a reliable wall-time improvement.

Next immediate target remains eliminating the remaining `294892052` byte bridge by making accumulation/TopAccum consume GPU-owned witness data directly, or by building a smaller GPU-resident accumulator consumer with real e2e wall gates. Do not broaden more opcode arms without representative wall proof.
