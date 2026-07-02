# SP7eb - Poseidon1 Direct Accumulator Rejected

Date: 2026-05-21

## Candidate

Test a narrow, hand-written direct accumulator for RV32IM major-10
Poseidon1 rows.

This was intentionally different from the already rejected generated arm10
TopAccum path: it did not generate the full TopAccum arm closure, and only
attempted to write the local lookup accumulator deltas needed for Poseidon1
rows before the existing GPU terminal-prefix and machine-column-carry passes.

## RED / GREEN Gate

RED compile gate:

```text
env CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_poseidon1_direct_accum_candidate_e2e_verify --no-run
```

Result: expected RED on missing candidate APIs:

```text
unresolved imports `accum_gpu_poseidon1_direct_rows`,
`set_accum_gpu_poseidon1_direct_enabled`
```

GREEN compile gate:

```text
env CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_poseidon1_direct_accum_candidate_e2e_verify --no-run
```

Result: PASS, release wasm test target compiled in `4m28s`.

## Representative e2e: BusyLoop + KeccakUnion

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_poseidon1_direct_accum_candidate_e2e_verify -- --nocapture
```

Result: PASS, high WebGPU limits, verified BusyLoop and KeccakUnion receipts,
zero `cpu_fallbacks`, zero `cpu_only_ops`, total test time `98.07s`.

Key lines:

```text
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
rv32im_accumulate step_top_accum_cpu_skip_replaced_misc0=true_mem0=true_major_mask=0x0446
rv32im_accumulate POSEIDON1 direct_gpu dispatched rows=19152
test result: ok. 1 passed; 0 failed; 0 ignored; 153 filtered out; finished in 98.07s
```

Comparison:

```text
SP7dy accepted BusyLoop+KeccakUnion: 98.15s
SP7eb candidate:                    98.07s
Movement:                           -0.08s / -0.1%
```

## Representative e2e: xgboost

The first xgboost attempt did not reach proof generation: Chrome selected a
low-limit adapter (`max_buffer_size=1073741824`,
`max_storage_buffer_binding_size=1073741824`,
`max_compute_workgroup_storage_size=32768`) and the representative gate
correctly rejected it. This run is excluded from performance evidence.

Rerun command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_poseidon1_direct_accum_candidate_xgboost_e2e_verify -- --nocapture
```

Result: PASS, high WebGPU limits, verified journal
`30.528042544062632`, zero `cpu_fallbacks`, zero `cpu_only_ops`, total test
time `72.47s`.

Key lines:

```text
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
rv32im_accumulate step_top_accum_cpu_skip_replaced_misc0=true_mem0=true_major_mask=0x0446
rv32im_accumulate POSEIDON1 direct_gpu dispatched rows=11394
test result: ok. 1 passed; 0 failed; 0 ignored; 153 filtered out; finished in 72.47s
```

Comparison:

```text
SP7dy accepted xgboost:       72.56s
Fresh same-tree profile:      72.68s
SP7eb candidate:              72.47s
Movement vs SP7dy:            -0.09s / -0.1%
Movement vs fresh profile:    -0.21s / -0.3%
```

## Decision

Reject and revert.

The candidate is correctness-clean, but the wall movement is noise-level on
both representative workload classes. It does not justify default enablement or
retaining additional direct-accumulator code while the user's priority is
immediate significant wall-time reduction.

Accepted wall-time gain: `0`.

Do not retry this exact Poseidon1 direct-accumulator path as an immediate
lever. The remaining meaningful headroom is still in chunk-complete GPU witgen
/ accumulation and in reducing the large sparse upload/zeroize traffic, not in
more one-major accumulator micro-slices.
