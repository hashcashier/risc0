# M3: zkr cache + timestamped attribution + succinct-phase pipelining

Date: 2026-07-03. Status: ACCEPTED — canonical gates green, receipts
verified, zero CPU fallbacks on all devices.

## Final gate results (same-day warm A/B vs M2 baseline)

| Gate | M2 | M3 | Movement |
|---|---:|---:|---:|
| BusyLoop wall | 3670 ms | **3026 ms** | **−17.5%** |
| KeccakUnion(1) wall | 44238 ms | **44123 ms** | flat |
| xgboost wall | 29108 ms | **27613 ms** (runs 27412–27613) | **−5.1%** |
| xgboost vs native CUDA (5.7 s) | 5.1× | **≈4.8×** | |

Representative test runtime 48.62→49.21 s (test total includes one-time
second-device acquisition + pipeline compiles; the session walls
improved). Extra coverage: the heavy `native_keccak_union_succinct_
receipt_verify` fixture (11 segments + 25 keccaks + 24 unions + resolve,
NOT part of the canonical baseline set) passes through the new scheduler,
127.7 s, receipts verified. Note for future close-outs: the canonical
"KeccakUnion 44 s" number comes from `rv32im_default_representative_
e2e_verify` (BusyLoop + KeccakUnion(1) in one test), not from the
similarly-named native_keccak_union fixture.

BusyLoop's −18%: its lone lift now runs on the dedicated recursion
device, no longer waiting behind residual segment-phase work queued on
the main device — the scheduler pool hands out dedicated devices first
for exactly this reason.

## M3a — instrumentation + zkr program cache (landed with this phase)

- **zkr Program cache** (`risc0_circuit_recursion::prove::zkr::get_zkr`,
  wasm32-only): every `lift`/`join` previously re-opened the embedded ZIP
  and DEFLATE-decompressed the program (`join.zkr` 23.9 MB decoded ×10,
  `lift_rv32im_v2_18.zkr` 13.4 MB ×11 per xgboost session). Cache decoded
  `Program` per (name, po2), clone on hit. Measured `cached=false` cost:
  32 ms (join) / 23 ms (lift) — subsequent hits ~1 ms.
- **Stage timestamps**: `browser-prove:stage start|done t=<Date.now()>`
  on every span; deleted the duplicate local `WebGpuStageTimer` in
  `prover_impl.rs` in favor of the HAL one.
- **New sub-stage spans**: `recursion_program_load`,
  `recursion_prover_setup lift|join`, `recursion_receipt_finish lift|join`,
  `recursion_preflight`, `recursion_witgen_new`, `recursion_header_commit`,
  `rv32im_prove_setup`, `rv32im_pre_witgen_dispatch`, `rv32im_header_commit`,
  `rv32im_accum_shadow_sync`, `rv32im_post_accum_sync`.
- Production xgboost: **28818 ms** (vs 29108 M2 baseline, −1.0%) —
  program load was NOT the bulk of lift/join self-time; the instrumentation
  relocated it (see below). BusyLoop 3719 ms (flat). Receipts verified.

## The production timeline (M3a, xgboost, drains OFF)

Self-time by family: `merkle * root_top_readback` = **12.4 s** of the
28.8 s wall (check 6.1 s, accum 2.8 s, code 0.7 s, recursion_data 0.6 s,
ctrl 0.6 s, data 0.6 s, fri ~0.5 s). Cross-check vs the drains-on M2
attribution (same readbacks ~2 ms there): these windows are CPU **waiting
on the GPU queue** — the natural pipelining slots. CPU compute:
rv32im_witgen 3.58 s, recursion_witgen 3.04 s, rv32im accum stepper
2.28 s, recursion_witgen_new 1.09 s, recursion_preflight 0.60 s, misc ~2 s.
GPU demand (from M2 serialized drains): ~14.1 s. CPU ~13-14 s. Balanced →
pipeline ceiling ≈ max ≈ 16 s wall.

Phases (M3a run): segments 15943 ms + gap 161 ms + succinct 12714 ms.

Gap report (uninstrumented, per-instance): rv32im witgen→header 89 ms/seg;
recursion ctrl upload tail 38 ms/proof; FRI tail (queries/seal) 22 ms/proof;
make_coeffs encode 51 ms/seg (rv32im data), 24 ms/proof (recursion data).

**M3b candidate discovered**: recursion `ctrl` group and rv32im `code`
group are program-constant (ctrl merkle root IS the control id) but are
rebuilt+recommitted per proof: commit recursion_ctrl 1221 ms + rv32im_code
946 ms per session (attribution), plus the 0.8 s ctrl upload tail.

## M3c v1 — join∥lift software pipeline (superseded by v2)

`composite_to_succinct_async`: `try_join!(join(acc, lift_i), lift_{i+1})`
with per-future virtualization of the HAL GPU-authoritative flag
(`with_authoritative_context`: plain RAII scopes assume stack discipline;
interleaved awaits violate it and would silently flip dispatch modes).

Result: xgboost **27780 ms** (−1038 vs M3a), succinct phase 12714→11745 ms.
Receipts verified, BusyLoop flat (degenerate pipeline).

Timeline: lifts DID overlap joins (6.6 s pairwise overlap; every lift ran
inside its join's window) — but the **serial join chain is the critical
path** (10 joins), and queue contention with the concurrent lift inflated
each join 800→1030 ms. A left fold cannot go below N_joins × join-time.

## M3c v2 — balanced join tree + width-2 scheduler (measured; kept as fallback shape)

Joins are associative over adjacent spans → balanced tree keeps the same
10 joins but cuts dependency depth 10→4; same-level joins become
schedulable work. Width-2 `FuturesUnordered` scheduler, joins preferred
over lifts.

Result: xgboost **27587 ms**, succinct 11745→11523 ms (−222 only).
Receipts verified; BusyLoop flat. Timeline: two proofs in flight for 80%
of the phase (9.2 s of pairwise overlap) — yet every proof INFLATED
(lifts 657→875-1268 ms, joins ~800→1040 ms) and wall ≈ CPU(6.5) +
GPU(5.5) ≈ 12 s.

**Root cause: a `mapAsync` readback waits for everything previously
submitted on the device's single FIFO queue — including the other
proof's batches.** Every transcript readback is a global queue barrier,
so each CPU burst begins only once the queue is empty: the GPU idles
during every CPU burst and CPU/GPU overlap never materializes.
Single-device interleaving cannot beat sum(CPU, GPU); the tree shape is
still right, the queue is the constraint.

## M3c v3 — one device per in-flight proof (LANDED shape, width 2)

`WebGpuProver` lazily acquires extra `GpuDevice`s (cached across prove
calls; pipeline compiles amortize), threaded to `ProverImpl`; the
scheduler assigns each admitted proof its own HAL. Falls back to
same-device interleaving when the browser refuses extra devices. The
zero-cpu-fallback gate now also asserts on every extra device's
diagnostics.

Result: xgboost **27412 ms**, succinct ~12.0 s — device separation did
NOT restore the ideal overlap (readback waits grew: the two devices still
share one physical GPU, so exec serializes on the hardware; and every
readback resume competes for the single wasm thread). **BusyLoop improved
3719→3055 ms (−18%)**: its lone lift moved off the main device and
stopped waiting behind residual segment-phase queue work — the pool now
hands out dedicated devices before the main one for exactly this reason.

Per-await round-trip floor measured ~2-3 ms (FRI-round readbacks in the
serial run) — latency-floor hypothesis dead; the remaining ~4.5 s gap to
ideal is CPU-idle windows where both in-flight proofs await the GPU.

## M3c width 3 — OOM (measured; do not retry at po2=18/memory32)

`WEBGPU_SUCCINCT_PIPELINE_WIDTH = 3` (third device): xgboost aborts
mid-phase with the wasm allocator-OOM `unreachable` signature (no panic
message) as the third proof enters `check_group`. Three concurrent
recursion proofs exceed the ~2 GiB wasm32 heap — empirical confirmation
of SP6d's ceiling. Width pinned at 2 with the rationale in the const doc.
Revisit only with memory64 or a smaller per-proof heap footprint.

## M3 final state

xgboost 29108 → **27412 ms wall** (−5.8%; runtime 29.65 → ~28.0 s,
≈ 4.8× native CUDA), BusyLoop 3719 → **3055 ms** (−18%). Succinct phase
12.7 → 12.0 s; its remaining structure: CPU 6.5 s + GPU-exec ~3.6 s +
un-fillable await windows. Next levers (M3b): program-constant ctrl/code
group caching (~2.2 s session-wide GPU+CPU), rv32im witgen tail 89 ms/seg,
FRI seal tail 22 ms/proof — all reduce BOTH the segment phase (untouched
15.9 s, the larger half) and the succinct floor.
