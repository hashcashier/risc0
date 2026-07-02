# SP7be/SP7bf witgen data attribution and shadow-readback elision

Date: 2026-05-19

## Goal

Keep moving the opt-in browser WebGPU witgen replacement path toward GPU-resident witness data without sacrificing proof correctness.

This slice had two parts:

- Split generic `data` diagnostics so recursion and direct Keccak data movement are attributed separately.
- Remove the MISC0 GPU-witgen replacement's `witgen_data_shadow_rows` GPU-to-CPU readback, then prove the resulting path end to end.

## RED

Attribution RED:

- Added representative assertions requiring recursion data uploads to be named `recursion_data`.
- The old diagnostics still attributed recursion data to generic `source=data`, so the new assertion failed.
- A first Keccak assertion for `keccak_data` was removed after the representative KeccakUnion path proved it exercises recursion uploads, not the direct Keccak prover data buffer.

Shadow-readback RED:

- Added `assert_witgen_shadow_readback_elided`.
- BusyLoop generated and verified a receipt, then failed the new assertion with:
  - `witgen_data_shadow_rows readbacks=2`
  - `witgen_data_shadow_rows readback_bytes=37191920`

First GREEN attempts caught correctness dependencies:

- Direct MISC0 replay initially used the current preflight row's `pc`, but `step_Top` uses the previous row's `next_pc`; failure:
  - `MISC0 direct replay instruction txn addr mismatch: got=0x30000001 expected=0x30000002`
- After fixing the PC source, the data commitment succeeded without the readback, but CPU `step_TopAccum` still consumed the host-side witness shadow:
  - `step_TopAccum failed at cycle=18320 major=0 minor=7`
- A post-commit CPU shadow replay then needed intentional overwrite semantics because the seeded host shadow could already contain stale zeroes:
  - `inconsistent set at row 18320, col 20: new=7 current=0`

## GREEN implementation

Changed attribution:

- `recursion_data` names recursion data buffers.
- `keccak_data` names direct Keccak data buffers.
- Sparse zeroize upload handling now recognizes `data`, `recursion_data`, and `keccak_data`.

Changed MISC0 replacement:

- Direct MISC0 lookup replay now derives the instruction PC/mode from the same previous-row inputs as `step_Top`.
- MISC0 replacement rows are no longer read back through `witgen_data_shadow_rows` before `generate_witness`.
- Added `repair_witgen_gpu_replace_shadow_for_accum`, a post-data-commit CPU shadow replay for only short-circuited rows. This feeds the existing CPU accumulation path without reintroducing a GPU readback or changing committed data authority.

The data commitment remains GPU-authoritative; the host replay is only for downstream accumulation's current CPU shadow dependency.

## Verification

Representative BusyLoop + KeccakUnion:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: PASS, `test result: ok`, finished in `114.15s`.

BusyLoop po2=18:

- Receipt verified.
- `wall_ms=10663`, `gpu_active_ms=3955`, `gpu_idle_ratio=0.629`.
- `rv32im_witgen elapsed_ms=401`.
- `rv32im_witgen_accum_shadow_replay rows=68239`, `elapsed_ms=170`.
- `readbacks=22`, `readback_bytes=478272`.
- Readback sources were normal proof/finalization sources; no `witgen_data_shadow_rows` source appeared.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.
- `recursion_data` upload source present.

KeccakUnion(1):

- Receipt path verified.
- `wall_ms=103174`, `gpu_active_ms=69593`, `gpu_idle_ratio=0.325`.
- Representative shape retained: `segments=4`, `pending_keccaks=9`, `assumptions=1`.
- `recursion_data uploads=200`, `upload_bytes=3355443200`.
- Readback sources were `final_coeffs`, `merkle_query`, `nodes`, and `out`; no `witgen_data_shadow_rows` source appeared.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.

Representative xgboost:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: PASS, `test result: ok`, finished in `103.16s`.

xgboost:

- Receipt journal assertion passed: `30.528042544062632`.
- `wall_ms=102938`, `gpu_active_ms=56637`, `gpu_idle_ratio=0.450`.
- `segments=11`, `user_cycles=2294946`, `total_cycles=2883584`.
- First segment shadow replay: `rows=73215`, `elapsed_ms=173`; later segments were similar (`~165-172 ms`).
- `recursion_data uploads=168`, `upload_bytes=2818572288`.
- Readback sources were `final_coeffs`, `merkle_query`, `nodes`, and `out`; no `witgen_data_shadow_rows` source appeared.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.

Additional checks:

```text
cargo fmt
git diff --check -- examples/browser-prove/src/lib.rs risc0/circuit/rv32im/src/prove/hal/rust_steps.rs risc0/circuit/rv32im/src/prove/hal/webgpu.rs risc0/zkp/src/hal/webgpu.rs risc0/circuit/recursion/src/prove/witgen.rs risc0/circuit/keccak/src/prove/hal/webgpu.rs
```

Result: PASS.

## Interpretation

Correctness:

- The representative browser proof matrix passed with real receipt verification: BusyLoop po2=18, KeccakUnion(1), and xgboost.
- No CPU fallback or CPU-only operations were introduced.
- The new test gate proves the old `witgen_data_shadow_rows` readback is gone for all representative workloads.

Performance:

- Deterministic data-movement improvement: xgboost removed the prior `witgen_data_shadow_rows` readback path entirely. The previous SP7bd replacement path still reported `witgen_data_shadow_rows=407004560` bytes for xgboost; this slice reports zero.
- Wall time did not improve materially:
  - BusyLoop `10600 -> 10663` ms versus SP7bd.
  - KeccakUnion `102887 -> 103174` ms versus SP7bd.
  - xgboost `101918 -> 102938` ms versus SP7bd.
- The new post-commit host shadow replay costs about `170 ms` per po2=18 RV32IM segment. That is much cheaper than the old readback in data-movement terms, but it is still CPU work and does not yet beat the previous wall baseline.

Accepted production wall-time gain remains `0`.

## Next target

The next immediate performance target should remove the CPU accumulation dependency that forced post-commit host shadow replay:

- either dispatch the matching TopAccum arm for GPU-owned MISC0 rows and skip those rows in CPU accumulation, or
- make the accumulation path consume GPU-owned witness data directly.

Do not expand more witgen opcode arms until this downstream consumer is GPU-resident enough to turn the data-movement reduction into wall-time reduction.
