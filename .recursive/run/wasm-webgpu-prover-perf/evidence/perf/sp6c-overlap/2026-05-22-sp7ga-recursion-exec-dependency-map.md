# SP7ga Recursion Exec Dependency Map

Date: 2026-05-22

## Purpose

After SP7fr, the accepted browser WebGPU recursion witness path is CPU `step_exec` plus GPU-resident WOM row generation/scatter/backfill/`verify_mem`. SP7fs-SP7fw proved that blindly eliding `recursion_data` CPU-shadow upload breaks proof correctness.

This checkpoint maps the generated recursion exec chunks against the current GPU chunks so the next witgen offload attempt has a proof-shaped target instead of repeating upload-elision guesses.

No production runtime code changed. Accepted wall-time gain: 0. Current accepted working state remains SP7fr.

## Current Accepted State

Evidence:

- `2026-05-22-sp7fr-recursion-witgen-submit-batched.md`
- `2026-05-22-sp7fr-default-representative-recursion-witgen-submit-batched.chrome.txt`
- `2026-05-22-sp7fr-default-xgboost-recursion-witgen-submit-batched.chrome.txt`
- `2026-05-22-sp7fy-current-drain-attribution.md`

SP7fr representative gate:

- BusyLoop: `wall_ms=5962`, `gpu_active_ms=4484`, `raw_compute_dispatches=613`, `queue_submits=168`
- KeccakUnion: `wall_ms=81813`, `gpu_active_ms=62499`, `raw_compute_dispatches=10900`, `queue_submits=3153`
- Test runtime: `88.54s`
- `cpu_fallbacks=0`, `cpu_only_ops=0`

SP7fr xgboost gate:

- xgboost: `wall_ms=61883`, `gpu_active_ms=47718`, `raw_compute_dispatches=9918`, `queue_submits=2803`
- Test runtime: `62.45s`
- `cpu_fallbacks=0`, `cpu_only_ops=0`

## Measured Recursion Ceiling From SP7fr

Local log aggregation from the SP7fr browser proof logs:

```text
SP7fr representative:
  recursion_witgen CPU exec/plan calls: n=26, sum=4072 ms, mean=156.6 ms, min/max=99/193
  gpu_verify_mem sequence calls:        n=26, sum=2 ms
  commit_group_async recursion_data:    n=26, sum=1687 ms, mean=64.9 ms
  recursion_data sparse zeroize upload: n=26, sum=1180461456 bytes

SP7fr xgboost:
  recursion_witgen CPU exec/plan calls: n=21, sum=3160 ms, mean=150.5 ms, min/max=105/203
  gpu_verify_mem sequence calls:        n=21, sum=3 ms
  commit_group_async recursion_data:    n=21, sum=1397 ms, mean=66.5 ms
  recursion_data sparse zeroize upload: n=21, sum=885025956 bytes
```

Interpretation:

- The remaining recursion CPU exec ceiling is real but bounded: about `3.2s` on xgboost and `4.1s` across BusyLoop+KeccakUnion.
- The larger recursion-side data movement is the sparse `recursion_data` zeroize/upload path, but SP7fv/SP7fw proved it cannot be skipped from CPU write tracking or simple generated store/read masks.
- A correct offload must make GPU-generated cells authoritative, not just mark CPU-shadow cells invalid.

## Static Exec Coverage

Source files inspected:

- Full generated recursion exec spike: `/tmp/sp7fa-recursion-wgsl-exec/step_exec.wgsl`
- Vendored GPU chunks:
  - `risc0/circuit/recursion/src/prove/hal/webgpu_step_exec_micro_ops.wgsl`
  - `risc0/circuit/recursion/src/prove/hal/webgpu_step_exec_macro_ops.wgsl`
  - `risc0/circuit/recursion/src/prove/hal/webgpu_step_exec_poseidon2_chain.wgsl`

Static selector summary from the full generated exec:

| Selector | Stores | Loads | Externs | Blocking dependency |
|---|---:|---:|---|---|
| `micro_ops` | `data2`: 579 stores / 81 unique cols | `data2` backs `[0,1]`, 75 unique cols | `womRead=45`, `womWrite=36`, `readIOPHeader=3`, `readIOPBody=3` | previous-row state plus IOP/WOM effects |
| `macro_ops` | `data2`: 742 stores / 123 unique cols; `out1`: 32 stores | `data2` backs `[0,1,2,3,4,7,15,16,68]`, 117 unique cols | `womRead=25`, `womWrite=5` | long backward dependencies up to 68 rows |
| `poseidon2_load` | `data2`: 227 stores / 96 unique cols | `data2` backs `[0,1]`, 96 unique cols | `womRead=8` | previous-row Poseidon state |
| `poseidon2_store` | `data2`: 64 stores / 64 unique cols | `data2` backs `[0,1]`, 64 unique cols | `womWrite=8` | previous-row Poseidon state and WOM writes |
| `checked_bytes` | `data2`: 156 stores / 110 unique cols | `data2` backs `[0,1]`, 115 unique cols | `womRead=1`, `readCoefficients=1`, `womWrite=1`, `plonkWrite=47` | coefficient/byte-read side input plus previous-row state |

Vendored chunk coverage:

| Vendored chunk | Covered selectors | Present role |
|---|---|---|
| `webgpu_step_exec_micro_ops.wgsl` | `micro_ops` | Produces GPU WOM/Plonk rows and data writes after CPU exec/zeroize |
| `webgpu_step_exec_macro_ops.wgsl` | `macro_ops` | Produces GPU WOM/Plonk rows, global writes, and data writes after CPU exec/zeroize |
| `webgpu_step_exec_poseidon2_chain.wgsl` | `poseidon2_load` + `poseidon2_store` chain | Produces GPU WOM/Plonk rows and Poseidon data writes after CPU exec/zeroize |
| manual checked-bytes candidate WGSL in `webgpu.rs` | checked-bytes WOM rows only | Does not vendor the full generated checked-bytes exec slice |

The generated checked-bytes slice exists in `/tmp/sp7fa-recursion-wgsl-exec/step_exec.wgsl` around lines 8112-8948, but the production candidate currently uses manual checked-bytes WOM-row WGSL only.

## Why CPU Exec Cannot Be Removed Yet

Current production flow:

1. `CircuitWitnessGenerator<WebGpuHal>::generate_witness` calls `generate_witness_exec_plan`.
2. `generate_witness_exec_plan` still runs CPU `ctx.do_step_exec(...)` and builds a `WomGpuVerifyPlan`.
3. `WitnessGenerator::new` adds noise, then zeroizes `data` and `global`.
4. `post_witness_zeroize` dispatches the GPU row/scatter/backfill/`verify_mem` sequence.

Therefore the GPU chunks are already valuable for `verify_mem`, but they are not yet an authoritative replacement for CPU `step_exec`.

The hard blockers are:

- `macro_ops` depends on prior rows up to `back=68`; naive one-pass parallel dispatch is not semantically equivalent to CPU exec.
- `micro_ops`, Poseidon2, and checked-bytes all have `back=1` dependencies that require ordered state propagation or staged wavefront execution.
- checked-bytes needs the generated `extern_readCoefficients()` side input before the full generated slice can replace CPU work.
- SP7fv/SP7fw demonstrated that even cells not caught by simple previous-row read masks can be required by proof verification.

## Next Correctness Gate

Do not accept another runtime witgen offload unless it has all of the following:

- A generated dependency/validity map for every `data`/`global` column the GPU claims authoritative.
- A GPU-produced authoritative-validity mask or equivalent proof that post-zeroize GPU writes cover the committed values.
- A focused parity guard proving GPU chunk output equals CPU `step_exec` output for the selected selector family on proof-shaped buffers.
- Full browser proof generation with receipt verification for BusyLoop + KeccakUnion, then xgboost.
- Zero `cpu_fallbacks` and zero `cpu_only_ops`.

## Decision

The next recursion-witgen implementation step should not be upload elision. It should be a dependency-aware GPU exec substrate:

1. vendor or generate the full checked-bytes exec slice, including coefficients input;
2. add a test-only CPU/GPU parity harness for one selector family;
3. only then attempt a small runtime replacement of a selector family behind a default-off gate;
4. promote only after representative e2e proof generation proves correctness and wall benefit.

Expected gain if fully successful from the current accepted state: `3-6%` on xgboost/KeccakUnion, lower on BusyLoop. The larger near-term wall-time lever remains FRI/check expansion NTT (`8-15%`) per SP7fy.
