# SP7fs Recursion Data Upload Elision Rejected

Date: 2026-05-22

## Goal

Test whether the SP7fq/SP7fr recursion witness GPU `verify_mem` candidate can skip uploading the CPU-generated `recursion_data` sparse shadow before the post-zeroize GPU witness rewrite.

## RED

Focused BusyLoop e2e proof gate:

```text
script -q -e -c "env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release recursion_witgen_gpu_verify_mem_candidate_busy_loop_e2e_verify -- --nocapture" .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7fs-red-busyloop-recursion-data-upload-cap.chrome.txt
```

Result: expected failure after proof generation, with receipt path otherwise intact.

Key evidence:

- `webgpu_zeroize_sparse_upload name=recursion_data values=6014460 ranges=832116 sparse_bytes=30714784 dense_bytes=134217728`
- `webgpu_zeroize_sparse_values uploads=2 upload_bytes=98063244`
- `gpu_dispatches=329 raw_compute_dispatches=613 queue_submits=168`
- `cpu_fallbacks=0 cpu_only_ops=0`
- assertion failed on the new `<= 80_000_000` upload cap

## GREEN Candidate

Candidate change: after `generate_witness_exec_plan(...)` in the recursion WebGPU witness candidate path, call `data.mark_gpu_dirty()` so subsequent zeroize/commit uses browser-zeroed GPU storage plus the post-zeroize GPU witness rewrite instead of uploading the stale CPU shadow.

Focused BusyLoop e2e proof gate:

```text
script -q -e -c "env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release recursion_witgen_gpu_verify_mem_candidate_busy_loop_e2e_verify -- --nocapture" .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7fs-green-busyloop-recursion-data-upload-elided.chrome.txt
```

Result: rejected for correctness.

Key evidence:

- `recursion_witgen_gpu_verify_mem_candidate_plan work_cycles=145470 total_cycles=262144 valid_rows=994436 buckets=317029`
- `webgpu_zeroize_sparse_values uploads=1 upload_bytes=74005320`
- `gpu_dispatches=329 raw_compute_dispatches=612 queue_submits=167`
- `cpu_fallbacks=0 cpu_only_ops=0`
- failed before the upload assertion with `verify lift` / `verification indicates proof is invalid`

Interpretation: the upload elision removes about 24 MB from this focused BusyLoop proof path, but the current GPU candidate does not rewrite every semantically required `recursion_data` cell. The CPU-generated sparse `recursion_data` shadow is still required for correctness until the generated GPU witness path covers more than the current WOM row / `verify_mem` subset.

## Post-Reject Restore

The production patch and RED upload-cap assertion were removed.

Restored focused BusyLoop e2e proof gate:

```text
script -q -e -c "env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release recursion_witgen_gpu_verify_mem_candidate_busy_loop_e2e_verify -- --nocapture" .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7fs-post-reject-busyloop-restored.chrome.txt
```

Result: PASS.

Key evidence:

- `test result: ok. 1 passed; 0 failed; 0 ignored; 168 filtered out; finished in 6.17s`
- `prove_session_async wall_ms=6001.0 gpu_active_ms=4510.0 gpu_idle_ratio=0.248`
- `webgpu_zeroize_sparse_upload name=recursion_data values=6002018 ranges=830286 sparse_bytes=30650376 dense_bytes=134217728`
- `webgpu_zeroize_sparse_values uploads=2 upload_bytes=98013444`
- `gpu_dispatches=329 raw_compute_dispatches=613 queue_submits=168`
- `cpu_fallbacks=0 cpu_only_ops=0`

## Decision

Rejected and reverted. Do not retry raw `recursion_data` CPU-shadow elision without first proving that the GPU witness kernels authoritatively rewrite all data cells needed by the recursion lift proof. The next GPU-witgen work should target missing generated recursion witness coverage or another measured hot bucket, not upload elision by stale-shadow invalidation.
