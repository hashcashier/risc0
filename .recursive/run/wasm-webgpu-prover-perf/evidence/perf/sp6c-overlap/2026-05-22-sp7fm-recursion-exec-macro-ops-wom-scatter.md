# SP7fm recursion exec macro_ops WOM scatter probe

Status: accepted test-only substrate. Accepted wall-time gain: 0.

## What changed

- Added `risc0/circuit/recursion/src/prove/hal/webgpu_step_exec_macro_ops.wgsl`, a pruned generated recursion `step_exec` slice for the macro-op families. Current size: 4,227 lines, 301,536 bytes.
- Exposed `recursion_exec_macro_ops_wom_scatter_probe_wgsl_module_for_test()` through the WebGPU recursion HAL and recursion prove module.
- Added browser test `recursion_exec_macro_ops_wom_scatter_sorts_rows_on_gpu`.
- The macro probe drives all generated macro selectors once and checks the 18 expected generated Plonk rows: `3 + 3 + 2 + 2 + 2 + 2 + 4`.
- The test verifies unsorted GPU row emission, decoded-address bucket scatter, atomic counters, sorted rows, and GPU backfill semantics. The expected sorted shape is 3 zero rows followed by addresses `1..=15`.
- The shared scatter helper allocates the test `global` buffer at `RECURSION_WOM_PROBE_DATA_COLS` words because macro `set_global` writes `out1` columns beyond index 0.

## RED evidence

The first focused Chrome run failed before row execution because the macro WGSL slice was missing the wrapper function:

- Log: `2026-05-22-sp7fm-recursion-exec-macro-ops-wom-scatter-probe.chrome.txt`
- Failure: Chrome WGSL parser rejected top-level `let x429`.
- Fix: wrap the generated slice in `fn recursion(code0, out1, data2, mix3, accum4) -> Val { ... }`, matching the already accepted Poseidon2 and micro-op slices.

## Validation

Focused macro probe:

- Command: `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release recursion_exec_macro_ops_wom_scatter_sorts_rows_on_gpu -- --nocapture`
- Log: `2026-05-22-sp7fm-recursion-exec-macro-ops-wom-scatter-probe-rerun.chrome.txt`
- WebGPU limits: `4294967292 / 2147483644 / 49152`
- Result: `1 passed; 0 failed; finished in 32.62s`

Combined generated row scatter probes:

- Command: `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release wom_scatter_sorts_rows_on_gpu -- --nocapture`
- Log: `2026-05-22-sp7fm-recursion-wom-scatter-probes.chrome.txt`
- Covered: checked_bytes, macro_ops, micro_ops, Poseidon2 chain
- Result: `4 passed; 0 failed; finished in 0.61s`

Representative e2e proof generation:

- Log: `2026-05-22-sp7fm-default-representative.chrome.txt`
- BusyLoop: `wall_ms=5655`, `gpu_active_ms=4045`, `gpu_idle_ratio=0.285`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- KeccakUnion: `wall_ms=85171`, `gpu_active_ms=63044`, `gpu_idle_ratio=0.260`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- Result: `rv32im_default_representative_e2e_verify` passed; runtime `91.56s`

xgboost e2e proof generation:

- Log: `2026-05-22-sp7fm-xgboost.chrome.txt`
- xgboost: `wall_ms=64728`, `gpu_active_ms=48309`, `gpu_idle_ratio=0.254`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- Result: `xgboost_succinct_receipt_verifies` passed; runtime `65.27s`

Static checks:

- `cargo fmt --manifest-path examples/browser-prove/Cargo.toml --check` passed.
- `git diff --check` passed.
- Rejected `recursion_verify_mem` marker sweep over recursion sources, browser tests, and SP7fm logs returned no matches.

## Interpretation

The generated row substrate now covers all generated WOM rows consumed by generated `verify_mem`:

- Poseidon2 chain: 18 rows
- checked_bytes: 2 rows
- micro_ops: 9 rows
- macro_ops: 18 rows
- Total: 47 / 47 rows covered in GPU scatter/backfill probes

This is still not production runtime wiring. The next performance-relevant step is a generated `verify_mem` retry that keeps exec Plonk rows GPU-resident and avoids the SP7fd/SP7ff upload-heavy sorted-row shape.
