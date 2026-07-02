Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7q -- chunked batch_evaluate_any partial reduction
Date: 2026-05-18

## Headline

`batch_evaluate_any_chunked_async` no longer reads every chunk partial back to
WASM for CPU reduction. Partial evaluation still runs on WebGPU, and a new small
WebGPU reduction kernel now writes the final `out` buffer directly. This removes
the `batch_evaluate_partials` readback source from recursion-heavy proofs.

This is a real data-movement reduction. The single-trial xgboost wall-time
movement is only about `0.204 s`, so treat wall-time impact as small/noisy.

## RED

Added focused browser HAL coverage in `webgpu_hal_core_gpu_results_match_cpu`:

- force the chunked path with degree `4096`;
- compare chunked output against the existing `batch_evaluate_any` result;
- assert no readback source named `batch_evaluate_partials`.

Initial compile failed because the hook did not exist:

```text
error[E0599]: no method named `debug_batch_evaluate_any_chunked` found for struct `WebGpuHal`
```

An intermediate RED also caught an invalid test fixture: degree `2300` failed
the existing power-of-two degree assertion. The fixture was corrected to
degree `4096`.

## GREEN

Implemented:

- `BATCH_EVALUATE_ANY_PARTIAL_REDUCE_WGSL`
- `WebGpuHal::debug_batch_evaluate_any_chunked`
- GPU-side reduction in `batch_evaluate_any_chunked_async`
- chunked dispatch preconditions now require an output GPU buffer and storage
  binding fit.

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
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 0.20s
```

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
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 118.10s
```

BusyLoop:

```text
browser-prove:metric prove_session_async wall_ms=10856.0 gpu_active_ms=7654.0 gpu_idle_ratio=0.295
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
```

KeccakUnion(1):

```text
browser-prove:metric prove_session_async wall_ms=107016.0 gpu_active_ms=73033.0 gpu_idle_ratio=0.318
browser-prove:done multi_test/keccak_union_topaccum_arm5_authoritative: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_authoritative: gpu_dispatches=5904 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0 readbacks=1218 readback_bytes=10183944
browser-prove:webgpu-readback multi_test/keccak_union_topaccum_arm5_authoritative: source=evaluated readbacks=257 readback_bytes=4509000
browser-prove:webgpu-readback multi_test/keccak_union_topaccum_arm5_authoritative: source=final_coeffs readbacks=38 readback_bytes=38400
browser-prove:webgpu-readback multi_test/keccak_union_topaccum_arm5_authoritative: source=nodes readbacks=771 readback_bytes=4796192
browser-prove:webgpu-readback multi_test/keccak_union_topaccum_arm5_authoritative: source=out readbacks=152 readback_bytes=840352
```

No `batch_evaluate_partials` readback source remains in the KeccakUnion summary.

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
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 104.11s
```

Key metrics:

```text
browser-prove:metric prove_session_async wall_ms=103879.0 gpu_active_ms=63194.0 gpu_idle_ratio=0.392
browser-prove:done xgboost_topaccum_arm5_authoritative: segments=11 user_cycles=2294890 total_cycles=2883584
browser-prove:webgpu xgboost_topaccum_arm5_authoritative: gpu_dispatches=5258 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4919 upload_bytes=8289639604 device_copies=32 device_copy_bytes=32768 readbacks=1056 readback_bytes=7488592
```

Readback sources:

```text
browser-prove:webgpu-readback xgboost_topaccum_arm5_authoritative: source=evaluated readbacks=224 readback_bytes=2708800
browser-prove:webgpu-readback xgboost_topaccum_arm5_authoritative: source=final_coeffs readbacks=32 readback_bytes=32768
browser-prove:webgpu-readback xgboost_topaccum_arm5_authoritative: source=nodes readbacks=672 readback_bytes=4383744
browser-prove:webgpu-readback xgboost_topaccum_arm5_authoritative: source=out readbacks=128 readback_bytes=363280
```

Comparison against SP7p xgboost:

```text
wall_ms:       104083 -> 103879  (-204 ms, noisy)
readback_bytes: 12963952 -> 7488592 (-5475360 bytes)
batch_evaluate_partials readback: 5496832 -> 0 bytes
```

Upload overhead from the new reduction params is negligible:

```text
webgpu_batch_evaluate_any_reduce_params uploads=85 upload_bytes=2720
```

## Hygiene

```text
git diff --check
# pass
```

## Decision

Accepted as a production data-movement reduction because focused HAL coverage,
BusyLoop + `KeccakUnion(1)`, and xgboost all verify receipts with zero CPU
fallbacks and zero CPU-only ops.

Accepted wall-time reduction: `0.204 s` observed on one xgboost trial, noisy.
Accepted readback reduction: `5.475 MB` on xgboost.
