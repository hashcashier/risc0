# SP7gi Strided-Local NTT Shape Coverage

Date: 2026-05-22

## Purpose

Quantify how much of the SP7fy FRI/check drain is covered by the first SP7gc/SP7ge guard:

```text
n_bits = 20
domain = 1048576
count = 4 FRI round0, or count = 16 check group
```

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Method

Parsed existing SP7fy browser logs only:

- `2026-05-22-sp7fy-drain-attribution-representative.chrome.txt`
- `2026-05-22-sp7fy-drain-attribution-xgboost.chrome.txt`

Used `browser-prove:webgpu-stage ... elapsed_us=...` summary lines for:

- `drain_after_expand_evaluate_ntt`
- `drain_after_batch_expand_into_evaluate_ntt`

## Representative BusyLoop + KeccakUnion

Total parsed FRI/check expand drain: `60043 ms`.

Top shapes:

| Shape | Time |
|---|---:|
| FRI `domain=1048576` | `25430 ms` |
| check `domain=65536 count=16 size=16384` | `23009 ms` |
| check `domain=1048576 count=16 size=262144` | `5163 ms` |
| accum `domain=1048576 count=12 size=262144` | `2749 ms` |

Strict first-guard coverage:

```text
eligible n_bits=20 FRI round0 + check16@1048576 = 30593 ms
missed check16@65536 = 23009 ms
eligible share of parsed FRI/check drains = 50.95%
```

Interpretation: the `n_bits=20` first guard can still materially move KeccakUnion through FRI round0 and the `1048576` check-group calls, but it leaves a large `domain=65536` check bucket untouched.

## Xgboost

Total parsed FRI/check expand drain: `41429 ms`.

Top shapes:

| Shape | Time |
|---|---:|
| FRI `domain=1048576` | `27242 ms` |
| check `domain=1048576 count=16 size=262144` | `9143 ms` |
| accum `domain=1048576 count=12 size=262144` | `2124 ms` |

Strict first-guard coverage:

```text
eligible n_bits=20 FRI round0 + check16@1048576 = 36385 ms
missed check16@65536 = 0 ms
eligible share of parsed FRI/check drains = 87.82%
```

Interpretation: the strict first guard is well targeted for xgboost.

## Decision Impact

Keep the first runtime candidate narrow enough to validate quickly:

- `n_bits=20`
- `count=4 || count=16`
- `expand_bits=2`
- full-block `blocks_per_row=1024`

But do not mistake that for full representative coverage. If the first candidate passes and improves xgboost while KeccakUnion remains limited by `domain=65536` check work, the next immediate extension should be the same strided-local design generalized to smaller full-block rows, especially:

```text
n_bits=16
domain=65536
count=16
blocks_per_row=64
```

That smaller-row extension needs a slightly different shader guard because the scratch-active block count is `blocks_per_row`, not always `1024`. The SP7gh scratch shader intentionally assumes the first `n_bits=20` full-block case and would not be valid as-is for `n_bits=16`.

## Acceptance Implication

The SP7gf e2e acceptance gate remains correct: do not accept on xgboost alone. Representative BusyLoop + KeccakUnion must also improve materially. If the first guard improves xgboost but leaves representative performance short of material improvement, keep the candidate default-off and extend coverage to `domain=65536` before accepting a default path.
