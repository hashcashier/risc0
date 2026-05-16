Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2.6 -- extern_getMemoryTxn patch + second localized bail
Date: 2026-05-16

## Headline

Patched `extern_getMemoryTxn` to read sequentially from preflight.txns
with a per-invocation call counter. Re-enabled MISC0-only mask 0x0001
to A/B. Result: `step_TopAccum failed at cycle=5557 major=0 minor=0:
Reached unreachable mux arm` (vs step 6.2.5's cycle=5866 -- bail
*moved* but didn't disappear).

The bail moved to an earlier cycle, suggesting the patched
extern_getMemoryTxn IS writing more correct cells than the previous
zero-stub but some inner mux check inside Misc0's accum step still
fails. Reverted mask=0; foundations retained.

## What landed (correctness-neutral with mask=0)

1. `patch_extern_get_memory_txn(wgsl)` in `wgsl_pruner.rs` --
   rewrites the `fn extern_getMemoryTxn(...) { return [0; 5]; }` stub
   body to read 5 u32 values from `preflight_txns_buf` at index
   `preflight_txn_start[cycle] + txn_call_idx`, advancing
   `txn_call_idx` per call. Prepends binding 8 (`preflight_txn_start`)
   and binding 9 (`preflight_txns_buf`) declarations and the
   `var<private> txn_call_idx: u32 = 0u` counter.
2. `build_preflight_txn_start(preflight) -> Vec<u32>` -- per-cycle
   `cycles[i].txn_idx` packed.
3. `build_preflight_txns(preflight) -> Vec<u32>` -- per-txn 5 u32s
   packed as `[prev_cycle, prev_word_low, prev_word_high, word_low,
   word_high]`, matching rust `get_memory_txn`'s 5-tuple.
4. Per-arm bind-group layout grows to 10 entries (added bindings 8 and
   9); chunk0 (`assemble_arm_kernel`) and chunk1 prewarm both apply
   `patch_extern_get_diff_count` THEN `patch_extern_get_memory_txn`
   before concat.
5. Bind groups bind the two new buffers per segment.

## Why the bail moved instead of disappearing

The remaining bail location is INSIDE step_TopAccum at cycle=5557
(MISC0/Add). step_TopAccum reads `inst_result._selector[0]._super`
first -- which IS shadow-init's majorOnehot value (correct). So the
arm0 branch enters. Inside, sub-arm muxes check specific
`inst_result.arm0.*._selector` cells.

Likely remaining causes:
- A cell written by FinalizeMisc -> WriteRd -> MemoryWrite that
  writes a 4th `extern_getMemoryTxn` consuming a 4th txn. The chunk0
  wrapper SHOULD consume 4 txns total (DecodeInst + 2x ReadReg +
  WriteRd's MemoryWrite). Each consumed correctly.
- BUT chunk1 also dispatches for the same cycle, consuming 3 txns
  (DecodeInst + 2x ReadReg, no WriteRd because minor=1 branch fails).
  Chunk1's MemoryArg writes overlap with chunk0's. They should match
  bit-for-bit because both pull same txns from same starting offset.
- Some cell that the accum step expects to be 1 (selector) is 0
  because the arm sub-fn's exec_NondetReg writes are conditional on
  the minor branch firing. For a different minor's chunk-wrapper
  dispatch, the inner minor selector cell might get written to 0.

Localizing requires line info on the 134 bail!() sites in
`steps.rs.inc`. Mass-sed of that vendored file was blocked by the
auto-classifier (correct call -- it's generated code; modifying it
isn't proper). Two paths forward:

1. Wrap the bail!() macro at the include! boundary to add line info
   without editing the generated file. Possibly via a custom macro
   defined before `include!`.
2. Build the data_buf diff diagnostic (originally planned at step
   6.2.2) to identify the first wrong cell per cycle. ~4-6 hours.

## Measured outcome

- Probe-only test (`iter6d_c_probe_xgboost`): PASSES 105.84s vs
  baseline ~105.57s (in-noise; no regression from new bind layout +
  buffers).
- Replace test (`iter6d_g_replace_xgboost`) with mask 0x0001: bail
  moved from cycle=5866 (step 6.2.5) to cycle=5557 (step 6.2.6).
  Bail location moved earlier, suggesting more cells are correct now
  but some still wrong. Mask reverted to 0.

## Architectural floor unchanged

Even if iter-6d-g lands bit-exact, ceiling is ~4.5s wall savings
(17.4x CUDA on xgboost vs current 18.0x), still far from the 5-8x
practical floor. The 5-8x floor remains gated on architectural
changes outside per-kernel scope.

## Status

Foundations landed across 6 commits (5db306deb through this one):
- shadow_init kernel pre-populating 29 outer cells/cycle
- per-arm chunk0 + chunk1 dispatch infrastructure
- synth_arm_wrapper + synth_arm_chunk1_wrapper
- per-arm bind layout (10 bindings) + read_only_storage for 5,6,7,8,9
- patch_extern_get_diff_count + preflight_diff_count_buf
- patch_extern_get_memory_txn + preflight_txn_start + preflight_txns_buf
- iter6d_c probe skip when replace flag on
- per-segment arm mask plumbing + chunks_ready=2 gate + minor<2 gating
- diagnostic map_err on step_exec + step_TopAccum

Probe-only path remains production. mask=0 forces no short-circuit.
All foundations are correctness-neutral and tested.
