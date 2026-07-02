# SP7cz - Recursion accumulator selector histogram

Date: 2026-05-21

## Purpose

After SP7cy made GPU witgen/direct-accum the default browser path, the next
candidate was recursion accumulation. The full generated recursion accumulator
kernels are too large to wire as one browser WGSL module, so this diagnostic
measured which selector arms actually dominate xgboost recursion accumulation.

The instrumentation was diagnostic-only: it logged selector counts from
`rust_kernels::accumulate(...)` and was removed after the run.

## Compile

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_succinct_receipt_verifies --no-run
```

Result:

- Passed.
- Elapsed: `3m01s`.

## xgboost e2e diagnostic

Command output: `/tmp/sp7cz-recursion-selector-xgboost.log`

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
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
    --target wasm32-unknown-unknown --release \
    xgboost_succinct_receipt_verifies -- --nocapture
```

Result:

- `test tests::xgboost_succinct_receipt_verifies ... ok`
- `test result: ok. 1 passed; 0 failed; 137 filtered out; finished in 78.09s`
- high WebGPU limits: `4294967292 / 2147483644 / 49152`
- `wall_ms=77848`
- `segments=11`
- `gpu_active_ms=44823`
- `gpu_idle_ratio=0.424`
- `raw_compute_dispatches=9674`
- `queue_submits=2718`
- `upload_bytes=3113951756`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

## Timing Split

Across 21 recursion accumulation calls in the xgboost proof:

| Bucket | total_ms | mean_ms |
|---|---:|---:|
| `recursion_witgen` | 5153 | 245.4 |
| `recursion_accumulate` total | 6066 | 288.9 |
| `compute_accum` | 3679 | 175.2 |
| `verify_accum` | 2283 | 108.7 |
| `prefix_products` | 61 | 2.9 |

## Selector Histogram

Aggregate selected cycles: `4,196,290`.

| Selector | cycles | share |
|---|---:|---:|
| `micro_ops` | 2,601,831 | 62.00% |
| `poseidon2_full` | 731,504 | 17.43% |
| `poseidon2_load` | 367,034 | 8.75% |
| `poseidon2_partial` | 182,876 | 4.36% |
| `macro_ops` | 166,614 | 3.97% |
| `poseidon2_store` | 146,431 | 3.49% |
| `checked_bytes` | 0 | 0.00% |

Macro-op breakdown:

| Macro op | cycles | share of all work |
|---|---:|---:|
| `bit_and_elem` | 149,280 | 3.56% |
| `sha_mix` | 11,856 | 0.28% |
| `sha_load` | 3,952 | 0.09% |
| `sha_fini` | 988 | 0.02% |
| `sha_init` | 412 | 0.01% |
| `set_global` | 84 | 0.00% |
| `wom_init` | 21 | 0.00% |
| `wom_fini` | 21 | 0.00% |
| `nop` / `bit_op_shorts` | 0 | 0.00% |

## Generated Branch Shape

Generated Rust accumulator branch sizes:

| Function | Selector | lines | `inv()` calls | accum writes | accum reads |
|---|---|---:|---:|---:|---:|
| `step_compute_accum` | `micro_ops` | 2923 | 3 | 1 | 0 |
| `step_compute_accum` | `macro_ops` | 6080 | 10 | 7 | 0 |
| `step_compute_accum` | `poseidon2_load` | 2923 | 3 | 1 | 0 |
| `step_compute_accum` | `poseidon2_full` | 5 | 0 | 1 | 0 |
| `step_compute_accum` | `poseidon2_partial` | 5 | 0 | 1 | 0 |
| `step_compute_accum` | `poseidon2_store` | 2923 | 3 | 1 | 0 |
| `step_compute_accum` | `checked_bytes` | 664 | 1 | 1 | 0 |
| `step_verify_accum` | `micro_ops` | 2029 | 2 | 0 | 1 |
| `step_verify_accum` | `macro_ops` | 1701 | 3 | 0 | 7 |
| `step_verify_accum` | `poseidon2_load` | 2029 | 2 | 0 | 1 |
| `step_verify_accum` | `poseidon2_full` | 13 | 0 | 0 | 1 |
| `step_verify_accum` | `poseidon2_partial` | 13 | 0 | 0 | 1 |
| `step_verify_accum` | `poseidon2_store` | 2029 | 2 | 0 | 1 |
| `step_verify_accum` | `checked_bytes` | 16 | 0 | 0 | 1 |

## Decision

Diagnostic accepted and reverted.

Macro-only recursion accumulation offload is rejected as an immediate
performance target. Even perfect macro offload is capped by only `3.97%` of
recursion accumulation, or about `0.24 s` of the current xgboost proof.

The only recursion-accum chunks with material ceiling are `micro_ops` and the
Poseidon2 selectors. They are generated-code chunks of roughly 2k-3k lines each
for compute/verify, and still require preserving accumulator prefix semantics.
This remains possible, but it is not the fastest high-confidence wall-time
target compared with the already measured `rows=65536 cols=64` Merkle row-hash
bucket.

Current performance interpretation remains:

- focused accepted xgboost working-state estimate: about `77.5 s`;
- latest canonical default xgboost sample: `78.466 s`;
- this diagnostic run with temporary logging: `77.848 s`;
- accepted wall-time gain from SP7cz: `0 s`.
