# SP6h NTT dynamic bind groups

Date: 2026-05-18
Worktree: `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf`
Branch: `recursive/wasm-webgpu-prover-perf`
Baseline commit: `10edcf38d SP6g: reduce fold-chain bind groups`

## Scope

SP6h extends the SP6g dynamic-offset uniform-buffer pattern from the
Poseidon2 fold-chain path to forward and inverse NTT steps. The goal is
to reduce per-NTT-level bind-group and uniform-buffer churn while keeping
the existing GPU NTT semantics unchanged.

Changed files:

- `risc0/zkp/src/hal/webgpu.rs`
- `examples/browser-prove/src/lib.rs`
- `docs/wasm-webgpu-prover.md`
- `docs/wasm-webgpu-cuda-comparison.md`

## RED

Test:

- `webgpu_hal_ntt_gpu_results_match_cpu`

Change:

- Reset WebGPU diagnostics before the focused NTT fixture.
- Assert that the combined forward expand/evaluate NTT and inverse
  interpolate NTT fixture creates exactly 4 bind groups.

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_ntt_gpu_results_match_cpu -- --nocapture
```

Expected RED failure:

```text
panicked at browser-prove/src/lib.rs:539:9:
assertion `left == right` failed
  left: 12
 right: 4
```

RED verified: PASS. The existing implementation created one NTT-step
bind group per level, producing 12 bind groups for the focused fixture.

## GREEN

Implementation:

- `dispatch_batch_expand_into_evaluate_ntt` now creates one dynamic
  NTT-step layout and one packed `webgpu_ntt_step_params` uniform buffer.
- Forward NTT step params are aligned to
  `min_uniform_buffer_offset_alignment`, packed into that buffer, and
  selected with dynamic offsets inside one compute pass.
- `dispatch_batch_interpolate_ntt` uses the same dynamic layout, packed
  params buffer, and single bind group for inverse NTT levels.
- The inverse NTT path now encodes all per-level dispatches into one
  compute pass instead of calling `dispatch_compute_1d` once per level.

No-run compile:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_ntt_gpu_results_match_cpu --no-run
```

Result:

```text
Finished `release` profile [optimized + debuginfo] target(s) in 4m 38s
```

Focused Chrome test:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_ntt_gpu_results_match_cpu -- --nocapture
```

Result:

```text
test tests::webgpu_hal_ntt_gpu_results_match_cpu ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 109 filtered out; finished in 0.11s
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
browser-prove:metric pool_prove_scheduled_async receipt_kind=Succinct wall_ms=102244 segments=11 keccaks=0 pool_size=2
browser-prove:metric pool_xgboost_smoke wall_ms=102246
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5365 cpu_mirrors=181 cpu_fallbacks=11 cpu_only_ops=0 uploads=5273 upload_bytes=15275746308 device_copies=128 device_copy_bytes=7222624256 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=37 bind_group_layout_cache_hits=1593 bind_group_creations=3358 compute_pipeline_creations=38 compute_pipeline_cache_hits=1592 buffers=6171 buffer_bytes=56466850780
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=webgpu_ntt_step_params uploads=352 upload_bytes=1540096
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 109 filtered out; finished in 102.41s
```

## Interpretation

Compared to SP6g:

- Bind groups: 9,022 -> 3,358
- Buffers: 11,835 -> 6,171
- Uploads: 10,937 -> 5,273

Compared to pre-SP6g/SP6h object churn:

- Bind groups: 12,510 -> 3,358
- Buffers: 15,323 -> 6,171
- Uploads: 14,425 -> 5,273

NTT step params uploads dropped from 6,016 to 352. Upload bytes rose from
192,512 to 1,540,096 because each dynamic uniform slice is padded to the
device minimum dynamic-offset alignment.

The xgboost wall time was 102.246 s. This is a small/noisy improvement
over the recent SP6d/SP6e/SP6g band at 102.82-102.86 s and over SP6f at
103.35 s, but the main confirmed win is sharply lower WebGPU object churn.
