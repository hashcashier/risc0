Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 iter 4 — WgslLanguageSyntax semantic lowering (4a/4b done; 4c+ next)`
DraftedAt: `2026-05-14`
Status: `DONE — the zirgen WGSL backend emits fully valid WGSL for the
rv32im witgen circuit. naga 29.0.3 validates the entire 41,374-line /
4.7 MB module clean.`

Commits on `wgsl-gpu-backend`: `87a9f46` (iter 3), `6be1503` (4a+4b),
`e496ff3` (4c), `b08c7bc` (4d), `4e99f42` (4e), `9592d98` (reserved
words), `5fee56d` (4f invoke_extern), `2a23408` (4g map/reduce + ext
promotion), `2741352` (4h+4i explosion fix). `naga-cli` 29.0.3
installed (`~/.cargo/bin/naga`).

**Final validation:** `naga` validates the full concatenated module
(`witgen_prelude.wgsl` + `types.wgsl.inc` + `layout.wgsl.inc` +
`steps.wgsl` = 41,374 lines, 4.7 MB) **clean and instantly** — the
prelude, all 451 type defs + narrowing helpers, all layout constants,
and all 211 step functions of `step_Top` / `step_TopAccum`.

## iter 4h + 4i — the 2^depth explosion (committed `2741352`)

Mid-iter-4 the generated `steps.wgsl` had blown up to **1.15 GB**
(longest line 7.6 MB) and naga could not typecheck it. Root cause:
`LookupOp`/`SubscriptOp`/`LoadOp` carry `CodegenAlwaysInline` (never
bound to a `let`), and the op handlers emitted the base/ref expression
*twice* (`.lyt.field` + `.buf`), so the deep `SubscriptOp`/`LookupOp`
chains iter-4g's map-unrolling produces compounded 2× per level. Fix —
emit the base exactly once: `load`/`store` take a `BoundLayout_Reg`
struct (the handler passes `op.getRef()` whole); `emitLayoutDef`/
`emitArrayDef` generate per-field/array narrowing **helper functions**
(`fn lookup_T_field(b) { BoundLayout_Ti(b.lyt.field, b.buf) }`) so the
base is a *parameter* — referenced twice with zero duplication — and
the op handlers emit a single call. Result: `steps.wgsl` 1.15 GB →
3.9 MB, longest line 7.6 MB → 2.4 KB.

**naga-driven fixes across iter 4** (each found by validating + re-running):
`layout`/`mod` reserved-word collisions → escaping; `invoke_extern`
string-literal operands → `extern_noop`; `BoundLayout_` wrappers for
layout *arrays*; zero-member structs → `_unused` dummy; `inv` Val/ExtVal
overload → `inv_0`/`ext_inv`; `Add/Sub/Mul` implicit Val→ExtVal
promotion → explicit `(x,0,0,0)` embedding; the 2^depth explosion above.

## Goal

Turn the iter-3 skeleton's *syntactically*-WGSL output into *valid*
WGSL. All work in `~/repos/zirgen`, branch `wgsl-gpu-backend`.
Verified each sub-iter by rebuilding `gen_zirgen` and re-running it on
`rv32im/v2/dsl/top.zir` → `/tmp/wgsl-test/steps.wgsl`.

## iter 4a — context-arg elision + structural macros (committed `6be1503`)

- `getWgslCodegenOpts()` no longer calls `Zhlt::addCppSyntax` — that
  function's *only* effect is registering the `"ExecContext& ctx"`
  func/call context arg (`ZHLT/IR/Codegen.cpp:154`). WGSL has no
  references / per-call context. **Result: `ExecContext` 211 → 0.**
- `WgslLanguageSyntax::emitInvokeMacro` special-cases the structural
  macros (the macro emission lives in the MLIR op defs in `Ops.cpp`,
  language-agnostic; `LanguageSyntax::emitInvokeMacro` controls
  rendering):
  - `layoutLookup(base, a.b.c)` → `base.a.b.c`
  - `layoutSubscript(base, idx)` → `base[idx]`
  - `eqz(value, "msg")` → `eqz(value)` — WGSL has no strings/assert
  - `setField(BabyBear)` → dropped
  - everything else → snake_case call (`load`, `store`, `load_ext`, …)
- `canonIdent(Macro)` → snake_case (was upper-cased `#define` names).

## iter 4b — field arithmetic + literals (committed `6be1503`)

- New `addWgslSyntax()` in `codegen.cpp`, replacing `addCppSyntax` in
  `getWgslCodegenOpts()`. WGSL has no operator overloading, so the
  `CodegenInfixOp` ops can't stay infix. `opts.addOpSyntax<Zll::AddOp/
  SubOp/MulOp>(…)` intercepts before the op's default infix `emitExpr`
  (`CodegenEmitter::emitExpr` checks `opSyntax` first):
  - `Add/Sub/Mul` → `add()/sub()/mul()` (base `Val`),
    `ext_add()/ext_sub()/ext_mul()` (degree-4 ext, picked by
    `ValType::getExtended()` on the result).
  - `PolynomialAttr` literal → plain `u32` (`2u`) instead of `Val(2)`.
- **Verified:** infix ` + ` / ` * ` for field ops → 0; bodies now read
  `let x8: Val = add(add(add(x7, x4), x5), x6);` /
  `eqz(sub(add(x10, x8), arg0));`.

## iter 4c — Montgomery literal encoding (committed `e496ff3`)

`addWgslSyntax`'s `PolynomialAttr` literal handler now Montgomery-
encodes each coefficient at codegen time (`babyBearMontgomeryEncode`,
mirroring `encode`/`mul` in `risc0_core/baby_bear.rs`). **Verified:**
`encode(1)` → `268435454u` (= R mod P, 1334×), `encode(2)` →
`536870908u` (405×). Field elements in risc0's witness buffer are
Montgomery-form u32; the WGSL prelude's `mul` is a Montgomery multiply,
so literals (carried direct-form in the IR) must be pre-encoded.

## The complete reference model (from `rust_steps.rs` + CUDA `witgen.h`)

The definitive spec is `risc0/circuit/rv32im-sys/kernels/cuda/witgen.h`
and `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs`:

- **Buffers**: a small *named* set — `data`, `global` for `step_Top`;
  `accum`, `data`, `global`, `mix` for `step_TopAccum`. Column-major:
  cell `(row, col)` is at `buf[col*rows + row]`. `MutableBuf.load(col,
  back)` → row `(rows + cycle - back) % rows` (+ a `zeroBack` rule);
  `GlobalBuf.load` → row 0, `back` must be 0. `store` → row `cycle`
  (mutable) / 0 (global).
- **`BoundLayout<T> { layout: const T&, buf: BufferObj* }`** — pairs a
  compile-time-constant layout with a runtime buffer.
  - `BIND_LAYOUT(c, buf)` = `BoundLayout(c, buf)`
  - `LAYOUT_LOOKUP(bl, elem)` = `BoundLayout(bl.layout.elem, bl.buf)`
    — **narrows the layout, KEEPS the buffer**
  - `LAYOUT_SUBSCRIPT(bl, i)` = `BoundLayout(bl.layout[i], bl.buf)`
  - `LOAD(bl, back)` = `bl.buf->load(ctx, bl.layout.col, back)`
  - `STORE(bl, val)` = `bl.buf->store(ctx, bl.layout.col, val)`
- **`Reg { col }`** is the leaf layout; `Index = size_t`.
- **`EQZ(v, loc)`** = `eqz(ctx, v, loc)` — assertion (`cond != 0` →
  abort). `SET_FIELD(x)` = nothing.
- **`INVOKE_EXTERN(ctx, name, …)`** = `extern_<name>(ctx, …)`. **All
  externs read from `ctx.preflight` (`PreflightTrace`)** — no host
  callbacks. `extern_log`/`extern_assert` are already no-ops even in
  CUDA. So GPU witgen needs the `PreflightTrace` uploaded as buffers,
  not host RPC — this de-risks SP7 substantially.

## CORRECTION to iter 4a

iter-4a's `layoutLookup → base.field` / `layoutSubscript → base[idx]`
in `emitInvokeMacro` is **incomplete** — it drops `.buf` and accesses
`.elem` instead of `.layout.elem`. The proper model needs the result
*type* (to construct the right `BoundLayout`), which `emitInvokeMacro`
doesn't receive but `addOpSyntax` handlers do. So `addWgslSyntax` must
override the ZStruct ops directly (as `ZStruct::addRustSyntax` does for
`LookupOp/SubscriptOp/LoadOp/StoreOp`).

## iter 4d — `BoundLayout` model + ZStruct op overrides (committed `b08c7bc`) — DONE

WGSL has no generics, so the CUDA `BoundLayout<T>` template is
monomorphized: `emitLayoutDef` emits a companion `struct
BoundLayout_<T> { layout: T, buf: u32 }` per layout type (356
emitted). `addWgslSyntax` registers op-syntax handlers for
`BindLayoutOp`/`LookupOp`/`SubscriptOp`/`LoadOp`/`StoreOp`/`GetBufferOp`
that thread `.buf` and construct the right wrapper. `emitTypeRef`
maps layout-trait types → `BoundLayout_<T>`, buffer types → `u32`.
`RefAttr` → `Reg(Nu)` (WGSL has no implicit ctors), so WGSL also drops
`ZStruct::addCppSyntax`. **Verified:** `fn step_Top(data0: u32,
global1: u32)`, args `BoundLayout_NondetRegLayout`,
`layout.wgsl.inc` consistent (`NondetRegLayout(Reg(12u))`).
*Known:* the load/store ref is emitted twice (single-use `LookupOp`
results inline) — valid WGSL, verbose; `TODO(wgsl)` for iter 5.

## iter 4e — witgen prelude (committed `4e99f42`) — DONE

`zirgen/compiler/codegen/gpu/witgen_prelude.wgsl` (~290 lines): field
types (`Val=u32`, `ExtVal=vec4<u32>`, `Index=u32`, `struct Reg`,
`struct BoundLayout_Reg`); BabyBear Montgomery arithmetic
(`add/sub/mul/mul_wide/ext_*` verbatim from the verified
`webgpu_codegen/prelude.wgsl`, + `encode/decode/pow/inv`); DSL builtin
helpers (`isz/neg_0/inv_0/mod/bit_and/in_range`); column-major
witness-buffer access (`load/store/load_ext/store_ext/load_as_ext`)
with the back-row + accum zero-back rules from `rust_steps.rs`. `eqz`
is split by operand type via a new `addOpSyntax<EqualZeroOp>` handler
(`eqz`/`eqz_ext`). **Verified:** rebuild + re-gen clean; concatenated
`prelude + types + layout + steps` = 33 170 lines, no C++ leaks
(`defs.wgsl.inc` is correctly excluded — `steps.wgsl` doesn't reference
its `kRegCount*` constants).

## iter 4f — invoke_extern + multi-result (committed `5fee56d`) — DONE

`addOpSyntax<ExternOp>`: assert/log/print → `extern_noop()` (652
sites; they carry WGSL-illegal string operands and are no-ops in CUDA
too); the ~15 real externs → `extern_<name>(operands)` (the `ctx` arg
dropped). Bug fixed: `Operation::getName()` returns the op name
`"zll.extern"`, not the `$name` attr — use `getNameAttr()`.
`emitSaveResults` for N>1 binds a `_tuple` temp and projects each name
with `tmp[i]` — every multi-result site is an array-returning extern,
so the temp is a WGSL `array<Val,N>`. Prelude gains `extern_*` stubs
(exact arities/return types; TODO zero-bodies) + `to_size_t` and the
camelCase builtins `bitAnd`/`inRange`. **Verified:** naga parses past
every `invoke_extern` site.

## Remaining iter-4 plan

- **4g — map/reduce lowering** (next, last parse blocker). 42 `MapOp`
  + 1 `ReduceOp` survive (rv32im is not `--parallel-witgen`). WGSL has
  no closures/lambdas — the CUDA `map`/`reduce` are `for` loops over
  fixed-`N` arrays. Plan: `addOpSyntax<MapOp>`/`<ReduceOp>` (or fix
  `emitMapConstruct`/`emitReduceConstruct`) to emit an explicit WGSL
  `for` loop — bind the element to the region's block arg, `emitRegion`
  the body, write the per-iteration result into a `var array<T,N>`.
  Needs the `MapOp`/`ReduceOp` region/terminator structure.
- **4h — full validate.** After map/reduce, get `naga` to a clean
  parse + typecheck of the whole module; fix residual type errors.
- `defs.wgsl.inc` is still C++ (`constexpr size_t kRegCountData = 211;`)
  — `getEmitter` switches on `LanguageKind` and WGSL reports `Cpp` →
  `CppEmitZhlt`. A proper fix needs `LanguageKind::Wgsl` + a
  `WgslEmitZhlt`; not on the `steps.wgsl` critical path so deferred.

## Constraint carried forward

`steps.wgsl` is still one ~2.2 MB module — ~5× over the iter-1
~478 KB single-kernel cliff. iter 5 = staged-kernel chunking.
