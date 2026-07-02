# SP7gj Tiled Strided-Local NTT Extension

Date: 2026-05-22

## Purpose

Resolve the SP7gi concern that a strict `n_bits=20` strided-local NTT guard misses the large KeccakUnion `domain=65536` check bucket.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Finding

For smaller rows, the one-offset-per-workgroup shape from SP7gh is not the best extension. At `n_bits=16`, `blocks_per_row=64`, so a workgroup can process multiple intra-block offsets while keeping scratch at `1024` elements.

Use a tiled-offset shader:

```text
offsets_per_tile = min(16, 1024 / blocks_per_row)
scratch_elems = blocks_per_row * offsets_per_tile
```

For the guarded shapes, `1024 / blocks_per_row` is already a power of two, so no new helper is required. This keeps scratch bounded at `1024` elements and covers both:

- `n_bits=20`: `blocks_per_row=1024`, `offsets_per_tile=1`
- `n_bits=16`: `blocks_per_row=64`, `offsets_per_tile=16`

## Static Sizing

| Shape | blocks/row | offsets/tile | scratch elems | Current remaining groups | Candidate strided groups | Current total groups | Candidate total groups | Remaining RW |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `n_bits=16 count=16` | 64 | 16 | 1024 | 12288 | 1024 | 13312 | 2048 | 48 MiB -> 8 MiB |
| `n_bits=18 count=16` | 256 | 4 | 1024 | 65536 | 4096 | 69632 | 8192 | 256 MiB -> 32 MiB |
| `n_bits=18 count=4` | 256 | 4 | 1024 | 16384 | 1024 | 17408 | 2048 | 64 MiB -> 8 MiB |
| `n_bits=20 count=16` | 1024 | 1 | 1024 | 327680 | 16384 | 344064 | 32768 | 1280 MiB -> 128 MiB |
| `n_bits=20 count=4` | 1024 | 1 | 1024 | 81920 | 4096 | 86016 | 8192 | 320 MiB -> 32 MiB |

This makes the `domain=65536 count=16` extension more attractive than the naive one-offset-per-workgroup shape: remaining workgroups drop `12288 -> 1024`, not up.

## Scratch WGSL Validation

Historical scratch path:

```text
/tmp/sp7gj-strided-local-ntt-tiled.wgsl
```

This shader validated the tiled concept, but SP7gn supersedes it for runtime implementation handoff with `/tmp/sp7gn-strided-local-ntt-tiled-final.wgsl`.

The scratch shader is `4583` bytes and uses:

- `offsets_per_tile` in params;
- `tiles_per_row = 1024 / offsets_per_tile`;
- `scratch[lane * blocks_per_row + block]`;
- independent local butterflies per lane;
- `s_original = block_s * 1024 + intra`.

Validation command:

```text
naga --input-kind wgsl /tmp/sp7gj-strided-local-ntt-tiled.wgsl
```

Result:

```text
Validation successful
```

## Field Simulation

Compared full CPU-style forward NTT against the tiled strided-local form using BabyBear arithmetic:

```text
n_bits=16 fused=10 expand_bits=2 offset_tile=16 match=True output_len=65536
n_bits=18 fused=10 expand_bits=2 offset_tile=4 match=True output_len=262144
```

This is stronger than the earlier small analogue for the exact `domain=65536` KeccakUnion extension and the nearby `domain=262144` shape.

## Decision Impact

When browser validation returns, the runtime candidate should prefer the tiled-offset shader over the one-offset-only SP7gh scratch shape. It can still be guarded tightly:

```text
expand_bits == 2
n_bits == 20 && count in {4,16} && offsets_per_tile == 1
or
n_bits == 16 && count == 16 && offsets_per_tile == 16
```

If validation cycles are scarce, implement `n_bits=20` first, but do not accept default enablement unless representative KeccakUnion improves materially. The `n_bits=16` extension is the obvious next step if the first guard helps xgboost but leaves representative time dominated by the SP7gi `domain=65536` bucket.

Acceptance still requires the SP7gf browser proof gates.
