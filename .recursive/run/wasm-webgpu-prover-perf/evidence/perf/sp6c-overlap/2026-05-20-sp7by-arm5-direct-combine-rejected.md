# SP7by: Combined Arm5 + Direct Accumulator Rejected

Date: 2026-05-20
Status: rejected

## Candidate

Combine the existing correctness-proven TopAccum arm5 authoritative offload with the accepted MISC0/MISC1/MISC2 direct accumulator path.

The candidate attempted two changes:

- Let the direct accumulator pass skip major 5 alongside MISC1/MISC2 and dispatch the existing arm5 authoritative kernels before MISC0/MISC1/MISC2 direct kernels.
- Change arm5 authoritative `accum` sync from dense `sync_cpu_to_gpu` to the sparse zero-default upload path, so combining arm5 would not reintroduce the old dense `source=accum` upload.

The goal was to remove more CPU `TopAccum` work without losing the SP7bx data-movement shape.

## Compile

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in `4m21s`.

## Representative E2E

Command:

```text
env ... cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: passed with verified BusyLoop and KeccakUnion receipts, high WebGPU limits, and zero fallback/CPU-only.

BusyLoop:

- `wall_ms=11343`
- `gpu_active_ms=7338`
- `gpu_idle_ratio=0.353`
- `raw_compute_dispatches=724`
- `queue_submits=178`
- `upload_bytes=200740324`
- `readback_bytes=478272`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- CPU accum skip mask: `step_top_accum_cpu_skip_replaced_misc0=true_major_mask=0x0026`
- arm5 rows: `topaccum_arm5_authoritative cycles=40968`
- arm5 inverse side-buffer items: `1065168`
- `commit_group_async rv32im_accum=3457 ms`

KeccakUnion:

- `wall_ms=102427`
- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`
- `gpu_active_ms=69354`
- `gpu_idle_ratio=0.323`
- `raw_compute_dispatches=12728`
- `queue_submits=3095`
- `upload_bytes=2982660728`
- `readback_bytes=10183944`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- arm5 cycle-list upload: `rv32im_accum_topaccum_arm5_authoritative_cycles uploads=4 upload_bytes=301904`

## Decision

Rejected before xgboost.

Correctness was clean, but the short representative workload failed the wall-time gate:

- Accepted SP7bx BusyLoop: `8230` / `8237 ms`
- Candidate BusyLoop: `11343 ms`
- Regression: about `+3.1 s`, roughly `+38%`

The hidden cost surfaced exactly where prior generated-arm attempts warned it would: not in the small arm5 dispatch timer (`29 ms`), but at the next proof synchronization point (`commit_group_async rv32im_accum=3457 ms`). Running xgboost after that would spend another proof run on a candidate already known to harm a representative workload.

The candidate code/test changes were reverted. Post-revert compile passed:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in `4m23s`.

`git diff --check` remained clean.

## Follow-Up

Do not combine generated TopAccum arm5 with the direct accumulator path as an immediate wall-time lever. The sparse upload reduction was not enough to overcome generated-arm GPU cost. Future generated-arm work needs a smaller real-GPU sync gate before full proof e2e and must clear BusyLoop before xgboost.
