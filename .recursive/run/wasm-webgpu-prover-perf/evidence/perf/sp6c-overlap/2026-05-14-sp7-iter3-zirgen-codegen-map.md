Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 iter 3 — zirgen codegen architecture map + WGSL-backend change-set`
DraftedAt: `2026-05-14`
Status: `IN-PROGRESS (zirgen build running)`

## Setup done

- zirgen fork already cloned at `~/repos/zirgen` (the user's
  `hashcashier/zirgen`, a fork of `risc0/zirgen`). Created feature
  branch **`wgsl-gpu-backend`** (worktree discipline — not working on
  `main`).
- bazel: not on PATH, but `bazelisk` had cached **bazel 6.0.0** at
  `~/.cache/bazelisk/downloads/sha256/<hash>/bin/bazel` (matches
  `.bazelversion`). Symlinked to `~/.local/bin/bazel`. A 5.9 GB warm
  `~/.cache/bazel/_bazel_rami/` cache exists (built here before).
- Build kicked off: `bazel build //zirgen/dsl:zirgen` — at
  `[1,129/1,318]` actions when this was written (compiling MLIR
  dialects; warm cache means it is not rebuilding all of LLVM).

## How risc0's generated circuit code is produced — RESOLVED (task 65)

`gen_zirgen` (`zirgen/Main/gen_zirgen.cpp`) `main()` emits **three
targets** via one shared `emitTarget(...)` flow:

```cpp
emitTarget(RustCodegenTarget(...), module, stepFuncs, getRustCodegenOpts(), splitCount);
emitTarget(CppCodegenTarget(...),  module, stepFuncs, getCppCodegenOpts(),  splitCount);
emitTarget(CudaCodegenTarget(...), module, stepFuncs, getCudaCodegenOpts(), splitCount);
```

Each target is:
- a **`CodegenTarget`** subclass (`zirgen/Main/Target.{h,cpp}`) —
  tiny: file extensions + mustache `Template`s (header/footer with
  license, `#include`s, namespace). `CudaCodegenTarget`:
  `getImplExtension()="cu"`, `getStepTemplate()` = `#include
  "steps.cuh"` + `namespace …::cuda { … }`.
- a **`CodegenOptions`** from `get<Lang>CodegenOpts()`
  (`zirgen/compiler/codegen/codegen.cpp`) which wires a
  **`LanguageSyntax`**:
  ```cpp
  CodegenOptions getCudaCodegenOpts() {
    static codegen::CudaLanguageSyntax kCuda;   // : public CppLanguageSyntax
    codegen::CodegenOptions opts(&kCuda);
    addCommonSyntax(opts); addCppSyntax(opts);
    ZStruct::addCppSyntax(opts); Zhlt::addCppSyntax(opts);
    return opts;
  }
  ```

The checked-in `risc0/circuit/rv32im/src/zirgen/steps.rs.inc` and
`risc0/circuit/rv32im-sys/kernels/cuda/steps.cu` are these
`LanguageSyntax`-framework outputs. **`gen_gpu.cpp`'s
`GpuStreamEmitterImpl` is a SEPARATE/legacy path — NOT what
`gen_zirgen` uses for the step functions.** So the WGSL backend is a
`LanguageSyntax`, not a `gen_gpu.cpp` branch.

The rv32im codegen Bazel target is
`//zirgen/circuit/rv32im/v2/dsl:codegen` (`build_circuit` rule,
`bin = //zirgen/Main:gen_zirgen`), `OUTS` includes `steps.rs.inc`,
`steps.cu`, `steps.cpp`, layouts, etc.

## The WGSL-backend change-set (bounded, well-patterned)

Mirror `Cuda` everywhere `Cuda` appears in the codegen path:

1. **`WgslLanguageSyntax : public LanguageSyntax`** — new
   `zirgen/compiler/codegen/WgslLanguageSyntax.cpp` + decl in
   `codegen.h`. The ~18 virtual methods (`emitFuncDefinition`,
   `emitConditional`, `emitSwitchStatement`, `emitReturn`,
   `emitCall`, `emitStructDef`, `emitArrayDef`, `emitLayoutDef`,
   `emitSaveResults`, `emitConstDecl`, `canonIdent`, …). WGSL is
   Rust-ish syntax (`fn f(x: u32) -> u32`, `let`/`var`) but not
   C-family — so standalone, not `: public CppLanguageSyntax`. **This
   is the real implementation work.**
2. **`getWgslCodegenOpts()`** in `codegen.cpp` — mirrors
   `getCudaCodegenOpts()`; static `WgslLanguageSyntax` + syntax
   bundles (`addCommonSyntax` + WGSL variants of `addCppSyntax` /
   `ZStruct::add*` / `Zhlt::add*`, or reuse where compatible).
3. **`WgslCodegenTarget : public CodegenTarget`** in `Target.{h,cpp}`
   — `getImplExtension()="wgsl"`; `getStepTemplate()` header = the
   WGSL field prelude (`add`/`sub`/`mul` — reuse risc0's
   `webgpu_codegen/prelude.wgsl` Montgomery `mul`) + storage/uniform
   bindings.
4. **`emitTarget(WgslCodegenTarget(...), …)`** in `gen_zirgen.cpp`
   `main()` (and a WGSL line in `emitAllLayouts`).
5. **`steps.wgsl`** added to `OUTS` in
   `zirgen/circuit/rv32im/v2/dsl/BUILD.bazel`.
6. **BUILD.bazel** wiring in `zirgen/compiler/codegen/BUILD.bazel`
   for the new `.cpp`.

## The hard problems — located, and one is already solved

- **No generics (WGSL).** Handled in `WgslLanguageSyntax::emitLayoutDef`
  / `emitStructDef` — flatten ZStruct layouts to `u32` offset
  constants. The IR models layouts structurally; the `emit*` methods
  control rendering, so the C++-template `BoundLayout<T>` is a
  *CudaLanguageSyntax rendering choice*, not an IR fact.
- **No recursion (WGSL).** `CodegenEmitter` emits functions; the
  circuit call graph is a DAG. Verify `CodegenEmitter` emits in
  dependency order (or add a topological pass for WGSL).
- **No `Result` (WGSL).** `emitReturn` / the `eqz`-style checks —
  WGSL impl drops the `assert`s or routes them to a debug error-flag
  buffer.
- **Chunking — ALREADY A ZIRGEN FEATURE.** The rv32im BUILD has a
  commented-out `--step-split-count` arg and `steps_<i>.cu` outputs.
  zirgen can already split the step function into N files. iter-1/2
  said the WGSL backend MUST chunk (capacity cliff); `--step-split-
  count` is the lever — emit each split as a staged `@compute` kernel
  with the iter-2 scratch handoff. The partitioning is zirgen's job;
  the WGSL backend just emits each split.

## Revised iteration plan (refined)

- **iter 3 (finishing):** build completes → confirm `gen_zirgen`
  builds + runs; confirm the rv32im `:codegen` target builds; read
  `CppLanguageSyntax.cpp` (the `LanguageSyntax` impl to mirror) and
  `CodegenEmitter.cpp` (the emit driver).
- **iter 4:** implement `WgslLanguageSyntax` + `WgslCodegenTarget` +
  `getWgslCodegenOpts()` + `emitTarget` call + BUILD wiring. Get
  `steps.wgsl` emitting for rv32im (start un-split, accept it
  exceeds the cliff — correctness first).
- **iter 5:** enable `--step-split-count` for the WGSL target; emit
  staged `@compute` kernels < ~250 KB with scratch handoff.
- **iter 6:** wire `steps.wgsl` into risc0 `WebGpuCircuitHal`;
  SP-CR byte-identical check vs CPU `rust_steps`; A/B measure.

## Next reads (while/after build)

- `zirgen/compiler/codegen/CppLanguageSyntax.cpp` — the concrete
  `LanguageSyntax` impl to mirror for WGSL.
- `zirgen/Dialect/Zll/IR/CodegenEmitter.cpp` — the emit driver:
  function ordering, how `emit*` hooks are called.
- `zirgen/compiler/codegen/RustLanguageSyntax.cpp` — WGSL syntax is
  closer to Rust (`fn`, `let`, `->`); useful reference for the
  function/return/let rendering.
