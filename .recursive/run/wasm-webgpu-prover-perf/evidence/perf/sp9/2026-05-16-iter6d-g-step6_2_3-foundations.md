Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2.3 -- short-circuit foundations + bug fixes
Date: 2026-05-16
Commit: e610550c3

## Headline

First end-to-end attempt at rust_steps short-circuit revealed TWO
foundational bugs that ALL prior iter-6d-g steps (6.2.0 through
6.2.1c) had silently been hitting -- probe-only tests passed
because rust_steps overwrites whatever GPU wrote.

## Bug 1: Tint compile FAILED for 8 of 13 arm wrappers since 6.2.1b

`synth_arm_wrapper` (added in commit 40a6e498d) called
`exec_InstInput` and `exec_OneHot_13_`. Neither is present in the
per-arm pruned delta -- the pruner only emits fns reachable from
the arm sub-fn. Both are in the full exec_top_chunk0.wgsl module
but the per-arm dispatch uses `WITGEN_BASELINE_WGSL + delta` (no
chunks).

Smoking gun in metric logs (visible only when iter6d_d_witgen_prewarm_async
finishes — after 90s):

```
iter6d_g_prewarm arm=misc0_chunk0 FAILED err=createComputePipelineAsync rejected: JsValue(GPUPipelineError: [Invalid ShaderModule "iter6d_g_arm_kernel"] is invalid due to a previous error.
```

Plus `iter6d_g_per_arm_dispatch dispatched=0 skipped=10` (10 arms
had cycles but kernel wasn't cached -- compile rejected).

The previous iter 6.2.1b/c "validated" status was an artifact of
probe-only tests: rust_steps ran every cycle correctly, so receipts
verified. The GPU dispatch was a no-op (kernel never cached).

### Fix (this commit):

- Rewrote `synth_arm_wrapper` to inline-construct InstInputStruct
  directly (NondetRegStruct/OneHot_8_Struct/InstInputStruct
  constructors -- no helper calls). The 8 minor cells are written
  by extended shadow_init now (cols 21-28), so the wrapper's
  minor_u value just goes into the struct field; the cells get
  written separately.
- Defined `back_NondetReg` / `back_Reg` inline at the top of the
  wrapper (the 8 zero-back-reg deltas don't emit these either,
  but the wrapper needs them to read previous-cycle next* cells).
- Extended `SHADOW_INIT_WGSL` to write ALL pre-known cells from
  preflight: cols 0 (cycle), 1-13 (majorOnehot), 14-18 (next* +
  isFirstCycle), 19-20 (major/minor), 21-28 (minorOnehot). 29
  cells/cycle, all derived from preflight (no arm sub-fn needed).
- naga-validated the assembled module standalone (`naga` accepted
  the 824 KB module).

## Bug 2: WGSL Chunks are sliced by CODE SIZE, NOT by minor opcode

`exec_Misc0Chunk0` only handles minor=0 (Add). The other 7 MISC0
minors (Sub/Xor/Or/And/Slt/SltU/AddI) are in `exec_Misc0Chunk1`.
This isn't documented in the iter-6d-g design notes -- step 6.2.4
("chunk1 repeat") was characterized as "diminishing returns since
chunk1 cycles are typically less frequent than chunk0". That
characterization was WRONG. chunk1 is REQUIRED for correctness, not
optional.

When the replace flag short-circuits rust_steps for MISC0 cycles,
only minor=0 cycles get GPU-written (by chunk0). Cells for the
other 7 minors are missing -> next cycle's dispatch reads garbage
-> "Reached unreachable mux arm" bail.

Status of chunk1 work needed for proper short-circuit:
- exec_top_chunk1.wgsl exists (1.1 MB, generated alongside chunk0)
- exec_(Misc0..ECall0)Chunk1 functions exist in chunk1.wgsl
- No per-arm chunk1 deltas exist (would need pruner run with
  target_chunk=1; pruner test requires `--features prove` which
  has host build errors -- crate deps gated to wasm32 target only)
- Alternative: use FULL EXEC_TOP_CHUNK1_WGSL + per-arm wrapper that
  calls exec_NxChunk1 directly. Module size ~1.1 MB per arm
  (8 modules × ~1.1 MB = 8.8 MB module text). Should fit Tint's
  whole-module 2 MB cliff comfortably.

Also: chunk0 vs chunk1 isn't just a code split -- they have
different semantics. chunk0 enters its arm-dispatch block on FIRST
CYCLE only (`if (x3._super) != 0u`); chunk1 enters on NON-FIRST
CYCLE (`if (x4) != 0u`). Need to verify both chunks dispatch
correctly relative to cycle 0.

## Other fixes (this commit)

- shadow_init binding 2 (preflight_meta) layout changed from
  `Storage` to `read_only_storage` to match the WGSL
  `var<storage, read>` declaration. Was causing GPUValidationError.
- Per-arm bind group layouts (bindings 5 = cycle_list, 6 =
  preflight_meta) changed from `Storage` to `read_only_storage`
  for the same reason.
- Per-segment arm mask plumbing:
  - `dispatch_witgen_per_arm_probe` returns `Vec<usize>` of
    dispatched arm_idx values.
  - webgpu.rs `set_witgen_gpu_replace_arm_mask(mask)` populates a
    bitmask of arms that BOTH have synthesized wrappers (in
    ZERO_BACK_REG_ARMS) AND actually dispatched this segment
    (kernel cached + cycles > 0).
  - rust_steps `cycle_short_circuited(major)` checks the mask
    bit-by-bit. CPU HAL leaves mask=0 (never short-circuits).
- `iter6d_g_replace_xgboost` test flips both
  WITGEN_GPU_PROBE_ENABLED and WITGEN_GPU_REPLACE_ENABLED.

## Validation status

- Probe-only test (`iter6d_c_probe_xgboost`): PASSES in 105.57s
  (vs 102.6s baseline, +3s within noise). All 13 arms now Tint-
  compile DONE. Per-segment dispatched count grows from 0
  (segment 0, kernels not ready) to 10+ (segments 3+, all
  cached arms dispatch).
- Replace test (`iter6d_g_replace_xgboost`): FAILS after segments
  0-1 because chunk0 alone covers only 1 minor per arm. Cells
  for other minors are missing -> mux unreachable bail.

## Next concrete step (step 6.2.4)

Dispatch BOTH chunk0 AND chunk1 (and any additional chunks) for
each of the 8 zero-back-reg arms. Mask the arm as "fully
dispatched" only when ALL chunks succeed.

Simplest path (avoids running pruner):
1. Add `EXEC_TOP_CHUNK1_WGSL` const (already exists in pruner)
2. Add `assemble_arm_chunk1_kernel(wrapper)` = chunk1 + wrapper
   (no baseline -- chunk1.wgsl is standalone)
3. Synthesize 8 chunk1 wrappers (use exec_InstInput / exec_OneHot_13_
   from chunk1.wgsl directly -- they ARE defined there)
4. Add `WITGEN_ARM_KERNELS_CHUNK1` cache
5. Prewarm 8 chunk1 kernels alongside chunk0
6. Dispatch both per arm per segment
7. Short-circuit only when BOTH chunks dispatched

Risks remaining:
- TopChunk0 vs TopChunk1 first-cycle/non-first-cycle semantics
  difference may need cycle 0 special handling
- shadow_init's pre-written cells may conflict with chunk1's
  internal writes (chunk1's arm dispatch also writes to those
  cells via exec_NondetReg etc.)
- chunk1 module size (1.1 MB) + wrapper -> still under 2 MB cliff;
  reachable closure (only the one sub-fn) should be small

## Architectural floor

Even with full chunk0+chunk1 dispatch landed and all 8 arms
short-circuiting correctly, the savings ceiling on xgboost is
~4.5s wall (102.6s -> ~98s = 17.4x CUDA). The 5-8x practical floor
remains gated on multi-device / Chrome-Dawn improvements outside
per-kernel scope, per project_sp7_witgen_savings_ceiling.
