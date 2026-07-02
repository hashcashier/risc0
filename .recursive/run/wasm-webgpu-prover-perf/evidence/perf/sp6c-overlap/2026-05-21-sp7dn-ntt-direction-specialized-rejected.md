# SP7dn NTT Direction-Specialized Step Shader Rejection

Date: 2026-05-21

## Candidate

Split the shared forward/inverse `NTT_STEP_WGSL` path into generated direction-specialized WGSL strings so the hot forward NTT path would not branch on `params.inverse`.

This was a small active-work candidate, not a memory-pass-reducing redesign.

## TDD Evidence

RED:

- Added browser test `webgpu_ntt_step_shaders_are_direction_specialized`.
- Command: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release webgpu_ntt_step_shaders_are_direction_specialized --no-run`
- Expected failure observed: `cannot find function ntt_step_shaders_are_direction_specialized in module risc0_zkp::hal::webgpu` at `browser-prove/src/lib.rs:799:37`.

GREEN:

- Added generated forward/inverse step shader helper and branch-free static test hook.
- Command: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release webgpu_ntt_step_shaders_are_direction_specialized -- --nocapture`
- Result: passed (`1 passed; 0 failed`), compile `4m52s`.

Functional parity:

- Command: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_ntt_gpu_results_match_cpu -- --nocapture`
- Result: passed after relaxing the exact bind-group assertion to `<= 4`.
- Note: this run negotiated low WebGPU limits (`1073741824 / 1073741824 / 32768`), so it is functional evidence only, not performance evidence.

## Representative E2E Evidence

Command:

```bash
script -q -e -c 'env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture' /tmp/sp7dn-ntt-direction-specialized-busy-keccak.log
```

Result: passed with verified receipts, high WebGPU limits, and zero CPU fallback/CPU-only.

Limits:

- `max_buffer_size=4294967292`
- `max_storage_buffer_binding_size=2147483644`
- `max_compute_workgroup_storage_size=49152`

BusyLoop:

- SP7dh baseline: `wall_ms=7217`, `gpu_active_ms=3198`, `raw_compute_dispatches=609`, `queue_submits=173`
- Candidate: `wall_ms=7284`, `gpu_active_ms=3232`, `raw_compute_dispatches=609`, `queue_submits=173`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- Movement: `+67 ms`

KeccakUnion:

- SP7dh baseline: `wall_ms=91900`, `gpu_active_ms=60170`, `raw_compute_dispatches=10660`, `queue_submits=3075`
- Candidate: `wall_ms=92618`, `segments=4`, `pending_keccaks=9`, `assumptions=1`, `gpu_active_ms=60529`, `raw_compute_dispatches=10660`, `queue_submits=3075`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- Movement: `+718 ms`

## Decision

Rejected and reverted before xgboost. BusyLoop and KeccakUnion already failed the representative wall gate, with KeccakUnion regressing by 718 ms. Spending another xgboost proof run would not be a good use of time.

Accepted wall-time gain: `0`.

Do not continue branch-only NTT shader specialization as an immediate lever. Future NTT work needs a real memory-pass-reducing design, or the run should pivot to another measured large bucket.

## Post-Revert Verification

- Marker sweep clean for `ntt_step_shaders_are_direction_specialized`, `NTT_STEP_WGSL_TEMPLATE`, `NTT_FORWARD_STEP_BODY`, `NTT_INVERSE_STEP_BODY`, `webgpu_ntt_step_forward_dynamic`, `webgpu_ntt_step_inverse_dynamic`, `ntt_step_wgsl`, and `OnceLock`.
- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`: passed.
- `cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml`: passed.
- `git diff --check`: passed.
- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies --no-run` from `examples/browser-prove`: passed after sandbox escalation for Cargo target-lock writes; compile `4m51s`.
