# M3b: eval_check mix-bank scalarization (+ operand-fusion negative result)

Date: 2026-07-03. Status: ACCEPTED — parity suite bit-exact, gates green.

## Gate results (same-day warm A/B vs M3 baseline)

| Gate | M3 | M3b | Movement |
|---|---:|---:|---:|
| BusyLoop wall | 3011 ms | **2764 ms** | −8.2% |
| KeccakUnion(1) wall | 44211 ms | **43172 ms** | −2.4% |
| xgboost wall | 27388 ms | **24320 ms** | **−11.2%** |
| xgboost vs native CUDA (5.7 s) | 4.8× | **≈4.3×** | |

Focused bench (po2=18, steady-state):

| Kernel | M3 | M3b |
|---|---:|---:|
| rv32im eval_check | 387 ms | **~130 ms (−66%)** |
| recursion eval_check | 84 ms | **~67 ms (−20%)** |

All four `*_eval_check_poly_ext_matches_cpu` parity tests bit-exact
(webgpu_hal custom def, recursion, keccak, rv32im production tapes).

## The change

`mix_tot`/`mix_mul` were `vec4<u32>` dynamically-indexed scratch arrays
(29 slots each for rv32im, 10 for recursion). Scalarized them exactly
like the M2a ext bank: mix value k lives at u32 words `4k..4k+3` in
`mixw_tot`/`mixw_mul` (module-scope `var<private>` in private mode,
`var<workgroup>` in lanes mode) with `mix_{tot,mul}_{load,store}`
helpers assembling/spreading `vec4<u32>`. Encoder and instruction
stream unchanged — pipeline keys/caches unaffected beyond the new WGSL.

**Threshold theory revised**: M2a concluded small vec4 arrays (≲30
slots) dodge the tax via register-select lowering. Wrong — recursion's
10-slot mix arrays also paid (84→67 after scalarization), and rv32im's
29-slot arrays were the dominant remaining cost (387→130). The rule is
now unconditional: **never dynamically index a vec4-typed local/private
array in hot WGSL under Chrome/Dawn; scalarize and assemble.** With fp
(u32 since M2a), the ext bank (scalarized M2a), and now the mix banks,
no dynamically-indexed vec4 arrays remain in the base interpreter.

## Operand fusion: tried, measured, REVERTED (twice)

The planned M3b lever (fold leaf Const/Get/GetGlobal into consumer
operands; host study said fp 681→360):

- v1 (fuse every leaf): parity-exact but rv32im 387→452 ms, recursion
  84→**231 ms**. Rematerializing a multi-use tap turns 1 storage read +
  K scratch reads into K storage reads — storage traffic dwarfs the
  scratch-shape saving.
- v2 (consts always, taps/globals only when single-use — never adds
  storage traffic): rv32im 387→403 ms, recursion 84→87 ms. Only −42
  slots (681→639; most rv32im tap values are multi-use) while the
  per-operand desc/payload decode cost ~15 ms. Net negative → reverted.

Lesson: the M2 "shape pressure ~2×" probe inflated fp AND ext AND mix
together; the pressure was mostly the vec4 mix arrays, not fp count.
Slot-count reduction via fusion attacks the wrong term.

## Session impact

eval_check GPU demand drops ~6.0 → ~2.8 s per xgboost session
(11×387+21×84 → 11×130+21×67). Segment phase carries most of the win
(serial, un-hidden); the succinct-phase share is partially absorbed by
pipelining slack.
