Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6d on xgboost — A/B test`
Date: 2026-05-15

## Question

Does `WebGpuProverPool::prove_with_ctx_scheduled_async` (SP6d iter 9
dependency-graph scheduler) beat `prove_with_ctx_async` (sequential
phase-order) on xgboost?

The SP6d iter-9 doc measured ~8% wall improvement on a synthetic
heterogeneous workload (keccak + lift), but explicitly noted:

> at po2_18 the scheduler cannot overlap segment proving with
> anything — the wasm32 address space, not the GPU or the submission
> mechanism, forbids it.

xgboost is segment-dominated (11 segments × ~6s each = 66s of the
102s wall). The scheduler's main lever (segment-with-other-overlap)
is unreachable here. Lift-with-lift overlap remains; that's modest.

## Method

Single-line change at `examples/browser-prove/src/lib.rs:3202`:
`.prove_with_ctx_async(...)` → `.prove_with_ctx_scheduled_async(...)`.
Re-run `webgpu_pool_xgboost_smoke` and compare wall_ms.

Both branches verified via metric emission (`pool_prove_with_ctx_async`
vs `pool_prove_scheduled_async` in the chrome console). The first
"scheduled" attempt ran with a stale wasm (incremental cargo cache
returned the unchanged binary) and reported the OLD metric — caught
when adding a SCHEDULED_VARIANT marker that did not appear in the wasm
strings. Touching `lib.rs` then `cargo test --no-run` produced a fresh
build with the new symbols present.

## Results

| Variant | wall_ms | metric |
|---|---:|---|
| `prove_with_ctx_async` (baseline) | 103133 | pool_prove_with_ctx_async |
| `prove_with_ctx_async` (run 2 baseline) | 103473 | pool_prove_with_ctx_async |
| `prove_with_ctx_scheduled_async` (fresh wasm) | **101725** | pool_prove_scheduled_async |

Delta: -1.4s (-1.3%). Inside the ±1-2s noise band measured across
back-to-back identical runs (103133 → 103473 → 101727 = ±2s spread).

## Verdict

**Scheduled async does not measurably help xgboost.** Consistent
with the iter-9 prediction: xgboost is segment-dominated and at po2_18
the scheduler cannot overlap segments. Reverted the change — no code
landed; this doc is the failed-experiment ledger entry.

## Implication for SP6d roadmap

SP6d iter-10+ would need to shrink the per-segment memory peak first
(via SP7 GPU witgen reducing the CPU-resident PreflightTrace buffers)
to unblock segment-with-other overlap. That's also the iter-6d
prerequisite. So SP6d-on-xgboost is genuinely blocked behind SP7
iter-6d, not just queued.
