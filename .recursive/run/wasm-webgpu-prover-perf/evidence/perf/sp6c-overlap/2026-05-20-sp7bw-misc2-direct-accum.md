Run: wasm-webgpu-prover-perf
Evidence: SP7bw MISC2 accumulator-only direct coverage
Date: 2026-05-20

## Decision

Accepted.

This keeps MISC2 witness generation on CPU, keeps the MISC2 GPU-witgen
replacement mask disabled, and offloads only the MISC2 TopAccum contribution to
a narrow WebGPU kernel. This avoids the shadow-repair readbacks that rejected
SP7bv while covering the large xgboost major-2 accumulator bucket.

## Changed Paths

- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
- `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs`
- `risc0/circuit/rv32im/src/prove/mod.rs`
- `examples/browser-prove/src/lib.rs`

## Implementation Summary

- Added opt-in `set_accum_gpu_misc2_direct_enabled` and
  `accum_gpu_misc2_direct_rows`.
- Reused the narrow direct accumulator WGSL shape for MISC0 and MISC2 by
  parameterizing the common `FinalizeMiscLayout` / `DoCycleTableLayout` /
  `MiscInputLayout` fields.
- Added an `rv32im_accum_misc2_direct` kernel that consumes CPU-witgen-owned
  data rows where `preflight.major == 2`.
- Added a CPU accumulation path that skips both accepted MISC0 direct rows and
  major-2 rows, then runs MISC0 direct, MISC2 direct, terminal ext prefix, and
  machine-column carry on WebGPU.
- Tightened the representative browser tests to assert MISC2 direct accumulator
  row counters increase while the MISC2 GPU-witgen replacement mask remains off.

## Validation Commands

Compile:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in 4m23s.

BusyLoop + KeccakUnion browser e2e:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: passed in 110.66s.

xgboost browser e2e, trial 1:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: passed in 93.09s.

xgboost browser e2e, trial 2:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: passed in 93.50s.

## E2E Proof Results

BusyLoop:

- Receipt verified.
- `wall_ms=8326`
- `gpu_active_ms=3980`
- `gpu_idle_ratio=0.522`
- `raw_compute_dispatches=720`
- `queue_submits=174`
- `upload_bytes=210934220`
- `readback_bytes=478272`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- First segment direct rows: MISC0 `68239`, MISC2 `25080`.

KeccakUnion:

- Receipt verified.
- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`
- `wall_ms=102019`
- `gpu_active_ms=69315`
- `gpu_idle_ratio=0.321`
- `raw_compute_dispatches=12712`
- `queue_submits=3079`
- `upload_bytes=2998557000`
- `readback_bytes=10183944`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

xgboost trial 1:

- Receipt verified.
- Journal decoded to `30.528042544062632`.
- `segments=11`
- `wall_ms=92843`
- `gpu_active_ms=56612`
- `gpu_idle_ratio=0.390`
- `raw_compute_dispatches=11455`
- `queue_submits=2729`
- `upload_bytes=3133253624`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

xgboost trial 2:

- Receipt verified.
- Journal decoded to `30.528042544062632`.
- `segments=11`
- `wall_ms=93262`
- `gpu_active_ms=56786`
- `gpu_idle_ratio=0.391`
- `raw_compute_dispatches=11455`
- `queue_submits=2729`
- `upload_bytes=3133095596`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

## Comparison

Recent accepted xgboost post-revert series before this candidate:

- `94385 ms`
- `94980 ms`
- `94792 ms`
- `95680 ms`
- Mean: `94959.25 ms`

SP7bw xgboost candidate series:

- `92843 ms`
- `93262 ms`
- Mean: `93052.5 ms`

Observed movement:

- Mean vs recent accepted mean: `-1906.75 ms`, about `-2.0%`.
- Candidate mean vs best recent accepted trial (`94385 ms`): `-1332.5 ms`,
  about `-1.4%`.
- Candidate upload bytes vs latest accepted xgboost (`3193303564`):
  `3133253624` / `3133095596`, about `-60 MB`.
- Candidate adds one MISC2 direct dispatch and one MISC2 row-list upload per
  segment, so raw dispatches/submits move from the accepted `11444` / `2718`
  shape to `11455` / `2729`.

## Correctness Gate

Passed.

- High Chrome WebGPU limits negotiated in every browser e2e run:
  `max_buffer_size=4294967292`,
  `max_storage_buffer_binding_size=2147483644`,
  `max_compute_workgroup_storage_size=49152`.
- BusyLoop, KeccakUnion, and xgboost all generated verified receipts.
- xgboost journal remained `30.528042544062632`.
- `cpu_fallbacks=0` and `cpu_only_ops=0` for all accepted proof runs.
- MISC2 GPU-witgen replacement remained disabled (`mask=0x0001`,
  `dispatched_arms=[0]`); this candidate did not reintroduce SP7bv's MISC2
  shadow-repair path.
- Existing no-shadow-readback assertions in the representative tests remained
  active and passed.

## Follow-up

The remaining xgboost wall is still dominated by RV32IM witness/accumulation
and sparse upload streams. This accepted change is modest but repeatably
wall-positive; next work should stay on larger RV32IM CPU-accumulation buckets
or sparse upload mechanisms with the same representative e2e gate.
