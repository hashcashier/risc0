Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 iter 3 — WGSL codegen backend skeleton; toolchain proven end-to-end`
DraftedAt: `2026-05-14`
Status: `DONE — iter 3 deliverable met; iter-4 gap map below`

## iter 3 deliverable: the codegen toolchain is proven end-to-end

The corrected SP7 plan's iter-3 goal was: "clone + build the zirgen
fork; resolve the path question; stand up a minimal WGSL emission to
prove the toolchain end-to-end." All three are done.

- **Build:** `bazel build //zirgen/Main:gen_zirgen` succeeds with the
  WGSL backend added (warm cache: ~5 s incremental).
- **Path question (RESOLVED, see iter-3 codegen-map doc):** risc0
  consumes the `LanguageSyntax`-framework outputs via
  `emitTarget(<Target>, …, get<Lang>CodegenOpts(), stepSplitCount)` in
  `gen_zirgen.cpp` `main()`. `gen_gpu.cpp` is dead for this path.
- **Toolchain proof:** `gen_zirgen` on `rv32im/v2/dsl/top.zir` (exit 0)
  now emits a **4th target** alongside Rust/C++/CUDA:
  - `steps.wgsl` — **2.27 MB, 26 805 lines, 211 `fn` defs**
  - `types.wgsl.inc` — 451 `struct` defs
  - `layout.wgsl.inc` — 275 KB of layout constants
  - `defs.wgsl.inc` — 7 lines

## Change-set — committed `87a9f46` on branch `wgsl-gpu-backend`

7 files, +456 lines, mirroring the `Cuda` target everywhere `Cuda`
appears in the `emitTarget` path:

1. `zirgen/Main/Target.{h,cpp}` — `WgslCodegenTarget` (ext `wgsl`,
   declExt == implExt so `emitTarget` skips decl-emission; minimal
   step template — license banner only, no prelude/bindings yet).
2. `zirgen/compiler/codegen/codegen.h` — `WgslLanguageSyntax :
   public LanguageSyntax` (NOT `: public CppLanguageSyntax` — WGSL
   diverges from C++ on generics/refs/recursion/Result/closures/
   tuples) + `getWgslCodegenOpts()` decl.
3. `zirgen/compiler/codegen/codegen.cpp` — `getWgslCodegenOpts()`,
   a copy of `getCudaCodegenOpts()`.
4. `zirgen/compiler/codegen/WgslLanguageSyntax.cpp` — NEW, ~330 lines.
   All 12 pure-virtual + 7 abort-default `LanguageSyntax` methods,
   emitting WGSL surface syntax; semantic gaps marked `TODO(wgsl)`.
5. `zirgen/compiler/codegen/BUILD.bazel` — `WgslLanguageSyntax.cpp`
   into the `codegen` `cc_library` srcs.
6. `zirgen/Main/gen_zirgen.cpp` — the 4th `emitTarget(WgslCodegenTarget
   …, getWgslCodegenOpts(), stepSplitCount)` call.

## What the skeleton emits CORRECTLY (valid WGSL surface syntax)

- `fn name(args) -> RetType { … }` function definitions (211 of them).
- `let x: T = expr;` saved results; `return x;`.
- Call syntax: `back_NondetReg(ctx, distance0, layout1)`.
- Positional struct construct: `NondetRegStruct(LOAD(…))`.
- `struct Name { field: T, }` (in `types.wgsl.inc`).
- `alias Name = array<T, N>;` — valid WGSL type aliases.
- Nested layout constants: `NondetRegLayout(/*offset=*/12)` — the
  per-register offsets are already structural in the IR.
- Source-location line comments preserved.

## iter-4 gap map — every semantic lowering, located and counted

| Gap | Where / count | iter-4 fix |
|---|---|---|
| `ExecContext& ctx` in every signature | 211 sigs — C++ ref syntax, invalid WGSL | Elide context args (don't pull C++ context-arg machinery into `getWgslCodegenOpts`, or skip them in `emitFuncDefinition`); buffers → module-scope `@group/@binding`. |
| `LOAD`/`STORE`/`LOAD_EXT`/`STORE_EXT`/`LAYOUT_LOOKUP`/`EQZ` as bare calls | pervasive — emitted as call syntax; CUDA `#define`s them, WGSL has no preprocessor | Real WGSL helper `fn`s in the step template prelude, or inline. `LAYOUT_LOOKUP(layout, _super)` → `.field` access (offsets are structural — see below). `EQZ(v, "msg")` → drop or error-flag buffer write (the string arg must go — WGSL has no strings). |
| `Val` / `ExtVal` / `Index` / `Reg` types | type names in sigs + `types.wgsl.inc` | `Val`→`u32`, `ExtVal`→`array<u32,4>` (or struct), `Index`→`u32`, `Reg`→`u32`. |
| Layout types as params (`layout1: NondetRegLayout`) | 0 `BoundLayout<>` refs (good — skeleton emits raw layout type names) | **Smaller than planned:** keep layouts as nested structs-of-`u32` (WGSL handles those); just make leaf `Reg`→`u32`. No flat-offset-constant rewrite needed. |
| Multi-result functions | 5 sites: 2×2-tuple, 2×16-tuple, 1×5-tuple, 1×4-tuple (`TODO(wgsl)` tagged) | WGSL has no tuples — wrap results in a struct or use out-params. Bounded. |
| `defs.wgsl.inc` is C++ (`constexpr size_t`, `SET_FIELD(BabyBear)`) | 7 lines | `const kRegCountX: u32 = N u;`; drop `SET_FIELD`. Trivial. |
| `_super` field name | leading underscore, WGSL-reserved-word risk (`super`) | `canonIdent` reserved-word + leading-`_` escaping. |
| 136× `TODO(wgsl): unreachable mux arm` | switch arms — already structurally correct, just a comment | Low priority; WGSL has no `assert`. |

## Constraints carried forward (iters 1–2, unchanged)

- **Capacity cliff:** `steps.wgsl` is one 2.27 MB module — ~5× over
  the ~478 KB single-kernel device-death cliff (iter 1). A single
  `@compute` module of this cannot run.
- **Chunking has no catastrophe** (iter 2, ~0.10 ns/cycle per stage
  boundary) — so the iter-5 fix is staged `@compute` kernels < ~250 KB
  with a scratch buffer for live state. `--step-split-count` does NOT
  do this (it splits whole `StepFuncOp`s across files; WGSL has no
  linker, so cross-file calls break) — iter 5 needs a real partition.

## Refined iteration plan

- **iter 4** — `WgslLanguageSyntax` semantic lowering: the gap-map
  table above. Target: `steps.wgsl` is *valid WGSL* (validates under
  `naga`/`tint` for the small leaf functions), accepting the whole
  module still exceeds the cliff.
- **iter 5** — chunking: partition `step_Top` into staged `@compute`
  kernels < ~250 KB + scratch handoff (iter-2 model).
- **iter 6** — wire `steps.wgsl` into risc0 `WebGpuCircuitHal`;
  SP-CR byte-identical check vs CPU `rust_steps`; A/B on R1 +
  xgboost + KeccakUnion(3).
