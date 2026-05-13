Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6c iter 1 — gpu_idle_ratio instrumentation baseline`
DraftedAt: `2026-05-13`

## What landed (iter 1)

- `risc0/zkp/src/hal/webgpu.rs`: added `WebGpuStageTimer::new_active`, `snapshot_gpu_active_ms`, `reset_gpu_active_ms`. Thread-local `WEBGPU_GPU_ACTIVE_MS` accumulator. Drop impl accumulates elapsed_ms when `gpu_active=true`. Added `log_webgpu_metric` helper that emits a `browser-prove:metric` line.
- `risc0/zkp/src/prove/prover.rs`: converted finalize-stage timers to `new_active`:
  - `finalize_async check_group`
  - `finalize_async check_commit`
  - `finalize_async eval_u_groups`
  - `finalize_async eval_u_check`
  - `finalize_async mix_poly_coeffs`
  - `finalize_async combos_prepare`
  - `finalize_async combos_divide`
  - `finalize_async bit_rev`
  - `finalize_async fri_prove`
  - `finalize_async eval_check_drain` (staged-only path)
- `risc0/zkvm/src/host/server/prove/prover_impl.rs`: in `prove_session_async`, capture wall + active snapshots and emit `browser-prove:metric prove_session_async wall_ms=X gpu_active_ms=Y gpu_idle_ratio=Z`.

## Baseline measurements (post iter 1, current branch state)

| Fixture | wall_ms | gpu_active_ms | gpu_idle_ratio |
|---|---:|---:|---:|
| poseidon2_basic | 3231 | 1745 | **0.460** |
| libm | 3246 | 1764 | **0.457** |
| keccak_union_small (4 seg + 9 keccaks + 1 assumption) | 107507 | 63413 | **0.410** |

No wall-time regression vs SP6b iter 4 baselines:
- poseidon2_basic: previous 3258 ms → 3231 ms (within noise)
- libm: previous 3258 ms → 3246 ms (within noise)
- keccak_union_small: no prior tight baseline; 107.5 s is in expected band

## What the metric over-counts (and what the next iters need)

The metric counts every wall-second of a `new_active` stage's lifetime, not strictly the GPU-busy slice. Inside `finalize_async fri_prove` (the dominant active stage at ~938 ms on lift), there is:
- GPU dispatch issuance (~tens of ms cumulative)
- GPU compute on the 5090 (Poseidon2 merkle, FRI fold)
- mapAsync drains (CPU thread parks, GPU may be done already)

So the 46% idle reported is an UPPER bound on actual idle. The nvidia-smi-measured 87% wall idle from 2026-05-13-webgpu-libm-utilization is the LOWER bound on actual idle (it under-counts dispatches < 200 ms cadence). Reality sits between.

Coverage gap in current marking:
- `commit_group_async` invocations (3 per lift, ~400 ms total) — IS GPU-active but no timer wraps it directly. Inner `make_coeffs_async` and `PolyGroup::new_async` could be marked active. Next iter.
- Circuit-level witgen/accumulate timers in `risc0/circuit/recursion/src/prove/hal/webgpu.rs` etc. — these wrap PORTABLE (CPU) implementations currently, NOT GPU work. Correctly left as `new`. When SP7 lands GPU witness/accumulate, those timers must flip to `new_active`.

## Stage cost decomposition (poseidon2_basic, libm)

rv32im prove_segment finalize active (sums 364 ms):
| Stage | ms |
|---|---:|
| check_group | 104 |
| check_commit | 2 |
| eval_u_groups | 70 |
| eval_u_check | 13 |
| mix_poly_coeffs | 0 |
| combos_prepare | 0 |
| combos_divide | 0 |
| bit_rev | 0 |
| fri_prove | 175 |

Lift finalize active (sums 1381 ms):
| Stage | ms |
|---|---:|
| check_group | 110 |
| check_commit | 2 |
| eval_u_groups | 301 |
| eval_u_check | 24 |
| mix_poly_coeffs | 6 |
| combos_prepare | 0 |
| combos_divide | 0 |
| bit_rev | 0 |
| fri_prove | 938 |

Lift non-finalize wall = 2223 - 1381 = 842 ms (witgen + accumulate + commit_groups + IOP commits)
Segment non-finalize wall = 957 - 364 = 593 ms (witgen + commit_groups + IOP commits)

## Where iter 2 should attack

The 842 ms lift non-finalize is the next target. Of that, ~400 ms is commit_group_async (GPU-active but currently mis-attributed as idle) and ~407 ms is genuine CPU work (witgen + accumulate). If we overlap the 407 ms CPU witgen with the 938 ms FRI dispatch, we shave ~400 ms off lift wall — a ~12% prove_session reduction.

The bigger win remains SP6c iter 3 (composite_to_succinct_async lift pipelining for multi-segment) and SP6d (workers). Multi-segment is where the structural idle dominates.
