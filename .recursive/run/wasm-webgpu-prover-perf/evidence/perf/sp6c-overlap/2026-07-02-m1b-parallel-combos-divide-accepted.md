# M1b: parallel-scan combos_divide ACCEPTED (−30% representative, −45% xgboost)

Date: 2026-07-02

## Problem

`COMBOS_DIVIDE_WGSL` ran synthetic division of each combo polynomial by `(x - z)` as a
single sequential `cycles`-iteration Horner loop on ONE thread per combo chunk
(`@compute @workgroup_size(1)`), i.e. ~11 active threads on a GPU with 21,760 lanes.
Fine-grained drain attribution (see the SP7go rejection evidence) showed this was
~25.9 s of the 57.5 s xgboost wall — the single largest bucket in the prover, hidden
for two months behind a drain label that blamed the FRI round-0 expand NTT.

## Change

The quotient coefficient `b_i = Σ_{j>i} a_j z^(j-i-1)` is an associative weighted
suffix sum. Decomposed into three data-parallel kernels over 256-element blocks
(`COMBOS_DIVIDE_SCAN_WGSL` / `_CARRY_WGSL` / `_FIXUP_WGSL`):

1. **scan** — per-block in-workgroup suffix scan `s_t = Σ_{j≥t} a_j z^(j-t)`
   (Hillis-Steele doubling, weight `z^(2^step)`); writes the shifted local part into a
   scratch buffer and block summary `S_k` into a carries buffer;
2. **carry** — per-chunk serial scan over block summaries
   `C_k = S_{k+1} + z^256·C_{k+1}` (≤1024 iterations, trivial);
3. **fixup** — per-element `b_i = scratch[i] + z^(255-t)·C_k` (pow-by-squaring,
   no Montgomery ONE needed).

Successive divisors of the same chunk are dependent → one round per divisor, all
rounds dispatched back-to-back in ONE compute pass / one queue submission. Identical
Montgomery arithmetic → bit-identical results. Scratch (combos-sized) and carries
buffers are explicitly `destroy()`ed post-submit (D14 lesson). Default-on via
`set_combos_divide_parallel_enabled` (legacy kernel retained as the flag-off path).
Host interface, chunk/pow buffer layout, and call sites unchanged.

WGSL gotchas hit: `active` is a reserved WGSL keyword (renamed `is_active`);
barriers require workgroup-uniform control flow, so inactive groups scan zeros
instead of early-returning (guards depend on storage loads, which the uniformity
analysis cannot prove uniform).

## TDD evidence

- Focused parity (new `webgpu_hal_combos_divide_parallel_matches_cpu_at_production_shape`):
  cycles=2^18, 3 chunks with 1/3/2 divisors — bit-exact vs CPU expected, zero
  fallbacks/mirrors. Existing `webgpu_hal_combos_authoritative_matches_cpu` (cycles=8,
  partial block) also passes through the new path. Both green in 0.30s.
- First run caught the reserved-keyword validation failure as "output = initial data"
  (voided submits); naga validation of the extracted kernels pinpointed it.

## Gate results (same-day A/B vs the M0 baseline, warm GPU, robustness on)

| Gate | M0 baseline | M1b | Movement |
|---|---:|---:|---:|
| Representative runtime | 82.12 s | **57.22 s** | **−30.3%** |
| BusyLoop wall / gpu_active | 5479 / 4047 ms | **3923 / 2480 ms** | −28% |
| KeccakUnion wall / gpu_active | 75930 / 56830 ms | **52562 / 33683 ms** | −31% |
| xgboost runtime | 58.05 s | **32.22 s** | **−44.5%** |
| xgboost wall | 57524 ms | **31670 ms** | −45% |

All receipts verified (xgboost journal `30.528042544062632`), `cpu_fallbacks=0`,
`cpu_only_ops=0`, queue_submits unchanged (168 / 3153 / 2803), parallel-path marker
assertions wired into both default gates.

## Attribution recheck

`finalize_async drain_after_combos_divide`: **25963 ms → 224 ms** (n=32). Wall moved
by the bucket delta (−25.8 s), confirming mechanism. Remaining top buckets:

| Bucket | ms | Note |
|---|---:|---|
| `finalize_async drain_after_eval_check` | 8565 | interpreter eval_check; known Chrome/Dawn ceiling (SP3) |
| `poly_group accum drain_after_batch_expand_into_evaluate_ntt` | 2545 | likely accum witgen queue-flush — needs its own drain split before believing the label |
| everything else | <700 each | |

## New accepted state

- xgboost **31.67 s** wall ≈ **5.6× native CUDA** (5.7 s). Project baseline was 117.9 s (21×).
- Representative 57.22 s (BusyLoop 3923 ms, KeccakUnion 52562 ms).
