# SP7fz expand4 load-once candidate not accepted

Date: 2026-05-22

## Candidate

Screened a small `batch_expand_into_evaluate_ntt` idea that is distinct from the reverted row4/radix/pair/local-12 variants: for the common `expand_bits=2` shape, the local NTT expand kernel currently reads each input coefficient four times while filling the duplicated expanded scratch block. A load-once branch would read one input coefficient and write the four duplicated scratch entries.

## RED Evidence

Added a focused browser HAL test requiring the optimized path marker while still comparing WebGPU output against the CPU mirror:

- Test: `webgpu_hal_batch_expand_ntt_uses_expand4_load_once_path`
- Evidence: `2026-05-22-sp7fz-red-expand4-load-once.chrome.txt`
- Result: failed as intended under high WebGPU limits.
- Failure reason: output parity reached, but diagnostics showed only `webgpu_batch_expand_local_ntt_params`; the expected `webgpu_batch_expand_local_ntt_expand4_params` marker was absent.
- Counters at failure: `raw_compute_dispatches=3`, `queue_submits=3`, `cpu_fallbacks=0`, `cpu_only_ops=0`.

## Outcome

The GREEN/browser validation command was rejected by the approval system because the session hit its usage limit before the focused test could rerun. Because the user requirement is e2e proof-backed correctness before accepting performance work, the temporary production shader branch and focused test were removed.

No runtime code was retained. Accepted wall-time gain: `0`.

This candidate can be retried only when browser GPU validation is available, and should still be rejected unless it passes the focused parity test plus representative BusyLoop+KeccakUnion and xgboost e2e proof gates with zero fallback/CPU-only and a measurable wall-time improvement.
