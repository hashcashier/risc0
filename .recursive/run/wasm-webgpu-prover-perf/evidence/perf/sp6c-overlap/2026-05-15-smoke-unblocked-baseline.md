Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `Post-environment-fix baseline (commit 172ae5404 + dc2e8f4f7)`
Date: 2026-05-15

## Context

After the Dawn-blocklist webdriver.json fix landed at 172ae5404, the
smoke environment is fully unblocked. This doc captures the new
baseline against the iter-6a/iter-6c landed work, to set up A/B
comparisons for subsequent SPs.

## R1 smoke (host: RTX 5090, NVIDIA 580.159.03, Chrome 148.0.7778.167)

| Fixture | wall_ms | gpu_idle_ratio | Status |
|---|---:|---:|---|
| hello_world_succinct_receipt_verifies | 4694 | 0.241 | OK |
| json_succinct_receipt_verifies | 3920 | 0.410 | OK |
| internal_cfg_succinct_receipt_verifies | 3229 | 0.365 | OK |

All three pass receipt verification. Walls within ~20% of the iter-5a
baseline (3.93s for hello_world).

## xgboost baseline (R10 indicator)

| Metric | Value | vs SP10 retro (104.2s) |
|---|---:|---:|
| wall_ms | **102603** | -1.5% (noise) |
| gpu_active_ms | 57564 | n/a |
| gpu_idle_ratio | **0.439** | +0.008 (noise) |
| Native CUDA | 5700 (per SP10) | n/a |
| **Ratio vs CUDA** | **18.0×** | flat vs 18.3× |

The iter-6a MuxChunk pass + iter-6c Rust pruner do NOT touch the
runtime path -- they are pure infrastructure prep. Flat result is
expected and correct (no regression). The actual performance lever is
iter-6d / iter-6e which wires the chunked WGSL into
`WebGpuCircuitHal::generate_witness`.

## Practical floor (per SP10 retro 2026-05-13)

- 1.0× CUDA: hardware-bound, not reachable on Chrome/Dawn
- 5-8× CUDA: practical floor on single-tab single-GPU
- 18× CUDA: current state
- ~12-15× CUDA: ideal after SP6d + iter-6d combined
- ~10× CUDA: ideal after SP6d + iter-6d + per-kernel SP3/SP6a revisits

## Next perf-leverage candidates (now unblocked by smoke recovery)

1. **iter-6d**: Wire the iter-6c Rust pruner into
   `risc0/circuit/rv32im/src/prove/hal/webgpu.rs::generate_witness` for
   `exec_Top` chunks. Expected: -5 to -10s per xgboost (witness phase
   only; accum still CPU).
2. **SP6d iter-10**: Route xgboost segment+lift+join through
   `WebGpuProverPool::prove_with_ctx_scheduled_async` (iter-9 landed
   ~8% win on synthetic mixed workload; xgboost has the same
   heterogeneous mix).
3. **SP9 (corrected)**: Pipeline + bind-group cache, properly this
   time (layout-shape cache FIRST, then pipeline cache, with
   per-helper-site audit). Earlier flag-only attempts failed; see
   61d3163c9 failed-experiment ledger.
4. **SP8 iter-2**: Coalesce check_group readback with poly_interpolate
   parallel work. ~1-3s per xgboost expected.

Each is independently verifiable via smoke A/B against the 102.6s
baseline measured here.
