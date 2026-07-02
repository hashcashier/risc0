# SP7fy current drain attribution diagnostic

Date: 2026-05-22

## Purpose

Re-check the current SP7fr working-state hot buckets after recursion witness submit batching, using the default-off explicit drain diagnostic. This is diagnostic-only evidence; no production runtime change is accepted here.

The temporary default-test toggles were removed after collecting the logs. The persistent diagnostic remains the existing default-off `set_poly_group_drain_diagnostic_enabled` hook and focused coverage.

## Representative proof validation

Default representative BusyLoop + KeccakUnion was run with the drain diagnostic enabled:

- Evidence: `2026-05-22-sp7fy-drain-attribution-representative.chrome.txt`.
- Result: passed under high WebGPU limits; BusyLoop and KeccakUnion receipts verified.
- Runtime: `92.05s`.
- BusyLoop: `wall_ms=5999`, `gpu_active_ms=4520`, `raw_compute_dispatches=613`, `queue_submits=168`, `cpu_fallbacks=0`, `cpu_only_ops=0`.
- KeccakUnion: `wall_ms=84662`, `gpu_active_ms=65056`, `raw_compute_dispatches=10900`, `queue_submits=3153`, `cpu_fallbacks=0`, `cpu_only_ops=0`.

Representative drain aggregates:

- BusyLoop: `fri_prove round=0 drain_after_expand_evaluate_ntt` summed `1712 ms`; `poly_group check drain_after_batch_expand_into_evaluate_ntt` summed `739 ms`; the first `poly_group code` drain was `596 ms` but overlapped replacement prewarm, so it is not a clean steady-state attribution point.
- KeccakUnion: `fri_prove round=0 drain_after_expand_evaluate_ntt domain=1048576` summed `23718 ms`; check-group expand drains summed `23009 ms` at the `65536` domain and `4424 ms` at the `1048576` domain. Recursion commit buckets were much smaller: `commit_group_async recursion_accum=3167 ms`, `recursion_data=1937 ms`, `recursion_ctrl=1047 ms`.

## xgboost proof validation

Default xgboost was run with the drain diagnostic enabled:

- Evidence: `2026-05-22-sp7fy-drain-attribution-xgboost.chrome.txt`.
- Result: passed under high WebGPU limits; succinct receipt and journal `30.528042544062632` verified.
- Runtime: `65.02s`.
- xgboost: `wall_ms=64065`, `gpu_active_ms=49728`, `raw_compute_dispatches=9918`, `queue_submits=2803`, `cpu_fallbacks=0`, `cpu_only_ops=0`.

xgboost drain aggregates:

- `fri_prove round=0 drain_after_expand_evaluate_ntt domain=1048576`: `27242 ms` over `32` calls.
- `poly_group check drain_after_batch_expand_into_evaluate_ntt count=16 size=262144 domain=1048576`: `9143 ms` over `32` calls.
- `commit_group_async recursion_accum`: `2658 ms` over `21` calls.
- `poly_group accum drain_after_batch_expand_into_evaluate_ntt count=12`: `2124 ms` over `21` calls.
- `commit_group_async recursion_data`: `1622 ms` over `21` calls.
- `commit_group_async rv32im_data`: `1398 ms` over `11` calls.
- `commit_group_async rv32im_accum`: `1302 ms` over `11` calls.
- `commit_group_async recursion_ctrl`: `1178 ms` over `21` calls.

## Interpretation

The current hot-bucket diagnosis remains stable after SP7fr: the dominant remaining queues are FRI round-0 and check-group `batch_expand_into_evaluate_ntt`, not Merkle root/top readback payloads. The readback-looking timings are queue drains.

Immediate small cleanup in recursion witness submission/upload paths is now mostly exhausted. The next material win likely needs either:

- a memory-pass-reducing `batch_expand_into_evaluate_ntt` redesign with a proof-shaped shader benchmark before e2e, or
- a larger recursion exec/WOM offload that removes CPU plan/bucket work without reintroducing SP7fd/SP7ff sorted-row uploads.

Expected remaining improvement from the accepted SP7fr state:

- Low-risk cleanup: `<3%` on xgboost unless it hits one of the listed hot buckets.
- Credible next win: `8-15%` if FRI/check expansion work is reduced materially.
- Larger upside: `15-25%` remains possible, but only through a deeper NTT/FRI redesign or a complete GPU-resident recursion exec path. It is not credible from submit-count-only or readback-order changes.
