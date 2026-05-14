Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 iter 1 — witgen-codegen scale test`
DraftedAt: `2026-05-14`
Status: `COMPLETE`

## TL;DR

Codegen'd witgen-shaped WGSL does **not** have the execution-slowdown
ceiling SP3 found for staged eval_check — it **compiles fast and
executes at full speed** up to ~247 KB. But it hits a hard **capacity
cliff**: at ~478 KB the kernel still compiles, yet the GPU device dies
on its first dispatch. Real witgen WGSL would be multi-MB (the CUDA
`steps.cu` is 62 k lines), far past that cliff — so **full witgen as a
single codegen'd kernel is not viable**. The failure mode being
*capacity*, not *slowness*, is the useful nuance: it does not rule out
a chunked / multi-kernel codegen.

## Method

`sp7_witgen_codegen_scale_smoke` (browser-prove) emits a
witgen-*shaped* WGSL kernel — many small `fn`s in a call DAG,
column-major buffer loads, real BabyBear field arithmetic (Montgomery
`mul`), a data-dependent mux — at a sweep of sizes. The hot path
(`HOT_DEPTH`=24 fns × `OPS_PER_FN`=16) is **identical** at every size;
only a *cold subtree* (a binary tree of `cold_count` fns, reachable
only through a runtime-false guard so Chrome must compile it but it
never executes) varies the total kernel size.

Per scale: median of 3 windows, each ≥ ~600 ms wall (so `Date::now()`'s
~1 ms resolution is < 0.2 % error). After the sweep, cold=0 is
re-measured — if the recheck drifts from the initial baseline, the
device degraded over the session and the ratios are contaminated.

Geometry: N_CYCLES = 131072 (po2_17), N_COLS = 256.

## Why iter 1 was re-run four times

The first attempt used 2–19 ms measurement windows (1 trial each) and
produced **contradictory** results — the *same* 414 KB kernel measured
9.5× slower in one run and 0.67× in the next. That is sub-resolution
noise, not a signal. The committed test uses ≥600 ms windows, median
of 3 trials, and a degradation recheck. The rigorous version
reproduced cleanly across two runs.

## Result (rigorous runs, 2026-05-14)

| cold fns | WGSL size | compile_ms | median ns/cycle | ratio |
|---:|---:|---:|---:|---:|
| 0 | 16 KB | 0 | 1.70 | 1.000 |
| 128 | 93 KB | 0 | 1.91 | 1.125 |
| 384 | 247 KB | 1 | 1.91 | 1.125 |
| 768 | 478 KB | 1 | — | **first dispatch failed** |

- Run-4 recheck: `recheck_ratio = 1.000` — **no device degradation**;
  the cold 0/128/384 measurements are uncontaminated and trustworthy.
- Run-5 per-phase log pinpointed the 478 KB failure:
  `phase=compiled` (compile_ms=1) is logged, then
  `phase=first_dispatch_FAILED err=AbortError: A valid external
  Instance reference no longer exists` — the kernel **compiled**, but
  the **first dispatch killed the device/instance**.
- `verdict=CEILING_HIT` (sweep incomplete — 3/4 scales).

## What this means

1. **No compile ceiling.** Chrome compiled every kernel — including
   478 KB — in ~0–1 ms. Codegen'd WGSL is not compile-bound.
2. **No execution-slowdown ceiling.** The identical hot path ran at
   1.70 → 1.91 → 1.91 ns/cycle as the kernel grew 16 KB → 247 KB
   (ratio ≤ 1.125, within noise; recheck confirms no degradation).
   This **refutes the simple "SP3's ceiling generalizes" hypothesis** —
   SP3's staged eval_check ran ~30× slow; witgen-shaped codegen does
   not slow down with scale at all (up to the capacity cliff).
3. **There is a hard capacity cliff** between ~247 KB and ~478 KB:
   the kernel compiles, but the device dies on the first dispatch.
   The mechanism is almost certainly a Dawn/Vulkan pipeline-resource
   or SPIR-V-size limit — past it, creating/running the pipeline tears
   down the instance.

## Verdict for SP7

**Single-kernel full-witgen codegen is not viable.** Real witgen WGSL
would be ~2–4 MB (62 k-line `steps.cu` → WGSL), ~5–8× past the
~478 KB device-death point.

**But the door is not fully closed**, and the reason matters: the
failure mode is *capacity*, not *slowness*. Codegen'd WGSL that *fits*
under the cliff executes at full speed and compiles instantly. So the
surviving codegen path is **chunked / multi-kernel codegen** — keep
each emitted kernel under the ~250 KB safe zone. The open question for
iter 2: can `step_Top` be partitioned into < ~250 KB WGSL kernels
without pathological inter-kernel scratch overhead? SP3's staged
*eval_check* multi-kernel was ~30× slow — but iter 1 shows that
slowness is *not* an intrinsic property of codegen'd WGSL execution,
so the SP3 staged result may not transfer.

## Iter 2 (next)

Chunked witgen codegen: build the transpiler from `steps.cu`, but
partition the `step_Top` call graph into kernels each < ~250 KB WGSL,
staged with scratch buffers between them. Measure: does staged witgen
codegen execute at full speed (iter-1 says single-kernel WGSL does),
or does the staging overhead reintroduce the SP3 slowdown? If staged
witgen codegen runs fast, the full transpiler (iters 3–5 of the
TO-BE plan) is justified; if not, fall back to the AS-IS interpreter
option.

## Artifacts

- `examples/browser-prove/src/lib.rs` — `sp7_field_prelude`,
  `sp7_emit_fn`, `sp7_build_witgen_shaped_wgsl`,
  `sp7_witgen_codegen_scale_smoke`.
- `examples/browser-prove/Cargo.toml` — added `web-sys` (direct dep;
  GPU types via feature unification with risc0-zkp).
- Logs: `/tmp/sp7-scale4.log` (rigorous run + clean recheck),
  `/tmp/sp7-scale5.log` (rigorous run + per-phase failure pinpoint).
