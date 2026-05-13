Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `03 Implementation — running status update`
Status: `INFORMATIONAL` (does not modify locked Phase 03 summary)
DraftedAt: `2026-05-13`
Workflow version: `recursive-mode-audit-v2`
Amends: none (status snapshot)
Inputs:
- `addenda/02-to-be-plan.addendum-03.md` (SP6c overlap + gpu_idle_ratio)
- `addenda/02-to-be-plan.addendum-04.md` (SP6c promoted, SP6d added, per-kernel SPs demoted)
- Hardware-utilization measurement 2026-05-13 (CUDA vs WebGPU)

## Landed in this implementation push (post Phase 03 lock)

| Phase | Iter | Commit | Outcome |
|---|---|---|---|
| SP6a | 1 | (earlier) | Per-round FRI timers; Poseidon2 merkle ceiling identified |
| SP6b | 4 | 31dfdf2c0 | Chunked Horner's heuristic; **-13% poseidon2_basic wall (3739→3258 ms)** |
| SP6c | 1 | 15a24b078 | `gpu_idle_ratio` metric instrumented |
| SP6c | 2 | 63e742b32 | `commit_group_async` + `witgen_accum` instrumented; metric tightens 0.46→0.34 |
| SP6c | 3 | 463924dd9 | PARKED — structural single-thread + single-device limit |
| SP6d | 1 | cfc0ed00d | `WebGpuProverPool` scaffold + 2-slot construct smoke |

## Reference measurements (2026-05-13)

CUDA vs WebGPU on RTX 5090 (libm succinct):
- CUDA: 437 ms wall, 25.6% mean GPU util, 119.4 W mean power
- WebGPU: 3231 ms wall, 12.6% mean GPU util, 53.7 W mean power
- Ratio: 7.4× CUDA wall speedup; 2× CUDA per-active-second density

R1 smoke matrix (current branch tip, gpu_idle_ratio reported):
| Fixture | wall_ms | gpu_idle_ratio |
|---|---:|---:|
| poseidon2_basic | 3219 | 0.344 |
| libm | 3213 | 0.343 |
| keccak_union_small | 107138 | 0.353 |

## What's left

### SP6d iter 2+ (work distribution across pool slots) — HIGH PRIORITY

Now that the pool scaffold exists, the remaining work:

1. **`prove_session_async(pool: &WebGpuProverPool)` variant** — accept a pool and round-robin segments across slots. Each slot has its own HAL, its own circuit context (via `with_webgpu_hal`), its own command queue. Concurrent execution via `futures::join_all` is safe because each slot's state is independent.
2. **`composite_to_succinct_async(pool)` variant** — distribute lifts across slots; joins at the same tree level can run on different slots.
3. **Buffer-handoff discipline** — receipts (small, ~KB) cross slot boundaries via CPU memory. Per-circuit large GPU buffers stay on their owning slot.
4. **Memory budget** — each WebGpuHal allocates ~6 GiB peak on poseidon2-sized lift. 4 slots × 6 GiB = 24 GiB. Budget allows 2–3 slots on a 32 GiB card before exceeding `max_buffer_size`. Validate with smoke.
5. **Multi-segment regression test** — `webgpu_multi_worker_xgboost_smoke` per Addendum 04. Target: gpu_idle_ratio < 0.20 on a 4-slot pool with xgboost (vs current 0.35 single-slot).

Expected wall-time win (extrapolated from utilization headroom):
- Single-segment (poseidon2_basic, libm): SP6d iter 2 win is bounded — only one segment + one lift fits in the pipeline naturally. Could shave ~30% if lift runs on slot B while segment finalize runs on slot A. Target: 3200 ms → ~2400 ms.
- Multi-segment (keccak_union_small 4 segments + 4 lifts): SP6d iter 2 win is bigger. Could shave ~50% with 2 slots. Target: 107 s → ~60 s.
- xgboost (R9 deferred, ~117 s baseline): expected 3–4× with 3-4 slots. Target: ~30–40 s.

### SP7-SP11 (per to-be plan)

These are structurally larger and not addressed in this session:

| SP | Description | Effort estimate | Why deferred |
|---|---|---|---|
| SP7 | GPU-resident witness + accumulate (replace `rust_kernels::generate_witness` and `rust_kernels::accumulate` with WGSL kernels for recursion / keccak / rv32im) | 1–3 weeks | Per-circuit witness gen is ~1000–3000 lines of CUDA per circuit. WGSL ports are non-trivial. Highest-value but multi-week scope. SP7 attacks the ~250 ms per-circuit `witgen_accum` time that is currently CPU-pure. |
| SP8 | Coalesce readbacks (4 per finalize → 1) | 1–2 days | Targeted at `eval_u_groups` (3 readbacks) + `eval_u_check` (1 readback). Each readback ~5–15 ms on Chrome WebGPU. Achievable saving: ~30–60 ms per finalize × 2 finalizes per prove = ~60–120 ms (~3% on poseidon2_basic). |
| SP9 | Pipeline + bind-group cache abstraction | 1 week | Currently each kernel creates its pipeline fresh per dispatch (except `eval_check_interpreter_pipelines`). General cache would benefit all kernels but require an audit + threading. |
| SP10 | Deferred fixture matrix (xgboost, BLST, etc) | execution-only | After SP6d delivers the multi-segment wins, R9 fixtures should be re-measured to determine wall ratios vs CUDA. |
| SP11 | Closing audit + parity tables | 2–3 days | Best done after SP6d + SP7 land so the final ratio table reflects all levers applied. |

## Realistic remaining roadmap

```
NEAR (1-2 weeks):
  SP6d iter 2 — distribute lifts across pool   (target: -30% single-seg, -50% multi-seg)
  SP8        — coalesce eval_u readbacks       (target: -3%  on smokes)

MEDIUM (3-6 weeks):
  SP7  — GPU-resident witness for recursion    (target: -7%  per circuit, -21% combined)
  SP7  — GPU-resident witness for rv32im
  SP7  — GPU-resident witness for keccak
  SP9  — pipeline cache                        (target: -5%  smoke)

CLOSING (1 week):
  SP10 — exercise R9 deferred matrix
  SP11 — closing audit + ratio tables
```

Composed expectation:
- Current poseidon2_basic: 3219 ms / 7.4× CUDA (CUDA 437 ms)
- After SP6d iter 2: ~2400 ms / 5.5× CUDA
- After SP7 (witness on GPU): ~2100 ms / 4.8× CUDA
- After SP8 + SP9: ~1950 ms / 4.5× CUDA
- After multi-device tuning: ~1500 ms / 3.4× CUDA (single-device floor)

For ≤ 1.0× CUDA parity (R10 closing target) we need either: a hardware change (multi-GPU per browser, not currently exposed), or per-kernel WGSL improvements approaching nvcc quality (per SP3/SP6a retros: structural ceiling on Chrome/Dawn). R10 may need a "structural residual" footnote per SP11's checklist.

## Memory + plan documents to update at session boundary

- `MEMORY.md` already updated with `project_webgpu_submission_bound` pointer.
- `addenda/02-to-be-plan.addendum-04.md` codified the reprioritization.
- This document is the running-status snapshot; future sessions can resume from here without re-deriving the measurement context.
