# SP7fl Recursion Exec Micro-Ops WOM Scatter Probe

Date: 2026-05-22

## Outcome

Accepted a generated `micro_ops` GPU WOM row scatter/backfill probe. This is test-only substrate for the future GPU-resident recursion `verify_mem` path; it does not change production proving behavior and has accepted wall-time gain `0`.

Generated row substrate coverage now stands at:

- Poseidon2 load/store: `18 / 47` rows covered by GPU scatter/backfill probes.
- Checked bytes: `2 / 47` rows covered by GPU scatter/backfill probes.
- Micro ops: `9 / 47` rows covered by GPU scatter/backfill probes.
- Remaining: macro-op families, `18 / 47` rows.

## Changes

- Added generated pruned WGSL slice: `risc0/circuit/recursion/src/prove/hal/webgpu_step_exec_micro_ops.wgsl`.
  - Source: `/tmp/sp7fa-recursion-wgsl-exec/step_exec.wgsl`.
  - Shape: generated `micro_ops` exec body plus generated `x5` Plonk row writer.
  - Size: `1766` lines, `89782` bytes.
- Added `recursion_exec_micro_ops_wom_scatter_probe_wgsl_module_for_test`.
- Generalized the browser scatter helper so Poseidon2-chain and micro-ops probes use the same GPU scatter/backfill path.
- Added `recursion_exec_micro_ops_wom_scatter_sorts_rows_on_gpu`, which verifies generated `micro_ops` Plonk rows, address-bucket scatter, per-bucket counters, and sorted-row backfill.

## Validation

Focused micro probe:

- Log: `2026-05-22-sp7fl-recursion-exec-micro-ops-wom-scatter-probe.chrome.txt`
- Chrome WebGPU limits: `4294967292 / 2147483644 / 49152`
- Result: `1 passed; 163 filtered out; finished in 1.81s`

Combined WOM scatter probe regression:

- Log: `2026-05-22-sp7fl-recursion-wom-scatter-probes.chrome.txt`
- Covered tests:
  - `recursion_checked_bytes_wom_scatter_sorts_rows_on_gpu`
  - `recursion_exec_micro_ops_wom_scatter_sorts_rows_on_gpu`
  - `recursion_exec_poseidon2_chain_wom_scatter_sorts_rows_on_gpu`
- Chrome WebGPU limits: `4294967292 / 2147483644 / 49152`
- Result: `3 passed; 161 filtered out; finished in 0.37s`

Representative e2e proof generation:

- Log: `2026-05-22-sp7fl-default-representative.chrome.txt`
- Chrome WebGPU limits: `4294967292 / 2147483644 / 49152`
- BusyLoop: `wall_ms=5693`, `gpu_active_ms=4121`, `gpu_idle_ratio=0.276`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- KeccakUnion: `wall_ms=85526`, `gpu_active_ms=63350`, `gpu_idle_ratio=0.259`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- Result: `1 passed; 163 filtered out; finished in 91.97s`

Static checks:

- `cargo fmt --manifest-path examples/browser-prove/Cargo.toml --check`
- `git diff --check`
- Source/log rejected-marker sweep for `recursion_verify_mem`, `verify_mem_post_zeroize`, `set_recursion_verify_mem_gpu`, `WomVerifyPreflight`, `generate_witness_exec_prepare_wom`, `webgpu_step_verify_mem`: clean.

## Performance Interpretation

Accepted wall-time gain is `0`: the new path is not wired into production proving. The representative e2e run is a regression guard only; BusyLoop movement vs SP7fk is noise-positive (`5733 -> 5693 ms`), KeccakUnion movement is noise-negative (`84684 -> 85526 ms`).

The useful performance impact is roadmap narrowing: only `18` generated macro-op WOM rows remain before a full GPU-resident row-generation substrate can feed generated `verify_mem` without the upload-heavy rejected SP7fd/SP7ff shape.
