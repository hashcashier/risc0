Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7p -- xgboost candidate-sync coverage
Date: 2026-05-18

## Headline

The representative e2e matrix now covers the candidate-sync timing gate on all
current workload categories:

- BusyLoop po2=18: single RV32IM segment.
- `KeccakUnion(1)`: RV32IM plus keccak/assumption-heavy succinct proving.
- xgboost: canonical deferred multi-segment workload.

This is validation coverage, not a production speedup. Default runtime behavior
is unchanged unless tests explicitly enable the sync gate.

## Change

Updated `xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies` to:

- enable `set_accum_gpu_candidate_sync_enabled(true)`;
- disable it after proof generation;
- assert `accum_gpu_candidate_sync_waits() > 0`.

The test already enabled authoritative TopAccum arm5 and the major histogram, so
this keeps xgboost as the canonical multi-segment TopAccum validation fixture.

## Browser E2E

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
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 104.31s
```

Key proof metrics:

```text
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
browser-prove:stage done rv32im_accumulate candidate_sync_wait elapsed_ms=3311.000 gpu_active=true
browser-prove:metric prove_session_async wall_ms=104083.0 gpu_active_ms=63566.0 gpu_idle_ratio=0.389
browser-prove:done xgboost_topaccum_arm5_authoritative: segments=11 user_cycles=2294869 total_cycles=2883584
browser-prove:webgpu xgboost_topaccum_arm5_authoritative: gpu_dispatches=5258 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4834 upload_bytes=8289636860 device_copies=32 device_copy_bytes=32768 readbacks=1056 readback_bytes=12963952
```

Upload/readback attribution from the same run:

```text
browser-prove:webgpu-upload xgboost_topaccum_arm5_authoritative: source=data uploads=322 upload_bytes=5252317184
browser-prove:webgpu-upload xgboost_topaccum_arm5_authoritative: source=accum uploads=119 upload_bytes=1716518912
browser-prove:webgpu-upload xgboost_topaccum_arm5_authoritative: source=combos uploads=64 upload_bytes=759169024
browser-prove:webgpu-upload xgboost_topaccum_arm5_authoritative: source=ctrl uploads=21 upload_bytes=506462208
browser-prove:webgpu-readback xgboost_topaccum_arm5_authoritative: source=batch_evaluate_partials readbacks=85 readback_bytes=5496832
browser-prove:webgpu-readback xgboost_topaccum_arm5_authoritative: source=nodes readbacks=672 readback_bytes=4383744
browser-prove:webgpu-readback xgboost_topaccum_arm5_authoritative: source=evaluated readbacks=224 readback_bytes=2708800
```

## Decision

Accepted as representative e2e coverage. Any future TopAccum candidate must pass
the BusyLoop + `KeccakUnion(1)` receipt gate before xgboost, and any accepted
wall-time claim must include xgboost proof verification with zero CPU fallback.

Accepted wall-time reduction: `0 s`.
