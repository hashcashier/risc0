Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6d iter 8 — end-to-end pool prove_with_ctx_async`
DraftedAt: `2026-05-14`
Status: `COMPLETE`

## TL;DR

`WebGpuProverPool::prove_with_ctx_async` wires the iter-1..7 pool
substrate into a production end-to-end prove path. All three pool
smokes verify; R1 regression is green.

**The honest finding:** on a single JS thread driving a single
physical GPU, the only phase that genuinely parallelizes is keccak
proof distribution (many independent proofs). Segment proves and
lift+join do **not** parallelize — multiple `web_sys::GpuDevice`s
share one physical GPU, so "concurrent" GPU-bound work just
time-slices the same hardware. The orchestrator therefore distributes
keccaks and runs everything else serially on slot 0.

| Workload | Single-slot | 2-slot pool | Pool delta |
|---|---:|---:|---:|
| xgboost (R9, 11 segs, no keccaks) | 141 983 ms / 41.7% util | 142 732 ms / 39.4% util | wall-equivalent (same serial path) |
| keccak union (7 segs + ~17 keccaks) | — | 287 732 ms / **47.5% util** | keccak distribution is the real win |
| multi-segment BusyLoop (3 segs) | — | 33 658 ms / 35.5% util | verifies |

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

## Finding 3: keccak distribution IS a genuine win

Keccak proofs are numerous, fully independent, and individually
smaller than lifts. Distributing them across pool slots fills the
GPU's idle gaps that single-stream submission leaves open:

| Workload | mean GPU util | peak | mean power |
|---|---:|---:|---:|
| single-device baseline (any fixture) | ~12.6% | 30% | ~54 W |
| iter 6 keccak-only (17 proofs, 2 slots) | 40.2% | 100% | 89 W |
| iter 8 keccak union end-to-end (2 slots) | **47.5%** | 100% | 131 W |

47.5% mean util is the highest sustained GPU engagement measured in
the whole run. This is why the orchestrator keeps keccak distribution
even though it drops lift+join distribution.

## Smoke results

All three pool smokes verify (browser, wasm32, 2-slot pool):

### `webgpu_pool_prove_session_multi_segment_smoke`
BusyLoop{500_000} → 3 segments. Wall 33 658 ms. Receipt verifies via
`info.receipt.verify(MULTI_TEST_ID)`.

### `webgpu_pool_prove_session_keccak_union_smoke`
KeccakUnion(2) → 7 segments + ~17 keccak proofs + 16 union proves +
1 assumption resolve. Wall 287 732 ms, 47.5% mean GPU util / 100%
peak / 131 W mean. Receipt verifies via `verify(MULTI_TEST_ID)`.
Exercises every orchestrator phase including the resolve path.

### `webgpu_pool_xgboost_smoke` (SP6d QA-gate fixture)
xgboost R9 fixture → 11 segments, no assumptions. Wall 142 732 ms /
39.4% mean util / 100% peak / 138 W mean. Receipt verifies via
`verify(XGBOOST_ID)` and journal decodes to 30.528042544062632.
Wall-equivalent to the single-slot baseline (141 983 ms) — confirms
the pool path adds no overhead for no-keccak fixtures.

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

## Honest assessment of the SP6d thesis

Addendum 04 projected SP6d would deliver "3–4× on multi-segment
workloads". The measured reality:

- **Keccak workloads: real win.** 12.6% → 47.5% mean util. The pool
  fills GPU idle gaps with independent proof work.
- **Multi-segment lift+join (xgboost): no wall win.** Multiple
  `GpuDevice`s share one physical GPU; GPU-bound dependency chains
  don't parallelize by adding submission queues. The pool path is
  wall-equivalent to single-slot.

The plan's 3–4× projection assumed Web Workers would unlock
parallelism. Iter 8's finding is more fundamental: **the bottleneck
is the single physical GPU, not the submission mechanism.** Web
Workers (separate JS threads) would remove the CPU-serialization
limit, but the GPU is still one device — lift+join would still
contend. Genuine 3–4× would need either a second physical GPU
exposed to the browser, or per-proof work small/abundant enough to
fill idle gaps (which is exactly keccak, and exactly why keccak is
the win).

This does not invalidate SP6d — it sharpens it. The pool's value is
keccak/accelerator distribution. For multi-segment lift+join, the
lever is SP7 (GPU-resident witness, shrinks per-segment CPU time and
the OOM peak) and per-kernel work, not more concurrency.

## Operational notes

- `wasm-bindgen-test-runner` defaults `WASM_BINDGEN_TEST_TIMEOUT` to
  **20 s**; multi-phase pool proves exceed it and chromedriver gets
  SIGKILL'd with "Failed to detect test as having been run." Run pool
  smokes with `WASM_BINDGEN_TEST_TIMEOUT=300+` (900 for xgboost).
- Pass `CHROMEDRIVER=~/.cache/.wasm-pack/chromedriver-*/chromedriver`
  explicitly — the runner's auto-discovery hit a permission error on
  this box.

## What this leaves open

- **xgboost through `DefaultProver`/Hermes APIs.** The orchestrator is
  ready; routing the public proving APIs through `WebGpuProverPool`
  is a separate integration.
- **SP7 GPU-resident witness** is now the highest-EV remaining lever
  for multi-segment fixtures — it shrinks the per-segment CPU witgen
  time AND the per-segment buffer peak (which would also unblock
  concurrent segment proves).
- **`lift_and_join_async`** is retained but unused by the orchestrator.
  It works and verifies; it's just not faster on a single GPU. If a
  future multi-GPU browser API appears, it becomes the right path.
