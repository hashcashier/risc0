# SP7bb GPU-witgen replacement prewarm scope

Date: 2026-05-19

## Goal

Keep GPU-witgen work focused on already-correct replacement arms and stop compiling unsupported replacement-arm pipelines during first-proof prewarm.

This is intentionally narrower than broadening witgen coverage: MISC0/MISC2 replacement is the current correctness-positive surface. The next structural target remains the dense final `source=data` upload.

## RED

Test change:

- Added `assert_witgen_replacement_pipeline_scope(...)` in `examples/browser-prove/src/lib.rs`.
- Applied it to `iter6d_g_replace_busy_loop_e2e_verify` and `iter6d_g_replace_xgboost` with `max_compute_pipeline_creations=52`.

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Expected failure observed:

- BusyLoop proof and receipt verification completed before the assertion.
- `compute_pipeline_creations=60 > max_compute_pipeline_creations=52`.
- Logs showed unsupported prewarm work: MISC1, MUL0, DIV0, MEM0, MEM1, CONTROL0, ECALL0, POSEIDON0/1, SHA0, BIGINT0, plus unsupported chunk1 zero-back arms.
- BusyLoop RED wall: `wall_ms=16909`, `cpu_fallbacks=0`, `cpu_only_ops=0`.

## GREEN implementation

Changed `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`:

- Added `WITGEN_REPLACE_SUPPORTED_ARM_MASK` and `WITGEN_REPLACE_SUPPORTED_ARMS = [0, 2]`.
- Restricted background `prewarm_witgen_kernel()` per-arm chunk0 and chunk1 compilation to supported replacement arms only.
- Kept diff-only candidate screening behavior intact through on-demand compilation.
- Made on-demand MISC0/MISC2 extra-minor compilation depend on the current `PreflightTrace` instead of compiling every possible extra minor.
- Replaced the hard-coded desired replacement mask with `WITGEN_REPLACE_SUPPORTED_ARM_MASK`.

## Verification

Compile gate:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: PASS in 4m25s, existing dead-code warnings only.

Representative BusyLoop + KeccakUnion browser proof:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: PASS, `test result: ok`, finished in 117.70s.

BusyLoop po2=18:

- `wall_ms=15214`, `gpu_active_ms=4121`, `gpu_idle_ratio=0.729`.
- `segments=1`, `user_cycles=202872`, `total_cycles=262144`.
- `gpu_dispatches=329`, `raw_compute_dispatches=724`, `queue_submits=182`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.
- `compute_pipeline_creations=43`, `compute_pipeline_cache_hits=93`.
- Prewarm scope logs: `iter6d_g_prewarm ALL arms requested=2`, `iter6d_g_prewarm chunk1 ALL requested=2`.

KeccakUnion(1):

- `wall_ms=102192`, `gpu_active_ms=69189`, `gpu_idle_ratio=0.323`.
- `segments=4`, `user_cycles=747265`, `total_cycles=917504`, `pending_keccaks=9`, `assumptions=1`.
- `gpu_dispatches=5900`, `raw_compute_dispatches=12677`, `queue_submits=3060`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.
- `source=data upload_bytes=4776263680`.
- `source=witgen_data_shadow_rows readbacks=16 readback_bytes=94777916`.
- `compute_pipeline_creations=1` after the BusyLoop proof warmed the cache.

Representative xgboost browser proof:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result: PASS, `test result: ok`, finished in 105.40s.

xgboost:

- Receipt journal assertion passed: `30.528042544062632`.
- `wall_ms=105182`, `gpu_active_ms=56892`, `gpu_idle_ratio=0.459`.
- `segments=11`, `user_cycles=2294946`, `total_cycles=2883584`.
- `gpu_dispatches=5260`, `raw_compute_dispatches=11479`, `queue_submits=2797`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.
- `source=data upload_bytes=5252317184`.
- `source=witgen_data_shadow_rows readbacks=44 readback_bytes=582212568`.
- `compute_pipeline_creations=43`, `compute_pipeline_cache_hits=1744`.
- Prewarm scope logs: `iter6d_g_prewarm ALL arms requested=2`, `iter6d_g_prewarm chunk1 ALL requested=2`.

Whitespace:

```text
git diff --check -- risc0/circuit/rv32im/src/prove/hal/webgpu.rs examples/browser-prove/src/lib.rs
```

Result: PASS.

## Interpretation

Correctness:

- Representative proof generation passed for BusyLoop po2=18, KeccakUnion(1), and xgboost.
- All three receipt paths verified with `cpu_fallbacks=0` and `cpu_only_ops=0`.

Performance:

- Deterministic first-proof pipeline waste reduction: BusyLoop RED `compute_pipeline_creations=60` -> GREEN `43` (`-17`, `-28.3%`).
- Unsupported background replacement-arm prewarm is gone from the accepted path.
- Observed wall movement in this single GREEN run was favorable for BusyLoop and xgboost versus the immediately prior SP7ba measurements, but KeccakUnion moved slightly upward. Treat the wall movement as directional/noisy, not an accepted structural reduction.
- Accepted material wall-time gain: 0 until repeat A/B proves it. Accepted deterministic gain: fewer first-proof pipeline creations and no unsupported replacement-arm prewarm.

## Next target

Do not broaden top-level opcode replacement opportunistically. Prior MISC1/MEM/etc. screening showed incomplete nested mux semantics. The highest-value GPU-witgen target is still eliminating or avoiding the dense final `source=data` upload, or generating chunk-complete kernels that can pass replacement-diff before e2e proof gating.
