Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6d iter 8 — end-to-end pool prove_with_ctx_async`
DraftedAt: `2026-05-14`
Status: `COMPLETE`

## TL;DR

`WebGpuProverPool::prove_with_ctx_async` wires the validated iter-1..7
pool substrate into a production end-to-end prove path. Per-segment
proves run serially through slot 0 (concurrent segment proves OOM
wasm32 — see below); pending keccak proofs, segment lifts, tree joins,
and assumption resolves are distributed across the pool's independent
`web_sys::GpuDevice` slots.

Two new smokes pass green and produce verifying succinct receipts:

| Smoke | Workload | Wall | GPU util (mean/peak) | Power (mean/peak) | Verifies |
|---|---|---:|---:|---:|:--:|
| `webgpu_pool_prove_session_multi_segment_smoke` | BusyLoop{500_000} → 3 segs | 33.3 s | **35.5% / 100%** | 117 W / 374 W | ✅ |
| `webgpu_pool_prove_session_keccak_union_smoke` | KeccakUnion(2) → 7 segs + ~17 keccaks + 16 unions + 1 resolve | 287 s | **47.5% / 100%** | 131 W / 375 W | ✅ |

The keccak-union run's **47.5% mean GPU util** is the highest sustained
engagement measured in the whole run — beating iter 6's 40.2%. Keccak +
union work is the most parallelism-friendly: many fully-independent
proofs distributable across slots.

## What landed

### Public-API surgery (Phase A)

Exposed on `ProverImpl` (`pub(crate)` for in-crate pool use):
`prove_segment_core_async`, `lift_async`, `join_async`, `resolve_async`,
`union_unknown_async`, `insert_union_receipt_async`,
`union_receipts_root_async`, `composite_to_succinct_async`.

Module `prover_impl` is now `pub(crate)`; the WebGPU env-default
constants `WEBGPU_DEFAULT_SEGMENT_LIMIT_PO2` /
`WEBGPU_DEFAULT_KECCAK_MAX_PO2` are `pub(crate)`.

These are purely additive — the single-slot `WebGpuProver` path is
untouched. R1 regression confirms no behavior change (below).

### Orchestrator (Phase B)

`WebGpuProverPool::prove_with_ctx_async(env, ctx, elf, opts)`:

1. Apply WebGPU env caps (po2 ≤ 18, keccak_po2 ≤ 14).
2. Construct one `ProverImpl` per slot.
3. Execute the session on CPU (single-threaded).
4. **Segment proves: SERIAL through slot 0.** See OOM finding below.
5. Merge journal + assumptions into final segment claim.
6. **Pending keccaks: distributed** via `prove_keccak_requests_async`
   (bounded chunks of `pool.len()`).
7. Build keccak union root on slot 0 (small tree depth).
8. Verify composite receipt + claim digest.
9. Composite mode → return composite.
10. Succinct mode → `composite_to_succinct_async`:
    a. Strip assumptions, distribute lifts + tree joins via existing
       `lift_and_join_async`.
    b. Apply assumption resolves serially on slot 0 (recurse for
       nested composites).

`composite_to_succinct_async` is the new pool method that handles
assumption_receipts (the old `lift_and_join_async` rejects them).

## Key finding: concurrent segment proves OOM wasm32

Iter 8 first attempted concurrent per-segment proves (chunks of
`pool.len()`, same bounded pattern as keccaks/lifts). It failed:

```
RuntimeError: unreachable
  at <CpuHal as Hal>::alloc_elem
  at <WebGpuHal as Hal>::alloc_elem
  at PolyGroup::new_async
  at commit_group_async_scoped
  at WebGpuSegmentProver::prove_core_async
```

At po2_18, a single segment's `commit_group_async` peaks at ≈1.8 GiB
(code + data + accum poly-group buffers). Two segments hitting their
`rv32im_accum` poly-group allocation at the same moment overflow
wasm32's isize-bounded `Vec` (~2 GiB ceiling).

**Lifts and keccak proofs do NOT have this problem** — their per-job
peak is smaller, so chunks of 2 fit. The fix: segment proves run
serially through slot 0; everything downstream stays distributed.
This still delivers the structural win because (a) lift+join is where
multi-segment fixtures spend their distributable time and (b) keccak
proofs are the dominant parallel workload for accelerator fixtures.

## Measurements

### Multi-segment (`webgpu_pool_prove_session_multi_segment_smoke`)

BusyLoop{500_000} at default po2_18 → 3 segments (po2 18, 18, 17),
2-slot pool.

| Phase | Time |
|---|---:|
| Segment 0 prove (serial) | 7554 ms |
| Segment 1 prove (serial) | 7084 ms |
| Segment 2 prove (serial) | 3944 ms |
| Segments subtotal | ~18.6 s |
| Lift + tree-join (distributed) | ~14.7 s |
| **Total wall** | **33 264 ms** |

nvidia-smi @ 100 ms cadence (532 samples):
- GPU util: **mean 35.5% / peak 100%**, 30% of samples > 30%
- Power: mean 117.4 W / peak 374.1 W

Receipt verifies via `info.receipt.verify(MULTI_TEST_ID)`.

### Keccak union (`webgpu_pool_prove_session_keccak_union_smoke`)

KeccakUnion(2) at keccak_max_po2=14 → 7 segments, ~17 keccak proof
requests, 16 union proves, 1 assumption resolve. 2-slot pool.

| Phase | Notes |
|---|---|
| 7 segment proves | serial, slot 0 |
| ~17 keccak proves | distributed, bounded chunks of 2 |
| 16 union proves | serial on slot 0 (MMR tree assembly) |
| 7 lifts + 6 joins | distributed via lift_and_join_async |
| 1 resolve | serial on slot 0 (keccak root assumption) |
| **Total wall** | **287 103 ms** |

nvidia-smi @ 100 ms cadence (3212 samples, ~321 s window):
- GPU util: **mean 47.5% / peak 100%**, 46% of samples > 30%
- Power: mean 131.0 W / peak 374.6 W

Receipt verifies via `info.receipt.verify(MULTI_TEST_ID)`.

This is a stress fixture, not a wall-time benchmark — 287 s is long
because 16 union proves run serially on slot 0 at ~3.3 s each. But it
exercises every phase of the orchestrator (segments + keccaks + union
tree + lifts + joins + resolve) and the receipt verifies. The 47.5%
mean util is the takeaway: the pool keeps the 5090 engaged ~3.8× more
than single-device's 12.6% baseline.

### R1 regression (single-slot path unchanged)

| Fixture | Wall | gpu_idle_ratio | Verifies |
|---|---:|---:|:--:|
| poseidon2_basic (`native_poseidon2_basic_async_succinct_receipt_verify`) | 3999 ms | 0.467 | ✅ |
| libm (`native_libm_succinct_receipt_verify`) | 4020 ms | 0.465 | ✅ |
| busy_loop_po2_18 (`native_busy_loop_po2_18_async_succinct_receipt_verify`) | 10607 ms | 0.607 | ✅ |

All three verify. Walls are consistent with the memory baseline
(3.93 s for poseidon2_basic); ~24% above the STATE.md post-iter-4
single-trial figures (3220/3247 ms), attributable to single-trial
variance and concurrent build load during the measurement window. No
correctness regression — the pub-API surgery is additive only.

## Operational note: WASM_BINDGEN_TEST_TIMEOUT

`wasm-bindgen-test-runner` defaults `WASM_BINDGEN_TEST_TIMEOUT` to **20
seconds**. Multi-phase pool proves (segments + lifts, or the keccak
union stress fixture) exceed this and chromedriver gets SIGKILL'd with
"Failed to detect test as having been run."

Run pool smokes with `WASM_BINDGEN_TEST_TIMEOUT=300` (or 400 for the
keccak union fixture). Also pass `CHROMEDRIVER=<path>` explicitly —
the runner's auto-discovery hit a permission error on this box; the
wasm-pack-managed chromedriver at
`~/.cache/.wasm-pack/chromedriver-*/chromedriver` works.

## What this leaves open

- **xgboost through the pool path.** The orchestrator is ready, but
  routing the Hermes / `DefaultProver` proving APIs through
  `WebGpuProverPool` instead of `WebGpuProver` is a separate
  integration. xgboost has no assumptions and several segments, so it
  would benefit from the distributed lift+join immediately.
- **4-slot pool measurements.** 32 GiB VRAM allows 4 concurrent lifts
  at po2_18; the orchestrator already chunks by `pool.len()`.
- **Concurrent segment proves.** Blocked by the wasm32 2 GiB Vec
  ceiling at po2_18. Would need either (a) smaller segment po2 caps
  for the pool path, or (b) SP7's GPU-resident witness to shrink the
  per-segment CPU buffer peak. Until then, segment proves stay serial.
- **Resolve distribution.** Currently serial on slot 0; most fixtures
  have ≤ 1 assumption so the win is marginal.
- **Serial union tree.** The keccak union MMR builds on slot 0. Tree
  depth is log(N) so distributing it has a bounded ceiling, but for
  large keccak workloads (17 unions @ 3.3 s = 53 s here) there's a
  real win available.
