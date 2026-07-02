# SP7co: NTT fused expand-first candidate rejected

Date: 2026-05-20
Status: rejected and reverted

## Candidate

Fuse `batch_expand` with the first active forward NTT stage in
`dispatch_batch_expand_into_evaluate_ntt`.

The candidate added `BATCH_EXPAND_FIRST_NTT_WGSL`, used the cached forward
twiddle table, and replaced the standalone expand dispatch plus first NTT stage
with one compute pass. Remaining NTT stages continued through `NTT_STEP_WGSL`.

Intended effect: remove one expand dispatch/submit per hot forward NTT call and
reduce raw dispatches/submits without changing arithmetic.

## Compile gate

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Candidate result: passed in `4m57s`.

## Representative e2e proof gate

BusyLoop + KeccakUnion command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Log: `/tmp/sp7co-busy-keccak-ntt-fused-expand-first.log`

Result: passed with verified BusyLoop and KeccakUnion receipts, high WebGPU
limits, and zero fallback/CPU-only counters.

BusyLoop:

- `wall_ms=7264`
- `gpu_active_ms=3233`
- `raw_compute_dispatches=707`
- `queue_submits=159`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

KeccakUnion:

- `wall_ms=92373`
- `gpu_active_ms=60286`
- `raw_compute_dispatches=12459`
- `queue_submits=2818`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

KeccakUnion FRI round-0 aggregate:

- `fri_prove round=0 domain_in=1048576`: `25620 ms`
- `fri_prove round=0 merkle_new rows=65536 cols=64`: `25577 ms`

## xgboost e2e proof gate

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_xgboost -- --nocapture
```

Log: `/tmp/sp7co-xgboost-ntt-fused-expand-first.log`

Result: passed with verified receipt and zero fallback/CPU-only counters.

xgboost:

- `wall_ms=78485`
- `gpu_active_ms=45047`
- `gpu_idle_ratio=0.426`
- `segments=11`
- `user_cycles=2294946`
- `total_cycles=2883584`
- `raw_compute_dispatches=11242`
- `queue_submits=2494`
- `upload_bytes=3114319100`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

Top xgboost stages:

```text
  78485.0    1 prove_session_async
  38639.0    1 composite_to_succinct_async
  29138.0   32 finalize_async fri_prove
  27422.0   32 fri_prove round=0 domain_in=1048576
  27376.0   32 fri_prove round=0 merkle_new rows=65536 cols=64
  20284.0   10 join_async
  20137.0   10 join_prove_async
  11513.0   11 rv32im_witgen_accum
  11427.0   11 rv32im_accumulate cycles=262144 data_rows=262144 accum_rows=262144
  11154.0   11 rv32im_accumulate step_top_accum_cpu_skip_replaced_misc0=true_major_mask=0x0006 cycles=262144
   9578.0   32 finalize_async check_group
   6456.0   21 recursion_witgen_accum
```

## Comparison against SP7cm

SP7cm accepted baseline:

- BusyLoop `wall_ms=7276`, `raw_compute_dispatches=721`, `queue_submits=173`
- KeccakUnion `wall_ms=92303`, `raw_compute_dispatches=12716`, `queue_submits=3075`
- xgboost `wall_ms=77800`, `gpu_active_ms=45060`, `raw_compute_dispatches=11466`, `queue_submits=2718`
- xgboost FRI round-0 aggregate `27383 ms`

SP7co candidate:

- BusyLoop `7276 -> 7264` (`-12 ms`, flat/noise)
- KeccakUnion `92303 -> 92373` (`+70 ms`, flat/slightly worse)
- xgboost `77800 -> 78485` (`+685 ms`, `+0.9%`)
- xgboost raw dispatches `11466 -> 11242` (`-224`)
- xgboost queue submits `2718 -> 2494` (`-224`)
- xgboost FRI round-0 `27383 -> 27422` (`+39 ms`, flat)

## Decision

Rejected. The candidate was correctness-clean and reduced command/dispatch
counts, but the representative wall gate did not improve. xgboost regressed by
about `0.7 s`, and the hot FRI/Merkle buckets were flat. Under the current
priority, deterministic submit-count cleanup is not enough without wall-time
movement.

The candidate was reverted manually:

- Removed `BATCH_EXPAND_FIRST_NTT_WGSL`.
- Restored standalone `BATCH_EXPAND_WGSL` dispatch before forward NTT stages.
- Restored forward NTT stages from `s_bits = 1 + expand_bits ..= n_bits`.
- Kept accepted SP7cm cached NTT twiddle tables.

Post-revert gates:

- `rg -n "BATCH_EXPAND_FIRST_NTT|webgpu_ntt_expand_first|fused expand NTT" risc0/zkp/src/hal/webgpu.rs` found no markers.
- `cargo fmt --manifest-path risc0/zkp/Cargo.toml` passed.
- `git diff --check` passed.
- Compile after revert passed in `4m55s`:
  `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`.

Do not pursue more NTT dispatch-count-only fusion unless it also reduces the hot
FRI/Merkle buckets or xgboost wall time in the representative e2e proof gate.
