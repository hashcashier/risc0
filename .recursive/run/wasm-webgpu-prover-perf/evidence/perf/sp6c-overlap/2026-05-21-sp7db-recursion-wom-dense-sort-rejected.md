# SP7db - Recursion WOM dense-sort rejected

Date: 2026-05-21
Status: rejected and reverted

## Candidate

`recursion_witgen` was the next measured CPU bucket after SP7cz:

- xgboost `recursion_witgen`: `5153 ms` across 21 lift/join proofs;
- xgboost `recursion_witgen_accum`: `6178 ms`;
- xgboost wall in the SP7cz diagnostic run: `77848 ms`.

The candidate changed recursion witness WOM bookkeeping to sort only emitted WOM
rows instead of sorting the fixed `cycles * 9` row array filled mostly with
invalid padding. Generated reads past the emitted row count still treated the
padding as implicit invalid rows, preserving the previous sorted-table
semantics.

## RED

Added a focused browser test:

```rust
#[wasm_bindgen_test]
fn recursion_witgen_dense_wom_storage_probe() {
    risc0_circuit_recursion::prove::debug_recursion_witgen_dense_wom_storage_probe().unwrap();
}
```

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  recursion_witgen_dense_wom_storage_probe --no-run
```

Expected failure:

```text
error[E0425]: cannot find function `debug_recursion_witgen_dense_wom_storage_probe`
```

## GREEN

Implementation:

- `MachineContext::wom_rows` became dense valid-row storage.
- `max_wom_rows` retained the old implicit padding bound.
- `plonk_read_wom` and previous-row injection treated missing dense entries as
  implicit invalid rows.
- Added a test-only probe exported through the recursion prove module.

Focused compile:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  recursion_witgen_dense_wom_storage_probe --no-run
```

Result:

- passed;
- elapsed: `3m01s`.

Focused Chrome test:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
    --target wasm32-unknown-unknown --release \
    recursion_witgen_dense_wom_storage_probe -- --nocapture
```

Result:

- `test tests::recursion_witgen_dense_wom_storage_probe ... ok`
- `test result: ok. 1 passed; 0 failed; 138 filtered out; finished in 0.00s`

## Representative e2e: BusyLoop and KeccakUnion

Command output: `/tmp/sp7db-busy-keccak-dense-wom.log`

Result:

- high WebGPU limits: `4294967292 / 2147483644 / 49152`
- `test tests::rv32im_default_representative_e2e_verify ... ok`
- `test result: ok. 1 passed; 0 failed; 138 filtered out; finished in 99.86s`
- verified receipts for BusyLoop and KeccakUnion
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

BusyLoop:

- `wall_ms=7230`
- `gpu_active_ms=3205`
- `gpu_idle_ratio=0.557`
- `raw_compute_dispatches=609`
- `queue_submits=173`

KeccakUnion:

- `wall_ms=92336`
- `gpu_active_ms=60357`
- `gpu_idle_ratio=0.346`
- `raw_compute_dispatches=10660`
- `queue_submits=3075`

## Representative e2e: xgboost

First xgboost attempt: `/tmp/sp7db-xgboost-dense-wom.log`

- failed before proof generation on the known invalid low-limit Chrome profile;
- limits: `1073741824 / 1073741824 / 32768`;
- not counted as correctness or performance evidence.

Rerun output: `/tmp/sp7db-xgboost-dense-wom-r2.log`

Result:

- high WebGPU limits: `4294967292 / 2147483644 / 49152`
- `test tests::xgboost_succinct_receipt_verifies ... ok`
- `test result: ok. 1 passed; 0 failed; 138 filtered out; finished in 78.80s`
- `wall_ms=78563`
- `segments=11`
- `gpu_active_ms=45041`
- `gpu_idle_ratio=0.427`
- `raw_compute_dispatches=9674`
- `queue_submits=2718`
- `upload_bytes=3113888480`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

Target buckets:

| Bucket | SP7cz | SP7db | Delta |
|---|---:|---:|---:|
| `recursion_witgen` | `5153 ms` | `5100 ms` | `-53 ms` |
| `recursion_accumulate` | `6066 ms` | `6408 ms` | `+342 ms` |
| `recursion_witgen_accum` | `6178 ms` | `6518 ms` | `+340 ms` |
| large Merkle row-hash | `27308 ms` | `27444 ms` | `+136 ms` |

## Decision

Rejected and reverted.

Correctness was clean across the focused probe, BusyLoop, KeccakUnion, and
xgboost, but the measured target bucket did not improve materially. The dense
WOM storage reduced `recursion_witgen` by only `53 ms` in the xgboost gate,
while overall xgboost wall was worse than the accepted default SP7cy sample
(`78466 -> 78563 ms`) and worse than the focused accepted SP7cq estimate
(`~77470 ms`). This does not meet the immediate significant wall-time
improvement bar.

Follow-up: do not spend more near-term work on CPU-side recursion WOM storage
layout. Remaining material recursion work would need generated `micro_ops` /
Poseidon2 witness or accumulator chunks, but the measured Merkle row-hash and
RV32IM TopAccum CPU buckets still have larger ceilings.
