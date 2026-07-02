# SP7ea - Recursion Poseidon2 accumulator fast path rejected

Date: 2026-05-21
Status: rejected and reverted

## Candidate

The next bounded recursion candidate was a CPU-side fast path for the
generated recursion accumulator rows whose selector is exclusively
`poseidon2_full` or `poseidon2_partial`.

Generated `step_compute_accum` writes accumulator factor `1` for those rows,
and generated `step_verify_accum` only copies the prefix product into the first
four accumulator columns. The candidate skipped the generated compute/verify
functions for those rows while preserving the generated baseline for all other
selectors.

This was intentionally tested as a cheap bounded candidate before attempting a
full generated WebGPU recursion accumulator/witgen port.

## RED

Added focused browser test:

```rust
#[wasm_bindgen_test(async)]
async fn recursion_accum_poseidon2_fastpath_probe()
```

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  recursion_accum_poseidon2_fastpath_probe --no-run
```

Expected failure:

```text
error[E0425]: cannot find function `debug_recursion_accum_poseidon2_fastpath_probe`
```

## GREEN

Implementation added:

- focused synthetic parity probe comparing generated baseline output with the
  fast path on six Poseidon2 full/partial accumulator rows;
- conservative selector guard requiring all selector values valid and exactly
  one of `poseidon2_full` / `poseidon2_partial` active;
- fast-path counter used by the focused browser assertion.

Focused compile passed after adding the needed trait imports:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  recursion_accum_poseidon2_fastpath_probe --no-run
```

Result:

- passed;
- elapsed: `3m04s`.

Focused Chrome test:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=180 \
  cargo test --manifest-path Cargo.toml \
    --target wasm32-unknown-unknown --release \
    recursion_accum_poseidon2_fastpath_probe -- --nocapture
```

Result:

- high WebGPU limits: `4294967292 / 2147483644 / 49152`;
- baseline synthetic compute/verify took `4ms` / `2ms`;
- fast-path synthetic compute/verify took `0ms` / `0ms`;
- `test tests::recursion_accum_poseidon2_fastpath_probe ... ok`;
- `test result: ok. 1 passed; 0 failed; 152 filtered out; finished in 0.09s`.

## Representative BusyLoop + KeccakUnion

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path Cargo.toml \
    --target wasm32-unknown-unknown --release \
    rv32im_default_representative_e2e_verify -- --nocapture
```

Result:

- high WebGPU limits: `4294967292 / 2147483644 / 49152`;
- BusyLoop receipt verified;
- KeccakUnion receipt verified;
- `cpu_fallbacks=0`;
- `cpu_only_ops=0`;
- `test tests::rv32im_default_representative_e2e_verify ... ok`;
- `test result: ok. 1 passed; 0 failed; 152 filtered out; finished in 97.97s`.

Performance versus latest accepted SP7dy:

- SP7dy BusyLoop+KeccakUnion: `98.15s`;
- SP7ea candidate: `97.97s`;
- delta: `-0.18s`, noise-level.

## Representative xgboost

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path Cargo.toml \
    --target wasm32-unknown-unknown --release \
    xgboost_succinct_receipt_verifies -- --nocapture
```

Result:

- high WebGPU limits: `4294967292 / 2147483644 / 49152`;
- xgboost receipt verified;
- journal verified as `30.528042544062632`;
- `cpu_fallbacks=0`;
- `cpu_only_ops=0`;
- `test tests::xgboost_succinct_receipt_verifies ... ok`;
- `test result: ok. 1 passed; 0 failed; 152 filtered out; finished in 73.23s`.

Performance versus latest accepted SP7dy:

- SP7dy xgboost: `72.56s`;
- SP7ea candidate: `73.23s`;
- delta: `+0.67s`.

## Decision

Rejected and reverted.

Correctness was clean across focused parity, BusyLoop, KeccakUnion, and
xgboost. The focused synthetic probe showed the local mechanism worked, and
BusyLoop+KeccakUnion moved by only `-0.18s`, but xgboost regressed by `+0.67s`
against the latest accepted state. This fails the immediate significant
wall-time improvement bar.

Post-revert marker sweep is clean for:

- `poseidon2_fastpath`
- `debug_recursion_accum_poseidon2`
- `recursion_accum_poseidon2`
- `RECURSION_SEL_POSEIDON2`
- `RECURSION_ACCUM_POSEIDON2`

Accepted wall-time gain: `0`.

Do not continue CPU-side trivial-selector recursion accumulator shortcuts as an
immediate performance lever. The remaining credible recursion path is a
chunk-complete generated GPU witgen/accum backend, not local generated-Rust
shortcuts that only shave subsecond CPU buckets.
