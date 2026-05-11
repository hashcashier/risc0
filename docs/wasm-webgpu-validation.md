# WASM/WebGPU Prover Validation

Status: active validation; complete parity is not yet achieved.

This file records the current correctness matrix for the browser WebGPU prover.
Native baselines are measured first with the local CUDA prover and the same
browser-oriented segment and Keccak caps used by the wasm harness.
The native CUDA and browser WebGPU proving paths are compared in
`docs/wasm-webgpu-cuda-comparison.md`.

## Commands

Native CUDA baseline helpers:

```bash
RECURSION_SRC_PATH=/home/rami/repos/risc0/examples/target/release/build/risc0-circuit-recursion-4e96382f0d1db440/out/recursion_zkr.zip \
RISC0_PROVER=local RISC0_EXECUTOR=local RISC0_INFO=1 RUST_LOG=info RISC0_PRINT_SEGMENTS=1 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml --release --features cuda \
  <native_stats_test> -- --ignored --nocapture
```

Browser Chrome/WebGPU harness, run from `examples/browser-prove` so the
checked-in `webdriver.json` is discovered:

```bash
WASM_BINDGEN_TEST_TIMEOUT=7200 \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
/home/rami/.cache/.wasm-pack/wasm-bindgen-c59d5019a2b42393/wasm-bindgen-test-runner \
  --nocapture \
  /home/rami/repos/risc0/examples/target/wasm32-unknown-unknown/release/deps/browser_prove-c5784704c29e61f5.wasm \
  <browser_test_filter>
```

Running the same command from the repository root misses `webdriver.json`; on
the current Chrome/ChromeDriver pair that launches without the WebGPU flags and
fails immediately with `GPUAdapter is not available`.

Targeted browser harness build check:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release --no-run
```

The current targeted browser harness build check passes. The latest focused
release test-artifact rebuild after adding the narrow async `gather_sample`
readback fallback took 9m34s and produces:

```text
/home/rami/repos/risc0/examples/target/wasm32-unknown-unknown/release/deps/browser_prove-c5784704c29e61f5.wasm
```

The broader examples workspace wasm build is not treated as the browser prover
gate; it currently reaches a native-terminal dependency path through
`crossterm` when built wholesale for `wasm32-unknown-unknown`.

## Public Example Matrix

All passed browser entries produced succinct receipts and verified with the
existing verifier. Browser diagnostics asserted at least one WebGPU dispatch and
zero CPU-only HAL operations.

| Guest | Native CUDA baseline | Chrome/WebGPU status |
| --- | --- | --- |
| `risc0-zkvm-methods/cfg` | 1 segment, 2261 user cycles, 32768 total cycles, 390.942019ms | Passed, same cycles |
| `hello-world` | 1 segment, 3532 user cycles, 32768 total cycles, 407.65863ms | Passed, same cycles |
| `json` | 1 segment, 13311 user cycles, 65536 total cycles, 416.113284ms | Passed, same cycles |
| `chess` | 1 segment, 22500 user cycles, 131072 total cycles, 503.676967ms | Passed, same cycles |
| `composition/multiply-assumption` | 1 segment, 3532 user cycles, 32768 total cycles, 410.961683ms | Passed, same cycles |
| `composition` | 1 segment, 13887 user cycles, 65536 total cycles, 428.996087ms | Passed, same cycles |
| `jwt-validator` | 2 segments, 160287 user cycles, 294912 total cycles, 992.560791ms | Passed, same cycles |
| `bevy` | 1 segment, 35838 user cycles, 131072 total cycles, 483.551339ms | Passed, same cycles |
| `digital-signature` | 1 segment, 7632 user cycles, 32768 total cycles, 420.762145ms | Passed, same cycles |
| `prorata` | 2 segments, 283348 user cycles, 393216 total cycles, 1.004217399s | Passed, same cycles |
| `wasm` | 1 segment, 64511 user cycles, 262144 total cycles, 592.912258ms | Passed, same cycles |
| `password-checker` | 1 segment, 34524 user cycles, 65536 total cycles, 418.088959ms | Passed, same cycles |
| `voting-machine/init` | 1 segment, 4705 user cycles, 32768 total cycles, 421.164178ms | Passed, same cycles |
| `voting-machine/submit` | 1 segment, 7635 user cycles, 32768 total cycles, 216.841948ms | Passed, same cycles |
| `voting-machine/freeze` | 1 segment, 7085 user cycles, 32768 total cycles, 211.629892ms | Passed, same cycles |
| `keccak` | 1 segment, 9590 user cycles, 65536 total cycles, 1.080891734s with `keccak_max_po2=14` | Passed, same cycles |
| `smartcore-ml` | 2 segments, 371804 user cycles, 524288 total cycles, 1.063480365s | Passed, same cycles |
| `wordle` | 1 segment, 7654 user cycles, 65536 total cycles, 409.913345ms | Passed, same cycles |
| `sha/hash` | 1 segment, 5321 user cycles, 32768 total cycles, 392.888719ms | Passed, same cycles |
| `sha/hash-rust-crypto` | 1 segment, 5965 user cycles, 65536 total cycles, 239.484618ms | Passed, same cycles |
| `c-guest/host` | 1 segment, 2393 user cycles, 32768 total cycles, 390.206884ms | Passed, same cycles |
| `waldo` | 1 segment, 56825 user cycles, 131072 total cycles, 450.70735ms | Passed, same cycles |
| `ecdsa/k256` | 2 segments, 343611 user cycles, 524288 total cycles, 1.08389501s | Passed, same cycles |
| `ecdsa/p256` | 2 segments, 232373 user cycles, 327680 total cycles, 978.112464ms | Passed, same cycles |
| `groth16-verifier` | 914 segments, 180291710 user cycles, 239370240 total cycles, 462.439128141s | Browser run deferred until remaining eval_check/readback and oversized-buffer bottlenecks are addressed |
| `xgboost` | 11 segments, 2294908 user cycles, 2883584 total cycles, 5.702544275s | Browser run deferred until remaining eval_check/readback and oversized-buffer bottlenecks are addressed |
| `bn254` | 189 segments, 37989643 user cycles, 49348608 total cycles, 100.237964323s | Browser run deferred until remaining eval_check/readback and oversized-buffer bottlenecks are addressed |

## Internal Parity Matrix

| Fixture | Native CUDA baseline | Chrome/WebGPU status |
| --- | --- | --- |
| WebGPU HAL readback tests | CPU reference | Passed for NTT, inverse NTT, FRI fold, hashing, mixing, copy, gather, scatter, prefix products, proof-shaped hash/NTT/bit-reverse dimensions, generated `poly_ext` `eval_check`, and GPU-authoritative async readback |
| `multi_test/do_nothing` composite plus compress | Composite: 1 segment, 3290 user cycles, 32768 total cycles, 263.060284ms; compress: 144.935682ms | Passed in API/compression group |
| `multi_test/echo` | 1 segment, 5784 user cycles, 32768 total cycles, 237.884784ms | Passed, same cycles |
| `multi_test/sha_cycle_count` | 1 segment, 3752 user cycles, 32768 total cycles, 234.860164ms | Passed, same cycles |
| `multi_test/read_write_mem` | 1 segment, 4016 user cycles, 32768 total cycles, 238.043293ms | Passed, same cycles; output captured from guest stdout |
| `multi_test/pause_resume` | Native execution-only: 1 segment, 3476 user cycles, 26.518041ms, `Paused(7)` | Passed as execution-mode coverage, not a succinct receipt |
| `multi_test/syscall` | 1 segment, 4281 user cycles, 32768 total cycles, 409.948531ms | Passed, same cycles |
| `multi_test/syscall_words` | 1 segment, 3487 user cycles, 32768 total cycles, 238.669801ms | Passed, same cycles |
| `multi_test/sys_input` | 1 segment, 4909 user cycles, 32768 total cycles, 234.739582ms | Passed integrity verification |
| `multi_test/sys_read` | 1 segment, 8471 user cycles, 65536 total cycles, 258.619384ms | Passed, same cycles |
| `multi_test/echo_stdout` | 1 segment, 4203 user cycles, 32768 total cycles, 236.010871ms | Passed, same cycles |
| `multi_test/echo_words` | 1 segment, 3996 user cycles, 32768 total cycles, 236.157902ms | Passed, same cycles |
| `multi_test/libm` | 1 segment, 3373 user cycles, 32768 total cycles, 436.441617ms | Passed standalone in 214.08s with succinct receipt; `rv32im_eval_check` 32.546s, `segment_prove_core` 43.592s, `recursion_eval_check` 119.937s, `lift_prove` 170.293s |
| `multi_test/poseidon2_basic` | 1 segment, 3598 user cycles, 32768 total cycles, 426.538233ms in the latest focused CUDA run | Passed via `prove_with_opts_async` with a succinct receipt in Chrome in 30.42s, same cycles; rv32im and recursion STARK commit/finalize plus combo prepare/divide ran in GPU-authoritative mode; `eval_check` recorded 6 WebGPU dispatches and 0 CPU fallbacks |
| `multi_test/poseidon2_short` | 1 segment, 3596 user cycles, 32768 total cycles, 235.5053ms | Passed in timed accelerator group, same cycles |
| `multi_test/poseidon2_long` | 1 segment, 3800 user cycles, 32768 total cycles, 238.775893ms | Passed in timed accelerator group, same cycles |
| `multi_test/poseidon2_continue` | 1 segment, 3835 user cycles, 32768 total cycles, 237.068497ms | Passed in timed accelerator group, same cycles |
| `multi_test/sha_conforms` | 1 segment, 61122 user cycles, 131072 total cycles, 296.37097ms | Passed in timed accelerator group, same cycles |
| `multi_test/rsa_compat` | 408 segments, 91535675 user cycles, 106758144 total cycles, 210.946054043s | Browser timed out after preceding pre-RSA fixtures; no succinct receipt yet |
| `multi_test/do_random` | 1 segment, 27956 user cycles, 65536 total cycles, 260.290704ms | Passed in timed post-RSA split, same cycles |
| `multi_test/aligned_alloc` | 1 segment, 3306 user cycles, 32768 total cycles, 234.941701ms | Passed in timed post-RSA split, same cycles |
| `multi_test/alloc_zeroed` | 1 segment, 6387 user cycles, 32768 total cycles, 238.588112ms | Passed in timed post-RSA split, same cycles |
| `multi_test/keccak_update` | 1 segment, 7916 user cycles, 65536 total cycles, 896.55495ms | Passed in timed post-RSA split, same cycles |
| `multi_test/sha_single_keccak` | 1 segment, 5348 user cycles, 65536 total cycles, 259.547386ms | Passed in timed post-RSA split, same cycles |
| `multi_test/sys_keccak` | 1 segment, 6423 user cycles, 32768 total cycles, 242.303054ms | Passed in timed post-RSA split, same cycles |
| `multi_test/sha_digest` | 1 segment, 5259 user cycles, 32768 total cycles, 236.749607ms | Passed in timed post-RSA split, same cycles |
| `multi_test/sha_digest_iter` | 1 segment, 14238 user cycles, 65536 total cycles, 255.627994ms | Passed in timed post-RSA split, same cycles |
| `multi_test/bigint` | 1 segment, 6913 user cycles, 65536 total cycles, 257.070061ms | Passed in timed post-RSA split, same cycles |
| `multi_test/bigint_raw` | 1 segment, 3854 user cycles, 32768 total cycles, 238.714152ms | Passed in timed post-RSA split, same cycles |
| `multi_test/keccak_update2` | 1 segment, 8054 user cycles, 65536 total cycles, 856.791179ms | Passed in timed post-RSA split, same cycles |
| `multi_test/keccak_union_small` (`KeccakUnion(1)`) | 4 segments, 747310 user cycles, 917504 total cycles, 7.495052818s in the latest focused CUDA run | Passed as standalone Chrome/WebGPU succinct proof in 1372.79s with 9 pending Keccak proofs and 1 assumption; same cycles; `cpu_only_ops=0` |
| `multi_test/keccak_union` (`KeccakUnion(3)`) | 11 segments, 2230730 user cycles, 2752512 total cycles, 20.682250251s | Browser standalone run progressed through all 11 RV32IM segments and at least 12 of 25 Keccak proof requests after the async union fix, then timed out/SIGKILLed after 3600s before producing a succinct receipt |
| `risc0-zkvm-methods/bench/simple_loop` | 1 segment, 3300 user cycles, 32768 total cycles, 444.669138ms | Passed, same cycles |
| `risc0-zkvm-methods/test_feature` | 1 segment, 2933 user cycles, 32768 total cycles, 396.733057ms | Passed, same cycles |
| `risc0-zkvm-methods/blst` | 1028 segments, 229848040 user cycles, 269287424 total cycles, 523.487318958s | Browser run deferred until remaining eval_check/readback and oversized-buffer bottlenecks are addressed |
| `risc0-zkvm-methods/verify` | 1290 segments, 281347188 user cycles, 338034688 total cycles, 676.456094923s | Browser run deferred until remaining eval_check/readback and oversized-buffer bottlenecks are addressed |

## Accelerator/Precompile Run Notes

The CUDA baseline command for `native_accelerator_and_precompile_stats` passed
in 237.31s with `RISC0_PROVER=local RISC0_EXECUTOR=local RISC0_INFO=1
RUST_LOG=info RISC0_PRINT_SEGMENTS=1` and `--features cuda`.

The full Chrome/WebGPU accelerator/precompile run used a 7200s timeout. It
proved and verified the pre-RSA fixtures through `multi_test/sha_conforms`,
then timed out after starting `multi_test/rsa_compat`.

The post-RSA split also used a 7200s timeout. It proved and verified
`multi_test/do_random`, allocation, SHA/Keccak digest/update, BigInt, raw
BigInt, and `multi_test/keccak_update2`, then timed out after starting
`multi_test/keccak_union`.

After replacing synchronous browser Keccak receipt unioning with async
`union_webgpu` calls and then moving the browser Keccak subproof path through
async WebGPU proving, a smaller standalone
`native_keccak_union_small_succinct_receipt_verify` run passed with a verified
succinct receipt:

```text
native CUDA: 7.495052818s, 4 segments, 747310 user cycles, 917504 total cycles
Chrome/WebGPU: 1372.79s, 4 segments, 747310 user cycles, 917504 total cycles
pending_keccaks=9 assumptions=1
gpu_dispatches=5913 cpu_mirrors=216 cpu_fallbacks=237 cpu_only_ops=0
uploads=14100 upload_bytes=33309357388
device_copies=152 device_copy_bytes=6076667392
readbacks=1314 readback_bytes=4383929880
buffers=15203 buffer_bytes=51838041536
eval_check gpu_dispatches=153 cpu_fallbacks=9
```

A narrow async `gather_sample` readback fallback now has focused Chrome
coverage via `webgpu_hal_oversized_async_gather_reads_only_sample`; it reads
only the sampled elements when an oversized source is GPU-owned but cannot be
bound as storage. The latest `KeccakUnion(1)` proof no longer reports
`gather_sample` fallbacks because FRI query openings now follow the existing
prover's transcript order while batching WebGPU Merkle value and sibling
readbacks per tree.

The full `native_keccak_union_succinct_receipt_verify` fixture remains
performance-blocked. In the latest focused run it progressed through all 11
RV32IM segments and into the async Keccak/union proof tree before the 3600s
browser runner limit killed the process.

## Current Performance Blocker

The public async proof path now uses GPU-authoritative ZKP commit/finalize for
rv32im segment proving and recursion lift. The focused Chrome test
`native_poseidon2_basic_async_succinct_receipt_verify` passed with a verified
succinct receipt in 30.42s after a 426.538233ms native CUDA baseline and
recorded:

```text
gpu_dispatches=303 cpu_mirrors=12 cpu_fallbacks=8 cpu_only_ops=0
uploads=725 upload_bytes=1042124836
device_copies=8 device_copy_bytes=212208640
readbacks=68 readback_bytes=122424608
buffers=795 buffer_bytes=1862479004
```

This is a material reduction from the earlier CPU-mirrored recursion fallback,
but the browser path remains CPU-bound. The dominant costs are still portable
temporary chunk uploads for oversized recursion data, explicit readbacks for
transcript and Merkle query materialization, and remaining oversized-buffer
fallbacks outside `eval_check`. The same focused run spent 995ms in
`segment_prove_core_async` and 29.223s in `lift_prove_async`.

Large examples and internal fixtures remain deferred until the remaining CPU
fallbacks and portable circuit checks are replaced by WebGPU-authoritative
paths or bounded chunked GPU kernels. A WebGPU circuit `eval_check` hook now
runs before fallback. The tiny generated `poly_ext` regression passes in
Chrome, and the new interpreted WebGPU `eval_check` prototype passes the
recursion CPU-equivalence smoke test. The latest focused proof verifies with
`eval_check` recorded as 6 GPU dispatches and 0 CPU fallbacks, so the interpreter
is correctness-positive for the focused rv32im and recursion paths. The rv32im path now
dispatches through the interpreter with
`domain=131072`, `instructions=20202`, `fp_slots=927`, and `mix_slots=29`. The
recursion data group is `536870912` bytes, above the `125829120` byte
conservative storage-binding cap, so the current prototype uploads it through
five bounded chunks and dispatches the interpreter for each chunk.

A split recursion `eval_check` prototype was tried after that. Naive 64-term
chunks lost the WebGPU device; a dependency-budgeted variant showed that one
recursion contribution still needs 1603 FP dependencies; and a slot-reusing
split shader still lost the device after 486.20s in Chrome on a `po2 = 0`
portable-reference smoke test. The split path is disabled, and the next
recursion-sized `eval_check` proof attempt needs the recursion data group to
stay GPU-resident instead of relying on larger generated straight-line
shaders.

`mix_poly_coeffs` now has an async GPU-authoritative path for STARK finalize.
The earlier one-line enablement failed because a later CPU fallback could read
a stale CPU shadow after an earlier GPU accumulation. The async path reads the
accumulated output back before CPU fallback. A dedicated forced-dispatch HAL
regression passes in Chrome, and the focused proof now records
`mix_poly_coeffs` as 7 GPU dispatches and 1 CPU fallback.

`combos_prepare` and `combos_divide` now have WebGPU kernels and async
GPU-authoritative wrappers. A focused Chrome HAL regression passes, and the
focused proof records 2 `combos_prepare` and 2 `combos_divide` GPU dispatches
with no CPU mirrors or fallbacks. This removes the explicit readback between
GPU-authoritative combo mixing and combo division. A later FRI/Merkle query
batching pass reduced the focused proof further to 68 readbacks and
122424608 readback bytes while preserving receipt verification.

A production `gather_sample` chunking attempt was tested with aligned source
bindings, a conservative chunk-width cap, per-chunk command submission, sliced
dirty-buffer uploads, and an oversized proof-shaped gather. It passed those
focused HAL regressions, but failed the full proof at `verify_lift`. A
recursion-sized production regression with `rows = 2^20` and `cols = 128`
then showed all-zero GPU output for the 512 MiB source matrix, indicating that
chunked bindings are not sufficient when the underlying single `GPUBuffer`
itself exceeds the browser/device's usable buffer size. The HAL now gates GPU
allocation with `GPUSupportedLimits.maxBufferSize`, and the normal proof path
therefore still uses the known-correct async CPU fallback for oversized
`gather_sample` sources. The chunked helper remains available for smaller
single-buffer diagnosis, and
`webgpu_hal_recursion_sized_gather_sample_falls_back_to_cpu` locks the current
correct fallback behavior until a tiled multi-buffer implementation exists.

Additional hidden CPU work remains in the generic prover shape:

- Merkle, DEEP, and FRI transcript steps still force explicit readbacks for
  transcript materialization and sampled query openings.
- The current browser circuit HALs use generated Rust/WASM bridges for
  correctness and still materialize witness/check data on CPU.

## Remaining Coverage

The following browser groups still need CUDA baselines and/or WebGPU execution
after the CPU-shadow bottleneck is removed:

- `native_zkvm_method_guests_succinct_receipts_verify`
- `native_assumption_continuation_povw_and_guest_error_receipts_verify`
- `bigint2_precompile_guest_succinct_receipts_verify`
- the deferred public examples and large internal fixtures listed above

`RunUnconstrained { unconstrained: true }` is excluded from active browser
proving parity in this checkout because the native syscall table has `SYS_FORK`
registration disabled and the corresponding native proving test is ignored.

The accelerator/precompile native CUDA baseline now passes end-to-end. The
previous `multi_test/poseidon2_basic` CUDA failure was a byte-address versus
word-address ABI mismatch in the Poseidon2 syscall path. Browser execution has
proved and verified every accelerator/precompile fixture except
`multi_test/rsa_compat` and `multi_test/keccak_union`, which remain
performance-blocked under the current browser proof path.

The latest focused `multi_test/poseidon2_basic` async browser run shows the
immediate blocker more precisely: browser witness generation and accumulation
are sub-second for both `rv32im` and recursion, and rv32im `eval_check` now
uses the interpreted WebGPU path. Recursion `eval_check` also runs on WebGPU
by chunk-uploading the oversized data group, cutting the focused proof to
30.42 seconds. The next optimization targets are replacing temporary chunk
uploads with proof-correct tiled GPU-resident buffers, expanding this path to
larger examples and Keccak-heavy fixtures, and reducing the remaining
transcript-driven readbacks.
