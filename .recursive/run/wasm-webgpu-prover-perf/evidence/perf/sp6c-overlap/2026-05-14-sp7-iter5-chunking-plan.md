Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 iter 5 — chunk the validated WGSL module under the device cliff`
DraftedAt: `2026-05-14`
Status: `BLOCKED — the iter-1 cliff is a phantom; the browser WebGPU
device is in a broken environmental state. Needs machine-level
intervention before iter 5/6 can proceed.`

## iter 5a probe result (2026-05-14) — the cliff is a phantom

`sp7_cliff_reachability_smoke` (browser-prove) ran in Chrome 3×:

| run | adapter `max_buffer_size` | result |
|---|---|---|
| 1 | 1 GB (`1073741824`) | **all OK** — incl. a 1.87 MB unreachable-cold module AND the reachable cold=768 (478 KB) "control" iter-1 said dies |
| 2 | 4 GB (`4294967292`) | **all FAIL** — incl. a 16 KB module: `AbortError: A valid external Instance reference no longer exists` |
| 3 | 4 GB | **all FAIL** — identical |

A 16 KB module failing its first dispatch+readback cannot be a
module-size cliff. `nvidia-smi`: GPU 880 MiB / 32 GB used, 0 compute
processes, 3% util — **not memory contention**. `systemctl is-active
hermes-vllm` → `failed` (the local inference service is down).

**Conclusions:**
1. **iter-1's "~478 KB capacity cliff" was device flakiness, not a
   real limit.** A healthy device (run 1) dispatched a 1.87 MB module
   fine. iter-1's 4-step sweep ran thousands of timing-loop dispatches
   before reaching cold=768 on an already-degraded device. **This
   removes the premise for iter 5 chunking** — the full ~4.7 MB witgen
   module may well dispatch on a healthy device.
2. **The browser WebGPU device on this machine is in a broken state
   right now** — 2 consecutive fresh-Chrome runs fail even trivial
   modules. The GPU is idle, so it is not contention. Earlier in this
   session the browser prover ran R1/xgboost proofs fine, so this
   degraded sometime today. A GPU/driver reset or reboot is likely
   needed — a destructive, machine-level action requiring the user.

## Revised iter 5 / iter 6 plan (pending the device fix)

- iter 5 (chunking) is **probably unnecessary** — re-confirm by
  re-running the capacity probe once the device is healthy. If a
  healthy device dispatches a ~5 MB module, skip straight to iter 6.
- iter 6 (wire `steps.wgsl` into `WebGpuCircuitHal`) — the *code* can
  be written without a working browser GPU; only the SP-CR
  byte-identical check + the A/B measurement need the device.

---

## (original plan — superseded by the probe result above)

Status: `PLANNING — empirical cliff probe must come first`

## Where iter 4 left things

The zirgen WGSL backend (branch `wgsl-gpu-backend`, commits
`6be1503`→`2741352`) emits a fully naga-valid WGSL witgen module for
rv32im: `witgen_prelude.wgsl` (13 KB) + `types.wgsl.inc` (524 KB) +
`layout.wgsl.inc` (254 KB) + `steps.wgsl` (3.9 MB) = ~4.7 MB, 41 374
lines, 211 `fn` defs.

The iter-1 capacity cliff: a single WGSL module/pipeline dies on its
first dispatch somewhere between ~247 KB and ~478 KB of WGSL source.
The full module is ~10× over. So it must be chunked (iter-2 validated
the staged-`@compute`-kernel + scratch-handoff model: ~0.10 ns/cycle
per stage boundary, no SP3-class catastrophe).

## iter-5 measurements (2026-05-14)

Compiled the full module + a `@compute` entry to SPIR-V with `naga`:

| entry | SPIR-V size |
|---|---:|
| `fn nop() {}` (calls nothing) | 3 406 924 B |
| `step_Top(...)` | 3 407 160 B |
| `step_TopAccum(...)` | 3 407 172 B |

**naga does not DCE** — the trivial entry's SPIR-V is the same 3.4 MB
as the real entries. naga emits every module function regardless of
reachability.

`step_TopAccum` is a 6-line shim: it builds two `BoundLayout`s and
calls `exec_TopAccum` — all accum work is in helpers. `step_Top`'s body
is lines 14 801–27 836 (~13 K lines) and makes 10 distinct direct
calls (transitively many more).

## The pivotal unknown — and why iter 5 must probe first

The iter-1 cliff fires at **dispatch** time ("first dispatch killed
the device"), and a dispatch runs a **pipeline**, which is
per-entry-point. So the device (Chrome → Tint) cliff may well be about
**code reachable from the entry point**, even though naga's
whole-module SPIR-V backend is not reachability-pruned. Which it is
decides the entire chunking strategy:

- **If reachable-code:** chunking = split `step_Top`'s body into N
  segments; each segment's `@compute` entry + its transitively-reached
  helpers must be < cliff. The module can still *contain* all
  types/layouts/helpers — Tint strips the unreached ones per pipeline.
  Moderate: a body-split + liveness + scratch pass, no pruning.
- **If whole-module:** each chunk must be emitted as a *minimal
  self-contained* module (only the types/layout-constants/helpers it
  uses). Hard: split + per-chunk transitive pruning.

`feedback_verify_assumptions_before_building`: do NOT build the
chunking transformation against a guess. **iter 5 step 1 is an
empirical cliff probe in Chrome.**

## iter 5 plan

- **5a — empirical cliff probe (browser-prove smoke).** Build a smoke
  that takes the real generated WGSL, wraps a `@compute` entry, and
  reports: does module creation succeed? pipeline creation? first
  dispatch? Vary which entry / how much code is reachable. Answers:
  (1) reachable-code vs whole-module cliff, (2) the real threshold for
  the actual `steps.wgsl` shape (211 small fns + one big entry — may
  behave differently from iter-1's synthetic single kernel).
- **5b — chunking transformation** (design per 5a's answer). The
  iter-2 staged model: partition `step_Top`'s body into N segments,
  liveness analysis for live-in/live-out across segment boundaries, a
  `scratch` storage buffer for the handoff, each segment emitted as
  `step_Top_chunk_i`. Likely a new MLIR pass in zirgen (mirrors how
  `createUnrollPass` is a WGSL-only clone-pass in `gen_zirgen.cpp`).
- **5c — validate** each chunk module with naga + the cliff probe.

## Note — possible faster path to a measurement

`step_TopAccum`'s reachable closure may be smaller than `step_Top`'s.
If it (or an early `step_Top` chunk) fits under the cliff, iter 6 could
wire up just that first, getting a real GPU-witgen-vs-CPU `rust_steps`
measurement before the full chunking pass is built. Decide after 5a.
