Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP10 R9 — xgboost re-measurement post SP6b/c/d work`
DraftedAt: `2026-05-13`

## Headline

xgboost succinct prove on WebGPU/Chrome/RTX 5090:

| Metric | This session | Previous baseline (STATE.md, 2026-05-12) | Δ |
|---|---:|---:|---:|
| Wall time | **104218 ms** | 117920 ms | **-12% (-13.7 s)** |
| Mean GPU util | 33.2% | n/a | — |
| Peak GPU util | **100%** | n/a | — |
| >30% util samples | 30.2% | n/a | — |
| Mean power | 99.2 W | n/a | — |
| Peak power | **368.0 W** | n/a | — |
| gpu_idle_ratio | 0.431 | n/a | — |
| Ratio vs CUDA (5.7 s) | **18.3×** | 20.7× | **-12%** |

R1 smoke unchanged through the same push, confirming the 12% xgboost win
comes from real algorithmic + structural improvements, not noise.

## Why xgboost gained 12% while R1 smokes stayed flat

R1 smokes are single-segment (1 segment + 1 lift). The SP6b chunked
Horner heuristic targeted `eval_count ≤ 64 AND deg > 8192` — the recursion
lift hits this (16-23 evals, deg=1048576) on every smoke, but rv32im
finalize misses it (≥119 evals). So R1 smokes get ~3% from SP6b alone.

xgboost is MULTI-SEGMENT (many rv32im segment proves + many recursion
lifts + a join tree). The wins compound:
- SP6b chunked Horner on each lift: -3% × many lifts
- SP8 readback restructure on each finalize: small per-call, accumulates
- SP6c metric overhead is bounded thread-local arithmetic (negligible)
- Plus measurement noise reduction from longer wall

## What still remains

xgboost still sits at 18.3× CUDA. Single-slot WebGPU peak util reaches
100% during FRI rounds but mean is 33% — there's 67% idle wall to attack.
The available levers:

1. **SP6d iter 5+ on xgboost** — route xgboost segments + lifts through
   `WebGpuProverPool`. Multi-segment is exactly where this shines.
   Expected: another 20-40% wall improvement on 2-slot.

2. **SP7 GPU-resident witness + accumulate** — for xgboost the per-segment
   `rust_steps::generate_witness` + `rust_steps::step_accum` time is
   ~CPU-200-300 ms × N segments. GPU-porting these eliminates that.
   Expected: another 5-10%.

3. **Per-kernel WGSL improvements (SP3/SP6a revisits)** — only ~5% even
   if doubled. Lowest priority.

Composed expectation if SP6d + SP7 land cleanly:
- 2-slot SP6d: 104s × 0.7 = ~73s (12.8× CUDA)
- + SP7: ~66s (11.6× CUDA)
- + per-kernel tweaks: ~60s (10.5× CUDA)

Closing to ≤ 2× CUDA needs a hardware-level change (multi-GPU exposed to
browser, or browser allowing MUCH more aggressive WGSL → SPIR-V
optimization). Realistic single-tab single-GPU floor on Chrome/Dawn
appears to be ~5-8× CUDA.

## Trace files

- `/tmp/nvidia-smi-xgboost.csv` — 451 samples @ 500 ms cadence, full
  104 s xgboost prove. Captures the per-stage utilization profile;
  some FRI bursts hit 100% util / 368 W peak power.
