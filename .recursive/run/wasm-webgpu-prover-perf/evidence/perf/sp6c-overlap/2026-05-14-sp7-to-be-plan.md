Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 — GPU-resident witness + accumulate (TO-BE plan)`
DraftedAt: `2026-05-14`
Status: `TO-BE`
Approach: `codegen WGSL` (user-directed 2026-05-14; betting SP3's eval_check
codegen ceiling does not generalize to witgen)

## Goal

Move `rv32im_witgen` (`step_Top`) and `rv32im_accumulate` (`step_TopAccum`
+ prefix sums) from CPU `rust_steps` to GPU WGSL kernels, eliminating
~4–5 s of CPU per segment and shrinking the per-segment CPU buffer peak.

## The model: mirror the CUDA backend

zirgen already emits a CUDA backend the WebGPU path can mirror:
- `risc0/circuit/rv32im-sys/kernels/cuda/steps.cu` — 62 013 lines of
  `__device__` `exec_*` / `back_*` functions; `step_Top` /
  `step_TopAccum` are the entry points.
- `kernels/cuda/ffi.cu` — `__global__` kernels `par_stepExec` /
  `fwd_stepExec` / `rev_stepExec` (one thread per cycle → `step_Top`)
  and `stepAccum` / `finalizeAccum`.
- `kernels/cuda/steps.cuh` + `witgen.h` — the type / macro substrate
  (`ExecContext`, `BoundLayout<T>`, `LOAD`/`STORE`/`LAYOUT_LOOKUP`,
  `Val`/`ExtVal`, the `*Struct` types).

The CPU path uses the equivalent generated **Rust** in
`risc0/circuit/rv32im/src/zirgen/steps.rs.inc` (30 468 lines).

WGSL codegen = a transpiler that emits WGSL `fn`s equivalent to the
CUDA `__device__` functions, plus a WGSL prelude for the substrate,
plus `@compute` entry kernels mirroring `par_stepExec` / `stepAccum`.

## Hard problems (must be solved by the transpiler design)

1. **No generics in WGSL.** CUDA uses `BoundLayout<NondetRegLayout>`
   etc. — templated layout bindings. WGSL has no templates. Resolution:
   flatten layouts to `u32` base-offset values; `bind_layout!` /
   `.map(|c| c.field)` become compile-time offset arithmetic the
   transpiler folds into constants. The 552 KB `layout.rs.inc` is the
   offset source of truth.
2. **Struct ABI.** `*Struct` return types (`NondetRegStruct`,
   `ValU32Struct`, …) — WGSL supports plain structs; the transpiler
   emits them. Nested struct returns are fine in WGSL.
3. **Recursion / call graph.** WGSL forbids recursion. The generated
   step call graph must be acyclic (it is — it is a DAG of circuit
   components); verify and emit in topological order.
4. **`Result` / `bail!`.** CUDA uses no error propagation in the hot
   path (mux `bail!("unreachable")` becomes a trap). WGSL: emit an
   error flag in a buffer the host checks post-dispatch.
5. **The Chrome compile ceiling (the central risk).** SP3 measured
   codegen'd eval_check WGSL ~30× slower than the interpreter — Chrome
   chokes on huge generated WGSL. `step_Top`'s call graph is large; a
   single monolithic WGSL module may not compile or may run slow.

## Iteration sequence

### Iter 1 — synthetic scale test + kill-criterion (THIS is the bet test)

**Sharpened by recon:** SP3's ceiling is an *execution-model* ceiling,
not just a compile-time one. The SP3 staged eval_check kernel was a
~1.6 MB generated WGSL shader (`webgpu.rs:5279`) that ran ~30× slower
than the interpreter *even with the compile cached* (`:5687`). So a
small correct hand-translated spike would NOT test the bet — it would
compile and run fine because it is small. The kill-criterion only
triggers at scale.

Iter 1 is therefore a **synthetic scale test**: programmatically emit
a WGSL `@compute` kernel that is *structurally faithful* to codegen'd
witgen (many small `fn`s in a call DAG, column-major buffer
loads/stores, BabyBear field arithmetic, if/else mux branches) and
*scaled* to a plausible full witgen size (target the line/byte scale
of the 30 K-line `steps.rs.inc` → est. multi-MB WGSL). Dispatch it on
a `WebGpuHal`, one invocation per "cycle", and measure:
- Chrome compile time of the scaled kernel.
- **Per-invocation execution throughput** vs an equivalent amount of
  CPU `rust_steps` work.

**Kill-criterion:** if the scaled witgen-shaped WGSL kernel executes
> ~5× slower per cycle than CPU `rust_steps` (compile cost amortized),
then SP3's *execution* ceiling has generalized to witgen — stop,
report, fall back to the interpreter (AS-IS option 1) or partial
port. The synthetic kernel decouples "does Chrome handle scale"
(testable cheaply, now) from "is the transpiler correct" (iters 2+,
only worth building if iter 1 clears the bar).

### Iter 2 — the transpiler skeleton
If iter 1 clears the kill-criterion: build the
`rust_steps`/`steps.cu` → WGSL transpiler. Source choice: the CUDA
`steps.cu` (flatter C, closer to WGSL than Rust). Start with the
substrate prelude (Val/ExtVal arithmetic, layout offset helpers,
buffer load/store) and the ~20 leaf `exec_*`/`back_*` builtins.

### Iter 3 — full `step_Top` codegen + witgen wiring
Transpile the full `exec_Top` call graph, emit the `@compute`
witgen kernel (one invocation per cycle), wire into `generate_witness`
behind the flag. Verify: R1 smoke + xgboost receipts still verify.

### Iter 4 — `step_TopAccum` + prefix sums
Transpile the accum call graph; the two tail prefix-sum passes are
sequential — emit them as separate small kernels or a serial pass.

### Iter 5 — measure + integrate
Full A/B: CPU-witgen vs GPU-witgen on R1 + xgboost + KeccakUnion(3).
If positive, make GPU witgen the default; feed the shrunk per-segment
footprint back to the iter-9 scheduler (segment-phase overlap may now
be reachable).

## Correctness discipline

Every iter: the SP-CR gate (Addendum 01) applies. GPU witgen must
produce byte-identical `data`/`accum` buffers to the CPU `rust_steps`
path (compare via a debug equality check on a smoke fixture before
trusting receipts). Any divergence halts SP7 and invokes SP-CR.

## Why iter 1 is structured as a spike

The user directed "codegen anyway" — explicitly betting against SP3's
prior. The honest way to honor that bet is to test it cheaply and
first: a representative hand-translated kernel answers "does Chrome
handle codegen'd witgen WGSL" for the cost of a spike. If the bet
pays, the full transpiler (iters 2–5) is justified. If it does not,
we have a real measurement, not weeks sunk — and the AS-IS already
documents the interpreter fallback.
