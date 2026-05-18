# SP6g Poseidon2 fold-chain dynamic bind group

Date: 2026-05-18

Purpose: reduce the largest safe slice of remaining bind-group churn without
caching bind groups that retain transient buffers. The Poseidon2 fold-chain
Merkle path previously created one params buffer and one bind group per layer
even though all layers share the same long-lived buffers and differ only by a
32-byte params block.

## RED

Test-first change: extend `webgpu_hal_core_gpu_results_match_cpu` with a
four-layer `hash_fold_chain_async` fixture that:

- compares the GPU-authoritative batched chain against serial `hash_fold`
  output, and
- asserts the chain records four GPU dispatches but only one bind-group
  creation.

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_core_gpu_results_match_cpu -- --nocapture
```

Expected failure after fixing the test fixture size:

```text
panicked at browser-prove/src/lib.rs:744:9:
assertion `left == right` failed
  left: 4
 right: 1
```

## GREEN implementation

- Added a separate Poseidon2 `fold_chain_layout` with
  `WebGpuBindingLayout::uniform_dynamic(4, 32)` so the single-fold path keeps
  its existing non-dynamic layout.
- Added a `fold_chain_kernel` compiled against that dynamic layout.
- Recorded `min_uniform_buffer_offset_alignment` from device limits.
- Packed all per-layer fold params into one aligned uniform buffer and uploaded
  it once per chain.
- Created one bind group for the whole chain and called
  `GPUComputePassEncoder.setBindGroup` with a per-dispatch dynamic offset.

## Verification

Focused no-run compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_core_gpu_results_match_cpu --no-run
Finished `release` profile [optimized + debuginfo] target(s) in 4m 39s
```

Focused browser test:

```text
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 109 filtered out; finished in 0.40s
```

Full xgboost pooled smoke with diagnostics:

```text
browser-prove:metric pool_prove_scheduled_async receipt_kind=Succinct wall_ms=102859 segments=11 keccaks=0 pool_size=2
browser-prove:metric pool_xgboost_smoke wall_ms=102861
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5365 cpu_mirrors=181 cpu_fallbacks=11 cpu_only_ops=0 uploads=10937 upload_bytes=15274398724 device_copies=128 device_copy_bytes=7222624256 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=37 bind_group_layout_cache_hits=1593 bind_group_creations=9022 compute_pipeline_creations=38 compute_pipeline_cache_hits=1592 buffers=11835 buffer_bytes=56465503196
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 109 filtered out; finished in 103.02s
```

## Interpretation

- The change reduces xgboost object churn materially:
  - bind-group creations: 12,510 -> 9,022
  - buffer allocations: 15,323 -> 11,835
  - uploads: 14,425 -> 10,937
- Upload bytes increase slightly because dynamic uniform offsets require
  alignment padding: `webgpu_poseidon2_fold_params` was 3,712 uploads /
  118,784 bytes; `webgpu_poseidon2_fold_chain_params` is 224 uploads /
  950,272 bytes. The extra ~0.83 MiB is negligible next to the 15.27 GiB total
  upload volume.
- Wall time remains flat at 102.86 s, matching SP6d/SP6e within run noise.
  The xgboost CUDA gap is therefore not primarily bind-group construction in
  the Poseidon2 fold-chain path.
