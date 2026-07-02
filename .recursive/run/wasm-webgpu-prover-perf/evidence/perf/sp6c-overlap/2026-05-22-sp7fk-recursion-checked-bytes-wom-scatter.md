# SP7fk recursion checked-bytes WOM scatter probe

Date: 2026-05-22

## Summary

Accepted a browser GPU-layout probe for the 2-row recursion `checked_bytes` WOM family and fixed WebGPU adapter selection so representative performance gates request the high-performance adapter.

The checked-bytes probe is test-only substrate. The production proving path is unchanged except for adapter selection robustness, so accepted wall-time gain is 0.

## Changed files

- `risc0/circuit/recursion/src/prove/hal/webgpu.rs`
  - Added `recursion_checked_bytes_wom_scatter_probe_wgsl_module_for_test()`.
  - Added a test-only WGSL probe that emits checked-bytes WOM rows, scatters them by decoded address bucket, and backfills data columns from prior sorted rows.
- `risc0/circuit/recursion/src/prove/mod.rs`
  - Exposed the checked-bytes probe module helper for browser tests.
- `examples/browser-prove/src/lib.rs`
  - Added `recursion_checked_bytes_wom_scatter_sorts_rows_on_gpu`.
  - The test verifies unsorted emission, bucket counters, sorted address order, and backfill semantics.
- `risc0/zkp/Cargo.toml`
  - Added the `GpuPowerPreference` `web-sys` feature.
- `risc0/zkp/src/hal/webgpu.rs`
  - `request_adapter()` now uses `GpuRequestAdapterOptions` with `HighPerformance`.
  - Fallback requests still set `force_fallback_adapter(true)`.

## RED

Two representative proof attempts were rejected before proof generation because Chrome selected a low-limit adapter:

- `2026-05-22-sp7fk-default-representative-low-limit-rejected.chrome.txt`
- `2026-05-22-sp7fk-default-representative-low-limit-rejected-2.chrome.txt`

Both logged:

```text
max_buffer_size=1073741824
max_storage_buffer_binding_size=1073741824
max_compute_workgroup_storage_size=32768
```

The representative gate correctly panicked with `representative performance proof gates require high WebGPU limits`.

## Validation

Static checks:

```text
cargo fmt --manifest-path examples/browser-prove/Cargo.toml --check
git diff --check
```

Rejected-marker sweep:

```text
rg -n "recursion_verify_mem|verify_mem_post_zeroize|set_recursion_verify_mem_gpu|WomVerifyPreflight|generate_witness_exec_prepare_wom|webgpu_step_verify_mem" \
  risc0/circuit/recursion/src/prove \
  examples/browser-prove/src/lib.rs \
  2026-05-22-sp7fk-recursion-checked-bytes-wom-scatter-probe.chrome.txt \
  2026-05-22-sp7fk-default-representative.chrome.txt \
  2026-05-22-sp7fk-xgboost.chrome.txt
```

Result: no output.

Focused checked-bytes browser probe:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release recursion_checked_bytes_wom_scatter_sorts_rows_on_gpu -- --nocapture
```

Evidence: `2026-05-22-sp7fk-recursion-checked-bytes-wom-scatter-probe.chrome.txt`

Result:

- High WebGPU limits: `4294967292 / 2147483644 / 49152`.
- `test tests::recursion_checked_bytes_wom_scatter_sorts_rows_on_gpu ... ok`
- Test runtime: `0.10s`.

Representative e2e proof gate:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture
```

Evidence: `2026-05-22-sp7fk-default-representative.chrome.txt`

Result:

- High WebGPU limits: `4294967292 / 2147483644 / 49152`.
- BusyLoop: `wall_ms=5733`, `gpu_active_ms=4167`, `gpu_idle_ratio=0.273`.
- KeccakUnion: `wall_ms=84684`, `gpu_active_ms=62426`, `gpu_idle_ratio=0.263`.
- Test runtime: `91.17s`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.

xgboost e2e proof gate:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture
```

Evidence: `2026-05-22-sp7fk-xgboost.chrome.txt`

Result:

- High WebGPU limits: `4294967292 / 2147483644 / 49152`.
- xgboost: `wall_ms=64138`, `gpu_active_ms=47690`, `gpu_idle_ratio=0.256`.
- Test runtime: `64.69s`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.

## Interpretation

SP7fk proves the checked-bytes row family can use the same GPU-resident row scatter/backfill shape as Poseidon2, and it removes a recurring source of bad performance decisions by forcing the high-performance adapter path for normal WebGPU requests.

The remaining generated recursion WOM rows are:

- Test-substrate covered: Poseidon2 `18`, checked-bytes `2`.
- Still not covered by GPU-resident row-generation substrate: `27` non-Poseidon2 rows.
- Still not production-wired: all `29` non-Poseidon2 rows, including checked-bytes.

Next production-relevant step: add GPU-resident scatter/backfill probes for the macro-op row families or `micro_ops`, then reintroduce generated `verify_mem` only when all 47 rows can stay GPU-resident.
