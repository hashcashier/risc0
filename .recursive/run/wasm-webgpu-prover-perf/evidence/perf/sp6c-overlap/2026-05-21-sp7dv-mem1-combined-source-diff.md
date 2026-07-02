# SP7dv MEM1 Combined Source-Register Diff Repair

Date: 2026-05-21

## Scope

Repair the MEM1 (`major=6`) GPU-witgen diagnostic after SP7du showed chunk-complete MEM1 kernels were still semantically wrong.

SP7du had rejected MEM1 because the high-limit focused browser diff reported 228 mismatches and nonzero CPU-only candidate cells. After wiring MEM1 chunks 2-7, the remaining mismatch set collapsed to 228 cells, all observed in column 30. Inspection showed `exec_mem1_chunk0_delta.wgsl` was calling the chunk-local source-register helper inside `exec_MemStoreInput`, while the combined top-level chunk uses the combined source-register helper. Store-byte rows with different source registers can therefore read the wrong source value.

## Changed Files

```text
risc0/circuit/rv32im/src/zirgen/exec_mem1_chunk0_delta.wgsl
risc0/circuit/rv32im/src/prove/wgsl_pruner.rs
```

## RED Evidence

Previous high-limit focused diff:

```text
DIFF_SUMMARY total_cells=55312384 gpu_wrote=11469798 cpu_wrote=33114247 both_match=11469570 mismatches=228 gpu_only=0 cpu_only=21644449 candidate_cpu_only_nonzero=349209 rows=262144 cols=211
```

After adding MEM1 chunks 2-7 but before repairing chunk0 source registers, the remaining failure was:

```text
candidate_cpu_only_nonzero=0
mismatches=228
observed mismatch column: 30
```

Representative mismatch examples:

```text
row 18368 col 30 gpu=0x67ffffa3 cpu=0x00000000
row 18371 col 30 gpu=0x07ffffaf cpu=0x00000000
row 18911 col 30 gpu=0x00000000 cpu=0x3ffffff8
row 219606 col 30 gpu=0x0ffffe7e cpu=0x07fffeef
```

## GREEN Evidence

Regenerated `exec_mem1_chunk0_delta.wgsl` from the combined chunk0 module so `exec_MemStoreInput` calls `exec_ReadSourceRegs_combined`.

Static regression:

```text
Command:
rustc --test risc0/circuit/rv32im/src/prove/wgsl_pruner.rs -O -o /tmp/wgsl_pruner_tests

Command:
/tmp/wgsl_pruner_tests mem1_chunk0_delta_uses_combined_source_registers --nocapture

Result:
test result: ok. 1 passed; 0 failed; 13 filtered out
```

Formatting and diff hygiene:

```text
cargo fmt --manifest-path risc0/circuit/rv32im/Cargo.toml
cargo fmt --check --manifest-path risc0/circuit/rv32im/Cargo.toml
git diff --check
```

Focused browser no-run build:

```text
Workdir:
examples/browser-prove

Command:
cargo test --target wasm32-unknown-unknown --release iter6d_g_diff_busy_loop_mem1 --no-run

Result:
PASS in 4m18s
```

Focused high-limit browser diff:

```text
Workdir:
examples/browser-prove

Command:
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --target wasm32-unknown-unknown --release \
    iter6d_g_diff_busy_loop_mem1 -- --nocapture
```

Accepted WebGPU limits:

```text
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152
```

Clean diff:

```text
iter6d_g_pre_witgen_dispatch_async mask=0x0000 dispatched_arms=[6]
iter6d_g_pre_witgen_dispatch_async cycles=262144 elapsed_ms=4977
rv32im_witgen elapsed_ms=506
DIFF_SUMMARY total_cells=55312384 gpu_wrote=12139173 cpu_wrote=33114247 both_match=12139173 mismatches=0 gpu_only=0 cpu_only=20975074 candidate_cpu_only_nonzero=0 rows=262144 cols=211
test result: ok
```

## Decision

MEM1 focused diff correctness is now clean for the BusyLoop diagnostic under the required high Chrome limits and with zero candidate nonzero cells left on CPU-only columns.

This does not promote MEM1 GPU-witgen replacement to production. The focused path still reports about 4.98s of pre-witgen dispatch work for this shape, so production replacement must remain opt-in until representative e2e proof generation on BusyLoop+KeccakUnion and xgboost proves a wall-time gain with verified receipts, high limits, and zero fallbacks.

Accepted wall-time gain: `0`.

Next step: add a profitability-gated MEM1 replacement candidate or reject it with representative production evidence.
