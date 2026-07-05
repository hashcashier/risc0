# M11 — ECALL0 + POSEIDON0 direct accum kernels: correct, wall-NEUTRAL, reverted

**Date:** 2026-07-05 · **Branch:** wasm-webgpu-prover-perf (reverted; tree at be6363484)
**Thesis:** after M8b (POSEIDON1), the M8 warm skip-mask timing said majors
8 (ECALL0) and 9 (POSEIDON0) each cost ~25% of the accum stepper pass;
eliminating both via the proven direct-kernel template was predicted
−2..3% xgboost.

## Implementation (preserved at scratchpad m11-arms89-implementation.diff, 860 lines)

One generic `accum_special_args_direct_wgsl(label, cycle_table, lead_u16,
memory_args, cycle_args, u16_args, u8_args, tail_u16, expected_terms)`
emitter + two thin wrappers:

- ECALL0 (arm8): 26 terms — `_0.arg1/arg2`, pcAddr.upperDiff/med14,
  memory_arg[0..8], cycle_arg[0..4], arg_u16[0..4], arg_u8[0..4],
  addPC.low16/high16 — snapshot cols 0..8 + col19.
- POSEIDON0 (arm9): 52 terms — `_0.arg1/arg2`, memory_arg[0..16],
  cycle_arg[0..8], arg_u16[0..24], arg_u8[0..2] — cols 0..17 + col19.

Term order machine-extracted from the zirgen arm slices; snapshot cadence
= store after every 3-term group INCLUDING the partial tail group, col19
= final (generalizes control0's cadence; poseidon1/control0 verified as
special cases). User-accum state = BigInt nop constants — verified from
the slices: `exec_TopExtractArm{8,9}` return the pruned CONSTANT zero
BigIntTopState, identical to arm10 (bigint state lives on arm12 rows
only). Full wiring: statics/setters, kind enum arms, lookup fns, grouped
dispatch params, step_accum row collection (majors 8/9), mask bits,
enable + prewarm, mod.rs re-exports, xgboost gate asserts.

## Correctness — all green

Parity 4/4; receipts verified on every rep (2 canaries, 5 xgboost, heavy
25-keccak); both dispatch-counter asserts moved; BusyLoop flat
(2201-2253 vs base 2195-2230 — no hidden-queue regression, prewarm
covers the two extra Tint compiles). The kernels are bit-correct in
production use.

## Perf — wall-NEUTRAL (same-afternoon A/B, n=5 medians per M10 policy)

| block | xgboost reps | median | keccak canaries (state) |
|---|---|---:|---|
| M11 (15:00-15:30) | 15375, 14552, 14939, 15150, 14590 | **14939** | 36954 / 36873 (transitioning) |
| base (16:10-16:40) | 14554, 15127, 14966, 15457, 14681 | **14966** | 38229 / 37069 (slow state) |

Median delta −0.2%, inside noise; the environment drifted SLOWER between
the blocks (afternoon slow-state onset, see the M10 appendix), which
biases in M11's favor — the true effect is ≈0 or slightly negative.
Verdict: post-M8b, the remaining accum stepper is NOT wall-exposed; its
CPU time hides under GPU/queue waits in the pipelined flow (and/or the
added GPU accum work cancels the CPU saving).

## Consequences for the lever board

- The stepper-elimination arc (arm3 MUL0/SLLI, mem0-remainder) is CLOSED
  as a wall lever — if 50% of the pass is wall-neutral, the remaining
  ~40% cannot beat noise either.
- Retrospective: M8b's −1.4% was a single-sample pair measured in the
  drift era; with today's noise floors (xgboost single samples ±2.5%) it
  was likely partial noise. M8b stays landed (correct, neutral-at-worst,
  and it removed real CPU), but its wall claim is downgraded.
- Revival condition for M11: any change that re-exposes the accum commit
  on the critical path (e.g., different segment pipelining) — reapply
  the archived diff and re-gate with rep blocks.

## Process notes

- SEQUENCING RULE (violated once, caught): while a measurement block is
  running, the tree is FROZEN — the per-rep `cargo test` picks up source
  edits, rebuilding mid-block (compile noise + wrong bytes). The first
  baseline block died this way (round 4 rc=101); salvaged rounds 1-3.
- Two background launches of the gate block were externally stopped;
  switched to per-rep foreground execution (each rep ~157 s fits the
  Bash cap comfortably) — more robust and gives per-rep narration.
