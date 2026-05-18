# SP6m fused copy-preserving coefficient materialization

Date: 2026-05-18
Worktree: `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf`
Branch: `recursive/wasm-webgpu-prover-perf`
Baseline commit: `20142792a SP6l: commit dead groups in place`

## Scope

SP6m removes the remaining large `coeffs` device-copy traffic without mutating
the witness buffers that later accumulation still reads. The copy-preserving
`make_coeffs` path now fuses witness-to-coefficients materialization into the
first inverse NTT pass when WebGPU can prove that the source and destination
buffers are distinct and dispatchable.

Changed files:

- `risc0/zkp/src/prove/prover.rs`
- `risc0/zkp/src/hal/webgpu.rs`
- `examples/browser-prove/src/lib.rs`

## RED

Test:

- `webgpu_prover_commit_group_fuses_copy_into_interpolate`

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_prover_commit_group_fuses_copy_into_interpolate -- --nocapture
```

Expected RED failure:

```text
thread 'tests::webgpu_prover_commit_group_fuses_copy_into_interpolate' panicked at browser-prove/src/lib.rs:969:9:
assertion `left == right` failed: normal commit_group should fuse the copy into interpolate
  left: 1
 right: 0
browser-prove:webgpu-diag commit_group_fused_interpolate: source=coeffs device_copies=1 device_copy_bytes=32
```

RED verified: PASS. The focused test failed because the normal
copy-preserving commit path still emitted one `coeffs` device copy.

## GREEN

Implementation:

- Added a WebGPU `batch_interpolate_ntt_from` path that reads from the
  original witness buffer and writes the first inverse NTT stage to the
  coefficient buffer.
- Left the source witness readable and unchanged, preserving the later
  accumulation lifecycle for RV32IM `data` and recursion `ctrl`/`data`.
- Fell back to the previous copy-then-interpolate path whenever the fused
  dispatch is not valid.

Focused Chrome test:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_prover_commit_group_fuses_copy_into_interpolate -- --nocapture
```

Result:

```text
test tests::webgpu_prover_commit_group_fuses_copy_into_interpolate ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 113 filtered out; finished in 0.11s
```

GREEN verified: PASS.

## Xgboost smoke

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=600 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_pool_xgboost_smoke -- --nocapture
```

Result:

```text
browser-prove:metric pool_prove_scheduled_async receipt_kind=Succinct wall_ms=102255 segments=11 keccaks=0 pool_size=2
browser-prove:metric pool_xgboost_smoke wall_ms=102257
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5269 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4974 upload_bytes=10909201828 device_copies=32 device_copy_bytes=32768 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=39 bind_group_layout_cache_hits=1644 bind_group_creations=3411 compute_pipeline_creations=40 compute_pipeline_cache_hits=1643 buffers=6181 buffer_bytes=55003026812
browser-prove:webgpu-pool-op pool_xgboost_smoke: op=eltwise_copy_elem gpu_dispatches=32 cpu_mirrors=0 cpu_fallbacks=0 cpu_only_ops=0
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=webgpu_ntt_interpolate_from_params uploads=53 upload_bytes=1696
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=webgpu_ntt_step_params uploads=352 upload_bytes=1526528
browser-prove:webgpu-pool-device-copy pool_xgboost_smoke: source=final_coeffs device_copies=32 device_copy_bytes=32768
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 113 filtered out; finished in 102.42s
```

## Interpretation

SP6m completes the large device-copy cleanup but confirms that copy cleanup is
not a material xgboost wall-time lever in this Chrome run:

- Device copies: 85 -> 32
- Device-copy bytes: 5,758,812,160 -> 32,768
- `coeffs` copies: 53 -> 0
- `coeffs` copy bytes: 5,758,779,392 -> 0
- Remaining device-copy source: `final_coeffs`, 32 copies / 32,768 bytes
- Wall: 101.974 s -> 102.257 s, flat/noisy

Host-to-GPU uploads remain about 10.91 GB because the affected witnesses are
still generated on CPU and must be uploaded before the fused interpolation can
run. The next wall-time target should move or reduce CPU-originated work,
especially RV32IM accumulation/witness generation, rather than continue
optimizing device-to-device copies.
