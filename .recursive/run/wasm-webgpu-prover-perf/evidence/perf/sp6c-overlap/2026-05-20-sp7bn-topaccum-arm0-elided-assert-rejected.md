# SP7bn TopAccum arm0 elided-assert candidate rejected

Date: 2026-05-20

## Scope

Evaluate a lower-complexity generated TopAccum major-0 authoritative path after SP7bl rejected the full generated arm0 batch-inverse path. This candidate stripped generated no-op `eqz` / `eqz_ext` assertion calls before WGSL compilation, then split `ext_inv` through the existing capture / inverse-buffer / consume pattern.

## RED

Added a wasm browser e2e gate for `rv32im_accum_topaccum_arm0_elided_assert_authoritative_e2e_verify`.

Initial RED compile failed as expected because the public control/metric APIs did not exist:

```text
error[E0432]: unresolved imports
risc0_circuit_rv32im::prove::accum_gpu_arm0_authoritative_dispatches
risc0_circuit_rv32im::prove::set_accum_gpu_arm0_authoritative_enabled
```

## Candidate

The candidate added an opt-in arm0 authoritative flag/API, generated arm0 kernels from the TopAccum slice, removed no-op assertion calls, replaced `25` generated `ext_inv` calls with capture/consume adapters, skipped CPU major-0 accumulation, and skipped the `witgen_accum_shadow_rows` bridge while the GPU owned those rows.

Compile gate passed:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm0_elided_assert_authoritative_e2e_verify --no-run

Finished release profile in 4m21s
```

## E2E Rejection

Real browser proof generation rejected the candidate before receipt verification on the first BusyLoop proof:

```text
wall_ms=53499
rv32im_witgen_accum_shadow_gpu_sync skipped=topaccum_arm0_authoritative
rv32im_accumulate topaccum_arm0_elided_assert_authoritative dispatched cycles=77196 inv_items=1929900
rv32im_accumulate topaccum_arm0_elided_assert_authoritative elapsed_ms=38
commit_group_async rv32im_accum elapsed_ms=49335
cpu_fallbacks=0 cpu_only_ops=0
AbortError: Failed to execute 'mapAsync' on 'GPUBuffer': A valid external Instance reference no longer exists.
```

This is the same class of GPU/device-capacity failure as prior full-arm0 attempts, not a CPU fallback or correctness-preserving speedup. Accepted wall-time gain: `0`.

## Cleanup Verification

The unvalidated arm0 runtime/test code was removed. Stale runtime search after cleanup leaves no runnable arm0 path:

```text
rg -n "arm0|ARM0|topaccum_replace_ext_inv_calls|strip_wgsl_noop_assert_calls|TOPACCUM_STEPS_PRUNED|ACCUM_GPU_ARM0" \
  examples/browser-prove/src/lib.rs \
  risc0/circuit/rv32im/src/prove/hal/webgpu.rs \
  risc0/circuit/rv32im/src/prove/mod.rs \
  risc0/circuit/rv32im/src/prove/wgsl_pruner.rs

risc0/circuit/rv32im/src/prove/wgsl_pruner.rs:1494: fixture string contains lookup_TopInstResultLayout_arm0(...)
risc0/circuit/rv32im/src/prove/hal/webgpu.rs:688: fn topaccum_replace_ext_inv_calls(...)
risc0/circuit/rv32im/src/prove/hal/webgpu.rs:709: topaccum_arm5_replace_ext_inv_calls uses the generic replacement helper
```

Post-cleanup compile gate passed:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify --no-run

Finished release profile in 4m25s
```

Post-cleanup representative e2e proof gates passed with real browser receipt verification:

```text
iter6d_g_replace_busy_loop_e2e_verify
BusyLoop wall_ms=9194
KeccakUnion wall_ms=104977 segments=4 pending_keccaks=9 assumptions=1
KeccakUnion witgen_accum_shadow_rows=48433004
cpu_fallbacks=0 cpu_only_ops=0
test result: ok, finished in 114.49s
```

```text
iter6d_g_replace_xgboost
xgboost wall_ms=99818
segments=11
journal=30.528042544062632
witgen_accum_shadow_rows=294892052
readback_bytes=302380644
cpu_fallbacks=0 cpu_only_ops=0
test result: ok, finished in 100.06s
```

## Decision

Reject. Do not retry generated TopAccum arm0 by deleting no-op assertions or reshuffling inverse plumbing. Any future accumulator work should be narrower than full generated arm0 and must pass BusyLoop, KeccakUnion, and xgboost e2e proof generation before acceptance.
