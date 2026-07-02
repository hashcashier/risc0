# SP7gr Static Prep Complete

Date: 2026-05-22

## Purpose

Record that the non-runtime static preparation for the next material performance candidate is complete enough to stop adding planning artifacts and wait for browser e2e validation.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Readiness Check

Reviewed the current handoff artifacts:

- SP7gn: final WGSL scratch shader validates with Naga.
- SP7go: runtime patch sequence, guard, param layout, labels, focused test requirements, e2e proof gates, drain-attribution gate, and rejection criteria are pinned.
- SP7gp: objective gap audit maps user requirements to evidence and records why the goal is incomplete.
- SP7gq: consistency audit reconciles stale handoff wording and marks SP7gn/SP7go as authoritative.

The remaining useful work is runtime implementation plus browser proof validation. That is blocked by SP7gb, which rejected the browser command before execution and produced no Chrome log.

## Not Doing Next

Do not add more speculative planning or unrelated optimization threads while the browser gate is blocked.

Do not retain runtime code for:

- tiled NTT;
- Memory64 experiments;
- iframe/multi-device partitioning;
- further recursion upload elision;
- additional witgen offload.

Those either require browser e2e proof validation or are lower-priority than the measured FRI/check NTT bucket.

## Next Action

When browser WebGPU proof validation is available again:

1. Implement the SP7go default-off/candidate-gated tiled NTT patch.
2. Run focused HAL parity/marker coverage.
3. Run representative BusyLoop + KeccakUnion proof generation.
4. Run xgboost proof generation.
5. Run SP7fy-style drain attribution.
6. Accept only if receipts verify, `cpu_fallbacks=0`, `cpu_only_ops=0`, and wall time materially improves.

If validation remains unavailable, no further performance claim can be accepted.

## Decision

Static preparation for the next material candidate is complete. The run remains validation-blocked and the objective is not achieved.
