# SP7cy: enable accepted GPU witgen/direct-accum path for default WebGPU prover

Date: 2026-05-20

## Problem

The accepted SP7cq working state depended on focused browser tests manually
setting:

- `set_witgen_gpu_probe_enabled(true)`
- `set_witgen_gpu_replace_enabled(true)`
- `set_accum_gpu_misc0_direct_enabled(true)`
- `set_accum_gpu_misc1_direct_enabled(true)`
- `set_accum_gpu_misc2_direct_enabled(true)`

The canonical `webgpu_prover()` path constructed `WebGpuProver::new()` without
enabling those flags, so production/default proof generation still followed the
older default path. This meant the fastest correctness-proven path was not the
actual browser WebGPU default.

## Change

Added `enable_webgpu_witgen_accum_acceleration_for_hal(hal)` in
`risc0/circuit/rv32im/src/prove/hal/webgpu.rs` and re-exported it from
`risc0/circuit/rv32im/src/prove/mod.rs`.

`WebGpuProver::new()` now:

1. creates the canonical single-device browser `WebGpuHal`;
2. enables the accepted GPU-witgen replacement and direct MISC0/1/2
   accumulator flags;
3. starts the existing witgen replacement prewarm on that HAL;
4. returns the normal `WebGpuProver`.

The existing `WebGpuProver::from_hal(...)` prewarm hook is retained for focused
tests and caller-supplied HALs that explicitly opt into the flags.

Strengthened default e2e gates:

- `rv32im_default_representative_e2e_verify` now asserts the default BusyLoop
  and `KeccakUnion(1)` proofs actually run GPU-witgen replacement and direct
  MISC0/1/2 accumulation.
- `xgboost_succinct_receipt_verifies` now asserts the same default-path
  acceleration, with no full data readback and coalesced accum-shadow
  readbacks.

## Compile and Hygiene

No-run compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_succinct_receipt_verifies --no-run

Finished `release` profile [optimized + debuginfo] target(s) in 4m 26s
```

Format and diff hygiene:

```text
cargo fmt --check --manifest-path risc0/circuit/rv32im/Cargo.toml
cargo fmt --check --manifest-path risc0/zkvm/Cargo.toml
cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml
git diff --check

all passed
```

## Default BusyLoop + KeccakUnion e2e

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
    rv32im_default_representative_e2e_verify -- --nocapture
```

Result: passed in 99.34 s with high WebGPU limits
`4294967292 / 2147483644 / 49152`.

| Workload | wall_ms | gpu_active_ms | raw dispatches | queue submits | cpu_fallbacks | cpu_only_ops |
|---|---:|---:|---:|---:|---:|---:|
| BusyLoop po2_18 default | 7507 | 3445 | 609 | 173 | 0 | 0 |
| KeccakUnion(1) default | 91537 | 60005 | 10660 | 3075 | 0 | 0 |

KeccakUnion retained representative shape:

- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`

The strengthened assertions proved that both workloads used:

- GPU-witgen replacement (`witgen_gpu_short_circuit_cycles` advanced);
- direct MISC0/1/2 GPU accumulation;
- no on-demand replacement kernel compiles after prewarm;
- no CPU `step_Top` replay for accum-shadow repair;
- no full `data` readback.

## Default xgboost e2e

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
    xgboost_succinct_receipt_verifies -- --nocapture
```

Result: passed in 78.72 s with high WebGPU limits, verified journal
`30.528042544062632`, and zero fallback/CPU-only counters.

| Workload | wall_ms | segments | gpu_active_ms | raw dispatches | queue submits | uploads | upload_bytes | readback_bytes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| xgboost default | 78466 | 11 | 45133 | 9674 | 2718 | 3236 | 3114046620 | 7488592 |

The strengthened assertions proved the default xgboost proof used GPU-witgen
replacement and direct MISC0/1/2 accumulation, with no full `data` readback and
at most one accum-shadow row readback per segment.

## Invalid rerun

A second xgboost attempt did not reach proof generation because Chrome
negotiated the known low-limit SP7am failure profile:

```text
max_buffer_size=1073741824
max_storage_buffer_binding_size=1073741824
max_compute_workgroup_storage_size=32768
```

The high-limit guard failed immediately before proof generation. This is not
valid performance evidence and was not included in the wall-time comparison.

## Comparison

Compared with the prior production/default SP7am gate:

| Workload | Prior default wall_ms | New default wall_ms | Movement |
|---|---:|---:|---:|
| BusyLoop po2_18 | 7542 | 7507 | -35 ms |
| KeccakUnion(1) | 103542 | 91537 | -12005 ms |
| xgboost | 100056 | 78466 | -21590 ms |

Compared with the accepted focused SP7cq xgboost mean (`77417` / `77523`,
mean `77470 ms`), the new default xgboost sample is slower by `996 ms`
(`+1.3%`). Treat that as cold-session/noise-level overhead for this integration,
not a new kernel-level improvement. The important acceptance criterion here is
that the real default prover now reaches the already-accepted SP7cq-class path.

## Decision

Accepted.

This is a production/default-path integration win:

- default `webgpu_prover()` now uses the accepted GPU witgen/direct-accum path;
- BusyLoop, KeccakUnion, and xgboost all prove successfully with verified
  receipts/journal and zero fallback/CPU-only counters;
- prior default xgboost wall drops from about `100.1 s` to `78.5 s`;
- prior default KeccakUnion wall drops from about `103.5 s` to `91.5 s`.

Current performance interpretation:

- canonical default xgboost sample: `78.466 s`;
- focused accepted SP7cq working-state estimate remains about `77.5 s`;
- accepted kernel-level wall-time gain over SP7cq: `0 s`;
- accepted production/default-path wall-time reduction versus SP7am default:
  about `21.6 s` on xgboost and `12.0 s` on KeccakUnion.
