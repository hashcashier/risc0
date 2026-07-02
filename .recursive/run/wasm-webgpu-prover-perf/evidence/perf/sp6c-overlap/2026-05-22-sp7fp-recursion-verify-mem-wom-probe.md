# SP7fp Recursion Verify-Mem WOM Probe

Date: 2026-05-22

Status: accepted correctness substrate; accepted wall-time gain is 0.

## Scope

Added a focused browser/WebGPU probe proving that generated recursion `verify_mem`
WGSL can consume GPU-resident sorted WOM rows through a real `extern_plonkRead`
implementation and write the expected data columns. This keeps the next
production candidate pointed at GPU-resident rows rather than the rejected
SP7fd/SP7ff host-upload shapes.

Changed files:

- `risc0/circuit/recursion/src/prove/hal/webgpu_step_verify_mem.wgsl`
- `risc0/circuit/recursion/src/prove/hal/webgpu.rs`
- `risc0/circuit/recursion/src/prove/mod.rs`
- `examples/browser-prove/src/lib.rs`

## TDD

RED:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release recursion_verify_mem_reads_sorted_rows_on_gpu -- --nocapture
```

Failed as expected with:

```text
cannot find function `recursion_verify_mem_wom_probe_wgsl_module_for_test` in module `risc0_circuit_recursion::prove`
```

GREEN implementation:

- Copied the generated SP7fc `step_verify_mem.wgsl` into the recursion WebGPU
  HAL source tree as `webgpu_step_verify_mem.wgsl`.
- Added `recursion_verify_mem_wom_probe_wgsl_module_for_test`, assembling the
  generated shader with a test-only `extern_plonkRead` backed by sorted-row,
  per-cycle cursor, and cycle-prefix storage buffers.
- Added browser test `recursion_verify_mem_reads_sorted_rows_on_gpu`.

GREEN compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release --no-run recursion_verify_mem_reads_sorted_rows_on_gpu
```

Passed in `3m 03s`.

First Chrome run failed with a useful WebGPU layout bug:

```text
GPUValidationError: The buffer type in the shader (ReadOnlyStorage) is not compatible with the type in the layout (Storage)
```

Repair: changed sorted rows and cycle prefixes to
`WebGpuBindingLayout::read_only_storage`.

Focused Chrome/WebGPU rerun:

```text
test tests::recursion_verify_mem_reads_sorted_rows_on_gpu ... ok
test result: ok. 1 passed; 0 failed; 165 filtered out; finished in 0.94s
```

Evidence:

- `2026-05-22-sp7fp-recursion-verify-mem-wom-probe-rerun.chrome.txt`

## Representative E2E Proof Gates

BusyLoop + KeccakUnion representative:

```text
test tests::rv32im_default_representative_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 165 filtered out; finished in 91.32s
```

Metrics:

- BusyLoop: `wall_ms=5689`, `gpu_active_ms=4131`, `gpu_idle_ratio=0.274`,
  `cpu_fallbacks=0`, `cpu_only_ops=0`
- KeccakUnion: `wall_ms=84903`, `gpu_active_ms=62774`,
  `gpu_idle_ratio=0.261`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- Hook metrics: `26`
- WOM sort summary: `calls=26`, `rows=38678651`, `addr_groups=12283601`,
  `repeated_addr_groups=11558970`, `distinct_value_groups=0`,
  `distinct_value_rows=0`, `max_addr_group=317993`

Evidence:

- `2026-05-22-sp7fp-default-representative.chrome.txt`

xgboost:

```text
test tests::xgboost_succinct_receipt_verifies ... ok
test result: ok. 1 passed; 0 failed; 165 filtered out; finished in 65.67s
```

Metrics:

- xgboost: `wall_ms=65070`, `gpu_active_ms=48601`,
  `gpu_idle_ratio=0.253`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- Hook metrics: `21`
- WOM sort summary: `calls=21`, `rows=28520256`, `addr_groups=9128218`,
  `repeated_addr_groups=8487866`, `distinct_value_groups=0`,
  `distinct_value_rows=0`, `max_addr_group=317959`

Evidence:

- `2026-05-22-sp7fp-xgboost.chrome.txt`

## Static Checks

Passed:

```text
cargo fmt --manifest-path examples/browser-prove/Cargo.toml --check
git diff --check
rejected-marker sweep over recursion prove and browser-prove code paths
```

The marker sweep exited with no matches.

## Decision

Keep SP7fp. It proves generated recursion `verify_mem` can read sorted rows
directly from GPU buffers and mutate the data buffer correctly. It does not
change production proving behavior and should not be counted as a performance
gain. The next production candidate should use this module only with
GPU-resident row generation/scatter/backfill, avoiding dense `recursion_data`,
sorted-row, and offset uploads from SP7fd/SP7ff.
