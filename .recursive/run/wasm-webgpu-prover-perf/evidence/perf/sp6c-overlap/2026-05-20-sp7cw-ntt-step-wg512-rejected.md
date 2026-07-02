# SP7cw NTT Step 512-lane Workgroup - Rejected

Date: 2026-05-20

## Scope

Candidate changed the standalone `NTT_STEP_WGSL` shader from `@workgroup_size(256)` to `@workgroup_size(512)` and requested the corresponding browser WebGPU limits:

- `maxComputeInvocationsPerWorkgroup=512`.
- `maxComputeWorkgroupSizeX=512`.
- NTT-step workgroup counts used 512-lane division for forward NTT, inverse NTT, and fused inverse NTT remaining stages.

The goal was to reduce workgroup scheduling overhead in the current largest xgboost bucket: FRI / NTT work.

## Validation

Compile/hygiene before e2e:

- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`: pass.
- `git diff --check`: pass.
- `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`: pass, 4m51s.

Representative browser proof generation:

- Browser accepted the requested limits: `max_compute_invocations_per_workgroup=512`, `max_compute_workgroup_size_x=512`.
- BusyLoop failed before receipt acceptance: `verify segment index=0` / `verification indicates proof is invalid`.
- Counters at failure were still zero fallback/CPU-only: `cpu_fallbacks=0`, `cpu_only_ops=0`.
- Partial BusyLoop counters: `raw_compute_dispatches=309`, `queue_submits=89`, `upload_bytes=156411388`, `readback_bytes=255512`.
- The test exited nonzero before KeccakUnion and before xgboost.

Post-revert hygiene/compile:

- Reverted the 512-lane NTT shader, workgroup-count changes, and extra requested WebGPU limits back to the SP7cq 256-lane NTT-step shape.
- Marker search for `WEBGPU_NTT_STEP_WORKGROUP_SIZE`, `maxComputeInvocationsPerWorkgroup`, `maxComputeWorkgroupSizeX`, and `@workgroup_size(512)` found no source matches.
- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`: pass.
- `git diff --check`: pass.
- `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`: pass, 4m50s.

## Decision

Rejected and reverted as a correctness regression. This hit the verifier-failure stop condition before wall-time comparison, so no xgboost run was valid.

Current accepted working-state estimate remains SP7cq: BusyLoop `7.265s`, KeccakUnion `91.943s`, xgboost about `77.5s`.
