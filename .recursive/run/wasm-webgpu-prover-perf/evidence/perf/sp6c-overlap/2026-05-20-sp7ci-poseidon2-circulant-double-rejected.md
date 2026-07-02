# SP7ci - Poseidon2 circulant double-add candidate rejected

Date: 2026-05-20

## Candidate

SP7ch proved that the hot xgboost Merkle bucket is almost entirely `hash_rows rows=65536 cols=64` (`27360 ms` across 32 calls), not fold-chain work. This candidate tried a small arithmetic improvement inside the shared Poseidon2 WGSL:

- Replace `mul(MONT_TWO, x)` with `add(x, x)`.
- Replace `mul(MONT_FOUR, x)` with two modular doublings.

The substitution is algebraically valid for Montgomery-represented field elements, and would avoid four full Montgomery reductions per 4x4 circulant call if Dawn/Tint did not already optimize this pattern.

## Compile

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result with candidate:

- Passed.
- Elapsed: `4m57s`.

## Representative e2e

Command output: `/tmp/sp7ci-busy-keccak-poseidon2-circulant.log`

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result:

- `test tests::iter6d_g_replace_busy_loop_e2e_verify ... ok`
- `test result: ok. 1 passed; 0 failed; 137 filtered out; finished in 109.07s`
- Correctness clean for both BusyLoop and KeccakUnion receipts.
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

BusyLoop:

- `wall_ms=8547`
- `gpu_active_ms=3955`
- `gpu_idle_ratio=0.537`
- `raw_compute_dispatches=721`
- `queue_submits=173`
- `upload_bytes=209748256`
- `readback_bytes=478272`

KeccakUnion:

- `wall_ms=100217`
- `gpu_active_ms=68599`
- `gpu_idle_ratio=0.315`
- `raw_compute_dispatches=12716`
- `queue_submits=3075`
- `upload_bytes=2991727684`
- `readback_bytes=10183944`

Hot Merkle samples:

- Candidate `fri_prove round=0 merkle_new rows=65536 cols=64`: `25677 ms` across 30 BusyLoop+KeccakUnion samples.
- SP7cg baseline for the same command/log shape: `25674 ms` across 30 samples.

## Comparison

Compared with accepted SP7cg:

- BusyLoop wall: `8072 -> 8547 ms` (`+475 ms`, about `+5.9%`).
- KeccakUnion wall: `100588 -> 100217 ms` (`-371 ms`, about `-0.4%`).
- Hot Merkle samples: `25674 -> 25677 ms`, effectively unchanged.
- Dispatch and queue-submit counts unchanged.

## Decision

Rejected before xgboost. Correctness was clean, but the target kernel did not improve and BusyLoop regressed materially. The candidate was reverted.

Post-revert compile:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result:

- Passed.
- Elapsed: `4m55s`.

Follow-up: do not spend more near-term time on scalar Poseidon2 algebra substitutions unless a focused shader-level measurement shows an actual `hash_rows` improvement. The next plausible Merkle win needs deeper row-hash parallelization or a different leaf-hash strategy.
