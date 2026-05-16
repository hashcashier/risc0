Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2.5 -- extern_getDiffCount patch + first localized bail
Date: 2026-05-16

## Headline

Patched `extern_getDiffCount` stub to read from a per-cycle preflight
buffer. Re-enabled MISC0-only short-circuit (mask 0x0001) for an A/B
test. Result: `step_TopAccum failed at cycle=5866 major=0 minor=0:
Reached unreachable mux arm`. First time we localized which cycle's
mux bails -- previous bisection iterations only saw the bare error.

Reverted mask to 0 (probe-only path restored as production). Retained
the extern_getDiffCount patch + preflight_diff_count buffer plumbing
as foundation for a future iteration.

## What landed (correctness-neutral with mask=0)

1. `patch_extern_get_diff_count(wgsl)` in
   `risc0/circuit/rv32im/src/prove/wgsl_pruner.rs` -- rewrites the
   `fn extern_getDiffCount(txn_cycle) { return 0u; }` stub body with a
   buffer-backed lookup `encode(preflight_diff_count_buf[decode(txn_cycle)])`.
   Prepends the binding-7 declaration so the patched fn sees it.
2. `build_preflight_diff_count(preflight) -> Vec<u32>` packs 2 u32 per
   cycle matching rust `get_diff_count(cycle*2+i)` semantics.
3. Per-arm bind-group layout extended to binding 7; bind groups bind
   the diff-count buffer.
4. `assemble_arm_kernel` now patches `WITGEN_BASELINE_WGSL` before
   concat; chunk1 prewarm patches `EXEC_TOP_CHUNK1_WGSL` before concat
   with the wrapper.
5. Diagnostic `map_err` wrappers added to `step_exec` and `step_TopAccum`
   call sites in `rust_steps.rs::run_witness_steps` and
   `run_accum_steps`. Tags the bail with the cycle / major / minor so a
   future regression localizes immediately.

## What didn't work (mask 0x0001 re-enable)

Test failure (xgboost, segment 4 onwards):
```
step_TopAccum failed at cycle=5866 major=0 minor=0: Reached unreachable mux arm
```

Cycle 5866 is a MISC0/Add (minor=0). It was short-circuited (mask
0x0001 set bit 0). GPU MISC0 chunk0 wrapper dispatched + wrote cells.
But step_TopAccum at cycle 5866 reads the per-arm `_selector[0]._super`
cell expecting it = encode(1), finds something else, falls through to
the no-arm-matched bail (steps.rs.inc:30458).

Root cause: arm sub-fns call FIVE distinct externs that ALL need
preflight backing for short-circuit to be bit-exact:

| Extern | Stub return | Rust impl | Patched? |
|---|---|---|---|
| `extern_getDiffCount(cycle)` | `0u` | `preflight.cycles[cycle/2].diff_count[cycle%2]` | YES (this iteration) |
| `extern_getMemoryTxn(addr)` | `[0u,0,0,0,0]` | sequential preflight.txns | NO |
| `extern_isFirstCycle_0()` | `0u` | `cycle == 0` | (inline in wrapper) |
| `extern_getMajorMinor()` | `[0u, 0u]` | preflight.cycles[cycle].(major, minor) | (inline in wrapper) |
| `extern_divide(...)` | `[0u,0,0,0]` | hardware divide | NO (DIV0 only) |

Patching `getDiffCount` fixes DoCycleTable's diff cells but
`DecodeInst` -> `MemoryRead` -> `extern_getMemoryTxn` still returns
zeros. So opcode/func3/func7/rs1/rs2/rd cells all end up as zero in
the GPU-short-circuited cycle. Downstream, step_TopAccum's arm
selector logic reads those zero cells (the _selector cell is derived
from majorOnehot but the arm-specific eqz writes inside DecodeInst
fail in WGSL silently — eqz is a no-op there — leaving zero in cells
the accum step relies on).

## Why this is multi-day, not multi-hour

`extern_getMemoryTxn` is the hard one:

- Stateful (txn_idx advances within a cycle execution)
- Per-cycle call count varies (DecodeInst = 1, ReadSourceRegsChunk0 = 1-2
  depending on rs1==rs2, plus per-arm reads for arm-specific memory ops)
- Both chunk0 AND chunk1 dispatches call it (each with its own thread-
  local var<private> counter that resets to 0 per dispatch) -- so each
  chunk would consume txns starting at the cycle's `txn_idx`. Same
  txns get read twice but written to the same cells with same values.
- Rust's exec_Misc0 calls it ~3 times for the cycle. So GPU and rust
  consume the same 3 txns starting from txn_idx. The buffer layout
  is straightforward (5 u32 per txn + cycle.txn_idx as offset).

But we also need to handle `extern_divide` for DIV0 (16-byte buffer per
DIV0 cycle), and the chunk2..chunk7 modules still aren't vendored
which caps coverage at minor < 2 per arm anyway.

Realistic effort: 4-8 hours to patch all needed externs + verify with
A/B against a single arm. Then 2-3 days to lift coverage above
`minor < 2` (requires re-running zirgen pipeline to emit chunk2-7).
Total: 4-5 days. Ceiling stays at ~4.5 s wall savings (17.4x CUDA on
xgboost, vs current 18.0x).

## Architectural floor unchanged

Per `project_sp7_witgen_savings_ceiling`: even if iter-6d-g lands
bit-exact, the savings ceiling is ~4.5s wall (102.6s -> ~98s = 17.4x
CUDA), still far from the 5-8x practical floor. The 5-8x floor
remains gated on architectural changes (multi-device, async overlap,
dispatch parallelism) outside per-kernel scope.

## Status

- Probe-only test (iter6d_c_probe_xgboost): PASSES 105.26s vs 105.57s
  baseline (in-noise; no regression from the extern_getDiffCount patch
  + diff_count buffer + bind layout extension).
- Replace test (iter6d_g_replace_xgboost): mask is forced to 0 in
  production -> reduces to probe-only behavior.
- Foundations landed and tested:
  - extern_getDiffCount patch (correctness-neutral when mask=0)
  - preflight_diff_count buffer build + upload + bind
  - per-arm bind layout has 8 entries (added binding 7)
  - diagnostic `map_err` on step_exec + step_TopAccum exposes the
    bail cycle/major/minor for any future regression
