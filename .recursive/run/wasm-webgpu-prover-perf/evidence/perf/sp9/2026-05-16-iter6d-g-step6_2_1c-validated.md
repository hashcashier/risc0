Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2.1c -- 8-arm synthesis validated
Date: 2026-05-16
Commit: e36933299

## What landed

Generalized Misc0 synthesizing wrapper to all 8 zero-back_Reg arms
identified in the 2026-05-16 audit:

- arm_idx 0: MISC0 → exec_Misc0Chunk0
- arm_idx 1: MISC1 → exec_Misc1Chunk0
- arm_idx 2: MISC2 → exec_Misc2Chunk0
- arm_idx 3: MUL0 → exec_Mul0Chunk0
- arm_idx 4: DIV0 → exec_Div0Chunk0
- arm_idx 5: MEM0 → exec_Mem0Chunk0
- arm_idx 6: MEM1 → exec_Mem1Chunk0
- arm_idx 8: ECALL0 → exec_ECall0Chunk0 (4-arg variant with global)

The other 5 arms (CONTROL0, BIGINT0, POSEIDON0/1, SHA0) keep
no-op wrappers — they have internal back_Reg deps that would read
uninitialized data without a deeper materialization scheme.

## Validation

xgboost integration test PASSED in 103.98 s (vs 102.6 s baseline,
+1.4 s within noise). All 8 synthesized wrappers Tint-compile and
dispatch without WebGPU runtime errors. Receipt verification
passes (rust_steps still authoritative).

## Today's session arc (9 commits)

1. 313a8c693 — Per-arm back_Reg audit correction (8 arms have 0 deps)
2. 094e42ea2 — Design doc for 8-arm replacement path
3. 3476f2f90 — shadow_init kernel (step 6.2.0)
4. 17aa28932 — shadow_init evidence
5. 8d8f82a47 — shadow_init also writes col 0 (cycle reg)
6. 481e01b2f — preflight_meta binding 6 plumbing (step 6.2.1a)
7. 40a6e498d — Misc0 real synthesis (step 6.2.1b)
8. 72b193d35 — Misc0 synthesis evidence
9. e36933299 — Generalized to 8 zero-back_Reg arms (step 6.2.1c)

## What's still needed for actual perf gain

Step 6.2.2: bit-exact validation
  - Build diagnostic that reads data_buf cells written by per-arm
    GPU dispatch BEFORE rust_steps overwrites
  - Compare to rust_steps-only output for the same cells
  - Per-arm column ranges differ (each arm writes to its
    instResult.armN slot + the shared output cells)

Step 6.2.3: rust_steps short-circuit
  - Once 6.2.2 confirms bit-exact match for the 8 arms, gate
    `step_exec` to skip cycles whose `cycle.major` is in
    {0, 1, 2, 3, 4, 5, 6, 8}
  - This is where the actual ~4.5 s wall savings materialize
    (102.6 s → ~98 s ≈ 17.4× CUDA on xgboost)

Step 6.2.4: chunk1 repeat
  - The 8 arms also have chunk1 sub-fns; replicate the synthesis
  - Diminishing returns since chunk1 cycles are typically less
    frequent than chunk0

## Architectural floor unchanged

Even after iter-6d-g full completion (~17.4× CUDA), the practical
floor of 5-8× CUDA per project_sp7_witgen_savings_ceiling is
unreachable via per-kernel work. Closing further requires
multi-device or Chrome/Dawn architectural improvements outside
the per-kernel optimization scope.
