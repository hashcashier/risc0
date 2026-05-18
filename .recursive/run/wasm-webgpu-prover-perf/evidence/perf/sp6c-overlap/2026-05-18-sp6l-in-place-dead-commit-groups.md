# SP6l in-place dead commit groups

Date: 2026-05-18
Worktree: `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf`
Branch: `recursive/wasm-webgpu-prover-perf`
Baseline commit: `e11533b41 SP6k: attribute device copy diagnostics`

## Scope

SP6l removes `make_coeffs` device copies only for commit groups whose witness
buffer is dead after commit. The generic copy remains for groups that are read
after their Merkle root is committed.

Changed files:

- `risc0/zkp/src/prove/prover.rs`
- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
- `risc0/circuit/recursion/src/prove/hal/webgpu.rs`
- `risc0/circuit/keccak/src/prove/hal/webgpu.rs`
- `examples/browser-prove/src/lib.rs`

## RED

Test:

- `webgpu_prover_commit_group_in_place_avoids_coeffs_device_copy`

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_prover_commit_group_in_place_avoids_coeffs_device_copy -- --nocapture
```

Expected RED failure:

```text
error[E0599]: no method named `commit_group_async_in_place` found for struct `risc0_zkp::prove::Prover` in the current scope
   --> browser-prove/src/lib.rs:931:14
```

RED verified: PASS. The focused test described the missing API and failed
before any production implementation existed.

## GREEN

Implementation:

- Added `Prover<WebGpuHal>::commit_group_async_in_place` and scoped variant.
- Added `make_coeffs_in_place_async`, which mutates the consumed witness only
  when WebGPU can run `batch_interpolate_ntt` and `zk_shift` on that buffer.
  Otherwise it falls back to the copy-preserving `make_coeffs_async` path.
- Wired in-place commit for:
  - RV32IM `code`
  - RV32IM `accum`
  - recursion `accum`
  - Keccak `code`, `data`, and `accum`
- Kept normal copy commit for RV32IM `data` and recursion `ctrl`/`data`,
  because those witnesses are read by later accumulation after the transcript
  has committed their Merkle roots.

Focused Chrome test:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_prover_commit_group_in_place_avoids_coeffs_device_copy -- --nocapture
```

Result:

```text
test tests::webgpu_prover_commit_group_in_place_avoids_coeffs_device_copy ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 112 filtered out; finished in 0.09s
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
browser-prove:metric pool_prove_scheduled_async receipt_kind=Succinct wall_ms=101972 segments=11 keccaks=0 pool_size=2
browser-prove:metric pool_xgboost_smoke wall_ms=101974
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5322 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4921 upload_bytes=10909213700 device_copies=85 device_copy_bytes=5758812160 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=37 bind_group_layout_cache_hits=1593 bind_group_creations=3358 compute_pipeline_creations=38 compute_pipeline_cache_hits=1592 buffers=6128 buffer_bytes=55003038684
browser-prove:webgpu-pool-device-copy pool_xgboost_smoke: source=coeffs device_copies=53 device_copy_bytes=5758779392
browser-prove:webgpu-pool-device-copy pool_xgboost_smoke: source=final_coeffs device_copies=32 device_copy_bytes=32768
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 112 filtered out; finished in 102.13s
```

## Interpretation

SP6l reduces xgboost structural copy traffic but does not materially move wall
time:

- Device copies: 128 -> 85
- Device-copy bytes: 7,222,624,256 -> 5,758,812,160
- `coeffs` copies: 96 -> 53
- `coeffs` copy bytes: 7,222,591,488 -> 5,758,779,392
- Buffers: 6,171 -> 6,128
- Buffer bytes: 56,466,850,780 -> 55,003,038,684
- Wall: 101.974 s, within the SP6i-SP6k noise band

The remaining `coeffs` copies correspond to groups whose witness is still
needed after commit: RV32IM `data`, recursion `ctrl`, and recursion `data`.
Eliminating those copies would require a different protocol/lifecycle shape,
because the transcript mix used by accumulation depends on the already
committed Merkle roots.
