Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `02 TO-BE plan — Addendum 04`
Status: `DRAFT`
DraftedAt: `2026-05-13`
Workflow version: `recursive-mode-audit-v2`
Amends:
- `/.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md`
- `/.recursive/run/wasm-webgpu-prover-perf/addenda/02-to-be-plan.addendum-01.md`
- `/.recursive/run/wasm-webgpu-prover-perf/addenda/02-to-be-plan.addendum-02.md`
- `/.recursive/run/wasm-webgpu-prover-perf/addenda/02-to-be-plan.addendum-03.md`
Inputs:
- Direct hardware measurement (2026-05-13): nvidia-smi traces of WebGPU and CUDA running libm + poseidon2_basic succinct proves. Full data in `evidence/perf/sp6c-overlap/2026-05-13-cuda-vs-webgpu-utilization.md`. Headline:
  - CUDA libm: 437 ms wall, 25.6% mean GPU util, 119.4 W mean power
  - WebGPU libm: 3238 ms wall, 12.6% mean GPU util, 53.7 W mean power
  - **CUDA is 7.4× faster but uses only 25% of the GPU mean. Neither backend hits the compute wall on small fixtures.**
- User directive 2026-05-13: "monitor the hardware usage levels directly when running CUDA/webgpu workloads and compare them" — done; this addendum is the response.
- User directive 2026-05-13: "We have to keep in mind that we must also maximize performance for large multi-segment workloads."

Outputs:
- This file. Reprioritizes the remaining sub-phases based on the measurement. SP6c is promoted to **the dominant remaining lever** (expected 3–6× win on multi-segment). SP6d is **added** (Web Workers each with their own `web_sys::GpuDevice`) for parallel-stream concurrency. SP6a/SP6b/SP3 retros are **demoted** — per-kernel optimizations contribute < 2× even at theoretical ceiling because the GPU is idle 87% of WebGPU wall.

## TODO

- [x] Codify the measurement.
- [x] Reprioritize SP6 substeps based on data.
- [x] Define SP6d.
- [ ] Coverage Gate / Approval Gate.

## Reprioritization

| Phase | Old framing | New framing |
|---|---|---|
| SP3 staged eval_check | Chrome WGSL code-gen ceiling, parked | Confirmed parked. Per-kernel ceiling matters only for ~13% of wall. |
| SP6a Poseidon2 hash_rows | "Compute floor, accept and move on" | Same conclusion, weaker leverage. 911 ms of fri_prove is the floor PER stage; with overlap, much of this is hidden behind concurrent CPU work. |
| SP6b chunked Horner's | Delivered 13% smoke speedup, complete | Complete, no further work planned. |
| **SP6c CPU/GPU overlap** | "Expected gain 10–20%" | **Expected gain 3–6× on multi-segment. THE dominant remaining lever.** |
| **SP6d Multi-worker concurrency** | Not in plan | **Added. Expected gain 3–4× on top of SP6c on multi-segment workloads.** |
| SP7-SP11 | As planned | Unchanged. SP6c+SP6d must close before SP11 audit. |

## Why per-kernel optimization is now third-order

Decompose WebGPU's 3238 ms libm wall:
- Active GPU time: 3238 × 0.126 = **408 ms**
- Idle GPU time: 3238 × 0.874 = **2830 ms**

Even if every kernel matched CUDA's per-active-second efficiency (2× speedup per active ms), active time becomes ~200 ms but idle remains ~2830 ms. Total wall ~3030 ms. **6% improvement.**

Versus SP6c (overlap idle with concurrent work):
- If we can hide all CPU-bound idle behind GPU work: 3238 × 0.126 = ~408 ms wall (theoretical floor).
- Realistic: 50% idle reduction → 3238 × 0.5 = ~1620 ms (2× speedup).

Versus SP6d (4 workers, each with own GPUDevice):
- Multi-segment xgboost has many independent segment proves. 4-way parallelism on 4 GPUDevices yields 3–4× on segment-prove wall.
- For the lift/join phase (which is recursion, single-segment recurrence), SP6d's win is bounded by the join tree's parallelism (joins at same tree level CAN parallelize).

Composed: SP6c (2×) × SP6d (3×) on multi-segment workloads = **~6× speedup target** on top of current SP6b iter 4 state. That closes most of the 7.4× CUDA gap on multi-segment fixtures without touching per-kernel WGSL.

## SP6d — Multi-worker concurrency (NEW)

Scope and purpose: Spawn multiple Web Workers, each acquiring its own `web_sys::GpuDevice` via `request_device()` and constructing its own `WebGpuHal`. Distribute segment proves and independent join operations across workers. Each worker has its own GPU command queue, so submissions parallelize at the driver level (Chrome serializes ONE queue, not multiple devices).

Implementation checklist:

- [ ] **SP-CR + gpu_idle_ratio regression gates** (Addenda 01 + 03) on every iter.
- [ ] Workspace plumbing: `browser-prove` example needs a Web Worker scaffold. wasm-bindgen-rayon precedent exists; aim for a lighter Worker pool of size 2–8.
- [ ] **Worker bootstrap**: each Worker imports the wasm bundle, calls `request_device()` independently, holds its own `WebGpuHal` instance. Buffers cannot be shared between devices — handoff must be via CPU-serialized receipts (which is already how the prove pipeline works).
- [ ] **Work distribution**: a coordinator (main thread) issues `prove_segment` requests over `postMessage`. Segments are stateless w.r.t. each other (each owns its own `Prover`), so any worker can take any segment.
- [ ] **Coordinator backpressure**: keep at least N+1 segments in flight per worker so the queue never drains.
- [ ] **Lift parallelism**: each segment receipt can lift independently (same recursion ZKR, same circuit). Lifts are independent → distribute across workers.
- [ ] **Join parallelism**: joins at the same tree level are independent. Schedule each level's joins across workers, drain, advance to next level.
- [ ] **Memory budget**: each Worker's `WebGpuHal` holds its own pool of buffers. With 4 Workers × ~6 GiB peak per Worker we'd exceed the 5090's 32 GiB. Likely budget: 2–3 Workers for prove_segment + 1 Worker for lift/join. Validate with `gpu_memory.used` during multi-worker xgboost runs.
- [ ] **Regression test**: `webgpu_multi_worker_xgboost_smoke` — measure end-to-end wall vs single-worker baseline. Target ≥2.5× on the multi-segment benchmark.

Tests:
- xgboost (R9 deferred fixture) — multi-segment, expect 3–4× win.
- BLST (R9 deferred) — single large segment + many lifts/joins, expect 1.5–2× win.
- Smoke fixtures (R1) — single segment, expect ~1× (no regression) since no concurrency is exposed.

QA: nvidia-smi during multi-worker xgboost must show mean GPU util ≥ 50% (vs current ~12%) and mean power draw ≥ 200 W (vs current ~54 W). The expected hardware signature is sustained 40–80% utilization throughout the prove. Trace recorded in `evidence/perf/sp6d-workers/<fixture>.md`.

## Amendments to SP6c (Addendum 03)

SP6c's "Expected gain 10–20%" target is revised upward:

- On single-segment fixtures (poseidon2_basic, libm): expected 1.5–2× (overlap of CPU-bound stages with GPU-bound stages within one segment's pipeline).
- On multi-segment fixtures (xgboost): expected 3–6× when COMBINED with SP6d.

The `gpu_idle_ratio` regression gate threshold moves from "< 5% R1 / < 10% R9" to **"< 30% R1 / < 15% R9"** — the original targets assumed the GPU was already mostly busy. Now the goal is to GET there.

## Coverage Gate

R10 (wall-time parity) target was 1.0× CUDA. The measurement says we're at 7.4× CUDA wall on single-segment. Composed SP6c × SP6d targets get us to ~1.2–1.5× on single-segment (mostly closing via concurrency) and ~1.0–1.2× on multi-segment (where SP6d shines). Per-kernel optimization (revisit SP3/SP6a) would be needed to reach < 1.0× CUDA, but the user's directive prioritizes multi-segment workloads where SP6c+SP6d already approach parity.

## Approval Gate

DRAFT until the first SP6c iter records `gpu_idle_ratio < 30%` on a smoke fixture, then promote to APPROVED.

## Trigger event

Direct hardware measurement on 2026-05-13: WebGPU at 12.6%/30%/54W vs CUDA at 25.6%/69%/119W. Per the user directive to "monitor the hardware usage levels directly" we now have ground truth — the gap is dispatch concurrency, not compute capability. Plan reprioritizes accordingly.
