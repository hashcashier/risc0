# WASM/WebGPU Succinct Prover Requirements

Status: approved requirements seed

Date: 2026-05-08

## Goal Prompt

Do not stop until you create a fully working end-to-end WASM/WebGPU proving
backend for the RISC Zero zkVM proof system with complete native local STARK
proving parity.

The WASM/WebGPU prover must run locally in the browser, expose a Rust/WASM
proving and execution API equivalent to the native local proving API wherever
browser constraints allow, and produce standard non-dev-mode RISC Zero STARK
receipts for every program, precompile, accelerator, and proving mode supported
by the native local STARK prover. Succinct receipts are required for acceptance.
All generated receipts must be fully verifiable by the existing verifier
without changing receipt formats, seal encoding, control IDs, verifier
parameters, claim semantics, or verifier behavior.

The implementation must extend the current codebase and reuse the existing
HAL, circuit HAL, prover, receipt, recursion, and verifier abstractions as much
as practical. WebGPU should be added as another backend beneath the existing
proving stack, not as a separate browser proof system.

Correctness is the initial release gate. Performance is not a blocking
acceptance criterion for v1, but the design must preserve a credible path
toward competitive performance with native CUDA and Metal proving.

## Requirements

### R1. Native-Equivalent Rust/WASM API

Expose a Rust-first `wasm32-unknown-unknown` proving and execution API for
browser consumers.

Acceptance criteria:

- Browser consumers can execute guests, prove with `ProverOpts`, request
  succinct receipts, and verify receipts through APIs equivalent in shape and
  behavior to native local proving where browser constraints allow.
- Any unavoidable async differences from WebGPU adapter/device initialization
  are explicit in the Rust/WASM API and documented.
- JavaScript ownership is limited to `wasm-bindgen` bindings and test harness
  glue. A full JavaScript SDK or application framework is not part of this
  requirement.

### R2. Reuse Existing Prover Architecture

Extend the current codebase rather than building a parallel browser prover.

Acceptance criteria:

- WebGPU plugs into the existing HAL, circuit HAL, prover server/client,
  `ProverOpts`, receipt, recursion, and verifier paths wherever practical.
- New abstractions are introduced only where browser async/WebGPU constraints
  make the existing synchronous or native shape insufficient.
- Existing receipt assembly, recursion orchestration, verifier logic, and
  claim handling are reused instead of duplicated.

### R3. Explicit WebGPU Backend Selection

Add an explicit WebGPU prover backend for browser builds.

Acceptance criteria:

- The browser proof path cannot silently call Bonsai, native server proving,
  CUDA, Metal, fake receipts, or dev-mode.
- Missing WebGPU support, insufficient adapter/device limits, device loss, or
  unsupported browser behavior returns a clear typed error.
- The selected backend is visible in diagnostics and test output.

### R4. WebGPU ZKP HAL Coverage

Implement a WebGPU-backed equivalent of the `risc0-zkp::hal::Hal` operations
used by STARK proving.

Acceptance criteria:

- WebGPU supports the proof-critical HAL operations required by the current
  prover, including buffer allocation/copy/slicing, NTT and polynomial
  evaluation, FRI fold, hashing, mixing, elementwise operations, scatter,
  gather, prefix products, and required host/device transfers.
- CPU-vs-WebGPU tests cover each implemented HAL operation with representative
  sizes and edge cases.
- GPU resources are managed safely across browser device loss, dropped Rust
  values, and repeated proofs.

### R5. WebGPU Circuit HAL Coverage

Support WebGPU proving for the zkVM proof path required by the examples.

Acceptance criteria:

- `rv32im` segment proving works through WebGPU, including witness generation,
  accumulation, and `eval_check`.
- `keccak` coprocessor proving works through WebGPU, including witness
  generation and receipt integration.
- `recursion` proving works through WebGPU for lift, join, resolve, union,
  identity, and related succinct receipt operations needed by the current
  prover.
- Circuit HAL tests compare CPU and WebGPU outputs for representative
  preflight traces, witness buffers, accumulation buffers, and check
  polynomials.

### R6. Succinct Receipts Required

The end-to-end browser prover must produce non-dev-mode succinct receipts.

Acceptance criteria:

- `Composite` receipts may be used as intermediate artifacts.
- v1 is not complete until every in-scope native local STARK proving path can
  produce a `Succinct` receipt in the browser.
- Generated receipts verify with the existing verifier and default verifier
  context for the corresponding image ID.

### R7. Verifier Compatibility

Do not change verifier-visible proof semantics.

Acceptance criteria:

- Existing receipt formats, seal encoding, control IDs, verifier parameters,
  claim semantics, and verifier behavior remain unchanged.
- Existing native verifier tests continue to pass.
- WebGPU-generated receipts verify through the existing RISC Zero verifier APIs
  without browser-specific verifier branches.

### R8. Native Local STARK Parity Scope

Match the native local STARK prover surface, not only the public examples.

Acceptance criteria:

- Any deterministic zkVM ELF and `ExecutorEnv` that can be proven by the native
  local STARK prover can be proven by the browser WebGPU prover within browser
  resource limits.
- The browser path supports the native local proving operations needed for
  `prove`, `prove_with_opts`, `compress`, segment proving, keccak proving,
  recursion lift/join/resolve/union/identity, assumptions, continuations,
  PoVW, guest-error receipts supported by native STARK proving, and
  composite-to-succinct compression.
- Native precompiles and accelerators are in scope, including SHA-256,
  Poseidon2, BigInt, bigint2 field/extension-field/EC/RSA/ECDSA guests,
  Keccak, guest verification, syscalls used by proving fixtures, and any future
  native local STARK accelerator unless explicitly classified.
- STARK parity does not require browser support for native-only process or
  service harnesses, CUDA/Metal/Groth16 proof generation, Bonsai, Docker, or
  host diagnostics that are not part of receipt-producing local STARK proving.
  Any such exclusion must be documented with the native reason.

### R9. Example Acceptance Scope

Cover proving examples from `examples/Cargo.toml`.

Acceptance criteria:

- In scope means examples that currently call `default_prover`, `.prove(...)`,
  `get_prover_server`, or equivalent proving APIs.
- `browser-verify` is excluded because it is verification-only.
- Executor-only examples such as `bls12_381`, `c-kzg`, and `profiling` are
  excluded unless they gain a proving host path or deterministic proving
  fixture.
- `examples/prover` is treated as API parity coverage rather than a normal
  single-application example. The browser backend should support the same
  underlying proving and execution operations it demonstrates, while the exact
  service-style harness may differ if native process, thread, or server
  assumptions do not map cleanly to browser execution.

### R10. Required Example Matrix

Acceptance must cover the current proving examples.

Acceptance criteria:

- The browser harness proves and verifies deterministic runs for:
  `hello-world`, `json`, `chess`, `composition`, `jwt-validator`, `bevy`,
  `digital-signature`, `groth16-verifier`, `prorata`, `wasm`, `xgboost`,
  `bn254`, `password-checker`, `voting-machine`, `keccak`, `smartcore-ml`,
  `wordle`, `sha`, `c-guest/host`, `waldo`, `ecdsa/k256`, and `ecdsa/p256`.
- The matrix is generated or checked against `examples/Cargo.toml` so future
  example additions are classified explicitly.
- Any example excluded from the matrix must have a documented reason and owner
  follow-up.

### R11. Native Parity Test Matrix

Acceptance must cover the native local STARK proving surface used outside the
public examples.

Acceptance criteria:

- Browser tests prove and verify succinct receipts for representative internal
  zkVM method guests, including basic execution, stdin/slice IO, env vars,
  args, buffered reads, heap behavior, rand, BLST, benchmark loops, feature
  flags, and in-guest receipt verification.
- Browser tests prove and verify syscalls and receipt composition paths,
  including host callbacks, word callbacks, input digests, read fds, stdout,
  memory IO, assumptions, `verify_integrity`, `verify_assumption`,
  continuations, nonzero halt receipts with `prove_guest_errors`, PoVW work
  receipts, composite receipts, and `compress` to succinct receipts.
- Browser tests prove and verify native accelerator/precompile fixtures for
  SHA-256, SHA iteration, Poseidon2, BigInt, bigint2, Keccak update/syscall/
  union, RSA compatibility, allocation, random, and supported cryptographic
  example guests.
- Ignored or disabled native fixtures such as fork syscalls, trace/profiler
  diagnostics, and non-STARK proof kinds are classified separately and are not
  allowed to silently masquerade as browser proving support.

### R12. Deterministic Browser Fixtures

Each accepted example must be reproducible in browser tests.

Acceptance criteria:

- Examples needing files, images, large assets, randomness, clocks, CLI state,
  or network inputs use checked-in or generated deterministic fixtures.
- Acceptance tests require no network access.
- Inputs are small enough for regular browser CI unless explicitly marked as a
  scheduled/manual long-running test.

### R13. Browser Coverage

Chrome/WebGPU is the primary CI gate, with Safari and Firefox included where
automation is manageable.

Acceptance criteria:

- Chrome WebGPU is required in automated CI.
- Safari 26+ and Firefox WebGPU are supported from the start where stable
  automation is available without browser-specific prover architecture.
- Browser-specific behavior is isolated to adapter/device setup, feature and
  limit negotiation, worker/support detection, and test harness configuration.
- The proving code avoids Chrome-only shader or limit assumptions unless
  documented as temporary debt.

### R14. Correctness Gate

v1 acceptance is correctness-only.

Acceptance criteria:

- Every in-scope native local STARK proving path and every in-scope example
  produces a non-dev-mode succinct receipt in browser.
- Each generated receipt verifies with the existing verifier.
- There is no wall-clock proof-time threshold for v1 completion.

### R15. Performance-Aware Design

Preserve a path toward CUDA/Metal competitive performance.

Acceptance criteria:

- Proof-critical bulk operations run through WebGPU unless explicitly
  documented as temporary debt.
- The design batches GPU work, minimizes GPU-to-WASM readbacks, reuses
  pipelines and buffers where practical, and avoids unnecessary host-visible
  memory traffic.
- The implementation avoids architectural choices that would permanently cap
  WebGPU performance, such as excessive CPU/WASM round trips, browser-specific
  shader duplication, or scalarizing naturally parallel prover work.

### R16. Performance Telemetry

Collect performance data as a non-blocking v1 baseline.

Acceptance criteria:

- Browser tests record timing for guest execution, segment proving, keccak
  proving, recursion/lift/join/resolve proving, total proof time, and receipt
  verification.
- Browser, OS, GPU adapter info, and major WebGPU limits are recorded with
  each run.
- Telemetry is informational for v1 and suitable as the baseline for later
  optimization work.

### R17. Testing

Add layered correctness tests from kernels through full examples.

Acceptance criteria:

- HAL operation tests compare CPU and WebGPU results.
- Circuit HAL tests compare CPU and WebGPU witness, accumulation, and
  `eval_check` behavior.
- Receipt tests cover rv32im segment receipts, keccak receipts, recursion
  receipts, and full succinct receipts.
- Browser end-to-end tests prove and verify the required example matrix.
- Native tests continue to pass without requiring WebGPU.

### R18. Documentation

Document the browser prover for crate consumers and maintainers.

Acceptance criteria:

- Documentation covers browser requirements, secure-context requirements,
  WebGPU requirements, Rust/WASM API usage, build flags, feature gates,
  unsupported receipt kinds, failure modes, and troubleshooting.
- Documentation explains how the WebGPU backend relates to existing CPU, CUDA,
  and Metal HALs.
- The example acceptance matrix and known exclusions are documented.

## Out of Scope

- Browser-side Groth16 proving or shrink-wrapping.
- A full JavaScript SDK or application framework.
- Verification-only examples.

## Implementation Discovery Notes

These notes are not additional acceptance criteria; they record repository
facts that affect the implementation path.

- The existing `Hal` and `CircuitHal` traits are synchronous, while browser
  WebGPU buffer mapping and adapter/device setup are asynchronous. The public
  browser prover API should therefore be async at construction time, and any
  deeper async boundary needed for real device readback must be explicit rather
  than hidden behind CPU fallback.
- The current native proof-critical circuit kernels are generated C++/CUDA for
  `rv32im` and `keccak`; `recursion` additionally has Metal kernels for
  accumulation and `eval_check`.
- Browser `wasm32-unknown-unknown` cannot compile those native C++/CUDA/Metal
  circuit kernels directly. The browser path therefore needs WebGPU kernels or
  generated browser-compatible Rust/WASM witness and accumulation code.
- Generated Rust circuit bridges are acceptable for correctness bring-up when
  they preserve native circuit semantics and remain behind the browser WebGPU
  backend selection. They are performance debt until proof-critical bulk work
  is moved to WebGPU kernels.

## Current Implementation Checkpoints

- `risc0-zkp` now has a browser-only `webgpu` HAL module that owns a real
  `GPUDevice` and `GPUQueue`, allocates browser `GPUBuffer` storage, and keeps
  a CPU shadow for compatibility with the existing synchronous `Hal::Buffer`
  contract.
- The WebGPU HAL has compile-checked substrate helpers for buffer creation,
  uploads, readback, bind group layouts, bind groups, WGSL compute pipelines,
  dispatch, queue submission, and queue idle waits.
- Initial WebGPU-backed HAL operations are wired for buffer movement and
  proof-critical bulk operations used by the current prover, with conservative
  CPU-shadow synchronization where the synchronous `Hal::Buffer` contract
  still requires it.
- `rv32im`, `keccak`, and `recursion` have browser circuit HALs that use
  browser-compatible Rust witness/accumulation bridges and portable
  `eval_check` for correctness parity.
- Circuit crates expose explicit `*_prover_with_hal` constructors for browser
  code that already has an initialized `WebGpuHal`, avoiding synchronous
  WebGPU device initialization.
- Browser proving and compression calls now run inside scoped circuit HAL
  bindings, so existing prover orchestration can resolve synchronous
  `segment_prover`, `keccak_prover`, and `recursion_prover` calls to the
  initialized WebGPU HAL instead of trying to create a device synchronously.
- The ZKP HAL, Merkle, PolyGroup, FRI, STARK commit/finalize, rv32im segment
  proving, recursion lift, and zkVM server path now have async
  GPU-authoritative browser proving hooks with explicit readback boundaries.
- `risc0-zkvm` exposes the async Rust/WASM `webgpu_prover().await` constructor
  and intentionally leaves `default_prover()` unavailable for browser WebGPU
  builds because WebGPU adapter/device acquisition is asynchronous.
- The browser prover path reuses `ProverOpts`, `Prover`, `Executor`,
  `ProverServer`, receipt assembly, recursion, assumptions, PoVW, and the
  existing verifier APIs. It must fail clearly rather than falling back to CPU,
  CUDA, Metal, Bonsai, native server proving, fake receipts, or dev-mode.
- `examples/browser-prove` is the browser parity harness. It covers public
  proving examples, internal native method guests, syscall/IO cases,
  assumptions, continuations, PoVW, guest-error receipts, composite-to-succinct
  compression, and native accelerator/precompile fixtures.
- `docs/wasm-webgpu-prover.md` documents the current Rust/WASM API shape,
  feature flag, secure-context/WebGPU requirement, backend selection rule,
  architecture, validation status, and known performance debt.

## Current Validation Status

- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown
  --release --no-run` passes from `examples/browser-prove`.
- Chrome/WebGPU HAL readback tests pass for representative proof-critical
  operations, including NTT, inverse NTT, FRI fold, hashing, mixing, copy,
  gather, scatter, and prefix products.
- Chrome/WebGPU proof validation has produced succinct receipts that verify
  with the existing verifier for CFG and the moderate public proving examples:
  `hello-world`, `json`, `chess`, `composition`, `jwt-validator`, `bevy`,
  `digital-signature`, `prorata`, `wasm`, `password-checker`,
  `voting-machine`, `keccak`, `smartcore-ml`, `wordle`, `sha`,
  `c-guest/host`, `waldo`, `ecdsa/k256`, and `ecdsa/p256`.
- Chrome/WebGPU proof validation has also passed internal
  `bench/simple_loop`, `test_feature`, API/compression, and syscall/IO
  fixtures.
- Chrome/WebGPU proof validation has passed accelerator/precompile fixtures for
  libm, Poseidon2, SHA-conformance, allocation, random, SHA/Keccak
  digest/update cases, BigInt, and raw BigInt. The accelerator/precompile CUDA
  baseline passes end-to-end with segment/cycle telemetry.
- The focused public async path
  `native_poseidon2_basic_async_succinct_receipt_verify` passed in Chrome with
  a verified succinct receipt. It used `WebGpuProver::prove_with_opts_async`,
  produced 1 segment with 3598 user cycles and 32768 total cycles, and kept
  both rv32im and recursion ZKP commit/finalize in GPU-authoritative mode. The
  latest focused rerun took 4.03s in Chrome after a 426.880871ms native CUDA
  baseline for the same fixture. Chrome negotiated 1 GiB buffer and
  storage-binding limits. The run recorded `eval_check` as 2 WebGPU
  dispatches and 0 CPU fallbacks, `batch_evaluate_any` as 8 GPU dispatches and
  0 CPU fallbacks, and `mix_poly_coeffs` as 8 GPU dispatches and 0 CPU
  fallbacks. The rv32im and recursion checks use the interpreted WebGPU path.
- A WebGPU circuit `eval_check` hook now runs before the portable CPU fallback
  in async finalize, and a tiny generated straight-line WGSL `poly_ext`
  regression passes in Chrome. The generated WGSL path now reuses fixed named
  scratch slots based on `poly_ext` liveness. A real recursion monolithic
  shader probe still lost the WebGPU device after about 128 seconds, so large
  straight-line real circuit shaders remain disabled. The current prototype
  uses a small interpreted WGSL program for large real definitions; its
  recursion CPU-equivalence smoke test passes in Chrome, and the focused proof
  verifies with all focused `eval_check` work dispatched to WebGPU.
- A split recursion `eval_check` prototype was also tested. Naive 64-term
  chunks produced a device loss; a dependency-budgeted version showed that one
  recursion contribution still needs 1603 FP dependencies; and a slot-reusing
  split shader still lost the WebGPU device after 486.20s in Chrome on a
  `po2 = 0` CPU-reference smoke test. A later Keccak-only split attempt avoided
  the CPU fallback for one request but deferred about 148s of GPU work into
  `check_group` and then lost the WebGPU device. The split path is therefore
  disabled and Keccak still uses the known-correct portable fallback for its
  oversized `eval_check`.
- Native CUDA baselines with segment/cycle telemetry have been collected for
  the passed browser cases and for the deferred large cases. See
  `docs/wasm-webgpu-validation.md` and
  `docs/wasm-webgpu-cuda-comparison.md`.
- The remaining native parity groups and performance-blocked public examples
  must finish before the goal is complete.

## Current Blockers And Debt

- The WebGPU HAL now has a GPU-authoritative mode and explicit async
  GPU-to-CPU readback, and the ZKP Merkle/PolyGroup/FRI/commit/finalize layers
  have WebGPU async variants. The focused public async path is wired end to
  end, but full parity is still blocked by production-capable WebGPU circuit
  `eval_check`, transcript/Merkle readbacks, and CPU fallbacks for oversized
  WebGPU storage bindings.
- Real circuit `eval_check` needs a different GPU decomposition than
  monolithic or term-chunked straight-line WGSL. The interpreted WGSL
  prototype avoids device loss and verifies a focused proof with rv32im and
  recursion `eval_check` on GPU. Recursion currently chunk-uploads its
  oversized data group, so the next viable shape is a tiled or staged
  GPU-resident representation for that group, followed by an optimized
  interpreter or circuit-specific kernels that keep shader size and
  per-dispatch work below Chrome's device-loss threshold.
- `mix_poly_coeffs` now uses an async GPU-authoritative path in STARK
  finalize, with explicit readback before CPU fallback. The focused browser
  proof records 7 `mix_poly_coeffs` GPU dispatches and 1 fallback while still
  producing a verified succinct receipt.
- `combos_prepare` and `combos_divide` now have WebGPU kernels and async
  GPU-authoritative wrappers. The focused proof records 2 GPU dispatches for
  each operation with no CPU mirrors or fallbacks, and now keeps `combos`
  GPU-owned between combo mixing and combo division.
- A production `gather_sample` chunking attempt was diagnosed with a
  recursion-sized regression: the 512 MiB single source buffer required by the
  recursion data group produced all-zero GPU output in Chrome. The HAL now
  gates GPU allocation with `GPUSupportedLimits.maxBufferSize`, and the proof
  path keeps the known-correct CPU fallback for oversized gather sources until
  the WebGPU HAL can represent such matrices as tiled or staged buffers instead
  of one oversized `GPUBuffer`.
- The large public examples `groth16-verifier`, `xgboost`, and `bn254`, plus
  large internal fixtures such as BLST and in-guest receipt verification, are
  deferred until the async GPU-authoritative path is extended and optimized for
  the full matrix.
- The accelerator/precompile group is not complete in browser:
  `multi_test/rsa_compat` and the full `multi_test/keccak_union` fixture time
  out under the current browser proof path before producing succinct receipts.
  A smaller `KeccakUnion(1)` diagnostic now proves and verifies as a
  standalone Chrome/WebGPU succinct receipt after the async Keccak receipt
  union fix and async WebGPU Keccak subproof path. The latest focused run took
  441.14s in Chrome/WebGPU versus a 7.46676192s native CUDA baseline for the
  same 4 segments after Chrome negotiated 1 GiB WebGPU buffer and
  storage-binding limits. All ZKP bulk ops except `scatter` had 0 CPU
  fallbacks; the remaining blocker is Keccak circuit `eval_check`, which still
  falls back 9 times because the generic interpreter needs 6741 FP slots. The
  full `KeccakUnion(3)` fixture remains performance-blocked.
- `RunUnconstrained { unconstrained: true }` is classified as a native-disabled
  fixture in this checkout because `SYS_FORK` is not registered in the native
  syscall table and the native proving test is ignored.
- Any failing parity group remains a correctness blocker and must be fixed
  before completion.
- Executor-only examples unless they gain a deterministic proving fixture.
- Native WebGPU via `wgpu` as a substitute for browser WASM acceptance.
- Performance parity with CUDA or Metal as a v1 release gate.
- Receipt format, verifier, control ID, or verifier parameter changes.

## Constraints

- Target browser `wasm32-unknown-unknown`.
- Keep the public proving and execution API equivalent to native local proving
  wherever browser constraints allow.
- Reuse existing HAL and prover abstractions extensively.
- Make the minimal additions needed for browser proving to work through
  WebGPU.
- Do not require network access during acceptance tests.
- Do not use fake receipts or dev-mode.
- Do not silently fall back to non-WebGPU proving in browser acceptance tests.
- Keep existing verifier compatibility unchanged.
- Treat `.recursive` control files as absent in this checkout unless a run is
  created or the control plane is restored later.

## Open Questions

- Which Safari and Firefox platforms should be automated first after Chrome:
  macOS Safari, iOS Safari, Firefox Windows, Firefox macOS, Firefox Linux, or
  a smaller initial subset?
- What maximum browser CI runtime is acceptable before moving the largest
  examples to scheduled/manual runs?
- Should `examples/prover` become a full browser example, or should it remain
  API parity coverage with a browser-specific harness?
- Should native `wgpu` tests be added as a developer convenience while keeping
  browser WASM as the acceptance target?
