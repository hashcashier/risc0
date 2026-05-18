# SP6o stale-destination scatter probe skip

Date: 2026-05-18
Worktree: `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf`
Branch: `recursive/wasm-webgpu-prover-perf`
Baseline commit: `605784781 SP6n: skip sparse scatter destination upload`

## Hypothesis

SP6n removed full destination uploads before non-authoritative sparse scatter,
but stale-destination mirror-mode scatter still uploaded `webgpu_scatter_offsets`
and `webgpu_scatter_values`, then dispatched a GPU kernel whose result could
not make the whole destination buffer current. On xgboost this remaining
scatter probe cost was about 187.5 MB.

If the destination GPU buffer is already stale, skip the GPU scatter probe and
run the existing CPU mirror only. If the destination GPU buffer is current,
keep the sparse GPU dispatch because the touched-cell writes preserve a fully
current GPU buffer. GPU-authoritative scatter remains unchanged.

## Change

- Extended the focused scatter HAL test to require zero offset/value uploads
  and zero GPU dispatches for stale-destination mirror-mode scatter.
- In `WebGpuHal::scatter`, added a non-authoritative branch:
  - destination has a GPU buffer;
  - destination GPU is not current;
  - run CPU scatter, record CPU mirror, leave CPU authoritative, and return.
- Kept the GPU dispatch path for GPU-authoritative scatter and for
  non-authoritative scatter whose destination GPU buffer is already current.

Changed files:

- `examples/browser-prove/src/lib.rs`
- `risc0/zkp/src/hal/webgpu.rs`

## TDD evidence

RED command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_scatter_skips_gpu_probe_for_stale_sparse_destination_in_mirror_mode -- --nocapture
```

Expected RED failure:

```text
assertion `left == right` failed: stale-destination mirror-mode scatter should not upload offsets for an unusable GPU probe
  left: 16
 right: 0
test tests::webgpu_hal_scatter_skips_gpu_probe_for_stale_sparse_destination_in_mirror_mode ... FAIL
```

GREEN result:

```text
test tests::webgpu_hal_scatter_skips_gpu_probe_for_stale_sparse_destination_in_mirror_mode ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 115 filtered out; finished in 0.10s
```

Broader HAL parity guard:

```text
test tests::webgpu_hal_core_gpu_results_match_cpu ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 115 filtered out; finished in 0.13s
```

The broader test covers the already-current destination scatter path, which
still dispatches sparse GPU writes and keeps GPU/CPU contents equal.

## Correctness evidence

Focused browser proof:

```text
browser-prove:metric prove_session_async wall_ms=7494.0 gpu_active_ms=4120.0 gpu_idle_ratio=0.450
browser-prove:webgpu multi_test/busy_loop_po2_18_async: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0 uploads=290 upload_bytes=562488124 device_copies=2 device_copy_bytes=2048 readbacks=66 readback_bytes=772032 bind_group_layout_creations=17 bind_group_layout_cache_hits=89 bind_group_creations=201 compute_pipeline_creations=18 compute_pipeline_cache_hits=88 buffers=371 buffer_bytes=3693196564
browser-prove:webgpu-op multi_test/busy_loop_po2_18_async: op=scatter gpu_dispatches=0 cpu_mirrors=1 cpu_fallbacks=0 cpu_only_ops=0
test tests::native_busy_loop_po2_18_async_succinct_receipt_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 115 filtered out; finished in 7.65s
```

Full xgboost browser proof, first trial:

- Log: `.recursive/run/wasm-webgpu-prover-perf/evidence/logs/sp6o-scatter-skip-stale-probe-xgboost-20260518.txt`

```text
browser-prove:metric pool_xgboost_smoke wall_ms=101350
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5258 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4798 upload_bytes=8287918812 device_copies=32 device_copy_bytes=32768 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=39 bind_group_layout_cache_hits=1644 bind_group_creations=3411 compute_pipeline_creations=40 compute_pipeline_cache_hits=1643 buffers=6159 buffer_bytes=54815488692
browser-prove:webgpu-pool-op pool_xgboost_smoke: op=scatter gpu_dispatches=0 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 115 filtered out; finished in 101.53s
```

Full xgboost browser proof, repeat trial:

- Log: `.recursive/run/wasm-webgpu-prover-perf/evidence/logs/sp6o-scatter-skip-stale-probe-xgboost-repeat-20260518.txt`

```text
browser-prove:metric pool_xgboost_smoke wall_ms=99468
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5258 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4798 upload_bytes=8287918812 device_copies=32 device_copy_bytes=32768 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=39 bind_group_layout_cache_hits=1644 bind_group_creations=3411 compute_pipeline_creations=40 compute_pipeline_cache_hits=1643 buffers=6159 buffer_bytes=54815488692
browser-prove:webgpu-pool-op pool_xgboost_smoke: op=scatter gpu_dispatches=0 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 115 filtered out; finished in 99.63s
```

## Comparison

SP6n baseline:

```text
trial1 wall_ms=100399
trial2 wall_ms=100962
mean_wall_ms=100680.5
gpu_dispatches=5269
uploads=4831
upload_bytes=8475457284
bind_group_creations=3422
compute_pipeline_creations=41
buffers=6192
buffer_bytes=55003027164
scatter gpu_dispatches=11 cpu_mirrors=11
```

SP6o:

```text
trial1 wall_ms=101350
trial2 wall_ms=99468
mean_wall_ms=100409
gpu_dispatches=5258
uploads=4798
upload_bytes=8287918812
bind_group_creations=3411
compute_pipeline_creations=40
buffers=6159
buffer_bytes=54815488692
scatter gpu_dispatches=0 cpu_mirrors=11
```

Deltas against SP6n:

```text
total_upload_bytes_delta=-187538472
buffer_bytes_delta=-187538472
upload_count_delta=-33
buffer_count_delta=-33
gpu_dispatches_delta=-11
bind_group_creations_delta=-11
compute_pipeline_creations_delta=-1
mean_wall_delta_ms=-271.5
```

## Interpretation

This is a small positive result. The byte/dispatch reductions are
deterministic across both full xgboost proofs:

- scatter GPU dispatches: `11 -> 0`
- total uploads: `4831 -> 4798`
- upload bytes: `8.475 GB -> 8.288 GB`
- buffer bytes: `55.003 GB -> 54.815 GB`

The wall signal is noisy because the lever is small. Trial 1 regressed against
SP6n, trial 2 improved substantially, and the two-run mean is 271.5 ms better
than the SP6n two-run mean. Keep the change because it removes provably
unusable GPU work, preserves receipt correctness, adds no CPU fallbacks or
CPU-only ops, and slightly improves the noise-averaged wall result.

## Next levers

- The remaining `data` and `accum` uploads are CPU-originated witness/accum
  ownership boundaries, not scatter probe waste.
- Further upload reductions require moving RV32IM witness/accum generation
  toward GPU authority, especially generated `step_TopAccum`.
- Small HAL traffic cleanup is now mostly exhausted; next meaningful xgboost
  wall reductions need generated-circuit GPU coverage or FRI/hash work.
