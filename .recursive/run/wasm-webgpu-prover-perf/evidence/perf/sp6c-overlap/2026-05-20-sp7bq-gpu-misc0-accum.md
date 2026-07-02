# SP7bq - GPU-resident MISC0 accumulator consumption

Date: 2026-05-20

Status: accepted, correctness-positive and xgboost wall-positive.

## Objective

Remove the remaining GPU-witgen MISC0 accumulator CPU bridge by computing the
MISC0 accumulator rows directly from GPU-resident witness data. This targets the
SP7bp blockers:

- xgboost `source=witgen_accum_shadow_rows readback_bytes=294892052`.
- Per-segment CPU `step_top_accum_direct_misc0` work.

## RED

Changed the representative browser e2e tests to require the new GPU-resident
API:

- `set_accum_gpu_misc0_direct_enabled`
- `accum_gpu_misc0_direct_rows`

Also tightened the xgboost accum-shadow readback cap to `<= 1_000_000` bytes.

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Expected RED result:

```text
unresolved imports `risc0_circuit_rv32im::prove::accum_gpu_misc0_direct_rows`
unresolved import `set_accum_gpu_misc0_direct_enabled`
```

## GREEN implementation

Changed files:

- `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs`
- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
- `risc0/circuit/rv32im/src/prove/mod.rs`
- `examples/browser-prove/src/lib.rs`

Key changes:

- Added default-off public GPU MISC0 accumulator controls and row counter.
- Added a Rust raw-accum wrapper that skips only GPU-witgen-owned MISC0 rows.
- Added a narrow WGSL kernel mirroring the accepted direct MISC0 accumulator
  formula rather than retrying a generated full TopAccum arm.
- Changed the accum-shadow repair path to omit MISC0 readback when GPU direct
  accum is enabled.
- Changed terminal-prefix GPU dispatch to use sparse zero-default `accum`
  upload so CPU-owned rows are uploaded without overwriting GPU-written MISC0
  rows.

## Compile gates

GREEN compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
Finished release profile in 4m23s
```

Post-format compile:

```text
cargo fmt --manifest-path examples/browser-prove/Cargo.toml
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost --no-run
Finished release profile in 2m17s
```

Hygiene:

```text
git diff --check
PASS
```

## E2E proof gates

Environment:

```text
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json
__VK_LAYER_NV_optimus=NVIDIA_only
__NV_PRIME_RENDER_OFFLOAD=1
__GLX_VENDOR_LIBRARY_NAME=nvidia
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver
WASM_BINDGEN_TEST_TIMEOUT=420
```

### BusyLoop + KeccakUnion

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: PASS.

BusyLoop:

- `wall_ms=11365`
- `gpu_active_ms=6921`
- `gpu_idle_ratio=0.391`
- `segments=1`
- `user_cycles=202872`
- `total_cycles=262144`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `rv32im_accumulate misc0_direct_gpu rows=68239`
- `source=accum upload_bytes=25165824`
- `source=witgen_accum_shadow_rows`: absent

KeccakUnion:

- `wall_ms=102304`
- `gpu_active_ms=69343`
- `gpu_idle_ratio=0.322`
- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `upload_bytes=5157917388`
- `readback_bytes=10183944`
- `source=accum upload_bytes=629145600`
- `source=rv32im_accum_misc0_direct_rows upload_bytes=505528`
- `source=witgen_accum_shadow_rows`: absent

### xgboost

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: PASS.

- `wall_ms=95324`
- `gpu_active_ms=56729`
- `gpu_idle_ratio=0.405`
- `segments=11`
- `user_cycles=2294867`
- `total_cycles=2883584`
- `gpu_dispatches=5260`
- `raw_compute_dispatches=11402`
- `queue_submits=2676`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `upload_bytes=5150602892`
- `readback_bytes=7488592`
- `source=accum upload_bytes=528482304`
- `source=rv32im_accum_misc0_direct_rows upload_bytes=3021872`
- `source=rv32im_accum_misc0_direct_params upload_bytes=352`
- `source=witgen_accum_shadow_rows`: absent
- journal decoded to `30.528042544062632`

## Delta vs SP7bp

xgboost:

- `source=witgen_accum_shadow_rows`: `294892052 -> 0` bytes.
- Total readback: `302380644 -> 7488592` bytes (`-294892052`, `-97.5%`).
- Wall: `97684 -> 95324` ms (`-2360 ms`, `-2.4%`, single browser trial).
- Total upload: `5152115064 -> 5150602892` bytes (`-1512172`).
- Raw compute dispatches: `11391 -> 11402` (`+11`, one narrow direct kernel per segment).
- Queue submits: unchanged at `2676`.

KeccakUnion:

- Wall: `102927 -> 102304` ms (`-623 ms`, single browser trial).
- `source=witgen_accum_shadow_rows`: `48433004 -> 0` bytes.

BusyLoop:

- Correctness passed and `source=witgen_accum_shadow_rows` is absent.
- Single-run wall moved `8734 -> 11365` ms. Treat this as noisy/first-run sensitive
  rather than a claimed win; do not use BusyLoop as positive wall evidence for SP7bq.

## Acceptance

Accepted because xgboost is the canonical multi-segment workload for this
blocker, proof generation and receipt verification pass with zero fallback, and
the targeted 294.9 MB bridge is removed entirely. This is a measured immediate
wall-time improvement, but not a 20% class win; the remaining large levers are
now outside the MISC0 accum-shadow bridge.
