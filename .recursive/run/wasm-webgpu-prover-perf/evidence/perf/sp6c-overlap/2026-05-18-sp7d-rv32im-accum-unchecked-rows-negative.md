# SP7d RV32IM Accum Unchecked Rows - Negative Result

Date: 2026-05-18

## Hypothesis

RV32IM `step_TopAccum` still runs generated Rust/WASM over checked `BufferRow`
accessors before the existing WebGPU machine-column carry kernel. Disabling
row consistency/range checks only for the WebGPU authoritative accumulation
path might reduce xgboost wall time without changing proof semantics.

## Change Tested

Temporary local patch:

- Added `step_accum_without_machine_column_carry_unchecked_rows`.
- Kept normal/native `step_accum` and non-WebGPU paths checked.
- Routed only WebGPU authoritative accumulation through unchecked `data` and
  `accum` rows before the existing GPU carry.

The patch was reverted after measurement.

## Correctness Evidence

Focused browser proof:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=300 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  native_busy_loop_po2_18_async_succinct_receipt_verify -- --nocapture
```

Result:

- `test tests::native_busy_loop_po2_18_async_succinct_receipt_verify ... ok`
- `prove_session_async wall_ms=7821.0`
- `cpu_fallbacks=0`
- RV32IM `step_top_accum`: `1973 ms` for the focused po2_18 segment.

Full xgboost browser proof:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=600 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_pool_xgboost_smoke -- --nocapture
```

Result:

- `test tests::webgpu_pool_xgboost_smoke ... ok`
- `browser-prove:metric pool_xgboost_smoke wall_ms=101319`
- `gpu_dispatches=5269`
- `cpu_fallbacks=0`
- `uploads=4985`
- `upload_bytes=10909202180`
- `device_copies=32`
- `device_copy_bytes=32768`
- `buffers=6192`
- `buffer_bytes=55003027164`

## Decision

Do not keep.

Latest proof-backed baseline before this experiment was SP7c:
`pool_xgboost_smoke wall_ms=101254`. SP7d measured `101319 ms`, a 65 ms
regression/noise result rather than a reduction.

Because the patch removes useful hot-path consistency checks and provides no
wall-time gain, it was reverted. Do not retry unchecked `BufferRow` access as a
performance lever unless a new profile shows the checks are again material and
the e2e xgboost proof demonstrates a real wall-time reduction.

## Next Levers

- Split or otherwise lower the generated RV32IM `step_TopAccum`/`step_TopExtract`
  closure so the remaining ~20 s accumulation body can move to WebGPU.
- Reduce large repeated host-to-device uploads (`data`, `accum`, `combos`,
  `ctrl`) after correctness-preserving buffer lifetime and ownership analysis.
- Attack FRI/hash dispatch density and recursion generated accumulation only
  with full receipt verification after each semantic change.
