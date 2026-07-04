# M6d — two-deep segment pipeline: witgen(N+1) under commit(N)

**Date:** 2026-07-04 · **Branch:** wasm-webgpu-prover-perf
**Thesis (from M6 close-out):** the segment phase is strictly serial and 42%
GPU-idle; the 4 GiB atomics ceiling (M6b) reopens overlapping segment N+1's
CPU-only prefix with segment N's GPU tail.

## Shape of the change

1. **Per-segment replace-arm-mask** (`M6d-1`): the SP7 witgen-GPU replacement
   arm mask moved from a process-wide static in `rust_steps` to a
   `Cell<u16>` on `WebGpuCircuitHal` (fresh instance per segment prove),
   flowing into `rust_steps` as an explicit parameter (pool workers read it
   from closure captures; task-locals can't reach them). The static remains
   as a write-only diagnostics mirror for browser-prove's assertions.
   Without this, pipelined segment N+1's pre-witgen dispatch would clobber
   the mask segment N's accum phase still reads.
2. **Phase split** (`M6d-2`): `prove_core_async` split at the transcript
   boundary into `webgpu_segment_witgen_phase` (setup + SP7 pre-dispatch +
   witness CPU pass; no Fiat-Shamir state) and
   `webgpu_segment_commit_phase` (header/code/data/accum commits + accum +
   finalize), joined by an opaque `WebGpuSegmentJob`. The trait
   `prove_core_async` and the SP6d pool's `prove_segment_core_async` now
   compose the same two phases serially — one implementation, two
   orchestrations.
3. **Pipeline** (`M6d-3`): `prove_session_async`'s segment loop became
   `try_join(commit_stage(N), witgen_stage(N+1))`, each stage future wrapped
   in `with_authoritative_context` (M4e discipline: plain scope guards
   assume stack discipline that interleaved awaits violate). Depth 2:
   commit phases never overlap each other (readbacks serialize on the one
   device queue), and a third in-flight segment costs ~350 MB of shadows
   for no overlap gain.

## Iteration 1 measurement: overlap real, wall flat — callback starvation

First gate: xgboost 16865 ms (baseline 17320), segment phase **8449 ms vs
8459 baseline — unchanged** despite the span timeline proving every witgen
fully nested inside the previous commit (2975 ms of 2975 possible).
Commit phases stretched by almost exactly the witgen time they absorbed.

Attribution: `merkle code root_top_readback` (the FIRST readback in each
commit) summed **338 → 3000 ms**. The commit phase is not a long GPU
drain; it is a chain of short readbacks whose completions require the main
wasm thread to observe mapAsync callbacks. Witgen's rayon-joins (~90-180 ms
chunks) block that thread, so each readback's latency became
max(GPU, current-witgen-chunk). This is the "ONE wasm thread serializes all
CPU" physics from SP6c, now measured at readback granularity.

## Iteration 2: offload the witness CPU pass to the pool (the fix)

`generate_witness_offloaded_async`: the witness pass runs on a rayon pool
worker via `rayon::spawn` + `futures::channel::oneshot` (cross-thread wake
works under wasm-bindgen-test in the atomics build — validated by gate).
Zero new unsafe: `CpuBuffer` is `Arc<RwLock<…>>` (Send + Sync) — the
`WebGpuBuffer` wrapper's `Rc` flags stay on the main thread behind
`begin/finish_cpu_shadow_offload_mut` (the two halves of `view_mut`'s flag
discipline), and the `PreflightTrace` travels through the worker and back
(no clone). Eqz elision is held as a guard across the await; all takers
set-true/restore so overlap with a blocking accum scope composes.

| gate | baseline (62c5d38) | M6d iter1 | **M6d iter2** |
|---|---:|---:|---:|
| BusyLoop | 2247 | 2233 | **2120** |
| KeccakUnion(1) | 38612 | 38433 | **38078** |
| xgboost | 17320 | 16865 | **15267 (−11.9%)** |
| xgboost segment phase | 8459 | 8449 | **6596 (−22%)** |
| xgboost gpu_idle_ratio | 0.352 | 0.193 | 0.240 |

All receipts verified; parity 4/4 on every build; dispatch-counter
acceptance asserts green (per-instance mask mirrors preserve them).

## Iteration 3 probe: preflight offload — wall-neutral, reverted

Offloading the ~30 ms/segment preflight replay the same way measured
xgboost 15532 vs 15267 (noise-band, slightly negative from pool
contention: witgen sum 3629 → 3729 ms). Reverted; the probe result is
recorded in a comment at the call site. The residual ~1.3 s commit stretch
(commit sum 6002 ms vs ~4664 serial floor) lives in the remaining
main-thread blocks: buffer setup + injector scatter (~29 ms), the
zeroize/sparse-scan tail (~50-76 ms), and the commit's own accum stepper —
each entangled with HAL authoritative-flag semantics (`finish_hal_op`);
none is a clean preflight-shaped move.

## Landing

Landing gates on the exact formatted bytes (fmt reflow shifts
panic-location line numbers — M6a-c lesson): parity 4/4, BusyLoop,
KeccakUnion(1), xgboost, plus the heavy 25-keccak fixture
(`native_keccak_union_succinct_receipt_verify`, post-M5 baseline 107.7 s)
per phase close-out discipline. Native: zkp 31/31 lib + 5 doctests;
rv32im native error count unchanged at the pre-existing 29.

Landing results (all receipts verified, CARGO_EXIT 0 everywhere):

| gate | post-M5 baseline | M6d landing bytes |
|---|---:|---:|
| parity suite | 4/4 | 4/4 |
| BusyLoop | 2247 ms | 2154 ms |
| KeccakUnion(1) | 38612 ms | 38249 ms |
| xgboost | 17320 ms | **15374 ms (−11.2%)** |
| heavy 25-keccak fixture | 107.7 s | **103.7 s (−3.7%)** |

xgboost now sits at ~15.3-15.4 s ≈ **2.7× native CUDA** (21× at project
start, 3.0× after M5).

## Next levers (post-M6d ranking)

1. **Cross-phase overlap (M7 candidate):** the recursion HALs idle through
   the entire 6.6 s segment phase while 11 lifts (~800 ms each on 3
   devices) wait for `composite_to_succinct`. Starting early lifts under
   the segment phase could hide ~2-3 s — but recursion witgen (~600 ms
   CPU/proof) needs the same offload treatment first, or it re-creates the
   callback starvation this arc just fixed.
2. Residual commit stretch (~1.3 s): zeroize-tail + setup offload — needs
   `finish_hal_op`/sparse-scan surgery; medium risk, ~0.5-0.9 s.
3. First-segment fill bubble (~590 ms exposed witgen(0)) — could hide under
   the executor, marginal.
