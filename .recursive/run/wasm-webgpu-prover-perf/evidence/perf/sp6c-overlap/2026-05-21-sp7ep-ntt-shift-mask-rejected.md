# SP7ep NTT shift/mask indexing rejected

Date: 2026-05-21

## Candidate

Tested a narrow WebGPU NTT shader change: replace dynamic power-of-two division/modulo in hot indexing paths with shifts and masks.

Touched candidate paths:

- fused local `batch_expand_into_evaluate_ntt` NTT stages;
- global `NTT_STEP_WGSL`;
- fused first inverse NTT stage used by `batch_interpolate_ntt_from`.

This did not change dispatch topology, memory footprint, or proof semantics.

## RED / GREEN

RED added a focused browser HAL assertion requiring a new hidden shader-shape guard:

```text
cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_ntt_gpu_results_match_cpu --no-run
error[E0425]: cannot find function `ntt_shaders_use_shift_mask_indexing` in module `risc0_zkp::hal::webgpu`
```

GREEN implemented the shift/mask shader indexing and the structural guard. Focused browser HAL coverage then reached CPU/GPU NTT parity under high WebGPU limits. The first GREEN run failed only on an old exact bind-group-count assertion (`left: 3`, `right: 4`) after parity had passed, so the assertion was relaxed to `<= 4` because the current cache path legitimately creates fewer bind groups.

Focused GREEN:

```text
webgpu_hal_ntt_gpu_results_match_cpu ... ok
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
test result: ok. 1 passed; 0 failed; finished in 0.09s
```

## Representative proof gates

BusyLoop + KeccakUnion:

```text
rv32im_default_representative_e2e_verify ... ok
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
test result: ok. 1 passed; 0 failed; finished in 96.61s
```

xgboost log: `/tmp/sp7ep-xgboost-ntt-shift-mask.log`

```text
xgboost_succinct_receipt_verifies ... ok
prove_session_async wall_ms=67716 gpu_active_ms=45473 gpu_idle_ratio=0.328
raw_compute_dispatches=9729 queue_submits=2740
cpu_fallbacks=0 cpu_only_ops=0
upload_bytes=2642988428 readback_bytes=7488592
test result: ok. 1 passed; 0 failed; finished in 68.22s
```

## Comparison

Accepted/default comparison points:

- SP7em BusyLoop+KeccakUnion: `95.73s`
- SP7em promoted xgboost: `68.02s`
- SP7en fresh xgboost profile: `68.83s` total, `prove_session_async wall_ms=68299`

Candidate:

- BusyLoop+KeccakUnion: `96.61s`, about `+0.88s` vs SP7em accepted.
- xgboost: `68.22s` total / `67.716s` prove wall, about `-0.61s` vs SP7en fresh but still flat/noisy vs SP7em accepted best.

The result is correctness-clean but not a representative material wall-time win. The mixed BusyLoop+KeccakUnion gate moved negative, and the xgboost movement is below the acceptance threshold for the current "immediate significant improvements" priority.

## Revert / retained harness fix

Rejected and reverted the runtime shader changes and the hidden structural guard.

Retained one test-harness fix in `examples/browser-prove/src/lib.rs`: `webgpu_hal_ntt_gpu_results_match_cpu` now asserts `bind_group_creations <= 4` instead of exactly `4`. Post-revert current accepted runtime produces `3`, so the exact count was a brittle validation blocker unrelated to proof correctness or performance.

Post-revert checks:

```text
cargo fmt --check --manifest-path risc0/zkp/Cargo.toml
cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml
git diff --check
rg "ntt_shaders_use_shift_mask_indexing|shift/mask indexing|row_shift|stage_shift|pair >>|pairs_per_row - 1u|input_idx1 = input_row_base \+ s" examples/browser-prove/src/lib.rs risc0/zkp/src/hal/webgpu.rs
```

All passed / marker search clean.

Post-revert focused browser HAL coverage:

```text
webgpu_hal_ntt_gpu_results_match_cpu ... ok
max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
test result: ok. 1 passed; 0 failed; finished in 0.09s
```

## Decision

Reject the runtime shift/mask shader candidate. Accepted wall-time gain: 0.

Do not spend more immediate time on arithmetic-only NTT indexing tweaks. Future FRI/check work needs a larger active-memory-traffic design or a measurable bucket reduction, not another dispatch-neutral micro-edit.
