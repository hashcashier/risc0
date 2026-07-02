# SP7dw - MEM1 GPU-Witgen Production Candidate Rejected

Date: 2026-05-21

## Candidate

Promote MEM1/store rows (`major=6`, minors `0..=2`) from CPU-owned witness
generation to GPU-witgen replacement, using the SP7dv combined-source-register
repair and the existing MEM1 direct accumulator so accumulator lookup terms stay
GPU-resident.

The candidate was explicit opt-in only:

- `set_accum_gpu_mem1_direct_enabled(true)`
- `set_witgen_gpu_mem1_replace_minor_mask(0x0007)`
- `set_witgen_gpu_mem1_replace_candidate_enabled(true)`
- nonblocking replacement prewarm disabled for the candidate gates so the MEM1
  replacement path had to run instead of being cold-skipped.

## Correctness Basis

SP7dv repaired the remaining focused MEM1 diff failure by regenerating
`exec_mem1_chunk0_delta.wgsl` with the combined source-register helper used by
the combined top-level chunk. The high-limit focused diff then reported:

```text
DIFF_SUMMARY total_cells=55312384 gpu_wrote=12139173 cpu_wrote=33114247 both_match=12139173 mismatches=0 gpu_only=0 cpu_only=20975074 candidate_cpu_only_nonzero=0 rows=262144 cols=211
```

That made MEM1 replacement eligible for a representative production wall-time
candidate, but not accepted by itself.

## Representative E2E Evidence

Both proof gates negotiated the required high WebGPU limits:

- `max_buffer_size=4294967292`
- `max_storage_buffer_binding_size=2147483644`
- `max_compute_workgroup_storage_size=49152`

Both gates generated real browser WebGPU proofs, verified receipts, and asserted
zero `cpu_fallbacks` and zero `cpu_only_ops`.

### BusyLoop + KeccakUnion

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --target wasm32-unknown-unknown --release iter6d_g_mem1_witgen_replace_candidate_e2e_verify -- --nocapture
```

Result: PASS, total test time `101.25s`.

Key proof metrics:

```text
iter6d_g_pre_witgen_dispatch_async mask=0x0061 dispatched_arms=[0, 5, 6]
iter6d_g_minor_dispatch arm=6 minor=2 cycles=31875
iter6d_g_pre_witgen_dispatch_async elapsed_ms=2769
rv32im_witgen elapsed_ms=250
rv32im_accumulate major_mask=0x0046
MEM1 direct_gpu dispatched rows=31917
```

Comparison:

```text
SP7dt accepted BusyLoop+KeccakUnion: 99.31s
SP7dw MEM1 witgen candidate:        101.25s
Movement:                           +1.94s slower
```

### xgboost

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --target wasm32-unknown-unknown --release iter6d_g_mem1_witgen_replace_candidate_xgboost_e2e_verify -- --nocapture
```

Result: PASS, total test time `73.77s`; journal decoded to
`30.528042544062632`.

Key proof metrics:

```text
segment 0:
iter6d_g_pre_witgen_dispatch_async mask=0x0061 dispatched_arms=[0, 5, 6]
iter6d_g_minor_dispatch arm=6 minor=2 cycles=31434
iter6d_g_pre_witgen_dispatch_async elapsed_ms=2624
rv32im_witgen elapsed_ms=240

segment 1:
iter6d_g_pre_witgen_dispatch_async elapsed_ms=17
rv32im_witgen elapsed_ms=220
```

Comparison:

```text
SP7dt accepted xgboost:      73.39s
SP7dw MEM1 witgen candidate: 73.77s
Movement:                    +0.38s slower
```

## Finding

MEM1 GPU-witgen replacement is correctness-clean on the representative matrix,
but the first-segment compile/wait/predispatch cost dominates the saved CPU
witness time:

- MEM1 predispatch wait: about `2.6-2.8s` on segment 0.
- MEM1 witness saving: about `250-280ms` per segment in the observed logs.
- Later xgboost segments can see hot predispatch around `17ms`, but the full
  proof still stayed slightly wall-negative in the accepted e2e gate.

## Decision

Reject MEM1 GPU-witgen replacement for production/default.

Accepted wall-time gain: `0`.

Keep the SP7dv focused-diff repair and MEM1 direct-accumulator default path.
Do not enable MEM1 witness replacement by default, and do not continue
opportunistic MEM1 replacement as the next immediate performance avenue unless
kernel compilation/predispatch is amortized outside proof wall time or a
chunk-complete witgen backend removes a much larger CPU-owned surface.

The next significant-performance target should be a measured large bucket:
FRI/check `batch_expand_into_evaluate_ntt` active work, or a broader
chunk-complete witness backend with representative e2e proof gates before any
performance claim.
