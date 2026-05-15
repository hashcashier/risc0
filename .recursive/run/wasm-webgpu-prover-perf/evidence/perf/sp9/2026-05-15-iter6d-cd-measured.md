Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 iter-6d-c/d -- end-to-end measurement (Tint compile = 2.65 s, NOT 60 s)`
Date: 2026-05-15

## Headline

The iter-6d retro (commit 550acc140) said Tint compile would block
~60 s on the first segment and projected iter-6d-c as a net wall
regression. **That estimate was wrong.** Direct measurement on
xgboost with the iter-6d-d async prewarm flag on:

- `iter6d_d_witgen_prewarm_async DONE elapsed_ms=2650` (one-time
  per session)
- Segment 1: `iter6d_c_witgen_probe SKIP kernel_not_ready` (the
  ~2.6 s compile overlaps with execution + segment 1's preflight)
- Segments 2-11: 10 successful `iter6d_c_witgen_probe` dispatches,
  each `elapsed_ms=0` queue time
- xgboost wall: **102.86 s with probe on vs ~102.6 s baseline** =
  no measurable regression

The 60 s figure in the retro was based on the iter-6d-b wasm test's
62 s wall time, but most of that was cargo build + webdriver session
setup + adapter init -- the actual Tint compile is ~2-3 s.

## Why this matters

iter-6d-c full deployment (replacing rust_steps witgen with GPU
dispatch on segments 2-N) was deferred in the retro under
"requires solving the ~60 s compile cost." That gate is now gone:
the async prewarm makes the compile invisible to wall time on any
multi-segment fixture (xgboost = 11 segments, bn254 = 189, etc.).

Expected savings on xgboost with full iter-6d-c deployment:
- rust_steps `step_exec` per segment: ~520 ms (measured 2026-05-15)
- Savings if replaced for segments 2-11: 10 × 520 ms = **5.2 s wall**
- Realistic floor after iter-6d-c: 102.6 s - 5.2 s = **97.4 s =
  17.1× CUDA** (down from 18.0×)

## Why iter-6d-c probe doesn't ALREADY save 5.2 s

The current probe runs BEFORE rust_steps in generate_witness:
- Probe dispatches exec_TopChunk0 over the data buffer
- exec_TopChunk0 writes its outputs (correct only for cycles whose
  major opcode is the chunk0 arm)
- rust_steps then OVERWRITES every cell, producing the correct
  witness for every cycle

So the probe's writes are wasted compute (garbage gets overwritten
by rust_steps). To actually save time, rust_steps would need to
*skip* the cycles the probe already correctly handled. Currently
rust_steps doesn't know which cycles the probe touched.

## What iter-6d-e needs (concrete now that the compile gate is gone)

The per-cycle major-opcode dispatch:

1. **Multi-chunk pruned modules**: Run the iter-6c pruner over the
   gen_zirgen output for each `exec_TopChunkN`, not just chunk 0.
   Each chunk's reach is ~280 KB (sub-cliff). Vendor N modules
   under `risc0/circuit/rv32im/src/zirgen/exec_top_chunk{0,1,...}.wgsl`.
   The MuxChunk pass output already has chunks separated; only the
   per-chunk pruner runs are missing.

2. **Per-chunk cycle filter**: Augment preflight to emit a
   `Vec<u32>` per major-opcode-chunk giving the cycle indices that
   map to that chunk. Upload to GPU as `cycle_list` per chunk.

3. **Indexed @compute entry**: Each chunk's wrapper becomes:
   ```wgsl
   @group(0) @binding(5) var<storage, read> cycle_list: array<u32>;
   @compute @workgroup_size(64)
   fn exec_top_chunkN_main(@builtin(global_invocation_id) gid: vec3<u32>) {
     if (gid.x >= cycle_list_len) { return; }
     cycle = cycle_list[gid.x];
     ...
     exec_TopChunkN(...);
   }
   ```
   Dispatch with `workgroups = ceil(cycle_count / 64)`. The kernel
   only touches cycles in its list.

4. **Replace rust_steps for handled cycles**: Either gate
   `rust_steps::step_exec` on `cycle ∉ union(all_chunk_lists)`, OR
   trust the GPU writes and skip rust_steps entirely (after a
   correctness comparison phase).

Scope estimate: 2-3 days for the multi-chunk dispatch + cycle-list
upload + correctness comparison; a follow-on day for the
rust_steps short-circuit once parity holds.

## Evidence

`evidence/perf/sp9/iter6d_c_probe_xgboost.txt` -- full xgboost run
with probe=on. Stage timer lines:
- `iter6d_d_witgen_prewarm_async DONE elapsed_ms=2650.000`
- 1 × `iter6d_c_witgen_probe SKIP kernel_not_ready`
- 11 × `stage done iter6d_c_witgen_probe ... elapsed_ms=0.000`
- `prove_session_async wall_ms=102856.0 gpu_idle_ratio=0.439`

## Memory updates

`project_sp7_witgen_savings_ceiling`: needs revision -- iter-6d-c
ceiling is ~5 s (no longer gated on compile cost); iter-6d-deeper
ceiling is ~22 s (TopAccum) gated on the straight-line chunking
pass; recursion ~12 s ceiling gated on the same chunking pattern.
Total addressable ceiling is unchanged at ~40 s, but iter-6d-c is
now the FIRST tractable step, not a deferred follow-on.

`feedback-no-multiday-deferrals`: this retro vindicates the
directive. Estimated compile cost was 60 s based on inference from
test wall time; measurement found 2.65 s. Don't trust inferred
costs -- measure.
