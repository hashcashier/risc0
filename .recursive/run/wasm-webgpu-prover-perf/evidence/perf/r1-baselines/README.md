# SP1 — R1 Baseline Capture

Status: initiated 2026-05-12; per-fixture runs land in Phase 4 test execution.

This directory holds the canonical six-fixture smoke-baseline pairs (native CUDA + Chrome WebGPU) that anchor the regression line for all subsequent SP2–SP11 work on this run.

## Fixtures in scope (R1 acceptance)

| Fixture | Native CUDA test name | Browser test name |
| --- | --- | --- |
| `risc0-zkvm-methods/cfg` | `native_stats_tests::native_cfg_prove_stats` | `internal_cfg_succinct_receipt_verifies` |
| `hello-world` | `native_stats_tests::native_hello_world_prove_stats` | `hello_world_succinct_receipt_verifies` |
| `json` | `native_stats_tests::native_json_prove_stats` | `json_succinct_receipt_verifies` |
| `multi_test/poseidon2_basic` | `native_stats_tests::native_poseidon2_basic_prove_stats` | `native_poseidon2_basic_async_succinct_receipt_verify` |
| `multi_test/libm` | `native_stats_tests::native_libm_prove_stats` | `native_libm_async_succinct_receipt_verify` |
| `multi_test/keccak_union_small` | `native_stats_tests::native_keccak_union_small_prove_stats` | `native_keccak_union_small_succinct_receipt_verify` |

## Native CUDA baseline command (per fixture)

```bash
cd /home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf

RECURSION_SRC_PATH=$(pwd)/examples/target/release/build/risc0-circuit-recursion-*/out/recursion_zkr.zip \
RISC0_PROVER=local RISC0_EXECUTOR=local RISC0_INFO=1 RUST_LOG=info RISC0_PRINT_SEGMENTS=1 \
cargo test --manifest-path examples/browser-prove/Cargo.toml --release --features cuda \
  <NATIVE_TEST_NAME> -- --ignored --nocapture \
  2>&1 | tee evidence/perf/r1-baselines/<FIXTURE>.native.txt
```

Replace `<NATIVE_TEST_NAME>` with the value from the table above and `<FIXTURE>` with the fixture's flat name (e.g., `poseidon2_basic`).

## Chrome WebGPU baseline command (per fixture)

```bash
cd /home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf/examples/browser-prove

WASM_BINDGEN_TEST_TIMEOUT=1200 \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
/home/rami/.cache/.wasm-pack/wasm-bindgen-c59d5019a2b42393/wasm-bindgen-test-runner \
  --nocapture \
  ../../examples/target/wasm32-unknown-unknown/release/deps/browser_prove-ea184bbfc45f01b0.wasm \
  <BROWSER_TEST_NAME> \
  2>&1 | tee ../../.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r1-baselines/<FIXTURE>.chrome.txt
```

## Per-fixture acceptance criteria (R1)

For each fixture:
- Native CUDA wall time + segment count + user cycles + total cycles recorded in `<fixture>.native.txt`.
- Chrome WebGPU wall time + receipt verification + `gpu_dispatches`/`cpu_mirrors`/`cpu_fallbacks`/`cpu_only_ops`/`uploads`/`upload_bytes`/`readbacks`/`readback_bytes`/`buffers`/`buffer_bytes` recorded in `<fixture>.chrome.txt`.
- `cpu_only_ops = 0` on every Chrome run (regression guard).
- Cycle count matches between native and Chrome.

## Summary table (to be populated in Phase 4)

| Fixture | Native CUDA (s) | Chrome WebGPU (s) | Ratio | Segments | User cycles | gpu_dispatches | cpu_fallbacks |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `risc0-zkvm-methods/cfg` | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| `hello-world` | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| `json` | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| `multi_test/poseidon2_basic` | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| `multi_test/libm` | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| `multi_test/keccak_union_small` | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |

Reference baselines (from pre-pause `docs/wasm-webgpu-cuda-comparison.md` for sanity-check during Phase 4 capture):
- `multi_test/poseidon2_basic`: 426 ms CUDA / 4.03 s Chrome (≈ 9.4×, 1 seg, 3598 user, 32768 total)
- `multi_test/libm`: 436 ms CUDA / 214.08 s Chrome (≈ 491×, 1 seg, 3373 user, 32768 total)
- `multi_test/keccak_union_small`: 7.47 s CUDA / 441.14 s Chrome (≈ 59×, 4 segs, 747310 user, 917504 total)
