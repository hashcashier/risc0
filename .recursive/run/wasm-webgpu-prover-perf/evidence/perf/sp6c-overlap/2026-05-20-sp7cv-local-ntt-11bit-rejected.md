# SP7cv Local NTT 11-bit Block - Rejected

Date: 2026-05-20

## Scope

Candidate extended the accepted SP7cq fused local `batch_expand_into_evaluate_ntt` early-stage NTT from 10 fused bits to 11 fused bits:

- `FUSED_BITS: 10 -> 11`.
- `FUSED_BLOCK_SIZE: 1024 -> 2048`.
- WGSL workgroup scratch size `1024 -> 2048`.
- Rust `LOCAL_NTT_FUSED_BITS: 10 -> 11`.

The goal was to remove one more standalone NTT stage per fused batch-expand call while preserving the SP7cq cached-twiddle path for larger stages.

## Validation

Compile/hygiene before representative e2e:

- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`: pass.
- `git diff --check`: pass.
- `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`: pass, 4m52s.

Representative browser proof generation:

- BusyLoop: pass, receipt verified, `wall_ms=7271`, `gpu_active_ms=3227`, `gpu_idle_ratio=0.556`, `raw_compute_dispatches=595`, `queue_submits=173`, `upload_bytes=219389840`, `readback_bytes=478272`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- KeccakUnion: pass, receipt verified, `segments=4`, `pending_keccaks=9`, `assumptions=1`, `wall_ms=93296`, `gpu_active_ms=60465`, `gpu_idle_ratio=0.352`, `raw_compute_dispatches=10403`, `queue_submits=3074`, `upload_bytes=2996326432`, `readback_bytes=10183944`, `cpu_fallbacks=0`, `cpu_only_ops=0`.

Post-revert hygiene/compile:

- Reverted only the 11-bit local NTT constants back to SP7cq's 10-bit/1024-element fused block.
- `cargo fmt --check --manifest-path risc0/zkp/Cargo.toml`: pass.
- `git diff --check`: pass.
- `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run`: pass, 4m51s.

## Comparison

Accepted SP7cq baseline:

- BusyLoop: `wall_ms=7265`, `raw_compute_dispatches=609`, `queue_submits=173`.
- KeccakUnion: `wall_ms=91943`, `raw_compute_dispatches=10660`, `queue_submits=3075`.
- xgboost: `wall_ms=77417` / `77523`, mean `77470`.

SP7cv candidate:

- BusyLoop: `7265 -> 7271`, `+6 ms`, effectively flat; raw dispatches improved `609 -> 595`.
- KeccakUnion: `91943 -> 93296`, `+1353 ms` / `+1.47%`; raw dispatches improved `10660 -> 10403`, queue submits `3075 -> 3074`.

xgboost was not run because the representative BusyLoop + KeccakUnion wall gate already failed. The KeccakUnion regression is large enough that fewer raw dispatches are not useful evidence for this path.

## Decision

Rejected and reverted. The candidate was correctness-clean but regressed representative KeccakUnion wall time by about 1.35s while only reducing dispatch count. Current accepted working-state estimate remains SP7cq: BusyLoop `7.265s`, KeccakUnion `91.943s`, xgboost about `77.5s`.

Do not continue simple local NTT fused-bit/block-size tuning without a focused shader benchmark or a design that reduces memory traffic or active time, not just dispatch count.
