Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6c iter 3 — composite_to_succinct lift pipelining (RETRO)`
DraftedAt: `2026-05-13`
Status: **PARKED with documented structural reason**

## What we tried

Pipeline the lift loop in `composite_to_succinct_async` so lift N+1's CPU
witgen + accumulate overlaps with lift N's GPU finalize.

```rust
let mut continuation_receipt = None;
for right in composite_receipt.segments.iter() {
    let lifted = self.lift_async(right).await?;  // strictly serial
    continuation_receipt = Some(match continuation_receipt {
        Some(left) => self.join_async(&left, &lifted).await?,
        None => lifted,
    });
}
```

## What blocks meaningful overlap

**Single-thread wasm + single GPUDevice = structural ceiling.**

1. **GPU queue is single-stream.** All `queue.submit()` calls go through one
   `web_sys::GpuQueue`. The driver serializes them. Running N concurrent
   lifts on one device cannot dispatch more work per unit time than the
   queue can absorb.

2. **CPU is single-thread.** `witgen.accum` (~200 ms for recursion,
   ~250 ms for rv32im) is synchronous CPU code with no yield points. While
   it runs, no other future can make progress. The only overlap available
   is during GPU-drain awaits inside `commit_group_async` /
   `finalize_async`.

3. **HAL state is `Rc<RefCell<…>>`.** `WebGpuHal` has Cells/RefCells for
   diagnostics, pipeline cache, `gpu_authoritative` mode. Calling
   `prove_segment_core_async` or `lift_async` on shared `&self` across
   concurrent futures (`futures::try_join_all`, `spawn_local` + channel)
   would race on these borrows whenever the polled future holds a borrow
   across an `await` — observed historically as Cell mutation across
   suspension points. The current code is single-flow safe; making it
   concurrent-flow safe is a non-trivial audit.

4. **Per-lift compute is small relative to the GPU's compute headroom.**
   nvidia-smi shows the 5090 at 12.6% mean utilization. Even on
   multi-segment keccak_union_small (4 segments, 4 lifts), single-queue
   pipelining inside one device CANNOT bring the GPU close to saturation —
   the queue can't absorb more than one stream of work, and the dominant
   per-lift cost (`finalize_async fri_prove` ≈ 938 ms, dominated by the
   Poseidon2 merkle floor per `project_sp6a_poseidon2_ceiling`) is itself
   on the GPU compute floor.

## Maximum theoretical iter-3-as-designed win

If we perfectly hide CPU witgen_accum (~456 ms for one segment+lift pair)
behind GPU work of the prior lift, we save ~456 ms per (segment, lift)
pair. For keccak_union_small (4 segments + 4 lifts):
- max saving: 3 × ~450 ms = ~1.35 s out of 107 s wall = **1.3% reduction**

For single-segment fixtures: 0 lifts to pipeline, no saving.

## Why we ship it as parked, not as a smaller patch

The smaller patch (hoist `segment_preflight` outside the loop) was
implemented and reverted because it changes hook ordering: `on_pre_prove_segment`
would fire for ALL segments before any prove starts, rather than once
per segment immediately before its prove. Tests in
`risc0/zkvm/src/host/server/prove/tests.rs:360` track hook ordering. The
behavioral change isn't covered by the iter-3 thesis and is risky.

The bigger redesign (split `commit_group_async` into submit + await,
attempt `try_join_all` across lifts) is blocked by point (3) above and
needs a HAL audit + concurrency-safe RefCell discipline.

## What unblocks this

**SP6d (Web Workers + per-worker GPUDevice)** lifts both single-stream
limits (point 1) and single-thread limits (point 2). Each Worker has its
own `web_sys::GpuQueue` (so multi-device queue parallelism), and runs in
its own JS thread (so witgen.accum on Worker A can overlap finalize on
Worker B). The HAL borrow problem (point 3) doesn't apply because each
Worker has its own HAL instance.

Until SP6d lands, single-device pipelining wins are < 2% on the smoke
matrix and below measurement noise. The honest call is to ship iter 3
as PARKED with this reasoning, and route the effort into SP6d.

## Closing measurements (post iter 2, no further changes for iter 3)

| Fixture | wall_ms | gpu_idle_ratio |
|---|---:|---:|
| poseidon2_basic | 3219 | 0.343 |
| libm | 3205 | 0.341 |
| keccak_union_small | 107138 | 0.353 |

No regression. The 34–35% idle is the structural floor for single-device
single-thread WebGPU; further reduction requires multi-device concurrency.
