# WASM/WebGPU Prover

> **Note:** `.recursive/...` evidence paths referenced in this document are preserved on the `wasm` archive branch; the presentation branch omits that process tree.

Status: optimization campaign complete through M11 (2026-07) — browser xgboost proving lands at ≈2.6× native CUDA on the same GPU (14.7 s vs 5.7 s). Labels like `SP<N>`/`M<N>` in these docs name campaign phases; `docs/wasm-webgpu-prover-learnings.md` records the SP-era in depth and summarizes the M-era in its closing section.

See `docs/wasm-webgpu-prover-learnings.md` for the pause handoff, current
implementation summary, validation evidence, performance findings, and resume
plan. See `.recursive/run/wasm-webgpu-prover-perf/02-to-be-plan.md` for the
locked ExecPlan and `.recursive/run/wasm-webgpu-prover-perf/addenda/02-to-be-plan.addendum-01.md`
for the **Correctness-First Discipline** that now governs every sub-phase.

## Correctness-First Discipline (binding rule, 2026-05-12)

Per `.recursive/run/wasm-webgpu-prover-perf/addenda/02-to-be-plan.addendum-01.md`:

Any correctness regression detected during performance work — verifier rejection,
proof panic before receipt, cycle drift, new `cpu_only_ops`/`cpu_fallbacks`
sites beyond the documented ledger, or Chrome WebGPU device loss — **IMMEDIATELY
halts all performance work** and invokes the `SP-CR` (Correctness Regression
triage) sub-phase. SP-CR preempts whichever SP is in flight, runs to completion
(reproduce → root-cause → fix → verify), and only then does the preempted SP
resume.

xgboost SP-CR **RESOLVED 2026-05-12 via D14+D15+D16 GPU-only fix** in
`risc0/zkp/src/hal/webgpu.rs`. D12 (cpu_mirror) was rejected by user and
reverted. Root cause: `WebGpuBuffer` had no `Drop` impl calling
`.destroy()`, so cumulative GPU memory across multi-segment recursion lifts
exceeded Chrome Dawn's per-context budget and the next allocation returned
an invalid buffer that silently failed validation (`VK_ERROR_OUT_OF_DEVICE_MEMORY`
captured by D14's `onuncapturederror` listener). D14 attaches the listener;
D15 caches `BabyBearElem::ROU_FWD` / `ROU_REV` once at HAL init; D16 wraps
`GpuBuffer` in `Rc<WebGpuBufferOwner>` whose `Drop::drop` calls
`buffer.destroy()` to release VRAM deterministically. xgboost succinct
receipt now verifies in 117.92 s on the full GPU path (~21× native CUDA).
xgboost reclassified to `verified` (full GPU path). SP10 unblocked.

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
let receipt = prover.prove_with_opts_async(env, ELF, &ProverOpts::succinct()).await?;
receipt.verify(IMAGE_ID)?;
```

For mixed workloads that can benefit from the SP6d dependency scheduler, use
the pooled constructor:

```rust
use risc0_zkvm::{webgpu_prover_pool, ExecutorEnv, ProverOpts};

let pool = webgpu_prover_pool(2).await?;
let env = ExecutorEnv::builder().build()?;
let receipt = pool.prove_with_opts_async(env, ELF, &ProverOpts::succinct()).await?;
receipt.verify(IMAGE_ID)?;
```

`webgpu_prover_pool(slots)` creates `slots` independent browser WebGPU devices
and routes the pool's async proving entrypoints through the dependency-graph
scheduler by default. The legacy phased pool route remains available as
`WebGpuProverPool::prove_with_ctx_sequential_async` for A/B comparisons.
`WebGpuProverPool::diagnostics()` and `reset_diagnostics()` aggregate backend
usage across all pool slots, matching the single-prover diagnostics workflow.
Diagnostics include bind-group layout creations/cache hits, bind-group
creations, and compute-pipeline creations/cache hits. The HAL caches compute
pipelines when their bind-group layouts were created by the same HAL; this is
safe because pipelines do not retain per-proof buffers. On the xgboost pooled
smoke, the cache reduced repeated compute-pipeline creation to 36 creations
and 1,592 hits, but wall time remained flat at 103.35 s.
The Poseidon2 fold-chain path also packs per-layer parameters into one
dynamic-offset uniform buffer and reuses one bind group per chain. On xgboost
this reduced bind-group creations from 12,510 to 9,022 and buffer allocations
from 15,323 to 11,835, while wall time stayed flat at 102.86 s.
The NTT step path uses the same dynamic-offset pattern for forward and inverse
NTT levels, reducing xgboost bind-group creations further to 3,358 and buffer
allocations to 6,171. The measured xgboost wall was 102.25 s, a small/noisy
improvement over the prior 102.82-103.35 s band.
The hash_rows Merkle path no longer uploads its output node buffer before
overwriting every digest. On xgboost this removed the 4.37 GB `nodes` upload
source, reducing total host-to-GPU upload bytes from 15.28 GB to 10.91 GB;
wall measured 101.93 s in the same diagnostic run.
Empty RV32IM scatter ranges are also treated as true no-ops before fallback
accounting, so xgboost now reports `cpu_fallbacks=0`; the change is a
diagnostic cleanup and wall time stayed noisy/flat.
Device-to-device copy diagnostics now attribute copies by destination buffer.
The xgboost pool smoke reported 7.22 GB of device-copy traffic before SP6l,
with 7,222,591,488 bytes of that total copied into `coeffs`; only 32,768 bytes
came from `final_coeffs`. Dead-after-commit groups now use an explicit
in-place WebGPU commit path:
RV32IM `code`/`accum`, recursion `accum`, and Keccak `code`/`data`/`accum`.
On xgboost this reduced device copies from 128 to 85 and device-copy bytes
from 7.22 GB to 5.76 GB, while wall time stayed flat at 101.97 s. RV32IM
`data` and recursion `ctrl`/`data` still use the copy-preserving path because
later accumulation reads those witnesses after their Merkle roots have seeded
the transcript. The copy-preserving path now fuses coefficient materialization
into the first inverse NTT pass, leaving the witness readable while avoiding the
large `coeffs` copy. Xgboost now reports only `final_coeffs` device copies:
32 copies / 32,768 bytes, with no `coeffs` copies. Wall time stayed flat/noisy
at 102.26 s and uploads stayed near 10.91 GB, so device-copy cleanup is closed
as a wall-time lever; the next material target is CPU-originated
RV32IM witness/accumulation work and its upload path.
SP7a split RV32IM accumulation timing and found xgboost spends about
22.13 s there across 11 segments: 20.55 s in generated `step_TopAccum`,
1.57 s in machine-column carry, and 0.01 s in terminal ExtVal prefix.
SP7b moved the isolated machine-column carry scan to WebGPU. The focused
GPU/CPU test and xgboost e2e proof both pass; xgboost wall measured
101.38 s, RV32IM accumulation fell to 21.03 s, and the remaining large
target is still generated `step_TopAccum`.
SP7c profiled recursion accumulation and found no equivalent cheap scan:
prefix products are only 59 ms across xgboost, while generated
`compute_accum` and `verify_accum` account for 5.66 s. The next material
recursion win also requires generated circuit execution on GPU.

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
- Async GPU-authoritative ZKP proof hooks for Merkle, PolyGroup, FRI,
  commit/finalize, rv32im segment proving, recursion lift, and browser server
  proof orchestration.
- Portable Rust `eval_check` support through generated `poly_ext` metadata,
  plus a WebGPU circuit `eval_check` hook with a generated straight-line WGSL
  prototype path that reuses fixed scratch slots based on generated
  `poly_ext` liveness.
- Browser circuit HALs for `rv32im`, `keccak`, and `recursion`, including
  browser-compatible Rust witness/accumulation bridges.
- Scoped WebGPU HAL bindings so existing prover orchestration can resolve the
  synchronous circuit prover constructors after async device setup.
- Browser `ProverImpl` and `WebGpuProver` plumbing that reuses the native
  prover server, recursion, assumptions, PoVW, receipt assembly, and verifier
  paths.
- `WebGpuProverPool` plumbing for multi-device browser runs, including a
  default dependency-graph scheduler path for heterogeneous segment, keccak,
  lift, join, and resolve work.
- A browser parity harness in `examples/browser-prove` covering public
  examples, native method guests, syscalls, precompiles, accelerators, and
  composite-to-succinct compression.

The current HAL keeps a CPU shadow because the existing `Hal::Buffer` trait is
synchronous while browser readback is asynchronous. Focused async proving can
now let successful WebGPU kernels own outputs until an explicit readback
boundary, but CPU shadows remain the correctness bridge for fallback paths and
transcript-facing data.

## Validation

The acceptance target is complete native local STARK proving parity in Chrome
WebGPU, with succinct receipts verified by the existing verifier.

Validated so far:

- wasm release test compilation for `browser-prove`
- WebGPU HAL readback tests for NTT, inverse NTT, FRI fold, hashing, mixing,
  copy, gather, scatter, prefix products, and generated `poly_ext`
  `eval_check`
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
- Focused async/GPU-authoritative `multi_test/poseidon2_basic` proving via
  `WebGpuProver::prove_with_opts_async`: the latest run produced a verified
  succinct receipt in Chrome in 4.03s after a 426.880871ms native CUDA
  baseline for the same 1-segment, 3598-user-cycle fixture. Chrome negotiated
  1 GiB buffer and storage-binding limits. This run recorded
  `eval_check` as 2 WebGPU dispatches and 0 CPU fallbacks, `batch_evaluate_any`
  as 8 GPU dispatches and 0 CPU fallbacks, and `mix_poly_coeffs` as 8 GPU
  dispatches and 0 CPU fallbacks. The rv32im and recursion checks use the
  interpreted WebGPU path.

Every group this section once listed as outstanding has since landed as
passing Chrome succinct-proof tests in `examples/browser-prove`: the
largest public examples (`groth16-verifier`, `xgboost`, `bn254`), BLST
and in-guest receipt verification, the assumption / continuation /
PoVW / guest-error and bigint2 parity groups, and the accelerator
completions (`multi_test/rsa_compat`, `multi_test/keccak_union`). The
known native-only exclusion (`RunUnconstrained`) is documented below.
The per-commit correctness gate and the close-out fixture set are
described in `docs/wasm-webgpu-validation.md`.

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

Correctness is the unconditional gate; the performance shape landed
through the SP- and M-era campaigns (see
`docs/wasm-webgpu-cuda-comparison.md`). Circuit `eval_check` runs on
the GPU for all four circuits through the scalar-bank hybrid
interpreter (PolyExt tape -> instruction stream; dynamically indexed
vec4 local arrays pay a large Tint tax, so the interpreter uses scalar
banks). A fully code-generated staged WGSL path also exists behind an
opt-in flag; it validates bit-exactly but measured ~33x slower than
the interpreter in Chrome, so it stays scaffolding.

Remaining known performance threads, in measured-impact order:
recursion transcript/FRI readback elimination (unscoped), the ~590 ms
fill bubble between segment proofs, and box cooling (the environment
is bimodal fast/slow — an operational lever, not a code one).

A production `gather_sample` chunking experiment showed that chunked binding
ranges alone are not enough for recursion-sized matrices: a 512 MiB single
source `GPUBuffer` produced all-zero output in Chrome. The HAL now gates GPU
allocation with `GPUSupportedLimits.maxBufferSize`, and the proof path keeps
the known-correct CPU fallback for oversized gather sources until it can
represent those matrices with tiled or staged WebGPU buffers.

`RunUnconstrained { unconstrained: true }` is a documented native-disabled
fixture in this checkout because the native syscall table does not register
`SYS_FORK` and the corresponding native proving test is ignored.

The accelerator/precompile group is complete end-to-end after fixing
the Poseidon2 syscall address ABI to use byte addresses at the
guest/ecall boundary. `multi_test/rsa_compat` and the full
`multi_test/keccak_union` fixtures — early-era timeouts — now produce
verified succinct receipts in Chrome: `KeccakUnion(1)` runs at ~34 s
(it doubles as the environment canary) and `KeccakUnion(3)` (11
segments + 25 keccak proofs) at 106.4 s vs the 20.7 s native CUDA
baseline. Keccak circuit `eval_check` runs on the GPU through the
interpreter (`cpu_fallbacks=0`); the early 1536-FP-slot cap that
forced its portable-WASM fallback was removed by the scalar-bank
interpreter banks.

## GPU-Authoritative Design

The `Hal` trait exposes synchronous CPU reads through `Buffer::view`,
`Buffer::get_at`, and `Buffer::to_vec`, while browser readback is
asynchronous. The async browser proof path bridges this with explicit
async readback points at the transcript boundaries where the generic
prover consumes GPU results:

- Merkle roots and top layers: `nodes.get_at(1)` and `nodes.slice(...).view(...)`
- Merkle query openings: gathered samples and sibling digests
- DEEP evaluations: `batch_evaluate_any` outputs copied into `eval_u` and
  `coeff_u`
- FRI final coefficients: `final_coeffs.view(...)`
- Portable circuit HAL bridges that call `to_vec`/`view_mut` for witness and
  check-polynomial work

Between those boundaries, successful WebGPU kernels own their
outputs; the CPU shadow remains as the correctness bridge for the
synchronous `Hal` contract and for fallback paths, not as a mirror of
every intermediate. Readback scheduling matters as much as kernel
speed here: `mapAsync` completion waits on the whole device queue, and
all awaiting tasks share one wasm thread, so the prover keeps every
pending readback inside a single `FuturesUnordered` and pool-offloads
CPU work that would otherwise starve completion callbacks.

See `docs/requirements/wasm-webgpu-prover.md` for the full acceptance matrix.
