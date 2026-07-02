# SP7dl Recursion Accum Assertion Elision Rejected

Date: 2026-05-21

## Candidate

Elide generated `ensure!` assertions only inside WebGPU recursion accumulation, relying on the enclosing receipt verification for correctness. The goal was to reduce the remaining CPU-side `recursion_witgen_accum` bucket in xgboost.

## RED

`xgboost_succinct_receipt_verifies --no-run` failed as expected before implementation because the test imported a missing `recursion_accum_assert_elision_scopes` counter.

Log:

- `/tmp/sp7dl-recursion-assert-elision-red.log`

## GREEN Compile

After adding the scoped elision wrapper and xgboost assertion, the focused wasm browser test binary compiled.

Log:

- `/tmp/sp7dl-recursion-assert-elision-green-compile.log`

## Representative E2E Proof

Command was run from `examples/browser-prove/` with the high Chrome/WebGPU limits:

- `max_buffer_size=4294967292`
- `max_storage_buffer_binding_size=2147483644`
- `max_compute_workgroup_storage_size=49152`

Result:

- xgboost receipt verified
- journal verified as `30.528042544062632`
- `wall_ms=78713`
- `segments=11`
- `raw_compute_dispatches=9674`
- `queue_submits=2718`
- `upload_bytes=3114057524`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

Stage totals:

- `prove_session_async count=1 sum_ms=78713`
- `prove_segment_async count=11 sum_ms=40012`
- `composite_to_succinct_async count=1 sum_ms=38532`
- `rv32im_witgen count=11 sum_ms=4043`
- `rv32im_witgen_accum count=11 sum_ms=11713`
- `recursion_witgen count=21 sum_ms=5326`
- `recursion_witgen_accum count=21 sum_ms=5964`

Log:

- `/tmp/sp7dl-xgboost-recursion-assert-elision.log`

## Decision

Rejected and reverted.

The candidate was correctness-clean, and `recursion_witgen_accum` improved against SP7dk's observed `6478 ms` bucket by about `514 ms`. But the total xgboost wall regressed against the current valid SP7dh baseline:

- SP7dh xgboost: `78096 ms`
- SP7dl xgboost: `78713 ms`
- delta: `+617 ms`

This fails the significant-wall-improvement rule. The local bucket win did not translate into end-to-end wall time, so BusyLoop and KeccakUnion were not rerun for this rejected candidate.

## Revert Verification

Removed all temporary assertion-elision markers:

- `recursion_accum_assert_elision`
- `with_accum_asserts_elided`
- `RECURSION_ACCUM_ASSERT_ELISION`
- `recursion_accum_asserts_elided`

Post-revert checks:

- `cargo fmt --check --manifest-path risc0/circuit/recursion/Cargo.toml`
- `cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml`
- `xgboost_succinct_receipt_verifies --no-run`

Post-revert compile log:

- `/tmp/sp7dl-post-revert-xgboost-no-run.log`

Accepted wall-time gain: 0.

Do not retry generated assertion elision as an immediate performance lever unless it is paired with a broader design that demonstrably removes enough CPU work to improve representative wall time. This result reinforces that local CPU-bucket savings below about one second can disappear into browser/GPU scheduling noise.
