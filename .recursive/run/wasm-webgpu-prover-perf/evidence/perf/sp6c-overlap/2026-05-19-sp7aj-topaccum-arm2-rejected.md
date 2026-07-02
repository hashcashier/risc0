# SP7aj: TopAccum arm2 additive candidate rejected

Date: 2026-05-19

## Candidate

After SP7ai was rejected as an upload-only sidequest, the next high-yield
candidate was a second RV32IM TopAccum generated arm. Arm2 was selected over
arm6 for the first bounded trial because the xgboost histogram shows similar
generated size, fewer inverse calls, and slightly more TopAccum cycles:

- arm2: 399,587 xgboost cycles, 25 `ext_inv` calls
- arm5: 430,888 xgboost cycles, 26 `ext_inv` calls
- arm6: 351,857 xgboost cycles, 29 `ext_inv` calls

The prototype kept the scope narrow:

- add explicit browser e2e gates for arm5+arm2
- generate the arm2 TopAccum WGSL slice at first use
- skip CPU TopAccum majors 5 and 2 only when the corresponding GPU candidate
  dispatches are enabled
- retain the existing GPU terminal-prefix and machine-column-carry path
- require candidate-sync waits so hidden queued GPU work is charged to wall time

## RED

The RED browser proof gate failed to compile before implementation, as intended:

```text
error[E0432]: unresolved imports
  risc0_circuit_rv32im::prove::accum_gpu_arm2_authoritative_dispatches
  risc0_circuit_rv32im::prove::set_accum_gpu_arm2_authoritative_enabled
```

Command:

```text
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  rv32im_accum_topaccum_arm5_arm2_authoritative_e2e_verify -- --nocapture
```

## GREEN correctness result

The prototype compiled after adding the narrow arm2 controls and dispatch path.
The representative browser proof gate passed receipt verification for BusyLoop
and KeccakUnion, with high WebGPU limits and no CPU fallback/CPU-only ops.

BusyLoop:

```text
browser-prove:metric prove_session_async wall_ms=129146.0 gpu_active_ms=126084.0 gpu_idle_ratio=0.024
browser-prove:stage done rv32im_accumulate step_top_accum_cpu_skip_major_mask0x0024 cycles=262144 elapsed_ms=1549.000
browser-prove:stage done rv32im_accumulate topaccum_arm5_authoritative cycles=40968 elapsed_ms=31.000
browser-prove:stage done rv32im_accumulate topaccum_arm2_authoritative cycles=25080 elapsed_ms=9.000
browser-prove:stage done rv32im_accumulate candidate_sync_wait elapsed_ms=122094.000
browser-prove:done multi_test/busy_loop_po2_18_topaccum_arm5_arm2_authoritative: segments=1 user_cycles=202872 total_cycles=262144
browser-prove:webgpu multi_test/busy_loop_po2_18_topaccum_arm5_arm2_authoritative: gpu_dispatches=328 raw_compute_dispatches=713 queue_submits=177 cpu_mirrors=10 cpu_fallbacks=0 cpu_only_ops=0 uploads=220 upload_bytes=504249156
```

KeccakUnion:

```text
browser-prove:metric prove_session_async wall_ms=101761.0 gpu_active_ms=69399.0 gpu_idle_ratio=0.318
browser-prove:stage done prove_session_async segments=4 pending_keccaks=9 assumptions=1 elapsed_ms=101761
browser-prove:done multi_test/keccak_union_topaccum_arm5_arm2_authoritative: segments=4 user_cycles=747265 total_cycles=917504
browser-prove:webgpu multi_test/keccak_union_topaccum_arm5_arm2_authoritative: gpu_dispatches=5896 raw_compute_dispatches=12633 queue_submits=3181 cpu_mirrors=175 cpu_fallbacks=0 cpu_only_ops=0 uploads=3760 upload_bytes=5910517800
```

## Decision

Rejected and reverted.

The correctness result is positive, but the performance result fails the user's
current priority: immediate significant wall-time reduction. The candidate-sync
gate exposed 122.094 seconds of hidden queued GPU compile/work on the BusyLoop
proof, moving the representative wall from the SP7ah baseline of 10.635 s to
129.146 s. KeccakUnion was correctness-positive and showed a tiny single-trial
wall movement versus SP7ah (102.226 s to 101.761 s), but it also increased raw
dispatches, queue submits, uploads, and upload bytes. That is not enough to
justify carrying a cold-start regression of this size.

xgboost was not run after BusyLoop failed the wall-time gate. No arm2 runtime
code or e2e tests are retained.

Follow-up constraint: do not add another TopAccum generated arm unless the
candidate first proves that pipeline creation and hidden queued work are either
prewarmed outside the measured proof path or bounded by a real-GPU sync gate.
