# M4b: program-constant recursion code-group cache

Date: 2026-07-03. Status: ACCEPTED — modest wall win, structural cost removal,
byte-compatible transcripts (all receipts verify).

## The change

The recursion circuit's ctrl (code) group is a pure function of
(program, po2): `make_coeffs` is interpolate + the fixed `3^i` zk-shift, and
the group's merkle root IS the public control ID — deterministic by
construction (a randomized padding would make published control IDs
impossible). So the fully built `PolyGroup` (coeffs + 4× LDE evaluated +
merkle tree) is reusable across every lift/join/union proof on a device.

- `Prover::commit_group_cached_async(tap_group_index, witness, control_id)`
  (webgpu-only): thread-local cache keyed
  `(hal.instance_id(), control_id, cycles, group_size)`. Miss → existing
  `commit_group_async` + insert a clone; hit → insert cached clone into
  `groups[]` and replay the transcript via `MerkleTreeProver::commit_async`
  (byte-identical top-layer digests + root commit).
- `WebGpuHal::instance_id()` (monotonic per-HAL id): buffers are
  device-scoped, so the cache never crosses `GPUDevice` boundaries.
- Manual `Clone` impls for `PolyGroup<WebGpuHal>` / `MerkleTreeProver<WebGpuHal>`
  (Rc buffer views onto shared GPU storage) + `Clone, Copy` derive on
  `MerkleTreeParams`.
- Recursion `prove_async_with_control_id` uses the cached variant whenever the
  caller supplies a control ID (production always does — zkvm
  `recursion::Prover::new` → `new_with_control_id`); `None` falls back to the
  uncached path.
- Only possible under M4a lazy shadows: the cached group pins ~zero wasm heap
  (coeffs shadow stays pending via GPU interpolate; evaluated + merkle nodes
  never materialize). Pre-M4a this pinned ~184 MB/entry — the M3b blocker.

Witgen still builds/uploads the ctrl witness per proof (its WGSL kernels bind
`buf_ctrl` directly during witgen/accum) — untouched by this cache.

## Cache behavior (measured)

- xgboost: **17 hits / 4 misses** — exactly 2 programs (lift_18, join) × 2
  devices (width-2 succinct pipeline).
- representative: 18 hits / 8 misses (BusyLoop session + KeccakUnion's
  lift/join/union chain across per-session devices).
- `without_eval_check_gpu` exact-fallback-count test passes (1 miss, count
  assertions hold).

## Gate results (vs M4a baseline, same warm conditions)

| Gate | M4a | M4b (gates / landing) | Movement |
|---|---:|---:|---:|
| BusyLoop wall | 2609/2613 ms | 2594 / **2587 ms** | ~−0.9% |
| KeccakUnion(1) wall | 42853–44534 | 44060 / 44001 ms | flat (variance band) |
| xgboost wall | 23742 ms | 23488 / **23430 ms** | **−1.2%** |

Parity 4/4 bit-exact; native `--features prove` 36/36; receipts verified;
zero fallbacks/CPU-only on all devices.

## Why the win is smaller than the span table promised

The M4a stage table showed 21 `commit_group_async recursion_ctrl` spans
summing 2873 ms. After caching, the same spans sum 1351 ms — but wall only
moved ~−300 ms. The ctrl-commit spans were **absorbing queued witgen GPU work**
(the M1 lesson: a span names the wait point, not the workload). Skipping the
commit removes its real GPU work (~30–50 ms/proof) and its root_top_readback
barrier, but the queued-work wait relocates to the next stage. The remaining
1351 ms of span time = 4 miss builds + hit-path `commit_async` top readbacks
absorbing the same queued witgen work.

Verdict: keep — real (consistent −250–310 ms xgboost across 2 runs + gate
run), removes 17 mapAsync barriers per xgboost session, zero added memory
under M4a, and validates the M4a unlock story. But the recursion-phase
bottleneck is the witgen work itself, not the ctrl commit.
