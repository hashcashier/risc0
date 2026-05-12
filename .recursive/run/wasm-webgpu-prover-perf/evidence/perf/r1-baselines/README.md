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

## Summary table (captured 2026-05-12)

Hardware: RTX 5090 (32 GB, SM120/Blackwell, CUDA 13.0), Chrome 148.0.7778.96, ChromeDriver 148.0.7778.97, headless w/ `enable-unsafe-webgpu enable-features=Vulkan use-angle=vulkan`. Negotiated WebGPU limits: `max_buffer_size=1073741824`, `max_storage_buffer_binding_size=1073741824`, `max_compute_workgroup_storage_size=49152`. Hermes vLLM service was inactive during these runs (freed GPU).

| Fixture | Native CUDA | Chrome WebGPU | Ratio | Segments | User cycles | gpu_dispatches | cpu_fallbacks | cpu_only_ops |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `risc0-zkvm-methods/cfg` | 499.5 ms | 3.82 s | **7.6×** | 1 | 2269 | 306 | 1 (scatter) | 0 |
| `hello-world` | 483.5 ms | 3.80 s | **7.9×** | 1 | 3560 | 306 | 1 (scatter) | 0 |
| `json` | 521.1 ms | 4.58 s | **8.8×** | 1 | 13319 | 312 | 1 (scatter) | 0 |
| `multi_test/poseidon2_basic` | 545.3 ms (cold) / 244.9 ms (warm) | 3.95 s | **7.2× / 16.1×** | 1 | 3553 | 306 | 1 (scatter) | 0 |
| `multi_test/libm` | 510.0 ms | 3.80 s | **7.5×** | 1 | 3328 | 306 | 1 (scatter) | 0 |
| `multi_test/keccak_union_small` | 7.748 s | 121.99 s | **15.7×** | 4 | 747265 | 6022 | 4 (scatter + 3 keccak eval_check) | 0 |

Per-fixture invariants confirmed:
- Receipt verifies through existing verifier on every fixture.
- Cycle counts match between native CUDA and Chrome WebGPU.
- `cpu_only_ops = 0` on every Chrome run.
- WebGPU negotiated 1 GiB buffer + 1 GiB storage-binding + 49 KiB workgroup-storage limits as expected.

Reference baselines (from pre-pause `docs/wasm-webgpu-cuda-comparison.md` for delta-vs-AS-IS visibility):
- `multi_test/poseidon2_basic`: was 426 ms CUDA / 4.03 s Chrome ≈ 9.4×; **now 7.2× (cold) — improvement**.
- `multi_test/libm`: was 436 ms CUDA / 214.08 s Chrome ≈ 491×; **now 7.5× — dramatic improvement (async/GPU-authoritative path now in effect for recursion eval_check)**.
- `multi_test/keccak_union_small`: was 7.47 s CUDA / 441.14 s Chrome ≈ 59×; **now 15.7× — significant improvement**.

Bonus native CUDA baselines (for the deferred-fixture matrix R9):
- `multi_test/rsa_compat`: 209.14 s, 407 segments, 91,535,630 user cycles, 106,463,232 total cycles.
- `multi_test/keccak_union` (KeccakUnion(3)): 20.68 s, 11 segments, 2,230,685 user cycles, 2,752,512 total cycles.

The remaining R10 work is no longer to close a 491× gap on libm — that already happened. The real residuals are now ~7-9× on small fixtures (smoke suite) and ~15× on keccak-heavy. SP2 (rv32im staged WGSL), SP3 (recursion staged WGSL + tiled data group), and SP6 (keccak staged WGSL) remain the levers to drive these toward 1.0×.
