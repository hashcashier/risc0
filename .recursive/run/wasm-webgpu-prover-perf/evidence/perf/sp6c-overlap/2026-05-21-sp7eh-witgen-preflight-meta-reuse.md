# SP7eh: GPU-witgen preflight metadata reuse

Date: 2026-05-21

## Decision

Accepted as a small GPU-witgen data-movement cleanup. This is correctness-clean
under representative browser proof gates, but it is not a material wall-time
lever by itself.

## Change

The RV32IM GPU-witgen preflight path previously uploaded the same per-cycle
preflight metadata twice for replacement-active RV32IM segments:

- `iter6d_g_arm_preflight`
- `iter6d_g_shadow_meta`

`dispatch_witgen_per_arm_probe` now uploads `iter6d_g_arm_preflight` once, runs
`shadow_init` against that same buffer, then continues with per-arm dispatch.

Code references:

- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`: shared metadata buffer inside
  `dispatch_witgen_per_arm_probe`
- `examples/browser-prove/src/lib.rs`: focused e2e assertion that
  `iter6d_g_shadow_meta` is absent from the proof delta

## RED

Focused browser proof gate:

```text
cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release \
  iter6d_g_preflight_meta_reuse_busy_loop_e2e_verify -- --nocapture
```

Corrected RED failed after receipt generation and verification because the old
path still emitted a separate shadow metadata upload:

```text
iter6d_g_shadow_init cycles=262144 meta_bytes=4194304
WebGpuUploadDiagnostics { name: "iter6d_g_arm_preflight", uploads: 1, upload_bytes: 4194304 }
WebGpuUploadDiagnostics { name: "iter6d_g_shadow_meta", uploads: 1, upload_bytes: 4194304 }
assertion failed: source `iter6d_g_shadow_meta` should be absent
```

## GREEN

Same focused browser proof gate passed:

```text
iter6d_g_shadow_init cycles=262144 meta_bytes=4194304 reused_arm_preflight=true
mask=0x0021 dispatched_arms=[0, 5]
rv32im_witgen elapsed_ms=294
test tests::iter6d_g_preflight_meta_reuse_busy_loop_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 152 filtered out; finished in 7.45s
```

The proof verified and the upload diagnostics no longer contained
`iter6d_g_shadow_meta`.

## Representative Gates

BusyLoop + KeccakUnion default representative gate:

```text
cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release \
  rv32im_default_representative_e2e_verify -- --nocapture
```

Result:

```text
test tests::rv32im_default_representative_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 152 filtered out; finished in 97.73s
```

xgboost default representative gate:

```text
cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release \
  xgboost_succinct_receipt_verifies -- --nocapture
```

First run:

```text
test tests::xgboost_succinct_receipt_verifies ... ok
test result: ok. 1 passed; 0 failed; 152 filtered out; finished in 73.17s
```

Captured rerun log: `/tmp/sp7eh-xgboost-preflight-meta-reuse.log`

Key captured lines:

```text
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
browser-prove:metric prove_session_async wall_ms=73307.0 gpu_active_ms=45175.0 gpu_idle_ratio=0.384
browser-prove:done xgboost: segments=11 user_cycles=2294869 total_cycles=2883584
browser-prove:webgpu xgboost: gpu_dispatches=5259 raw_compute_dispatches=9718 queue_submits=2740 cpu_mirrors=170 cpu_fallbacks=0 cpu_only_ops=0 uploads=3282 upload_bytes=2724587656 device_copies=74 device_copy_bytes=12075008 readbacks=352 readback_bytes=7488592
browser-prove:webgpu-upload xgboost: source=iter6d_g_arm_preflight uploads=10 upload_bytes=41943040
browser-prove:metric iter6d_g_shadow_init cycles=262144 meta_bytes=4194304 reused_arm_preflight=true
test tests::xgboost_succinct_receipt_verifies ... ok
test result: ok. 1 passed; 0 failed; 152 filtered out; finished in 73.86s
```

`iter6d_g_shadow_meta` is absent from the captured xgboost upload diagnostics.
The test also verified the expected xgboost journal
`30.528042544062632`.

## Performance Read

The deterministic upload reduction is:

- xgboost: `10 * 4 MiB = 40 MiB` less duplicated metadata upload
- BusyLoop cold default: mostly hidden by nonblocking first-segment skip
- KeccakUnion: one 4 MiB duplicate upload removed per replacement-active RV32IM
  segment

Observed wall time is essentially flat/noisy versus the current default band:

- SP7dy accepted xgboost: `72.56s`
- SP7ed fresh default: `72.76s`
- SP7eh xgboost captured proof wall: `73.31s`, total test `73.86s`
- SP7eh BusyLoop+KeccakUnion: `97.73s` total test

Accepted wall-time gain: 0 for planning purposes. The change is retained because
it removes deterministic duplicate upload work, preserves proof correctness, and
slightly simplifies the GPU-witgen data path without affecting fallback policy.

## Next Lever

This does not change the main performance estimate. The remaining material
avenues are still structural:

- chunk-complete GPU witgen/accumulation, likely the only credible path to a
  multi-second segment-local reduction
- larger sparse upload/zeroize reductions that preserve proof-level semantics
- browser submission/readback drain reduction only if a focused gate shows real
  wall movement on KeccakUnion and xgboost

