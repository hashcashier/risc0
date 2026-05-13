Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6c iter 0 — hardware utilization baseline`
DraftedAt: `2026-05-13`
Workload: `MultiTestSpec::LibM` succinct receipt, WebGPU prover (Chrome/Dawn, RTX 5090)

## Capture method

- `nvidia-smi --query-gpu=timestamp,utilization.gpu,utilization.memory,memory.used,memory.free,temperature.gpu,power.draw --format=csv,noheader -lms 200`
- 200 ms cadence, 617 samples, total wall ≈ 123 s
- Captured during `cargo test --target wasm32-unknown-unknown --release -p browser-prove webgpu_libm_native_async_succinct_receipt_verify` (SP6b iter 4 validation run)
- Raw CSV: `/tmp/nvidia-smi-libm.csv` (not committed)

## Headline numbers

| Metric | Value |
|---|---|
| Wall time | ~123 s (test harness + prove + verify) |
| Prove `prove_session_async` elapsed | 3238 ms |
| GPU utilization — mean | **12.6 %** |
| GPU utilization — max | **30 %** |
| Power draw — mean | **53.7 W** |
| Power draw — max | **76.9 W** |
| 5090 TDP | 575 W |
| Power envelope utilized | **9.3 % mean / 13.4 % peak** |

## Histogram (GPU util %)

| Band | Samples | % of run |
|---|---:|---:|
| 0% | 29 | 4.7% |
| 1–5% | 40 | 6.5% |
| 6–10% | 183 | 29.7% |
| 11–20% | 310 | 50.2% |
| 21–30% | 55 | 8.9% |
| 31–100% | 0 | 0% |

**The GPU never crossed 30% utilization for the entire run.**

## What this means

The WebGPU prover on this fixture is **submission-bound, not compute-bound**. The 5090's compute throughput is engaged at ~1/8 of capacity. Per-kernel optimization (workgroup_size tuning, SBox encoding, etc — SP6a's revisits) cannot move the needle because most wall time is spent OUTSIDE GPU kernels: dispatch latency, Dawn command-buffer encoding, CPU bookkeeping between dispatches, and submission queue serialization.

This validates the SP6c thesis (Addendum 03) but multiplies its expected impact: we are not extracting 5–10% of headroom — we are extracting 5–10× headroom *if* we can keep the queue saturated.

## Confidence caveats

- 200 ms sampling cadence will alias against bursty <100 ms compute kernels. The TRUE peak utilization is higher than 30%; the TRUE mean is in the same neighborhood because most wall time is non-GPU CPU bookkeeping.
- nvidia-smi reports utilization as "fraction of time at least one kernel was active on the GPU during the sampling window", not "SM occupancy". 12.6% means the GPU was active ~12% of the wall, NOT that the active kernels engaged 12% of the SMs.
- Power draw is the more sensitive indicator of SM engagement. 53.7 W / 575 W = 9.3% suggests both submission AND occupancy are well below ceiling.

## Next: CUDA comparison

Need to capture the same trace on a native CUDA prove of the same fixture (`cuda_baseline_libm_succinct`, added 2026-05-13 to `risc0/zkvm/src/host/server/prove/tests.rs`). Compare:
1. Mean GPU util %  — expectation: CUDA much higher, possibly 60–90% if FRI/Poseidon2 kernels saturate
2. Peak power draw — expectation: CUDA approaches 300–400 W during FRI rounds
3. Wall time — expectation: CUDA likely 2–4× faster end-to-end (which combined with higher util means a 5–10× throughput gap per second of wall)

The headline product of the comparison: CUDA-wall × CUDA-util-mean vs WebGPU-wall × WebGPU-util-mean is the actual "GPU-seconds delivered" ratio. That is the real gap to close, and the GPU-idle-ratio metric in Addendum 03 captures it.
