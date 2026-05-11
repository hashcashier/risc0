# WASM/WebGPU Prover Learnings, Methods, And Results

Status: paused at user request on 2026-05-11.

This document records what we built, how we validated it, what we learned, and
where the implementation still differs from native CUDA/Metal proving. It is
written as a handoff and retrospective, not as a formal completion audit. The
browser prover is capable of producing standard succinct STARK receipts through
the existing verifier path for the proof shapes we exercised, and the design is
now aligned with native local proving. The remaining work is mostly exhaustive
coverage, browser portability, and performance.

## Executive Summary

The WebGPU prover now extends the existing RISC Zero proving stack rather than
forking it. Browser proving enters through `WebGpuProver`, uses the existing
executor, prover server, circuit HALs, ZKP HAL, recursion path, receipt
assembly, and verifier APIs, and produces normal non-dev-mode receipts. The
important architectural result is that browser proofs reuse the native proof
system and verifier-visible semantics.

The browser path is async where WebGPU forces async behavior. In practice this
means browser callers should use `webgpu_prover().await`,
`prove_with_opts_async`, and `compress_async`. The synchronous proof APIs still
exist for narrow compatibility coverage, but the browser parity harness now
uses the async path by default so it exercises the GPU-authoritative proof
route instead of the older CPU-shadowed compatibility route.

Correctness is substantially ahead of performance. Focused proof fixtures
produce succinct receipts that verify with the existing verifier, and browser
stage diagnostics show WebGPU dispatches for proof-critical ZKP operations with
`cpu_only_ops=0` in the exercised paths. The same proof claims and cycle counts
match native runs for validated fixtures. The browser prover is still much
slower than native CUDA because native uses generated CUDA/Metal circuit
kernels, while WebGPU currently relies on generic/interpreted WGSL for
`eval_check` plus browser-compatible Rust/WASM witness paths for some circuit
work.

The biggest lesson is that generated WGSL is still the right direction, but the
right shape is not a single huge shader and not hundreds of tiny split shaders.
The next generated WGSL step should mirror CUDA/Metal: circuit-specific staged
kernels with explicit intermediate buffers and a bounded number of dispatches.

## What Was Implemented

### Rust/WASM API

The browser prover exposes a Rust-first API through the existing zkVM crate
surface:

- `webgpu_prover().await` initializes WebGPU and returns a `WebGpuProver`.
- `WebGpuProver::prove_with_opts_async` proves with normal `ExecutorEnv`,
  guest ELF, and `ProverOpts`.
- `WebGpuProver::compress_async` converts composite receipts to succinct
  receipts using the browser WebGPU recursion path.
- Receipts verify through existing verifier APIs such as `receipt.verify(...)`
  and `verify_integrity_with_context(...)`.

The async shape is intentional. Browsers require async adapter/device creation
and async buffer readback. Trying to hide this behind the native synchronous
API leads either to blocking shims or accidental CPU-shadowed execution.

### Existing Architecture Reuse

The implementation reuses the existing proof system boundaries:

- `risc0_zkp::hal::Hal` is extended by `WebGpuHal`.
- The browser prover plugs into the existing prover server/client path.
- `ProverOpts`, receipt kinds, claims, assumptions, recursion receipts, PoVW,
  and verifier logic remain the native ones.
- No verifier-visible receipt format, control ID, seal encoding, claim
  semantics, or verifier behavior was changed.

Browser-specific handling is concentrated around WebGPU device setup,
GPU-buffer state, async readback, and circuit HAL bindings.

### WebGPU ZKP HAL

The WebGPU HAL owns a real browser `GPUDevice` and `GPUQueue`, creates
`GPUBuffer`s, compiles WGSL kernels, and implements proof-critical bulk
operations. The implemented HAL coverage includes:

- buffer allocation, slicing, upload, download, and device copy
- NTT and inverse NTT
- batch expansion and polynomial evaluation
- FRI fold
- Poseidon2 row and fold hashing
- elementwise zero/copy/add/sum operations
- gather and scatter
- prefix products
- polynomial coefficient mixing
- combo preparation and division
- generated/interpreted `eval_check` paths

The key internal state change is that `WebGpuBuffer` tracks CPU-dirty and
GPU-dirty state. Older code updated CPU shadows after every GPU dispatch. The
async path can now let GPU kernels own outputs until a real transcript,
verification, or diagnostic boundary requires an explicit async readback.

### Circuit HAL Coverage

Browser circuit HALs exist for the proof circuits needed by zkVM proving:

- `rv32im` segment proving
- `keccak` coprocessor proving
- `recursion` proving for lift/compression paths

The current circuit implementation is correctness-oriented. Some circuit work
still uses browser-compatible Rust/WASM paths, especially witness-generation
style work. The WebGPU `eval_check` hook is in place and can dispatch
interpreted WGSL for real circuit checks. Keccak also has a focused WebGPU
versus portable eval-check regression.

### Async/GPU-Authoritative Proof Path

The async proof route now keeps more of the STARK proof pipeline on the GPU:

- async segment proof core
- async group commit/finalize
- async Merkle/Fri paths at the ZKP layer
- async recursion lift/compression
- async composite-to-succinct conversion
- async Keccak receipt unioning in the browser path

The browser acceptance harness in `examples/browser-prove` now uses
`prove_succinct_async`, `prove_succinct_info_async`, `prove_multi_async`, and
`prove_succinct_integrity_async` for parity tests. This matters because the
old synchronous helpers could produce valid receipts while taking a
CPU-shadowed route that did not exercise the GPU-authoritative browser backend
we want to validate.

### Keccak Eval-Check Parity Hook

Keccak received a wasm-only test utility:

- `risc0_circuit_keccak::webgpu_testutil::eval_check_webgpu_matches_portable`

The browser harness calls it through:

- `keccak_eval_check_poly_ext_matches_cpu`

This test compares WebGPU Keccak base eval-check output against the portable
CPU path. The current stable route uses the base interpreter WGSL path rather
than the disabled split-generated path.

## Validation Method

### Native First

The intended validation method is to measure native proving before browser
proving. This prevents browser timeouts from being misclassified as broken
when the native proof is already large.

The native baseline command should explicitly enable local proving, telemetry,
and CUDA when CUDA is part of the comparison:

```bash
RECURSION_SRC_PATH=/home/rami/repos/risc0/examples/target/release/build/risc0-circuit-recursion-4e96382f0d1db440/out/recursion_zkr.zip \
RISC0_PROVER=local RISC0_EXECUTOR=local RISC0_INFO=1 RUST_LOG=info RISC0_PRINT_SEGMENTS=1 \
cargo test --manifest-path examples/browser-prove/Cargo.toml --release --features cuda \
  <native_stats_test> -- --ignored --nocapture
```

A late `native_cfg_prove_stats` run completed while we were pausing:

```text
native_prove name=risc0-zkvm-methods/cfg elapsed=4.469601666s segments=1 user_cycles=2261 total_cycles=32768
```

That command did not explicitly pass `--features cuda`, so it should be treated
as a local native measurement, not as CUDA evidence. Earlier native CUDA
baselines in the validation notes remain the correct comparison anchors.

### Browser Harness

The browser harness is run through `wasm-bindgen-test-runner` with ChromeDriver
and the checked-in `webdriver.json` discovered from `examples/browser-prove`.

Typical command:

```bash
WASM_BINDGEN_TEST_TIMEOUT=1200 \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
/home/rami/.cache/.wasm-pack/wasm-bindgen-c59d5019a2b42393/wasm-bindgen-test-runner \
  --nocapture \
  /home/rami/repos/risc0/examples/target/wasm32-unknown-unknown/release/deps/browser_prove-0d71f73dbf7c3024.wasm \
  <browser_test_filter>
```

The working directory matters. Running from `examples/browser-prove` lets the
runner find browser capabilities that enable WebGPU. Running from the repo root
can fail before proof work with no `GPUAdapter`.

### Build Gate

The current browser harness rebuilds successfully:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release --no-run
```

Latest observed result after async harness conversion and cleanup:

```text
Finished `release` profile [optimized + debuginfo] target(s) in 1m 57s
Executable unittests src/lib.rs (examples/target/wasm32-unknown-unknown/release/deps/browser_prove-0d71f73dbf7c3024.wasm)
```

### Diagnostics

The browser prover prints:

- negotiated WebGPU limits
- proof stage timings
- segment count, user cycles, and total cycles
- HAL op dispatch counts
- CPU mirrors, CPU fallbacks, CPU-only ops
- upload, device-copy, readback, and buffer byte counts

The most useful correctness signal is not just "receipt verifies." The more
useful signal is:

1. Receipt is succinct.
2. Receipt verifies with the existing verifier.
3. Segment and cycle counts match native.
4. WebGPU diagnostics show GPU dispatches for proof-critical operations.
5. `cpu_only_ops=0`.
6. Any `cpu_fallbacks` are named and understood.

## Results

### Focused Small Proof: Poseidon2

`multi_test/poseidon2_basic` is the best small correctness and performance
anchor because it exercises native accelerator/precompile plumbing without a
huge proof.

Native CUDA baseline from prior validation:

```text
native_prove name=multi_test/poseidon2_basic elapsed=422.581291ms segments=1 user_cycles=3598 total_cycles=32768
```

Browser WebGPU:

```text
native_poseidon2_basic_async_succinct_receipt_verify passed
prove_session_async elapsed_ms=4163
test finished in 4.31s
```

This establishes the shape of the current performance gap for a small proof:
roughly 10x slower than CUDA, but still fast enough for iterative validation.
The receipt is succinct and verifies through the existing verifier.

### Public Example: Hello World

Native CUDA baseline:

```text
native_prove name=hello-world elapsed=417.445975ms segments=1 user_cycles=3532 total_cycles=32768
```

An earlier browser run accidentally used the synchronous helper and took about
207 seconds. After converting the public example harness to async:

```text
browser-prove:stage start prove_session_async segments=1 pending_keccaks=0 assumptions=0
browser-prove:stage start composite_to_succinct_async
browser-prove:stage done prove_session_async segments=1 pending_keccaks=0 assumptions=0 elapsed_ms=3781
test tests::hello_world_succinct_receipt_verifies ... ok
finished in 3.89s
```

This was a major diagnostic result. The prover itself was not inherently 200
seconds slow for the small example; the harness was calling the wrong path.

### Public Example: JSON

Native baseline recorded before the browser run:

```text
native_prove name=json elapsed=435.653682ms segments=1 user_cycles=13311 total_cycles=65536
```

Browser WebGPU after the async harness conversion:

```text
test tests::json_succinct_receipt_verifies ... ok
test result: ok. 1 passed; 0 failed; 75 filtered out; finished in 4.68s
```

Important browser details:

```text
browser-prove:webgpu-limits max_buffer_size=1073741824 max_storage_buffer_binding_size=1073741824 max_compute_workgroup_storage_size=49152
browser-prove:stage done prove_session_async segments=1 pending_keccaks=0 assumptions=0 elapsed_ms=4554
browser-prove:done json: segments=1 user_cycles=13311 total_cycles=65536
browser-prove:webgpu json: gpu_dispatches=312 cpu_mirrors=11 cpu_fallbacks=1 cpu_only_ops=0
```

The JSON proof demonstrates the current intended v1 behavior for normal public
examples: succinct receipt, verifier-compatible, matching cycles, WebGPU bulk
dispatches, and no CPU-only HAL operations.

### Keccak Eval-Check

The focused Keccak eval-check parity test currently passes:

```text
keccak_eval_check_poly_ext_matches_cpu
max_compute_workgroup_storage_size=49152
eval_check_base_interpreter_submit domain=65536 ... fp_slots=6741 mix_slots=11 workgroup_size=1
passed in 40.59s
```

This tells us two things:

- The WebGPU base interpreter path is correct for Keccak eval-check at the
  tested size.
- It is slow because Keccak needs 6741 FP slots and Chrome only grants about
  49 KiB of compute workgroup storage, forcing very little lane-level
  parallelism for this shader shape.

### Keccak Union Small

Native CUDA baseline:

```text
native_prove name=multi_test/keccak_union_small elapsed=7.549653789s segments=4 user_cycles=747310 total_cycles=917504
```

Browser WebGPU stable proof:

```text
native_keccak_union_small_succinct_receipt_verify passed
prove_session_async elapsed_ms=125251
test finished in 125.42s
```

This is an important result because it exercises pending Keccak proofs,
assumption plumbing, Keccak receipt integration, recursion, and succinct
compression. It is still about 16x slower than native CUDA, but the latest
stable implementation is much faster than earlier Keccak-union browser runs.

### CFG

The CFG fixture has native and browser harness coverage in the test suite. The
last run during the pause completed only the native local side:

```text
native_prove name=risc0-zkvm-methods/cfg elapsed=4.469601666s segments=1 user_cycles=2261 total_cycles=32768
```

As noted above, because that command did not explicitly pass `--features cuda`,
do not use it as a CUDA comparison. It is still useful as a proof that the
native stats helper works after the current code changes.

### Browser Harness Conversion

The browser harness now uses async proving for:

- public example acceptance tests
- internal method guest tests
- composite-to-succinct API coverage
- syscall and IO tests
- accelerator/precompile helpers
- assumption, continuation, PoVW, and guest-error integrity tests
- bigint2 precompile guest tests

Remaining sync proof call sites are intentionally narrow:

- sync helper definitions
- `native_busy_loop_po2_18_sync_succinct_receipt_verify`
- `native_keccak_union_sync_succinct_receipt_verify`

Those should be interpreted as sync API smoke coverage, not as the main
browser parity evidence.

## What We Learned

### 1. Verifier Compatibility Was The Right North Star

The most important correctness constraint was to avoid browser-specific receipt
semantics. Reusing the existing verifier forced the implementation to respect
native proof claims, control IDs, seal formats, and recursion behavior. This
kept the browser backend honest: a receipt either verifies through the existing
API or the backend is wrong.

### 2. Async Is Not Optional In Browsers

WebGPU device creation and buffer readback are async. The native prover API is
mostly synchronous. The first working browser implementation could hide that
with CPU mirrors, but performance and validation quality suffered.

The right model is:

- explicit async browser prover construction
- async proof and compression APIs
- explicit readback only at transcript or verifier-facing boundaries
- synchronous compatibility kept small and named

The hello-world result showed why this matters. The sync path produced valid
proofs but made a tiny proof look hundreds of seconds slower than it really was
on the async path.

### 3. CPU Shadows Are A Correctness Bridge, Not A Performance Strategy

The existing `Hal::Buffer` surface assumes synchronous CPU visibility through
methods like `view`, `get_at`, and `to_vec`. Browser WebGPU cannot provide
that without async readback.

The implementation therefore uses CPU shadows for compatibility but tracks
whether CPU or GPU owns the current value. That lets the async proof path
avoid unnecessary CPU mirrors while still supporting fallback and transcript
reads.

The long-term direction is to narrow synchronous CPU reads to explicit
boundaries:

- Merkle roots and top layers
- query openings
- DEEP evaluations
- final FRI coefficients
- verifier/transcript data

### 4. WebGPU Limits Shape The Kernel Design

Chrome commonly negotiated:

```text
max_buffer_size=1073741824
max_storage_buffer_binding_size=1073741824
max_compute_workgroup_storage_size=49152
```

The 1 GiB buffer/binding limits are enough for many focused proofs. The 49 KiB
workgroup storage limit is much tighter and directly affects eval-check
parallelism. Keccak's base eval-check needs 6741 FP slots, so the stable
interpreter path cannot run many lanes per workgroup.

This is why WebGPU proving can be GPU-dispatched but still much slower than
native CUDA: the shader shape underuses the GPU for large constraint systems.

### 5. Generic Generated WGSL Hit Browser Compiler/Device Limits

Generated WGSL was not abandoned. It was tried in several forms:

- monolithic straight-line generated eval-check WGSL
- slot-reusing generated WGSL
- split generated WGSL chunks
- batched split dispatches in one compute pass

The small generated path works, but large real circuits exposed practical
limits. Keccak split eval-check produced hundreds of chunks and eventually hit
Chrome/WebGPU instance or device loss during readback. Increasing private
scratch allowed some cases to pass but made performance much worse.

The conclusion is that generated WGSL should be circuit-specific and staged,
not generic and expression-tree based.

### 6. CUDA/Metal Are Faster Because They Have Circuit-Specific Kernels

Native CUDA and Metal do not run a generic eval-check interpreter. The tree has
generated circuit-specific kernels such as:

- `risc0/circuit/keccak-sys/kernels/cuda/eval_check_*.cu`
- `risc0/circuit/rv32im-sys/kernels/cuda/eval_check_*.cu`
- `risc0/circuit/recursion-sys/kernels/cuda/eval_check.cu`
- `risc0/circuit/recursion-sys/kernels/metal/eval_check.metal`

Those kernels are shaped for the circuit and native GPU backend. WebGPU needs
the same idea, translated into WGSL and adapted to browser limits.

The plan should be:

1. Keep the generic/interpreted WGSL path as a correctness fallback.
2. Generate staged WGSL for `rv32im`, `keccak`, and `recursion`.
3. Use explicit intermediate buffers between stages.
4. Keep the number of kernels bounded.
5. Validate each generated kernel against portable CPU before using it in full
   browser proofs.

### 7. Pipeline Caching Helps Correctness Hygiene More Than Runtime Today

Eval-check interpreter pipeline caching was added. It avoids recreating the
same pipeline shape repeatedly. For the current bottlenecks, the measured
effect was small:

- Keccak-small proof changed from about 125.398s to about 125.251s.

This means current runtime is dominated by actual proof work, shader shape,
uploads/readbacks, and circuit work rather than pipeline creation alone.

### 8. Keccak Scatter Was Not The Next Leverage Point

It was tempting to move Keccak scatter first, but inspection showed the
browser Keccak witness generation still uses Rust/WASM witness code. Moving
scatter alone would not remove the main CPU cost until the witness path itself
becomes GPU-side or better staged.

### 9. Device Loss Is A Real Optimization Constraint

The split eval-check experiments did not merely run slowly. Some lost the
WebGPU instance/device. That changes the optimization strategy:

- avoid huge shaders
- avoid hundreds of pipelines per proof stage
- avoid deferred giant GPU workloads that only fail at readback
- prefer semantic staging with bounded resource usage

The browser backend must treat device loss as a first-class error condition,
not just a performance failure.

### 10. Correctness Tests Need To Prove Receipts, Not Just Kernels

HAL CPU-vs-GPU tests are necessary, but not sufficient. Several optimizations
can pass isolated kernel tests and still break the transcript order, Merkle
query order, or verifier-facing data. The useful ladder is:

1. isolated HAL kernel equivalence
2. circuit eval-check equivalence
3. focused segment proof
4. composite receipt verification
5. composite-to-succinct compression
6. final receipt verification with the existing verifier
7. native cycle/segment comparison

## Why Browser Proving Is Slower Than Native CUDA

The browser prover is slower for several compounding reasons.

First, native CUDA uses generated, circuit-specific kernels. Browser WebGPU
currently uses generic/interpreted WGSL for large eval-check shapes. That is
correct but not competitive.

Second, some circuit phases still run through browser-compatible Rust/WASM
paths. CUDA runs witness, accumulation, and check code closer to the device.

Third, WebGPU has stricter resource and API constraints. Workgroup storage,
storage binding limits, dispatch limits, async readback, and browser device
loss all shape what kernels are viable.

Fourth, transcript and Merkle boundaries still require host-visible data.
Those readbacks are small compared with whole buffers, but they introduce
synchronization points that CUDA handles with a more mature native pipeline.

Fifth, current WebGPU kernels are still relatively granular. Many small
dispatches, uniform uploads, and temporary buffers add browser overhead.

The good news is that the gap is now explainable. For small proofs the browser
is roughly one order of magnitude slower, not hundreds of seconds slower. For
Keccak-heavy proofs the browser is still much slower because the Keccak
eval-check shader shape is correct but inefficient.

## Current Design Differences From CUDA/Metal

| Area | Native CUDA/Metal | Browser WebGPU |
| --- | --- | --- |
| API setup | Synchronous local prover setup | Async WebGPU adapter/device setup |
| Circuit kernels | Generated backend-specific kernels | Generic/interpreted WGSL plus Rust/WASM circuit bridges |
| Buffer model | Device buffers with native host synchronization | `GPUBuffer` plus CPU shadow and explicit async readback |
| Eval-check | Circuit-specific generated kernels | Small generated path, stable interpreter path, split path disabled |
| Keccak | Generated CUDA kernels for heavy circuit work | Correct browser path, still slow for large base eval-check |
| Recursion | Generated CUDA/Metal kernels | Async WebGPU ZKP path plus interpreted eval-check |
| Failure modes | Native CUDA errors | Browser adapter/device limits and device loss |
| Portability | Native backend-specific | Chrome primary, Firefox/Safari require limit and shader validation |

## Current Evidence By Requirement Area

This is not a completion audit. It records evidence gathered so far.

### API Parity

Evidence:

- `WebGpuProver::prove_with_opts_async`
- `WebGpuProver::compress_async`
- browser harness proves with `ProverOpts::succinct`
- composite-to-succinct compression is exercised
- receipts verify through existing APIs

Remaining:

- public documentation of the async differences should be tightened before a
  release.

### Architecture Reuse

Evidence:

- existing HAL and circuit HAL abstractions are reused
- existing prover server/client and recursion paths are reused
- existing verifier is unchanged

Remaining:

- keep new WebGPU-specific abstractions small and isolated as generated WGSL
  is added.

### Backend Selection

Evidence:

- browser path explicitly constructs WebGPU prover
- browser diagnostics print WebGPU limits and operation counters
- no Bonsai/dev-mode/fake receipt path is used by the acceptance harness

Remaining:

- typed error coverage for every device loss and unsupported-limit case should
  be audited.

### HAL Coverage

Evidence:

- focused HAL tests cover NTT, inverse NTT, FRI fold, hashing, mixing,
  elementwise operations, gather, scatter, prefix products, proof-shaped
  hash/NTT dimensions, and eval-check smoke paths
- browser proof diagnostics show WebGPU dispatches for proof-critical ops

Remaining:

- continue widening edge cases, especially oversized buffers and device-loss
  behavior.

### Circuit HAL Coverage

Evidence:

- rv32im, recursion, and Keccak browser circuit paths exist
- rv32im and recursion are exercised by small public examples
- Keccak eval-check has direct CPU parity coverage
- KeccakUnion(1) proves and verifies as a succinct browser receipt

Remaining:

- generated/staged WGSL circuit kernels are needed for competitive performance
  and broader long-running fixtures.

### Succinct Receipts And Verifier Compatibility

Evidence:

- focused browser runs produce succinct receipts
- receipts verify with the existing verifier
- no verifier-visible semantics were changed

Remaining:

- exhaustive matrix validation was paused, not completed in this turn.

### Example Matrix

Evidence:

- public examples are represented in the browser harness
- the public example proof calls now use async proving
- hello-world and JSON were re-confirmed after async conversion
- many examples had prior passing browser evidence in
  `docs/wasm-webgpu-validation.md`

Remaining:

- if work resumes, rerun the full example matrix after the latest async harness
  conversion and record fresh results.

### Internal Matrix

Evidence:

- internal method guests, syscall/IO paths, accelerator/precompile helpers,
  assumption/continuation/PoVW/guest-error paths, and bigint2 guests are wired
  to async browser proving in the harness
- focused Poseidon2 and KeccakUnion(1) proofs passed

Remaining:

- rerun the full internal matrix after the latest harness conversion.

### Browser Coverage

Evidence:

- Chrome/WebGPU is the active validation target
- code negotiates adapter/device limits and logs them

Remaining:

- Firefox and Safari automation remain future work unless their WebGPU
  automation becomes manageable without backend forks.

## Known Failed Or Reverted Experiments

### Split Eval-Check

The split-generated eval-check path was enabled experimentally. Keccak produced
251 chunks. It avoided some CPU fallback behavior but eventually failed with
Chrome/WebGPU instance/device loss around readback. Batching split dispatches
into a single compute pass did not fix it.

Current status:

```rust
const WEBGPU_EVAL_CHECK_ENABLE_SPLIT: bool = false;
```

### Large Private Scratch

Increasing private scratch so Keccak could run a different path made the
focused Keccak eval-check pass, but it was much slower:

- stable path: about 40.6s
- large private scratch experiment: about 179.1s

Current status:

```rust
const WEBGPU_EVAL_CHECK_BASE_PRIVATE_MAX_FP_SLOTS: usize = 1536;
```

### Pipeline Caching As Main Optimization

Pipeline caching was kept because it is correct and reduces repeated setup, but
it was not the main runtime lever. Keccak-small stayed around 125 seconds.

## Recommended Resume Plan

When work resumes, do not start by trying another generic split shader. The
next large step should be generated, circuit-specific WGSL.

Recommended order:

1. Freeze the current correctness state with a fresh full browser harness
   build and a small proof smoke suite.
2. Regenerate or translate circuit-specific eval-check kernels for WGSL,
   starting with recursion and rv32im because they affect every succinct proof.
3. Stage generated WGSL into a small number of kernels with explicit
   intermediate buffers.
4. Validate generated WGSL against portable CPU eval-check for tiny, medium,
   and production-shaped domains.
5. Run native CUDA baseline first, then browser proof, for each target.
6. Move Keccak witness/check work closer to GPU only after eval-check staging
   is stable.
7. Add browser portability runs for Firefox/Safari only after Chrome is stable
   and shader resource usage is bounded.

The next verification commands should start small:

```bash
# Native CUDA baseline
RECURSION_SRC_PATH=/home/rami/repos/risc0/examples/target/release/build/risc0-circuit-recursion-4e96382f0d1db440/out/recursion_zkr.zip \
RISC0_PROVER=local RISC0_EXECUTOR=local RISC0_INFO=1 RUST_LOG=info RISC0_PRINT_SEGMENTS=1 \
cargo test --manifest-path examples/browser-prove/Cargo.toml --release --features cuda \
  native_stats_tests::native_cfg_prove_stats -- --ignored --nocapture

# Browser async proof
cd examples/browser-prove
WASM_BINDGEN_TEST_TIMEOUT=1200 \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
/home/rami/.cache/.wasm-pack/wasm-bindgen-c59d5019a2b42393/wasm-bindgen-test-runner \
  --nocapture \
  /home/rami/repos/risc0/examples/target/wasm32-unknown-unknown/release/deps/browser_prove-0d71f73dbf7c3024.wasm \
  internal_cfg_succinct_receipt_verifies
```

Compact unpause checklist:

- Rebuild the wasm harness and record the new `browser_prove-*.wasm` artifact.
- Run native CUDA first for `cfg`, `hello-world`, `json`, `poseidon2_basic`,
  and `keccak_union_small`.
- Run the matching Chrome/WebGPU async browser tests from
  `examples/browser-prove`.
- Confirm each browser run logs `prove_session_async`, produces a succinct
  receipt, verifies with the existing verifier, and matches native segment and
  cycle counts.
- Refresh `docs/wasm-webgpu-validation.md` with only newly observed evidence.
- Start generated WGSL work only after the smoke suite is green.

## Files Changed During This Phase

Primary implementation files:

- `risc0/zkp/src/hal/webgpu.rs`
- `risc0/circuit/keccak/src/lib.rs`
- `examples/browser-prove/src/lib.rs`
- `examples/browser-prove/Cargo.toml`
- `examples/Cargo.lock`

Relevant documentation:

- `docs/requirements/wasm-webgpu-prover.md`
- `docs/wasm-webgpu-prover.md`
- `docs/wasm-webgpu-validation.md`
- `docs/wasm-webgpu-cuda-comparison.md`
- `docs/wasm-webgpu-prover-learnings.md`

## Pause State

The long-running goal is paused by user request. It should not be marked
complete without a fresh completion audit against
`docs/requirements/wasm-webgpu-prover.md`.

The strongest current statement is:

- The backend architecture is in place.
- The async browser proof path can produce verifier-compatible succinct STARK
  receipts.
- Focused public, internal, accelerator, and Keccak-heavy proofs have passed.
- The latest harness changes make async GPU-authoritative proving the default
  for parity tests.
- Exhaustive final parity validation and performance work remain for a future
  resume.
