Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2.4 -- bisection dead-end, short-circuit disabled
Date: 2026-05-16
Final commit: 12c8cd113

## Headline

iter-6d-g step 6.2.4 (rust_steps short-circuit) reached a bit-exact
synthesis blocker that can't be localized without a data_buf diff
diagnostic. Three full bisection iterations failed:

1. Full mask (0x0177, all 8 zero-back-reg arms) → mux unreachable
2. DIV0-only mask (0x0010) with chunks_ready=2 gate → mux unreachable
3. MISC0-only mask (0x0001) with minor<2 gating → mux unreachable

Plus a structural blocker discovered mid-bisection: each arm has 8
chunks (per chunk0_all.wgsl, one per minor opcode) but only chunk0
and chunk1 WGSL modules are vendored. So per-arm GPU dispatch can
cover at most 2 of 8 minors per arm even if synthesis were correct.

Short-circuit forced disabled (mask=0). Probe-only path restored as
production behavior. Foundations retained.

## Session arc (17 commits, all on recursive/wasm-webgpu-prover-perf)

Today's iter-6d-g work spans steps 6.2 → 6.2.4:

```
12c8cd113 iter 6d-g step 6.2.4: disable broken short-circuit (THIS)
0d0095432 iter 6d-g step 6.2.4: evidence note on chunk1 + probe-skip
1579a5a3f iter 6d-g step 6.2.4: skip iter6d_c probe when replace on
5db306deb iter 6d-g step 6.2.4: chunk1 dispatch + global buf fix
072572751 iter 6d-g step 6.2.3: evidence on foundations + chunk1 gap
e610550c3 iter 6d-g step 6.2.3: short-circuit foundations + bug fixes
5bb1c37b1 iter 6d-g step 6.2.1c: evidence for 8-arm synth validation
e36933299 iter 6d-g step 6.2.1c: replicate Misc0 synth to 8 arms
72b193d35 iter 6d-g step 6.2.1b: evidence for Misc0 synth validation
40a6e498d iter 6d-g step 6.2.1b: real Misc0 wrapper synthesis
481e01b2f iter 6d-g step 6.2.1a: per-arm bind group binding 6
8d8f82a47 iter 6d-g step 6.2.0a: shadow_init also writes col 0
17aa28932 iter 6d-g step 6.2.0: evidence for shadow_init validation
3476f2f90 iter 6d-g step 6.2.0: shadow-init kernel for outer cells
094e42ea2 iter 6d-g step 6.2: design doc for 8-arm replacement path
313a8c693 iter 6d-g step 6.2 CORRECTION: 8 of 13 arms have 0 back_Reg
```

## Critical discoveries (logged for memory update)

1. **Tint compile silently failed since iter 6.2.1b**: synth_arm_wrapper
   called exec_InstInput + exec_OneHot_13_ which aren't in baseline+
   delta. Test passed via rust_steps fallback (probe-only path). Log
   `iter6d_g_prewarm arm=X FAILED` was the smoking gun. Fix: inline
   InstInputStruct + define back_NondetReg/back_Reg locally.

2. **iter6d_c probe writes garbage to all arms when replace on**:
   extern_isFirstCycle_0 returns 0 always; extern_getMajorMinor returns
   [0,0]. So TopChunk1 enters every cycle as not-first, dispatches as
   MISC0/minor=0 to ALL cycles, writes wrong cells to all arm layouts.
   Per-arm dispatch only fixes one arm. Fix: skip iter6d_c probe when
   replace flag on.

3. **Latent pre-existing bug**: dispatch_witgen_per_arm_probe was
   binding `data` to BOTH binding 0 AND binding 1 (global_gpu was
   read from data.buf not global.buf). Only surfaced with chunk1
   added; fixed via threading `global` parameter through.

4. **Each arm has 8 chunks, only 2 vendored**: chunk0_all.wgsl shows
   exec_<arm>Chunk0..Chunk7 = one chunk per minor opcode. chunk0.wgsl
   and chunk1.wgsl only contain the per-arm chunk0 and chunk1. The
   chunk2..chunk7 modules don't exist. This caps short-circuit coverage
   at 2/8 minors per arm even if synthesis were correct.

5. **Synthesis is bit-incorrect even for MISC0 minors 0,1 alone**:
   MISC0-only mask with minor<2 gating still fails verify. The bug is
   in the synthesized wrapper itself, not in chunk coverage. Can't
   localize without a data_buf diff diagnostic.

## Foundations retained (survive mask=0)

- Extended SHADOW_INIT_WGSL: writes 29 pre-known cells/cycle from
  preflight (cols 0, 1-13, 14-18, 19-20, 21-28).
- Per-arm chunk0 + chunk1 dispatch infrastructure (WITGEN_ARM_KERNELS
  + WITGEN_ARM_KERNELS_CHUNK1 caches, prewarm in iter6d_d task,
  dispatch loop in dispatch_witgen_per_arm_probe).
- synth_arm_wrapper + synth_arm_chunk1_wrapper.
- Per-arm bind layouts use read_only_storage for bindings 5,6.
- Per-segment mask plumbing (set_witgen_gpu_replace_arm_mask + gated
  on chunks_ready=2 + global buf threaded correctly).
- Per-cycle minor-gating in rust_steps' cycle_short_circuited
  (minor<2 since only chunks 0,1 vendored).
- iter6d_c probe skip when replace flag on.

These remain landed under mask=0 (no functional effect on production
but the test infrastructure exists for a future iteration to flip
mask=1 once the synth bug is found via data_buf diff).

## Next session unblocks (multi-hour each)

1. **Build data_buf diff diagnostic** (4-6 hours):
   - Add post-dispatch GPU-to-GPU buffer snapshot in step_witgen
   - Async readback path in step_witgen async hook
   - Side-by-side test that runs xgboost twice with/without replace
   - Cell-by-cell diff + first-mismatch report
   - Then per-arm synthesis debug + fix
2. **Generate chunk2..chunk7.wgsl** (zirgen build pipeline invocation):
   Lifts the 2-of-8-chunk vendoring limit. Doesn't help if (1) synth
   is bit-wrong; only useful AFTER (1) is resolved.
3. **Pivot**: SP8 (readback coalesce, ~3%), SP9 (pipeline cache,
   ~5%), or submission-bound architectural changes (multi-device,
   async overlap, dispatch parallelism — the 5-8x target).

## Architectural floor reaffirmed

Per project_sp7_witgen_savings_ceiling.md: even if iter-6d-g
short-circuit landed bit-exact, the ceiling is ~4.5s wall savings
(17.4x CUDA on xgboost). The 5-8x practical floor remains gated on
multi-device + Chrome/Dawn architectural improvements outside per-
kernel scope. Today's per-arm investigation does not change that.

## Status

- Probe-only test still PASSES (105.57s baseline). No regression from
  the foundation work.
- Replace test would PASS now too (mask=0 → no short-circuit) — though
  the test name implies short-circuit, the gating mechanism is what's
  disabled.
- All 14 commits landed cleanly on recursive/wasm-webgpu-prover-perf.
