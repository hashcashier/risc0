# SP6e xgboost bind-group diagnostic counts

Date: 2026-05-18

Purpose: measure the SP6e bind-group diagnostics on the canonical pooled
xgboost workload before designing any bind-group cache. This was a diagnostic
run only: the test was temporarily patched to panic after the proof, receipt
verification, journal decode, and metric log had succeeded.

Command:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=600 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release webgpu_pool_xgboost_smoke
```

Expected result: FAIL, from the intentional post-proof diagnostic panic.

Observed proof metrics before the intentional panic:

```text
browser-prove:metric pool_prove_scheduled_async receipt_kind=Succinct wall_ms=102857 segments=11 keccaks=0 pool_size=2
browser-prove:metric pool_xgboost_smoke wall_ms=102859
```

Diagnostic panic payload:

```text
SP6E_XGBOOST_BIND_GROUP_DIAGNOSTICS wall_ms=102859 bind_group_layout_creations=31 bind_group_layout_cache_hits=1593 bind_group_creations=12510 gpu_dispatches=5365 cpu_only_ops=0
```

Interpretation:

- The default scheduled pool path remains flat for xgboost: 102.859s here,
  matching the SP6d iter 12 measurement of 102.82s within noise.
- Layout caching is already effective on this workload: 31 layout creations
  against 1,593 layout cache hits, about a 98.1% hit rate.
- Bind-group creation volume is much larger than dispatch volume:
  12,510 bind groups for 5,365 GPU dispatches, or about 2.33 bind-group
  creations per dispatch.
- This makes bind-group construction a plausible optimization target, but
  a global bind-group cache remains unsafe unless it distinguishes stable
  long-lived buffers from transient operation buffers and has explicit
  lifetime/invalidation semantics. Otherwise cached `GpuBindGroup` handles
  may retain transient `GpuBuffer`s and undo the deterministic
  `buffer.destroy()` memory fix.

Temporary code status: the diagnostic panic and diagnostic reset were removed
after the run. No permanent test behavior was changed by this measurement.
