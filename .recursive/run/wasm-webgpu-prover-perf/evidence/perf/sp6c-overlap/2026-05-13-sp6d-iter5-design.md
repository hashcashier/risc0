Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP6d iter 5 — single-prove segment distribution (DESIGN)`
DraftedAt: `2026-05-13`
Status: `DESIGN`

## Goal

Distribute the segments of a SINGLE prove across pool slots. Multi-segment
workloads (keccak_union_small, xgboost) currently run their N segments
strictly serially. With a 2-slot pool, two segments can prove in parallel;
with a 4-slot pool, four.

Iter 3's two-independent-proves smoke proved 2 concurrent succinct proves
go from 12.6% to 52.7% mean GPU util. Iter 5 unlocks the same benefit for
a single multi-segment prove.

## Architecture

### Today

```
ProverImpl::prove_session_async(&self, ctx, session):
  for segment in session.segments:
    let preflight = self.segment_preflight(segment);
    let receipt = self.prove_segment_core_async(ctx, preflight).await;  // uses self.hal
    segments.push(receipt);
  CompositeReceipt { segments, ... }
  if Succinct:
    self.composite_to_succinct_async(composite).await  // lift loop, uses self.hal
```

ProverImpl holds ONE `Rc<WebGpuHal>`. All segment proves and lift proves
go through that HAL.

### Iter 5 target

```
WebGpuProverPool::prove_session_async(&self, ctx, session):
  // Preflight all segments upfront on slot 0 (CPU-only work).
  let preflights: Vec<_> = session.segments.iter().map(|s| preflight(s)).collect();

  // Distribute segment proves: each segment N proves on slot (N % pool.len()).
  let segment_futures: Vec<_> = preflights.iter().enumerate().map(|(i, p)| {
      let slot = self.get(i % self.len());
      slot.prove_segment_core_async(ctx, p)
  }).collect();
  let segments = futures::future::try_join_all(segment_futures).await?;

  // Lifts: distribute similarly.
  let lift_futures = segments.iter().enumerate().map(|(i, seg)| {
      let slot = self.get(i % self.len());
      slot.lift_async(seg)
  }).collect();
  let lifted = futures::future::try_join_all(lift_futures).await?;

  // Joins: build tree, distribute each level's joins across slots.
  let mut tier = lifted;
  while tier.len() > 1 {
    let join_futures = tier.chunks(2).enumerate().map(|(i, pair)| {
        let slot = self.get(i % self.len());
        match pair {
          [a, b] => slot.join_async(a, b),
          [a] => async { Ok(a.clone()) },
        }
    });
    tier = futures::future::try_join_all(join_futures).await?;
  }

  let final_receipt = tier.into_iter().next().unwrap();
  Ok(ProveInfo { receipt: ..., ... })
```

## Public-API surface needed on WebGpuProver

Currently exposed:
- `prove_async(env, elf)` — end-to-end
- `prove_with_opts_async(env, elf, opts)` — end-to-end
- `prove_with_ctx_async(env, ctx, elf, opts)` — end-to-end
- `compress_async(opts, receipt)` — composite → succinct compression

Needed (new):
- `segment_preflight(segment) -> Result<PreflightResults>` — CPU only, can run on any slot or no slot
- `prove_segment_core_async(ctx, preflight) -> Result<SegmentReceipt>` — uses THIS slot's HAL
- `lift_async(segment_receipt) -> Result<SuccinctReceipt<ReceiptClaim>>` — uses THIS slot's HAL
- `join_async(left, right) -> Result<SuccinctReceipt<ReceiptClaim>>` — uses THIS slot's HAL

These all already exist as `pub(crate)` methods on `ProverImpl`. The work is making them public on `WebGpuProver` (which holds its own ProverImpl).

## Risks

### 1. Cross-HAL receipt portability

Receipts are pure CPU data (no GPU resources retained). Crossing HAL
boundaries is safe — confirmed by the fact that `WebGpuProver::compress_async`
already accepts a receipt produced by a different prover and lifts/compresses
it on its own HAL.

### 2. Memory budget

Each slot allocates ~6 GiB peak during a lift. The 32 GiB 5090 can hold
4-5 slots concurrently before exceeding `max_buffer_size`. For an N-slot
pool, peak VRAM = N × per-slot-peak. xgboost (~6 GiB lift) on 4 slots =
24 GiB — feasible. On 6 slots = 36 GiB — risky.

Add: pool memory budget assertion at construction time, refuse pools larger
than `floor(gpu_total_vram_gib / per_slot_peak_gib)`.

### 3. Cycle preflight ordering

`segment_preflight` may depend on session-global state (cycle ranges,
proof system info). Validate that running preflights in any order produces
the same per-segment results as the current serial-loop order.

### 4. Hooks

`session.hooks.on_pre_prove_segment` / `on_post_prove_segment` fire per
segment in the current code. With parallel proves, these fire concurrently
and may race if the hook does mutable bookkeeping. Test hooks use Rc<RefCell>
flags — race-free for single-thread async. Production hooks not surveyed.
Add a `hooks_are_thread_safe()` capability check.

## Estimated effort

- WebGpuProver public API: ~half day
- ProverImpl public lift/join methods: ~half day
- WebGpuProverPool::prove_session_async: ~1 day
- Memory budget assertion: ~few hours
- Regression test webgpu_multi_segment_pool_smoke (keccak_union_small): ~half day
- Hook ordering audit + fix: ~half day to 1 day

**Total: 3-4 days.** Defers to dedicated follow-on session.

## Validation

Smoke fixture: `keccak_union_small` (4 segments + 9 keccaks + 1 assumption).
Current baseline: 107 s, gpu_idle_ratio 0.35.

Iter 5 target:
- 2-slot pool: wall ~70 s (segments interleave), gpu_idle_ratio ~0.20
- 4-slot pool: wall ~50 s, gpu_idle_ratio ~0.15

xgboost (R9, ~117 s baseline):
- 2-slot pool: wall ~70 s
- 4-slot pool: wall ~40-50 s

These projections derive from iter 3's measurement (multi-device delivers
4× util / 2× power). Translated into wall reduction for a fixed workload:
2 slots → ~0.65× wall, 4 slots → ~0.45× wall.

## What this session leaves for iter 5+

- The pool type structure exists (iter 1).
- Per-HAL metric works (iter 4).
- Concurrent multi-prove validated end-to-end (iter 3).

What needs adding:
1. Pub-API surgery to expose ProverImpl's per-phase methods.
2. The `WebGpuProverPool::prove_session_async` orchestrator.
3. The multi-segment smoke.
4. Memory budget assertion.
