# M7 — cross-phase overlap: early lifts under the segment phase

**Date:** 2026-07-04 · **Branch:** wasm-webgpu-prover-perf
**Thesis (from M6d close-out):** the recursion HALs idle through the entire
~6.6 s segment phase while all 11 lifts (~800 ms each over 3 devices) wait
for `composite_to_succinct`. A lift's only input is one finished
`SegmentReceipt`, so lift(i) can start the moment commit(i) lands — hiding
lift time under the remaining segment pipeline. Prerequisite: recursion
proofs carry ~150 ms/proof of main-thread CPU (preflight 30, witgen_new 92,
verify 15), which under the segment phase would starve commit readback
callbacks exactly the way M6d iteration 1 did.

## M7a — pool-offload the recursion main-thread CPU (prerequisite + own win)

Per-proof main-thread blocking measured from the M6d landing xgboost log
(21 recursion proofs: 11 lifts + 10 joins):

| chunk | sum (21 proofs) | per-proof | disposition |
|---|---:|---:|---|
| `recursion_preflight` | 633 ms | 30 ms | → pool worker (owned data) |
| `recursion_witgen_generate` (exec-plan build) | 633 ms | 30 ms | → pool worker (shadow handles) |
| ctrl construct + global copy (witgen_new residue) | ~705 ms | ~34 ms | stays (M4b cache-entangled) |
| `recursion_witgen_alloc_init` | 267 ms | 13 ms | stays (buffer creation) |
| `recursion_witgen_zeroize` + noise | ~120 ms | ~6 ms | stays (cheap eltwise) |
| `recursion_witgen_post_zeroize` | 225 ms | 11 ms | stays (GPU dispatch) |
| `verify_lift`/`verify_join` (+resolve/union) | 309 ms | 15 ms | → pool worker (owned receipt) |

Shape of the change (mirrors rv32im's M6d `generate_witness_offloaded_async`):

1. `rust_kernels`: exec-plan body extracted to a slice-based helper;
   new `generate_witness_exec_plan_on_shadows` runs it over `Send + Sync`
   `CpuBuffer` handles. The kernel context was already cross-thread-capable
   (`KernelArgs`/`MachineContext` are unsafe-Send with per-cycle-disjoint
   writes, ported from the C++ poolstl reference) — only the
   `WebGpuBuffer` view wrapper pinned it to the main thread.
2. `witgen.rs`: `WitnessGenerator::new` split into `alloc_buffers` /
   `finish_after_generate` (noise + zeroize + deferred GPU dispatches),
   with `new` recomposing them for the native/sync paths.
3. recursion `hal/webgpu.rs`: `preflight_offloaded_async` (Program + input
   move through the worker and back) and `witgen_new_offloaded_async`
   (alloc on main → exec-plan on worker via shadow handles → plan stashed
   and consumed with no await in between → finish on main). The
   candidate-disabled diagnostics path keeps the inline constructor.
4. zkvm `prover_impl.rs`: `verify_integrity_offloaded` helper; lift, join,
   resolve, and union integrity checks move to pool workers (receipts are
   plain owned data).

The raw-pointer `RawPreflightTrace` is built AND consumed on the worker
from the owned `Preflight` — no pointer crosses a thread. Zero new unsafe.

### M7a gate (same-conditions A/B vs M6d landing)

| gate | M6d landing | M7a |
|---|---:|---:|
| parity suite | 4/4 | 4/4 |
| BusyLoop | 2154 ms | 2267 ms (noise band) |
| KeccakUnion(1) | 38249 ms | **35658 ms (−6.8%)** |
| xgboost | 15374 ms | 15287 ms (≈flat) |
| xgboost `composite_to_succinct_async` | 8693 ms | **8134 ms (−6.4%)** |

All receipts verified; dispatch-counter asserts green. KeccakUnion is the
big M7a win — its union chain interleaves proofs on the main thread for
~30 s, so it had the most starved callbacks to reclaim. Span diagnostics
behave exactly per the M6d physics: the offloaded exec-plan SPAN stretches
633 → 1644 ms (pool contention + oneshot wake), `recursion_witgen_new`
1934 → 2971 ms, while the phases around them compact — spans are
diagnostics, wall is the goal. xgboost's segment phase swung +400 ms in
this run (its run-to-run spread was ±600 ms across the M6d iterations;
M7a does not touch that path).

## M7b — early lifts under the segment phase

Shape: for Succinct receipts with >1 segment, `prove_session_async` hands
each finished non-final `SegmentReceipt` to the dedicated recursion
devices the moment its commit lands (the final segment's claim gets the
session output merged after the loop, so it cannot lift early). Lift
futures are driven by the same awaits as the M6d pipeline
(`future::select(main, early_lifts.select_next_some())` — a bare await
would freeze them on the single-threaded executor); in-flight lifts are
drained before the keccak phase claims the devices; the pre-lifted leaves
seed `composite_to_succinct`'s join-tree scheduler
(`composite_to_succinct_with_prelifted_async`), which skips them.

### Iteration 1 (width 2): overlap perfect, exchange rate poor

| gate | M7a | M7b width-2 |
|---|---:|---:|
| parity suite | 4/4 | 4/4 |
| BusyLoop | 2267 ms | 2211 ms |
| KeccakUnion(1) | 35658 ms | 35173 ms |
| xgboost | 15287 ms | **14695 ms** |
| xgboost `composite_to_succinct_async` | 8134 ms | **4674 ms** |
| xgboost segment-phase window | ~6.9 s | **9.8 s (stretched)** |

Timeline: all 10 non-final lifts started AND finished inside the segment
window (11 995 ms of lift span hidden; `early_lift_drain in_flight=1
queued=0` took 16 ms). But the segment window stretched 6.9 → 9.8 s —
the commits absorbed 2.9 s of the 3.5 s moved. M6d iteration-1 physics in
softer form: M7a offloaded the lifts' witgen/preflight/verify CPU, but
each lift still carries ~250 ms of UN-offloaded main-thread CPU (zkp
`Prover` transcript hashing, FRI-fold continuations between readbacks,
ctrl transpose ~34 ms, alloc 13 ms, post-zeroize dispatch encode), and
ten of those under the segment commits' readback chain inflate every
readback's observed latency. Net −592 ms — real, but a 17% exchange
rate.

### Iteration 2 (M7c, width 1): the measured optimum

One lift device instead of two — halves the density of injected
main-thread CPU; lift cadence (~0.7 s) still beats segment cadence
(~0.9 s).

| metric | width 2 | **width 1** |
|---|---:|---:|
| xgboost | 14695 ms | **14348 ms** |
| segment window | 9839 ms | 9028 ms |
| `composite_to_succinct_async` | 4674 ms | 5011 ms |
| lift span under window | 11 995 ms | 7742 ms |

Exchange rate improved 17% → 30%: hiding LESS but stretching less nets
more wall. Cumulative vs M6d landing: **15374 → 14348 ms (−6.7%)**.

### Iteration 3 probe (M7d): early joins + last-leaf-at-root tree — WORSE, reverted

Two changes together: (a) the join tree reshaped so the root splits off
the FINAL leaf — the final segment can never lift early (output merge),
so with it isolated the whole (0, N−1) subtree is early-completable,
leaving lift(N−1) + one root join as the only post-window work; (b) the
second recursion device runs joins (only joins) as pairs become ready.

Result: the tail collapsed exactly as designed —
`composite_to_succinct_async` 5011 → 3271 ms, 5 joins + 8 lifts ran
in-window — and the wall got WORSE: xgboost 14554 ms (+206 vs M7c),
segment window 9028 → 11 050 ms, `gpu_active` +1.3 s in the same wall,
gpu_idle_ratio 0.30 → 0.22. Confirmed from both directions (width-2
lifts, width-1 lifts + join device): **a second concurrent recursion
proof under the segment phase costs more than it hides** — part
un-offloaded main-thread CPU, part physical GPU contention with the
commit kernels (idle-ratio DOWN while wall UP is the tell; GPU
utilization is an empty proxy, again). Reverted to the M7c
configuration; the tree-shape probe is recorded in the
`join_tree_internal_nodes` doc comment. Two structural improvements
kept: tree nodes carry their `mid` (single source of shape), and the
composite scheduler seeds from node-keyed `prespawned`/`predone` sets.

## Landing

Configuration: M7a offloads + width-1 early lifts, midpoint tree.
Landing gates on the exact formatted bytes (all receipts verified,
CARGO_EXIT 0 everywhere; native: zkp 31/31 lib + 5 doctests, rv32im
coded-error count 28 = baseline 28, recursion 2 = baseline 2 — both
pre-existing):

| gate | M6d landing | M7 landing |
|---|---:|---:|
| parity suite | 4/4 | 4/4 |
| BusyLoop | 2154 ms | 2167 ms |
| KeccakUnion(1) | 38249 ms | **35374 ms (−7.5%)** |
| xgboost | 15374 ms | **14509 ms (−5.6%)** |
| heavy 25-keccak fixture | 103.7 s | **100.4 s (−3.2%)** |

xgboost ≈ 14.4-14.5 s ≈ **2.55× native CUDA** (21× at project start).

**Measurement-hygiene incident:** the first landing-gate run overlapped
the native `cargo check/test` suite and inflated every fixture 8-15%
(BusyLoop +270 ms with zero code in its path; KeccakUnion 41.1 s). The
correctness results stood; every wall number was invalid. Re-ran on a
quiet machine — the numbers above. Never overlap native builds with
browser wall-time gates.

## Next levers (post-M7 ranking)

1. **Residual segment-commit main-thread blocks** (M6d lever #2, now the
   dominant phase — the segment window is ~9 s of the 14.5 s wall and it
   is the thing early-lift stretch taxes): zeroize/sparse-scan tail,
   setup + injector scatter, the commit's own accum stepper — entangled
   with `finish_hal_op` authoritative semantics; ~0.5-0.9 s direct, plus
   it lowers the stretch tax on every overlap scheme.
2. **Un-offloaded recursion main-thread CPU** (zkp `Prover` transcript +
   FRI continuations, ~0.2 s/proof): would improve the early-lift
   exchange rate (possibly re-opening width-2 / early joins — the M7d
   machinery is a small re-add) and compact the keccak/union phase too.
3. Keccak-phase levers for the heavy fixture (25-keccak fixture still
   ~100 s; the keccak/union chain is ~95% CPU on the main thread).
4. First-segment fill bubble (~590 ms exposed witgen(0)).
