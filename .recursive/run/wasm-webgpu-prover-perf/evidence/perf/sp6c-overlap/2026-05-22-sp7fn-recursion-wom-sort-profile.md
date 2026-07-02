# SP7fn recursion WOM sort profile

## Purpose

Decide whether production GPU-resident recursion WOM scatter must implement a full lexicographic sort, or whether the cheaper address-bucket scatter already proven in SP7fi-SP7fm can be used for representative proof workloads.

## Change

- Added an opt-in recursion WOM sort profile in the WebGPU recursion Rust-kernel path.
- Added profile assertions and metric summaries to the representative BusyLoop + KeccakUnion gate and the xgboost gate.
- The profile is disabled by default and does no row scan unless explicitly enabled by a test.

Profile tuple layout:

`[calls, rows, addr_groups, repeated_addr_groups, distinct_value_groups, distinct_value_rows, max_addr_group]`

## RED

Command:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture
```

Expected failure: the browser e2e test imported `set_recursion_wom_sort_profile_enabled` and `recursion_wom_sort_profile_snapshot` before the API existed.

Observed failure:

```text
no `recursion_wom_sort_profile_snapshot` in `prove`
no `set_recursion_wom_sort_profile_enabled` in `prove`
```

## GREEN / representative e2e proof generation

Command:

```bash
script -q -e -c "env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=300 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture" .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7fn-default-representative-wom-sort-profile.chrome.txt
```

Result:

- High WebGPU limits: `4294967292 / 2147483644 / 49152`.
- BusyLoop receipt verified: `wall_ms=5680`, `gpu_active_ms=4128`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- KeccakUnion receipt verified: `wall_ms=85030`, `gpu_active_ms=62871`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- Test passed: `1 passed; 0 failed; 164 filtered out; finished in 91.44s`.
- WOM sort profile: `calls=26`, `rows=38678651`, `addr_groups=12283601`, `repeated_addr_groups=11558970`, `distinct_value_groups=0`, `distinct_value_rows=0`, `max_addr_group=317993`.
- Recursion witgen sum: `7025 ms`.
- Recursion data sparse upload: `1592493192 bytes`.

Command:

```bash
script -q -e -c "env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=300 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture" .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7fn-xgboost-wom-sort-profile.chrome.txt
```

Result:

- High WebGPU limits: `4294967292 / 2147483644 / 49152`.
- xgboost receipt verified: `wall_ms=64104`, `gpu_active_ms=47684`, `gpu_idle_ratio=0.256`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- Test passed: `1 passed; 0 failed; 164 filtered out; finished in 64.66s`.
- WOM sort profile: `calls=21`, `rows=28520256`, `addr_groups=9128218`, `repeated_addr_groups=8487866`, `distinct_value_groups=0`, `distinct_value_rows=0`, `max_addr_group=317959`.
- Recursion witgen sum: `5416 ms`.
- Recursion data sparse upload: `1168626960 bytes`.

## Decision

For BusyLoop, KeccakUnion, and xgboost representative proof workloads, same-address WOM groups repeat heavily but never contain distinct values. That means a production GPU path can proceed with address-bucket scatter as the next candidate instead of first building a full lexicographic GPU sort.

This does not by itself reduce wall time. The expected gain remains in the next production step: skip CPU recursion verify_mem and avoid uploading sorted WOM rows by generating/scattering/backfilling rows on GPU.

## Next implementation shape

The next candidate should be guarded and measured:

- Split WebGPU recursion witness into CPU `exec` only plus post-zeroize GPU WOM row generation/scatter/backfill/verify_mem.
- Keep rows GPU-resident.
- Prefer GPU-generated bucket/cycle metadata; if using compact CPU metadata temporarily, measure it separately and reject if it recreates SP7fd/SP7ff's upload behavior.
- Gate with BusyLoop + KeccakUnion + xgboost e2e proof generation and zero fallback/CPU-only.

