# SP7cg - WebGPU witness eqz elision

Date: 2026-05-20

## Candidate

SP7cc proved that generated `eqz!` assertion-expression evaluation was a measurable CPU cost in the remaining WebGPU direct-accum raw-step path. This candidate generalizes the guard name and applies it to the WebGPU RV32IM witness-generation path only:

- `rust_steps.rs`: `ACCUM_EQZ_ELIDED` renamed to generic `EQZ_ELIDED`; new `with_eqz_elided(...)`; existing `with_accum_eqz_elided(...)` retained as an alias for accepted accumulator call sites.
- `webgpu.rs`: `WebGpuCircuitHal::generate_witness(...)` wraps `super::rust_steps::generate_witness(...)` in `with_eqz_elided(...)`.

CPU/native witness generation remains unchanged. This does not alter generated witness arithmetic or GPU dispatches; it avoids evaluating generated assertion expressions while browser receipt verification remains the correctness gate.

## Compile

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result:

- Passed.
- Elapsed: `4m27s`.

## BusyLoop + KeccakUnion e2e

Command output: `/tmp/sp7cg-busy-keccak-eqz-witgen.log`

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result:

- `test tests::iter6d_g_replace_busy_loop_e2e_verify ... ok`
- `test result: ok. 1 passed; 0 failed; 137 filtered out; finished in 108.96s`
- Chrome WebGPU high limits: `max_buffer_size=4294967292`, `max_storage_buffer_binding_size=2147483644`, `max_compute_workgroup_storage_size=49152`.

BusyLoop:

- `wall_ms=8072`
- `gpu_active_ms=3987`
- `gpu_idle_ratio=0.506`
- `raw_compute_dispatches=721`
- `queue_submits=173`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `upload_bytes=209704004`
- `readback_bytes=478272`
- `rv32im_witgen elapsed_ms=361`
- `rv32im_accumulate step_top_accum_cpu_skip_replaced_misc0=true_major_mask=0x0006 elapsed_ms=1215`

KeccakUnion:

- `wall_ms=100588`
- `gpu_active_ms=68646`
- `gpu_idle_ratio=0.318`
- `raw_compute_dispatches=12716`
- `queue_submits=3075`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `upload_bytes=2991671768`
- `readback_bytes=10183944`
- RV32IM segment `rv32im_witgen` elapsed values: `1031`, `1050`, `1048`, `495` ms.
- RV32IM segment `step_top_accum_cpu_skip_replaced_misc0=true_major_mask=0x0006` elapsed values: `1236`, `1241`, `1249`, `563` ms.

Comparison vs SP7cc:

- BusyLoop wall: `8159 -> 8072 ms` (`-87 ms`, about `-1.1%`).
- KeccakUnion wall: `101378 -> 100588 ms` (`-790 ms`, about `-0.8%`).
- Dispatch/submission counts unchanged.

## xgboost e2e

Command output: `/tmp/sp7cg-xgboost-eqz-witgen.log`

Command:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_xgboost -- --nocapture
```

Result:

- `test tests::iter6d_g_replace_xgboost ... ok`
- `test result: ok. 1 passed; 0 failed; 137 filtered out; finished in 89.65s`
- `segments=11`, `pending_keccaks=0`, `assumptions=0`.
- `wall_ms=89418`
- `gpu_active_ms=56304`
- `gpu_idle_ratio=0.370`
- `raw_compute_dispatches=11466`
- `queue_submits=2718`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `upload_bytes=3104671440`
- `readback_bytes=7488592`

Target bucket comparison vs SP7cc:

- `rv32im_witgen` sum: `4419 -> 4047 ms` (`-372 ms`, about `-8.4%`).
- `rv32im_accumulate step_top_accum_cpu_skip_replaced_misc0=true_major_mask=0x0006` sum: `11256 -> 11146 ms` (`-110 ms`, about `-1.0%`).
- xgboost wall: `89769 -> 89418 ms` (`-351 ms`, about `-0.4%`).

Data movement and command counts:

- Raw dispatches unchanged: `11466`.
- Queue submits unchanged: `2718`.
- Readback bytes unchanged: `7488592`.
- Upload bytes changed `3104584972 -> 3104671440` (`+8868 bytes`, noise-level metadata drift).

## Decision

Accepted as a small, correctness-preserving CPU-side improvement. The performance gain is not a major wall-time lever, but all representative browser proof workloads passed receipt verification with zero fallback/CPU-only ops, and the targeted witness bucket improved consistently.

Current accepted xgboost working-state estimate moves from about `89.8-90.0 s` to about `89.4-89.6 s` on this browser/NVIDIA setup.
