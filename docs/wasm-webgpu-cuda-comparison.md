# Browser WebGPU vs Native CUDA Proving

Status: working comparison for optimization discovery.

This note compares the current browser WebGPU proving path with native CUDA
proving as implemented in this branch. It is intentionally focused on the
parts that explain correctness parity and the current order-of-magnitude
runtime gap.

## Current Measurement Anchor

All native baselines should be measured before browser runs with:

```bash
RISC0_PROVER=local RISC0_EXECUTOR=local RISC0_INFO=1 RUST_LOG=info RISC0_PRINT_SEGMENTS=1
```

The focused async/GPU-authoritative correctness anchor is
`multi_test/poseidon2_basic`. Both paths prove one segment with the same cycle
count and produce a succinct receipt that verifies with the existing verifier:

| Path | API | Receipt | Segments | User cycles | Total cycles | Wall time |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| Native CUDA | native local prover | Succinct | 1 | 3598 | 32768 | 426.538233ms |
| Chrome/WASM/WebGPU | `WebGpuProver::prove_with_opts_async` | Succinct | 1 | 3598 | 32768 | 30.42s |

Browser stage telemetry for that run:

| Browser stage | Time |
| --- | ---: |
| `execute` | 56ms |
| `segment_preflight` | 22ms |
| `rv32im_witgen` | 86ms |
| `rv32im_accumulate` | 238ms |
| `rv32im eval_check_interpreter_submit` | 1ms |
| `segment_prove_core_async` | 995ms |
| `verify_segment` | 13ms |
| `recursion_witgen` | 204ms |
| `recursion_accumulate` | 201ms |
| `recursion chunked eval_check_interpreter_submit` | 5 dispatches |
| `lift_prove_async` | 29.223s |
| `verify_lift` | 15ms |
| `prove_session_async` | 30.282s |

The same proof recorded mostly GPU-authoritative ZKP dispatches, with explicit
readbacks and CPU fallbacks at the remaining compatibility boundaries:

```text
gpu_dispatches=303 cpu_mirrors=12 cpu_fallbacks=8 cpu_only_ops=0
uploads=725 upload_bytes=1042124836
device_copies=8 device_copy_bytes=212208640
readbacks=68 readback_bytes=122424608
buffers=795 buffer_bytes=1862479004
```

This latest run is correctness-positive for the interpreted circuit
`eval_check` path: the proof still verifies, and the HAL operation counters
now record `eval_check` as 6 WebGPU dispatches and 0 CPU fallbacks. The first
dispatch is the rv32im check (`domain=131072`, `instructions=20202`,
`fp_slots=927`, `mix_slots=29`). The recursion data group is too large for one
GPU buffer/binding (`536870912` bytes with a `125829120` byte binding cap), so
the prototype uploads it in five chunks and dispatches the interpreter over
those chunks.

For the older `multi_test/libm` performance anchor, both paths also prove one
segment with the same cycle count:

| Path | Receipt | Segments | User cycles | Total cycles | Wall time |
| --- | --- | ---: | ---: | ---: | ---: |
| Native CUDA | Succinct | 1 | 3373 | 32768 | 436.441617ms |
| Chrome/WASM/WebGPU | Succinct | 1 | 3373 | 32768 | 214.08s |

Browser stage telemetry for that run:

| Browser stage | Time |
| --- | ---: |
| `execute` | 57ms |
| `segment_preflight` | 21ms |
| `rv32im_witgen` | 85ms |
| `rv32im_accumulate` | 309ms |
| `rv32im_eval_check` | 32.546s |
| `segment_prove_core` | 43.592s |
| `verify_segment` | 14ms |
| `recursion_witgen` | 207ms |
| `recursion_accumulate` | 207ms |
| `recursion_eval_check` | 119.937s |
| `lift_prove` | 170.293s |
| `verify_lift` | 15ms |
| `prove_session` | 213.951s |

That older browser proof recorded WebGPU dispatches, but it predated the full
async/GPU-authoritative recursion path and still used the synchronous CPU-shadow
path, so every dispatch had a CPU mirror update:

```text
gpu_dispatches=950 cpu_mirrors=950 cpu_fallbacks=1 cpu_only_ops=0
uploads=2015 upload_bytes=643307516
device_copies=8 device_copy_bytes=212208640
readbacks=0 readback_bytes=0
buffers=2032 buffer_bytes=1736647420
```

## Side-By-Side Proving Path

| Phase | Browser WASM/WebGPU | Native CUDA |
| --- | --- | --- |
| API entry | `webgpu_prover().await` initializes a browser `WebGpuHal` and returns `WebGpuProver`. `default_prover()` is intentionally unavailable for browser WebGPU because adapter and device creation are async. | Native callers use the normal local prover path. With CUDA enabled and `RISC0_PROVER=local`, the existing prover selectors create CUDA-backed segment and recursion provers. |
| Backend binding | `WebGpuProver` runs proving and compression inside scoped `with_webgpu_hal` bindings for `rv32im`, `keccak`, and `recursion`, so synchronous prover code resolves to the initialized browser HAL. | Native prover construction resolves directly to CUDA circuit HALs when CUDA support is compiled and selected. |
| Guest execution | The guest executes in the browser WASM host executor. Execution is not the current bottleneck for small fixtures; `libm` execution measured 56ms. | The guest executes on the native host executor, then proof work is handed to CUDA-backed proving. |
| Segment proof core | Uses the browser `rv32im` circuit HAL plus WebGPU ZKP HAL operations. The circuit path is browser-compatible Rust/WASM for witness and accumulation, with portable check evaluation where WebGPU circuit kernels are not yet present. | Uses generated CUDA circuit kernels for `rv32im` witness generation, accumulation, and `eval_check`, plus CUDA ZKP HAL kernels. |
| Keccak proof core | Uses the browser `keccak` circuit HAL plus WebGPU ZKP HAL operations. Like `rv32im`, correctness is brought up through browser-compatible circuit code rather than generated CUDA kernels. | Uses generated CUDA Keccak witness and `eval_check` kernels plus CUDA ZKP HAL operations. |
| Recursion proof core | Uses the browser recursion circuit HAL for lift, join, resolve, union, and identity. The async `poseidon2_basic` run now keeps ZKP commit/finalize and combo prepare/divide GPU-authoritative. The interpreted WebGPU `eval_check` path runs for both rv32im and recursion; recursion `group2` is uploaded through bounded chunks because the full group exceeds browser buffer/binding limits. | Uses generated CUDA recursion witness, accumulation, and `eval_check` kernels. The focused native CUDA proof, including compression to succinct, completes in 426.538233ms in the latest run. |
| ZKP HAL buffers | `WebGpuBuffer` owns a real `GPUBuffer` and tracks CPU-dirty vs CPU-stale state. The async public path can keep successful WebGPU outputs GPU-owned and materialize CPU shadows with explicit async readback. | CUDA buffers are device-resident. Circuit and ZKP kernels operate on device pointers; host visibility is explicit and much less frequent. |
| ZKP HAL kernels | WebGPU kernels cover proof-critical bulk operations such as NTT, hashing, FRI fold, gather, scatter, mixing, copies, and prefix products. Current async diagnostics assert `cpu_only_ops == 0` and reduce CPU mirrors sharply, but some large storage-binding cases still fall back to CPU after explicit readback. | CUDA kernels are the authoritative execution path for bulk ZKP work, with no browser-style synchronous CPU shadow. |
| Transcript and verification | Existing receipt assembly and verifier logic are reused. Internal `verify_segment`, `verify_lift`, and `verify_composite` timings are milliseconds and are not the blocker. | Existing receipt assembly and verifier logic are reused. Verification work is also not the dominant cost. |
| Receipt compatibility | Produces standard succinct receipts that verify with the existing verifier for the passing browser cases. | Produces the native reference receipts used as the parity baseline. |

## Why The Browser Path Is CPU-Bound Today

The public async browser proof path is now GPU-authoritative for STARK
commit/finalize in the focused segment and recursion paths. The remaining gap
is no longer blanket CPU mirroring; it is explicit compatibility work:
batched transcript/Merkle query readbacks, fallback readbacks for buffers that
exceed the conservative WebGPU storage-binding limit, and portable WASM circuit
`eval_check`.

That explains the current diagnostics: `gpu_dispatches` is high,
`cpu_only_ops` is zero, and `cpu_mirrors` is much lower than dispatches. The
batched Merkle query path reduced the focused proof to 68 readbacks and 8 CPU
fallbacks, but circuit checks and host-visible transcript boundaries still
dominate wall time.

The second major gap is circuit-specific work. Native CUDA uses generated CUDA
kernels for `rv32im`, `keccak`, and especially `recursion` witness,
accumulation, and `eval_check`. The browser currently uses browser-compatible
Rust/WASM circuit paths and portable check evaluation where WebGPU circuit
kernels have not yet been implemented. The `libm` measurement points directly
at this: recursion `lift_prove` alone takes about 170.3 seconds in the browser,
with `recursion_eval_check` accounting for about 119.9 seconds. Native CUDA
finishes the entire succinct proof in about 436 milliseconds.

Keccak-heavy fixtures amplify the same gap. The smaller
`KeccakUnion(1)` diagnostic now proves and verifies as a Chrome/WebGPU
succinct receipt after the async Keccak receipt union fix and async WebGPU
Keccak subproof path, but it still takes 1372.79s versus a 7.495052818s native
CUDA baseline for the same 4 segments, 9 pending Keccak proofs, and 1
assumption. The async Keccak work reduced CPU mirrors from the prior 7659 to
216, and batched Merkle query readbacks reduced CPU fallbacks to 237 and
readbacks to 1314. The full `KeccakUnion(3)` fixture still exceeds the current
browser runner budget: the latest run progressed through all 11 RV32IM
segments and into the async Keccak/union proof tree before a 3600s
timeout/SIGKILL.

The portable `eval_check` loop now precomputes the zerofier inverse table for
the `INV_RATE` expanded-domain residues instead of recomputing powers and
inverses for every row. A follow-up reusable scratch path for the interpreted
`poly_ext` executor preserved correctness but stayed within noise by itself.
Precomputing the PolyExt constraint-compression mix powers once per
`eval_check` moved the focused `libm` browser proof from 228.71s to 214.08s.
The remaining cost is still dominated by generated/interpreted constraint
evaluation and CPU-shadow buffer traversal in WASM. A WebGPU circuit
`eval_check` hook now runs before the compatibility readback barrier, and a
tiny generated straight-line WGSL `poly_ext` regression passes in Chrome. The
generator now assigns fixed named scratch slots from `poly_ext` liveness;
measured liveness is about 1000 FP slots for recursion and 926 for rv32im. A
real recursion monolithic shader probe still lost the WebGPU device after
about 128 seconds, so large straight-line shaders remain disabled. The current
prototype instead uses a small interpreted WGSL program for large real
definitions. A recursion CPU-equivalence smoke test passes in Chrome, and the
latest focused proof verifies with `eval_check` recorded as 6 WebGPU dispatches
and 0 CPU fallbacks. The rv32im interpreter path is fast enough for the
focused segment proof. Recursion `group2` is still CPU-shadow sourced, but it
is uploaded in bounded chunks and evaluated on WebGPU instead of running the
portable recursion check.

A split recursion `eval_check` prototype is also not production-ready. It
showed that one recursion contribution needs 1603 FP dependencies, and the
slot-reusing split shader still lost the Chrome WebGPU device after 486.20s on
a `po2 = 0` reference-comparison smoke test. Real circuit `eval_check` now
needs GPU-resident recursion data, then smaller semantic staging or
circuit-specific kernels rather than larger straight-line WGSL chunks.

`mix_poly_coeffs` now has an async GPU-authoritative wrapper for STARK
finalize. The prior one-line enablement failed because a later CPU fallback
could read a stale CPU shadow after an earlier GPU accumulation. The async path
materializes the accumulated output before CPU fallback, and the focused
`poseidon2_basic` proof now records `mix_poly_coeffs` as 7 GPU dispatches and
1 CPU fallback while still producing a verified succinct receipt.

## GPU-Authoritative Refactor Status

The async/GPU-authoritative slice is implemented in the HAL, ZKP proof layers,
and focused public browser proving path:

- `WebGpuBuffer` now distinguishes CPU writes that need upload from GPU writes
  that make the CPU shadow stale.
- `WebGpuHal::gpu_authoritative_scope(true)` lets successful WebGPU kernels
  mark outputs GPU-owned without running the CPU mirror.
- `WebGpuBuffer::sync_gpu_to_cpu(...).await` is the explicit readback boundary
  before synchronous transcript or diagnostic reads.
- WebGPU-specific async variants now exist for Merkle tree construction,
  PolyGroup construction, FRI proving, STARK group commit, and STARK finalize.
- The rv32im, recursion, and keccak WebGPU circuit prover wrappers can attempt
  a WebGPU circuit `eval_check` hook before the portable CPU fallback. The
  currently generated real circuit definitions still exceed the conservative
  shader-size gate, but the hook removes the architectural blocker at the
  async finalize boundary.
- The rv32im and recursion WebGPU circuit prover wrappers use the async ZKP
  commit/finalize path while keeping existing CPU witness/accumulation phases
  outside GPU-authoritative mode.
- `WebGpuProver::prove_with_opts_async` has produced a verified succinct
  `multi_test/poseidon2_basic` receipt with both segment and recursion ZKP
  commit/finalize in GPU-authoritative mode.

The remaining refactor work is to extend this across the full parity matrix,
replace oversized-buffer CPU fallbacks with proof-correct tiled buffers or
chunked WebGPU kernels where possible, and make recursion/rv32im/keccak
`eval_check` shaders production-capable so the compatibility readback before
portable `eval_check` is skipped on real proofs. The current interpreted
`eval_check` prototype now proves correctness for rv32im and recursion in the
focused proof. Full GPU-authoritative recursion data still requires keeping
the recursion data group GPU-resident across stages; the current prototype
uses temporary chunk uploads, which is correct but doubles upload volume. A
production `gather_sample` chunking attempt,
including aligned source bindings and per-chunk command submission, passed
isolated HAL tests but failed the focused proof at `verify_lift`. A
recursion-sized production regression then showed all-zero output for a
512 MiB single source buffer (`rows = 2^20`, `cols = 128`). The HAL now gates
GPU allocation with `GPUSupportedLimits.maxBufferSize`, and the proof path
keeps the CPU fallback for oversized gather sources until a tiled multi-buffer
implementation replaces the single-buffer assumption.

## Optimization Priorities

1. Replace temporary recursion `group2` chunk uploads with a tiled/staged
   GPU-resident representation so recursion `eval_check`, gather, and related
   proof stages can share the same tiles without re-uploading 512 MiB per
   proof.
2. Replace CPU fallbacks for oversized storage bindings with proof-correct
   chunked WebGPU kernels or bounded staged/tiled buffers, especially hash
   rows, gather, NTT, and polynomial evaluation. `gather_sample` now has a
   recursion-sized fallback regression; production GPU re-enable requires a
   tiled source representation for matrices that exceed the browser/device
   single-buffer limit.
3. Add or port remaining WebGPU circuit kernels for `rv32im` and `keccak` after
   recursion, preserving the existing `CircuitHal` boundary.
4. Reduce synchronous transcript-driven buffer views and per-round host access
   in FRI, DEEP, and Merkle paths.
5. Batch small WebGPU dispatches and reuse pipelines/buffers aggressively; the
   current `libm` run performs 950 dispatches for a one-segment proof.
6. Keep native CUDA measurements beside every browser measurement so timeouts
   can be interpreted against known segment counts and expected proof scale.

## Relevant Code Paths

- Browser API and scoped HAL binding:
  `risc0/zkvm/src/host/client/prove/webgpu.rs`
- Browser prover orchestration and stage telemetry:
  `risc0/zkvm/src/host/server/prove/prover_impl.rs`
- WebGPU ZKP HAL and buffer diagnostics:
  `risc0/zkp/src/hal/webgpu.rs`
- Browser circuit HALs:
  `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`,
  `risc0/circuit/keccak/src/prove/hal/webgpu.rs`,
  `risc0/circuit/recursion/src/prove/hal/webgpu.rs`
- Native CUDA circuit HALs:
  `risc0/circuit/rv32im/src/prove/hal/cuda.rs`,
  `risc0/circuit/keccak/src/prove/hal/cuda.rs`,
  `risc0/circuit/recursion/src/prove/hal/cuda.rs`
- Native CUDA ZKP HAL:
  `risc0/zkp/src/hal/cuda.rs`
