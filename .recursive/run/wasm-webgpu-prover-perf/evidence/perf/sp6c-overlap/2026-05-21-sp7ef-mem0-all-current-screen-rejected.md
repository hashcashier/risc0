# SP7ef - All-MEM0 Current-State Screen Rejected

Date: 2026-05-21

## Intent

Screen the retained all-MEM0 opt-in browser proof gate against the current accepted
LW-only default baseline before spending implementation time on a workload/shape
gate. Prior SP7dp evidence showed all-MEM0 was xgboost-positive versus the older
SP7do state but BusyLoop+KeccakUnion-negative. After SP7dq/SP7dy, the remaining
possible xgboost delta needed to be rechecked.

No production code was changed for this screen.

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
    iter6d_g_mem0_direct_accum_candidate_xgboost_e2e_verify -- --nocapture \
    > /tmp/sp7ef-mem0-all-current-xgboost.log 2>&1
```

## Result

Result: PASS. Test result: `1 passed`, finished in `72.52s`.

Key proof evidence from `/tmp/sp7ef-mem0-all-current-xgboost.log`:

- High WebGPU limits: `max_buffer_size=4294967292`, `max_storage_buffer_binding_size=2147483644`, `max_compute_workgroup_storage_size=49152`.
- `prove_session_async wall_ms=71993`, `gpu_active_ms=45239`, `gpu_idle_ratio=0.372`.
- `segments=11`, `user_cycles=2294946`, `total_cycles=2883584`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.
- `raw_compute_dispatches=9718`, `queue_submits=2740`.
- `upload_bytes=2766743276`, `readback_bytes=7488592`.
- `rv32im_accumulate MEM0 direct_gpu` dispatched on all 11 RV32IM segments.

Current accepted comparison points:

- SP7dy accepted xgboost: `prove_session_async wall_ms=72023`, total test `72.56s`.
- SP7ed fresh current baseline: `prove_session_async wall_ms=72220`, total test `72.76s`.

The all-MEM0 opt-in movement is therefore noise-level versus the current accepted
baseline: roughly `-30ms` versus SP7dy prove-session wall and `-0.04s` on total
test time. It does not justify a default workload/shape gate, especially because
SP7dp already showed the same all-MEM0 path was BusyLoop+KeccakUnion-negative.

## Decision

Rejected as an immediate lever. Accepted wall-time gain: 0.

Do not implement a broad all-MEM0 workload/shape gate unless new evidence shows
a material current-baseline xgboost win and the representative BusyLoop +
KeccakUnion gate remains flat or positive.

Next priority returns to higher-ceiling work:

- a real memory-pass reduction in `batch_expand_into_evaluate_ntt`, or
- chunk-complete GPU-witgen/accumulation that removes more CPU-owned surface
  than the current opportunistic per-arm replacements.
