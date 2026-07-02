# SP7dp MEM0 Direct Accumulator Candidate

Date: 2026-05-21

## Scope

Follow-up to SP7do. Production MEM0 GPU-witgen replacement was correctness-clean but wall-negative because it needed broad `witgen_accum_shadow_rows` repair before accumulation. This candidate keeps production default MISC0-only and adds an opt-in MEM0 replacement path that is enabled only when both:

- `set_witgen_gpu_mem0_replace_candidate_enabled(true)`
- `set_accum_gpu_mem0_direct_enabled(true)`

are set. The direct MEM0 accumulator writes TopAccum columns for GPU-owned MEM0 load rows, so CPU `step_TopAccum` does not need a broad MEM0 witness row readback.

## Changed Surface

- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
  - Added opt-in MEM0 replacement arm mask expansion.
  - Added `rv32im_accum_mem0_direct` WGSL generation and dispatch through the existing grouped direct-accumulator sequence.
  - Excluded direct-MEM0 rows from `sync_witgen_replace_accum_shadow_rows`.
- `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs`
  - Added MEM0 load minors `0..=4` to short-circuit side-effect coverage when arm bit 5 is set.
  - Added a separate `skip_replaced_mem0` accumulator skip so only selected MEM0 load rows are skipped.
- `risc0/circuit/rv32im/src/prove/mod.rs`
  - Re-exported the MEM0 candidate controls and row counter.
- `examples/browser-prove/src/lib.rs`
  - Added BusyLoop+KeccakUnion candidate e2e proof gate.
  - Added xgboost candidate e2e proof gate.

## RED Evidence

Command:

```bash
cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_mem0_direct_accum_candidate_e2e_verify --no-run
```

Workdir: `examples/browser-prove`

Result: RED failed as expected before production code existed, with unresolved imports:

- `accum_gpu_mem0_direct_rows`
- `set_accum_gpu_mem0_direct_enabled`
- `set_witgen_gpu_mem0_replace_candidate_enabled`

This proved the new browser e2e gate was not satisfied by the accepted SP7do post-revert state.

## Compile Evidence

Command:

```bash
cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_mem0_direct_accum_candidate_e2e_verify --no-run
```

Workdir: `examples/browser-prove`

Result: PASS. Finished release wasm test compile in `4m28s`.

Command:

```bash
cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_mem0_direct_accum_candidate_xgboost_e2e_verify --no-run
```

Workdir: `examples/browser-prove`

Result: PASS. Finished release wasm test compile in `2m19s`.

Formatting/whitespace:

- `cargo fmt --check --manifest-path risc0/circuit/rv32im/Cargo.toml`: PASS
- `cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml`: PASS
- `git diff --check`: PASS

## Default Production Sanity

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

Result: PASS. Test result: `1 passed`, finished in `78.33s`.

Key proof evidence:

- High WebGPU limits: `max_buffer_size=4294967292`, `max_storage_buffer_binding_size=2147483644`, `max_compute_workgroup_storage_size=49152`.
- Journal verified: `30.528042544062632`.
- Default replacement mask remained `0x0001`, dispatched arms `[0]`; no MEM0 prewarm or MEM0 direct accumulator dispatch.
- Production-path accumulator log used `skip_replaced_misc0=true_mem0=false_major_mask=0x0006`.

Conclusion: the MEM0 candidate is opt-in and did not regress the latest default xgboost path. Latest default xgboost sample after this patch is `78.33s`.

## BusyLoop + KeccakUnion E2E

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
    iter6d_g_mem0_direct_accum_candidate_e2e_verify -- --nocapture
```

Result: PASS. Test result: `1 passed`, finished in `104.93s`.

Key proof evidence:

- High WebGPU limits: `max_buffer_size=4294967292`, `max_storage_buffer_binding_size=2147483644`, `max_compute_workgroup_storage_size=49152`.
- BusyLoop generated and verified a succinct receipt.
- BusyLoop replacement mask: `0x0021`, dispatched arms `[0, 5]`.
- BusyLoop MEM0 replacement minor dispatches included minor 2 (`40795` rows) and minor 3 (`171` rows).
- BusyLoop direct accumulator rows included `MEM0 direct_gpu dispatched rows=40968`.
- KeccakUnion representative shape guard passed and the proof verified.
- The test asserted the MEM0 direct row counter increased for both BusyLoop and KeccakUnion.
- The test asserted bounded `witgen_accum_shadow_rows` readback bytes for both fixtures.

Wall decision versus accepted SP7do post-revert BusyLoop+KeccakUnion (`99.56s`):

- Candidate: `104.93s`
- Delta: `+5.37s` (`+5.4%`)
- Decision: do not default-enable MEM0 candidate on this evidence.

## XGBoost E2E

First attempt result: FAIL before proof generation due a non-representative Chrome adapter profile:

- `max_buffer_size=1073741824`
- `max_storage_buffer_binding_size=1073741824`
- `max_compute_workgroup_storage_size=32768`

This failed the high-limit representative gate and is not counted as correctness or performance evidence.

Rerun command:

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
    iter6d_g_mem0_direct_accum_candidate_xgboost_e2e_verify -- --nocapture
```

Result: PASS. Test result: `1 passed`, finished in `76.62s`.

Key proof evidence:

- High WebGPU limits: `max_buffer_size=4294967292`, `max_storage_buffer_binding_size=2147483644`, `max_compute_workgroup_storage_size=49152`.
- xgboost session shape: `segments=11`, no pending keccaks, no assumptions.
- Journal verified: `30.528042544062632`.
- Segment 0 replacement mask: `0x0021`, dispatched arms `[0, 5]`.
- Segment 0 MEM0 replacement minor dispatches included minor 2 (`40301` rows) and minor 3 (`2` rows).
- Segment 0 direct accumulator rows included `MEM0 direct_gpu dispatched rows=40303`.
- Segment 1 direct accumulator rows included `MEM0 direct_gpu dispatched rows=41337`.
- The test asserted the MEM0 direct row counter increased.
- The test asserted `witgen_accum_shadow_rows` readback bytes stayed under `1_000_000`.

Wall decision versus accepted SP7do post-revert xgboost (`78.62s`):

- Candidate: `76.62s`
- Delta: `-2.00s` (`-2.5%`)
- Decision: xgboost-positive, but not enough to default-enable because BusyLoop+KeccakUnion regressed.

## Decision

MEM0 direct accumulation is correctness-viable and removes the SP7do blocker that forced broad MEM0 accum-shadow repair. It is not accepted as a default production path yet because representative wall evidence is mixed:

- xgboost improved by about `2.0s`.
- BusyLoop+KeccakUnion regressed by about `5.4s`.

Keep the opt-in candidate tests and implementation surface. Do not call it an accepted wall-time gain until either:

1. a profitability/shape gate limits MEM0 replacement to workloads where it wins, or
2. follow-up work reduces the MEM0 prewarm/dispatch overhead enough for BusyLoop+KeccakUnion to pass the wall gate too.

Accepted production wall-time gain: `0`.
