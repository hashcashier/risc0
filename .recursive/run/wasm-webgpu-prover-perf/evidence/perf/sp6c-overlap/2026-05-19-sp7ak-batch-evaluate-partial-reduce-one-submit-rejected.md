# SP7ak: batch_evaluate_any partial+reduce one-submit candidate rejected

Date: 2026-05-19

## Candidate

For the 2D chunked `batch_evaluate_any` path, defer the partial dispatch and submit it with the reduce dispatch in one WebGPU command buffer using two compute passes. This keeps the storage-buffer visibility boundary while removing one queue submit for every chunked 2D call.

Expected counter impact:

- BusyLoop: 5 fewer queue submits.
- KeccakUnion: 110 fewer queue submits.
- xgboost: 85 fewer queue submits.

## Focused RED/GREEN

Focused RED added an assertion that `debug_batch_evaluate_any_chunked` should have exactly one queue submit before output readback. It failed as intended on the accepted SP7ah tree with:

- `queue_submits=2`
- `raw_compute_dispatches=2`
- `gpu_dispatches=1`

GREEN changed only the 2D chunked branch and left the oversized per-eval fallback path unchanged. The focused browser GPU test passed:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_core_gpu_results_match_cpu -- --nocapture
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
```

## Representative Proof Gate

All representative e2e proof-generation tests passed receipt verification with high WebGPU limits and zero CPU fallback/CPU-only ops.

| Workload | SP7ah Wall | Candidate Wall | SP7ah Queue Submits | Candidate Queue Submits | Raw Dispatches |
|---|---:|---:|---:|---:|---:|
| BusyLoop po2_18 | 10635 ms | 10624 ms | 174 | 169 | 710 |
| KeccakUnion(1) | 102226 ms | 103515 ms | 3169 | 3059 | 12621 |
| xgboost | 100335 ms | 100899 ms | 2765 | 2680 | 11331 |

KeccakUnion retained the representative shape:

- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`

Upload counts and bytes were unchanged, as expected; this candidate only removed queue submissions:

- KeccakUnion: `uploads=3748`, `upload_bytes=5910276364`
- xgboost: `uploads=3359`, `upload_bytes=7026039532`

## Decision

Rejected and reverted.

The submit-count reduction was exactly as predicted, but the two long representative wall signals moved the wrong direction:

- KeccakUnion: +1289 ms
- xgboost: +564 ms

Under the updated priority, counter-only improvements that do not reduce e2e proving wall time are not worth retaining. This path should not be retried unless paired with a larger batching change that removes more browser round trips or demonstrably improves wall time in repeated A/B runs.

Accepted wall-time reduction: 0 s.
