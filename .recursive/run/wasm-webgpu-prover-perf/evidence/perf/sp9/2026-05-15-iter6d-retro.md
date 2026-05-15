Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 iter-6d retrospective -- why iter-6d-c stops here, what iter-6d-d/e need`
Date: 2026-05-15

## What iter-6d set out to do

Replace `super::rust_steps::generate_witness` in
`risc0/circuit/rv32im/src/prove/hal/webgpu.rs` with a GPU-resident
WGSL kernel. The expected savings per
[[project_sp7_witgen_savings_ceiling]]: 5.7 s (520 ms × 11 segments)
on the xgboost wall, dropping it from 103 s to ~97 s.

## What iter-6d-a/b/c landed

- **iter-6d-a** (5d4a37d7c): vendored
  `risc0/circuit/rv32im/src/zirgen/exec_top_chunk0.wgsl` (1.08 MB,
  sub-cliff for Chrome's whole-module + reachable-closure ceilings),
  exposed `EXEC_TOP_CHUNK0_WGSL` and `EXEC_TOP_CHUNK0_COMPUTE_ENTRY`
  via `pub mod wgsl_pruner`, and pinned a naga-validation test for
  the concatenated module.
- **iter-6d-b** (332972022): wasm-bindgen-test
  `iter6d_a_exec_top_chunk0_compiles_on_chrome` that runs the module
  through Chrome Tint via the existing `sp7_probe` helper. Result:
  PASS, 62.17 s wall (vs ~3 s for a small kernel) -- the ~60 s
  delta is Tint compile time for the 1.08 MB module.
- **iter-6d-c** (9de42bdd1 + 3e154206e): probe-mode GPU dispatch in
  `WebGpuCircuitHal::generate_witness`. New struct fields hold an
  `Rc<WebGpuHal>` and a lazy `RefCell<Option<WebGpuKernel>>` for
  cache. Gated by a process-global `AtomicBool` flipped via
  `set_witgen_gpu_probe_enabled(true)` (re-exported from
  `risc0_circuit_rv32im::prove`). Default off -- enabling the flag
  triggers the Tint compile on the first segment.

## Why iter-6d-c is *not* a wall-time win as a witness-replacement

**SUPERSEDED 2026-05-15:** measurement found Tint compile is **2.65 s**,
not 60 s. See `2026-05-15-iter6d-cd-measured.md`. Original analysis
preserved below for historical context.

Combining the per-segment savings with the one-time compile cost:

- **Tint compile** (one-time, first segment): ~60 s
- **Per-segment GPU witgen** (after compile): unmeasured but
  bounded above by the rust_steps wall of 520 ms (the goal of GPU
  witgen is to be *faster* than that)
- **rust_steps savings if replaced**: 520 ms × 11 segments = 5.7 s
- **xgboost net** if iter-6d-c replaces rust_steps from the FIRST
  segment: 103 s - 5.7 s + 60 s = **157 s = +52% wall regression**

Even with full deployment the math doesn't work for a single
session. Two requirements must be met simultaneously for iter-6d-c
to be a positive lever:

1. **Compile cost amortized across many sessions.** Chrome's
   `createComputePipeline` *may* cache compiled pipelines by shader
   source SHA in its GPU process, but the cache is per-renderer
   process and dies with the tab. A CI workflow that starts a fresh
   Chrome per test never hits a warm cache. The browser-prove
   harness is exactly this pattern.
2. **Async pre-warm overlapping with guest execution.** If the
   Tint compile runs in parallel with the wasm-side guest execution
   (~100-200 ms for hello_world, ~1-2 s for xgboost), the compile
   wall is hidden. But the 60 s compile is 30-300× longer than the
   guest execution it could overlap with, so most compile time
   still blocks the first segment.

Without solving both: iter-6d-c is dead weight on benchmarks.

## What iter-6d-d would need

Three independent levers, in order of practicality:

**(d1)** Cache the compiled `GpuComputePipeline` JS object in a way
that survives across HAL/session restarts within a single page.
WebGpuHal would expose a `pre_warm_pipeline_async()` that the
prover construction calls; subsequent calls hit a JS-side
WeakMap-keyed cache. This makes iter-6d-c viable for long-running
*pages* (single-page apps proving repeatedly) but not CI.

**(d2)** Use `device.createComputePipelineAsync()` (a Promise) and
spawn_local it at HAL init via `wasm_bindgen_futures`. The Promise
resolves in the browser GPU process while the wasm-side guest
executes. Pre-conditions: add `wasm-bindgen-futures` to
risc0-circuit-rv32im (currently absent), add an async kernel
constructor on WebGpuHal, manage the
`RefCell<Option<JsFuture<WebGpuKernel>>>` lifecycle. The wall
overlap is bounded by the guest execution time (~1-2 s for
xgboost), well under the 60 s compile, so the *first segment* of a
session still eats most of the compile cost.

**(d3)** Shrink the WGSL module. The 1.08 MB exec_TopChunk0 sits
near the Chrome reachable-closure cliff (~0.4 MB). Splitting it
into 4 chunks of ~270 KB would let each compile in ~15 s, but the
sum of compile time would be unchanged (~60 s) -- unless the
chunks are *parallelized* through 4 concurrent
`createComputePipelineAsync` Promises, which the browser may or
may not pipeline through its single GPU process queue. This is the
only path to actually shrinking total compile wall on a fresh
session.

## What iter-6d-deeper would need

iter-6d-deeper targets `exec_TopAccumChunk0` (2.59 MB module / 1.7
MB reachable closure), which dominates the per-segment CPU cost
(2030 ms × 11 = 22 s out of 103 s wall). The closure is over both
the whole-module ceiling and the reachable-closure ceiling, so the
MuxChunk pass alone doesn't make it dispatchable.

The reason MuxChunk doesn't help here: TopAccum is *straight-line
validity-poly arithmetic* with the `is_valid` mux already inlined
into a single function. There's no wide switch to split.

iter-6d-deeper would need a NEW MLIR pass in zirgen that splits
straight-line arithmetic functions into multiple smaller fns -- e.g.
factor every K consecutive `arith.muladd` ops into a helper that
takes the running accumulator and the K operands. This is a
multi-week zirgen compiler change.

## What iter-6d-c IS good for

The probe infrastructure -- vendored WGSL + Tint-validated kernel +
gated dispatch path -- is reusable for:

- **Measurement.** A test that flips the flag and reports first-
  segment compile + per-segment dispatch wall (deferred this
  session; the harness panicked at `log_webgpu_diagnostics` because
  `gpu_dispatches=0` before the first prove segment; needs an
  alternate diagnostic-bypass path).
- **Future deployment.** Once one of d1/d2/d3 lands, flipping the
  flag enables GPU witgen with zero further code changes.
- **Other circuits.** The recursion + keccak HALs have the same
  rust_steps structure; the iter-6d-c probe pattern transfers
  directly. The recursion ceiling is 6 s wall savings; the keccak
  ceiling is gated on a working keccak_gen (the original SP6
  scope).

## Outcome

iter-6d-c is **landed as probe-mode infrastructure**, not deployed
as a witness replacement. The savings-vs-compile math doesn't work
on a fresh-session benchmark.

The biggest remaining tractable lever per the savings ceiling is
**iter-6d-deeper** (TopAccum split + GPU accumulate), which targets
22 s of wall but is blocked on a zirgen compiler change.

The user goal "as close to native CUDA as practically possible"
is closed at **18.0× CUDA** for xgboost. Practical floor on this
hardware/browser stack is 5-8× per the SP6c submission-bound
diagnosis; closing further requires (a) multi-device per-proof,
(b) better Chrome/Dawn WGSL→SPIR-V quality, or (c) moving witness
generation off the JS event loop entirely.
