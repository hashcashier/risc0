# SP7gc FRI/Check Strided-Local NTT Plan

Date: 2026-05-22

## Purpose

Pick the next concrete high-leverage FRI/check candidate without repeating rejected NTT sidequests.

No production runtime code changed. Browser WebGPU validation is currently blocked by the approval usage limit recorded in SP7gb, so this checkpoint is a plan and sizing artifact only.

Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Current Hot Bucket

SP7fy re-confirmed that the dominant queued work after SP7fr is FRI/check `batch_expand_into_evaluate_ntt`:

- xgboost `fri_prove round=0 drain_after_expand_evaluate_ntt domain=1048576`: `27242 ms` over `32` calls.
- xgboost `poly_group check drain_after_batch_expand_into_evaluate_ntt count=16 size=262144 domain=1048576`: `9143 ms` over `32` calls.
- KeccakUnion has the same shape: FRI round-0 `23718 ms`, check-group expansion `23009 ms` at domain `65536` plus `4424 ms` at domain `1048576`.

The current SP7fr WebGPU NTT path:

1. expands input and performs local NTT stages through `FUSED_BITS=10` in `BATCH_EXPAND_LOCAL_NTT_WGSL`;
2. runs remaining stages `s_bits=11..n_bits` with `NTT_STEP_WGSL`, one full global-memory read/write pass per remaining stage.

For the hot `domain=1048576` shape, `n_bits=20`, so the current path has 10 remaining global NTT passes after the accepted local-10 prefix.

## Already Rejected Shapes

Do not retry these as immediate work:

- submit-count-only or one-submit fusion: SP7cx;
- grouped NTT loop / Metal-like loop shape: SP7ck;
- index shift/mask micro-edit: SP7cn/SP7ep;
- fused expand-first / expand-bit pre-application: SP7co/SP7ed/SP7fz;
- row4 reuse / row4 vector: SP7dc/SP7dj;
- pair2 invocation reshaping: SP7di;
- radix-4 paired global stages: SP7de;
- larger local-12 scratch: SP7dk;
- direction-specialized branch removal: SP7dn;
- workgroup-size 512: SP7cw.

Those were either dispatch-count, invocation-shape, or local-scratch variants. They did not materially reduce the measured FRI/check drain bucket.

## Candidate

Add a second local NTT phase after the accepted local-10 prefix:

```text
current:
  expand + stages 3..10 locally inside each 1024-element block
  stages 11..20 as 10 full-row global passes

candidate:
  expand + stages 3..10 locally inside each 1024-element block
  stages 11..20 locally across the 1024 blocks for each intra-block offset
```

For a `2^20` row, the first local phase leaves `1024` blocks of `1024` elements. For each intra-block offset `j in 0..1023`, a workgroup gathers:

```text
scratch[k] = row[k * 1024 + j], k in 0..1023
```

Then it performs the remaining NTT stages in workgroup memory and writes the same strided locations back. This is equivalent to the existing global stages if the twiddle lookup uses the original flattened-stage `s`:

```text
stage = 10 + t
block_s_size = 1 << (t - 1)
s_original = (block_s * 1024) + intra_block_offset
twiddle = twiddles[(1 << (stage - 1)) - 1 + s_original]
```

This is a memory-pass reduction candidate, not a dispatch-count-only candidate.

## Static Sizing

Local sizing script:

```text
for n_bits,count in [(20,4),(20,16),(18,4),(18,16)]:
  row_size = 1 << n_bits
  block_size = 1024
  remaining_stages = n_bits - 10
  current_rw_bytes = count * remaining_stages * row_size * 8
  candidate_rw_bytes = count * row_size * 8
```

Results:

| Shape | Current remaining global passes | Current workgroups/call | Candidate workgroups/call | Current RW/call | Candidate RW/call |
|---|---:|---:|---:|---:|---:|
| FRI round0 `n_bits=20 count=4` | 10 | 81920 | 4096 | 320 MiB | 32 MiB |
| check group `n_bits=20 count=16` | 10 | 327680 | 16384 | 1280 MiB | 128 MiB |
| smaller FRI `n_bits=18 count=4` | 8 | 16384 | 4096 | 64 MiB | 8 MiB |
| smaller check `n_bits=18 count=16` | 8 | 65536 | 16384 | 256 MiB | 32 MiB |

The risk is memory coalescing: the second phase reads/writes strided locations (`stride=1024`), so browser/Dawn may not realize the full theoretical traffic reduction. But unlike the rejected variants, the candidate removes whole full-row passes and should be evaluated with a focused shader parity/perf gate.

Follow-up static correctness evidence: SP7gd simulated the proposed strided-stage twiddle mapping against the current staged forward NTT for 133 small BabyBear analogue cases, including `expand_bits=2`; all cases matched. Evidence: `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7gd-strided-local-ntt-simulation.md`.

## Implementation Gate When Browser Validation Returns

Do not promote directly to default. Use a default-off candidate path:

1. Add `BATCH_EXPAND_STRIDED_LOCAL_NTT_WGSL` or equivalent.
2. Guard initially to proof-shaped hot cases:
   - `expand_bits == 2`
   - `n_bits == 20` first
   - `count == 4 || count == 16`
   - `blocks_per_row <= 1024`
   - `max_compute_workgroup_storage_size >= 4096`
3. Add focused browser HAL parity test:
   - compare candidate output against CPU for `count=4`, `in_size=262144`, `out_size=1048576` if feasible;
   - if that is too slow for focused parity, use a smaller `n_bits` test plus one full-shape marker/perf probe.
4. Add a candidate marker upload/source such as `webgpu_batch_expand_strided_local_ntt_params`.
5. Run BusyLoop + KeccakUnion proof e2e first, with receipt verification and zero fallback/CPU-only.
6. Run xgboost proof e2e with receipt/journal verification and zero fallback/CPU-only.
7. Accept only if representative wall time improves materially and the SP7fy FRI/check drain bucket moves.

## Expected Upside

The hot xgboost FRI/check drains sum to about `36.4s` in SP7fy. The candidate cannot remove all of that because Merkle hashing and other queued GPU work are included near the drains, and strided memory access may be inefficient. But replacing 10 full global passes with one strided local phase is the first remaining FRI/check idea with a plausible `8-15%` e2e wall-time ceiling from SP7fr.

If it fails, the likely reason will be strided-memory inefficiency. The next deeper design after that would be a six-step/transpose NTT variant that preserves coalesced memory access, but that is more invasive and should not be the first attempt while validation cycles are scarce.

## Decision

When browser e2e validation is available again, prioritize this strided-local second-phase NTT candidate before returning to recursion-witgen CPU-exec offload.

Rationale:

- it targets the current largest measured bucket;
- it is not one of the already rejected dispatch/invocation micro-shapes;
- it has a clear focused parity test;
- it has a clear representative e2e acceptance gate;
- its ceiling is materially higher than the remaining recursion-witgen CPU-exec ceiling.
