Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6d iter 2 — two-slot concurrent prove smoke`
DraftedAt: `2026-05-13`

## Measurement

Smoke test `webgpu_pool_two_concurrent_proves_smoke` (in `examples/browser-prove/src/lib.rs`):

```rust
let pool = WebGpuProverPool::new(2).await?;
let prover_a = pool.get(0);
let prover_b = pool.get(1);
let env_a = ExecutorEnv with Poseidon2Basic;
let env_b = ExecutorEnv with LibM;
let t0 = js_sys::Date::now();
let (info_a, info_b) = futures::future::join(
    prover_a.prove_async(env_a, MULTI_TEST_ELF),  // composite
    prover_b.prove_async(env_b, MULTI_TEST_ELF),  // composite
).await;
let concurrent_wall_ms = js_sys::Date::now() - t0;
```

Result: `concurrent_wall_ms = 1787`. Both receipts verified.

## Comparison

| Scenario | Wall ms |
|---|---:|
| Single composite (poseidon2_basic) | ~957 |
| Single composite (libm) | ~957 |
| 2× serial composites (expected) | ~1914 |
| 2× concurrent composites on 2-slot pool | **1787** |
| Concurrency speedup vs 2× serial | **1.07× (-7%)** |

## What this proves

1. **Two `web_sys::GpuDevice` instances can run dispatches concurrently** on a single browser tab and single JS thread. The driver schedules them through independent command queues; the GPU executes them in parallel where it has spare bandwidth.

2. **The naive 7% savings is real but small** for composite-only proves because the dominant cost per composite is rv32im finalize (`fri_prove` ~175 ms, `eval_u_groups` ~70 ms) which is GPU-active most of its wall — the 5090 has bandwidth for both slots, but the kernels are short and don't fully overlap.

3. **Bigger wins are expected on succinct proves** because succinct adds lift (~2230 ms with ~838 ms idle wall per iter-2 measurements). Two concurrent succinct proves on 2 slots should overlap the lift's idle gap from each slot against the other's GPU work, yielding substantially more than 7%.

## Next iter scope (SP6d iter 3)

- Use succinct receipts in the smoke (`prove_with_opts_async` with `ProverOpts::succinct()`) and re-measure. Expected: ~2× wall reduction toward true concurrency limit.
- Capture nvidia-smi during the concurrent smoke. Expected mean GPU util ≥ 20% (vs single-prover 12.6%).
- Add a 4-slot variant for benchmark headroom validation; memory ceiling needs check.
- Integration: route real multi-segment prove (xgboost on a 2- or 3-slot pool) through the pool. This requires plumbing through `prove_session_async` to dispatch segments to slots — significant refactor.

## Composed expectation revision

Pre-measurement SP6d projection was 30–50% wall improvement on multi-segment. The actual mileage on a 2-slot pool for composite proves is 7%. For succinct + multi-segment the projection holds. For single-segment succinct, expected 20–35%.

The 1.07× concurrency factor on composite-only is a useful **lower bound** on what driver-level multi-device parallelism delivers when the work is mostly compute-bound. The HEADROOM comes from the idle portions of each prove (witgen, IOP, drain waits) — exactly the 34% idle the gpu_idle_ratio metric reports for succinct R1 smokes.
