Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 iter 6a (zirgen MuxChunk pass) + 6b (per-leaf module probe)`
Date: 2026-05-15
Status: `iter 6a LANDED on hashcashier/zirgen wgsl-gpu-backend (b956e80);
         iter 6b reveals exec_Top viable, exec_TopAccum/Extract need more
         work before risc0-side wiring.`

## iter 6a — MuxChunk pass + canonicalize/CSE/SymbolDCE

`zirgen/Dialect/ZStruct/Transforms/MuxChunk.cpp` (b956e80). Splits every
wide `zstruct.switch` into per-arm-group chunk `zhlt.step_func`s:
- prologue cloned per chunk (idempotent witgen stores)
- restricted switch carrying only that chunk's selectors+arms
- epilogue inside a single-arm guard switch keyed on the sum of those
  selectors (only the firing chunk runs the epilogue → no scratch buffer)

Critical design lesson recorded in memory:
[[project_sp7_codegen_capacity_cliff]]: bottom-up `expandToLeaves`
combinatorially explodes the entry's variant count (exec_Top × N1 × N2
× ... = millions of variants, 71 GiB RSS in <2 min). Cross-product
expansion belongs at risc0 emit time, NOT inside MLIR.

`gen_zirgen.cpp` adds CSE+canonicalize+SymbolDCE after MuxChunk —
load-bearing because each chunk clones the entire prologue and any op
only consumed by an arm not in this chunk becomes dead. Notably
`zll.variadic_pack` (otherwise the WGSL emitter outputs the C++ name
`std::initializer_list<Val>`, which naga rejects).

For rv32im_v2: 211 originals + 174 chunk fns = 385 fns / 9.16 MB
`steps.wgsl`. The 9.95 MB combined module
(`witgen_prelude + types + layout + steps`) **naga-validates clean**.

## iter 6b — per-leaf module probe with chunk0-everywhere rewriting

Question: can we shrink the per-@compute-entry closure under both
device cliffs (whole < 2 MB; reachable closure < 400 KB per iter-5b/d)
by linking exactly one chunk per chunked worker?

`prune_chunked.py` walks each chunk's transitive closure as-is
(originals still called) — confirms the iter-6a output naga-validates
but the top-level chunks (`exec_TopChunk`, `exec_TopAccumChunk`,
`exec_TopExtractChunk`) still reach 1.99–2.51 MB modules (over both
cliffs).

`perleaf_module.py` adds the call-rewrite step: for every callsite to
a chunked worker `exec_W`, replace with `exec_WChunk0`. Recompute
closure. Result for the 17 top-level chunks:

| entry                  | closure | reach KB | mod MB |  verdict |
|---|---:|---:|---:|---|
| **exec_TopChunk0**         | 599 | **281.5** | **1.03** | **OK** |
| **exec_TopChunk1**         | 600 | **281.4** | **1.03** | **OK** |
| exec_TopAccumChunk0..12 | ~640 | 1700–1820 | 2.43–2.53 | FAIL (mod>2MB, reach>400KB) |
| exec_TopExtractChunk0/1 | 577 | ~1702 | ~2.42 | FAIL |

Naga validates the emitted exec_TopChunk0 per-leaf module
(`/tmp/perleaf_exec_TopAccumChunk0.wgsl`, 2535 KB).

`perleaf_module2.py` greedy search varies the chunk index per chunked
callee — confirms TopAccum/Extract closure size is **structural**,
not a chunk-pick issue. Best moves:
- exec_TopChunk0: 281 → 205 KB reach (further -27%)
- exec_TopAccum/Extract: < 0.1 KB savings

`closure_anatomy2.py` on exec_TopAccumChunk0 explains why:
**`exec_TopExtractChunk0` alone contributes 1674.5 KB** of the 1762.5 KB
closure (95%). exec_TopExtract is the witness-extraction glue — only 2
arms in its switch, each arm reaches a giant subtree (validity
constraints / polynomial extraction). The remaining 88 KB is layout
helpers (back_ShaState 17 KB, etc.).

## Implication for SP7 risc0 wiring (iter 6c+)

iter 6b conclusion:
- **exec_Top chunks (witness generation) — risc0 wiring is unblocked.**
  205–281 KB reachable closure / 0.95–1.03 MB module. Both well under
  the device cliffs measured in iter 5b/d. Per-leaf module emitter +
  GPU dispatch can ship for the witness-gen phase.
- **exec_TopAccum + exec_TopExtract chunks — blocked.** The 2-arm
  switches don't shrink the giant downstream closures. Two options:
  1. Chunk the deeper validity / poly_ext code in zirgen (extra MLIR
     pass — likely needs a different chunking strategy than wide-mux
     splitting, since validity/poly_ext aren't muxes).
  2. Restructure the WGSL emission to factor TopExtract by instruction
     class (separate per-instruction extract paths).
  Either is multi-day; both are out of scope for this iteration.

## Next iteration

iter 6c: write a Rust per-leaf module emitter
(`risc0/circuit/rv32im/src/prove/hal/webgpu_witgen.rs` or similar)
that ports `perleaf_module.py`'s logic, generates per-instruction-class
WGSL modules at risc0 build time, and dispatches them from
`WebGpuCircuitHal::generate_witness`. Scope: WITGEN ONLY (not accum) —
exec_TopAccum still uses CPU `rust_steps::step_accum` until iter 6d
shrinks TopAccum/Extract.

Validation: SP-CR byte-identical check vs `rust_steps::generate_witness`
on R1 + xgboost + KeccakUnion(3) per
[[feedback_full_benchmarks_at_phase_end]].

Probe scripts captured here:
- `prune_chunked.py` (raw closure of every chunk in iter-6a output)
- `perleaf_module.py` (chunk0-everywhere rewriting + naga-validate)
- `perleaf_module2.py` (greedy per-callee chunk-pick search)
- `closure_anatomy2.py` (heaviest fns in a chunk's closure)
