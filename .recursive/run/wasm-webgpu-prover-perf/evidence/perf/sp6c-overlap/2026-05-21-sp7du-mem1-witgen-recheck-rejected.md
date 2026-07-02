# SP7du MEM1 GPU-Witgen Recheck Rejected

Date: 2026-05-21

## Scope

Rechecked MEM1 (`major=6`) GPU-witgen replacement viability against the latest accepted working state after SP7dr-SP7dt direct-accum and cold-skip changes.

This was a no-production-code diagnostic. The goal was to decide whether MEM1 could be a near-term witgen offload candidate or whether it still requires a larger chunk-complete/codegen repair.

## Command

```text
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

Workdir:

```text
examples/browser-prove/
```

The first sandboxed attempt failed on Cargo's target lock path:

```text
failed to open ... examples/target/release/.cargo-lock
Read-only file system (os error 30)
```

The command was rerun with the approved `cargo test` escalation.

## Result

The browser test negotiated the representative WebGPU limits:

```text
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152
```

The diff path still fails:

```text
DIFF_SUMMARY total_cells=55312384 gpu_wrote=11469798 cpu_wrote=33114247 both_match=11469570 mismatches=228 gpu_only=0 cpu_only=21644449 candidate_cpu_only_nonzero=349209 rows=262144 cols=211
```

Other relevant diagnostics:

```text
iter6d_g_pre_witgen_dispatch_async elapsed_ms=3296
iter6d_g_per_arm_dispatch dispatched=1 skipped=0
iter6d_g_pre_witgen_dispatch_async mask=0x0000 dispatched_arms=[6]
rv32im_witgen elapsed_ms=485
raw_compute_dispatches=3
queue_submits=4
cpu_fallbacks=0
cpu_only_ops=0
```

The failure matches the earlier SP7ay MEM1 screen: MEM1 remains both mismatching and incomplete in the current top-level chunk0/chunk1 shape.

## Decision

Reject MEM1 GPU-witgen replacement as an immediate performance avenue.

Do not spend near-term effort forcing MEM1 into production replacement. A viable MEM1 path would need chunk-complete generated kernels plus semantic repair for the mismatched cells and missing CPU-owned cells before any representative e2e wall-time test would be meaningful.

Accepted wall-time gain: `0`.

Next performance work should target a larger measured bucket or a genuinely chunk-complete witgen backend, not opportunistic MEM1 replacement.
