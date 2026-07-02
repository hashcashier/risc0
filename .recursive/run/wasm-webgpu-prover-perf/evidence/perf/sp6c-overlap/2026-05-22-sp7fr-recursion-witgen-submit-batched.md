# SP7fr recursion witgen submit batching

Date: 2026-05-22

## Problem

SP7fq default-enabled the CPU-exec-only recursion witness path with GPU-resident WOM row generation, scatter/backfill, and generated `verify_mem`. It improved KeccakUnion and xgboost, but added command overhead and regressed the single-segment BusyLoop case:

- BusyLoop submits increased `167 -> 171`, and wall moved `5689 -> 6159 ms` relative to SP7fp.
- KeccakUnion submits increased `3128 -> 3228`.
- xgboost submits increased `2782 -> 2866`.

The SP7fq candidate issued recursion witness row kernels as one submitted sequence, then submitted scatter, backfill, and generated `verify_mem` separately. That made up to four queue submissions per recursion witness call.

## Change

Batch the recursion witness GPU row kernels, WOM scatter, optional WOM backfill, and generated `verify_mem` dispatch into one compute pass / one queue submission via `dispatch_compute_1d_bind_group_sequence`.

The dispatch count is unchanged. The accepted change only removes intermediate queue submissions for the SP7fq recursion witness GPU verify_mem path.

## TDD evidence

RED:

- Added a focused BusyLoop candidate assertion that `queue_submits <= 168`.
- Browser proof run failed after proof generation with `queue_submits=171`, proving the test caught the SP7fq submit overhead.
- Evidence: `2026-05-22-sp7fr-red-busyloop-candidate-submit-cap.chrome.txt`.

GREEN:

- Batched row/scatter/backfill/verify dispatches into a single bind-group sequence.
- `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release --no-run recursion_witgen_gpu_verify_mem_candidate_busy_loop_e2e_verify` passed in `3m03s`.
- Focused BusyLoop candidate browser proof passed with verified receipt, `wall_ms=5896`, `gpu_active_ms=4396`, `raw_compute_dispatches=613`, `queue_submits=168`, `cpu_fallbacks=0`, and `cpu_only_ops=0`.
- Evidence: `2026-05-22-sp7fr-green-busyloop-candidate-submit-batched.chrome.txt`.

## Representative proof validation

Default representative BusyLoop + KeccakUnion:

- Command: `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture`
- Result: passed under high WebGPU limits; BusyLoop and KeccakUnion receipts verified.
- Runtime: `88.54s`.
- BusyLoop: `wall_ms=5962`, `gpu_active_ms=4484`, `raw_compute_dispatches=613`, `queue_submits=168`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- KeccakUnion(1): `wall_ms=81813`, `gpu_active_ms=62499`, `raw_compute_dispatches=10900`, `queue_submits=3153`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- Evidence: `2026-05-22-sp7fr-default-representative-recursion-witgen-submit-batched.chrome.txt`.

Default xgboost:

- Command: `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture`
- Result: passed under high WebGPU limits; succinct receipt and journal `30.528042544062632` verified.
- Runtime: `62.45s`.
- xgboost: `wall_ms=61883`, `gpu_active_ms=47718`, `raw_compute_dispatches=9918`, `queue_submits=2803`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- Evidence: `2026-05-22-sp7fr-default-xgboost-recursion-witgen-submit-batched.chrome.txt`.

## Measurements

Compared with SP7fq default:

| Workload | SP7fq wall | SP7fr wall | Movement | SP7fq submits | SP7fr submits |
|---|---:|---:|---:|---:|---:|
| BusyLoop | `6159 ms` | `5962 ms` | `-197 ms` | `171` | `168` |
| KeccakUnion(1) | `82208 ms` | `81813 ms` | `-395 ms` | `3228` | `3153` |
| BusyLoop + KeccakUnion test runtime | `89.15s` | `88.54s` | `-0.61s` | n/a | n/a |
| xgboost | `61841 ms` | `61883 ms` | `+42 ms` | `2866` | `2803` |
| xgboost test runtime | `62.46s` | `62.45s` | `-0.01s` | n/a | n/a |

Compared with SP7fp:

| Workload | SP7fp wall | SP7fr default wall | Movement |
|---|---:|---:|---:|
| BusyLoop | `5689 ms` | `5962 ms` | `+273 ms` |
| KeccakUnion(1) | `84903 ms` | `81813 ms` | `-3090 ms` |
| BusyLoop + KeccakUnion test runtime | `91.32s` | `88.54s` | `-2.78s` |
| xgboost | `65070 ms` | `61883 ms` | `-3187 ms` |
| xgboost test runtime | `65.67s` | `62.45s` | `-3.22s` |

Current accepted default state:

- BusyLoop: `wall_ms=5962`, `gpu_active_ms=4484`, `raw_compute_dispatches=613`, `queue_submits=168`.
- KeccakUnion(1): `wall_ms=81813`, `gpu_active_ms=62499`, `raw_compute_dispatches=10900`, `queue_submits=3153`.
- xgboost: `wall_ms=61883`, `gpu_active_ms=47718`, `raw_compute_dispatches=9918`, `queue_submits=2803`.

## Decision

Accept SP7fr. It gives deterministic queue-submit reduction with full e2e proof validation, improves the representative BusyLoop + KeccakUnion gate, and leaves xgboost flat within noise while still reducing submissions.

This exhausts the obvious low-risk submit batching in the SP7fq recursion witness GPU verify_mem path. Further material gains likely need a larger hot-bucket change, not another local submit cleanup.
