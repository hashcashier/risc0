Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP7 iter 6d-g design -- per-major-arm dispatch concrete scope`
Date: 2026-05-15

## Verdict

Per-major-arm dispatch is **architecturally viable** and the path
to iter-6d-c witgen replacement:

- exec_TopChunk0 has **13 major opcode arms** (decode values:
  0, 134217711, 268435454, 402653165, 536870908, 671088619,
  805306362, 939524073, 1073741816, 1207959527, 1342177270,
  1610612724, 1879048178).
- Each arm calls one sub-fn (exec_Control0, exec_Mem0, exec_Sha0,
  exec_BigInt0, etc).
- exec_TopChunk1 has similar count → ~13 + ~13 = **~26 distinct
  major-arm kernels** needed.
- Per-arm kernel size: **820-1000 KB** (well under Chrome's 2 MB
  whole-module cliff).
- Tint compile per kernel: ~2-3 s (measured for one arm).
- naga validates. Tint validates. Dispatch works.

## What the session DID land

`risc0/circuit/rv32im/src/zirgen/exec_sha0_chunk0_only.wgsl`
(838 KB) -- one concrete per-arm kernel. Smoke
`iter6d_f_take2_sha0_per_arm_compiles_on_chrome` confirms Tint
compile + dispatch passes (phase=OK in 0.14 s test wall).

40 per-chunk kernels generated under `/tmp/iter6d_g_*.wgsl`
totaling 33 MB. Per-major-arm (sub-fn level, not per-Chunk)
would be ~26 kernels × ~1 MB = 26 MB.

## What iter-6d-g needs to land

**Vendoring 26+ MB of WGSL into the crate makes the wasm too big.**
Three production-grade paths:

### Path A: runtime generation from packaged artifacts

Ship the gen_zirgen source artifacts in the crate:
- prelude (~50 KB)
- types.wgsl.inc (~520 KB)
- layout.wgsl.inc (~250 KB)
- steps.wgsl (~9.2 MB) -- big but compressible

Total ~10 MB uncompressed, ~1.5-2 MB gzip-compressed. Decompress
at HAL init. Run the pruner (port of `gen_chunk1.py` to Rust,
already partially done in `wgsl_pruner.rs`) at HAL init to emit
the 26 per-arm modules. Each pruner run is ~5-10 ms; total ~150
ms one-time cost.

### Path B: vendor only the deltas

The prelude+types+layout (~820 KB) is SHARED across every per-arm
kernel. Vendor it ONCE. Then vendor only the per-arm "deltas"
(just the unique sub-fn bodies + supporting fns reachable from
that arm) -- each ~50-100 KB. Total ~820 KB shared + 26 × 80 KB =
~3 MB vendored.

At HAL init, concatenate `prelude + types + layout + delta_N` for
each kernel and pass through `create_compute_kernel_async`. Same
total compile time as path A.

### Path C: subset to hot kernels

Profile real workloads to find which major opcode arms actually
fire on hot fixtures (xgboost, keccak, bn254). Likely many arms
are cold (ecall arms, sha arms for non-sha programs). Vendor only
the top-K hot kernels. K=5-8 covers the common case at ~5-8 MB
of vendored WGSL.

**Recommended: path B**. Smaller vendor footprint than A (~3 MB
vs ~10 MB), simpler than full runtime generation, no subset-tuning
needed.

## Concrete remaining work

1. **Pruner: emit "delta-only" modules.** Add a
   `pruned_delta_at_chunk(steps, entry, target_chunk)` that emits
   ONLY the steps fns in the closure -- no prelude, types, or
   layout. ~30 lines of pruner change.
2. **Vendor deltas.** Run pruned_delta for each of the 13+13=~26
   major-arm entries. Vendor as
   `risc0/circuit/rv32im/src/zirgen/exec_top_chunk{0,1}_arm{N}_delta.wgsl`.
   Total ~3 MB.
3. **HAL: per-arm kernel cache.** Extend
   `WITGEN_TOP_CHUNK0_KERNEL` to a map keyed by major opcode arm.
   At HAL init, async-compile all 26 kernels (Chrome pipelines).
4. **Per-cycle major-opcode lookup.** Preflight already tracks
   the major opcode per cycle internally (via `cycles[i].major`
   or equivalent in `PreflightTrace`). Need to expose it as a
   slice for upload.
5. **Multi-kernel dispatch.** In generate_witness: build a
   per-arm cycle-list buffer (CPU side, fast). Dispatch each arm's
   kernel over its cycle list. Kernel reads `cycle_list[gid.x]`
   instead of `gid.x`.
6. **rust_steps short-circuit.** Once GPU output verified
   bit-exact against rust_steps, gate `step_exec` to run only on
   cycles not covered by GPU dispatch (none -- the 26 arms cover
   all rv32im cycles by construction).

Scope estimate: 3-5 days of focused work. Each piece small (~50-
200 lines); the integration is the bulk.

## Expected savings

Per `project-sp7-witgen-savings-ceiling` and the measured
`rv32im_witgen ~520 ms / segment`:
- iter-6d-g lands -> 5.2 s xgboost wall saved (segments 2-11
  use GPU, segment 1 prewarm SKIPs as today)
- xgboost wall: 102.6 s -> 97.4 s = **17.1× CUDA**
- Plus iter-6d-deeper for TopAccum (22 s ceiling, blocked on
  zirgen straight-line chunking pass) → **11× CUDA ceiling**
- Plus recursion GPU witgen (12 s ceiling, same chunking pass) →
  closer to 9× CUDA

Practical floor on this hardware/browser stack remains **5-8×
CUDA** per the SP6c submission-bound diagnosis -- closing further
requires multi-device or Chrome/Dawn architectural improvements.

## Session iter-6d arc

iter-6d-a: vendored exec_TopChunk0.wgsl (chunk0 everywhere). Probe.
iter-6d-b: Tint compile validation on Chrome.
iter-6d-c: probe-mode dispatch in generate_witness (process-flag gated).
iter-6d-d: async Tint prewarm via create_compute_pipeline_async.
iter-6d-e: dual-chunk (chunk0 + chunk1) dispatch.
iter-6d-f attempt-1: fat-module all-chunks merge -> Tint cliff FAIL.
iter-6d-f-take-2: per-major-arm dispatch -> Tint cliff PASS.
**iter-6d-g**: production vendoring + multi-kernel dispatch + rust_steps short-circuit.
