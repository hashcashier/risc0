Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2.8-11 -- async refactor (path A) LANDED
Date: 2026-05-16
Commit: 9dfbb2d45

## Headline

The architectural CPU/GPU shadow buffer desync blocker identified in
step 6.2.7 is RESOLVED. Async refactor (path A) successfully lands:

- `iter6d_c_probe_xgboost`: PASSES 104.25s vs 105.57s baseline (in-noise)
- `iter6d_g_replace_xgboost`: prove completes (witgen + accum + commit
  + finalize) but `verify_segment` fails — meaning GPU-written cells
  pass all schema validation but final accumulator polynomial has
  values the verifier rejects. Test takes 28.76s (vs ~1s in earlier
  failure modes).

## What landed (commit 9dfbb2d45)

1. **WitnessGenerator split into allocate + populate phases**
   - `allocate_buffers(hal, global_vec, cycles, injector)` -> creates
     global/code/data buffers + scatters injector data
   - `populate_from_parts(hal, circuit_hal, mode, trace, cycles, ...)`
     -> calls generate_witness + zeroize + alloc accum, returns Self
   - `preflight_components(preflight) -> (global, injector, cycles, trace, po2)`
   - Sync prove path (`WitnessGenerator::new`) unchanged.

2. **WebGpuCircuitHal::pre_witgen_dispatch_async** (NOT in trait)
   - Async method called from `prove_core_async` BEFORE sync
     `generate_witness`. Does shadow_init dispatch + per-arm chunk0+chunk1
     dispatches + sync_gpu_to_cpu_unchecked on data buffer.
   - Sets the rust_steps short-circuit mask based on dispatched arms.
   - EARLY-RETURNS when probe flag OR replace flag is off (avoids
     conflict with rust_steps when no short-circuit happens).
   - Strips iter-6d-g dispatch logic from sync `generate_witness`
     (which now just runs rust_steps).

3. **WebGpuBuffer::sync_gpu_to_cpu_unchecked**
   - Bitwise GPU->CPU readback variant that preserves `Val::INVALID`
     (0xffffffff) cells. Standard `sync_gpu_to_cpu` rejects them via
     `CheckedBitPattern`. The unchecked variant reads bytes as u32
     (Pod) and transmutes via `repr(transparent)` to T.
   - Required because GPU's shadow_init + per-arm chunks only write
     SOME cells (~29 outer + arm-specific). Other cells stay INVALID;
     standard validator chokes; unchecked variant preserves the
     INVALID sentinel so rust_steps' `set_at` sees uninitialized cells
     correctly (no `inconsistent set` panic).

4. **shadow_init WGSL convention fix**
   - Was writing `encode(next_cycle.state)` etc. for nextPc/State/Mode
     cells (semantically "next cycle's state").
   - Fixed to write `encode(current_cycle.state)` matching scatter's
     `build_injector::set_cycle`. Earlier convention broke cycle 0's
     wrap-around eqz check at `top.zir:50` (expected ControlDone state
     = encode(7) from padding cycle rows-1; was getting encode(0)
     from cycle 0 state).

5. **prove_core_async orchestration**
   - Refactored to: `allocate_buffers` -> `pre_witgen_dispatch_async`
     -> `populate_from_parts`. The async pre-dispatch hook is the
     architectural fix.

## Why the previous failures are GONE

Step 6.2.7's bail at `steps.rs.inc:25306` (major_onehot[i] = 0 for all
i) was caused by GPU writes never reaching CPU shadow. With
`sync_gpu_to_cpu_unchecked`, GPU writes are imported BEFORE rust_steps
reads. The major_onehot cells now have correct values when read.

## What's left for the replace path

The `verify_segment` failure indicates GPU's per-arm dispatches still
produce cells that don't match what rust_steps would write. Likely
candidates:

- **extern_lookupDelta**: rust updates `tables.lookup_delta` count
  cells. GPU stub is void. Per-cycle lookup arg counts (the ArgU16
  count cells written by exec_CycleArg / exec_MemoryArg) need
  preflight-derived values or a state-tracking GPU buffer.
- **extern_lookupCurrent**: defined in WGSL stub returning 0 but NOT
  called by chunk0/chunk1 (only def, no call sites). Not a blocker.

Estimating 1-2 more extern patches needed. Each iteration is ~15min
(compile + browser test). Probably 5-10 hours of focused work to land
the replace path bit-exact.

## Architectural ceiling unchanged

Per `project_sp7_witgen_savings_ceiling.md`: ceiling is ~4.5s wall
savings on xgboost (~4.4%, 17.4x CUDA vs current 18.0x). The 5-8x
practical floor remains gated on multi-device + Chrome/Dawn
architectural changes outside per-kernel scope. Even with full
iter-6d-g landed bit-exact, we stay above the 5-8x target floor.

## Foundations summary (9 commits today)

Today's iter-6d-g arc spans 9 commits, each landing correctness-neutral
foundation pieces:

```
9dfbb2d45 step 6.2.8-11: async refactor (THIS)
77b45ce78 step 6.2.7 evidence: ROOT CAUSE GPU/CPU shadow desync
ebdaca39b step 6.2.7: bail line-info diagnostic
4be17b350 step 6.2.6: extern_getMemoryTxn patch
76a8c0329 step 6.2.5: extern_getDiffCount patch
f35758ab2 step 6.2.4 evidence: bisection dead-end
12c8cd113 step 6.2.4: disable broken short-circuit
0d0095432 step 6.2.4 evidence: chunk1 + probe-skip
1579a5a3f step 6.2.4: skip iter6d_c probe when replace on
```

Plus prior arc: shadow_init kernel, per-arm wrappers, layout fixes,
diagnostic map_errs, bail shadow macro.

All landed correctness-neutrally (mask = 0 in production). The async
refactor IS the architectural fix; remaining work is per-extern
diagnosis.
