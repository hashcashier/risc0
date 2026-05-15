Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2.0 -- shadow_init kernel validated
Date: 2026-05-16
Commit: 3476f2f90

## What landed

Self-contained shadow_init WGSL kernel that pre-populates the 5
outer Top layout cells (cols 14-18: nextPcLow, nextPcHigh,
nextState_0, nextMachineMode, isFirstCycle) in `data_buf` from a
per-cycle preflight metadata buffer.

3 bindings: data_buf (rw), params (uniform), preflight_meta (read).
Compiles synchronously via `create_compute_kernel` (small kernel,
~50 lines including BabyBear arithmetic), cached in
`SHADOW_INIT_KERNEL` thread_local.

## Validation

xgboost integration test PASSED in 103.97 s (vs 102.6 s baseline,
+1.4 s within noise; vs prior 104.31 s without shadow_init,
slight improvement).

Per-segment metrics from the run:
- `iter6d_g_shadow_meta` upload: 11 uploads × 4.19 MB = 46.1 MB
  (matches expected 262144 cycles × 16 bytes per segment)
- `iter6d_g_shadow_params` upload: 11 × 16 bytes = 176 bytes

Receipt verification PASSED (xgboost journal decodes correctly).

## Limitations of this step

shadow_init writes to data_buf BEFORE rust_steps runs. rust_steps
then OVERWRITES whatever shadow_init wrote with its own (correct)
values. So shadow_init's correctness can't be validated end-to-end
through the receipt -- need a separate diagnostic that reads
data_buf cells AFTER shadow_init but BEFORE rust_steps and compares
to rust_steps output.

This validation is deferred to step 6.2.2 (after the per-arm
wrapper synthesis is in place to actually consume shadow_init
values via `back_Reg(1, ...)`).

## Next: step 6.2.1

Replace the no-op @compute wrappers for the 8 zero-back_Reg arms
(MISC0/1/2, MUL0, DIV0, MEM0/1, ECALL0) with real synthesis:
1. Read 5 outer cells via back_Reg(1, lookup_TopLayout_*) (from
   shadow-init'd cells)
2. Compute x4 = sub(MONT_ONE, isFirstCycle._super)
3. Read major/minor from preflight_meta (need binding 6)
4. Construct InstInputStruct via exec_InstInput
5. Call exec_<arm>Chunk0(nondet, inst_input, layout)

Pilot: start with Misc0 (simplest — basic ALU ops, no memory or
ecall hooks). Validate xgboost still passes. Then replicate to
other 7.
