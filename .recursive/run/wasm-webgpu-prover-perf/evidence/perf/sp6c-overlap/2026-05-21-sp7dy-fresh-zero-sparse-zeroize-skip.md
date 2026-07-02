# SP7dy - Fresh-Zero Sparse Zeroize Skip

Date: 2026-05-21

## Candidate

Skip the full-buffer `eltwise_zeroize_elem` GPU pass after a sparse zeroize
upload when the destination WebGPU buffer is known to be fresh browser-zeroed
memory.

The sparse upload already writes all valid nonzero cells. For a fresh
invalid-initialized buffer, cells skipped by the sparse upload are either
logical zero or logical invalid, and the GPU backing is already zero, so the
full invalid-to-zero scan is redundant. The implementation tracks
`gpu_known_zero` conservatively per shared buffer allocation and clears it on
any normal GPU sync/write path.

## RED / GREEN Focused Gate

Focused test:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_core_gpu_results_match_cpu -- --nocapture
```

RED log: `/tmp/sp7dy-zeroize-fresh-invalid-red.log`

- High WebGPU limits negotiated.
- Sparse upload metric moved for fresh invalid `data`.
- Assertion expected one raw dispatch but observed two:
  sparse upload plus redundant full zeroize kernel.
- Zero `cpu_fallbacks`; zero `cpu_only_ops`.

GREEN log: `/tmp/sp7dy-zeroize-fresh-invalid-green.log`

- Same focused GPU/CPU parity test passed.
- The fresh-invalid sparse zeroize case now completes with one raw dispatch.
- Zero `cpu_fallbacks`; zero `cpu_only_ops`.

## Representative e2e: BusyLoop + KeccakUnion

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture
```

Log: `/tmp/sp7dy-zeroize-skip-default-busy-keccak-rerun.log`

Result: PASS, high WebGPU limits, verified BusyLoop and KeccakUnion receipts,
zero `cpu_fallbacks`, zero `cpu_only_ops`, total test time `98.15s`.

Key lines:

```text
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
multi_test/busy_loop_po2_18_default_representative: wall_ms=6170.0 gpu_active_ms=3884.0
multi_test/busy_loop_po2_18_default_representative: raw_compute_dispatches=603 queue_submits=165 cpu_fallbacks=0 cpu_only_ops=0
multi_test/keccak_union_default_representative: wall_ms=91256.0 gpu_active_ms=60029.0
multi_test/keccak_union_default_representative: raw_compute_dispatches=10671 queue_submits=3078 cpu_fallbacks=0 cpu_only_ops=0
test result: ok. 1 passed; 0 failed; 0 ignored; 151 filtered out; finished in 98.15s
```

Comparison:

```text
SP7dt accepted BusyLoop+KeccakUnion: 99.31s
SP7dy BusyLoop+KeccakUnion:         98.15s
Movement:                          -1.16s / -1.2%
```

## Representative e2e: xgboost

The first xgboost attempt was run from the workspace root without
`--manifest-path examples/browser-prove/Cargo.toml` and failed during wasm
compilation of unrelated workspace crates (`mio`, `socket2`, `liblzma-sys`).
It did not enter proof generation and is excluded.

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture
```

Log: `/tmp/sp7dy-zeroize-skip-xgboost-targeted.log`

Result: PASS, high WebGPU limits, verified journal
`30.528042544062632`, zero `cpu_fallbacks`, zero `cpu_only_ops`, total test
time `72.56s`.

Key lines:

```text
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
prove_session_async wall_ms=72023.0 gpu_active_ms=45252.0 gpu_idle_ratio=0.372
xgboost: segments=11 user_cycles=2294890 total_cycles=2883584
xgboost: gpu_dispatches=5259 raw_compute_dispatches=9718 queue_submits=2740 cpu_fallbacks=0 cpu_only_ops=0 uploads=3292 upload_bytes=2766391756 readbacks=352 readback_bytes=7488592
webgpu_zeroize_sparse_ranges uploads=53 upload_bytes=638954592
webgpu_zeroize_sparse_values uploads=53 upload_bytes=1623831224
test result: ok. 1 passed; 0 failed; 0 ignored; 151 filtered out; finished in 72.56s
```

Comparison:

```text
SP7dt accepted xgboost:              73.39s
SP7dx same-tree fresh default:       72.77s
SP7dy xgboost:                       72.56s
Movement vs SP7dt:                   -0.83s / -1.1%
Movement vs same-tree fresh default: -0.21s / -0.3%
```

## Decision

Accept as a small correctness-clean default-path reduction.

This is not a major wall-time lever by itself. It deterministically removes
redundant raw GPU work in the focused case and shifts representative e2e walls
in the right direction, but much of the xgboost movement is inside normal
browser-run variance. The useful outcome is that the sparse-zeroize path is now
less wasteful while preserving proof correctness under the full receipt gates.

Accepted wall-time gain estimate:

- BusyLoop+KeccakUnion: about `1.2%` in this run.
- xgboost: about `0.3%` versus same-tree fresh default, `1.1%` versus SP7dt.

Remaining immediate performance headroom is still dominated by larger structural
work: chunk-complete GPU witness generation / TopAccum replacement and reducing
the remaining sparse upload volume. Micro-skips like this are expected to yield
sub-1% to low-1% gains each.
