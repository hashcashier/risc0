# SP7ey: recursion WOM compact/sort-valid candidate rejected

Date: 2026-05-22

## Candidate

Compact valid recursion WOM rows before sorting and sort only the valid prefix, with temporary substage timers around recursion witgen. The intended benefit was lower CPU recursion witness time after SP7ex moved recursion accumulation to GPU.

## Evidence

Raw logs:

- Initial low-limit failed attempt: `2026-05-22-sp7ey-xgboost-wom-compact-candidate.chrome.txt`
- Valid xgboost rerun: `2026-05-22-sp7ey-rerun-xgboost-wom-compact-candidate.chrome.txt`
- Valid BusyLoop + KeccakUnion rerun: `2026-05-22-sp7ey-rerun-default-representative-wom-compact-candidate.chrome.txt`

Correctness passed on both valid reruns:

- xgboost succinct receipt verified journal `30.528042544062632`.
- BusyLoop and KeccakUnion receipts verified.
- All valid reruns reported `cpu_fallbacks=0` and `cpu_only_ops=0`.

Performance versus accepted SP7ex:

| Gate | SP7ex accepted | SP7ey candidate | Delta |
|---|---:|---:|---:|
| xgboost proof wall | `64309 ms` | `63511 ms` | `-798 ms` |
| xgboost test runtime | `64.84s` | `64.06s` | `-0.78s` |
| xgboost recursion_witgen total | `5254 ms` | `5189 ms` | `-65 ms` |
| BusyLoop proof wall | `5683 ms` | `5680 ms` | `-3 ms` |
| KeccakUnion proof wall | `85436 ms` | `85437 ms` | `+1 ms` |
| BusyLoop + KeccakUnion test runtime | `91.83s` | `91.85s` | `+0.02s` |
| BusyLoop + KeccakUnion recursion_witgen total | `6986 ms` | `6979 ms` | `-7 ms` |

## Decision

Rejected and reverted. The candidate is correctness-clean, but the structural hot-bucket improvement is only `65 ms` on xgboost and effectively zero on BusyLoop + KeccakUnion. The larger xgboost proof-wall movement is within browser-run noise and is not enough to keep a CPU-side optimization while the requested priority is immediate significant wall-time reduction and GPU witgen offload.

Next work should return to high-upside paths: full recursion witness `exec` WGSL generation/preflight wiring, or a non-repeated FRI/check Merkle strategy.
