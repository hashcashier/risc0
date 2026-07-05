# M10 — characterizing the intra-day drift (thermal hypothesis)

**Date:** 2026-07-05 · **Branch:** wasm-webgpu-prover-perf (bytes fixed @ 0c9e9c02b)
**Hypothesis (user, 2026-07-05):** the +3-9% intra-day drift on identical
bytes (M8 evidence: KeccakUnion 35374@15:21 → 38329@20:05; xgboost
14509 → 15109 through the evening of 07-04) is THERMAL — heat-soak under
sustained proving/compile load lowering boost clocks. Directive adopted:
measurements are repetitions + distribution comparisons from now on,
never single samples.

## Design

Fixed binary (last night's M9a landing bytes, prebuilt before the ladder so
no compile runs inside any rep). Overnight-idle cold start (GPU 47-50 °C,
machine quiet, vllm inactive, no reboot since 07-02 — so if the slow state
had been persistent driver/OS state, it would still be present this morning).

- **Ramp:** 5 back-to-back rounds of [xgboost, rep(BusyLoop+KeccakUnion)].
- **Cooldown:** 720 s forced idle.
- **Recovery:** 2 more rounds.
- **Telemetry:** 1 Hz GPU temp / SM clock / mem clock / power / util /
  throttle-reason bitmask (nvidia-smi) + CPU Tctl (k10temp) + DDR5 DIMM
  temp (spd5118).

Discriminating predictions:
- Thermal, fast time-constant: walls climb over ramp rounds with GPU/CPU
  temps, recover after the 12-min cooldown.
- Thermal, slow time-constant (case/VRM/room heat-soak): walls flat-cold
  across one ~25-min ladder; drift only reproduces after hours of load or
  higher afternoon room ambient.
- Persistent driver/OS state: cold-morning walls would have STAYED at the
  evening-slow level (already falsified by ramp1 — see below).

## Anchors on these exact bytes

Binary identity verified: every ladder rep and every one of last night's
gates executed the SAME unit `browser_prove-2d841e107f34e974.wasm`.

## Results — morning ladder (all receipts verified, tests_ok on every rep)

| fixture | phase | n | median | min | max | vs evening median (same bytes) |
|---|---|---:|---:|---:|---:|---:|
| KeccakUnion | ramp | 5 | 34224 | 33989 | 36199 | **−10.5%** |
| KeccakUnion | recovery | 2 | 34131 | 33744 | 34518 | −10.7% |
| xgboost | ramp | 5 | 14777 | 14463 | 14972 | −2.0% |
| xgboost | recovery | 2 | 14705 | 14646 | 14764 | −2.4% |
| BusyLoop | ramp | 5 | 2192 | 2160 | 2255 | −3.6% |
| BusyLoop | recovery | 2 | 2230 | 2213 | 2247 | −2.0% |

Ramp order effects: KeccakUnion 36199 → 34337 → 34146 → 33989 → 34224
(got FASTER under load; first run after the morning compile was the
outlier); xgboost bounced 14463..14972 with no trend. Recovery after the
12-min idle sits exactly in the warm band — nothing to recover from.

Telemetry (1 Hz, whole ladder): SM clock during active bursts median
2977 MHz, per-run means 2937-2977 — pinned, no degradation; the only
clock-event bit ever set was the routine transient SW power cap (0x4),
never a thermal-slowdown bit; GPU max 67 °C once (right after the
morning compile) then ≤63 °C; CPU Tctl bursts to 76-77.5 °C identically
every round; DIMM (in-case ambient proxy) dead flat 36.75-37.5 °C.
Fastest xgboost run had the LOWEST mean active clock — on a
submission-bound prover, core clocks are not the wall lever.

## Findings

1. **Not persistent state:** the slow evening state did not survive
   overnight idle, with NO reboot (uptime 2d18h) — driver/OS persistent
   state ruled out.
2. **Not proving-load thermal (this timescale):** 30 min of back-to-back
   proving did not move any temperature or clock and made walls slightly
   FASTER. A 12%-util submission-bound prover cannot heat this box.
3. **Not GPU-core thermal at all:** clocks pinned at 2977 MHz active in
   both the fast state (all day today) — and the evening samples' spans
   pointed at readbacks, not kernels, anyway.
4. **Within-state noise floor quantified:** ±1-2% per fixture. Yesterday's
   evening shift (+8-9% KeccakUnion) was 4-8× the noise band — a real
   bimodal state change, not sampling error. Single-sample deltas under
   ~2% are meaningless; REPS + DISTRIBUTIONS from now on.
5. **Workload ordering of the state gap:** KeccakUnion −10.5% ≫
   BusyLoop −3.6% ≈ xgboost −2.0%. The slow state taxes the
   readback/GPU-merkle-dense fixture hardest — same signature as the
   onset measurements in the M8 evidence.

## Heater probe (does all-core CPU load flip the state?)

Yesterday's onset window (15:30-16:30) contained two ~10-min all-core
native compile sessions — a much stronger heater than proving (GPU fans
idle while case air warms; GDDR7 junction not exposed on GeForce).
Probe: 25 min of `openssl speed -multi 32` (all cores, GPU idle), then
an immediate 3-round [xgboost, rep] block, sampler running throughout.

**Result: FALSIFIED.** The burn delivered the dose — CPU ≥94 °C for
1252 of 1352 s (pinned at the Tctl boost limit, exactly compile-like),
DIMM/case-air proxy 37.25 → 41 °C — and the immediate post-burn
measurements were the FASTEST block of the day. (The GPU sensor stayed
41-45 °C through the burn: case-air heating never reached the card. The
GPU-board side of the thermal space was covered by the ramp itself —
60-67 °C active bursts, clocks pinned, walls improving — so heat is
falsified from both sides at the doses this box actually experiences.)

| fixture | post-burn (n=3) | morning warm band |
|---|---|---|
| KeccakUnion | median 33913 [33826, 34612] (33826 = 2nd-best ever) | median 34224 |
| xgboost | median 14836 [14806, 14901] | median 14777 |
| BusyLoop | median 2173 [2171, 2227] | median 2192 |

A 2.5× dose of the suspected trigger produced zero effect. CPU/case
heat is ruled out alongside GPU-core thermal, proving-load thermal, and
persistent driver/OS state.

## M10c — memory-churn probe (compiles churn memory, openssl doesn't)

What yesterday's onset window and this morning's ramp-1 outlier share is
a preceding COMPILE (two ~10-min all-core native sessions at 15:30-16:30
on 07-04; the 9m42s wasm rebuild at 09:47 today). What a compile does
that the openssl burn doesn't: massive page-cache turnover, allocator
pressure, THP/contiguity fragmentation. The slow state's tax
concentrates in readbacks (mapAsync + host memcpy) — host-memory-path
operations; overnight-idle recovery fits kcompactd/khugepaged catching
up. Fast-state baseline: buddyinfo Normal order-10 = 727 free blocks,
MemAvailable 87.5 GB, page cache 72.6 GB.

Probe: cold `cargo check --workspace --tests` into a throwaway target
dir (timeout 900 s), buddyinfo/meminfo bracketing, then an immediate
3-round [xgboost, rep] block.

**Result: FALSIFIED.** The churn delivered: 688 s of all-core
compilation writing 7.7 GB of artifacts; buddyinfo Normal order-10 free
blocks 727 → 555 (mild), MemAvailable unmoved (~89.6 GB — this box has
too much headroom for one build to dent). Post-churn block:

| fixture | post-churn (n=3) | note |
|---|---|---|
| xgboost | 14429 / 14801 / 15174 | 14429 = fastest EVER, on the first post-churn run; 15174 = day's high outlier |
| KeccakUnion | 34432 / 34465 / 33856 | all firmly fast-state |
| BusyLoop | 2277 / 2070 / 2262 | 2070 = fastest ever |

The churn-3 xgboost outlier (15174) was immediately followed by a
33856 KeccakUnion — an xgboost noise excursion, not a state flip.

## Verdict

1. **The bimodal state is real**: evening-of-07-04 slow state
   (KeccakUnion 37.1-38.3 K across 5+ samples) vs everything measured
   today (33.7-34.6 K warm band, 22 KeccakUnion samples) on identical
   bytes — a +9-10.5% state gap, 4-8× the noise floor.
2. **It resets on long idle without a reboot** (uptime 2d18h across the
   transition).
3. **It is NOT reproduced by** (all falsified today with rep
   distributions on fixed bytes): 30 min of back-to-back proving load;
   25 min of CPU-at-thermal-limit burn (+case air); 11.5 min of
   compile-shaped memory churn. GPU-core thermal additionally excluded
   by pinned clocks/no throttle bits in both states' load profiles.
4. **Noise floors (today, fast state)**: KeccakUnion ±1% (excluding
   first-of-day), xgboost ±2.5%, BusyLoop ±5%. First proof after long
   idle reads high (+5.8% KeccakUnion) — warmup, discard it.
5. **Remaining candidates**: time-of-day/room ambient (the slow state
   has only ever been observed 16:00-21:45 on a July afternoon;
   passively discriminated by any late-afternoon session — the M11 gate
   blocks will provide this) or a one-off driver/GSP event on 07-04
   (unfalsifiable retroactively; if the state never recurs, this was it).

## Standing policy (locked in regardless of mechanism)

- Reps + distributions (n≥3-5/arm, medians + spread), A/B back-to-back
  in-session, quiet machine, no compiles interleaved.
- Discard the first proof after long idle.
- **Keccak canary** before/after gate blocks: one rep-fixture run;
  KeccakUnion <35 K ⇒ fast state, >36.5 K ⇒ slow state (re-baseline
  everything, note wall numbers are state-relative).
- If the slow state is ever caught live: capture nvidia-smi -q
  PERFORMANCE + buddyinfo + a rep distribution BEFORE touching anything,
  then try, in order: 10-min idle; `systemctl suspend`/resume probe;
  reboot — record which one resets it.

Scripts: thermal-sampler.sh, run-thermal-ladder.sh, run-heater-probe.sh,
run-churn-probe.sh + all CSVs/logs in the 07-05 session scratchpad.
