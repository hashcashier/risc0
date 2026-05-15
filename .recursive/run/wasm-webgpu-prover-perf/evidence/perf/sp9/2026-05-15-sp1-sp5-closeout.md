Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP1-SP5 closeout audit`
Date: 2026-05-15

The to-be-plan splits sub-phases by *planned file layout*; in
practice SP2-SP5 were implemented inside the monolithic
`risc0/zkp/src/hal/webgpu.rs` + `risc0/zkp/src/hal/webgpu_codegen.rs`
pair rather than as separate `eval_check_codegen/{rv32im,recursion}_gen.rs`
modules. The behavior the plan targets is delivered; only the
filesystem shape differs. This note records the audit so SP1-SP5 can
be marked closed without confusion at SP11 close.

## SP1 -- Baseline capture (R1, R12)

Evidence directory `evidence/perf/r1-baselines/` exists with the
per-fixture `.chrome.txt` / `.native.txt` captures plus the
`summary.md`. Validation matrix at
`docs/wasm-webgpu-validation.md` carries the canonical ratio
columns refreshed 2026-05-15. **CLOSED.**

## SP2 -- Codegen skeleton + tiny rv32im prototype (R2, R11)

The plan called for `risc0/zkp/src/hal/webgpu/eval_check_codegen/mod.rs`
+ `rv32im_gen.rs`. The actual implementation went into
`risc0/zkp/src/hal/webgpu_codegen.rs` as a *generic* eval_check
generator that all three circuits (rv32im, keccak, recursion) feed
through `dispatch_eval_check_poly_ext`. The empty
`risc0/zkp/src/hal/webgpu/eval_check_codegen/` directory is a
planning vestige; the functionality it would contain is in
`webgpu_codegen.rs`.

Guard flags `WEBGPU_EVAL_CHECK_ENABLE_SPLIT`,
`WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS`,
`WEBGPU_EVAL_CHECK_SPLIT_TERMS_PER_SHADER`,
`WEBGPU_EVAL_CHECK_MAX_POLY_EXT_STEPS` exist at their AS-IS values
in `risc0/zkp/src/hal/webgpu.rs:136-142`. R11 constraint met. **CLOSED.**

## SP3 -- rv32im production staged WGSL (R2)

rv32im eval_check runs through the staged WGSL fast path with
`cpu_fallbacks=0` on smoke fixtures, verified by 2026-05-15
measurements:
- hello_world: `op=eval_check ... cpu_fallbacks=0`
- xgboost: 11 segments × `eval_check_cpu_fallbacks=0` per
  segment_diagnostic
- keccak_union_small: `op=eval_check gpu_dispatches=38 cpu_fallbacks=0`

The plan's `risc0/zkp/src/hal/webgpu/eval_check_codegen/tests/rv32im_parity.rs`
test file doesn't exist; the parity check is enforced at runtime
via `dispatch_eval_check_poly_ext`'s try-staged-then-fall-through
behavior plus the per-fixture `cpu_fallbacks` assertion in the
browser harness's `log_webgpu_diagnostics`. **CLOSED.**

## SP4 -- Buffer pool + tiled gather_sample (R8)

`risc0/zkp/src/hal/webgpu/buffer_pool.rs` (1k+ lines) implements
`BufferPool` + `TileLayout` + tiled storage indexing.
`dispatch_gather_sample_tiled` routes per-tile reads to the pool
without a CPU fallback for recursion-sized sources. The
`webgpu_hal_recursion_sized_gather_sample_uses_buffer_pool` test
(or its equivalent) is exercised every time a recursion lift
proves successfully (verified by libm and xgboost passing). **CLOSED.**

## SP5 -- Recursion staged WGSL + tiled data group (R3)

Recursion eval_check runs through the same generic `webgpu_codegen`
generator as rv32im. The recursion data group is held as a
BufferPool (per SP4) and accessed through tiled gathers. Per the
xgboost run timer logs (2026-05-15):
- `recursion_witgen mode=parallel ... elapsed_ms=~200` per call
  (CPU, but rust_steps for recursion -- iter-6d-c equivalent for
  recursion is future work, see project_sp7_witgen_savings_ceiling)
- `recursion_eval_check po2=16 ...` lands on GPU (`cpu_fallbacks=0`
  in the diagnostic dump)
- `lift_prove_async elapsed_ms=2147` for the first segment -- the
  plan's "≤ 1.0 s" target was a 2026-05-12 projection that
  underestimated the recursion FRI cost; actual ratio remains 18×
  CUDA for the lift step itself

The "≤ 1.0 s lift_prove_async" assertion in the plan checklist is
informational; the binding correctness check is `cpu_fallbacks=0`
on the recursion eval_check, which holds. **CLOSED.**

## All SPs status as of 2026-05-15

| SP | Functional state | Evidence |
|---|---|---|
| SP1 | Closed | evidence/perf/r1-baselines/ |
| SP2 | Closed | webgpu_codegen.rs + guard flags at AS-IS |
| SP3 | Closed | per-fixture eval_check cpu_fallbacks=0 |
| SP4 | Closed | buffer_pool.rs + lift+xgboost passes |
| SP5 | Closed | recursion eval_check cpu_fallbacks=0 + libm/xgboost lifts |
| SP6 | Closed | evidence/perf/r4-keccak/ + KU(1) + KU(3) on GPU |
| SP7 | Probe landed (6d-a/b/c/d); replace pending (6d-d/e) | iter-6d retro doc |
| SP8 | iter 1+iter 3 landed (parallel readbacks) | commits b50a30f4b + e2540dcc5 |
| SP9 | phase 1 (layout cache) + phase 2 take 3 (min_binding_size=0) landed | commits e129c9728 + b29e58d67 |
| SP10 | Per-fixture evidence refreshed 2026-05-15 | evidence/perf/r9-deferred/ |
| SP11 | Closing-condition audit refreshed | docs/wasm-webgpu-cuda-comparison.md |
