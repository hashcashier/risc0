# SP7bl: TopAccum arm0 batch-inverse candidate rejected

Date: 2026-05-20

## Goal

Move the next material MISC0 bottleneck from CPU to GPU by replacing `TopAccum` major 0 with an authoritative generated WGSL path. This targeted the remaining xgboost `witgen_accum_shadow_rows` bridge left after SP7bk (`370,728,064` bytes).

## RED

Added a focused browser e2e gate:

```text
rv32im_accum_topaccum_arm0_authoritative_e2e_verify
```

The first compile failed as intended because the arm0 authoritative API did not exist:

```text
unresolved imports:
  accum_gpu_arm0_authoritative_dispatches
  set_accum_gpu_arm0_authoritative_enabled
```

## Candidate

Implemented an opt-in major-0 authoritative path:

- generated `step_TopAccumArm0` from the reproducible TopAccum arm slicer
- replaced 25 `ext_inv(...)` calls with capture/consume adapters
- added one batched inverse kernel per row, reducing inverse formula work from one inverse per denominator to one extension inverse per row
- skipped CPU `step_TopAccum` major 0 and ran terminal prefix + machine-column carry on GPU

The wasm compile gate passed:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm0_authoritative_e2e_verify --no-run
```

Result: pass in `4m23s`; warnings only.

## Browser e2e failure

High-limit Chrome/WebGPU was active:

```text
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152
```

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm0_authoritative_e2e_verify -- --nocapture
```

Result: failed on BusyLoop before receipt verification.

Key metrics:

```text
fixture=multi_test/busy_loop_po2_18_topaccum_arm0_authoritative
rv32im_witgen=538 ms
commit_group_async rv32im_data=264 ms
rv32im_accumulate step_top_accum_cpu_skip_major0=1507 ms
topaccum_arm0_authoritative cycles=77196
topaccum_arm0_authoritative dispatched cycles=77196 inv_items=1929900 batched_inv_items=77196
rv32im_witgen_accum=1573 ms
commit_group_async rv32im_accum=47525 ms
prove_session_async=50210 ms
raw_compute_dispatches=192
queue_submits=36
buffers=49
buffer_bytes=2200147008
cpu_fallbacks=0
cpu_only_ops=0
```

Failure:

```text
AbortError: Failed to execute 'mapAsync' on 'GPUBuffer':
A valid external Instance reference no longer exists.
```

## Decision

Rejected. Even with per-row batch inversion, generated arm0 TopAccum still reproduces the same device-loss / hidden-GPU-work failure shape as SP7m. The inverse count reduction is real (`1,929,900` formula inverses down to `77,196` row inverses), but the generated arm0 body remains too expensive or capacity-hostile for the browser e2e proof path.

Accepted wall-time gain: `0`.

Do not retry full generated arm0 TopAccum unless the strategy first proves a lower-complexity real-GPU gate that avoids the ~45-47 s hidden `rv32im_accum` work and browser instance loss.

## Cleanup Validation

The unvalidated arm0 runtime flag/API, kernels, dispatch branch, and e2e test were removed.

Compile gate after cleanup:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: pass in `4m24s`; warnings only.

Representative browser proof gate after cleanup:

```bash
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: pass.

```text
BusyLoop:
  wall_ms=9256
  total_cycles=262144
  cpu_fallbacks=0
  cpu_only_ops=0
  witgen_accum_shadow_rows readbacks=1
  witgen_accum_shadow_rows bytes=34394720

KeccakUnion(1):
  wall_ms=102880
  segments=4
  pending_keccaks=9
  assumptions=1
  cpu_fallbacks=0
  cpu_only_ops=0
  witgen_accum_shadow_rows readbacks=4
  witgen_accum_shadow_rows bytes=60937792
```

Multi-segment xgboost proof gate after cleanup:

```bash
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_xgboost -- --nocapture
```

Result: pass.

```text
xgboost:
  wall_ms=99800
  segments=11
  user_cycles=2294867
  total_cycles=2883584
  journal=30.528042544062632
  cpu_fallbacks=0
  cpu_only_ops=0
  raw_compute_dispatches=11380
  queue_submits=2665
  upload_bytes=5889868092
  readback_bytes=378216656
  witgen_accum_shadow_rows readbacks=11
  witgen_accum_shadow_rows bytes=370728064
```

## Next Direction

Full generated arm0 TopAccum is not the next immediate path. The viable target remains the same structural bottleneck, but via a narrower GPU-owned accumulation consumer:

- avoid broad generated TopAccum arms that include the full closure shape
- target the MISC0 replacement rows and accumulator columns that currently force the `370,728,064` byte bridge
- require BusyLoop + KeccakUnion proof first, then xgboost proof, before claiming any wall-time gain
