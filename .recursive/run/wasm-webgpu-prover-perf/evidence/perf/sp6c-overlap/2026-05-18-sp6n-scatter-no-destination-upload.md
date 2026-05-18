# SP6n scatter destination upload elision

Date: 2026-05-18
Worktree: `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf`
Branch: `recursive/wasm-webgpu-prover-perf`
Baseline commit: `483b692fc SP7d: record unchecked row negative result`

## Hypothesis

`WebGpuHal::scatter` was uploading the whole sparse destination before
dispatching a small scatter kernel in non-authoritative mirror mode. For RV32IM
data this meant a full invalid-initialized data trace upload before scatter
writes sparse touched cells and the CPU mirror remains the semantic source of
truth.

Skipping that destination upload in mirror mode should preserve correctness and
cut xgboost host-to-device traffic. GPU-authoritative scatter must keep the old
sync-before-scatter behavior because sparse GPU writes need the previous
destination contents.

## Change

- Added `webgpu_hal_scatter_does_not_upload_sparse_destination_in_mirror_mode`.
- Added a `sync_destination` argument to `WebGpuHal::dispatch_scatter`.
- Passed `sync_destination = self.gpu_authoritative()`.
- In non-authoritative mode, after a successful sparse GPU dispatch:
  - mirror the scatter on CPU,
  - record a GPU dispatch plus CPU mirror,
  - leave the CPU shadow authoritative with `into.mark_cpu_result(false)`.

Changed files:

- `examples/browser-prove/src/lib.rs`
- `risc0/zkp/src/hal/webgpu.rs`

## TDD evidence

RED was observed before the production change with the new HAL test:

```text
test tests::webgpu_hal_scatter_does_not_upload_sparse_destination_in_mirror_mode ... FAILED
left: 4096
right: 0
non-authoritative scatter should not upload the whole sparse destination
```

GREEN command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_scatter_does_not_upload_sparse_destination_in_mirror_mode -- --nocapture
```

GREEN result:

```text
test tests::webgpu_hal_scatter_does_not_upload_sparse_destination_in_mirror_mode ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 115 filtered out; finished in 0.09s
```

The test verifies:

- destination upload bytes for the sparse destination are `0`;
- scatter still dispatches on GPU;
- CPU mirror count is `1`;
- CPU fallback count is `0`;
- `into.gpu_is_current()` is false after scatter, forcing a later GPU reader to
  upload the full CPU shadow instead of trusting partial GPU contents;
- an explicit later `sync_cpu_to_gpu` restores GPU/CPU equality.

## Correctness evidence

Focused browser proof:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=300 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  native_busy_loop_po2_18_async_succinct_receipt_verify -- --nocapture
```

Result:

```text
browser-prove:metric prove_session_async wall_ms=7819.0 gpu_active_ms=4123.0 gpu_idle_ratio=0.473
browser-prove:webgpu multi_test/busy_loop_po2_18_async: gpu_dispatches=329 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0 uploads=293 upload_bytes=580415564 device_copies=2 device_copy_bytes=2048 readbacks=66 readback_bytes=772032 bind_group_layout_creations=18 bind_group_layout_cache_hits=89 bind_group_creations=202 compute_pipeline_creations=19 compute_pipeline_cache_hits=88 buffers=374 buffer_bytes=3711124004
browser-prove:webgpu-op multi_test/busy_loop_po2_18_async: op=scatter gpu_dispatches=1 cpu_mirrors=1 cpu_fallbacks=0 cpu_only_ops=0
test tests::native_busy_loop_po2_18_async_succinct_receipt_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 115 filtered out; finished in 7.97s
```

Full xgboost browser proof, first trial:

```text
browser-prove:metric pool_xgboost_smoke wall_ms=100399
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5269 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4831 upload_bytes=8475457284 device_copies=32 device_copy_bytes=32768 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=40 bind_group_layout_cache_hits=1654 bind_group_creations=3422 compute_pipeline_creations=41 compute_pipeline_cache_hits=1653 buffers=6192 buffer_bytes=55003027164
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=data uploads=322 upload_bytes=5252317184
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 115 filtered out; finished in 100.56s
```

Full xgboost browser proof, repeat trial:

- Log: `.recursive/run/wasm-webgpu-prover-perf/evidence/logs/sp6n-scatter-no-dest-upload-xgboost-repeat-20260518.txt`

```text
browser-prove:metric pool_xgboost_smoke wall_ms=100962
browser-prove:webgpu-pool pool_xgboost_smoke: gpu_dispatches=5269 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0 uploads=4831 upload_bytes=8475457284 device_copies=32 device_copy_bytes=32768 readbacks=1056 readback_bytes=12963952 bind_group_layout_creations=40 bind_group_layout_cache_hits=1654 bind_group_creations=3422 compute_pipeline_creations=41 compute_pipeline_cache_hits=1653 buffers=6192 buffer_bytes=55003027164
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=data uploads=322 upload_bytes=5252317184
test tests::webgpu_pool_xgboost_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 115 filtered out; finished in 101.15s
```

## Comparison

SP7c baseline:

```text
browser-prove:metric pool_xgboost_smoke wall_ms=101254
browser-prove:webgpu-pool pool_xgboost_smoke: uploads=4985 upload_bytes=10909202180 device_copies=32 device_copy_bytes=32768
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=accum uploads=119 upload_bytes=1716518912
browser-prove:webgpu-pool-upload pool_xgboost_smoke: source=data uploads=476 upload_bytes=7686062080
```

SP6n:

```text
trial1 wall_ms=100399
trial2 wall_ms=100962
mean_wall_ms=100680.5
upload_bytes=8475457284
data_upload_bytes=5252317184
```

Deltas against SP7c:

```text
total_upload_bytes_delta=-2433744896
data_upload_bytes_delta=-2433744896
upload_count_delta=-154
data_upload_count_delta=-154
trial1_wall_delta_ms=-855
trial2_wall_delta_ms=-292
mean_wall_delta_ms=-573.5
```

## Interpretation

This is a correctness-positive and traffic-positive change. The byte reduction
is deterministic across xgboost proofs and comes exactly from `data` uploads:
`10.909 GB -> 8.475 GB` total, with `data` `7.686 GB -> 5.252 GB`.

The wall-time win is real in both SP6n trials but modest and noise-adjacent:
about `0.29-0.86 s` versus the SP7c proof-backed baseline, mean `0.57 s`.
Keeping the patch is still justified because it removes a provably unnecessary
2.43 GB host-to-device transfer without adding CPU fallbacks, device copies, or
receipt risk.

The result also falsifies the broader assumption that the full `accum` preupload
was caused by scatter destination sync. `accum` remained unchanged at
`1.716 GB`, so the next upload target must be a different ownership transition.

## Next levers

- Find the `accum` ownership transition that forces `1.716 GB` of uploads.
- Continue treating RV32IM generated `step_TopAccum` as the largest single
  CPU wall surface, but only accept changes after focused proof and xgboost
  proof receipt verification.
- Look for similar sparse-write destination uploads in non-scatter paths before
  deeper generator-level work.
