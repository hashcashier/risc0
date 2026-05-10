# WASM/WebGPU Prover Validation

Status: active validation; complete parity is not yet achieved.

This file records the current correctness matrix for the browser WebGPU prover.
Native baselines are measured first with the local CUDA prover and the same
browser-oriented segment and Keccak caps used by the wasm harness.

## Commands

Native CUDA baseline helpers:

```bash
RISC0_PROVER=local RISC0_EXECUTOR=local RISC0_INFO=1 RUST_LOG=info RISC0_PRINT_SEGMENTS=1 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml --release --features cuda \
  <native_stats_test> -- --ignored --nocapture
```

Browser Chrome/WebGPU harness:

```bash
WASM_BINDGEN_TEST_TIMEOUT=7200 \
WASM_BINDGEN_TEST_WEBDRIVER_JSON=webdriver.json \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
/home/rami/.cache/.wasm-pack/wasm-bindgen-c59d5019a2b42393/wasm-bindgen-test-runner \
  --nocapture \
  /home/rami/repos/risc0/examples/target/wasm32-unknown-unknown/release/deps/browser_prove-a7fada4366bfd68d.wasm \
  <browser_test_filter>
```

Build check:

```bash
cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release --no-run
```

The current build check passes and produces:

```text
/home/rami/repos/risc0/examples/target/wasm32-unknown-unknown/release/deps/browser_prove-a7fada4366bfd68d.wasm
```

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
| `groth16-verifier` | 914 segments, 180291710 user cycles, 239370240 total cycles, 462.439128141s | Browser run deferred until GPU-authoritative HAL work |
| `xgboost` | 11 segments, 2294908 user cycles, 2883584 total cycles, 5.702544275s | Browser run deferred until GPU-authoritative HAL work |
| `bn254` | 189 segments, 37989643 user cycles, 49348608 total cycles, 100.237964323s | Browser run deferred until GPU-authoritative HAL work |

## Internal Parity Matrix

| Fixture | Native CUDA baseline | Chrome/WebGPU status |
| --- | --- | --- |
| WebGPU HAL readback tests | CPU reference | Passed for NTT, inverse NTT, FRI fold, hashing, mixing, copy, gather, scatter, and prefix products |
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
| `multi_test/libm` | 1 segment, 3373 user cycles, 32768 total cycles, 413.333083ms | Passed in timed accelerator group, same cycles |
| `multi_test/poseidon2_basic` | 1 segment, 3598 user cycles, 32768 total cycles, 243.110394ms | Passed in timed accelerator group, same cycles |
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
| `multi_test/keccak_union` | 11 segments, 2230730 user cycles, 2752512 total cycles, 20.549581894s | Browser standalone run timed out after 7200s at fixture start; no succinct receipt yet |
| `risc0-zkvm-methods/bench/simple_loop` | 1 segment, 3300 user cycles, 32768 total cycles, 444.669138ms | Passed, same cycles |
| `risc0-zkvm-methods/test_feature` | 1 segment, 2933 user cycles, 32768 total cycles, 396.733057ms | Passed, same cycles |
| `risc0-zkvm-methods/blst` | 1028 segments, 229848040 user cycles, 269287424 total cycles, 523.487318958s | Browser run deferred until GPU-authoritative HAL work |
| `risc0-zkvm-methods/verify` | 1290 segments, 281347188 user cycles, 338034688 total cycles, 676.456094923s | Browser run deferred until GPU-authoritative HAL work |

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
`multi_test/keccak_union`. A standalone browser run for
`native_keccak_union_succinct_receipt_verify` also timed out after 7200s with
only the fixture start log, so this is not explained only by accumulated prior
fixtures in the combined test.

## Current Performance Blocker

The WebGPU HAL currently dispatches GPU kernels and validates GPU output in
readback tests, but the proof path still keeps a CPU mirror for every HAL
buffer. Normal browser proofs therefore run the proof-critical operations twice:
once on WebGPU and once in WASM/CPU to satisfy the synchronous `Hal::Buffer`
contract. The resulting proof is correct and WebGPU-dispatched, but practical
runtime is CPU-bound.

This is why moderate examples pass while very large examples and internal
fixtures are deferred. The next parity blocker is making proof buffers
GPU-authoritative and materializing CPU data only at explicit readback or
receipt-construction boundaries.

Additional hidden CPU work remains in the generic prover shape:

- `Hal::combos_prepare` and `Hal::combos_divide` are default CPU
  implementations unless the backend overrides them.
- Merkle, DEEP, and FRI transcript steps synchronously read buffer contents.
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
performance-blocked under the CPU-shadow proof path.
