Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g ROOT CAUSE — GPU/CPU shadow buffer desync
Date: 2026-05-16

## Headline

After 8 commits of iter-6d-g foundations and 7 sub-steps of debugging,
the FUNDAMENTAL architectural issue is identified: my GPU dispatches
(shadow_init + per-arm chunks) write to GPU storage but DO NOT sync
the WebGpuBuffer's CPU shadow. step_TopAccum reads the stale CPU
shadow (still 0 from buffer initialization), not the GPU's correct
writes. Hence "major_onehot[0] reads 0 not MONT_ONE" at MISC0 cycle
5866 -> bail at steps.rs.inc:25306.

## The WebGpuBuffer model

```rust
pub struct WebGpuBuffer<T> {
    cpu: CpuBuffer<T>,              // CPU shadow
    gpu: Option<Rc<WebGpuBufferOwner>>,  // GPU storage
    cpu_dirty: Rc<Cell<bool>>,      // CPU has newer data
    cpu_stale: Rc<Cell<bool>>,      // GPU has newer data
    ...
}
```

`view()` and `view_mut()` call `assert_cpu_current("view")` which
panics if `cpu_stale == true`. So the CPU shadow must be in sync
when rust_steps reads it.

`sync_gpu_to_cpu(&self, hal).await` reads GPU → CPU shadow, then
sets cpu_stale = false. This is the ONLY way to import GPU writes
back into the CPU shadow.

## My GPU dispatches don't trigger sync

`shadow_init`, `dispatch_witgen_per_arm_probe` -> `dispatch_compute`
-> WebGPU pipeline submit. The submit returns once the command is
queued (not when GPU finishes). It does NOT mark `cpu_stale = true`
on the data buffer. So the CPU shadow's flags stay as they were.

## Why probe-only test passed all along

In probe-only mode, rust_steps runs step_Top for every cycle. step_Top
calls `data.view_mut(|view| ...)` which:
- Sets cpu_dirty = true (clears cpu_stale).
- Writes to the CPU shadow.

After rust_steps, the CPU shadow has rust's authoritative values.
GPU's probe writes are silently clobbered. step_accum reads the
correct rust values. Test passes.

The probe writes never mattered because rust_steps always overwrote
them. This is why iter-6d-c probe test could pass with garbage GPU
writes — they were ALWAYS overwritten.

## Why replace test fails

mask=0x0001 short-circuits MISC0 cycles in rust_steps. step_Top is
skipped for those cycles. The CPU shadow at those cells retains
whatever was there before — typically 0 from buffer initialization.

GPU wrote MONT_ONE to cell[1, 5866] (major_onehot[0]) but that
write is invisible to CPU. step_TopAccum reads cell[1, 5866] from
CPU shadow, gets 0, falls through the major mux, bails.

## Three fix paths

### (A) Async sync — ~1-2 days, smallest delta

Make `step_witgen` async, add `data.buf.sync_gpu_to_cpu().await`
after the GPU dispatch path but before rust_steps. The surrounding
pipeline (segment_prove, WebGpuProver::prove_with_opts_async) is
already async, so threading async through is local.

`CircuitWitnessGenerator::generate_witness` is sync in the trait
though, so the trait might need an async variant or a workaround.

After (A), the iter-6d-g foundations (shadow_init + extern patches +
per-arm chunks + mask) all start working as designed.

### (B) GPU-side step_TopAccum — multi-week

Run the entire accum step on GPU too, reading directly from GPU
storage. Avoids the CPU shadow round-trip entirely. Requires porting
the step_TopAccum codegen to WGSL (analog to what iter-6d-g did for
witgen). Much bigger effort.

### (C) Rust-side preflight shadow — ~1 day, partial benefit

For short-circuited cycles, write the shadow_init cells in RUST from
preflight (so CPU shadow gets the values). But the arm-specific cells
(decoded, sourceRegs, miscOutput, etc.) would still be missing —
those require running the arm sub-fn. So (C) doesn't save the cost
that iter-6d-g targets.

## Status

- Foundation infrastructure landed across 8 commits (all
  correctness-neutral with mask=0):
  - shadow_init kernel + per-cycle preflight_meta buffer
  - per-arm chunk0 + chunk1 dispatch + bind layout (10 entries)
  - synth_arm_wrapper + synth_arm_chunk1_wrapper
  - patch_extern_get_diff_count + diff_count buffer
  - patch_extern_get_memory_txn + txn_start + txns buffers
  - iter6d_c probe skip when replace flag on
  - per-segment mask plumbing + chunks_ready=2 gate + minor<2 gating
  - diagnostic map_err on step_exec + step_TopAccum
  - bail! shadow macro carrying steps.rs.inc line info
- Probe-only test: 105.11s vs ~105.57s baseline (in-noise, no
  regression).
- Replace test (mask=0): equivalent to probe-only (no short-circuit).

The only remaining unblock is implementing path (A): thread async
through generate_witness + add sync_gpu_to_cpu after GPU dispatch.

## Architectural ceiling unchanged

Per project memory: even if iter-6d-g lands bit-exact via path (A),
ceiling is ~4.5s wall savings (17.4x CUDA on xgboost vs current
18.0x). The 5-8x practical floor remains gated on architectural
changes outside per-kernel scope.
