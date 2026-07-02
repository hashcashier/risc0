# SP7dm MISC1 Opportunistic Replacement Rejected

Date: 2026-05-21

## Candidate

Reintroduce the corrected MISC1 source-register replacement delta, but avoid the SP7ct first-proof prewarm regression by using MISC1 only opportunistically after its cached kernels are ready. MISC0 remained the only blocking production replacement arm.

Changed files during the candidate:

- `examples/browser-prove/src/lib.rs`
- `risc0/circuit/rv32im/src/prove/wgsl_pruner.rs`
- `risc0/circuit/rv32im/src/zirgen/exec_misc1_chunk0_delta.wgsl`
- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
- `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs`

## RED/GREEN

RED browser focused test:

- Command: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release misc1_replacement_delta_uses_combined_source_regs -- --nocapture`
- Workdir: `examples/browser-prove/`
- Result: failed as expected before implementation.
- Failure: `MISC1 replacement must cover distinct source registers before CPU witgen can be short-circuited`
- Compile time: `4m22s`

GREEN focused test:

- Same command from `examples/browser-prove/`
- Result: `test tests::misc1_replacement_delta_uses_combined_source_regs ... ok`
- Compile time: `4m26s`

Native `cargo test --manifest-path risc0/circuit/rv32im/Cargo.toml misc1_chunk0_delta_uses_combined_source_regs` initially ran zero tests because the `prove` feature was disabled. The `--features prove` harness failed on unrelated feature-dependency wiring, so the browser focused test was the valid RED/GREEN proof.

## Representative E2E Proofs

All representative proof runs used high Chrome/WebGPU limits:

- `max_buffer_size=4294967292`
- `max_storage_buffer_binding_size=2147483644`
- `max_compute_workgroup_storage_size=49152`

BusyLoop and KeccakUnion were run together:

- Test: `rv32im_default_representative_e2e_verify`
- Log: `/tmp/sp7dm-misc1-opportunistic-busy-keccak.log`
- Result: passed with verified receipts.

BusyLoop result:

- SP7dh baseline: `wall_ms=7217`
- SP7dm candidate: `wall_ms=7457`
- Delta: `+240 ms`
- `gpu_active_ms=3176`
- `raw_compute_dispatches=609`
- `queue_submits=173`
- `upload_bytes=219427132`
- `readback_bytes=478272`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

KeccakUnion result:

- SP7dh baseline: `wall_ms=91900`
- SP7dm candidate: `wall_ms=91851`
- Delta: `-49 ms`
- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`
- `gpu_active_ms=59993`
- `raw_compute_dispatches=10668`
- `queue_submits=3083`
- `upload_bytes=2996405632`
- `readback_bytes=10183944`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

xgboost was run because KeccakUnion was flat and the candidate might have paid off on repeated RV32IM segments:

- Test: `xgboost_succinct_receipt_verifies`
- Log: `/tmp/sp7dm-misc1-opportunistic-xgboost.log`
- Result: passed with verified receipt.
- SP7dh baseline: `wall_ms=78096`
- SP7dm candidate: `wall_ms=78591`
- Delta: `+495 ms`
- `segments=11`
- `gpu_active_ms=44864`
- `gpu_idle_ratio=0.429`
- `raw_compute_dispatches=9678`
- `queue_submits=2722`
- `upload_bytes=3113933436`
- `readback_bytes=7488592`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

The candidate did dispatch MISC1 direct accumulation rows, including xgboost rows per segment, but the e2e wall did not improve.

## Decision

Rejected and reverted.

The candidate was correctness-clean across BusyLoop, KeccakUnion, and xgboost, with zero CPU fallback and zero CPU-only operations. It failed the representative wall-time gate:

- BusyLoop regressed: `7217 -> 7457 ms`
- KeccakUnion moved by noise only: `91900 -> 91851 ms`
- xgboost regressed: `78096 -> 78591 ms`

It also increased representative dispatch/submission counts for the larger workloads:

- KeccakUnion: `raw_compute_dispatches 10660 -> 10668`, `queue_submits 3075 -> 3083`
- xgboost: `raw_compute_dispatches 9674 -> 9678`, `queue_submits 2718 -> 2722`

Accepted wall-time gain: 0.

## Revert Verification

Removed the temporary MISC1 opportunistic markers:

- `misc1_replacement_delta_uses_combined_source_regs`
- `misc1_chunk0_delta_uses_combined_source_regs`
- `exec_ReadSourceRegsChunk1`
- `merge_ReadSourceRegsStruct`
- `WITGEN_REPLACE_BLOCKING_ARM_MASK`
- `misc1_xori_replay_values`
- `replay_misc1_xori_lookup_deltas`
- `misc1_short_circuit_minor`
- MISC1 production short-circuiting in `cycle_short_circuited`

One unrelated existing MISC2 combined-delta test still references `exec_ReadSourceRegs_combined`; that is not part of SP7dm and was not reverted.

Post-revert checks:

- `cargo fmt --check --manifest-path risc0/circuit/rv32im/Cargo.toml`
- `cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml`
- `git diff --check`
- `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies --no-run`

The no-run compile required sandbox escalation for Cargo target-lock writes and passed in `4m28s`.

Do not continue MISC1 replacement as an immediate lever unless the extra kernels are prewarmed fully outside measured first-proof wall time or the MISC1 dispatches are fused into an already-paid replacement/accum path.
