# SP6i hash_rows output upload removal

Date: 2026-05-18
Worktree: `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf`
Branch: `recursive/wasm-webgpu-prover-perf`
Baseline commit: `b1d777bb0 SP6h: batch NTT bind groups`

## Scope

SP6i removes an unnecessary host-to-GPU upload in the Poseidon2
`hash_rows` path. The kernel overwrites every digest in the output
slice, so uploading the output buffer before dispatch is pure waste.

Changed files:

- `risc0/zkp/src/hal/webgpu.rs`
- `examples/browser-prove/src/lib.rs`

## Diagnosis

Fresh xgboost diagnostics at the SP6h baseline showed host uploads were
still dominated by a few large CPU-shadow syncs:

```text
browser-prove:webgpu-pool pool_xgboost_smoke: uploads=5273 upload_bytes=15275746308 ...
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=data uploads=476 upload_bytes=7686062080
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=nodes uploads=352 upload_bytes=4366532608
```

The `nodes` source is the Merkle node buffer. `dispatch_poseidon2_hash_rows`
called `output.sync_cpu_to_gpu(self)?` before dispatch even though the
hash_rows kernel writes the full output row range.

## RED

Test:

- `webgpu_hal_hash_rows_does_not_upload_output_buffer`

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_hash_rows_does_not_upload_output_buffer -- --nocapture
```

Expected RED failure:

```text
panicked at browser-prove/src/lib.rs:570:9:
assertion `left == right` failed
  left: 256
 right: 0
```

RED verified: PASS. The tiny fixture uploaded its 8-digest output buffer
before `hash_rows` overwrote it.

## GREEN

Implementation:

- Removed `output.sync_cpu_to_gpu(self)?` from
  `dispatch_poseidon2_hash_rows`.
- Kept `matrix.sync_cpu_to_gpu(self)?`, because the matrix is an input.
- The existing `finish_hal_op("hash_rows", ...)` path still marks the
  output GPU-dirty after dispatch.

Focused Chrome test:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_hash_rows_does_not_upload_output_buffer -- --nocapture
```

Result:

```text
Finished `release` profile [optimized + debuginfo] target(s) in 4m 43s
test tests::webgpu_hal_hash_rows_does_not_upload_output_buffer ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 110 filtered out; finished in 0.09s
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
browser-prove:metric pool_prove_scheduled_async receipt_kind=Succinct wall_ms=101930 segments=11 keccaks=0 pool_size=2
browser-prove:metric pool_xgboost_smoke wall_ms=101932
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5365 cpu_mirrors=181 cpu_fallbacks=11 cpu_only_ops=0 uploads=4921 upload_bytes=10909213700 device_copies=128 device_copy_bytes=7222624256 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=37 bind_group_layout_cache_hits=1593 bind_group_creations=3358 compute_pipeline_creations=38 compute_pipeline_cache_hits=1592 buffers=6171 buffer_bytes=56466850780
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 110 filtered out; finished in 102.09s
```

No `source=nodes` upload line was emitted after the fix. `nodes` remains
a readback source for Merkle proofs:

```text
browser-prove:webgpu-pool-readback pool_xgboost_smoke: source=nodes readbacks=672 readback_bytes=4383744
```

## Interpretation

Same-session SP6h baseline rerun:

- Uploads: 5,273
- Upload bytes: 15,275,746,308
- `nodes` upload bytes: 4,366,532,608
- Wall: 103.130 s

SP6i:

- Uploads: 4,921
- Upload bytes: 10,909,213,700
- `nodes` upload bytes: 0
- Wall: 101.932 s

The main confirmed win is a 4.37 GB upload reduction per xgboost proof.
Wall time moved in the right direction, but the measured ~1.2 s
same-session improvement should be treated as small/noisy relative to
the browser run variance.
