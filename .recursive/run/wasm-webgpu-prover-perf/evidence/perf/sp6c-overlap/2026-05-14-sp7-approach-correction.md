Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 — approach correction: zirgen WGSL backend, NOT a transpiler`
DraftedAt: `2026-05-14`
Status: `SUPERSEDES the transpiler framing in 2026-05-14-sp7-{as-is,to-be-plan}.md`

## The correction

The SP7 AS-IS and TO-BE planned a **transpiler** for the generated
`steps.cu` / `steps.rs.inc`, on the stated assumption that "zirgen —
the circuit compiler — isn't available, so I can't add a backend."

**That assumption was wrong.** zirgen is available — the user's fork
`github.com/hashcashier/zirgen` (a fork of `risc0/zirgen`). The risc0
*repo* only checks in zirgen's *generated outputs*; the compiler
itself is a separate repo. I never verified the assumption — the user
caught it with "why a transpiler".

Transpiling 62 K lines of generated CUDA would have been parsing a
compiler's *output* — fragile, and re-broken by every circuit change.
The correct approach is a **backend in the compiler**, exactly how
zirgen's CUDA and Metal GPU backends exist.

## zirgen's GPU codegen architecture (from the fork's `main`)

`zirgen/compiler/codegen/`:
- **`gen_gpu.cpp` → `GpuStreamEmitterImpl`** — the GPU step-function
  emitter. `emitStepFunc(name, func)` picks a mustache template by
  suffix: `.cu` → `cu_step_tmpl`, `.metal` → `metal_step_tmpl`. It
  walks the lowered MLIR function body (`emitStepBlock` →
  `emitOperation`) emitting per-op code lines: `ConstOp`, `GetOp`
  (buffer load `buf[off*steps + ((cycle-back)&mask)]`), `SetOp`
  (store), `Get/SetGlobalOp`, `Add/Sub/Mul/Neg/Inv/IsZero/BitAnd/Mod
  Op`, plus `IfOp` / `NondetOp` / `ExternOp` blocks.
- **`LanguageSyntax` framework** (`Dialect/Zll/IR/Codegen.h`,
  `CppLanguageSyntax.cpp`, `RustLanguageSyntax.cpp`) — the structured
  path with a ~18-method virtual interface (`emitFuncDefinition`,
  `emitConditional`, `emitSwitchStatement`, `emitStructDef`,
  `emitLayoutDef`, `emitCall`, ...). `CudaLanguageSyntax : public
  CppLanguageSyntax`. This path produces the structured `steps.cu`
  with `__device__` sub-functions + `BoundLayout<T>`.
- GPU templates: `gpu/step.tmpl.cu.h`, `gpu/step.tmpl.metal.h`,
  `gpu/eval_check.tmpl.{cu,metal}.h`.

## Why the hard problems vanish

The "no generics / no recursion / no `Result`" problems I flagged for
transpiling generated CUDA **do not exist at the codegen layer**,
because the GPU emitter works from **post-lowering MLIR IR**:
- No generics — buffers are already flat `Fp*` + integer offsets in
  the IR; `BoundLayout<T>` is a *C++-output* artifact, not an IR one.
- No recursion — `emitStepFunc` emits one function body; the step
  block is a flat op sequence with nested `IfOp`/`NondetOp` regions.
- No `Result` — the emitter uses `assert(...)` for the `eqz` /
  unchecked-read checks; WGSL has no `assert`, so the WGSL emitter
  drops them (or routes them to a debug error-flag buffer).

CUDA and Metal are the existence proof: two working GPU targets from
this same IR. WGSL is a third.

## The corrected SP7 approach

Add a **WGSL GPU backend to the zirgen fork**, mirroring CUDA/Metal:

1. **Clone + build the zirgen fork.** Bazel + LLVM/MLIR project —
   the build is non-trivial but documented (`bazel build`). This is
   iter 3's first task and gates everything.
2. **`gpu/step.tmpl.wgsl.h`** — the WGSL kernel scaffold: `@compute`
   entry, storage/uniform bindings, and the BabyBear field prelude
   (`add`/`sub`/`mul` — the Montgomery `mul` already exists in
   risc0's `webgpu_codegen/prelude.wgsl`, reuse it).
3. **WGSL emission in `GpuStreamEmitterImpl`** — a `.wgsl` branch in
   `emitStepFunc`, and WGSL variants of the `emitOperation` cases.
   WGSL syntax deltas vs CUDA: `let`/`var` not `auto`; field type is
   `u32` with `add`/`sub`/`mul` helper fns (no operator overloading);
   `IfOp` → `if (cond != 0u) { }`; drop `assert`. ~15 op cases,
   mechanical.
4. **Chunking** — iter 1 found a single WGSL kernel dies at ~478 KB
   and full witgen is multi-MB. iter 2 found chunking has no
   catastrophe (~0.1 ns/cycle per stage boundary). So the WGSL
   backend must emit the step function **split across staged
   `@compute` kernels**, each < ~250 KB, with a `scratch` buffer for
   live state across the split. Decide the split granularity from
   the IR (e.g. at `IfOp`/region boundaries or by emitted-line count).
5. **Regenerate** rv32im's witgen as `steps.wgsl` from the modified
   zirgen.
6. **Wire into risc0** — `WebGpuCircuitHal::generate_witness` /
   `step_accum` dispatch the WGSL step kernels instead of calling
   `rust_steps`.
7. **Measure** — chunked GPU witgen vs CPU `rust_steps` on R1 +
   xgboost + KeccakUnion(3). SP-CR gate: GPU witgen must produce
   byte-identical `data`/`accum` buffers.

## Open question for iter 3

Which path does risc0's rv32im *actually* consume — the
`gen_gpu.cpp` flat-step output, or the `CudaLanguageSyntax`
structured `steps.cu`? The checked-in
`risc0/circuit/rv32im-sys/kernels/cuda/steps.cu` is the *structured*
(`__device__` + `BoundLayout<T>`) form → that is the
`LanguageSyntax`/`gen_cpp` path, not `gen_gpu.cpp`. But risc0's
WebGPU `rust_steps` path uses the **Rust** generated `steps.rs.inc`
(also a `LanguageSyntax` output). `gen_gpu.cpp`'s flat emitter may be
an older/alternate path. **Resolve this when zirgen is cloned and
built** by tracing how `risc0/circuit/rv32im` invokes zirgen codegen
— it determines whether the WGSL backend is a `gen_gpu.cpp` branch
(simpler, flat) or a `WgslLanguageSyntax` (structured, ~18 methods).
Both are far smaller and less fragile than transpiling 62 K lines of
generated output.

## What carries over from iters 1-2

The spike findings are about the *execution model* and still hold
regardless of how the WGSL is produced:
- iter 1: single WGSL kernel dies ~478 KB (capacity cliff) → the
  backend MUST chunk.
- iter 2: chunking has no SP3-class catastrophe (~0.1 ns/cycle per
  boundary) → chunking is a viable emission strategy.

## Revised iteration plan

- **iter 3** — clone + build the zirgen fork; resolve the
  path question; stand up a minimal WGSL emission (one trivial step
  op → valid WGSL) to prove the toolchain end-to-end.
- **iter 4** — full WGSL `emitStepFunc` for rv32im witgen (`step_Top`),
  with chunking; regenerate `steps.wgsl`.
- **iter 5** — `step_TopAccum` + the accum prefix-sum passes.
- **iter 6** — wire into `WebGpuCircuitHal`, SP-CR byte-identical
  check, A/B vs CPU `rust_steps` on R1 + xgboost + KeccakUnion(3).
