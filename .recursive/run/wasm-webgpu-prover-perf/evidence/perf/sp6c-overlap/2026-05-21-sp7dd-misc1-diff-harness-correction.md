# SP7dd MISC1 Diff Harness Correction

Date: 2026-05-21

## Scope

Corrected the candidate-major GPU-witgen diff screen before using it for new
MISC1 work.

## Finding

The first `iter6d_g_diff_busy_loop_misc1` attempt was invalid evidence. The
browser helper set `set_witgen_gpu_diff_major(Some(1))`, but `init_prover()`
constructs the default WebGPU prover and re-enables GPU-witgen replacement.
That turned the run into replacement-diff over the current supported MISC0 arm
instead of diff-only MISC1.

Invalid signal:

```text
iter6d_g diff=on fixture=busy_loop candidate_major=1
iter6d_g_pre_witgen_dispatch_async mask=0x0001 dispatched_arms=[0]
REPLACE_FINAL_SUMMARY ... mismatches=0
panic: REPLACE DIFF mode: 0 final data mismatches
```

## Fix

`run_witgen_diff_busy_loop_candidate_major` now explicitly disables replacement
and probe mode after `init_prover().await`, because `init_prover()` is the point
where default acceleration is re-enabled.

## Validation

Focused browser RED after the harness correction:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_diff_busy_loop_misc1 -- --nocapture
```

Result: failed for the correct reason, with high WebGPU limits and actual MISC1
candidate dispatch.

```text
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152
iter6d_g_pre_witgen_dispatch_async mask=0x0000 dispatched_arms=[1]
DIFF_SUMMARY total_cells=55312384 gpu_wrote=9009581 cpu_wrote=33114247 both_match=9009557 mismatches=24 gpu_only=0 cpu_only=24104666 candidate_cpu_only_nonzero=131099 rows=262144 cols=211
```

## Decision

Keep the harness correction. Do not reapply MISC1 combined-source-regs in this
iteration: SP7ct already proved that path correctness-clean and rejected it on
representative wall time due per-proof prewarm/compile overhead. MISC1
replacement is only worth revisiting with ahead-of-proof kernel readiness or a
smaller dispatch/list overhead design.

