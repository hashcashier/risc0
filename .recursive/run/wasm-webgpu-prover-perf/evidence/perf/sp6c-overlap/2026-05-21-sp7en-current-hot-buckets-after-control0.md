# SP7en - Current Hot Buckets After CONTROL0

Date: 2026-05-21

## Purpose

Capture a fresh xgboost e2e proof profile after SP7em default-enabled the
direct CONTROL0 accumulator, so the next optimization is selected from current
evidence rather than stale pre-CONTROL0 measurements.

## Command

Workdir: `examples/browser-prove`

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
    xgboost_succinct_receipt_verifies -- --nocapture \
    > /tmp/sp7en-current-xgboost.log 2>&1
```

The command required the normal browser/Cargo-test escalation. Release wasm
compile took `2m19s`; proof timing is the wasm test runtime below.

## Result

PASS.

Key lines:

```text
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
browser-prove:metric prove_session_async wall_ms=68299.0 gpu_active_ms=45544.0 gpu_idle_ratio=0.333
browser-prove:done xgboost: segments=11 user_cycles=2294946 total_cycles=2883584
browser-prove:webgpu xgboost: gpu_dispatches=5259 raw_compute_dispatches=9729 queue_submits=2740 cpu_fallbacks=0 cpu_only_ops=0 uploads=3304 upload_bytes=2642986704 readbacks=352 readback_bytes=7488592
test result: ok. 1 passed; 0 failed; 0 ignored; 154 filtered out; finished in 68.83s
```

The journal assertion in `xgboost_succinct_receipt_verifies` remained active
and passed (`30.528042544062632`).

## Stage Aggregates

Approximate aggregate stage sums from `/tmp/sp7en-current-xgboost.log`:

```text
fri_prove                 count=32 sum_ms=29291
check_group               count=32 sum_ms=9632
recursion_witgen_accum    count=21 sum_ms=6448
recursion_witgen          count=21 sum_ms=5211
rv32im_witgen             count=11 sum_ms=3644
rv32im_witgen_accum       count=11 sum_ms=2542
commit_rv32im_data        count=11 sum_ms=1322
commit_rv32im_accum       count=11 sum_ms=1255
commit_rv32im_code        count=11 sum_ms=931
pre_witgen_dispatch       count=11 sum_ms=120
```

The RV32IM direct accumulators are now doing their job: segment 0 logged
`major_mask=0x00c6` and dispatched direct rows for MISC0, MISC1, MISC2, MEM0,
MEM1, and CONTROL0.

## Upload / Readback Buckets

Largest xgboost upload sources:

```text
1673427040 webgpu_zeroize_sparse_values
 540042928 webgpu_zeroize_sparse_ranges
 172402720 iter6d_g_arm_txns
  57897424 webgpu_zero_default_sparse_ranges
  45609536 webgpu_zero_default_sparse_values
  41943040 iter6d_g_arm_preflight
  37267544 recursion_ctrl_compact
  20971520 iter6d_g_arm_diff_count
```

Readbacks:

```text
6856000 merkle_query
 363280 out
 236544 nodes
  32768 final_coeffs
```

## Decision

This is a profile checkpoint, not a runtime optimization.

Current accepted xgboost remains in the `68-69s` band after SP7em. The next
highest immediate wall-time targets are no longer one-off RV32IM direct
accumulator arms:

- FRI/check proving dominates measured wall (`fri_prove` about `29.3s`,
  `check_group` about `9.6s`).
- If staying strictly on witness-generation offload, the largest remaining
  CPU-owned bucket is recursion witness/accumulation (`recursion_witgen` +
  `recursion_witgen_accum` about `11.7s`), not another opportunistic RV32IM
  opcode arm.
- RV32IM witness/accum is now about `6.2s` combined on xgboost, so further
  RV32IM one-arm replacement work has limited upside unless it is
  chunk-complete or removes broad sparse upload/CPU-shadow work.

Do not reopen generated CONTROL0, MISC1, MEM1, or broad MEM0 replacement as
the next immediate path without a design that eliminates the prior prewarm,
dispatch, or mixed-representative wall regressions.
