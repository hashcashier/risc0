# SP7ck grouped NTT-step loop rejected

Date: 2026-05-20

## Purpose

SP7cj showed that the apparent xgboost Merkle row-hash cost was actually queued FRI round-0 `batch_expand_into_evaluate_ntt` work. SP7ck tested a bounded WebGPU NTT-step reshaping candidate: make the WGSL NTT step follow the Metal kernel's `(s, group, row)` loop shape so each invocation computes the twiddle for a starting `s` once and reuses it across multiple butterfly groups/rows, instead of computing `root^s` independently for every butterfly pair.

## Candidate

Temporary changes in `risc0/zkp/src/hal/webgpu.rs`:

- Added `WEBGPU_NTT_STEP_WORKGROUP_SIZE = 128` and `WEBGPU_NTT_STEP_GRID_BUDGET = 256`.
- Added `ntt_step_workgroups(...)`.
- Changed `NTT_STEP_WGSL` from a one-invocation-per-butterfly-pair kernel to a grouped loop kernel using `@builtin(num_workgroups)`.
- Routed forward and inverse NTT step dispatches through per-stage grouped workgroup dimensions.

## Commands

Compile before e2e:

```bash
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in `4m50s`.

Browser proof e2e:

```bash
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json __VK_LAYER_NV_optimus=NVIDIA_only __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver WASM_BINDGEN_TEST_TIMEOUT=420 cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture > /tmp/sp7ck-busy-keccak-ntt-step-grouped.log 2>&1
```

Post-revert hygiene:

```bash
rg -n "WEBGPU_NTT_STEP|ntt_step_workgroups|@compute @workgroup_size\\(128\\)|for \\(var s = gid.x" risc0/zkp/src/hal/webgpu.rs
git diff --check
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

## E2E Result

Log: `/tmp/sp7ck-busy-keccak-ntt-step-grouped.log`

- Test result: `ok. 1 passed; 0 failed; 0 ignored; 137 filtered out; finished in 113.65s`
- BusyLoop: `wall_ms=8425`, `gpu_active_ms=4339`, `gpu_idle_ratio=0.485`, `raw_compute_dispatches=721`, `queue_submits=173`, `cpu_fallbacks=0`, `cpu_only_ops=0`
- KeccakUnion: `wall_ms=104920`, `gpu_active_ms=73417`, `gpu_idle_ratio=0.300`, `raw_compute_dispatches=12716`, `queue_submits=3075`, `cpu_fallbacks=0`, `cpu_only_ops=0`

Comparison against SP7cg accepted baseline:

```text
BusyLoop    8072 -> 8425 ms   +353 ms  +4.4%
KeccakUnion 100588 -> 104920 ms +4332 ms +4.3%
```

## Decision

Rejected before xgboost. Correctness was clean, and the candidate did not introduce fallback or CPU-only execution, but it failed the representative wall-time gate on both BusyLoop and KeccakUnion. The likely issue is that the grouped-loop shape reduced twiddle exponentiation but under-parallelized or worsened memory scheduling enough to lose wall time in Chrome/Dawn/NVIDIA.

## Cleanup

- Candidate code was reverted.
- Marker search found no remaining `WEBGPU_NTT_STEP`, `ntt_step_workgroups`, grouped `@workgroup_size(128)`, or grouped `for (var s = gid.x ...)` source.
- `git diff --check` passed.
- Post-revert compile passed in `4m57s`: `iter6d_g_replace_busy_loop_e2e_verify --no-run`.

## Follow-up

The corrected SP7cj target remains valid, but this specific grouped-loop implementation is not an immediate wall-time lever. Do not repeat this shape without a more parallel design or a focused browser shader benchmark that proves the NTT step itself improves before e2e.

