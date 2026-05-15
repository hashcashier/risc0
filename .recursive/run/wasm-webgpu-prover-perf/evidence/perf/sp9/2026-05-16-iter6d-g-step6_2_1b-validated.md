Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2.1b -- Misc0 wrapper synthesis validated
Date: 2026-05-16
Commit: 40a6e498d

## What landed

Replaced the Misc0 per-arm @compute wrapper's no-op body with real
InstInputStruct synthesis that mirrors `exec_TopChunk0` lines
16934-16979 but bypasses externs by reading major/minor from
preflight_meta. The 4 inter-cycle reads (`back_Reg(1, ...)` on
nextPcLow/High/State/Mode) hit cells shadow-init'd by
`dispatch_shadow_init`. The cycle nondet reg (`back_Reg(0, ...)`)
also hits a shadow-init'd cell.

Synthesis chain (per misc0_synth_wrapper):
1. Read packed major|minor from preflight_meta[cycle*4+3]
2. Compute is_first_v = encode(1) at cycle 0, else 0
3. exec_NondetBitReg writes isFirstCycle cell + asserts bit
4. back_Reg(1, ...) reads next* shadow-init'd cells
5. exec_NondetReg(major, ...) and exec_NondetReg(minor, ...) write
   major/minor cells (cols 19/20)
6. exec_InstInput constructs InstInputStruct from x4-scaled shadow
   values + isFirstCycle (with proper x4 = 1-isFirstCycle modulus)
7. back_Reg(0, ...) reads cycle nondet reg (col 0, shadow-init'd)
8. exec_Misc0Chunk0(nondet, inst_input, instResult.arm0_layout)

Other 7 zero-back_Reg arms (MISC1/2, MUL0, DIV0, MEM0/1, ECALL0)
keep no-op wrappers for now to isolate any failure to Misc0.

## Validation

xgboost integration test PASSED in 103.84 s (vs 102.6 s baseline,
+1.2 s within noise).

The pass implies:
- Misc0 wrapper Tint-compiles successfully (would have produced
  `iter6d_g_prewarm arm=misc0_chunk0 FAILED` log otherwise)
- Misc0 wrapper dispatches without WebGPU/Tint runtime errors
  (would have produced `iter6d_g_per_arm_dispatch FAILED` log
  otherwise)
- The synthesis call chain (encode, sub, mul, exec_NondetReg,
  exec_NondetBitReg, exec_InstInput, exec_Misc0Chunk0) is
  WGSL-valid
- Receipt verification still passes (rust_steps still runs
  AFTER the GPU dispatch and overwrites whatever Misc0 wrote --
  so output remains correct regardless of the synthesis result)

## What's NOT yet validated

- Bit-exactness of Misc0's GPU output vs rust_steps' Misc0 output
  (would require a diagnostic test that reads data_buf cells
  written by Misc0 between shadow_init+per_arm and rust_steps,
  compares to rust_steps-only run -- step 6.2.2)
- The other 7 zero-back_Reg arms' wrapper synthesis (step 6.2.1c-h)
- Performance impact (still gated on rust_steps short-circuit,
  step 6.2.3)

## Next: step 6.2.1c

Replicate Misc0 synthesis to MISC1, MISC2, MUL0, DIV0, MEM0, MEM1,
ECALL0. Each requires only:
- Different sub-fn name (exec_Misc1Chunk0 vs exec_Misc0Chunk0)
- Different instResult.arm{N} index (arm1 for MISC1, etc)
- ECALL0 has 4-arg signature (extra `global3: u32`); requires a
  slightly different wrapper template

Estimated: ~50 lines per arm × 7 = ~350 lines, 1-2 hours of WGSL
generation + 7 × ~5 min = ~35 min of test cycles.
