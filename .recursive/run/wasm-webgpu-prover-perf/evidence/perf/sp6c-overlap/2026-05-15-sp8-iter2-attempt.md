Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP8 iter 2 — overlap check_group batch_evaluate with poly_interpolate`
Date: 2026-05-15

## Question

The SP10 retro projected "~1-3s per xgboost" from coalescing the
check_group readback with poly_interpolate's CPU work. Does the
straightforward refactor (dispatch check_group BEFORE poly_interpolate,
await readback AFTER) yield real wall-time savings?

## Method

Reordered `risc0/zkp/src/prove/prover.rs::finalize_async` so the
check_group `batch_evaluate_any_async` dispatches BEFORE the
`poly_interpolate` scope (the dispatch returns once the GPU command is
queued, not when the GPU completes the work). Then `poly_interpolate`
runs on the JS thread (writing the prefix of `coeff_u`) while the GPU
processes the check evaluation in parallel. Finally, `sync_gpu_to_cpu`
+ `coeff_u.extend` runs at the end.

Builds against the 102.6s baseline measured at e335679f9.

## Result

xgboost wall (single-prover xgboost_succinct_receipt_verifies):

| Variant | wall_ms | gpu_idle_ratio |
|---|---:|---:|
| baseline (e335679f9, pre-iter-2) | 102603 | 0.439 |
| SP8 iter-2 (this attempt) | **104279** | 0.436 |

Delta +1676ms (+1.6%) -- within the ±2s back-to-back noise band. No
meaningful win, possibly a tiny regression (extra dispatch coordination
+ borrow restructuring).

## Why it didn't work

The optimization assumes the GPU and CPU phases overlap. In practice:
1. The check_group `batch_evaluate` GPU work is SMALL (~CHECK_SIZE=512
   ext-elements). It completes faster than poly_interpolate even when
   serialized.
2. The single-threaded JS event loop cannot dispatch new GPU work or
   read GPU results while poly_interpolate is running (a sync CPU
   block). So the only overlap window is "GPU work already queued,
   completes during CPU work" -- which only saves time if the GPU work
   would have been the longer leg. Here it isn't.

Per the SP6c diagnosis (`project_webgpu_submission_bound`), the
submission overhead, not the compute, dominates. SP8 iter-2 reduces
neither submissions nor the dominant CPU work, so it should have been
flat.

## Reverted -- failed-experiment ledger entry

No code shipped. The 1.6% wall delta is inside the noise band.

The SP10 retro's "1-3s per xgboost" projection appears to have been
optimistic for this specific overlap pattern. The remaining levers
worth measuring (in decreasing expected impact):

1. **SP7 iter-6d**: GPU witgen for exec_Top chunks. ~5-10% per SP10.
2. **SP9 corrected**: pipeline+layout cache (the iter-9 SP9 attempt
   failed; needs layout-first then pipeline). ~3-5%.
3. **SP6e**: bind-group cache for the 62 per-call create_bind_group
   sites in webgpu.rs. ~2-3%.
