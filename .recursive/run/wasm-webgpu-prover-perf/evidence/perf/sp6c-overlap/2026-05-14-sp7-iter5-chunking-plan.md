Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 iter 5 — chunk the validated WGSL module under the device cliff`
DraftedAt: `2026-05-14`
Status: `iter 5c COMPLETE — TWO ceilings confirmed for the real-module
shape (whole-module in (1.99, 3.27] MB; reachable < step_Top's 1.68 MB
closure); BOTH step entries need body-splitting. iter 5d: exec_Top
arm-split prototyped + naga-validated; sp7_chunk_sweep_probe running to
pin the reachable ceiling and confirm arm-split chunks dispatch.`

## iter 5a FINAL (2026-05-14, post-reboot, NVIDIA ICD forced)

The "broken device" was **Vulkan ICD mis-selection**: with `nvidia`,
`lvp` (lavapipe), and `nouveau` ICDs all installed, Chrome's Dawn
non-deterministically picked a broken Mesa device. **Fix:
`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json`** on the
browser test invocation. iter 6 must set this.

With the NVIDIA ICD forced, the refined probe
(`sp7_cliff_reachability_smoke`, fresh `WebGpuHal` per probe, ordered
passes-first) gave a clean, decisive verdict:

| probe | module | reachable cold? | result |
|---|---:|---|---|
| anchor_lo | 1.87 MB | n/a (sub-cliff) | **OK** |
| unreachable_cold6144 | 3.73 MB | **no** | **OK** |
| reachable_cold6144 | 3.73 MB | **yes** | **FAILED** (device lost) |
| many_dispatch (×1000) | 478 KB | n/a | **OK** |

**`cliff_type = reachable_code`.** Same 3.73 MB module: unreachable
cold → dispatches; reachable cold → device lost. So the device/Tint
capacity cliff counts **code reachable from the `@compute` entry**
(post-DCE, per-pipeline), NOT whole-module source size. The reachable
cliff is **~1.9–2.8 MB** (cold=3072/1.87 MB OK, cold=4608/2.80 MB FAIL
from the prior run) — iter-1's "478 KB" was the broken-Mesa-device
artifact. No degradation: a sub-cliff module survived 1000 dispatches.

Operational fact for iter 6: a device-loss makes `requestAdapter()`
fail for the **rest of that Chrome process** — recovery needs a fresh
process.

## iter 5b STAGED PROBE verdict (2026-05-14) — a SECOND ceiling

The call-graph analysis below assumed the only constraint was the
reachable-closure cliff. The staged real-module probe (`sp7_probe`
ladder) tested `witgen_nop` — a trivial `@compute` entry that touches
all 5 bindings but reaches ZERO step functions — against three
real-module subsets:

| subset | contents | size | result |
|---|---|---:|---|
| prelude_mod | witgen_prelude.wgsl only | 13 KB | **OK** |
| typed_mod | + all types + all layout (no step fns) | 791 KB | **OK** |
| full_mod | + all 211 step fns | 4.69 MB | **dispatch_FAILED** (device lost) |

`witgen_nop` reaches no step functions, yet adding the 3.9 MB of
(unreachable!) `steps.wgsl` killed the device. `typed_nop` at 791 KB —
with all 863 structs, 1206 layout consts, 2279 helper fns — passed, so
it is NOT the type/layout structure; it is the bulk of `steps.wgsl`.

**There are TWO ceilings:**
1. **whole-module ceiling ∈ (3.73, 4.69] MB** — above this the module
   loses the device for ANY entry, even one reaching nothing. (3.73 MB
   is iter-5a's `unreachable_cold6144` pass; 4.69 MB is this fail —
   both needed, because iter-5a showed unreachable-cold at 3.73 MB
   passed, so per-pipeline DCE *does* work *below* this ceiling.)
2. **reachable-closure ceiling ~1.9-2.8 MB** — iter-5a, still holds
   *below* the whole-module ceiling.

**This overturns the original iter-5b plan** ("one module, staged
`@compute` entries"): the 4.69 MB module is itself over the
whole-module ceiling, so it must be SPLIT, not just have its entries
chunked. The corrected strategy is **per-entry pruned modules** — see
iter 5c.

## iter 5c — pruned per-entry module probe (2026-05-14)

`/tmp/wgsl-test/prune_closure.py` computes each step entry's transitive
call closure over `steps.wgsl` + `types.wgsl.inc` and emits a pruned
steps file (closure's `steps.wgsl` fns only; `types.wgsl.inc` +
`layout.wgsl.inc` kept wholesale — the 791 KB baseline already probed
OK and Tint DCEs unreached helpers per-pipeline):

| entry | closure fns | pruned steps | whole module | reachable closure |
|---|---:|---:|---:|---:|
| step_Top | 2328 (195 steps + 2133 helpers) | 1265 KB | **1.99 MB** | 1.68 MB |
| step_TopAccum | 733 (29 steps + 704 helpers) | 2572 KB | **3.27 MB** | 2.68 MB |

`sp7_pruned_top_probe` / `sp7_pruned_accum_probe` (separate
wasm-bindgen-test-runner invocations → fresh Chrome each, no
contamination; `--nocapture` to surface metrics — the runner hides
console.log for *passing* tests).

**iter 5c RESULTS (2026-05-14):**

| probe | entry | module | result |
|---|---|---:|---|
| sp7_pruned_top | witgen_nop | 1.99 MB | **OK** |
| sp7_pruned_top | witgen_top | 1.99 MB | **dispatch_FAILED** (device lost) |
| sp7_pruned_accum | witgen_nop | 3.27 MB | **dispatch_FAILED** (device lost) |

Two findings, both *tighter* than iter-5b's synthetic-based bounds:
1. **Whole-module ceiling ∈ (1.99, 3.27] MB** for the real-module
   shape — `nop` OK at 1.99 MB, `nop` FAILED at 3.27 MB. (iter-5b's
   (3.73, 4.69] came from a *synthetic* module; the real shape's
   ceiling is lower.)
2. **The reachable ceiling is < step_Top's 1.68 MB closure** —
   `witgen_top` failed in the *same 1.99 MB module* where `nop`
   passed, so per-pipeline DCE works but step_Top's closure is over
   the ceiling. **BOTH step entries need body-splitting**, not just
   step_TopAccum.

Corrected design: **per-entry pruned module (clears whole-module
ceiling) + body-split the giant step entry into staged `@compute`
chunks within it (clears reachable ceiling)** — a *combination* of
iter-5b's two rejected plans.

## iter 5d — chunking anatomy + arm-split (2026-05-14)

`closure_anatomy.py` + `chunk_plan.py`: the giants are flat MUX chains,
not splittable SSA. `step_Top` is a 6-line shim → `exec_Top`, an 8 KB
**dispatcher** with a flat 13-arm mux (one arm per instruction class,
each ~3 lines calling one `exec_<Class>` worker, all assigning the
escaping `var x20`). `step_TopAccum` → `exec_TopAccum` (831 KB own
flat-mux body) which reaches `exec_TopExtract` (1674 KB own flat-mux
body). So the split unit is the **mux arm**.

`split_exectop.py` arm-splits `exec_Top`: prologue (45 lines) + N-way
arm partition + epilogue (9 lines). Computed per-chunk closures: **N=4
→ ~0.5 MB, N=2 → ~0.9 MB** — both far under the 1.68 MB that failed.
prologue+epilogue shared closure is only **0.01 MB** (the arms' closures
overlap with *each other*, not the prologue). naga-validates clean.

**iter 5d on-device probes — the Python text-splicer is Tint-pathological.**
Every spliced chunk device-loses after **~60–95 s** (whether `-> TopStruct`
+ `_ =` form OR the void-shim form byte-identical to the working
`witgen_top_full`; whether robustness on or off). The control
`witgen_top_full` (unsplit `step_Top`→`exec_Top`, 1.68 MB closure)
device-loses *fast* (~3 s, clean — capacity cliff at pipeline creation);
`witgen_nop` on the same 2.12 MB chunked module is OK. So: the chunked
module + harness are fine; my **Python text-splice produces WGSL that
grinds Tint** (naga-valid + brace-balanced, but Tint-pathological).
The splicer was always a *probe prototype* — it confirmed structure,
closure sizes, and naga-validity, but its text-spliced output is not
Tint-clean. **De-risk in flight:** single-arm spliced chunks
(`step_chunk_arm3` ~0.11 MB, `step_chunk_arm11`/Sha ~0.39 MB) — if even
a 1-arm splice grinds, the splice *structure* is broken (→ the MLIR
pass, which emits idiomatically via `WgslLanguageSyntax`, is the fix
and will be Tint-clean by construction).

**MLIR-pass chunking design (no scratch buffer needed for step_Top):**
each chunk = `prologue` (idempotent — witgen `store`s are deterministic,
so recomputing per chunk is correct) + `switch(this group's arms)` +
`if(OR of this group's selectors) { epilogue }`. The `if`-guard means
only the chunk whose arm actually fired runs the epilogue → non-firing
chunks can't overwrite real witness values. Exactly one chunk does real
work per cycle. Pass location: a new `ZStruct/Transforms/` pass
(splits `ZStruct::SwitchOp`), registered in `Passes.td`, called in
`gen_zirgen.cpp` right after `createUnrollPass` on `wgslStepFuncs`.

## iter 6a — MuxChunk MLIR pass: WORKS (2026-05-14)

`zirgen/Dialect/ZStruct/Transforms/MuxChunk.cpp` — written, compiled,
runs, **produces naga-valid chunked WGSL**. Splits a wide
`zstruct.switch` into per-arm-group chunk `zhlt.step_func`s: idempotent
prologue (cloned) + a restricted switch carrying ONLY that group's
selectors+arms (the WGSL `else{}` handles "no arm fired" → zero default)
+ the epilogue inside a single-selector guard switch keyed on the sum of
the group's selectors (only the firing chunk runs the epilogue → no
scratch buffer). Registered in `Passes.td`/`Passes.h`/`BUILD.bazel`,
called in `gen_zirgen.cpp` after `createUnrollPass` on `wgslStepFuncs`.
Output: `exec_TopChunk0..6`, `exec_TopExtractChunk0..6`,
`exec_TopAccumChunk*`, plus sub-worker chunks (302 fns vs 211); the
assembled module **naga-validates clean** ("Duplicate type name Reg" is
a benign warning — both ref types share the prelude's `Reg`). Two
runtime bugs fixed: chunk `StepFuncOp` needs `argNames` passed (verifier
wants `arg_attrs.size()==numArgs`); reset the builder insertion point
each loop iteration.

**Limitation — non-recursive.** The pass splits each function's OWN
switch, but the call graph is unchanged: `exec_TopChunk5` (with the Sha
arm) still calls the full `exec_Sha0`, so its closure stays big. Next:
recursion — either (a) `kMaxArmsPerChunk=1` + call-rewrite (a chunk
calling a chunked worker `@W` spawns variants calling `@WChunk_j`,
iterated to fixedpoint), or (b) split wide switches at any nesting
depth. Then: emit per-leaf-chunk `@compute` entries, wire into risc0
`WebGpuCircuitHal`, SP-CR byte-identical check, measure vs CPU
`rust_steps`.

## iter 5b call-graph analysis (2026-05-14) — chunking may be barely needed

Reachable-closure analysis of the real `steps.wgsl` (211 fns, 3.72 MB)
— for each step entry, the byte size of {entry} + {fns it transitively
calls}, i.e. what Tint keeps per-pipeline:

| entry | closure fns | closure bytes |
|---|---:|---:|
| `step_Top` | 195 | **1.24 MB** |
| `step_TopAccum` | 29 | **2.51 MB** |

Surprises:
- **`step_Top`'s closure is only 1.24 MB** — *under* the ~1.9 MB
  confirmed-OK point. It delegates to 195 *small* functions (~6 KB
  avg). It may not need chunking at all (pending: do referenced
  type/layout defs count toward the cliff? — the iter-5a probe used
  function-only modules).
- **`step_TopAccum`'s closure is 2.51 MB** from just 29 functions —
  the accum path has giants (`exec_TopAccum` 2.51 MB, `execUser_Accum`
  1.70 MB, `exec_TopExtract` 1.66 MB). This is the entry that needs
  chunking — *if* 2.51 MB is over the real cliff (it's inside the
  1.9–2.8 MB uncertainty band).
- step_Top and step_TopAccum reach nearly-disjoint fn sets
  (195 + 29 ≈ 211) — witgen and accum are separate subgraphs.

**Revised plan:** before building any chunking pass, run a *real-module*
device probe — embed the actual `witgen_prelude.wgsl` + `types.wgsl.inc`
+ `layout.wgsl.inc` + `steps.wgsl`, add `@compute` entries for
`step_Top` and `step_TopAccum` with the 5 prelude buffers, and test
dispatch per-entry (`VK_ICD_FILENAMES` forced). If both dispatch →
**iter 5b is unnecessary, go straight to iter 6.** If only
`step_TopAccum` fails → chunk just that one (likely 2 chunks).

## (original) iter 5b plan — staged `@compute` entries, ONE module (no pruning)

Because the cliff is reachable-code-based, chunking is much simpler
than the whole-module case would have been:

- Emit **one** WGSL module containing the prelude + all type defs +
  all layout constants + all 211 step functions (the 4.7 MB module
  iter 4 already produces and naga-validates).
- Partition `step_Top`'s body (and `step_TopAccum`'s) into N contiguous
  segments. Each segment becomes a separate `@compute` entry
  `step_Top_chunk_i` whose transitively-reachable code is < ~1.5 MB
  (safe margin under the ~1.9 MB-confirmed-OK point). Tint DCEs
  everything not reached from that entry per-pipeline, so the 4.7 MB
  module is fine — only each entry's closure counts.
- Liveness analysis across segment boundaries: values defined in
  segment i and used in segment j>i are live-outs → written to a
  `scratch` storage buffer; segment j reads them back as live-ins
  (the iter-2 staged model — ~0.10 ns/cycle per boundary, no
  catastrophe).
- Estimated ~3–4 chunks for `step_Top` (~4.5 MB reachable / ~1.3 MB
  budget), ~1–2 for `step_TopAccum`.
- Likely a new MLIR pass in zirgen (mirrors how `createUnrollPass`
  runs on a WGSL-only clone in `gen_zirgen.cpp`): split the
  `Zhlt::StepFuncOp` body, compute liveness, emit per-segment
  `StepFuncOp`s with scratch read/write prologue/epilogue.
- Validate each chunk-entry's reachable closure stays < cliff with the
  probe harness; then iter 6 wires it in.

---

## (superseded) earlier status + plan

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
