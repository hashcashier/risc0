# SP7am: default representative gate and high-limit guard

Date: 2026-05-19

## Problem

SP7al showed that the diagnostic candidate-sync wait was no longer forced, but
it also made the current measurement shape clearer: the opt-in TopAccum arm5
authoritative path moves real queued GPU work into the first accum commit.

Before adding more generated TopAccum arms, the run needed a production-default
representative gate. The default browser prover does not enable authoritative
arm5, so arm5-specific proof gates must not be treated as the production wall
baseline.

A second reliability issue also reappeared: Chrome can intermittently negotiate
low WebGPU limits even with the corrected `webdriver.json` flags. In that state
representative proofs produce invalid wall signals or time out after minutes.

## Low-Limit Reproduction

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  native_keccak_union_small_succinct_receipt_verify -- --nocapture
```

The run negotiated low limits and timed out before receipt verification:

```text
browser-prove:webgpu-limits max_buffer_size=1073741824 max_storage_buffer_binding_size=1073741824 max_compute_workgroup_storage_size=32768
browser-prove:stage done commit_group_async rv32im_code elapsed_ms=4366.000 gpu_active=true
browser-prove:stage done commit_group_async rv32im_data elapsed_ms=66639.000 gpu_active=true
browser-prove:stage done commit_group_async rv32im_accum elapsed_ms=59997.000 gpu_active=true
Failed to detect test as having been run. It might have timed out.
```

This is not comparable performance evidence. It is a harness reliability issue.

## RED

Added a representative performance-limit guard and a new default representative
proof gate that calls a missing `WebGpuProver::webgpu_limits()` API.

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_default_representative_e2e_verify --no-run
```

Expected failure:

```text
error[E0599]: no method named `webgpu_limits` found for reference `&WebGpuProver`
```

## GREEN

Implementation:

- `WebGpuHal::performance_limits()` returns the negotiated max buffer, max
  storage binding, and max workgroup-storage limits.
- `WebGpuProver::webgpu_limits()` exposes those limits to browser proof gates.
- `assert_representative_webgpu_limits()` fails fast unless Chrome negotiates
  the high-limit RTX 5090 profile:
  `4294967292 / 2147483644 / 49152`.
- Added `rv32im_default_representative_e2e_verify`, which proves BusyLoop and
  `KeccakUnion(1)` on the default path and asserts no opt-in arm5 dispatch.
- Added the same high-limit guard to xgboost default and arm5-authoritative
  gates.

GREEN compile:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_default_representative_e2e_verify --no-run

Finished `release` profile [optimized + debuginfo] target(s) in 4m 57s
```

## Default Representative e2e

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_default_representative_e2e_verify -- --nocapture
```

Result: passed in 111.37 s with high WebGPU limits, receipt verification, zero
CPU fallback/CPU-only ops, zero candidate-sync waits, zero opt-in arm5
dispatches, and zero `code` uploads.

| Workload | wall_ms | gpu_active_ms | raw dispatches | queue submits | uploads | upload_bytes |
|---|---:|---:|---:|---:|---:|---:|
| BusyLoop po2_18 default | 7542 | 3983 | 706 | 170 | 213 | 503984852 |
| KeccakUnion(1) default | 103542 | 69234 | 12605 | 3153 | 3732 | 5909974204 |

KeccakUnion retained representative shape:

- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`

## Default xgboost e2e

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  xgboost_succinct_receipt_verifies -- --nocapture
```

Result: passed in 100.27 s with high WebGPU limits, succinct receipt
verification, zero CPU fallback/CPU-only ops, zero `code` uploads, and retained
eval-u readback coalescing assertions.

| Workload | wall_ms | segments | gpu_active_ms | raw dispatches | queue submits | uploads | upload_bytes |
|---|---:|---:|---:|---:|---:|---:|---:|
| xgboost default | 100056 | 11 | 57145 | 11287 | 2721 | 3315 | 7024315252 |

## Comparison Against Opt-In Arm5 Gate

Compared against the high-limit SP7al arm5-authoritative representative data:

| Workload | Default wall_ms | Arm5 wall_ms | Direction |
|---|---:|---:|---|
| BusyLoop po2_18 | 7542 | 10473 | default faster by 2931 ms |
| KeccakUnion(1) | 103542 | 102905 | arm5 faster by 637 ms, noisy |
| xgboost | 100056 | 100474 | default faster by 418 ms |

Arm5 remains correctness-positive, but it is not an immediate production-wall
win. Its large hidden GPU work makes BusyLoop decisively worse, and xgboost does
not recover enough wall time to justify treating arm5 as the production
baseline.

## Decision

Accepted as a measurement and gate correction.

The production/default representative gate is now:

1. `rv32im_default_representative_e2e_verify` for BusyLoop + `KeccakUnion(1)`.
2. `xgboost_succinct_receipt_verifies` for multi-segment xgboost.
3. High WebGPU limits required up front.
4. Zero CPU fallback/CPU-only ops required through diagnostics.
5. KeccakUnion shape retained: `pending_keccaks=9`, `assumptions=1`.

The arm5-authoritative tests remain useful candidate/correctness gates, but
arm5 should not be used as the production performance baseline or expanded into
more arms until a candidate beats the default representative matrix.

Accepted production wall-time reduction: 0 s. This did not change the default
runtime path; it prevents invalid measurements and redirects work toward
immediate wall-time improvements.
