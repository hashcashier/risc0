# SP7ec - MISC2 GPU-Witgen + Direct Accum Rejected

Date: 2026-05-21

## Candidate

Recheck MISC2 GPU-witgen replacement after MISC2 direct accumulation became
default.

Earlier MISC2 replacement attempts were correctness-positive but wall-negative
because they had to repair CPU data/accum shadow rows. This candidate paired
MISC2 replacement with direct MISC2 accumulation and skipped the MISC2 shadow
repair paths while the candidate flag was active.

## RED / GREEN Compile Gate

RED command:

```text
env CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_misc2_witgen_direct_accum_candidate_e2e_verify --no-run
```

Result: expected RED on missing API:

```text
unresolved import `risc0_circuit_rv32im::prove::set_witgen_gpu_misc2_replace_candidate_enabled`
```

GREEN compile command after adding the opt-in candidate API:

```text
env CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_misc2_witgen_direct_accum_candidate_e2e_verify --no-run
```

Result: PASS, release wasm test target compiled in `4m25s`.

## Readiness Repair

The first BusyLoop+KeccakUnion e2e attempt proved the candidate wiring was
incomplete, not performance-ready:

```text
iter6d_g_pre_witgen_dispatch_async mask=0x0000 no_ready_replacement no_sync
panicked: MISC2 candidate should include MISC2 replacement once kernels are ready
```

Root cause: default nonblocking replacement prewarm did not start the six
MISC2 extra minor kernels, so arm 2 could not become ready. The candidate was
temporarily repaired by adding MISC2 extra kernels to the pending/prewarm path.
The repaired compile gate passed in `4m23s`.

## Representative e2e: BusyLoop + KeccakUnion

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_misc2_witgen_direct_accum_candidate_e2e_verify -- --nocapture
```

Result: PASS with high WebGPU limits, verified BusyLoop and KeccakUnion
receipts, zero CPU fallback/CPU-only assertions, no MISC2 data shadow repair,
and MISC2 direct accumulation active.

Key lines:

```text
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
iter6d_g_prewarm misc2_extra requested=6
iter6d_g_pre_witgen_dispatch_async mask=0x0025 dispatched_arms=[0, 2, 5]
test result: ok. 1 passed; 0 failed; 0 ignored; 153 filtered out; finished in 99.52s
```

Comparison:

```text
SP7dy accepted BusyLoop+KeccakUnion: 98.15s
SP7ec candidate:                    99.52s
Movement:                           +1.37s / +1.4%
```

## Representative e2e: xgboost

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_misc2_witgen_direct_accum_candidate_xgboost_e2e_verify -- --nocapture
```

Result: PASS with high WebGPU limits, verified journal
`30.528042544062632`, zero CPU fallback/CPU-only assertions, no MISC2 data
shadow repair, and MISC2 replacement active after prewarm.

Key lines:

```text
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
prove_session_async segments=11 pending_keccaks=0 assumptions=0
iter6d_g_prewarm misc2_extra requested=6
iter6d_g_pre_witgen_dispatch_async mask=0x0025 dispatched_arms=[0, 2, 5]
test result: ok. 1 passed; 0 failed; 0 ignored; 153 filtered out; finished in 73.34s
```

Comparison:

```text
SP7dy accepted xgboost: 72.56s
SP7ec candidate:       73.34s
Movement:              +0.78s / +1.1%
```

## Post-Revert Verification

The candidate API, tests, MISC2 direct-shadow elision, and MISC2 extra-kernel
prewarm additions were removed.

Marker sweep:

```text
rg -n "misc2_witgen_direct_accum|set_witgen_gpu_misc2_replace_candidate_enabled|WITGEN_GPU_MISC2_REPLACE|witgen_misc2_candidate_enabled|WITGEN_MISC2_EXTRA_PENDING" examples/browser-prove/src/lib.rs risc0/circuit/rv32im/src/prove/hal/webgpu.rs risc0/circuit/rv32im/src/prove/mod.rs
```

Result: clean.

Post-revert compile gate:

```text
env CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies --no-run
```

Result: PASS, release wasm target compiled in `4m27s`.

`git diff --check -- examples/browser-prove/src/lib.rs risc0/circuit/rv32im/src/prove/hal/webgpu.rs risc0/circuit/rv32im/src/prove/mod.rs` also passed.

## Decision

Reject and revert.

The candidate proved correctness-clean, and the direct-accum pairing did remove
the broad MISC2 shadow-repair dependency in the tested path. It still regressed
wall time on both representative gates because the extra MISC2 replacement
kernels and per-segment dispatch work cost more than the CPU witness work they
replace.

Accepted wall-time gain: `0`.

Do not retry this exact opportunistic MISC2 replacement pairing as an immediate
lever. MISC2 may still belong in a chunk-complete generated GPU-witgen backend,
but not as another bolt-on replacement arm under the current prewarm/dispatch
shape.
