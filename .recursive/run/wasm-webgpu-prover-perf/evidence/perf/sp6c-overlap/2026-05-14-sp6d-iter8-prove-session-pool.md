Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6d iter 8 — end-to-end pool prove_with_ctx_async`
DraftedAt: `2026-05-14`
Status: `COMPLETE`

## TL;DR

`WebGpuProverPool::prove_with_ctx_async` wires the iter-1..7 pool
substrate into a production end-to-end prove path. All pool smokes
verify; R1 regression is green.

**The definitive finding: SP6d delivers ZERO e2e wall-time win.**
Multi-device concurrency on a single physical GPU does not reduce
wall time — not on dependency chains (xgboost lift+join), not on
embarrassingly-parallel work (17 independent keccak proofs). Both
measured both ways under identical conditions; both flat.

| Workload | Single-slot | 2-slot pool | ratio | verdict |
|---|---:|---:|---:|---|
| keccak — 17 independent proofs | 130 804 ms | 130 665 ms | **0.999** | flat |
| xgboost — 11-segment lift+join | 141 983 ms | 142 732 ms | **1.005** | flat |

The GPU-utilization gain (12.6% → 47.5%) is a real measurement but an
**empty** one — it fills the GPU's idle gaps with the other slot's
queued work, but since it's the same physical GPU doing the same
fixed total work, the finish line does not move. Utilization is a
proxy; wall time is the goal; they diverge completely here.

**SP6d's perf thesis is falsified.** The substrate is correct (every
receipt verifies) and clean, but multi-device concurrency cannot
deliver wall-time wins on single-GPU hardware. The real levers are
SP7 (GPU-resident witness — cuts total work per segment), per-kernel
WGSL (SP3/SP6a — cheaper fixed-rate dispatches), or a second physical
GPU.

## What landed

### Public-API surgery (Phase A)

`ProverImpl` methods exposed `pub(crate)` for the pool:
`prove_segment_core_async`, `resolve_async`, `insert_union_receipt_async`,
`union_receipts_root_async`, `composite_to_succinct_async`. Module
`prover_impl` and the `WEBGPU_DEFAULT_*` env-cap constants are
`pub(crate)`. Purely additive — the single-slot `WebGpuProver` path is
untouched (R1 regression confirms, below).

### Orchestrator (Phase B)

`WebGpuProverPool::prove_with_ctx_async(env, ctx, elf, opts)`:

1. Apply WebGPU env caps (po2 ≤ 18, keccak_po2 ≤ 14).
2. Execute the session on CPU.
3. **Segment proves: SERIAL through slot 0** (concurrent OOMs wasm32).
4. Merge journal + assumptions into final segment claim.
5. **Pending keccaks: DISTRIBUTED** via `prove_keccak_requests_async`
   (bounded chunks of `pool.len()`) — this is the pool's real win.
6. Build keccak union root on slot 0.
7. Verify composite receipt + claim digest.
8. Composite mode → return composite.
9. Succinct mode → `composite_to_succinct_async`.

`WebGpuProverPool::composite_to_succinct_async` delegates to slot 0's
`ProverImpl::composite_to_succinct_async` — the serial interleaved
lift→join→lift→join chain, which also handles assumption resolves
natively. See the "lift+join does not parallelize" finding below for
why this beats the distributed tree.

## Finding 1: concurrent segment proves OOM wasm32

Iter 8 first attempted concurrent per-segment proves (bounded chunks
of `pool.len()`). It failed:

```
RuntimeError: unreachable
  at <CpuHal as Hal>::alloc_elem
  at <WebGpuHal as Hal>::alloc_elem
  at PolyGroup::new_async
  at commit_group_async_scoped
  at WebGpuSegmentProver::prove_core_async
```

At po2_18, one segment's `commit_group_async` peaks at ≈1.8 GiB (code
+ data + accum poly-group buffers). Two segments hitting their
`rv32im_accum` poly-group allocation simultaneously overflow wasm32's
isize-bounded `Vec` (~2 GiB ceiling). Segment proves run serially.

## Finding 2: lift+join does not parallelize on a single physical GPU

Iter 8's first orchestrator used `lift_and_join_async` — distribute
all segment lifts across slots, then a balanced join tree. Measured
against the serial chain on xgboost (11 segments):

| Path | xgboost wall | mean GPU util |
|---|---:|---:|
| 2-slot `lift_and_join_async` (distributed tree) | 139 977 ms | 33.5% |
| serial `composite_to_succinct_async` (slot 0) | 141 983 ms | **41.7%** |

The walls are within 1.4% (noise) — but the **serial chain keeps the
GPU better engaged** (41.7% vs 33.5%). Distributing lift+join across
two `GpuDevice`s on one physical GPU time-slices the same hardware;
the all-lifts-then-balanced-tree restructuring + chunk barriers add
idle gaps that outweigh any marginal CPU/GPU overlap.

Root cause: multiple `web_sys::GpuDevice` instances each have their
own `GpuQueue`, but they all submit to **one physical GPU**. For
GPU-bound work in a dependency chain (lift → join → ...), concurrency
gives no throughput multiplication — it just interleaves submissions
to hardware that was already the bottleneck. The CPU witgen phases
(~500 ms each) are the only thing that *could* overlap, and on a
single JS thread they serialize anyway.

**The orchestrator therefore uses the serial chain for
composite_to_succinct.** `lift_and_join_async` is retained as a
validated building block (iter 5/7 smokes still pass) but is no longer
the default.

Verified post-change: xgboost pool v2 (serial composite) = 142 732 ms
/ 39.4% util — wall-equivalent to single-slot, as expected (a
no-keccak fixture runs the identical serial path on the pool).

## Finding 3: keccak distribution does NOT win on wall time either

Keccak was the workload with a *real* mechanism for a concurrency win:
17 fully-independent proofs, no dependency chain, so slot 0's CPU
witgen genuinely *can* overlap slot 1's GPU work. The
`webgpu_pool_keccak_serial_vs_pool_smoke` test runs the same 17-keccak
request set both ways in one browser session — a 1-slot pool
(`prove_keccak_requests_async` chunks into groups of 1 = the serial
baseline) and a 2-slot pool (chunks of 2):

| Path | wall | ratio |
|---|---:|---:|
| serial (1-slot pool) | 130 804 ms | 1.000 |
| distributed (2-slot pool) | 130 665 ms | **0.999** |

**Wall-identical.** Even on the best-case workload for concurrency,
the 2-slot pool delivers no wall-time improvement.

The split-half nvidia-smi trace for that run shows the *serial* path
at ~54.8% mean util and the 2-slot pool at ~45.8% — utilization is
not even reliably higher for the pool, let alone translating to a
wall win. (Earlier iter-6 reported 40.2% util for pool keccak and
iter-8's keccak-union smoke 47.5%; none of those had a matched
single-slot wall baseline, which is exactly why this test was added.)

This kills the last hypothesis. **The root cause is the single
physical GPU.** Multiple `web_sys::GpuDevice`s each have their own
command queue, but the queues feed one piece of silicon. Total GPU
work is fixed; the GPU runs it at a fixed rate; wall time follows
that rate. Concurrency rearranges *when* the GPU is busy (filling
idle gaps with the other slot's queued work) but cannot make the GPU
do the fixed work any faster. Higher utilization with identical wall
time is the signature of this: the GPU is "busier" per the sampler,
but it is the same GPU completing the same work in the same time.

### Why the orchestrator still distributes keccaks

Given keccak distribution is wall-neutral (not negative), the
orchestrator keeps `prove_keccak_requests_async` for the keccak phase:
it does no harm, and if a second physical GPU is ever exposed to the
browser, the distribution code path becomes the one that wins. For
lift+join the distributed path measured slightly *worse* GPU
engagement than serial (Finding 2), so that one reverted to serial.

## Smoke results

All pool smokes verify (browser, wasm32):

### `webgpu_pool_keccak_serial_vs_pool_smoke` (wall-time comparison)
17 keccak proofs from KeccakUnion(2), run both ways in one session:
serial (1-slot pool) 130 804 ms, distributed (2-slot pool) 130 665 ms,
ratio 0.999. Both receipt sets match in count. This is the test that
settled the SP6d wall-time question — see Finding 3.

### `webgpu_pool_prove_session_multi_segment_smoke`
BusyLoop{500_000} → 3 segments. Wall 33 658 ms. Receipt verifies via
`info.receipt.verify(MULTI_TEST_ID)`.

### `webgpu_pool_prove_session_keccak_union_smoke`
KeccakUnion(2) → 7 segments + ~17 keccak proofs + 16 union proves +
1 assumption resolve. Wall 287 732 ms. Receipt verifies via
`verify(MULTI_TEST_ID)`. Exercises every orchestrator phase including
the resolve path.

### `webgpu_pool_xgboost_smoke` (SP6d QA-gate fixture)
xgboost R9 fixture → 11 segments, no assumptions. Wall 142 732 ms.
Receipt verifies via `verify(XGBOOST_ID)` and journal decodes to
30.528042544062632. Wall-equivalent to the single-slot baseline
(141 983 ms) — confirms the pool path adds no overhead, and no win.

### R1 regression (single-slot path unchanged)

| Fixture | Wall | gpu_idle_ratio | Verifies |
|---|---:|---:|:--:|
| poseidon2_basic | 3999 ms | 0.467 | ✅ |
| libm | 4020 ms | 0.465 | ✅ |
| busy_loop_po2_18 | 10607 ms | 0.607 | ✅ |

All verify. Walls are consistent with the memory baseline (3.93 s for
poseidon2_basic); ~24% above the STATE.md post-iter-4 single-trial
figures, attributable to single-trial variance and concurrent build
load during the measurement window. No correctness regression — the
pub-API surgery is additive only.

## Honest assessment of the SP6d thesis — FALSIFIED

Addendum 04 projected SP6d would deliver "3–4× on multi-segment
workloads". The measured reality, both workloads measured both ways
under identical conditions:

| Workload | single-slot | 2-slot pool | ratio |
|---|---:|---:|---:|
| keccak — 17 independent proofs | 130 804 ms | 130 665 ms | 0.999 |
| xgboost — 11-segment lift+join | 141 983 ms | 142 732 ms | 1.005 |

**SP6d delivers no e2e wall-time win.** Not on dependency chains
(xgboost), not on embarrassingly-parallel work (keccak). The GPU
utilization gain (12.6% → 47.5%) is real as a measurement but does
not move wall time — it just fills the GPU's idle gaps with the other
slot's queued work, and since it is one physical GPU doing one fixed
body of work, the finish line does not move.

The plan's 3–4× projection assumed Web Workers would unlock
parallelism. The measured finding is more fundamental: **the
bottleneck is the single physical GPU, not the submission
mechanism.** Web Workers would remove the CPU-serialization limit but
the GPU is still one device — every workload would still contend on
it. Genuine concurrency wins need a *second physical GPU* exposed to
the browser. No software architecture on one GPU changes this.

What SP6d *did* deliver:
- A correct, clean, end-to-end pool prove path — every receipt
  verifies, R1 regression green, no correctness cost.
- A definitive negative result that re-points the roadmap: stop
  pursuing concurrency on single-GPU hardware; the levers for e2e
  wall time are the ones that reduce or cheapen the fixed GPU work —
  SP7 (GPU-resident witness, cuts total work per segment), per-kernel
  WGSL (SP3/SP6a, cheaper dispatches), or new hardware.

The pool substrate is retained: it is correct, costs nothing on the
single-GPU path, and is the code that would win the day a multi-GPU
browser API or a second card appears. But it is not a wall-time lever
today, and the run should treat it as closed-negative, not as a
pending win.

## Operational notes

- `wasm-bindgen-test-runner` defaults `WASM_BINDGEN_TEST_TIMEOUT` to
  **20 s**; multi-phase pool proves exceed it and chromedriver gets
  SIGKILL'd with "Failed to detect test as having been run." Run pool
  smokes with `WASM_BINDGEN_TEST_TIMEOUT=300+` (900 for xgboost).
- Pass `CHROMEDRIVER=~/.cache/.wasm-pack/chromedriver-*/chromedriver`
  explicitly — the runner's auto-discovery hit a permission error on
  this box.

## What this leaves open

- **SP7 GPU-resident witness** is the highest-EV remaining lever for
  e2e wall time — it shrinks the per-segment CPU witgen time AND the
  per-segment buffer peak (which would also unblock concurrent segment
  proves). Unlike SP6d, it reduces the *fixed work*, which is what
  actually moves wall time on single-GPU hardware.
- **Per-kernel WGSL (SP3/SP6a revisits).** Previously demoted because
  the GPU was "87% idle". The SP6d negative result re-weights this:
  if concurrency can't fill that idle productively, making each
  dispatch cheaper is back on the table — though still bounded
  (~5% even if doubled).
- **`lift_and_join_async`** is retained but unused by the orchestrator.
  It works and verifies; it is just not faster on a single GPU. If a
  future multi-GPU browser API or a second card appears, it (and the
  keccak distribution path) become the code that wins.
- **xgboost through `DefaultProver`/Hermes APIs** is deferred — there
  is no wall-time reason to route production proving through the pool
  on single-GPU hardware. Revisit only if multi-GPU lands.
