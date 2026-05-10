# WASM/WebGPU Prover

Status: implementation active; Chrome/WebGPU parity validation in progress.

The browser WebGPU prover is intended to run local proving in a
`wasm32-unknown-unknown` browser build while reusing the native zkVM proving
stack. It exposes a Rust API and leaves JavaScript application bindings to the
crate consumer.

## Build

Use the `webgpu` feature on `risc0-zkvm` for browser proving builds:

```bash
cargo build -p risc0-zkvm --target wasm32-unknown-unknown --no-default-features --features webgpu
```

The repository config enables the required browser wasm cfgs:

- `web_sys_unstable_apis`
- `getrandom_backend="wasm_js"`

Browser execution requires a secure context and a browser with WebGPU exposed
through `navigator.gpu`.

## Rust API

WebGPU device initialization is asynchronous, so browser consumers should use
the async constructor instead of `default_prover()`:

```rust
use risc0_zkvm::{webgpu_prover, ExecutorEnv, ProverOpts};

let prover = webgpu_prover().await?;
let env = ExecutorEnv::builder().build()?;
let receipt = prover.prove_with_opts(env, ELF, &ProverOpts::succinct())?;
receipt.verify(IMAGE_ID)?;
```

`default_prover()` is intentionally unavailable for browser WebGPU builds
because it cannot synchronously request a `GPUAdapter`/`GPUDevice`.

## Backend Selection

The browser WebGPU path is explicit. It must not silently fall back to Bonsai,
native server proving, CUDA, Metal, fake receipts, or dev-mode. Unsupported
browser/device behavior and unimplemented kernels should fail clearly.

## Architecture

The implementation adds:

- `risc0_zkp::hal::webgpu::WebGpuHal`, a browser-only HAL that owns a
  `GPUDevice` and `GPUQueue`.
- WebGPU-backed HAL operations used by the current prover, with CPU-shadow
  synchronization where the synchronous `Hal::Buffer` contract still requires
  it.
- Portable Rust `eval_check` support through generated `poly_ext` metadata.
- Browser circuit HALs for `rv32im`, `keccak`, and `recursion`, including
  browser-compatible Rust witness/accumulation bridges.
- Scoped WebGPU HAL bindings so existing prover orchestration can resolve the
  synchronous circuit prover constructors after async device setup.
- Browser `ProverImpl` and `WebGpuProver` plumbing that reuses the native
  prover server, recursion, assumptions, PoVW, receipt assembly, and verifier
  paths.
- A browser parity harness in `examples/browser-prove` covering public
  examples, native method guests, syscalls, precompiles, accelerators, and
  composite-to-succinct compression.

The current HAL keeps a CPU shadow because the existing `Hal::Buffer` trait is
synchronous while browser readback is asynchronous. This is a correctness bridge,
not the intended final performance shape.

## Validation

The acceptance target is complete native local STARK proving parity in Chrome
WebGPU, with succinct receipts verified by the existing verifier.

Validated so far:

- wasm release test compilation for `browser-prove`
- WebGPU HAL readback tests for NTT, inverse NTT, FRI fold, hashing, mixing,
  copy, gather, scatter, and prefix products
- Chrome/WebGPU succinct proofs for the moderate public examples:
  `hello-world`, `json`, `chess`, `composition`, `jwt-validator`, `bevy`,
  `digital-signature`, `prorata`, `wasm`, `password-checker`,
  `voting-machine`, `keccak`, `smartcore-ml`, `wordle`, `sha`,
  `c-guest/host`, `waldo`, `ecdsa/k256`, and `ecdsa/p256`
- Chrome/WebGPU succinct proofs for `risc0-zkvm-methods/cfg`,
  `bench/simple_loop`, and `test_feature`
- Chrome/WebGPU API parity coverage for composite proof generation,
  composite-to-succinct compression, guest stdout output, and paused execution
- Chrome/WebGPU syscall/IO succinct proof coverage for host callbacks, word
  callbacks, input digests, read fds, stdout, and word output
- Chrome/WebGPU accelerator/precompile coverage for libm, Poseidon2,
  SHA-conformance, allocation, random, SHA/Keccak digest/update fixtures,
  BigInt, and raw BigInt fixtures

Still required before completion:

- browser runs for the largest public examples: `groth16-verifier`, `xgboost`,
  and `bn254`
- large internal fixtures such as BLST and in-guest receipt verification
- remaining native method, assumption, continuation, PoVW, and bigint2 parity
  groups
- accelerator/precompile completion for `multi_test/rsa_compat` and
  `multi_test/keccak_union`
- documentation of any native-only exclusions discovered during validation

Native baselines must be measured before browser attempts using local CUDA
proving and segment/cycle telemetry:

```bash
RISC0_PROVER=local RISC0_EXECUTOR=local RISC0_INFO=1 RUST_LOG=info RISC0_PRINT_SEGMENTS=1 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml --release --features cuda \
  <native_stats_test> -- --ignored --nocapture
```

See `docs/wasm-webgpu-validation.md` for the detailed matrix and current
segment/cycle counts.

## Performance Debt

Correctness is the v1 gate, but the current CPU-shadow bridge is not the final
performance shape. Proof-critical work should move toward batched WebGPU
kernels with minimal GPU-to-WASM readback and explicit async boundaries.

The current browser proof path is still CPU-bound because each WebGPU HAL
operation also updates the CPU mirror to satisfy the synchronous `Hal::Buffer`
contract. Browser diagnostics assert `cpu_only_ops == 0`, but they also show
`cpu_mirrors` tracking GPU dispatches. Making HAL buffers GPU-authoritative is
the next major requirement for practical large-example parity.

`RunUnconstrained { unconstrained: true }` is a documented native-disabled
fixture in this checkout because the native syscall table does not register
`SYS_FORK` and the corresponding native proving test is ignored.

The accelerator/precompile CUDA baseline passes end-to-end after fixing the
Poseidon2 syscall address ABI to use byte addresses at the guest/ecall boundary.
Chrome/WebGPU proves and verifies the smaller accelerator/precompile fixtures,
but the group is not complete: `multi_test/rsa_compat` and
`multi_test/keccak_union` still time out under the CPU-shadow proof path before
producing succinct receipts. `multi_test/keccak_union` also times out in a
standalone 7200s browser run, so it is a focused performance blocker.

## GPU-Authoritative Work

The current `Hal` trait exposes synchronous CPU reads through `Buffer::view`,
`Buffer::get_at`, and `Buffer::to_vec`. The generic prover uses those reads at
transcript boundaries:

- Merkle roots and top layers: `nodes.get_at(1)` and `nodes.slice(...).view(...)`
- Merkle query openings: gathered samples and sibling digests
- DEEP evaluations: `batch_evaluate_any` outputs copied into `eval_u` and
  `coeff_u`
- FRI final coefficients: `final_coeffs.view(...)`
- Default `combos_prepare` and `combos_divide`, unless a backend overrides
  them
- Portable circuit HAL bridges that call `to_vec`/`view_mut` for witness and
  check-polynomial work

Making buffers GPU-authoritative therefore requires more than deleting the CPU
mirror. The browser proof path needs explicit async readback points at the
transcript boundaries, plus WebGPU or generated browser kernels for the current
default CPU combo and circuit-HAL paths.

See `docs/requirements/wasm-webgpu-prover.md` for the full acceptance matrix.
