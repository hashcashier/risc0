# browser-prove — RISC Zero proving in the browser (WASM + WebGPU)

This crate is the end-to-end harness for the WASM/WebGPU prover: a suite of
`wasm-bindgen` tests that run the full RISC Zero proving pipeline (rv32im
execution → STARK → lift/join → succinct receipt) inside Chrome, with GPU
work on WebGPU and multi-threaded CPU work on a rayon pool backed by web
workers + SharedArrayBuffer.

It doubles as the performance lab for that prover: most of the ~200 tests are
diagnostics and A/B probes from the optimization campaign (SP0–SP7, M0–M11).
The narrative documentation lives in [`docs/`](../../docs/):

- [`wasm-webgpu-prover.md`](../../docs/wasm-webgpu-prover.md) — design + plan
- [`wasm-webgpu-prover-learnings.md`](../../docs/wasm-webgpu-prover-learnings.md) — what worked, what didn't, and why
- [`wasm-webgpu-validation.md`](../../docs/wasm-webgpu-validation.md) — correctness/validation discipline
- [`wasm-webgpu-cuda-comparison.md`](../../docs/wasm-webgpu-cuda-comparison.md) — browser vs native CUDA baselines

> The `.recursive/...` evidence paths those documents cite are preserved on
> the `wasm` archive branch; the presentation branch omits that process tree.

As of 2026-07, the browser prover completes the xgboost succinct-receipt
fixture at ≈2.5× native CUDA wall time on the same GPU (RTX 5090: ~14.8 s
browser vs ~5.7 s CUDA, fast-state medians).

## Prerequisites

- Linux with an NVIDIA GPU and the Vulkan ICD installed (tested: RTX 5090,
  Chrome 148/149 + matching chromedriver).
- The Rust toolchain pinned by `rust-toolchain.toml`, plus the
  `wasm32-unknown-unknown` target and `rust-src` component (the harness
  rebuilds `std` with atomics via `-Z build-std`, hence `RUSTC_BOOTSTRAP=1`).
- `wasm-bindgen-test-runner` on `PATH`, matching the workspace's
  `wasm-bindgen` version.

## Running

From the repository root:

```bash
export RUSTC_BOOTSTRAP=1
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner
export CHROMEDRIVER=/path/to/chromedriver          # must match your Chrome
export WASM_BINDGEN_TEST_TIMEOUT=1200
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json
export VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json

cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release -Z build-std=panic_abort,std \
  --config 'target.wasm32-unknown-unknown.rustflags=["--cfg","getrandom_backend=\"wasm_js\"","--cfg","web_sys_unstable_apis","-C","debuginfo=0","-C","target-feature=+atomics,+bulk-memory,+mutable-globals","-C","link-arg=--max-memory=4294967296"]' \
  <TEST_FILTER> -- --nocapture
```

`webdriver.json` supplies the Chrome flags the prover needs (unsafe WebGPU
APIs, Vulkan ANGLE, SharedArrayBuffer, no GPU watchdog). The
`--max-memory=4294967296` link arg gives the wasm heap the full 4 GiB.

## Headline tests

| Filter | What it proves |
|---|---|
| `eval_check_poly_ext_matches_cpu` | GPU/CPU parity of the eval_check polynomial for all four circuits (rv32im, recursion, keccak, plus the zkp webgpu HAL) — the standard correctness gate, 4 tests |
| `rv32im_default_representative_e2e_verify` | Representative fixture: BusyLoop + KeccakUnion(1) segments proven and verified end-to-end; emits two wall-time metric lines — the smoke test and perf canary |
| `xgboost_succinct_receipt_verifies` | Full composite → lift/join → succinct pipeline on the xgboost guest (~15 s wall on an RTX 5090) |
| `native_keccak_union_succinct_receipt_verify` | Heavy fixture: 25-segment keccak union to a succinct receipt (~107 s) |

Each proving test prints a metric line on completion:

```
prove_session_async wall_ms=<f64> gpu_active_ms=<f64> gpu_idle_ratio=<f64>
```

## Measuring performance

Wall times in a browser/GPU environment are noisy and environment-coupled;
single samples are meaningless. The discipline that survived the campaign:

- **Repetitions + distributions**: n≥5 per arm, compare medians and spread;
  noise floors on the reference box are ±1% (KeccakUnion warm), ±2.5%
  (xgboost), ±5% (BusyLoop).
- **Same-state A/B only**: the environment is bimodal (readback-heavy
  fixtures shift +8–10% on warm afternoons with zero throttle flags —
  evidence points at GDDR temperature-compensated refresh). Bracket
  measurement blocks with a `rv32im_default_representative_e2e_verify`
  canary: KeccakUnion < 35 s ⇒ fast state, > 36.5 s ⇒ slow state.
- **Discard the first run** after any long idle gap (+5–8% warmup outlier),
  keep the machine quiet (no concurrent compiles), and freeze the tree while
  a block runs — the per-rep `cargo test` will happily rebuild mid-block.
