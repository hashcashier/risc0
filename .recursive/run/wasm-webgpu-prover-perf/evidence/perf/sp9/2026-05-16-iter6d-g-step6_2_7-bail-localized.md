Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2.7 -- bail line-info diagnostic + final localization
Date: 2026-05-16

## Headline

Added a `bail!` macro shadow at the `include!("steps.rs.inc")` boundary
that wraps every "Reached unreachable mux arm" with the line number.
Re-ran MISC0-only mask 0x0001 A/B. Result:

```
step_TopAccum failed at cycle=5866 major=0 minor=0:
  Reached unreachable mux arm (steps.rs.inc:25306)
```

Line 25306 is the END of the major_onehot mux in `exec_TopExtract`.
Reached when major_onehot[i] is 0 for ALL i. But for a MISC0 cycle,
major_onehot[0] should be MONT_ONE (encode(1) = 268435454u), both
because:
- `shadow_init` writes `data_buf[1*rows + cycle] = MONT_ONE` for
  major=0.
- chunk1 wrapper calls `exec_OneHot_13_(x15._super, ...)` with
  x15._super = encode(0) = 0, which evaluates `isz(0 - 0) = MONT_ONE`
  and writes that to cell[1, cycle].

Both writes go to the same cell with the same value. Yet step_TopAccum
sees 0 there. Something between the GPU dispatch and the read is
corrupting the cell. Without a `data_buf` snapshot diagnostic
(originally step 6.2.2, deferred multiple times), I can't identify
which write or which cycle's GPU dispatch overwrites major_onehot[0]
to 0.

## What landed (correctness-neutral)

1. `macro_rules! bail` in `rust_steps.rs` BEFORE the
   `include!("steps.rs.inc")`. Wraps every `bail!("Reached unreachable
   mux arm")` (134 sites in steps.rs.inc) with line number context.
   Pattern: `"{} (steps.rs.inc:{})", msg, line!()`.
2. Verified probe-only test still passes: 105.11s vs ~105.57s baseline
   (in-noise, no regression).

## Diagnostic ceiling reached

Per the project memory's "Unblocks (multi-hour each)":

> 1. Build data_buf cell-by-cell diff diagnostic: post-dispatch GPU
>    snapshot buffer + async readback + side-by-side compare test.
>    ~4-6 hours. Identifies first wrong cell → per-arm synthesis fix.

This unblock is now the ONLY remaining path. The bail localization
narrows the symptom (major_onehot[0] reads as 0 at cycle 5866) but
not the cause (which GPU dispatch writes 0 there, and when).

Likely root causes to validate with the snapshot:
- A subsequent chunk's dispatch overwrites earlier writes (compute
  pass ordering across submit boundaries).
- A buffer-row indexing mismatch between shadow_init and arm wrappers
  (different `rows` interpretation).
- Some store inside Misc0Chunk0's call chain hits cell[1, cycle] by
  accident (column alias).

## Foundations now landed (8 commits total, all correctness-neutral)

- shadow_init kernel writing 29 outer cells/cycle from preflight
- per-arm chunk0 + chunk1 dispatch infrastructure
- synth_arm_wrapper + synth_arm_chunk1_wrapper
- per-arm bind layout with 10 entries (read_only_storage for 5..9)
- patch_extern_get_diff_count + preflight_diff_count buffer
- patch_extern_get_memory_txn + preflight_txn_start + preflight_txns
  buffers
- iter6d_c probe skip when replace flag on
- per-segment arm mask plumbing + chunks_ready=2 gate + minor<2 gating
- diagnostic map_err on step_exec + step_TopAccum
- bail! shadow macro carrying steps.rs.inc line info

These all work together if mask is ever set nonzero. With mask=0
forced, they have no functional effect (only the diff_count + txn
buffers are uploaded per segment, which the patched externs read
correctly when stub returns aren't called -- a slight overhead but
small).

## Architectural floor unchanged

Even if iter-6d-g lands bit-exact, ceiling is ~4.5s wall savings
(17.4x CUDA on xgboost vs current 18.0x). The 5-8x practical floor
remains gated on architectural changes (multi-device, async overlap,
dispatch parallelism) outside per-kernel scope.

## Status

Probe-only test passes 105.11s. Replace test (mask=0) reduces to
probe-only. Foundations landed and correctness-tested. The only
remaining blocker is the data_buf snapshot diagnostic that requires
adding GPU readback infrastructure to capture cell values mid-pipeline
for side-by-side compare against rust step_Top output.
