# M4a: lazy CpuBuffer shadows

Date: 2026-07-03. Status: ACCEPTED — parity bit-exact, native suite green, gates green.

## The change

`CpuBuffer::new` no longer eagerly allocates `vec![T::default(); size]`.
The backing storage is now `Arc<RwLock<LazyVec<T>>>` where `LazyVec` is
either `Pending { size, fill: fn() -> T }` (logically `vec![fill(); size]`,
zero heap) or `Ready(TrackedVec<T>)`. Materialization happens on the first
CPU access (`as_slice`, `as_slice_mut`, `view`, `view_mut`, `get_at`,
`to_vec`, `get_ptr`). Slices share the base `Arc`, so the Pending→Ready
transition is coherent across every aliasing view. Constructors with real
data (`copy_from`, `from_fn`, `From<Vec>`) build `Ready` directly.

The fill is a plain `fn() -> T` pointer (`T::default` coerces), which
sidesteps the bound mismatch between `CpuBuffer::new` (`T: Default+Clone`)
and the `Buffer` trait accessors (`T: Clone` only).

Two WebGPU fast paths ride on it:

1. **Skip-zero-upload** (`sync_cpu_to_gpu`): fresh buffers are born
   `cpu_dirty=true`, so every alloc'd-then-dispatched buffer used to pay a
   full-size `queue.writeBuffer` of zeros at first bind. Now: shadow still
   pending + `gpu_known_zero` + fill bit-pattern is zero ⇒ clear dirty and
   return. (Fill zero-ness checked at runtime via `bytemuck::bytes_of`, no
   type assumptions.)
2. **Materialize-from-readback** (`sync_gpu_to_cpu`/`_unchecked` via new
   `CpuBuffer::copy_in_from_slice`): full-region readback into a pending
   shadow materializes directly from the readback bytes, skipping the
   default fill that used to precede `clone_from_slice`.

Attribution: `set_buffer_materialize_observer` hook in cpu.rs; the WebGPU
HAL installs a logger emitting `browser-prove:metric cpu_shadow_materialize
name=... bytes=...` for events ≥ 1 MiB.

Dead-code discipline: the two new `CpuBuffer` helpers are cfg-gated to the
webgpu module's exact cfg (their only consumer) so native builds stay
warning-clean under `-D warnings`.

## What still materializes (xgboost, 11 segs + 21 recursion proofs)

| Shadow | bytes × count | Why |
|---|---|---|
| rv32im `data` | 221.2 MB × 11 | CPU witgen writes (INVALID fill at alloc) |
| rv32im `accum` | 108.0 MB × 11 | accum shadow sync for CPU stepper |
| rv32im `combos` | 21.0 MB × 11 | FRI combos CPU stage |
| `recursion_data` | 134.2 MB × 21 | recursion witgen CPU writes |
| recursion `combos` | 25.2 MB × 21 | FRI combos CPU stage |
| recursion `accum` | 12.6 MB × 21 | accum path |

**Absent** (never materialize any more): every `evaluated` PolyGroup buffer
(884 MB logical per rv32im segment!), all merkle digest buffers, NTT
temporaries, `check_poly`, FRI layers. Those shadows previously cost alloc
memset + heap pin + full zero upload each.

## Gate results (warm A/B vs M3b baseline)

| Gate | M3b | M4a | Movement |
|---|---:|---:|---:|
| BusyLoop wall | 2773 ms | **2609 / 2613 ms** | **−5.8%** |
| KeccakUnion(1) wall | 42932 ms | 44534 / **42853 ms** | flat (variance band) |
| xgboost wall | 24409 ms | **23742 ms** | **−2.7%** |
| xgboost vs native CUDA (5.7 s) | 4.3× | **≈4.2×** | |

Parity suite 4/4 bit-exact; focused bench rv32im eval_check ~130-140 ms
(unchanged); `cargo test -p risc0-zkp --features prove` 36/36 native
(default-feature lib tests are pre-existing broken: no_std `#[test]` in
poseidon2, stale `params.layers` asserts in merkle.rs — untouched).
Receipts verified, zero cpu fallbacks / cpu-only ops on all devices.

## Unlocks

- **M4b program-constant code-group caching**: a cached recursion
  `PolyGroup` now pins ~0 CPU bytes (coeffs shadow stays pending — GPU
  interpolate; evaluated + merkle never materialize). Previously ~184 MB.
- **Width-3 succinct pipelining retry**: per-recursion-proof CPU footprint
  dropped from ~600 MB to ~172 MB materialized.
- KeccakUnion note: 44534 first read was variance — re-run 42853 (below
  baseline); rule stays "never call KeccakUnion on one reading".
