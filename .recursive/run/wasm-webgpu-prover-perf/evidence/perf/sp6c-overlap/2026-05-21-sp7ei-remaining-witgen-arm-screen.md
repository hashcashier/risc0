# SP7ei Remaining Witgen Arm Screen

Date: 2026-05-21

## Objective

Screen the next obvious GPU-witgen offload candidate after SP7eh before spending implementation time on another small generated arm.

## Current Representative Baseline

Latest accepted representative state is SP7eh:

- BusyLoop + KeccakUnion representative e2e: verified receipts, zero `cpu_fallbacks` / `cpu_only_ops`, total `97.73s`.
- xgboost representative e2e captured rerun: verified journal `30.528042544062632`, zero `cpu_fallbacks` / `cpu_only_ops`, `prove_session_async wall_ms=73307`, `gpu_active_ms=45175`, `raw_compute_dispatches=9718`, `queue_submits=2740`, `upload_bytes=2724587656`.

Current xgboost stage aggregation from `/tmp/sp7eh-xgboost-preflight-meta-reuse.log` still points at the larger buckets:

- `finalize_async fri_prove`: about `29.2s` total, dominated by round-0 `batch_expand_into_evaluate_ntt`.
- `finalize_async check_group`: about `9.6s` total, dominated by check-group `batch_expand_into_evaluate_ntt`.
- `rv32im_witgen_accum`: about `7.0s` total.
- `recursion_witgen_accum`: about `7.6s` total.
- `rv32im_witgen`: about `4.1s` total.

## MUL0 Focused Diff

Command run from `examples/browser-prove/`:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_diff_busy_loop_mul0 -- --nocapture
```

Captured log: `/tmp/sp7ei-diff-busyloop-mul0.log`

Result: expected diagnostic failure after diff proof path.

Key evidence:

- high WebGPU limits: `max_buffer_size=4294967292`, `max_storage_buffer_binding_size=2147483644`, `max_compute_workgroup_storage_size=49152`
- `iter6d_g diff=on fixture=busy_loop candidate_major=3`
- on-demand compilation paid two `mul0_chunk0` compiles
- `iter6d_g_pre_witgen_dispatch_async elapsed_ms=3337`
- `rv32im_witgen ... elapsed_ms=517`
- `DIFF_SUMMARY total_cells=55312384 gpu_wrote=9187708 cpu_wrote=33114247 both_match=9187708 mismatches=0 gpu_only=0 cpu_only=23926539 candidate_cpu_only_nonzero=148 rows=262144 cols=211`
- missing nonzero cells are in columns `29`, `31`, `33`, `35`, and `37` for four rows, with CPU value `0x0ffffffe`
- WebGPU diagnostics stayed clean for the focused path: `cpu_fallbacks=0`, `cpu_only_ops=0`

## Interpretation

MUL0 is semantically clean for the cells it writes (`mismatches=0`, `gpu_only=0`) but not chunk-complete (`candidate_cpu_only_nonzero=148`). It would need generated-kernel repair before any production e2e timing.

The expected payoff is also too small for the current priority. The earlier xgboost major histogram showed MUL0 at `65151 / 2883584` rows, or about `2.26%`. Even a perfect MUL0 production path would be unlikely to produce an immediate significant wall-time gain, and the focused run shows cold compile/predispatch cost of `3337ms`, far larger than the per-segment `rv32im_witgen` bucket it could replace.

## Decision

Reject MUL0 as the next immediate-performance lever.

Do not continue screening small zero-back arms one by one unless the candidate has both:

- a large representative histogram share, and
- focused diff evidence showing chunk-complete output before production e2e timing.

The next material work should target the larger measured buckets: FRI/check `batch_expand_into_evaluate_ntt` first, or a chunk-complete RV32IM witness/accumulator design with a concrete path to removing a much larger CPU-owned surface.

Accepted wall-time gain: 0.
