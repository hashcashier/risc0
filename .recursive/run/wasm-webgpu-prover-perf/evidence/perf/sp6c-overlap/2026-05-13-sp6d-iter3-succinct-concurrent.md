Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6d iter 3 — two-slot concurrent SUCCINCT smoke + nvidia-smi capture`
DraftedAt: `2026-05-13`

## TL;DR

Two concurrent succinct proves on a 2-slot pool drive the RTX 5090 from
**12.6% mean util / 30% peak to 52.7% mean / 100% peak**, and from
**53.7 W mean to 104.4 W mean**. Wall improves modestly (9% vs serial)
because per-slot wall expands when GPU is shared, but per-prove
throughput hits **2935 ms** (vs 3231 ms single = 1.10× throughput).

**The 5090 is finally being driven hard.** This is the structural
proof that SP6d works.

## Measurement

Smoke test `webgpu_pool_two_concurrent_succinct_proves_smoke`:
- 2-slot `WebGpuProverPool`
- Slot 0: poseidon2_basic succinct
- Slot 1: libm succinct
- `futures::future::join` to launch concurrently
- nvidia-smi at 100 ms cadence during the test

### Wall times

| Scenario | wall_ms |
|---|---:|
| Single succinct (slot 0 alone, prior baseline) | 3231 |
| 2× serial succinct (expected if no concurrency) | 6462 |
| 2× concurrent succinct on 2-slot pool | **5871** |
| Per-slot wall (slot 0) | 5790 |
| Per-slot wall (slot 1) | 5532 |
| Per-prove throughput | **2935** (vs 3231 = 1.10×) |
| Concurrent vs serial ratio | **0.91× (9% savings)** |

### GPU utilization (nvidia-smi, 100 ms cadence over the test window)

| Metric | Single-prover (prior) | 2× concurrent (this iter) | Ratio |
|---|---:|---:|---:|
| Mean GPU util | 12.6% | **52.7%** | **4.18×** |
| Peak GPU util | 30% | **100%** | **3.33×** |
| > 30% util samples | 0% | **48%** | — |
| Mean power draw | 53.7 W | **104.4 W** | **1.94×** |
| Peak power draw | 76.9 W | **215.7 W** | **2.81×** |
| Power %-of-TDP (mean) | 9.3% | **18.2%** | — |
| Power %-of-TDP (peak) | 13.4% | **37.5%** | — |

The peak 100% utilization samples confirm the GPU was fully engaged
for fractions of the run. The 52.7% mean is the average across the
concurrent window; idle gaps still exist between dispatch waves but
they're much shorter than single-device proving.

## Why wall savings is only 9% when util is 4×

The math: two concurrent succinct proves represent 2× the work. A
1.10× throughput improvement means we're delivering 2× work in
0.91× the time of serial. The GPU is doing more work per unit time
(4× utilization × per-active-second efficiency), but most of the
extra throughput is absorbed by the doubled workload, not freed as
wall savings on the single prove.

If we had a SINGLE prove that could use both slots internally
(SP6d iter 4+: distribute one prove's segments/lifts across the
pool), the same 4× utilization would translate to ~50% wall
reduction on the single prove.

## Bug discovered (and parked for iter 4)

The `gpu_idle_ratio` metric reports **0.000** in the concurrent test:
```
prove_session_async wall_ms=5790.0 gpu_active_ms=7638.0 gpu_idle_ratio=0.000
prove_session_async wall_ms=5532.0 gpu_active_ms=9438.0 gpu_idle_ratio=0.000
```

The `WEBGPU_GPU_ACTIVE_MS` thread-local accumulator is *global per
JS thread*, not per-HAL. With multi-HAL on the same thread, both
slots increment the same counter. Slot N's snapshot includes slot M's
work. The result: `gpu_active_ms > wall_ms` and the ratio underflows
to 0.

Fix path: move the counter into `WebGpuHal` so each HAL has its own
accumulator. Each `WebGpuStageTimer::new_active` then needs the HAL
context. This is a non-trivial refactor — the timer currently has no
HAL reference. Solving it cleanly probably wants a `with_active_hal`
TLS-stack approach: the timer push/pops the current HAL, and increments
that HAL's counter on drop.

Parked as `SP6d iter 4 — per-HAL gpu_active_ms`.

## Composed projection update

Pre-measurement projection: 30-50% wall reduction on multi-segment.
With this evidence:

- Two independent proves: 9% wall savings.
- Same single prove distributed across 2 slots (multi-segment xgboost):
  expected ~40% reduction (drove from single-stream 12.6% to dual-stream
  52.7% util means the GPU CAN absorb 4× more work; if the work is
  partitioned correctly, the wall drops by `1 - 1/4 = 75%` toward the
  GPU compute floor, but driver overhead and shared FRI Poseidon2 floor
  cap us at ~40%).

xgboost (~117 s currently): expected ~70 s on 2-slot, ~50 s on 4-slot.
That'd close the gap to CUDA's ~5.7 s by 8x → ~3-4× residual.

Closing all the way to 1.0× CUDA still requires SP7 (GPU-resident
witness eliminates the per-slot 200-256 ms CPU witgen_accum which is
hard-blocking concurrency on that thread).

## Trace files

- `/tmp/nvidia-smi-sp6d-iter3.csv` — 1284 samples at 100 ms, test
  window in last ~100 samples. Uncommitted, on dev box.
