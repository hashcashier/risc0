# SP7de NTT Radix-4 Global Step Rejected

Date: 2026-05-21

## Scope

Tested a forward-only radix-4 global NTT step for
`batch_expand_into_evaluate_ntt`, intended to combine two post-local NTT stages
per dispatch after the accepted 10-bit local NTT block.

## Candidate

- Added a focused browser HAL test for a proof-shaped global NTT case:
  `count=4`, `in_size=1024`, `expand_bits=2`, `out_size=4096`.
- Added `NTT_STEP_RADIX4_WGSL`.
- Routed remaining forward NTT stages through radix-4 paired stages, with an
  odd leftover stage falling back to the existing radix-2 kernel in the same
  command submission.

## Validation

Focused RED:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_batch_expand_ntt_uses_radix4_global_steps -- --nocapture
```

Result: CPU parity passed, then the test failed because
`webgpu_ntt_step_radix4_params` was absent.

Focused GREEN:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  webgpu_hal_batch_expand_ntt_uses_radix4_global_steps -- --nocapture
```

Result: passed in Chrome with high WebGPU limits.

Representative BusyLoop + KeccakUnion proof gate:

Log: `/tmp/sp7de-busy-keccak-radix4-ntt.log`

```text
BusyLoop wall_ms=7523 gpu_active_ms=3429 raw_compute_dispatches=551 queue_submits=173
KeccakUnion wall_ms=91879 gpu_active_ms=60202 raw_compute_dispatches=9682 queue_submits=3075
cpu_fallbacks=0 cpu_only_ops=0
receipts verified
webgpu_ntt_step_radix4_params uploads: BusyLoop=14, KeccakUnion=256
```

Representative xgboost proof gate:

Log: `/tmp/sp7de-xgboost-radix4-ntt.log`

```text
xgboost wall_ms=78302 segments=11 journal=30.528042544062632
gpu_active_ms=44539 raw_compute_dispatches=8746 queue_submits=2718
upload_bytes=3113706988 readback_bytes=7488592
cpu_fallbacks=0 cpu_only_ops=0
receipt verified
webgpu_ntt_step_radix4_params uploads=224
```

## Decision

Rejected and reverted.

The candidate was correctness-clean and reduced raw compute dispatches, but it
did not produce a material wall-time win:

- BusyLoop stayed flat vs SP7cy (`7507 -> 7523 ms`).
- KeccakUnion was slightly worse (`91537 -> 91879 ms`).
- xgboost was effectively flat vs latest default (`78466 -> 78302 ms`) and
  worse than the accepted focused SP7cq estimate (`~77470 ms`).

Accepted wall-time gain: `0`.

Do not retry simple radix-4 dispatch-count reduction as an immediate lever. A
future NTT attempt needs to reduce active memory traffic or the measured FRI
drain bucket materially, not just the raw dispatch count.

