Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7w -- coalesce finalize eval_u output readbacks
Date: 2026-05-19

## Headline

`finalize_async` used to read four `out` buffers per circuit proof: three
group evaluation outputs plus the check-group evaluation output. The check
evaluation depends only on `z`, `z_pow`, and `check_group`, not on the CPU
`poly_interpolate` result, so the four evaluations can be dispatched before
any readback and copied into one staging buffer/map operation without changing
the transcript.

Accepted as a production browser round-trip reduction:

- one `out` readback per proof instead of four;
- one queue submit for finalize eval_u output readback per proof instead of
  four;
- unchanged `out` readback bytes;
- receipt verification on BusyLoop, `KeccakUnion(1)`, and xgboost;
- no CPU fallback or CPU-only ops in proof e2e.

Accepted wall-time reduction: `0 s` for now. Single-run wall moved in the
right direction on KeccakUnion and xgboost, but the movement is below the
noise threshold for a material wall-time claim.

## RED

Added an e2e assertion that finalized eval_u outputs must be read with at most
one `out` readback per proof:

```text
out_readbacks <= final_coeffs_readbacks
```

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

Result: high WebGPU limits, BusyLoop and KeccakUnion receipts verified, then
the new assertion failed on the current SP7v implementation:

```text
webgpu-limits max_buffer_size=4294967292 \
  max_storage_buffer_binding_size=2147483644 \
  max_compute_workgroup_storage_size=49152

multi_test/keccak_union_topaccum_arm5_authoritative:
out=152 final_coeffs=38
readbacks=704 readback_bytes=10183944
queue_submits=3291
cpu_fallbacks=0 cpu_only_ops=0
```

This is a valid RED because proof generation and receipt verification completed
before the diagnostic assertion failed.

## GREEN

Implemented:

- `WebGpuHal::read_buffer_ranges_named`, a multi-buffer contiguous-range
  readback helper using one staging buffer, one queue submit, and one map.
- `read_webgpu_ext_buffers_named` for checked `BabyBearExtElem` decoding.
- `Prover<WebGpuHal>::finalize_async` now dispatches the three group
  `batch_evaluate_any` calls and the check `batch_evaluate_any` call before
  reading any `out` values, then reads all four output buffers in one `out`
  readback and writes the same `coeff_u` transcript bytes.
- Browser e2e assertions for KeccakUnion and xgboost require the coalesced
  `out` readback shape.

Compile/no-run:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify --no-run

result: passed
```

## Representative E2E

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_authoritative_e2e_verify -- --nocapture
```

Result:

```text
webgpu-limits max_buffer_size=4294967292 \
  max_storage_buffer_binding_size=2147483644 \
  max_compute_workgroup_storage_size=49152
test tests::rv32im_accum_topaccum_arm5_authoritative_e2e_verify ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 113.77s
```

BusyLoop:

```text
wall_ms=10650 gpu_active_ms=7427 gpu_idle_ratio=0.303
gpu_dispatches=328 raw_compute_dispatches=712 queue_submits=174
readbacks=32 readback_bytes=478272
cpu_fallbacks=0 cpu_only_ops=0
```

KeccakUnion(1):

```text
wall_ms=102883 gpu_active_ms=69885 gpu_idle_ratio=0.321
segments=4 pending_keccaks=9 assumptions=1
gpu_dispatches=5904 raw_compute_dispatches=12679 queue_submits=3177
readbacks=590 readback_bytes=10183944
final_coeffs readbacks=38 readback_bytes=38400
merkle_query readbacks=257 readback_bytes=9033800
nodes readbacks=257 readback_bytes=271392
out readbacks=38 readback_bytes=840352
cpu_fallbacks=0 cpu_only_ops=0
```

Comparison to SP7v:

```text
BusyLoop queue_submits: 180 -> 174
BusyLoop readbacks: 38 -> 32

KeccakUnion wall_ms: 103398 -> 102883
KeccakUnion queue_submits: 3291 -> 3177
KeccakUnion readbacks: 704 -> 590
KeccakUnion out readbacks: 152 -> 38
KeccakUnion readback_bytes: 10183944 -> 10183944
```

## xgboost E2E

Command:

```text
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies -- --nocapture
```

Result:

```text
webgpu-limits max_buffer_size=4294967292 \
  max_storage_buffer_binding_size=2147483644 \
  max_compute_workgroup_storage_size=49152
test tests::xgboost_topaccum_arm5_authoritative_succinct_receipt_verifies ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 120 filtered out; finished in 102.13s
```

Key metrics:

```text
wall_ms=101910 gpu_active_ms=61318 gpu_idle_ratio=0.398
segments=11
gpu_dispatches=5258 raw_compute_dispatches=11382 queue_submits=2774
readbacks=512 readback_bytes=7488592
final_coeffs readbacks=32 readback_bytes=32768
merkle_query readbacks=224 readback_bytes=6856000
nodes readbacks=224 readback_bytes=236544
out readbacks=32 readback_bytes=363280
cpu_fallbacks=0 cpu_only_ops=0
```

Comparison to SP7v:

```text
xgboost wall_ms: 102044 -> 101910
xgboost queue_submits: 2870 -> 2774
xgboost readbacks: 608 -> 512
xgboost out readbacks: 128 -> 32
xgboost readback_bytes: 7488592 -> 7488592
```

## Decision

Accepted. This removes three browser readback/map/submit round trips per
circuit proof in `finalize_async`, verified by real browser proof generation
and receipt verification on BusyLoop, `KeccakUnion(1)`, and xgboost.

Structural reduction:

```text
KeccakUnion readbacks: 704 -> 590 (-114)
KeccakUnion queue_submits: 3291 -> 3177 (-114)
xgboost readbacks: 608 -> 512 (-96)
xgboost queue_submits: 2870 -> 2774 (-96)
```

Wall movement is directionally positive but not accepted as a material win
without repeated A/B. Remaining largest readback sources are Merkle query and
commit-node data; further reductions likely need larger transcript-aware
batching, not await-order reshuffling.
