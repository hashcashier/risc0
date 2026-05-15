Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.1 -- cycle_list buffer plumbing validated
Date: 2026-05-15
Commit: ac7a684c0

## What landed

Per-arm dispatch now uploads one `cycle_list: array<u32>` buffer per
arm and binds it at `@group(0) @binding(5)`. Each per-arm wrapper
reads its assigned cycle index via `cycle = cycle_list[gid.x]` rather
than `cycle = gid.x` directly, so dispatching `cycle_count_for_this_arm`
workgroups iterates over the *actual* cycle indices matching this
arm's major opcode -- not the first N rows of the witness.

Bind group layout grows 5 → 6 entries. The chunk0/chunk1 layout
stays at 5 (those dispatch over all cycles via the internal mux).

## Validation

1. `iter6d_g_assembled_arm_kernel_compiles_on_chrome` smoke test
   PASSES (0.15 s test wall) -- assembled per-arm kernel still
   Tint-compiles under the new wrapper format.
2. `iter6d_c_probe_xgboost` integration test -- ran end-to-end with
   `WASM_BINDGEN_TEST_TIMEOUT=600` (the earlier SIGKILL was the default
   20 s wasm-bindgen-test timeout, not GPU pressure):
   - **PASSED** in 104.53 s
   - xgboost wall: 104.31 s (vs 102.6 s baseline, +1.7 s ≈ 1.6%
     within noise)
   - prove_session_async wall_ms=104312, gpu_active_ms=58052,
     gpu_idle_ratio=0.443
   - 11 segments, 2,294,890 user cycles, 2,883,584 total cycles
   - `iter6d_g_arm_params_ph uploads=11 upload_bytes=352` -- per-arm
     params buffer was uploaded once per segment (correct -- 11
     segments × 32 byte params = 352 bytes total)
   - Receipt verification PASSED (`receipt.journal.decode::<f64>() == 30.528042544062632`)

The new cycle_list buffer plumbing runs cleanly in production for
all 11 segments without breaking xgboost correctness or adding
measurable wall time.

## Why this matters

Step 6.1 is the structural foundation for step 6.2 (synthesizing
`InstInputStruct` from preflight data) and step 6.3 (replacing the
no-op `data_buf[cycle] = data_buf[cycle]` body with a real arm sub-fn
call). Without per-arm `cycle_list` indexing, every per-arm kernel
would either dispatch over ALL cycles (wasteful, mux re-runs) or
only over the first N rows (wrong, ignores actual cycle distribution).

## Remaining iter-6d-g work

- 6.2: synthesize `InstInputStruct` from a side metadata buffer
  uploaded at preflight time. Each cycle's (minor, pcU32, state,
  mode) tuple needs to be pre-computed on CPU and uploaded as a
  `preflight_meta` storage buffer; WGSL constructs `InstInputStruct`
  per cycle from indexed reads.
- 6.3: replace each per-arm wrapper's no-op body with `exec_<arm>Chunk0(
  back_NondetReg(0, ...), inst_input, BoundLayout_<arm>Layout(...))`.
- 6.4: implement the extern_getMemoryTxn buffer upload from
  `preflight.txns` (arms that touch memory need this).
- 6.5: validate bit-exact output against rust_steps per-arm; gate
  rust_steps::step_exec off for arms covered.

Each of 6.2-6.5 is contained (50-200 lines) but the integration is
the bulk. Expected wall savings on completion: 5-6 s xgboost
(102.6 s → 96-97 s ≈ 17.1× CUDA per the iter-6d-g design doc).

## Architectural floor reminder

Per `project_sp7_witgen_savings_ceiling`: the practical floor on this
hardware/browser stack is **5-8× CUDA**, gated on multi-device or
Chrome/Dawn architectural improvements outside per-kernel scope.
Closing the gap from 17.1× → 5-8× requires changes not feasible from
within risc0/zirgen.
