# M2a: eval_check interpreter — reorder + scalar-bank hybrid (ACCEPTED)

Date: 2026-07-02. Status: ACCEPTED — gates green, receipts verified.

## Gate results (same-day A/B vs M1b baseline, warm GPU)

| Gate | M1b | M2a | Movement |
|---|---:|---:|---:|
| Representative runtime | 57.22 s | **48.62 s** | **−15%** |
| BusyLoop wall / gpu_active | 3923 / 2480 ms | **3670 / 2225 ms** | −6.4% |
| KeccakUnion wall / gpu_active | 52562 / 33683 ms | **44238 / 25337 ms** | **−15.8%** |
| xgboost runtime | 32.22 s | **29.65 s** | −8% |
| xgboost wall | 31670 ms | **29108 ms** | **−8.1%** |

All receipts verified, `cpu_fallbacks=0`, `cpu_only_ops=0`. xgboost
≈ **5.1× native CUDA** (5.7 s); project baseline was 117.9 s (21×).

Close-out attribution (drains on, log
`2026-07-02-m2-close-out-attribution.chrome.txt`):
`finalize_async drain_after_eval_check` **8565 → 6040 ms** (n=32) —
exactly the focused-bench prediction (11×~390 + 21×~84). The M2b split
(below) re-ranks the remaining buckets: eval_check 6.0 s,
accumulate-witgen GPU work 2.7 s, FRI 1.9 s.

## M2b split result (same run)

The suspect `poly_group accum drain_after_batch_expand` 2545 ms bucket
was ANOTHER label lie: with the new `commit_group <group>
drain_before_commit` drain in place, **`commit_group accum
drain_before_commit` absorbs 2675 ms** (queued accumulate-witgen GPU
kernels) and the real accum expand NTT is ~190 ms. Also code/ctrl
`drain_before_commit` absorb 687/619 ms of queued witgen work. The
drains are retained default-off alongside the finalize set.

## Starting point (post-M1b attribution)

`drain_after_eval_check` = 8.57 s of the 31.7 s xgboost wall (largest bucket,
`gpu_active=true` — real GPU execution, not queue-wait). Distribution:
**rv32im segments ~605 ms each** (ext interpreter: 20,202 instrs, 927 vec4
slots = 14.8 KiB/thread private scratch), **recursion proofs ~91 ms each**
(base interpreter: 12,359 instrs, 1001 u32 slots = 4 KiB/thread). rv32im is
6.6× the per-proof cost on 1.63× the instructions — per-op cost tracks the
slot representation width (vec4 vs u32), not tape length.

rv32im rides the ext interpreter only because its tape has **six `ConstExt`**
instructions; taint analysis shows ≤101 simultaneously-live ext values.

## Focused benchmark (landed)

`webgpu_eval_check_bench_production_shape` + `eval_check_webgpu_bench` helpers
in the rv32im/recursion crates: full dispatch + drain at po2=18
(domain=2^20), zero-filled buffers (arithmetic is data-independent), 5 reps.
Baseline reproduces production drains exactly:
- rv32im: [1547 cold, 606, 607, 606, 610] — steady ~607 ms
- recursion: [433 cold, 93, 90, 92, 93] — steady ~92 ms

Also landed `rv32im_eval_check_poly_ext_matches_cpu` (production tape parity
vs portable at po2=1 — rv32im previously had only a tiny custom-def parity).

## Step 1 — lazy tape reorder (ACCEPTED, kept)

`eval_check_reorder_lazy`: mix ops keep original order (mix ids / mix-pow
indices unchanged); fp ops emitted on first demand via iterative post-order
DFS, ids renumbered. Same ops, same operands, valid topological order →
bit-identical output; only peak liveness changes.

| Circuit | fp_slots | bench (steady) |
|---|---|---:|
| rv32im (ext interp) | 927 → 685 | 607 → 601 ms (≈flat) |
| recursion (base interp) | 1001 → 287 | 92 → 84 ms (−9%) |

All parity tests green. KEY NEGATIVE RESULT: scratch-size reduction alone
barely moves either circuit → the interpreter is NOT scratch-traffic-bound
at these sizes. Per-op cost gap (30 ps vs 7 ps per op-cycle) matches the
vec4-vs-scalar ALU width instead. The reorder's real value: caps the ext
bank at 101 live slots, enabling the hybrid.

## Step 2 — hybrid base+ext-bank interpreter (REGRESSION — under diagnosis)

Encoder `eval_check_base_interpreter_instructions` rewritten as a two-bank
hybrid (base u32 bank ops 0-9 byte-identical for pure-base tapes; ext vec4
bank ops 10-18 for ConstExt-tainted values; commutative EB/BE normalized).
naga-validates; all parity tests green (rv32im now `base_private` with
fp_slots=681 ext_slots=101); recursion/keccak streams unchanged.

Bench: **rv32im 601 → 1570 ms (2.6× REGRESSION)**, recursion 84 → 86 ms.
Same kernel source, two pipelines: recursion (fp=287/ext=1/mix=10) runs
7 ps/op; rv32im (fp=681/ext=101/mix=29) runs 78 ps/op — 11× per-op gap.
All uniform-kernel-slowdown hypotheses (switch size, register pressure,
icache) refuted by recursion's unchanged time on the same kernel source.

Probe matrix (temporary bench-only knobs, to be removed after diagnosis):
- `set_eval_check_bench_slot_floor` — recursion stream on rv32im-shaped
  arrays → tests "compiled array shape" hypothesis.
- `set_eval_check_bench_neuter_ext_ops` — rv32im stream with ops 1,10-18
  rewritten to base analogues (same length/slots, timing-only) → tests
  "ext-case execution" hypothesis.

RESULTS (steady-state ms, po2=18):

| Cell | ms | per-op |
|---|---:|---|
| recursion, normal (fp=287/ext=1/mix=10) | 86 | 7 ps control |
| recursion @ rv32im shape (681/101/29), same stream | 162 | shape alone ≈ 1.9× |
| rv32im, ext ops neutered to base analogues | 318 | shape + longer tape, sane |
| rv32im hybrid, real | 1570 | **~1.25 s from 2,926 ext-op executions ≈ 430 ps each** |

Verdict: executing the ext-bank cases is the pathology (~50× a base op),
NOT the arithmetic (the identical `ext_mul` cost ~25 ps/op in the all-ext
interpreter). Structural suspect: the all-ext kernel declares FLAT
function-scope scratch (`array<vec4<u32>, N>`), while the base kernel's
private variant reuses the workgroup-lanes template shape — NESTED
`array<array<vec4<u32>, N>, 1>` indexed `[0u][slot]`. Small arrays (mix,
10-29 slots) hide this because dynamically-indexed small arrays lower to
register-select trees; a 101-slot vec4 array goes to local memory where the
nested lowering defeats the fast path.

Fix A (flatten the nested private arrays): NO EFFECT — rv32im still
~1580 ms, probes byte-identical. Nesting hypothesis refuted (the compiler
already folds `[0u][slot]`). Kept anyway as a shape simplification.

Fix B (scalarize the ext bank): **WORKS — rv32im 1570 → 387 ms.**
`fpe` becomes a scalar `u32` array (ext value k at words `4k..4k+3`,
module-scope `var<private>` in private mode / `var<workgroup>` in lanes
mode) with `ext_load`/`ext_store` helpers assembling `vec4<u32>` from four
scalar accesses. Encoder unchanged; bit-exact (same field ops on the same
values); parity suite green.

## Root cause (confirmed)

**Dynamically-indexed vec4-typed local/private arrays are pathologically
slow under Chrome/Dawn/NVIDIA-580 lowering** — ~430 ps per executed
vec4-array op vs ~25 ps for the identical arithmetic against scalar u32
arrays (~17×). Small vec4 arrays (mix_tot/mix_mul, 10–29 slots) dodge it
(register-select-tree sized); big ones (fpe at 101, and the all-ext
interpreter's 685–927-slot fp) pay it on every access. This retroactively
explains why the all-ext interpreter ran 30 ps/op and why shrinking its
slot count (927→685) changed nothing while changing the *representation*
changed everything.

## Final focused-bench state (po2=18, steady-state)

| Kernel | rv32im | recursion |
|---|---:|---:|
| Baseline (ext interp / base interp) | 607 ms | 92 ms |
| + lazy reorder | 601 ms | 84 ms |
| + hybrid, vec4 ext bank | 1570 ms (regression, diagnosed) | 86 ms |
| + hybrid, **scalarized ext bank (landed)** | **387 ms (−36%)** | **84 ms (−9%)** |

rv32im per-proof eval_check drain expectation: ~605 → ~390 ms
(segments bucket 6.6 s → ~4.3 s of xgboost wall). Remaining headroom on
this bucket (not pursued this phase): declared-shape pressure costs ~2×
(recursion stream on rv32im-shaped arrays: 86→162 ms) — operand fusion
(leaf remat folded into consumers, host study says fp 681→360) would
shrink it further.

## Diagnostic knobs

`set_eval_check_bench_slot_floor` / `set_eval_check_bench_neuter_ext_ops`
were TEMPORARY and are removed again; the probe matrix numbers above are
preserved here and in the run logs (`/tmp` probe logs copied alongside if
needed).

## M2b rider

Added default-off `commit_group <group> drain_before_commit` and
`drain_after_make_coeffs` drains (both commit variants) so the suspect
`poly_group accum drain_after_batch_expand` bucket (2.5 s) can be split
from queued accumulate-witgen work in the next attribution pass.

## Unrelated pre-existing failure found and fixed

`native_busy_loop_po2_18_async_without_eval_check_gpu_succinct_receipt_verify`
fails at HEAD too (verified byte-identical code on the disabled path):
`set_eval_check_gpu_enabled(false)` records one intentional cpu_fallback per
proof (segment + lift = 2) but the shared helper asserted 0. Fixed by
`prove_succinct_info_async_expecting_cpu_fallbacks(..., 2)` — the exact
expected count keeps unexpected fallbacks fatal.
