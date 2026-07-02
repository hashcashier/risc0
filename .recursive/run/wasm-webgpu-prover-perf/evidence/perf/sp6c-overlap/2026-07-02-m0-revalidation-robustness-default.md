# M0 re-validation: SP7fr state landed, robustness made default

Date: 2026-07-02

## Context

The SP7fr accepted state sat uncommitted in the worktree since 2026-05-22 (browser gate was approval-blocked). This checkpoint committed it (`39bc885d6`), re-ran the proof gates, and landed the result.

## Environment regressions found and resolved

The initial gate reruns failed with `mapAsync ... A valid external Instance reference no longer exists` (Chrome GPU process SIGTRAP, `exit_code=133`). Root causes, in order of discovery:

1. **hermes-vllm crash-loop (system, not repo):** the local inference service had been
   crash-looping since boot (91 restarts), cyclically loading ~24.5 GiB VRAM every ~35 s.
   This starved Chrome's Vulkan device mid-proof (readback drains of 17–57 s, then device
   loss). Gates require the service stopped; it also independently fails with
   `Free memory on device cuda:0 (26.41/31.35 GiB) < desired (0.92, 28.84 GiB)`.
2. **`disable_robustness` now crashes driver 580.159.04:** with a quiet GPU, proofs still
   died at the recursion ctrl commit under `--enable-dawn-features=...disable_robustness...`
   (validated fine on 580.159.03 in May). Not Chrome-version-related: reproduced identically
   on pinned Chrome-for-Testing 148.0.7778.178 and system Chrome 149.0.7827.102. No kernel
   Xid errors; the fault is in the userspace Vulkan driver's non-robust pipeline path.
3. **Cold-clock artifact (diagnostic red herring):** first run after idle measured
   BusyLoop `wall_ms=60222` with the GPU pinned in P8 (270–742 MHz, 25–42 W); the driver
   ramped to P0/2625 MHz on the next run which measured `wall_ms=5452`, at parity with the
   May baseline. Same-conditions comparisons must warm the GPU first.

## Fix

Removed `disable_robustness` from `examples/browser-prove/webdriver.json`. This is the
configuration real end-user Chrome runs anyway (users cannot pass Dawn unsafe flags), so
gates now validate the deployment-realistic mode. Measured cost: none (numbers below beat
the May robustness-off baselines). Follow-up (external): the 580.159.04 non-robust SIGTRAP
is an NVIDIA driver regression; revisit `disable_robustness` only if a future driver fixes
it AND it measurably wins.

## Gate results (2026-07-02, driver 580.159.04, Chrome 149.0.7827.102, chromedriver 149.0.7827.155, robustness ON, hermes-vllm stopped)

| Gate | 2026-05-22 (SP7fr, robustness off) | 2026-07-02 (this run) |
|---|---:|---:|
| Representative test runtime | 88.54 s | 82.12 s |
| BusyLoop wall / submits | 5962 ms / 168 | 5479 ms / 168 |
| KeccakUnion(1) wall / submits | 81813 ms / 3153 | 75930 ms / 3153 |
| xgboost test runtime | 62.45 s | 58.05 s |
| xgboost wall / submits | 61883 ms / 2803 | 57524 ms / 2803 |

All receipts verified; `cpu_fallbacks=0`, `cpu_only_ops=0` across all workloads; queue
submit counts exactly match the SP7fr acceptance record, confirming the same execution
shape. Logs: `/tmp/gate1_representative.log`, `/tmp/gate2_xgboost.log` (session-local).

## Decision

Accept as the current baseline: representative `82.12 s`, xgboost `58.05 s` (~10.1× native
CUDA's 5.7 s). SP7fr state + robustness-default merged to `wasm`. Next planned runtime work
remains SP7go (tiled strided-local NTT), gated on these same proof gates.
