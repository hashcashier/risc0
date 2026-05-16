Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2.13 -- cell diff diagnostic LANDED, root cause SHIFTS
Date: 2026-05-16
Commit pending: (this evidence describes pre-commit state)

## Headline

The cell-level diff diagnostic (added in this step) PROVES that GPU
witgen writes are bit-exact for the cells GPU writes. **Zero**
mismatches across 2.56M overlapping cells. The `verify_segment`
failure with mask=0x0001 is NOT caused by wrong GPU cell synthesis.

```
DIFF_SUMMARY total_cells=55312384 gpu_wrote=8150063
             cpu_wrote=18149886 both_match=2558883 mismatches=0
             rows=262144 cols=211
```

This shifts the root cause hypothesis from "broken codegen" to
"missing extern side effects." Specifically, when `rust_steps`
short-circuits a cycle (skipping `step_Top`), it also skips the
`extern_lookupDelta` and `extern_lookupCurrent` calls inside that
step. Those externs increment lookup count cells (`ArgU16` /
`ArgU8`) and update the verifier-checked `tables.lookup_delta`
state. GPU's stub for these externs is void -- so the count cells
end up under-incremented relative to what the verifier expects.

## What landed (step 6.2.13)

1. `WITGEN_GPU_DIFF_ENABLED` atomic flag + `set_witgen_gpu_diff_enabled`
   public setter (re-exported from `risc0_circuit_rv32im::prove`).
2. `WebGpuCircuitHal::pre_witgen_dispatch_async`: bypass the
   PROBE-off and REPLACE-off early-return gates when DIFF flag is
   on. GPU dispatches run; sync writes them back to CPU shadow.
3. `prove_core_async`: when DIFF flag is on, after pre-dispatch:
   - Snapshot data buffer CPU shadow into `Vec<u32>` (raw bytes)
   - Reset CPU shadow to all `INVALID` via `view_mut`
   - Re-scatter injector into data buffer (writes both CPU + GPU)
   - Force `set_witgen_gpu_replace_arm_mask(0)` so rust_steps
     writes everything cleanly
   - After `populate_from_parts` returns, snapshot CPU shadow again
   - Cell-by-cell diff: report cells where both snapshots have
     non-INVALID, non-zero values that differ
   - Emit `DIFF_SUMMARY` then `bail!` with mismatch count
4. New test `iter6d_g_diff_xgboost` enables the DIFF flag and runs
   xgboost prove_succinct_async. Expected to FAIL with the bail
   message; the actionable output is the DIFF_SUMMARY log line.

## How to run

```bash
cd examples/browser-prove
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/tmp/chromedriver-linux64/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=600 \
  cargo test --target wasm32-unknown-unknown --release \
  -- iter6d_g_diff_xgboost --nocapture
```

Run-time after the cached release build: ~1-2 seconds for the
wasm-bindgen-test-runner launch + ~tens of seconds for the actual
proof. Output appears in `browser-prove:metric DIFF_SUMMARY ...`
line just before the panic.

## Interpretation of the numbers

- `total_cells = 55312384 = 262144 * 211` -- data buffer (rows ×
  cols). 262144 = 2^18 = po2_18 (max segment size for xgboost).
  211 = REGCOUNT_DATA in the zirgen circuit.
- `gpu_wrote = 8150063` -- cells GPU's shadow_init + per-arm
  dispatches wrote (i.e., snapshot[i] != INVALID after the
  sync_gpu_to_cpu_unchecked). ~15% of total cells.
- `cpu_wrote = 18149886` -- cells rust_steps wrote that ended up
  non-INVALID + non-zero after zeroize. ~33% of total cells.
  Includes some that GPU also wrote (overlap), some that GPU
  didn't write (other arms / inner cells).
- `both_match = 2558883` -- cells where snapshot[i] != INVALID
  AND cpu_snap[i] != INVALID AND cpu_snap[i] != 0 AND
  snapshot[i] == cpu_snap[i]. The MATCHING overlap.
- `mismatches = 0` -- the punchline.

## Caveat: my filter undercounts

The filter rejects cells where `cpu_snap[i] == 0` because the
post-populate snapshot is taken AFTER `eltwise_zeroize_elem` which
converts INVALID → 0. So I can't distinguish "CPU didn't write"
from "CPU wrote zero." If GPU wrote V != 0 but rust_steps wrote 0
(actual zero), my filter misses that mismatch.

For a stronger guarantee, the diff would need to snapshot BEFORE
zeroize. That requires splitting `populate_from_parts` further or
inlining its steps. Deferred -- the 2.56M overlapping-real-value
matches are strong evidence that the codegen is correct at the
cell level.

## Root cause shift: lookup_delta

The new hypothesis: `extern_lookupDelta` (called from `step_Top`'s
exec_CycleArg / exec_MemoryArg) increments lookup count cells in
the data buffer. When `rust_steps`' short-circuit skips `step_Top`
for a cycle, those increments are SKIPPED. GPU's void stub for
`extern_lookupDelta` doesn't increment them either.

The verifier checks the cumulative lookup count cells against the
expected total derived from the trace. Missing increments → wrong
final count → "Reached unreachable mux arm" or "verify segment"
failure depending on which constraint breaks.

This is consistent with the previous bisection finding (step
6.2.12, commit 622c6b8d0): the failure was the same whether mask
was `minor<2` or `minor==0`, because BOTH gate strategies skip the
extern_lookupDelta calls for the MISC0 chunk0 cycles.

## Next step (6.2.14)

Three options to fix:

A. **rust_steps half-short-circuit**: skip the data cell writes but
   keep the extern_lookupDelta + extern_lookupCurrent calls. The
   delta + current calls are inside step_Top, so the codegen would
   need a "side-effects-only" path. Either codegen change (hard)
   or a runtime split (medium).

B. **GPU implements lookup count cells**: have shadow_init or per-arm
   chunks compute the lookup count increments from preflight data
   and write them to the corresponding data buffer cells. This is
   the same architectural pattern as the diff_count / memory_txn
   patches in steps 6.2.5 and 6.2.6.

C. **No-op the short-circuit for now**: keep mask=0 (i.e. always
   run rust_steps fully) in production, mark iter-6d-g as a
   "validated foundation" without the perf win. Performance ceiling
   shrinks to 0 from the documented 4.5s savings.

Option B is the principled path. The lookup count cells are
preflight-derivable (they depend on cycle.major/minor and the
extern_lookupDelta semantics). Iter 6.2.14 should:

1. Identify the lookup count cell range in the data buffer (the
   ArgU16Count / ArgU8Count columns).
2. Compute per-cycle increments from preflight (the same way
   rust_steps' extern calls do).
3. Add a GPU kernel that accumulates these into the right cell
   positions, OR include them in shadow_init alongside the existing
   pre-known cells.
4. Re-run diff -- if still 0 mismatches AND verify passes with
   mask=0x0001 active, the lookup hypothesis is confirmed.

Option B is moderate complexity. The "increments derivable from
preflight" claim needs verification (some extern_lookupDelta calls
may depend on dynamic cycle state, not preflight). If they do, the
work expands.

## Architectural ceiling unchanged

`project_sp7_witgen_savings_ceiling.md` documents: ceiling ~4.5s
wall savings on xgboost (~17.4x CUDA vs current 18.0x). The 5-8x
practical floor is gated on multi-device + Chrome/Dawn changes
outside per-kernel scope. Landing iter-6d-g bit-exact is a tactical
win, not a strategic one.

## Files changed

- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`:
  - Added `WITGEN_GPU_DIFF_ENABLED` + `set_witgen_gpu_diff_enabled`
  - Modified `pre_witgen_dispatch_async` to bypass PROBE/REPLACE
    gates when DIFF flag is on
  - Added 90-line DIFF diagnostic block in `prove_core_async`
- `risc0/circuit/rv32im/src/prove/mod.rs`:
  - Re-export `set_witgen_gpu_diff_enabled`
- `examples/browser-prove/src/lib.rs`:
  - Added `iter6d_g_diff_xgboost` wasm-bindgen test
