# SP7gp Current Objective Gap Audit

Date: 2026-05-22

## Purpose

Audit the active objective against the current evidence before any further runtime work.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain in this checkpoint: 0.

## Objective Restatement

Concrete success criteria for the current thread:

1. Make the browser WebGPU prover materially closer to native CUDA wall time.
2. Preserve absolute proof correctness.
3. Validate every retained performance change with browser e2e proof generation, not proxy metrics.
4. Keep representative coverage broad enough to include BusyLoop, KeccakUnion, and xgboost.
5. Prioritize immediate significant wall-time reductions and avoid sidequests.
6. Do not retain speculative runtime code when the browser proof gate is unavailable.

## Prompt-To-Evidence Checklist

| Requirement / Ask | Current Evidence | Status |
|---|---|---|
| Minimize proving wall time with correctness as primary goal | SP7fr accepted default state has representative runtime `88.54s` and xgboost runtime `62.45s`, both with verified receipts and zero fallback/CPU-only | Partially satisfied; not close enough to CUDA |
| Validate changes with e2e proof generation | SP7fr has BusyLoop+KeccakUnion and xgboost browser proof evidence; SP7gb records that further browser validation is blocked | Satisfied for accepted SP7fr only |
| Representative workloads must include KeccakUnion | SP7fr representative gate includes BusyLoop + KeccakUnion; SP7gf/SP7go require KeccakUnion `n_bits=16` coverage for the next NTT candidate | Satisfied for accepted state and planned next gate |
| xgboost remains covered | SP7fr xgboost proof verifies journal `30.528042544062632`; SP7go keeps xgboost as an acceptance gate | Satisfied for accepted state and planned next gate |
| Prioritize immediate significant gains | SP7fy shows FRI/check expansion NTT is the current largest bucket; SP7ga bounds recursion exec at only `3-6%` potential | Satisfied by prioritizing tiled NTT over witgen side paths |
| Move on to GPU witgen | SP7ga maps why full recursion exec offload is dependency-bound and smaller upside than FRI/check NTT | Investigated; deferred by evidence |
| Do not be distracted by sidequests | SP7go pins one next runtime patch path and rejects partial/default enablement without e2e gain | Satisfied in current plan |
| Current working performance | SP7fr: representative `88.54s`; xgboost `62.45s`; zero fallback/CPU-only | Current accepted baseline |
| Have gains been made | SP7fr improved vs SP7fp by `-2.78s` representative and `-3.22s` xgboost test runtime | Yes, accepted gains exist |
| Are 20-25% streams still viable | SP7fy says `8-15%` credible from FRI/check NTT; `15-25%` only with deeper NTT/FRI or complete GPU-resident recursion exec | Viable but not yet proven |
| WASM Memory64 helpful | No current hot-bucket evidence points to address-space pressure as the limiting factor after high-limit Chrome fix; current blockers are queue drains and NTT memory passes | Not prioritized |
| iframe multi-device workaround | No evidence yet that iframe/device partitioning reduces the measured hot bucket; multi-device work is a side path until one-device FRI/check NTT is tested | Deferred |

## Evidence Chain

Accepted performance state:

- SP7fr: `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7fr-recursion-witgen-submit-batched.md`
- Representative BusyLoop + KeccakUnion: `88.54s`, BusyLoop `5962 ms`, KeccakUnion `81813 ms`, zero fallback/CPU-only.
- xgboost: `62.45s`, `wall_ms=61883`, journal `30.528042544062632`, zero fallback/CPU-only.

Current hot-bucket attribution:

- SP7fy: `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7fy-current-drain-attribution.md`
- xgboost FRI/check drains: `27242 ms + 9143 ms`.
- KeccakUnion FRI/check drains: `23718 ms + 23009 ms + 4424 ms`.

Validation blocker:

- SP7gb: `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7gb-browser-validation-blocked.md`
- Browser command approval was rejected before execution; no Chrome log was produced.
- Runtime performance changes cannot be made or retained while this gate is unavailable.

Next implementation handoff:

- SP7gm exact arithmetic simulation passed for `n_bits=16`, `18`, and `20`.
- SP7gn final WGSL scratch shader validates with Naga.
- SP7go maps the exact runtime patch sequence and acceptance gates.

## Completion Audit

The objective is not achieved.

Missing or unverified requirements:

- No retained runtime implementation exists yet for the tiled strided-local NTT candidate.
- No focused browser HAL parity proof exists for the final SP7gn shader.
- No representative BusyLoop + KeccakUnion e2e proof exists for the tiled NTT candidate.
- No xgboost e2e proof exists for the tiled NTT candidate.
- No post-candidate drain attribution exists proving FRI/check expansion buckets moved down.
- No new accepted wall-time reduction beyond SP7fr exists after the browser validation blocker.
- Browser WebGPU remains substantially slower than native CUDA, so the top-level performance objective is incomplete.

## Next Concrete Action When Gate Returns

Implement only the SP7go default-off/candidate-gated tiled NTT patch, then run gates in order:

1. focused HAL parity/marker test;
2. representative BusyLoop + KeccakUnion proof gate;
3. xgboost proof gate;
4. SP7fy-style drain attribution recheck.

If any gate fails, remove the candidate. If the candidate is correct but flat, reject it and move to a coalesced transpose/six-step NTT design rather than continuing local cleanup.

## Decision

Continue, but do not mark the goal complete. The run is validation-blocked, not performance-complete.
