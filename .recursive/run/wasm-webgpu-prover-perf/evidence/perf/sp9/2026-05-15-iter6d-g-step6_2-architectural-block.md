Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2 -- architectural block on inter-cycle reads
Date: 2026-05-15

## Finding

Step 6.2 (synthesize `InstInputStruct` and call arm sub-fn) is
**architecturally blocked** by inter-cycle data dependencies that
the iter-6d-g design doc did not surface. The block kills the
projected 17.1x CUDA savings (about 5 s) until resolved.

## Root cause

`exec_TopChunk0` and every arm sub-fn read several cycles' back
state via `back_Reg(1, ...)`, e.g. line 16953-16959 of
`exec_top_chunk0.wgsl`:

```
let x9: NondetRegStruct = back_Reg(1, lookup_TopLayout_nextPcLow(layout0));
let x10: NondetRegStruct = back_Reg(1, lookup_TopLayout_nextPcHigh(layout0));
let x11: NondetRegStruct = back_Reg(1, lookup_TopLayout_nextState_0(layout0));
let x12: NondetRegStruct = back_Reg(1, lookup_TopLayout_nextMachineMode(layout0));
```

`back_NondetReg` reads `data_buf[col_offset + (cycle - 1) % rows]`.
So cycle N's InstInputStruct construction reads cells WRITTEN by
cycle N-1's step_exec.

`risc0/circuit/rv32im/src/prove/hal/rust_steps.rs:737-742` confirms
the sequential semantics:

```
for cycle in 0..split {
    step_exec(preflight, &tables, cycle, data, global)?;
}
for cycle in split..last_cycle {
    step_exec(preflight, &tables, cycle, data, global)?;
}
```

Cycles are NOT parallel. rust_steps runs them sequentially because
each cycle's `back_Reg(1, ...)` calls depend on the previous
cycle's writes. (rayon parallelism in the broader prove path
operates on independent groups -- never across cycles.)

## What this means for GPU witgen

The current GPU probe dispatches `exec_TopChunk0` over ALL N
cycles in parallel. Every thread reads `data_buf[(cycle-1)*cols+k]`
expecting cycle N-1's writes to be present. They aren't (rust_steps
runs AFTER the probe, not before), so the GPU probe writes garbage.

For step 6.2 onwards to produce CORRECT witgen output, one of:

1. **Sequential GPU dispatch** -- one workgroup per cycle, awaited
   sequentially. Defeats the whole point of GPU parallelism
   (about 1M cycles times about 1 ms per dispatch = about 17 min
   of pure submission overhead, way worse than rust_steps 500 ms).
2. **Materialize all inter-cycle reads as preflight buffers** --
   for every `back_Reg(N, ...)` call, pre-compute the value on CPU
   from preflight data, upload as a side buffer, have WGSL read from
   the side buffer instead. This requires:
   - Identifying every `back_Reg(N, ...)` call site (dozens to hundreds
     across the 13 arm sub-fns plus exec_TopChunk0)
   - Computing the right value on CPU from preflight (each call site
     has its own layout cell, possibly inferred from prior cycles'
     state -- but those prior cycles' state isn't trivially
     extractable from preflight either; it's the OUTPUT of step_exec,
     not the INPUT)
   - Uploading per-cycle per-call-site values
   - Plumbing through wrapper layouts

Option 2 is essentially "re-implement step_exec on CPU and ship the
output to GPU just to re-dispatch through pretty WGSL." That's
**slower than running step_exec on CPU directly**, defeating the
purpose.

## Reconciliation with iter-6d-g design

The design doc (commit e26f0c597) projected step 6 as "synthesize
InstInputStruct + dispatch arm sub-fn" yielding 17.1x CUDA. That
projection ignored the back_Reg(1) inter-cycle reads. Once those
are accounted for, the path is:

- **Step 6.1 (DONE, ac7a684c0)**: cycle_list buffer plumbing. xgboost
  wall 104.31 s (no regression). Structural infrastructure for any
  future per-arm dispatch.
- **Step 6.2 (BLOCKED)**: arm sub-fn synthesis with correct args.
  Blocked on inter-cycle read materialization, which itself blocks
  on a deeper redesign of the witgen path.
- **Step 6.3-6.5**: dependent on 6.2.

## Practical implication

iter-6d-g cannot reach 17.1x CUDA without solving the inter-cycle
read problem. Possible futures:

- A zirgen MLIR pass that lifts all `back_Reg(1, ...)` calls to a
  preflight-data lookup table (CPU pre-computes; GPU just reads).
  Multi-week zirgen work; impact about 5 s on xgboost.
- A WGSL execution model with explicit cycle-by-cycle ordering using
  workgroupBarrier semantics across all 1M cycles. Not possible in
  WGSL (workgroups don't synchronize across the grid).
- Hybrid: GPU witgen runs for the "trivial" cycles (Misc0/Misc1/Misc2
  arms that may have shallower back_Reg chains) while rust_steps
  handles the rest. Requires per-arm analysis of which back_Reg
  reads are inter-cycle vs intra-cycle.

## Current stance

iter-6d-g step 6.1 (cycle_list plumbing) is landed and validated.
The remaining 6.2-6.5 steps are deferred to a future session that
can prosecute one of the three futures above. **The 17.1x CUDA
projection is not achievable from step 6 alone** under the existing
back_Reg circuit semantics.

This also strengthens `project_sp7_witgen_savings_ceiling` -- the
practical floor of 5-8x CUDA is **even further out of reach** than
that memory anticipated, because the witgen-replacement path
itself has architectural blockers that need to be unblocked before
the 5 s rv32im_witgen savings materialize.

## CORRECTION 2026-05-16: per-arm back_Reg count finding

The pessimistic conclusion above was **overly broad**. Per-arm
`back_Reg(N, ...)` call counts measured on each delta:

| Arm | back_Reg calls |
|-----|---------------:|
| MISC0 | 0 |
| MISC1 | 0 |
| MISC2 | 0 |
| MUL0 | 0 |
| DIV0 | 0 |
| MEM0 | 0 |
| MEM1 | 0 |
| ECALL0 | 0 |
| CONTROL0 | 19 |
| BIGINT0 | 24 |
| POSEIDON0 | 40 |
| POSEIDON1 | 40 |
| SHA0 | 106 |

**8 of 13 arms have ZERO internal `back_Reg` calls.** They are
pure per-cycle: they depend only on the `InstInputStruct` passed
in, not on cycle-N-1 data.

For xgboost (mostly ALU/MEM ops -- minimal SHA/Poseidon),
**these 8 arms cover the vast majority of cycles**. So GPU witgen
replacement on those 8 arms IS tractable, requiring only:

1. Shadow-init 5 outer cells per cycle (`nextPcLow`, `nextPcHigh`,
   `nextState_0`, `nextMachineMode`, `isFirstCycle`) -- 5 × 4 ×
   N_cycles ≈ 20 MB upload (column offsets 14-18 in
   `kLayout_Top`). Pre-computed from `preflight.cycles[N+1].pc`,
   `state`, `machine_mode`.
2. Synthesize `InstInputStruct` from preflight (5 fields:
   minor, pcU32 lo/hi, state, mode, plus minorOnehot derived).
3. Call each of the 8 zero-back_Reg arm sub-fns via the per-arm
   cycle_list dispatch (already plumbed in step 6.1).
4. Short-circuit `rust_steps::step_exec` only for cycles whose
   major opcode is in {MISC0, MISC1, MISC2, MUL0, DIV0, MEM0,
   MEM1, ECALL0}.

The 5 inter-cycle-heavy arms (CONTROL0, BIGINT0, POSEIDON0/1,
SHA0) remain blocked on a deeper materialization scheme. But for
the typical workload mix on xgboost, leaving those 5 to rust_steps
should still yield most of the 5 s rv32im_witgen savings.

Lesson: don't generalize from one arm's complexity to all 13.
Per-arm analysis was the missing diagnostic step in the original
block claim above.
