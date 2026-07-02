# SP7es - recursion accumulator WGSL generation

Date: 2026-05-22

## Purpose

Continue the SP7eq/SP7er GPU-witgen path by unblocking generated WGSL artifacts for recursion accumulation. SP7er proved that the local Zirgen EDSL recursion generator did not emit WGSL. This spike adds the smallest EDSL WGSL emission path needed for recursion accumulator stages and validates the generated WGSL with a real parser.

## Zirgen patch scope

External checkout:

```text
/home/rami/repos/zirgen
branch: wgsl-gpu-backend
```

Modified files:

```text
bazel/rules/zirgen/edsl-defs.bzl
zirgen/compiler/codegen/BUILD.bazel
zirgen/compiler/codegen/codegen.cpp
zirgen/compiler/codegen/gpu/witgen_prelude.wgsl
```

Summary:

- Add EDSL default outputs:
  - `step_compute_accum.wgsl`
  - `step_verify_accum.wgsl`
  - `layout.wgsl.inc`
- Emit WGSL layout from the EDSL `emitCode` path.
- Emit WGSL accumulator-stage functions after split-stage lowering plus WGSL unroll/CSE/canonicalize/SymbolDCE passes.
- Add WGSL syntax lowering for base Zll ops used by recursion accumulator:
  - `neg`
  - `get`
  - `get_global`
  - `set`
  - `set_global`
- Add WGSL prelude definitions for the accumulator Plonk extern names:
  - `extern_plonkWriteAccum`
  - `extern_plonkReadAccum`

## Build verification

```text
bazel build //zirgen/circuit/recursion:recursion_gen
```

Result: passed.

```text
bazel build //zirgen/circuit/recursion:recursion
```

Result: passed and declared the new outputs:

```text
bazel-bin/zirgen/circuit/recursion/step_compute_accum.wgsl
bazel-bin/zirgen/circuit/recursion/step_verify_accum.wgsl
bazel-bin/zirgen/circuit/recursion/layout.wgsl.inc
```

Generated sizes:

```text
   607   36358 layout.wgsl.inc
  3081  266817 step_compute_accum.wgsl
  1710  131542 step_verify_accum.wgsl
  5398  434717 total
```

## WGSL validation

Combined modules were validated with `naga`:

```text
cat witgen_prelude.wgsl layout.wgsl.inc step_compute_accum.wgsl \
  > /tmp/sp7es-bazel-compute-accum-combined.wgsl
naga /tmp/sp7es-bazel-compute-accum-combined.wgsl \
  /tmp/sp7es-bazel-compute-accum.spv
```

Result: passed.

```text
cat witgen_prelude.wgsl layout.wgsl.inc step_verify_accum.wgsl \
  > /tmp/sp7es-bazel-verify-accum-combined.wgsl
naga /tmp/sp7es-bazel-verify-accum-combined.wgsl \
  /tmp/sp7es-bazel-verify-accum.spv
```

Result: passed.

Additional hygiene:

```text
git -C /home/rami/repos/zirgen diff --check
```

Result: passed.

## Interpretation

This converts the SP7er blocker from "recursion EDSL produces no WGSL artifacts" to "RISC0 can now consume syntactically valid generated recursion accumulator WGSL, but runtime wiring and proof-level correctness remain open."

This does not yet claim a RISC0 proving-time gain. No RISC0 runtime path has dispatched these shaders, and no e2e proof has been generated with them.

## Correctness risks before runtime use

The current WGSL Plonk accumulator externs are sufficient for parser validation, but RISC0 runtime wiring must preserve the native accumulation dataflow:

- `step_compute_accum` writes per-cycle WOM accumulator products.
- GPU prefix products run over that WOM buffer.
- `step_verify_accum` reads WOM prefix products and writes final accumulator columns.

The WebGPU implementation should therefore use an explicit WOM/prefix buffer like the native Metal path, rather than assuming final `accum` and WOM can safely alias.

The WGSL prelude still contains broader TODOs that matter before witness-generation offload, especially checked-read / `valid_or_zero` semantics and non-accumulator externs. This SP only advances accumulator artifacts.

## Next action

Wire a RISC0-side opt-in WebGPU recursion accumulator path:

1. Include/generated-load the new recursion accumulator WGSL artifacts.
2. Compile `step_compute_accum` and `step_verify_accum` with `witgen_prelude.wgsl` and `layout.wgsl.inc`.
3. Allocate a GPU WOM buffer with 4 extension-field columns, initialized to one.
4. Dispatch compute-accum over `steps - ZK_CYCLES`.
5. Run the existing WebGPU prefix-products implementation over WOM.
6. Dispatch verify-accum into the final accumulator buffer.
7. Gate with focused CPU/GPU parity first, then BusyLoop + KeccakUnion + xgboost e2e proof generation before any performance claim.

Accepted wall-time gain: 0.
