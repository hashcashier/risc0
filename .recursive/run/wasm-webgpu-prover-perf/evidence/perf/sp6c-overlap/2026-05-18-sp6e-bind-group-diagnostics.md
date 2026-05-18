# SP6e - bind-group diagnostics surface

Date: 2026-05-18
Baseline commit before change: `98ccb8560`

## Purpose

SP6e remains the next low-risk post-scheduler lever, but the naive global bind-group cache is dangerous in this codebase: a cached `GpuBindGroup` can retain references to transient `GpuBuffer`s and undo the SP-CR D16 deterministic `GpuBuffer.destroy()` fix that made multi-segment xgboost reliable.

This step adds the missing diagnostics surface before attempting any bind-group reuse:

- bind-group-layout creations
- bind-group-layout cache hits
- bind-group creations

The counters make future SP6e probes measurable at the HAL boundary and let browser acceptance helpers print the same data as other WebGPU backend counters.

## TDD evidence

### RED

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_reports_layout_and_bind_group_diagnostics --no-run
```

Expected failure: the new browser test references diagnostics fields before they exist.

Observed failure:

- `error[E0609]: no field bind_group_layout_creations on type WebGpuDiagnostics`
- `error[E0609]: no field bind_group_layout_cache_hits on type WebGpuDiagnostics`
- `error[E0609]: no field bind_group_creations on type WebGpuDiagnostics`

RED verified: PASS.

### GREEN

Implemented:

- `WebGpuDiagnostics::{bind_group_layout_creations, bind_group_layout_cache_hits, bind_group_creations}`.
- Matching `WebGpuDiagnosticsState` counters, reset behavior, and record helpers.
- Counter updates in `WebGpuHal::create_bind_group_layout` and `WebGpuHal::create_bind_group`.
- Pool aggregation in `WebGpuProverPool::diagnostics`.
- Browser diagnostic log strings now include the new counters.

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_reports_layout_and_bind_group_diagnostics --no-run
```

Result:

- Finished release profile.
- Produced `examples/target/wasm32-unknown-unknown/release/deps/browser_prove-8b5b728d1fc35820.wasm`.

GREEN compile verified: PASS.

## Browser verification

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_reports_layout_and_bind_group_diagnostics
```

Result:

```text
running 1 test
test tests::webgpu_hal_reports_layout_and_bind_group_diagnostics ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 109 filtered out; finished in 0.11s
```

The test constructs the same layout twice and creates two bind groups against the same buffer. It verifies:

- `bind_group_layout_creations == 1`
- `bind_group_layout_cache_hits == 1`
- `bind_group_creations == 2`

Additional aggregation compile guard:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release webgpu_prover_pool_convenience_uses_scheduled_path_smoke --no-run
```

Result: PASS.

## Interpretation

This is instrumentation, not a speedup. It intentionally stops short of caching `GpuBindGroup` globally because retaining bind groups can retain transient buffers and increase GPU memory pressure. The next SP6e step should use these counters on real smokes to identify stable, long-lived binding sets before introducing any cache.
