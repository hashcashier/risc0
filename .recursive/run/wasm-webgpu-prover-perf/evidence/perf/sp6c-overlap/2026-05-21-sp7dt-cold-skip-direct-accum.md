# SP7dt - Cold-Skip Direct Accumulation

Date: 2026-05-21

## Scope

SP7dt keeps the SP7ds nonblocking first-segment behavior but restores MISC0/MEM0 direct accumulation when a cold segment skips GPU-witgen replacement. The witness is still CPU-generated for that skipped segment, but its data is already uploaded to the GPU before accumulation, so the grouped direct-accumulator kernel can safely cover the same MISC0/MEM0 lookup rows instead of forcing CPU TopAccum to process them.

## Changed Surface

- `risc0/circuit/rv32im/src/prove/hal/rust_steps.rs`
  - CPU TopAccum skip for direct MISC0/MEM0 accumulation now keys off the direct-accum minor predicates, not the GPU-witgen replacement mask.
- `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`
  - Direct-accum row collection now includes MISC0/MEM0 rows whenever direct accumulation is enabled, even if the current segment's GPU-witgen replacement mask is `0`.
- `examples/browser-prove/src/lib.rs`
  - Tightened the default BusyLoop gate: if cold replacement is skipped, MISC0 and MEM0 direct rows must still move.

## RED/GREEN Evidence

RED:

- Command: `cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture`
- Result: failed after BusyLoop proof generation and receipt verification, before KeccakUnion.
- Correct failure:
  - `GPU direct MISC0 accumulator should still cover CPU-witgen-owned BusyLoop rows when cold replacement is skipped`
- RED evidence showed the current cold skip path:
  - `nonblocking_pending_skip`
  - `mask=0x0000 no_ready_replacement no_sync`
  - CPU TopAccum `step_top_accum_cpu_skip_replaced_misc0=false_mem0=false_major_mask=0x0046`
  - Direct rows only for MISC1/MISC2/MEM1.

GREEN:

- Changed direct-row selection and CPU TopAccum skip semantics as described above.
- The same representative BusyLoop+KeccakUnion test passed.

## Representative E2E Results

All accepted runs negotiated high WebGPU limits:

- `max_buffer_size=4294967292`
- `max_storage_buffer_binding_size=2147483644`
- `max_compute_workgroup_storage_size=49152`

| Workload | Prior accepted SP7ds | SP7dt | Movement | Notes |
|---|---:|---:|---:|---|
| xgboost | `73.74s` | `73.39s` | `-0.35s` / `-0.5%` | Verified journal `30.528042544062632`; zero fallback/CPU-only assertions; cold segment now uses direct MISC0/MEM0 accumulation. |
| BusyLoop+KeccakUnion | `98.82s` | `99.31s` | `+0.49s` / `+0.5%` | Verified receipts; zero fallback/CPU-only assertions; full-wall movement treated as browser noise while segment-local work dropped materially. |

Segment-local effect:

- BusyLoop cold segment:
  - SP7ds: CPU TopAccum `false_mem0=false`, `rv32im_witgen_accum=1412ms`, `segment_prove_core_async=4955ms`.
  - SP7dt: CPU TopAccum `true_mem0=true`, `rv32im_witgen_accum=831ms`, `segment_prove_core_async=4285ms`.
  - Direct rows: MISC0 `68239`, MEM0 `40795`, plus existing MISC1/MISC2/MEM1.
- xgboost cold segment:
  - SP7ds: `rv32im_witgen_accum=1271ms`, `segment_prove_core_async=4737ms`.
  - SP7dt: `rv32im_witgen_accum=646ms`, `segment_prove_core_async=4245ms`.
  - Direct rows: MISC0 `73215`, MEM0 `40301`, plus existing MISC1/MISC2/MEM1.
- xgboost segment 1 still used warmed GPU-witgen replacement:
  - `mask=0x0021 dispatched_arms=[0, 5]`
  - `rv32im_witgen_accum=575ms`

## Decision

Accept as a narrow production improvement with conservative wall-time accounting.

Rationale:

- Correctness is covered by representative e2e proof generation on BusyLoop, KeccakUnion, and xgboost.
- The default gates enforce high WebGPU limits and zero `cpu_fallbacks` / `cpu_only_ops`.
- The change removes about `0.5-0.7s` of cold-segment CPU accumulator work.
- xgboost full-wall improved by `0.35s`; BusyLoop+KeccakUnion full-wall was flat/noisy at `+0.49s`.

Accepted production wall-time gain: `0.35s` on xgboost (`~0.5%`). Treat BusyLoop+KeccakUnion as flat/noisy, not a regression signal.

## Hygiene

- RED `iter6d_g_replace_busy_loop_e2e_verify -- --nocapture`: failed on the new MISC0 cold-skip direct-row assertion after verified BusyLoop proof.
- GREEN `iter6d_g_replace_busy_loop_e2e_verify -- --nocapture`: PASS, `99.31s`.
- GREEN `xgboost_succinct_receipt_verifies -- --nocapture`: PASS, `73.39s`.
- `cargo fmt --manifest-path examples/browser-prove/Cargo.toml --check`: PASS.
- `cargo fmt --manifest-path risc0/circuit/rv32im/Cargo.toml --check`: PASS.
- `git diff --check`: PASS.

## Next Target

Do not keep harvesting cold-start-only subsecond wins unless they fall out of the main path. The next material target remains GPU-witgen coverage for CPU-owned rows or a structural reduction in CPU-shadow/data upload work. Prior candidate screens rule out opportunistic broad MISC1/MISC2/MEM0 production replacement without a new correctness or shadow-repair reason.
