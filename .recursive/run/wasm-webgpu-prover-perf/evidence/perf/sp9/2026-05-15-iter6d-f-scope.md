Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 iter 6d-f scope -- the chunk-merge problem`
Date: 2026-05-15

## Why iter-6d-e is still measurement-only

Current iter-6d-e dispatches `exec_TopChunk0` + `exec_TopChunk1`, both
pruned with the "chunk0 everywhere" / "chunk1 everywhere" rewrites.
Each chunked-base call site (e.g., `exec_Sha0(...)`) becomes
`exec_Sha0Chunk0(...)` in the chunk0 module and
`exec_Sha0Chunk1(...)` in the chunk1 module.

The problem: every sub-function's chunks check **disjoint mux arms**
internally. For example, `exec_Sha0Chunk0` only does meaningful work
when `arg1_0.minorOnehot._super[decode(0u)]._super != 0u`. For cycles
whose Sha minor opcode is in `Sha0Chunk1`'s arm
(`decode(268435454u)`), `Sha0Chunk0` hits its `else { unreachable }`
branch and returns an uninitialized `InstOutputBaseStruct` (WGSL
"undefined" behavior, in practice often zeros but not guaranteed).

So a cycle with Sha minor opcode in Chunk1's arm:
- TopChunk0 dispatches → calls Sha0Chunk0 → uninitialized return → wrong cell writes
- TopChunk1 dispatches → calls Sha0Chunk1 → correct cell writes

rust_steps runs after and overwrites everything correctly. iter-6d-e
GPU output is garbage that gets overwritten -- no wall savings.

## What iter-6d-f needs

Each chunked-base call in `exec_TopChunkN` needs to call **all**
chunks of that base, and merge the results. Three sub-problems:

**1. Pruner: emit multi-chunk call sequences.** Replace
```wgsl
let x43 = exec_Sha0Chunk0(a, b, c);
```
with
```wgsl
let x43_0 = exec_Sha0Chunk0(a, b, c);
let x43_1 = exec_Sha0Chunk1(a, b, c);
...
let x43_N = exec_Sha0ChunkN(a, b, c);
let x43 = merge_InstOutputBaseStruct_N(x43_0, x43_1, ..., x43_N);
```
This requires the pruner to know each chunked-base's chunk count
(already tracked in `chunked_max_idx`).

**2. Zirgen: zero-init the uninitialized return path.** Each
ChunkN's `var x: T;` declaration needs an explicit zero initializer
so the unreachable-arm return is deterministically zero, not
undefined. Either (a) modify the zirgen WGSL backend to emit
`var x: T = T::ZERO;` for declarations, or (b) post-process the
emitted WGSL to add zero-inits.

**3. Pruner: emit struct-merge helpers.** For each `T` returned by
a chunked base, emit:
```wgsl
fn merge_T_or(a: T, b: T) -> T {
  return T(a.f1 | b.f1, a.f2 | b.f2, ...);
}
```
Bitwise OR works because at most one chunk's arm fires (the
minorOnehot one-hot encoding guarantees mutual exclusion), and all
non-firing chunks return zero after fix (2).

## Wasted compute

With the merge approach, every cycle runs ALL chunks of every
chunked base it transitively reaches. For `exec_Sha0` (8 chunks),
that's 8× work per cycle that reaches Sha0. The chunks themselves
each do early validity checks (~3-5 ops) and a single mux arm
check (1-2 ops) before branching to their main body. Average
overhead: ~10-15 ops × 8 chunks = 80-120 ops "wasted" per Sha0
call, dominated by the few hundred ops of the actual chunk body.

Estimated per-segment GPU witgen work after iter-6d-f: ~1.5-2× the
ideal per-cycle dispatch (still much less than 11× CUDA savings
worth of work).

## Why this is multi-day, not multi-week

The pruner change (1) is ~50 lines of Rust. The merge helpers (3)
are ~20 lines × N return types (~5-10 types) = ~100-200 lines.
The hard part is (2) -- modifying zirgen's WGSL backend or
post-processing the emitted output to add zero-inits everywhere.

If post-processing handles it: ~200 lines of pruner work.
If zirgen needs touching: 1-2 days of MLIR navigation + a fresh
`gen_zirgen` run.

## Status

iter-6d-a through iter-6d-e infrastructure is committed:
- Pruner extended for per-chunk rewrite (commit 682606033)
- chunk1 module vendored (commit 5119c2395)
- Async prewarm compiles both chunks concurrently
- Dual-chunk dispatch wired

iter-6d-f implementation deferred only because the
chunk-merge-with-zero-init design needs a session focused on the
pruner refactor + zirgen WGSL backend tweak, not because the work
is "too big" -- it is now well-scoped at ~2-3 days.

The 5.2 s wall savings ceiling (rust_steps witgen on segments 2-N)
remains the closure target. Beyond that, iter-6d-deeper for
TopAccum (22 s ceiling) still needs the straight-line arithmetic
chunking pass.

## Memory

Update [[project-sp7-witgen-savings-ceiling]] when iter-6d-f
lands to record the actual measured savings (vs estimated 5.2 s
for full deployment of GPU witgen on xgboost).
