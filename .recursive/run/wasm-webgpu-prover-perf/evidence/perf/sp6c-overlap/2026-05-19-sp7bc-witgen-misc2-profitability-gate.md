# SP7bc GPU-witgen MISC2 profitability gate

Date: 2026-05-19

## Goal

Keep GPU-witgen replacement on the immediately profitable MISC0 slice and exclude MISC2 from production replacement until its CPU-shadow repair cost is removed or made GPU-resident.

MISC2 remains available for diff/probe/candidate work. This is a profitability gate, not a correctness rollback.

## RED

Test change:

- Tightened replacement shadow-readback caps for BusyLoop, KeccakUnion, and xgboost.
- Added a representative assertion that MISC2 must not be present in the replacement mask.

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Expected failure observed after BusyLoop receipt generation and verification:

- Current replacement still used `mask=0x0005`, `dispatched_arms=[0, 2]`.
- MISC2 extra kernels were compiled/dispatched for minors `2..=6`.
- `iter6d_g_pre_witgen_dispatch_async elapsed_ms=7631`.
- `prove_session_async wall_ms=15206`.
- `source=witgen_data_shadow_rows readback_bytes=49265896`, above the new MISC0-only cap of `40000000`.
- `compute_pipeline_creations=44`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.

## GREEN implementation

Changed `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`:

- Changed `WITGEN_REPLACE_SUPPORTED_ARM_MASK` from arms `[0, 2]` to arm `[0]`.
- Changed `WITGEN_REPLACE_SUPPORTED_ARMS` from `[0, 2]` to `[0]`.
- Added an inline comment documenting that MISC2 is correctness-positive but currently wall-negative.
- Made `ready_witgen_replace_mask` skip unsupported arms directly.
- Made replacement-mode on-demand prewarm skip unsupported arms so MISC2 does not compile just because the preflight contains MISC2 cycles.

Changed `examples/browser-prove/src/lib.rs`:

- BusyLoop replacement sparse readback cap: `55_000_000 -> 40_000_000`.
- KeccakUnion replacement sparse readback cap: `105_000_000 -> 75_000_000`.
- xgboost replacement sparse readback cap: `680_000_000 -> 450_000_000`.
- Added the MISC2 mask exclusion assertion in the representative BusyLoop proof gate.

## Verification

Formatted:

```text
cargo fmt
```

Representative BusyLoop + KeccakUnion browser proof:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: PASS, `test result: ok`, finished in `114.30s`.

BusyLoop po2=18:

- Receipt verified.
- `mask=0x0001`, `dispatched_arms=[0]`.
- `iter6d_g_prewarm ALL arms requested=1`, `chunk1 ALL requested=1`.
- `iter6d_g_pre_witgen_dispatch_async elapsed_ms=3355`.
- `prove_session_async wall_ms=10778`, `gpu_active_ms=4021`, `gpu_idle_ratio=0.627`.
- `segments=1`, `user_cycles=202872`, `total_cycles=262144`.
- `gpu_dispatches=329`, `raw_compute_dispatches=716`, `queue_submits=172`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.
- `compute_pipeline_creations=36`.
- Total `readback_bytes=37670192`, below the new BusyLoop sparse-shadow cap.

KeccakUnion(1):

- Receipt path verified.
- Representative shape retained: `segments=4`, `pending_keccaks=9`, `assumptions=1`.
- `prove_session_async wall_ms=103241`, `gpu_active_ms=69524`, `gpu_idle_ratio=0.327`.
- `gpu_dispatches=5900`, `raw_compute_dispatches=12646`, `queue_submits=3021`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.
- `compute_pipeline_creations=1` after BusyLoop warmed the pipelines.
- `source=witgen_data_shadow_rows readbacks=8 readback_bytes=67004128`, below the new KeccakUnion cap.
- `source=data upload_bytes=3930062848`.

Representative xgboost browser proof:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: PASS, `test result: ok`, finished in `102.21s`.

xgboost:

- Receipt journal assertion passed: `30.528042544062632`.
- `mask=0x0001`, `dispatched_arms=[0]`.
- `prove_session_async wall_ms=101985`, `gpu_active_ms=56675`, `gpu_idle_ratio=0.444`.
- `segments=11`, `user_cycles=2294946`, `total_cycles=2883584`.
- `gpu_dispatches=5260`, `raw_compute_dispatches=11391`, `queue_submits=2687`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.
- `compute_pipeline_creations=36`.
- `source=witgen_data_shadow_rows readbacks=22 readback_bytes=407004560`, below the new xgboost cap.
- `source=data upload_bytes=2818572288`.

## Interpretation

Correctness:

- Representative e2e proof generation passed for BusyLoop po2=18, KeccakUnion(1), and xgboost.
- Receipt verification/journal assertions passed.
- No CPU fallback or CPU-only HAL operations were observed.
- The replacement mask is now MISC0-only in production replacement mode.

Performance:

- Versus the same-session RED with MISC2 still active, BusyLoop first-proof wall improved `15206 -> 10778` ms and pre-witgen dispatch improved `7631 -> 3355` ms.
- First-proof compute pipeline creations dropped to `36`, because MISC2 replacement pipelines no longer compile on demand.
- MISC2 sparse shadow repair is removed from the production replacement path: xgboost `witgen_data_shadow_rows` is back in the MISC0-only range (`407004560`) instead of the MISC2 range from SP7bb (`582212568`).
- This does not yet beat the SP7an default xgboost mean (`99692` ms), so accepted production wall-time gain remains `0`.
- The current tree also contains the sparse-zeroize upload experiment, so `source=data upload_bytes` should not be compared directly with SP7bb as a pure MISC2-gate effect.

## Next target

Stop broadening top-level opcode replacement until a candidate can pass replacement-diff and beat the default representative wall matrix. The immediate structural GPU-witgen target remains eliminating the final dense `source=data` upload / CPU-shadow dependency, or moving enough downstream accumulation consumption GPU-resident that the upload is unnecessary.
