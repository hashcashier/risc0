Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6d iter 9 — dependency-graph scheduler + heterogeneous-overlap benchmark`
DraftedAt: `2026-05-14`
Status: `COMPLETE`

## TL;DR

The dependency-graph scheduler delivers the **first positive
wall-time result** from SP6d concurrency: a 2-slot pool runs the mixed
`KeccakUnion(3)` workload **~8% faster** than a 1-slot baseline —
confirmed across two same-session A/B trials (ratio 0.923 and 0.918).
Same code, same fixture, same browser session; the only variable is
pool width.

This does **not** overturn the iter-8 negative result; it sharpens it:

- **Homogeneous concurrency** (keccak∥keccak, lift∥lift on one
  circuit) — flat (iter 8). Identical proofs contend the same
  resources at the same time.
- **Heterogeneous concurrency** (a mixed job graph: keccak proofs,
  union proves, segment lifts, joins all in flight) — a real ~7.7%
  win. Different circuits have different CPU/GPU balance, so
  overlapping them genuinely uses CPU and GPU at the same time.
- **Segment∥anything** — impossible at po2_18: wasm32's 32-bit
  address space has no room for two full-size proves at once.

The user's hypothesis (heterogeneous overlap is where concurrency
pays) is **partially validated**.

## Why iter 9

The SP6d A/B tests through iter 8 used **homogeneous** workloads —
17 independent keccak proofs, or xgboost lift+join. Both came out
wall-flat. Those tests could not rule out the case where concurrency
*should* help: a **heterogeneous** job mix where overlapping tasks
stress different resources.

Iter 9 builds the test:
1. `WebGpuProverPool::prove_with_ctx_scheduled_async` — a full
   dependency-graph scheduler. Segment proves, keccak proves, the
   keccak union build, segment lifts, and join-tree nodes are all
   schedulable; ready tasks are assigned to free pool slots as
   dependencies resolve. A 1-slot pool runs it strictly serially.
2. `webgpu_pool_scheduled_serial_vs_pool_smoke` — runs a scaled-up
   `KeccakUnion(3)` (11 rv32im segments + 25 pending keccak proofs +
   a 24-node union tree + a resolve) through the scheduler at 1-slot
   then 2-slot, in one browser session.

## Finding 1: wasm32 address space forbids segment∥keccak overlap

The first scheduler build admitted any ready task to any free slot,
gated only by "at most 1 segment in flight." The 1-slot run completed
(432 385 ms, verifies); the **2-slot run crashed** with
`RuntimeError: unreachable` — at the moment segment 2 hit its
`commit_group_async rv32im_accum` allocation (~1.8 GiB) while a keccak
proof was concurrently in `finalize_async fri_prove` with buffers
resident. 1 segment + 1 keccak exceeded wasm32's ~2 GiB usable
address space.

**This is a binding constraint, and it is not the GPU.** A po2_18
rv32im segment prove cannot share wasm32's 32-bit address space with
*any* other prove. The fix: a segment runs strictly **alone** —
admitted only when nothing else is in flight, blocking all other
admissions while it runs. Light tasks (keccak proofs, lifts, joins,
the union build) are individually smaller and fill the pool up to
`pool_size` among themselves.

## Finding 2: heterogeneous concurrency is a real ~7.7% win

With "segment alone" enforced, the 2-slot scheduler can only overlap
*light* tasks — but unlike iter-8's homogeneous tests, the
post-segment phase of `KeccakUnion(3)` is a **rich mix**: 25 keccak
proofs (each = keccak circuit + an internal recursion lift), 24 union
proves, 11 segment lifts, 10 joins, 1 resolve. The scheduler keeps
two of these in flight at once, and they are frequently *different*
task types.

### A/B result (KeccakUnion(3): 11 segments + 25 keccaks) — two trials

| Trial | 1-slot (serial) | 2-slot (concurrent) | ratio | saved |
|---|---:|---:|---:|---:|
| Run 1 | 434 336 ms | 400 895 ms | **0.923** | 33 441 ms |
| Run 2 (confirmation) | 408 915 ms | 375 385 ms | **0.918** | 33 530 ms |

**2-slot is ~8% faster, confirmed across both trials.** Well above the
0.1% noise floor measured in the iter-8 keccak A/B. The absolute walls
drift between runs (system load — earlier builds finished) but the
*ratio* is stable, because each run is a controlled same-session A/B.

### GPU utilization (nvidia-smi @ 100 ms, split by run)

| Trial / Path | mean util | peak | mean power |
|---|---:|---:|---:|
| Run 1 / 1-slot | 48.0% | 100% | 137.4 W |
| Run 1 / 2-slot | 48.8% | 100% | 139.3 W |
| Run 2 / 1-slot | 44.8% | 100% | 137.6 W |
| Run 2 / 2-slot | 47.5% | 100% | 139.3 W |

Note the util is **roughly flat** (±3 points) while wall drops ~8%.
This is the signature of CPU/GPU overlap rather than "more GPU
engagement": the GPU is busy roughly the same *fraction* of the time,
but the 2-slot run packs the same work into less wall by running one
task's GPU phase during another task's CPU phase. Contrast iter 8's
keccak case (util went 12.6%→47.5% but wall stayed flat) — there, util
rose without a wall win; here, wall falls without a real util rise.
Utilization genuinely does not predict wall time; only the A/B does.

## Why heterogeneous overlaps but homogeneous doesn't

iter 8: 2 keccak proofs concurrent — flat. Two identical proofs move
through identical phases; when both want the GPU they contend, when
both want the CPU the single JS thread serializes them. Net: no
overlap, because there is no phase diversity.

iter 9: a keccak proof (GPU-commit-heavy) overlapping a segment lift
(recursion circuit, relatively more CPU-witgen) — the lift's CPU
witgen runs on the JS thread *while* the keccak's GPU commit drains on
the device. Different circuits → different CPU/GPU ratios → the phases
are out of step → real overlap. The dependency-graph scheduler is
what makes this happen: it does not wait for the keccak phase to
finish before starting lifts/unions; it keeps the mix in flight.

## Honest scope of the win

- It is **modest** (~8%) — not the "3–4×" Addendum 04 projected.
- It is **confirmed** — two same-session A/B trials, ratios 0.923 and
  0.918, stable despite absolute-wall drift from system load.
- It does **not** come from the biggest theoretical lever — segment∥
  keccak overlap — which wasm32 forbids at po2_18.
- It is **mechanistically coherent**: a dependency scheduler over a
  heterogeneous job mix on a 2-slot pool genuinely reduces wall time
  by overlapping CPU and GPU phases of dissimilar proofs.

So SP6d is **not fully falsified**. The corrected statement:
multi-device concurrency does nothing for homogeneous GPU-bound work,
but a dependency scheduler over a *heterogeneous* mix yields a small
real win, and the larger win (segment overlap) is gated on shrinking
the per-segment memory footprint.

## What this means for SP7

SP7 (GPU-resident witness + accumulate) is doubly motivated:
1. Direct: it cuts the per-segment CPU witgen+accum time (~4–5 s of
   the ~7–10 s per segment is CPU `rust_steps`).
2. Enabling: it shrinks the per-segment CPU buffer peak — the ~1.8 GiB
   that makes a segment prove unable to share wasm32's address space.
   If a segment's resident footprint drops far enough to run
   concurrently with a keccak, the heterogeneous overlap iter 9 found
   (7.7% on light tasks) extends to the segment phase too, which is a
   much larger fraction of total wall on segment-heavy fixtures.

## Artifacts

- `risc0/zkvm/src/host/client/prove/webgpu_pool.rs` —
  `prove_with_ctx_scheduled_async` + `SchedTask` / `SchedDone` +
  `propagate_join_carries`.
- `examples/browser-prove/src/lib.rs` —
  `webgpu_pool_scheduled_serial_vs_pool_smoke`.
- Traces: `/tmp/nvidia-smi-sched-ab2.csv`, `/tmp/sched-ab2.log` (run 1);
  `/tmp/nvidia-smi-sched-ab3.csv`, `/tmp/sched-ab3.log` (confirmation).
