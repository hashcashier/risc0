# M8 — the segment-commit accum stepper: offload rejected, GPU coverage extension

**Date:** 2026-07-04 · **Branch:** wasm-webgpu-prover-perf
**Thesis (post-M7 lever #1):** the segment window is ~9 s of the 14.5 s
xgboost wall, and its residual main-thread blocks both cost wall directly
and starve every concurrent readback (the M7 early lift, the pipelined
next segment's witgen completion, and — as M8a proved — the FRI query
tails of every in-flight proof). The largest single block is the accum
stepper: ~86-106 ms/segment of `step_TopAccum` CPU between the data and
accum commits, Fiat-Shamir-pinned to that spot.

## FIRST: the environment-drift incident that re-litigated this phase

The M8a probe initially read as a hard regression (KeccakUnion
35374 → 38300, +8.3%) against the M7-landing numbers taken at 15:21 the
same day. It was NOT the change: re-running the UNTOUCHED M7-landing
commit at 20:05 gave KeccakUnion **38329 ms** and xgboost **14982 ms** —
the environment itself drifted +8.4% / +3.3% within five hours (same
machine, same Chrome 149.0.7827.102, same driver, no reboot, quiet both
times; the delta concentrates in the GPU-merkle `check_group` readbacks).
The tell that finally exposed it: two UNRELATED changes (M8a offload,
M8b kernel) showing the IDENTICAL "+3 s KeccakUnion, +5.8 s check
readbacks" signature. Same-day baselines: BusyLoop 2255, KeccakUnion(1)
38329, xgboost 14982. Every number below is judged against THESE.
Rule memorized: A/B pairs must be back-to-back in-session; when a result
surprises, re-run the unchanged baseline before analyzing the change.

## M8a probe — pool-offloading the stepper: wall-NEUTRAL, reverted

The M6d/M7a offload pattern (rayon::spawn + oneshot + `Send` CPU-shadow
handles; trace moved through the worker; GPU dispatches kept in exact
order on main; `cpu_shadow_current()` eligibility probe) applied to the
accum stepper. Parity 4/4, all receipts verified.

| gate | same-day baseline | M8a probe |
|---|---:|---:|
| BusyLoop | 2255 ms | 2290 ms (noise) |
| KeccakUnion(1) | 38329 ms | 38300 ms (flat) |
| xgboost | 14982 ms | 14865 ms (−0.8%, sub-noise) |

Wall-neutral: the offload's genuine benefits and genuine costs CANCEL.
Reverted — complexity with no measured win. The within-run spans identify
both sides precisely:

- **What improved:** `lift_prove_async` −1280 ms, `composite_to_succinct`
  −596 ms (xgboost), and the FRI query tails halved (2245 → 1098 ms
  summed) — freed-main-thread effects, proving those tails are largely
  readback-callback starvation, not local CPU.
- **What it costs (xgboost):** the stepper is mid-transcript FOREGROUND
  work — the commit chain blocks on it. `rayon::spawn` queues it in the
  pool injector, and injected tasks only run when a worker's deque
  empties — behind witgen(N+1)'s 262 K-cycle par_iter. Critical-path
  queueing latency plus pool oversubscription; the witgen_accum span
  stretched 950 → 1420 ms.

**Physics:** only phase-head "background" passes benefit cleanly from
pool offload (witgen at the head of its own proof — M6d, M7a).
Mid-transcript foreground CPU pays injector-queue priority inversion on
its own critical path plus oversubscription against every concurrent
pool-saturating pass; the measured starvation relief elsewhere roughly
cancels those costs. Elimination beats relocation.

## Per-arm diagnostics — what the stepper actually spends

The (major, minor) cycle matrix (11 xgboost segments, temp diagnostic):
CPU-stepped remainder = 415 852 cycles (14.4% of all), concentrated:
POSEIDON1 (m10) 46.8% of stepped cycles, ECALL0 (m8) 18.0%,
MUL0 (m3, 99.75% minor 1 = SLLI) 15.7%, MEM0-non-LW 7.9%,
POSEIDON0 (m9) 7.3%.

Warm same-buffer skip-mask timing (extra passes with candidate majors
also skipped, production pass last so receipts stay valid; first attempt
was confounded by cold-cache order — the cold pass ran 848 ms vs 335 ms
warm, so warmups first): **majors 3, 8, 9, 10 each cost ~25% ± 5 of the
pass; mem0-remainder ~16%.** Cycle count and inversion count are BOTH
poor single proxies (arm10: 42% of cycles but 2 ext_inv/row; arm3: 16%
of cycles but 37 ext_inv/row — they land in the same time bucket).

## M8b — POSEIDON1 (arm 10) hand-written direct accum kernel

Equal-largest share at trivial kernel complexity, and it validates the
special-major kernel shape arms 8/9 will reuse. The generated TopAccum
arm10 slice reduces to: the two `DoCycleTable` cycle-arg terms →
`columns[0]`, `columns[19] = columns[0]` (back-references read as zero in
the parallel phase — cross-row recurrences live in the terminal-prefix +
machine-column-carry passes), plus the BigInt nop user-state constants —
the exact recipe the six production direct arms already implement by
hand. NOT the SP7n zirgen-generated arm10 kernel (rejected 2026-05-18:
45 s of hidden queued GPU work on BusyLoop from the generated body).

Shape: `accum_poseidon1_direct_wgsl()` (scalar Montgomery WGSL, 2
cycle-terms + user constants), `AccumMiscDirectKind::Poseidon1` in the
grouped dispatch, rows = all major-10 cycles, `skip_major_mask |= 1<<10`,
default-enabled in `enable_webgpu_witgen_accum_acceleration_for_hal`,
xgboost gate asserts the dispatch counter moves.

### M8b gate (vs M7 landing baselines)

| gate | M7 landing | M8b |
|---|---:|---:|
| parity suite | 4/4 | TBD |
| BusyLoop | 2167 ms | TBD |
| KeccakUnion(1) | 35374 ms | TBD |
| xgboost | 14509 ms | TBD |

TBD: step_top span delta, hidden-queue check on the accum commit
(SP7n canary), receipts.
