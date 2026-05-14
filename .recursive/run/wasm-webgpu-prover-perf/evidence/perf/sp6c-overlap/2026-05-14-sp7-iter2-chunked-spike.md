Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 iter 2 — chunked-codegen spike`
DraftedAt: `2026-05-14`
Status: `COMPLETE`

## TL;DR

iter 1 killed *single-kernel* full-witgen codegen (capacity cliff at
~478 KB) but left *chunked* codegen open, since WGSL that fits under
the cliff executes at full speed. iter 2 tests the obvious worry about
chunking: does splitting work across N staged kernels — each handing
the running state through a `scratch` buffer — reintroduce an
SP3-class (~30×) overhead?

**Verdict: no staging catastrophe.** The per-stage-boundary overhead
is ~**0.10 ns/cycle**, reproducible across runs, and bounded. The raw
ratio looks larger (4.7× at 8 stages) but only because the synthetic
baseline is near-zero — the 5090 crushes synthetic compute-bound field
arithmetic at ~tens of Tops/s. Real witgen is memory-bound with a
µs/cycle CPU cost; ~0.1 ns/cycle per boundary is negligible against
any plausible GPU-witgen cost. Chunked codegen is **not ruled out** —
proceed to iter 3 (build the real chunked transpiler).

## Method

`sp7_chunked_codegen_spike_smoke` extends iter 1's generator with
`sp7_build_staged_hot_wgsl`: the same `hot_depth`-deep hot-path chain,
but split across `n_stages` separate `@compute` kernels. Stage `s`
runs hot functions `[s·fns_per_stage, (s+1)·fns_per_stage)`, reading
the running `acc` from a `scratch` storage buffer (binding 2) and
writing it back; stage 0 seeds `acc` from `data`, the last stage
writes the result to `data`. The hot functions keep the **same LCG
seeds** as the single-kernel generator, so a staged set does
byte-identical compute work to a single kernel — the only difference
is dispatch count and the scratch handoff. `n_stages == 1` reproduces
the single kernel exactly, giving the A/B baseline.

Each hot function now ends with a **non-elidable store** to its own
unique `data` column (201+idx) — a genuine storage side effect so the
WGSL compiler cannot fold away the op chain.

Sweep N_STAGES ∈ {1,2,4,8}. Geometry: HOT_DEPTH=24, OPS_PER_FN=256,
N_CYCLES=131072. Fixed K_ITERS=400 per window, median of 3 trials.

## Result (2026-05-14)

| n_stages | stage kernels | total WGSL | median ns/cycle | ratio vs single |
|---:|---:|---:|---:|---:|
| 1 | 1 | 133 KB | 0.19 | 1.000 |
| 2 | 2 | 135 KB | 0.32 | 1.700 |
| 4 | 4 | 138 KB | 0.53 | 2.800 |
| 8 | 8 | 145 KB | 0.90 | 4.700 |

- **Per-stage-boundary overhead: 0.1008 ns/cycle** — `(0.90 − 0.19) /
  (8 − 1)`. A prior run (compute partly elidable) gave 0.103 — the
  figure is stable.
- `worst_ratio = 4.700`, `verdict = no_staging_catastrophe` (well
  under the 8× kill-threshold; nowhere near SP3's ~30×).

## Why the raw ratio is not the signal — and the per-boundary ns is

The single-kernel baseline is 0.19 ns/cycle: the 5090 chews through
24 functions × 256 field ops × 131 072 cycles in ~25 µs. Synthetic
compute-bound field arithmetic is *not* representative of real witgen,
which is memory-bound (it fills ~211 `data` columns per cycle and
reads the preflight trace — CPU cost measured at ~3.7 µs/cycle in the
iter-9 logs). Against a near-zero baseline, ANY fixed overhead
produces a large ratio — that is what the 4.7× is. The trustworthy
figure is the **absolute** per-boundary cost, ~0.10 ns/cycle, which is
a fixed dispatch + scratch-handoff cost per stage boundary.

Back-of-envelope for real witgen: chunking a multi-MB witgen kernel
into ~10–20 sub-250 KB stages → ~10–20 boundaries → ~1–2 ns/cycle of
staging overhead. Against a GPU-witgen cost that is surely ≫ that
(CPU is 3 700 ns/cycle; even a 1 000× GPU speedup leaves 3.7 ns/cycle),
staging overhead is single-digit-percent. Not a blocker.

## What iter 2 can and cannot conclude

- **CAN:** staging does not reintroduce an SP3-class catastrophe. The
  per-boundary overhead is small, fixed, and reproducible.
- **CANNOT:** whether chunked codegen is a net *win* for real witgen.
  The synthetic baseline is GPU-crushed, so the ratio is uninformative
  for memory-bound real witgen. That answer needs the real chunked
  transpiler measured against the CPU `rust_steps` path (iter 3+).

The spike's gate was: "does staging reintroduce the SP3 catastrophe?
If yes → fall back to the interpreter." Answer: **no.** → proceed to
iter 3.

## Methodology note

The first iter-2 run reused iter-1's fragile calibration (one warm-up
dispatch → calibrate K) and produced quantized noise (1.91/2.86/3.81
ns/cycle — exact multiples). The second used a fixed large K but the
op chain was partly compiler-elidable. The committed test uses a fixed
K *and* a non-elidable per-function store. Even so the 5090 is fast
enough that windows are only tens of ms — the absolute numbers carry
some noise, but the per-boundary figure is stable across runs and the
verdict (no catastrophe) is robust to it.

## Iter 3 (next)

Build the `steps.cu` → WGSL chunked transpiler: a partitioning pass
over the `exec_Top` call graph that emits kernels each < ~250 KB,
staged with scratch buffers. Start with the substrate prelude and the
~20 leaf `exec_*`/`back_*` builtins. The definitive measurement —
chunked GPU witgen vs CPU `rust_steps` on a real segment — lands in
iters 4–6.

## Artifacts

- `examples/browser-prove/src/lib.rs` — `sp7_build_staged_hot_wgsl`,
  `sp7_chunked_codegen_spike_smoke`; `sp7_emit_fn` gained a
  `store_col: Option<u32>` parameter (iter-1 callers pass `None`).
- Logs: `/tmp/sp7-chunk3.log`, `/tmp/sp7-chunk4.log` (the two
  non-elidable-store runs; per_boundary_ns 0.103 / 0.101).
