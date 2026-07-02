Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7f -- TopAccum arm5 selector validation + representative e2e coverage
Date: 2026-05-18

## Headline

The TopAccum arm5 scratch probe now records the sampled preflight major, data
buffer major, selector value, mismatch count, and first mismatching accum
column. The focused browser e2e coverage now runs both `BusyLoop` and
`KeccakUnion(1)` sequentially in one wasm test so the global opt-in probe state
cannot race between independent async wasm tests.

This is still a diagnostic/probe step, not a wall-time speedup. Authoritative
replacement remains blocked.

## TDD / Correctness Evidence

RED compile for the richer summary API:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=180 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_real_buffer_probe_e2e_verify --no-run

error[E0432]: unresolved import `risc0_circuit_rv32im::prove::accum_gpu_arm5_probe_summary`
```

GREEN compile after adding `TopAccumArm5ProbeSummary` and
`accum_gpu_arm5_probe_summary()`:

```text
Finished `release` profile [optimized + debuginfo] target(s) in 2m 10s
Executable unittests src/lib.rs (.../browser_prove-8b5b728d1fc35820.wasm)
```

The stricter semantic assertion (`mismatch_count == 0`) was intentionally used
as a RED diagnostic and failed on the real browser proof path:

```text
browser-prove:metric topaccum_arm5_probe sample_cycle=18334 preflight_major=5 selector_value=1 mismatch_count=40 first_mismatch_col=23
assertion `left == right` failed: generated TopAccum arm5 row must match the authoritative accum row
left: 40
right: 0
```

Interpretation: the sampled row and selector are correct, but generated arm5
semantics still disagree with CPU `step_TopAccum`. The representative probe
test therefore logs the mismatch and asserts only e2e receipt correctness plus
sample-row sanity until the arm5 body is fixed.

## Representative Browser E2E

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_representative_probe_e2e_verify -- --nocapture
```

Result:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 117 filtered out; finished in 111.72s
```

BusyLoop proof:

```text
browser-prove:metric prove_session_async wall_ms=8626.0 gpu_active_ms=5272.0 gpu_idle_ratio=0.389
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_probe: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_probe: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
browser-prove:metric topaccum_arm5_probe workload=multi_test/busy_loop_po2_18_topaccum_arm5_probe sample_cycle=18334 available_cycles=40968 preflight_major=5 data_major_value=5 selector_value=1 mismatch_count=40 first_mismatch_col=23
```

KeccakUnion(1) proof:

```text
browser-prove:metric prove_session_async wall_ms=102837.0 gpu_active_ms=69706.0 gpu_idle_ratio=0.322
browser-prove:done multi_test/keccak_union_topaccum_arm5_probe: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_probe: gpu_dispatches=5904 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0
browser-prove:metric topaccum_arm5_probe workload=multi_test/keccak_union_topaccum_arm5_probe sample_cycle=14689 available_cycles=10648 preflight_major=5 data_major_value=5 selector_value=1 mismatch_count=40 first_mismatch_col=23
```

## Interpretation

The earlier selector-mismatch hypothesis is falsified for two workload classes:

- BusyLoop: `preflight_major=5`, `data_major_value=5`, `selector_value=1`.
- KeccakUnion(1): `preflight_major=5`, `data_major_value=5`, `selector_value=1`.

The mismatch is also stable across both workloads: `mismatch_count=40`,
`first_mismatch_col=23`. That points away from workload selection and toward a
semantic mismatch in the generated arm5 WGSL body, wrapper state, field math, or
zero-back/load semantics.

## Rejected ext_inv Attempt

The first concrete field-math suspect was the WGSL prelude placeholder:

```text
fn ext_inv(x: ExtVal) -> ExtVal {
  return x;
}
```

A direct port of `BabyBearExtElem::inv()` was tried and the representative e2e
was tightened back to `mismatch_count == 0`. That is not safe to retain yet: two
consecutive Chrome e2e runs lost the WebGPU instance before receipt
verification:

```text
browser-prove:stage done prove_session_async segments=1 pending_keccaks=0 assumptions=0 elapsed_ms=34442
browser-prove:async-prove-error name=multi_test/busy_loop_po2_18_topaccum_arm5_probe err=JsValue(AbortError: Failed to execute 'mapAsync' on 'GPUBuffer': A valid external Instance reference no longer exists.)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 117 filtered out; finished in 34.58s
```

The direct ext-inverse port and zero-mismatch assertion were reverted. Next
work should isolate extension inversion in a minimal WGSL probe before wiring it
back into the large TopAccum arm module.

## Minimal ext_inv Formula Probe

A focused browser WebGPU probe was added to validate the candidate extension
inverse formula outside the large TopAccum arm module. It writes the WGSL
inverse and `x * inverse` product to a storage buffer and compares them against
`BabyBearExtElem::inv()` and extension-field one.

RED evidence: the same test was temporarily pointed at the current prelude
placeholder `ext_inv(x) { return x; }` and failed for the expected reason:

```text
assertion `left == right` failed: WGSL ExtElem inverse formula must match BabyBearExtElem::inv()
left: [1338199130, 1537125080, 1736051030, 1934976980]
right: [537199129, 631101944, 691837405, 1376831437]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 118 filtered out; finished in 0.10s
```

GREEN evidence: after switching the probe to the direct
`BabyBearExtElem::inv()` formula:

```text
test tests::sp7_ext_inv_formula_matches_baby_bear_on_chrome ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 118 filtered out; finished in 0.13s
```

Interpretation: the candidate formula is arithmetically valid in a minimal
Chrome WebGPU module. The earlier Chrome `mapAsync` / external-instance failure
when this formula was placed into the full TopAccum arm is therefore not enough
to reject the formula; it points instead to large-module integration, shader
capacity/device-loss behavior, or another generated-arm semantic mismatch. The
production prelude remains unchanged until the representative proof e2e can
pass with `mismatch_count=0`.

Follow-up diagnostic: the same formula was substituted into the prelude of the
guarded TopAccum arm5 split-module probe. That module keeps
`step_TopAccumArm5` reachable to Tint but does not execute it over proof data.
It reproduced the Chrome WebGPU instance-loss symptom:

```text
browser-prove:metric sp7_topaccum_arm5_ext_inv split module assembled: 904920 bytes
browser-prove:metric sp7_topaccum_arm5_ext_inv topaccum_arm5_guarded_main module_bytes=904920 phase=dispatch_FAILED err=JsValue(AbortError: Failed to execute 'mapAsync' on 'GPUBuffer': A valid external Instance reference no longer exists.)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 119 filtered out; finished in 32.60s
```

That negative probe is not retained as a default wasm test because it
intentionally loses the WebGPU device. The diagnostic result is still useful:
the direct formula is correct in a tiny module, but making it reachable inside
the current arm5 module exceeds or triggers a Chrome/Tint/Dawn capacity failure
before proof-data semantics can be evaluated. Next implementation should split
or specialize the inverse path rather than drop the direct formula into the
full arm module.

Representative receipt e2e rerun after adding this test-only probe:

```text
browser-prove:metric prove_session_async wall_ms=8750.0 gpu_active_ms=5298.0 gpu_idle_ratio=0.395
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_probe: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_probe: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
browser-prove:metric topaccum_arm5_probe workload=multi_test/busy_loop_po2_18_topaccum_arm5_probe sample_cycle=18334 available_cycles=40968 preflight_major=5 data_major_value=5 selector_value=1 mismatch_count=40 first_mismatch_col=23
browser-prove:metric prove_session_async wall_ms=109582.0 gpu_active_ms=75397.0 gpu_idle_ratio=0.312
browser-prove:done multi_test/keccak_union_topaccum_arm5_probe: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_probe: gpu_dispatches=5904 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0
browser-prove:metric topaccum_arm5_probe workload=multi_test/keccak_union_topaccum_arm5_probe sample_cycle=14689 available_cycles=10648 preflight_major=5 data_major_value=5 selector_value=1 mismatch_count=40 first_mismatch_col=23
test result: ok. 1 passed; 0 failed; 0 ignored; 118 filtered out; finished in 118.59s
```

## Split-Inverse Scratch Probe

The direct formula cannot be placed in the full TopAccum arm5 module without
losing the Chrome WebGPU device, so the probe now splits inversion into three
small steps:

1. A large arm5 capture kernel records each `ext_inv` denominator into a side
   buffer while keeping the full inverse formula unreachable.
2. A tiny inverse-buffer kernel applies the verified `BabyBearExtElem::inv()`
   WGSL formula to the side buffer.
3. A large arm5 consume kernel reruns the row using the side-buffered inverses.

First GREEN after this split reduced the mismatch from 40 columns to only the
terminal ExtVal column:

```text
browser-prove:metric topaccum_arm5_probe workload=multi_test/busy_loop_po2_18_topaccum_arm5_probe ... mismatch_count=4 first_mismatch_col=99
browser-prove:metric topaccum_arm5_probe workload=multi_test/keccak_union_topaccum_arm5_probe ... mismatch_count=4 first_mismatch_col=99
```

The remaining four columns were not inverse math. They were the
`terminal_ext_prefix` pass in `run_accum_steps`: the CPU reference has already
prefix-summed the final ExtVal column after raw `step_TopAccum`, while the
scratch probe was comparing a single raw row. The consume-side scratch probe now
adds the previous row's already-prefix-summed terminal value before comparison.

The representative browser e2e has been tightened back to assert
`mismatch_count == 0` for every workload. Assertion-active proof run:

```text
browser-prove:metric prove_session_async wall_ms=9918.0 gpu_active_ms=6460.0 gpu_idle_ratio=0.349
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_probe: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_probe: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
browser-prove:metric topaccum_arm5_probe workload=multi_test/busy_loop_po2_18_topaccum_arm5_probe sample_cycle=18334 available_cycles=40968 preflight_major=5 data_major_value=5 selector_value=1 mismatch_count=0 first_mismatch_col=4294967295 first_mismatch_expected=4294967295 first_mismatch_actual=4294967295

browser-prove:metric prove_session_async wall_ms=106829.0 gpu_active_ms=73198.0 gpu_idle_ratio=0.315
browser-prove:done multi_test/keccak_union_topaccum_arm5_probe: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_probe: gpu_dispatches=5904 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0
browser-prove:metric topaccum_arm5_probe workload=multi_test/keccak_union_topaccum_arm5_probe sample_cycle=14689 available_cycles=10648 preflight_major=5 data_major_value=5 selector_value=1 mismatch_count=0 first_mismatch_col=4294967295 first_mismatch_expected=4294967295 first_mismatch_actual=4294967295
test result: ok. 1 passed; 0 failed; 0 ignored; 118 filtered out; finished in 117.00s
```

## Wall-Time Impact

Current accepted wall-time reduction: **0 s**. The probe runs on a scratch
`accum` copy and intentionally adds overhead when enabled. Its value is reducing
the correctness search space before any authoritative GPU TopAccum replacement.

The representative correctness gate is now strong enough for the next step:
enable an authoritative arm5 replacement behind the same receipt-verification
test, initially for the exact arm5 rows proven by the scratch path.

## Authoritative Arm5 Replacement

TDD RED:

```text
error[E0432]: unresolved import `risc0_circuit_rv32im::prove::set_accum_gpu_arm5_authoritative_enabled`
    --> browser-prove/src/lib.rs:4665:13
     |
4665 |         use risc0_circuit_rv32im::prove::set_accum_gpu_arm5_authoritative_enabled;
     |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^----------------------------------------
     |             |                            |
     |             |                            help: a similar name exists in the module: `set_accum_gpu_arm5_probe_enabled`
     |             no `set_accum_gpu_arm5_authoritative_enabled` in `prove`
```

Implementation:

- Added an opt-in authoritative arm5 flag.
- Split CPU accumulation so authoritative mode runs raw `step_TopAccum` for all
  non-arm5 rows and skips major 5 before terminal prefix/carry.
- Reused the split-inverse arm5 kernels on the real `accum` buffer.
- Added a raw consume entry for authoritative mode and kept the prefixed consume
  entry for scratch comparison.
- Added a small GPU `terminal_ext_prefix` kernel so the post-arm5 buffer stays
  GPU-resident before machine-column carry.

GREEN compile:

```text
Finished `release` profile [optimized + debuginfo] target(s) in 4m 18s
Executable unittests src/lib.rs (.../browser_prove-8b5b728d1fc35820.wasm)
```

Authoritative BusyLoop + KeccakUnion receipt e2e, second run after first-use
noise:

```text
browser-prove:stage done rv32im_accumulate step_top_accum_cpu_skip_major5 cycles=262144 elapsed_ms=1709.000
browser-prove:stage done rv32im_accumulate topaccum_arm5_authoritative cycles=40968 elapsed_ms=31.000 gpu_active=true
browser-prove:metric prove_session_async wall_ms=10674.0 gpu_active_ms=7474.0 gpu_idle_ratio=0.300
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0

browser-prove:stage done rv32im_accumulate step_top_accum_cpu_skip_major5 cycles=262144 elapsed_ms=1599.000
browser-prove:stage done rv32im_accumulate step_top_accum_cpu_skip_major5 cycles=262144 elapsed_ms=1650.000
browser-prove:stage done rv32im_accumulate step_top_accum_cpu_skip_major5 cycles=262144 elapsed_ms=1634.000
browser-prove:stage done rv32im_accumulate step_top_accum_cpu_skip_major5 cycles=131072 elapsed_ms=752.000
browser-prove:metric prove_session_async wall_ms=105815.0 gpu_active_ms=72610.0 gpu_idle_ratio=0.314
browser-prove:done multi_test/keccak_union_topaccum_arm5_authoritative: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_authoritative: gpu_dispatches=5904 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0
test result: ok. 1 passed; 0 failed; 0 ignored; 119 filtered out; finished in 116.73s
```

The existing scratch representative gate was rerun after the shared split
kernel changes and still verifies both receipts with exact scratch comparison:

```text
browser-prove:metric topaccum_arm5_probe workload=multi_test/busy_loop_po2_18_topaccum_arm5_probe ... mismatch_count=0 first_mismatch_col=4294967295
browser-prove:metric topaccum_arm5_probe workload=multi_test/keccak_union_topaccum_arm5_probe ... mismatch_count=0 first_mismatch_col=4294967295
test result: ok. 1 passed; 0 failed; 0 ignored; 119 filtered out; finished in 117.99s
```

Canonical xgboost same-path A/B:

```text
authoritative:
browser-prove:metric prove_session_async wall_ms=103259.0 gpu_active_ms=63039.0 gpu_idle_ratio=0.390
browser-prove:done xgboost_topaccum_arm5_authoritative: segments=11 user_cycles=2294869 total_cycles=2883584
browser-prove:webgpu xgboost_topaccum_arm5_authoritative: gpu_dispatches=5258 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0

baseline:
browser-prove:metric prove_session_async wall_ms=103854.0 gpu_active_ms=60155.0 gpu_idle_ratio=0.421
browser-prove:done xgboost: segments=11 user_cycles=2294946 total_cycles=2883584
browser-prove:webgpu xgboost: gpu_dispatches=5258 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0
```

Interpretation:

- Correctness gate passed across BusyLoop, KeccakUnion(1), and xgboost.
- xgboost raw CPU `step_TopAccum` time dropped from 20.835 s to 17.842 s
  across 11 segments, while the authoritative arm5 GPU kernels added about
  0.28 s total.
- Same-path single-trial xgboost wall improved by 0.595 s (about 0.6%), but
  that is still small enough to treat as noisy until repeated. The important
  performance finding is that one arm is not a material lever by itself.

Current accepted default-path wall-time reduction: **0 s** because
authoritative arm5 remains opt-in. Current opt-in xgboost single-trial
observation: **-0.595 s**. Next material step is to extend the same
correctness-gated pattern to additional TopAccum arms or generate a broader
chunked TopAccum backend; arm5 alone cannot close the CUDA gap.

## SP7j -- representative workload coverage and TopAccum major histogram

Coverage check:

- `rv32im_accum_topaccum_arm5_representative_probe_e2e_verify` runs BusyLoop
  and `KeccakUnion(1)` sequentially in one browser wasm test. Both paths call
  `prove_succinct_info_async`, which asserts the receipt is succinct and verifies
  it against the image ID before returning.
- `rv32im_accum_topaccum_arm5_authoritative_e2e_verify` also runs BusyLoop and
  `KeccakUnion(1)` under the opt-in authoritative arm5 path and asserts the
  arm5 GPU dispatch counter advanced.
- `xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies` covers the
  canonical multi-segment deferred workload and validates the expected journal
  result after receipt verification.

RED for histogram instrumentation:

```text
error[E0432]: unresolved import `risc0_circuit_rv32im::prove::set_accum_gpu_major_histogram_enabled`
```

GREEN implementation:

- Added opt-in `set_accum_gpu_major_histogram_enabled`.
- `step_accum` logs a per-segment `topaccum_major_histogram` only when the flag
  is enabled.
- The xgboost authoritative receipt test enables the histogram for the proof
  and disables it after proof generation.

GREEN e2e proof with histogram:

```text
browser-prove:metric prove_session_async wall_ms=104633.0 gpu_active_ms=63260.0 gpu_idle_ratio=0.395
browser-prove:done xgboost_topaccum_arm5_authoritative: segments=11 user_cycles=2294869 total_cycles=2883584
browser-prove:webgpu xgboost_topaccum_arm5_authoritative: gpu_dispatches=5258 cpu_mirrors=181 cpu_fallbacks=0 cpu_only_ops=0
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 104.86s
```

Corrected xgboost TopAccum major totals across 11 segments:

```text
total=2883584
major0=768461 pct=26.65
major1=173854 pct=6.03
major2=399587 pct=13.86
major3=65151 pct=2.26
major4=5203 pct=0.18
major5=430888 pct=14.94
major6=351857 pct=12.20
major7=388742 pct=13.48
major8=74670 pct=2.59
major9=30308 pct=1.05
major10=194571 pct=6.75
major11=292 pct=0.01
major12=0 pct=0.00
```

Interpretation:

- KeccakUnion is now part of the required representative e2e proof gate, not a
  side measurement.
- arm5 is correctness-proven under BusyLoop, KeccakUnion, and xgboost, but it is
  only 14.94% of xgboost TopAccum cycles. That explains the small/noisy wall
  improvement.
- major0 is the next best target: it is 26.65% of xgboost TopAccum cycles and
  about 1.78x the major5 cycle count. It should not be implemented as another
  one-off hand slice; first make the arm extraction/generation reproducible by
  regenerating the existing arm5 artifact and comparing it to the checked-in
  slice, then use the same path for arm0.

## SP7k -- ChromeDriver flag correction for representative e2e reliability

User directive: representative e2e coverage must include workloads beyond the
single busy-loop path; `KeccakUnion` is a required candidate.

Current-tree scratch representative e2e:

```text
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
browser-prove:metric prove_session_async wall_ms=10901.0 gpu_active_ms=7456.0 gpu_idle_ratio=0.316
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_probe: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_probe: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
browser-prove:metric prove_session_async wall_ms=107352.0 gpu_active_ms=72786.0 gpu_idle_ratio=0.322
browser-prove:done multi_test/keccak_union_topaccum_arm5_probe: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_probe: gpu_dispatches=5904 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0
browser-prove:metric topaccum_arm5_probe workload=multi_test/keccak_union_topaccum_arm5_probe ... mismatch_count=0 first_mismatch_col=4294967295
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 118.52s
```

Diagnostic RED:

The first current-tree authoritative rerun did not reach receipt verification.
The session negotiated only the default Chrome limits, then died after a very
slow real-buffer proof path:

```text
browser-prove:webgpu-limits max_buffer_size=1073741824 max_storage_buffer_binding_size=1073741824 max_compute_workgroup_storage_size=32768
browser-prove:stage done commit_group_async rv32im_data elapsed_ms=67816.000 gpu_active=true
browser-prove:stage done commit_group_async rv32im_accum elapsed_ms=25545.000 gpu_active=true
browser-prove:async-prove-error name=multi_test/busy_loop_po2_18_topaccum_arm5_authoritative err=JsValue(AbortError: Failed to execute 'mapAsync' on 'GPUBuffer': A valid external Instance reference no longer exists.)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 120 filtered out; finished in 100.77s
```

Fast reproduction confirmed the environment problem without doing a full proof:

```text
browser-prove:webgpu-limits max_buffer_size=1073741824 max_storage_buffer_binding_size=1073741824 max_compute_workgroup_storage_size=32768
test tests::sp7_ext_inv_formula_matches_baby_bear_on_chrome ... ok
```

Fix:

- Updated `examples/browser-prove/webdriver.json` to pass Chrome arguments with
  explicit leading `--`.
- This made the unsafe WebGPU / Dawn flags take effect reliably for the wasm
  test runner.

Fast GREEN:

```text
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
test tests::sp7_ext_inv_formula_matches_baby_bear_on_chrome ... ok
```

Authoritative BusyLoop + KeccakUnion GREEN after the ChromeDriver fix:

```text
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152
browser-prove:metric prove_session_async wall_ms=10693.0 gpu_active_ms=7514.0 gpu_idle_ratio=0.297
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0
browser-prove:metric prove_session_async wall_ms=106012.0 gpu_active_ms=72536.0 gpu_idle_ratio=0.316
browser-prove:done multi_test/keccak_union_topaccum_arm5_authoritative: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_authoritative: gpu_dispatches=5904 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 116.94s
```

Interpretation:

- KeccakUnion is now included in both current representative browser gates:
  scratch semantic equality and authoritative real-buffer receipt verification.
- The low-limit Chrome session was an actual worktree/test-environment problem:
  it could cause false performance regressions and device-loss failures before
  the receipt verifier runs.
- Performance comparisons must record negotiated WebGPU limits. Runs with
  `max_buffer_size=1073741824` / `max_compute_workgroup_storage_size=32768` are
  not comparable to the high-limit RTX 5090 evidence set.

## SP7l -- reproducible TopAccum arm generator and post-generator e2e

User directive: do not add another TopAccum arm by hand; any new arm work must
be correctness-gated and representative across workloads, including KeccakUnion.

RED/GREEN generator test:

```text
rustc --edition=2021 --test risc0/circuit/rv32im/src/prove/wgsl_pruner.rs -o /tmp/wgsl_pruner_test
/tmp/wgsl_pruner_test topaccum_arm_generator_reproduces_vendored_arm5 --nocapture

running 1 test
test tests::topaccum_arm_generator_reproduces_vendored_arm5 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.01s
```

The test originally failed twice and caught real generator mistakes:

- first failure retained prior TopAccum mux arms in the generated prefix
  (`5780` generated nonblank lines vs `1114` expected);
- second failure exposed the step wrapper's extra source-location comment and
  compute-entry indentation differences.

The accepted comparison now checks exact nonblank line order/content against the
checked-in arm5 slice. Blank-line layout is ignored; WGSL code, comments, names,
arm-specific lookups, closure order, and compute entry content are not.

Implementation:

- Added `topaccum_arm_probe_wgsl(steps, arm)` in
  `risc0/circuit/rv32im/src/prove/wgsl_pruner.rs`.
- It parses the steps-only WGSL artifact, slices the selected `exec_TopExtract`
  and `exec_TopAccum` mux arms, renames the four arm entry functions, rewrites
  the reachable call graph, and emits only the closure plus
  `topaccum_arm{N}_main`.
- This is preparatory only; it does not change the runtime proving path until a
  generated arm is wired into `hal/webgpu.rs`.

Targeted cargo wasm compile note:

- `cargo test -p risc0-circuit-rv32im --target wasm32-unknown-unknown --features webgpu topaccum_arm_generator_reproduces_vendored_arm5 --lib --no-run`
  was attempted for the cfg-exact crate path.
- The first attempt left a stale build lock; it was killed.
- The restarted attempt spent over 20 minutes in one `rustc` process at ~100%
  CPU and was terminated to avoid blocking the proof-gate work. The pure
  generator is dependency-free enough to validate with direct `rustc --test`.

Post-generator representative authoritative browser e2e:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

The sandboxed run failed before execution because Cargo could not open
`examples/target/release/.cargo-lock` (`Read-only file system`), so the command
was rerun outside the sandbox. Release wasm compilation took 4m17s.

```text
browser-prove:webgpu-limits max_buffer_size=4294967292 max_storage_buffer_binding_size=2147483644 max_compute_workgroup_storage_size=49152

browser-prove:metric prove_session_async wall_ms=10718.0 gpu_active_ms=7512.0 gpu_idle_ratio=0.299
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_authoritative: gpu_dispatches=328 cpu_mirrors=11 cpu_fallbacks=0 cpu_only_ops=0

browser-prove:metric prove_session_async wall_ms=106276.0 gpu_active_ms=72927.0 gpu_idle_ratio=0.314
browser-prove:done multi_test/keccak_union_topaccum_arm5_authoritative: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_authoritative: gpu_dispatches=5904 cpu_mirrors=188 cpu_fallbacks=0 cpu_only_ops=0

test tests::rv32im_accum_topaccum_arm5_authoritative_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 117.22s
```

Interpretation:

- KeccakUnion remains in the representative proof gate after the generator work.
- The generator change is correctness-positive scaffolding, not a wall-time win.
- Next generated-arm work can target major0 without hand slicing, but must not
  be accepted until the same BusyLoop + KeccakUnion receipt-verification test
  passes, followed by xgboost for the multi-segment wall-time signal.
