# SP7cp: all-MISC0 direct accumulator candidate rejected

Date: 2026-05-20
Status: rejected and reverted

## Candidate

Extend the existing narrow MISC0 direct-accumulator WGSL from the
GPU-witgen-owned MISC0 minors `{0,1,2,3,4,7}` to every MISC0 row, including
CPU-witgen-owned compare minors `{5,6}`.

Implementation shape:

- Build `misc0_rows` from all `preflight.cycles` with `major == 0`.
- Include bit `0` in the CPU TopAccum skip major mask.
- Reuse the existing grouped MISC0/MISC1/MISC2 direct-accum dispatch.

This was intentionally a bounded reuse of the accepted direct formula, not a
new generated TopAccum arm.

## Compile gate

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Candidate result: passed in `4m24s`.

## Representative e2e proof gate

Command:

```text
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
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Log: `/tmp/sp7cp-busy-keccak-misc0-all-direct-accum.log`

Result: passed with verified BusyLoop and KeccakUnion receipts, high WebGPU
limits, and zero fallback/CPU-only counters.

BusyLoop:

- `wall_ms=7490`
- `gpu_active_ms=3240`
- `raw_compute_dispatches=721`
- `queue_submits=173`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- CPU TopAccum timer: `step_top_accum_cpu_skip_replaced_misc0=false_major_mask=0x0007 elapsed_ms=1138`
- MISC0 direct rows: `77196`
- MISC1 direct rows: `7111`
- MISC2 direct rows: `25080`

KeccakUnion:

- `wall_ms=92624`
- `gpu_active_ms=60866`
- `raw_compute_dispatches=12716`
- `queue_submits=3075`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- MISC0 direct row upload bytes: `505636`

## Comparison against SP7cm

SP7cm accepted baseline:

- BusyLoop `wall_ms=7276`
- KeccakUnion `wall_ms=92303`
- BusyLoop raw dispatch/submits `721/173`
- KeccakUnion raw dispatch/submits `12716/3075`

SP7cp candidate:

- BusyLoop `7276 -> 7490` (`+214 ms`, `+2.9%`)
- KeccakUnion `92303 -> 92624` (`+321 ms`, `+0.35%`)
- Raw dispatch/submits unchanged.
- MISC0 direct coverage increased for BusyLoop (`68239` accepted-path rows to `77196`), but CPU TopAccum did not improve enough to offset the altered skip path and row-list work.

## Decision

Rejected before xgboost. Correctness was clean, but both representative
short/medium workloads regressed and the candidate had no dispatch-count
reduction. Running xgboost would be a sidequest under the current prioritization
because the mechanism already failed the wall gate.

The candidate was reverted to the accepted behavior:

- MISC0 direct-accum rows are again limited to GPU-witgen-owned MISC0 minors
  `{0,1,2,3,4,7}` when replacement mask bit `0` is active.
- CPU TopAccum skip mask again covers only MISC1/MISC2 as `major_mask=0x0006`,
  with replaced-MISC0 skipping handled by the existing `skip_replaced_misc0`
  path.

Post-revert gates:

- `cargo fmt --manifest-path risc0/circuit/rv32im/Cargo.toml` passed.
- `git diff --check` passed.
- Compile after revert passed in `4m24s`:
  `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`.

Do not retry all-MISC0 direct accumulation in this shape. Future accumulator
work needs a larger target than compare-minor coverage or a lower-overhead GPU
consumer for the remaining TopAccum CPU bucket.
