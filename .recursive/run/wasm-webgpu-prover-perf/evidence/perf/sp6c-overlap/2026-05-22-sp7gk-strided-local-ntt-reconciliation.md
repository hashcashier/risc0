# SP7gk Strided-Local NTT Reconciliation

Date: 2026-05-22

## Purpose

Reconcile the earlier SP7ge/SP7gf implementation and validation artifacts with the SP7gj tiled-offset extension before any runtime code is written.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Reconciliation

SP7ge originally pinned the first retained candidate to a strict `n_bits=20`, one-offset-per-workgroup shape. SP7gi later showed that this covers most parsed xgboost FRI/check drains but only about half of the representative BusyLoop + KeccakUnion parsed FRI/check drains, because KeccakUnion has a large `domain=65536`, `n_bits=16`, `count=16` check bucket.

SP7gj resolves that coverage gap with a tiled-offset shader:

```text
offsets_per_tile = min(16, 1024 / blocks_per_row)
scratch_elems = blocks_per_row * offsets_per_tile
```

For the guarded shapes, `1024 / blocks_per_row` is already a power of two. This keeps the same 1024-element scratch ceiling while covering both:

- `n_bits=20`, `blocks_per_row=1024`, `offsets_per_tile=1`
- `n_bits=16`, `blocks_per_row=64`, `offsets_per_tile=16`

## Artifact Updates

Updated SP7ge:

- marks the original `n_bits=20`-only blueprint as refined by SP7gj;
- changes the runtime guard to the tiled shape;
- replaces `total_offsets` with `offsets_per_tile` / `tiles_per_row` / `total_tiles`;
- keeps the strided params as a 12-word / 48-byte uniform to match the existing local-prefix param layout style;
- records `total_local_prefix_groups = count * blocks_per_row`;
- records `total_strided_groups = count * tiles_per_row`;
- adds the KeccakUnion `n_bits=16 count=16` expected delta: current `13312` workgroups / `48 MiB` remaining RW to candidate `2048` workgroups / `8 MiB`;
- requires the representative gate to exercise the KeccakUnion `n_bits=16` bucket when the tiled shader is enabled.

Updated SP7gf:

- treats KeccakUnion `n_bits=16` coverage as part of the representative acceptance gate;
- adds focused hot-shape smoke requirements for `count=16`, `in_size=16384`, `out_size=65536`, `expand_bits=2`, `offsets_per_tile=16`;
- requires drain attribution to show the `domain=65536` / `n_bits=16` check bucket moving down when that guard is enabled;
- forbids accepting an `n_bits=20`-only default enablement if representative performance remains dominated by the SP7gi KeccakUnion bucket.

## Runtime Handoff

When browser WebGPU validation becomes available again, the next runtime candidate should be:

1. Default-off or candidate-gated tiled strided-local NTT.
2. Guarded initially to:
   - `expand_bits == 2`
   - `local_ntt_fused_bits == 10`
   - `n_bits=20 && count in {4,16} && offsets_per_tile=1`
   - `n_bits=16 && count=16 && offsets_per_tile=16`
3. Validated first by focused HAL parity/marker coverage.
4. Accepted only after representative BusyLoop + KeccakUnion and xgboost browser proof gates verify receipts, keep `cpu_fallbacks=0` / `cpu_only_ops=0`, and materially reduce wall time versus SP7fr.

## Decision

The stale `n_bits=20`-only blueprint is not the acceptance target anymore. It may still be used as a temporary implementation step if browser validation cycles are scarce, but the performance claim and default enablement require representative KeccakUnion coverage, including the `n_bits=16` check bucket.
