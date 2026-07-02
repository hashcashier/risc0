# SP7er - recursion WGSL generator gap

Date: 2026-05-22

## Purpose

Follow up on the current highest-value GPU-witgen target from SP7eq: chunk-complete recursion witness/accumulation offload. The question was whether the local Zirgen WGSL branch can already generate recursion WGSL step artifacts for RISC0-side WebGPU wiring.

## Commands

Zirgen checkout:

```text
/home/rami/repos/zirgen
branch: wgsl-gpu-backend
status: clean
```

Build:

```text
bazel build //zirgen/circuit/recursion:recursion_gen
```

Result: succeeded.

Generation attempt:

```text
/home/rami/repos/zirgen/bazel-bin/zirgen/circuit/recursion/recursion_gen \
  --output-dir /tmp/sp7er-recursion-wgsl
```

Result: exit 0.

Generated files:

```text
eval_check.cu
eval_check.cuh
eval_check.h
eval_check.metal
impl.h
info.rs
layout.cpp.inc
layout.cu.inc
layout.rs.inc
poly_edsl.cpp
poly_ext.rs
rust_poly_fp.cpp
rust_step_compute_accum.cpp
rust_step_exec.cpp
rust_step_verify_accum.cpp
rust_step_verify_bytes.cpp
rust_step_verify_mem.cpp
step_compute_accum.cu
step_compute_accum.metal
step_exec.cu
step_verify_accum.cu
step_verify_accum.metal
step_verify_bytes.cu
step_verify_mem.cu
taps.cpp
taps.rs
```

No `.wgsl` files were generated:

```text
find /tmp/sp7er-recursion-wgsl -maxdepth 1 -name '*.wgsl' -printf '%f %s\n'
```

Output was empty.

## Findings

The WGSL path in the local Zirgen checkout is wired through the DSL frontend:

```text
/home/rami/repos/zirgen/zirgen/Main/gen_zirgen.cpp
```

That path clones the step functions, runs WGSL-specific lowering, and calls `emitTarget(WgslCodegenTarget(...), ...)`.

The recursion circuit uses the EDSL generator:

```text
/home/rami/repos/zirgen/zirgen/circuit/recursion/recursion.cpp
```

It calls:

```text
emitCode(module.getModule(), opts);
```

The EDSL default output list also has no WGSL artifacts:

```text
/home/rami/repos/zirgen/bazel/rules/zirgen/edsl-defs.bzl
```

## Interpretation

The immediate blocker for recursion WebGPU witgen/accumulation offload is generator coverage, not RISC0 runtime wiring. RISC0 currently has no generated recursion WGSL artifacts to compile or dispatch.

The existing CUDA/Metal recursion generated outputs are the right shape for the target behavior:

- `step_compute_accum.cu` / `.metal`
- `step_verify_accum.cu` / `.metal`
- `step_exec.cu`

But the current WGSL branch does not produce equivalent EDSL recursion WGSL files.

## Correctness risk

This is diagnostic-only. No runtime code was changed and no performance gain is claimed.

Before any generated WGSL recursion path can be trusted for proof generation, the WGSL backend also needs the semantic TODOs resolved for the used operations and externs. The most visible current hazards are in:

```text
/home/rami/repos/zirgen/zirgen/compiler/codegen/gpu/witgen_prelude.wgsl
```

Notable examples:

- `ext_inv` placeholder currently returns its input.
- checked-read / `valid_or_zero` behavior is marked TODO.
- several externs are stubbed to zero.

## Next action

The fastest credible path toward the requested GPU witgen offload is to extend the Zirgen EDSL recursion generation path to emit WGSL artifacts for recursion, starting with accumulator pieces:

- `step_compute_accum.wgsl`
- `step_verify_accum.wgsl`
- required layout/type/prelude includes

Only after those artifacts exist should RISC0-side WebGPU runtime wiring begin.

Accepted wall-time gain: 0.
