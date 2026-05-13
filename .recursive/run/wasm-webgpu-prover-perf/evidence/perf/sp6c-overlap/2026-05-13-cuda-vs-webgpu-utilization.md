Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6c iter 0 — hardware utilization A/B`
DraftedAt: `2026-05-13`
Hardware: RTX 5090 (32 GiB, Blackwell SM120), AMD CPU, CUDA 13.0, Chrome/Dawn
Workload: `MultiTestSpec::{LibM, Poseidon2Basic}` succinct receipt

## TL;DR

**CUDA is 7.4× faster than WebGPU at end-to-end prove wall time, but uses only 25% of the GPU on average.** Neither backend saturates the 5090 on these single-segment fixtures. The 5090 has 3–4× more compute headroom even for CUDA, and 6× more for WebGPU.

→ The bottleneck for both backends is **dispatch concurrency**, not per-kernel throughput. The dominant remaining lever is multi-stream parallelism (multiple segments / lifts / joins running concurrently). Per-kernel WGSL optimization is third-order.

## Measurement setup

### CUDA (native)
- Built `cargo build --release --features cuda -p risc0-zkvm --tests` (17m 06s including nvcc kernel compile + linking with cust/sppark)
- Added `cuda_baseline_libm_succinct` and `cuda_baseline_poseidon2_basic_succinct` tests to `risc0/zkvm/src/host/server/prove/tests.rs`
- Ran 10 back-to-back iterations to amortize CUDA cold-start and lengthen the measurement window for nvidia-smi:
  ```
  for i in 1..=10:
    target/release/deps/risc0_zkvm-* cuda_baseline_libm_succinct --nocapture --test-threads=1
  ```
- Monitor: `nvidia-smi --query-gpu=... -lms 50` (50 ms cadence — finer than WebGPU's 200 ms because CUDA prove is short)

### WebGPU (Chrome/Dawn)
- Branch state post SP6b iter 4 (commit 31dfdf2c0)
- Single test invocation via WebDriver:
  ```
  cargo test --target wasm32-unknown-unknown --release -p browser-prove
    webgpu_libm_native_async_succinct_receipt_verify
  ```
- Monitor: `nvidia-smi --query-gpu=... -lms 200` (200 ms cadence, prove ~3s)

## Headline numbers

### libm succinct

| Metric | WebGPU | CUDA | Ratio |
|---|---:|---:|---:|
| Wall, steady state | 3238 ms | **437 ms** | **7.4× CUDA faster** |
| Mean GPU util | 12.6 % | 25.6 % | 2.0× |
| Peak GPU util | 30 % | 69 % | 2.3× |
| Mean power draw | 53.7 W | 119.4 W | 2.2× |
| Peak power draw | 76.9 W | 151.3 W | 2.0× |
| Power %-of-TDP (mean) | 9.3 % | 20.8 % | 2.2× |
| Power %-of-TDP (peak) | 13.4 % | 26.3 % | 2.0× |

### poseidon2_basic succinct (10-iter loop on CUDA, single shot on WebGPU)

| Metric | WebGPU | CUDA | Ratio |
|---|---:|---:|---:|
| Wall, steady state | 3258 ms | **442 ms** | **7.4× CUDA faster** |
| Mean GPU util | ~12 % | 25.0 % | 2.1× |
| Peak GPU util | ~30 % | 47 % | 1.6× |
| Mean power | ~54 W | 121.7 W | 2.3× |

### Combined "GPU-seconds delivered per second of wall"
- WebGPU: 0.126 × (1 s) = 0.126 GPU-seconds/wall-second × 1/3.238 wall = **0.039 GPU-seconds delivered per second of wall-time for the prove**
- CUDA: 0.256 × (1 s) × 1/0.437 = **0.586 GPU-seconds delivered per second of wall-time**

**CUDA delivers 15× the GPU-seconds per wall-second.** That's the gap to close, decomposable into:
- 2× compute density per active second (per-kernel + dispatch overhead difference)
- 7.4× more wall efficiency (less idle queue time)

## Implications for the plan

### 1. SP3 / SP6a "Chrome code-gen ceiling" framing was misleading

The retrospective memories for SP3 (staged eval_check) and SP6a (Poseidon2 hash_rows) claimed a 30–150× compute ceiling from WGSL→SPIR-V→Vulkan code-gen quality. That framing was **per-dispatch correct but globally wrong**: the per-dispatch comparison only addresses 12.6% of WebGPU's wall — the other 87.4% is idle. Even if per-dispatch matched CUDA, total wall would shrink from 3238 → ~1620 ms (a 2× win), not the 7.4× we need.

### 2. SP6c (CPU/GPU overlap) is now THE phase

Addendum 03's SP6c was framed as "expected gain 10–20%". The data here says the expected gain is **3–6×** for multi-segment workloads where idle queue time compounds.

### 3. SP6d (Web Workers + multiple GPUDevices) becomes the second phase

Each Web Worker can hold its own `web_sys::GpuDevice` via a separate `request_device()` call. Multiple devices = multiple submission queues. Even if Chrome serializes one queue, N queues = N× concurrent submission. On a 10-segment xgboost, 4 workers each handling 2-3 segments would multiply throughput by 3–4× on top of SP6c's overlap gains.

### 4. CUDA itself is under-saturated → the compute wall is FAR away

Even CUDA peaks at 69% util / 26% power on these small fixtures. A multi-segment xgboost on CUDA would be closer to saturation, but on this poseidon2-sized workload there's headroom for ~3× more concurrent work even on CUDA. WebGPU has ~6× more.

**This means: closing the WebGPU gap to CUDA does not require reaching CUDA's per-kernel speed. It requires reaching CUDA's submission/concurrency efficiency.** That's a fundamentally easier target than nvcc-quality SPIR-V.

## Trace files (uncommitted, on dev box)
- `/tmp/nvidia-smi-libm.csv` — WebGPU libm single-shot, 200 ms cadence, 617 samples
- `/tmp/nvidia-smi-cuda-loop.csv` — CUDA libm 10-iter loop, 50 ms cadence, 116 samples
- `/tmp/nvidia-smi-cuda-p2.csv` — CUDA poseidon2_basic 10-iter loop, 50 ms cadence, 117 samples

## Next: plan amendment

Will draft Addendum 04: SP6c (overlap) promoted to dominant phase + SP6d (Web Workers with own GPUDevice) added. Per-kernel SPs (SP6a, future) deprioritized.
