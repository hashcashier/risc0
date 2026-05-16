Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2.4 -- chunk1 dispatch + iter6d_c probe-skip + MISC0 bisection (BLOCKED)
Date: 2026-05-16
Commits: 5db306deb (chunk1 + global buf fix), 1579a5a3f (probe-skip + MISC0 bisection)

## Headline (achievement vs blocker)

**Achievement:** Short-circuit IS active. xgboost prove_session_async
drops from ~100s to 23s for 11 segments — roughly 4× faster witgen
when the replace flag is on. This proves the per-arm dispatch path is
wired correctly end-to-end (kernels prewarm, dispatch with cycle_list,
shadow_init for outer cells, rust_steps short-circuit).

**Blocker:** Receipt verification fails with "Reached unreachable mux
arm" even after isolating to MISC0 only (mask=0x0001). The cells GPU
writes don't bit-match what rust_steps would have written.

## Surfaces fixed this session arc (10 commits total, 6 earlier today + 4 in this arc)

This arc (chunk1 + probe + bisection):

1. **5db306deb** — chunk1 per-arm dispatch + global buf fix
   - Added WITGEN_ARM_KERNELS_CHUNK1 cache, synth_arm_chunk1_wrapper,
     prewarm 8 chunk1 kernels alongside chunk0, per-arm dispatch loop
     dispatches both
   - Fixed pre-existing latent bug: `global_gpu` was reading from
     `data.buf` not `global.buf`, causing GPUValidationError aliasing
     once chunk1 added a code path that exercises both bindings

2. **1579a5a3f** — skip iter6d_c probe when replace flag on
   - CRITICAL FINDING: iter6d_c full TopChunk0/Chunk1 path actively
     corrupts cells because extern_isFirstCycle_0 returns 0 always
     (TopChunk0 never enters arm dispatch; TopChunk1 enters every cycle
     with major=0 from extern_getMajorMinor → dispatches MISC0 path to
     EVERY cycle, writing wrong cells to MISC0 layout for non-MISC0
     cycles too)
   - Per-arm dispatch then only fixes one arm's layout per cycle;
     non-matching arms retain garbage. rust_steps overwrites for arms
     NOT short-circuited but for short-circuited arms the garbage
     persists.
   - Fix: skip iter6d_c probe entirely when WITGEN_GPU_REPLACE_ENABLED
     is set. Per-arm dispatch alone does the work correctly.

## Measured outcomes

- Probe-only test (iter6d_c_probe_xgboost): PASSES 105.57s (+1.5s noise
  vs 102.6s baseline). All 13 arms Tint-compile. Per-arm chunk0+chunk1
  prewarm completes within ~90s background.
- Replace test (iter6d_g_replace_xgboost) with mask=full (0x0177):
  prove_session_async 23s for 11 segments. Verify fails.
- Replace test with mask=0x0001 (MISC0 only): prove_session_async 23s.
  Verify fails. **Synthesis bug isolated to MISC0 arm itself.**

## Remaining mystery

Even with only MISC0 short-circuited:
- shadow_init writes 29 pre-known cells/cycle (0, 1-13, 14-18, 19-20,
  21-28) from preflight — these should match rust_steps' values
- Per-arm chunk0+chunk1 wrappers dispatch with correct preflight inputs
  (major/minor from preflight_meta, pcU32 from back_Reg on shadow-init'd
  cells)
- Both wrappers call exec_NondetBitReg / exec_NondetReg / exec_InstInput
  / exec_OneHot_13_ — shared writes are idempotent (same values)
- exec_Misc0Chunk0 handles minor=0 path; exec_Misc0Chunk1 handles
  other minors

Yet the proof breaks at receipt verification. The first wrong cell is
not identified.

## Why bisection further isn't tractable

Each test cycle is ~10 min (wasm build) + ~5 min (test run) = ~15 min.
Hypothesis-driven debugging without a data_buf diff diagnostic burns
this 15 min per attempted explanation.

The right next step is **a real bit-exact diagnostic**: run xgboost
twice on the same segment, capture data_buf after both runs, diff them
cell-by-cell. Identifies which cells differ and at which cycles.

This was originally step 6.2.2 in the iter-6d-g design, deferred to
step 6.2.3 (short-circuit). It is now the critical missing piece. The
diagnostic itself needs:
- A debug-only readback path in webgpu.rs that copies data_buf to host
- A side-by-side test that runs the same fixture twice (with/without
  replace) and stores both data_buf snapshots
- Cell-by-cell diff + report

Estimated effort: 4-6 hours of focused implementation + test cycles.

## Status of stated goal

Short-circuit foundations are sound (~4× witgen speedup measured) but
correctness is broken. Receipt verification fail blocks any wall-time
A/B comparison. The achievable wall-time savings if synthesis were
correct: ~4.5s (per project_sp7_witgen_savings_ceiling) on xgboost,
moving CUDA ratio from 18.0× to ~17.4×.

The 5-8× practical floor documented in project memory remains
unreachable via per-kernel work alone, requiring multi-device or
Chrome/Dawn architectural improvements outside this iteration's scope.

## Concrete next step (multi-hour, requires fresh session)

Build the bit-exact data_buf diff diagnostic (step 6.2.2 deferred from
2026-05-16 morning), then use it to find the first wrong cell in MISC0
short-circuit output.
