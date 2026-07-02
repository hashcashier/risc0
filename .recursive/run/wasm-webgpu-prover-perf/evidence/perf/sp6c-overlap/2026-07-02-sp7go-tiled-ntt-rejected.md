# SP7go tiled strided-local NTT: implemented, correct, REJECTED (wall-flat)

Date: 2026-07-02

## What was done

Implemented the SP7go runtime patch exactly per the 2026-05-22 checklist: SP7gn shader
as `BATCH_EXPAND_STRIDED_LOCAL_NTT_WGSL`, guard-matched candidate branch in
`dispatch_batch_expand_into_evaluate_ntt`, single-submission local-prefix + strided
sequence, 48-byte params. RED→GREEN focused parity passed bit-exactly on both hot
shapes (n_bits=16/count=16 and n_bits=20/count=4) in 1.10s. Full patch preserved at
`2026-07-02-sp7go-tiled-ntt-rejected.patch`.

## Gate results (vs the 2026-07-02 M0 baseline, robustness-on environment)

| Gate | Baseline | SP7go candidate | Movement |
|---|---:|---:|---:|
| Representative runtime | 82.12 s | 81.69 s | −0.5% (noise) |
| BusyLoop wall | 5479 ms | 5463 ms | flat |
| KeccakUnion wall | 75930 ms | 75511 ms | −0.55% |
| xgboost runtime | 58.05 s | 57.99 s | flat |
| xgboost wall | 57524 ms | 57462 ms | flat |

Candidate verified active: queue_submits dropped (168→164, 3153→3088, 2803→2739) and
marker assertions passed. All receipts verified, zero fallback/CPU-only. Rejected per
the checklist's own criterion: wall flat despite fewer dispatches.

## Root cause of the miss: drain-bucket misattribution

The SP7fy attribution drains measure ALL work queued since the previous drain point,
not the labeled operation. A finer-grained attribution pass (temporary drains between
finalize substages — retained default-off) decomposed the buckets:

| SP7fy label (May) | ms | Actual content (2026-07-02 fine attribution) | ms |
|---|---:|---|---:|
| `fri_prove round=0 drain_after_expand_evaluate_ntt` | 27242 | `finalize_async combos_divide` | 25963 |
| `poly_group check drain_after_batch_expand_into_evaluate_ntt` | 9143 | `finalize_async eval_check` (interpreter) | 8566 |
| — | | actual fri round-0 expand NTT | ~0 |
| — | | actual check expand NTT | 88 |

Napkin check that should have been run in May: the 10 global NTT passes the tiled
kernel eliminates move ~320 MiB at count=4 — sub-millisecond on a 5090, never 26 s.

## Consequences

1. SP7go rejected and reverted (patch + logs preserved in evidence).
2. The checklist's fallback (coalesced transpose / six-step NTT) is equally dead —
   there is no NTT time to recover. Do not revisit NTT redesigns for wall time.
3. The SP7fy "8–15% credible from FRI/check NTT" estimate is refuted.
4. The true dominant bucket, `combos_divide` (~26 s, 42% of xgboost wall), is a
   `workgroup_size(1)` kernel running a sequential 262144-iteration synthetic-division
   loop on one thread per combo chunk — ~11 threads total. Attacked in M1b (accepted,
   see `2026-07-02-m1b-parallel-combos-divide-accepted.md`).
5. Attribution discipline: a drain label names the wait point, not the workload.
   Finer default-off drains between finalize substages are now retained
   (`drain_after_eval_check`, `drain_after_check_interpolate`,
   `drain_after_mix_poly_coeffs`, `drain_after_combos_prepare`,
   `drain_after_combos_divide`, `drain_after_combos_sum`,
   `drain_after_final_bit_rev`).

Logs: `2026-07-02-sp7go-xgboost-drain-attribution-candidate-on.chrome.txt` (coarse,
candidate on), session-local `/tmp/attr_fine.log` (fine decomposition).
