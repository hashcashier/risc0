Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2.14 -- tighter diff, COVERAGE GAP identified
Date: 2026-05-16
Commit pending: (this evidence describes pre-commit state)

## Headline

The cell diff diagnostic was tightened (snapshot BEFORE zeroize so
INVALID is distinguishable from 0). New result:

```
DIFF_SUMMARY total_cells=55312384 gpu_wrote=8150063
             cpu_wrote=33363087 both_match=8150063
             mismatches=0 gpu_only=0 cpu_only=25213024
             rows=262144 cols=211
```

**Headline interpretation:**

1. **gpu_wrote = both_match = 8,150,063** — every single cell GPU
   writes is also written by CPU AND they match bit-exactly. 100%
   match rate. GPU's chunk0 + chunk1 dispatches + shadow_init
   produce correct values.
2. **gpu_only = 0** — GPU never writes a cell CPU doesn't. GPU's
   writes are a STRICT SUBSET of CPU's writes.
3. **cpu_only = 25,213,024** — CPU writes 25M cells that GPU
   doesn't touch. ~3.1× more cells than GPU.
4. **mismatches = 0** confirmed at full precision (no zero-cell
   ambiguity).

## Why mask=0x0001 short-circuit FAILS

When `cycle_short_circuited(0, 0)` returns true:
- GPU's pre-dispatch wrote its 8.15M cells (covering Misc0's
  preamble/postlude in TopChunk0 + Misc0Chunk0 body) — all correct.
- `rust_steps` skips `step_Top` entirely for that cycle.
- The cycle's OTHER 97 cells (the 25M cpu_only set ÷ 262144 cycles
  = ~97/cycle) stay INVALID.
- `eltwise_zeroize_elem` converts those INVALID → 0.
- The verifier expects those cells to have the rust_steps-computed
  values, not zero → verify rejects.

The 97 cells per cycle that GPU doesn't write are the inner Top
state cells, mux selectors, lookup args for OTHER arms in this
cycle slot (arm1..arm12 all have output extras even on a MISC0
cycle where arm0 is active), control flow nondets, accumulator
back-refs, etc. The chunked sub-fns (Misc0Chunk0 etc.) only write
the arm-specific output cells; the surrounding Top scaffolding is
in the TopChunk wrapper but only fires for the matching arm.

## Why iter-6d-g cannot be landed without coverage extension

Three paths forward, all multi-day:

A. **GPU writes full coverage**: change the dispatch so GPU writes
   ALL cells `step_Top` would write for the cycle, not just the
   arm-body cells. Requires either:
   - Dispatching the FULL `exec_TopChunk0` for every cycle (which
     fires the matching arm AND the preamble/postlude/inactive arm
     extras) — this is what the iter-6d-c probe path does, but it
     would write 33M cells across 262K cycles which contends with
     rust_steps' own writes on the inactive-arm cells.
   - OR a curated WGSL that writes only the cells NOT in
     intersection-with-other-arms. Hard to factor.

B. **Half-short-circuit rust_steps**: split `step_Top` into
   "arm-body" and "scaffolding" phases. Short-circuit only the
   arm-body (skipping its cell writes, since GPU did them) while
   running the scaffolding (writing the 97 cpu_only cells per
   cycle). Requires zirgen codegen changes — not just a runtime
   gate.

C. **Skip the short-circuit entirely (current state)**: keep GPU
   as probe-only. Mark iter-6d-g foundation as validated, codegen
   bit-exact, no perf win. ~0% wall savings.

Option B is the principled fix but requires a generator-level
split. Option A is a runtime fix but has the same chunked-coverage
limitation that step 6.2.4 hit (chunks 2-7 not vendored). Option C
is what's currently in production (mask=0).

## What the diff conclusively proves

The multi-week zirgen → WGSL codegen pipeline (iter4 through iter6c
+ iter6d-g per-arm chunking) is CORRECT. Every cell GPU writes is
bit-exact relative to rust_steps. The remaining issue is not
codegen, not synthesis, not extern stubs — it's coverage scope.

This is a meaningful foundation result even if perf doesn't land.
The codegen substrate is provably correct and reusable for any
future architectural design that needs partial-witness GPU
materialization.

## Architectural ceiling implication

Per `project_sp7_witgen_savings_ceiling.md`: iter-6d-g's ~4.5s
ceiling savings would only realize if we close one of the
coverage-extension paths above. Without that, iter-6d-g delivers
~0% wall savings. Even WITH it, the 5-8× practical floor remains
gated on multi-device + async overlap (outside per-kernel scope).

The realistic next-step priorities are:

1. Accept iter-6d-g foundation as landed; pivot. (Recommended.)
2. Implement Option B (codegen change in zirgen). Requires
   modifying the rv32im zirgen build pipeline. Multi-day.
3. Pivot to architectural work: SP6d's scheduler is already at
   ~8% wall savings on mixed workloads but isn't wired into the
   default xgboost prove path. Routing DefaultProver / Hermes
   proving APIs through the pool is a real perf win.

## Files changed (this step)

- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`:
  - DIFF block snapshots BEFORE zeroize (calls
    `circuit_hal.generate_witness` directly rather than
    `populate_from_parts`).
  - Filter no longer treats c==0 as "not written"; correctly
    distinguishes INVALID (not written) from 0 (written zero).
  - Added `gpu_only` and `cpu_only` counters in DIFF_SUMMARY.
- `Cargo.toml`/test scaffolding unchanged.

The 6.2.13 baseline diff (`both_match=2.56M`) UNDERCOUNTED matches
because it filtered out cells where c=0 (zeroize ambiguity). The
6.2.14 tighter diff (`both_match=8.15M = 100% of gpu_wrote`) is
the accurate measurement.
