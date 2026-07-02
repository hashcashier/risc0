Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7r -- chunked batch_evaluate_any 2D partial dispatch
Date: 2026-05-18

## Headline

`batch_evaluate_any_chunked_async` now uses one 2D partial-evaluation
dispatch per chunked call when `coeffs` and `which` fit storage bindings. The
old path submitted one partial dispatch per evaluation and uploaded one
`webgpu_batch_evaluate_any_partial_params` buffer for each. The oversized-buffer
fallback keeps the old dynamic-slice path.

This is accepted as a production command/parameter reduction. The single-trial
xgboost wall movement is favorable, but still treated as noisy.

## RED

Extended the focused browser HAL coverage added in SP7q. The test already
compared the chunked evaluator against the existing evaluator and asserted no
`batch_evaluate_partials` readback. SP7r added the behavioral assertion that the
chunked path must not upload per-eval
`webgpu_batch_evaluate_any_partial_params`.

RED browser result:

```text
test tests::webgpu_hal_core_gpu_results_match_cpu ... FAILED

panicked at browser-prove/src/lib.rs:867:9:
chunked batch_evaluate_any should use one 2D partial dispatch instead of per-eval params
```

## GREEN

Implemented:

- `BATCH_EVALUATE_ANY_PARTIAL_2D_WGSL`
- fast-path binding of full `coeffs`, `which`, `xs`, and `partials` storage buffers
- 2D partial dispatch with `gid.x = chunk_idx` and `gid.y = eval_idx`
- retained dynamic-slice fallback for oversized `coeffs` or `which`
- kept SP7q's GPU reduction kernel for final `out`

Focused HAL browser test:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_core_gpu_results_match_cpu -- --nocapture
```

Result:

```text
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 1.03s
```

The focused diagnostics contain neither `batch_evaluate_partials` readbacks nor
`webgpu_batch_evaluate_any_partial_params` uploads.

## Representative E2E

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

Result:

```text
test tests::rv32im_accum_topaccum_arm5_authoritative_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 115.50s
```

BusyLoop:

```text
browser-prove:metric prove_session_async wall_ms=10707.0 gpu_active_ms=7487.0 gpu_idle_ratio=0.301
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
```

KeccakUnion(1):

```text
browser-prove:metric prove_session_async wall_ms=104558.0 gpu_active_ms=70684.0 gpu_idle_ratio=0.324
browser-prove:done multi_test/keccak_union_topaccum_arm5_authoritative: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_authoritative: gpu_dispatches=5904 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0 uploads=4085 upload_bytes=7166152108 device_copies=38 device_copy_bytes=38400 readbacks=1218 readback_bytes=10183944
browser-prove:webgpu-upload multi_test/keccak_union_topaccum_arm5_authoritative: source=webgpu_batch_evaluate_any_partial_2d_params uploads=110 upload_bytes=5280
browser-prove:webgpu-upload multi_test/keccak_union_topaccum_arm5_authoritative: source=webgpu_batch_evaluate_any_reduce_params uploads=110 upload_bytes=3520
browser-prove:webgpu-readback multi_test/keccak_union_topaccum_arm5_authoritative: source=evaluated readbacks=257 readback_bytes=4509000
browser-prove:webgpu-readback multi_test/keccak_union_topaccum_arm5_authoritative: source=final_coeffs readbacks=38 readback_bytes=38400
browser-prove:webgpu-readback multi_test/keccak_union_topaccum_arm5_authoritative: source=nodes readbacks=771 readback_bytes=4796192
browser-prove:webgpu-readback multi_test/keccak_union_topaccum_arm5_authoritative: source=out readbacks=152 readback_bytes=840352
```

No `batch_evaluate_partials` readback source and no
`webgpu_batch_evaluate_any_partial_params` upload source remain in the
KeccakUnion summary.

## xgboost E2E

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies -- --nocapture
```

Result:

```text
test tests::xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 102.85s
```

Key metrics:

```text
browser-prove:metric prove_session_async wall_ms=102627.0 gpu_active_ms=61829.0 gpu_idle_ratio=0.398
browser-prove:done xgboost_topaccum_arm5_authoritative: segments=11 user_cycles=2294890 total_cycles=2883584
browser-prove:webgpu xgboost_topaccum_arm5_authoritative: gpu_dispatches=5258 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=3662 upload_bytes=8289600740 device_copies=32 device_copy_bytes=32768 readbacks=1056 readback_bytes=7488592
browser-prove:webgpu-upload xgboost_topaccum_arm5_authoritative: source=webgpu_batch_evaluate_any_partial_2d_params uploads=85 upload_bytes=4080
browser-prove:webgpu-upload xgboost_topaccum_arm5_authoritative: source=webgpu_batch_evaluate_any_reduce_params uploads=85 upload_bytes=2720
browser-prove:webgpu-readback xgboost_topaccum_arm5_authoritative: source=evaluated readbacks=224 readback_bytes=2708800
browser-prove:webgpu-readback xgboost_topaccum_arm5_authoritative: source=final_coeffs readbacks=32 readback_bytes=32768
browser-prove:webgpu-readback xgboost_topaccum_arm5_authoritative: source=nodes readbacks=672 readback_bytes=4383744
browser-prove:webgpu-readback xgboost_topaccum_arm5_authoritative: source=out readbacks=128 readback_bytes=363280
```

Comparison against SP7q xgboost:

```text
wall_ms:       103879 -> 102627  (-1252 ms, noisy single trial)
upload_count:  4919 -> 3662      (-1257 uploads)
upload_bytes:  8289639604 -> 8289600740 (-38864 bytes)
readback_bytes: 7488592 -> 7488592 (unchanged)
old partial params: webgpu_batch_evaluate_any_partial_params 1342 uploads / 42944 bytes -> 0
new 2D partial params: webgpu_batch_evaluate_any_partial_2d_params 85 uploads / 4080 bytes
```

Note: the current `gpu_dispatches` diagnostic is operation-level, not raw
WebGPU command-level, so it does not show the internal partial-dispatch
collapse. The upload-source replacement is the concrete e2e evidence that the
per-eval path is no longer used.

## Hygiene

```text
git diff --check
# pass
```

## Decision

Accepted as a production command-submission and parameter-upload reduction
because focused HAL coverage, BusyLoop + `KeccakUnion(1)`, and xgboost all
verify receipts with zero CPU fallbacks and zero CPU-only ops.

Observed xgboost wall movement: `-1.252 s` in one browser trial. Treat this as
directional/noisy until repeated A/B runs confirm it.
