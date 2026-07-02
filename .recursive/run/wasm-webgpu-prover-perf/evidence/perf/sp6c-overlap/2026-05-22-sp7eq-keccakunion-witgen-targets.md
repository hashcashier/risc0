# SP7eq - KeccakUnion Witgen Target Recheck

Date: 2026-05-22

## Purpose

Refresh the default representative proof gate after the rejected SP7eo/SP7ep
experiments and quantify the remaining CPU-owned witgen buckets on the
KeccakUnion-inclusive workload. This is a diagnostic checkpoint, not a runtime
optimization.

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
    rv32im_default_representative_e2e_verify -- --nocapture
```

Full local log: `/tmp/sp7eq-current-busy-keccak-witgen-target.log`.

## Result

PASS.

Key lines:

```text
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
browser-prove:metric prove_session_async wall_ms=5694.0 gpu_active_ms=3875.0 gpu_idle_ratio=0.319
browser-prove:done multi_test/busy_loop_po2_18_default_representative: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_default_representative: gpu_dispatches=328 raw_compute_dispatches=604 queue_submits=165 cpu_fallbacks=0 cpu_only_ops=0 upload_bytes=177287244 readback_bytes=478272
browser-prove:metric prove_session_async wall_ms=90796.0 gpu_active_ms=60451.0 gpu_idle_ratio=0.334
browser-prove:done multi_test/keccak_union_default_representative: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_default_representative: gpu_dispatches=5900 raw_compute_dispatches=10675 queue_submits=3078 cpu_fallbacks=0 cpu_only_ops=0 upload_bytes=2812842624 readback_bytes=10183944
test result: ok. 1 passed; 0 failed; 0 ignored; 154 filtered out; finished in 97.20s
```

## CPU-Owned Witgen Buckets

Aggregate `stage done` sums from the representative run:

```text
sum_ms count label
  8547    26 recursion_witgen_accum
  6960    26 recursion_witgen
  4120     5 rv32im_witgen
  2835     5 rv32im_witgen_accum
  2434     9 keccak_witgen
  1121     9 keccak_scatter_preflight
```

The direct Keccak CPU work is real but small relative to recursion: direct
Keccak scatter plus witgen is about `3.555s` on this representative
KeccakUnion run, while recursion witness plus recursion accumulation is about
`15.507s`.

## GPU/FRI Buckets

Aggregate WebGPU stage sums for `multi_test/keccak_union_default_representative`:

```text
sum_s  count label
28.216    38 finalize_async check_group
27.269    38 finalize_async fri_prove
23.871    28 merkle fri_round0 root_top_readback rows=65536 top_size=32
22.979     9 merkle check root_top_readback rows=65536 top_size=32
 4.697    28 merkle check root_top_readback rows=1048576 top_size=32
 1.640    25 commit_group_async recursion_data
 0.982    25 commit_group_async recursion_ctrl
```

The `root_top_readback` labels still include queued GPU work draining at the
readback boundary unless an explicit drain timer is active. They are therefore
attribution evidence for the FRI/check NTT/Merkle pipeline, not proof that
host readback latency alone is dominant.

## Decision

Accepted wall-time gain: `0`.

This run confirms the current default proof path remains correctness-clean on
the BusyLoop plus KeccakUnion representative gate after the latest rejected
experiments. For the user's GPU-witgen priority, the best next offload target
is still chunk-complete recursion witness or recursion accumulation, not a
direct Keccak-only port. Direct Keccak offload has an approximate
KeccakUnion-only ceiling of `3.555s`; recursion witness plus accumulation has
about `15.507s` of cross-workload CPU-owned work here and about `11.7s` on the
latest xgboost profile.

Do not treat this as a new performance win. Use it as the current target
selection evidence for the next implementation attempt.
