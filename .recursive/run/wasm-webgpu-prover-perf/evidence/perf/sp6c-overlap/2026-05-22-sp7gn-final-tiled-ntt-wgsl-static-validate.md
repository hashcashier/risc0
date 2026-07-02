# SP7gn Final Tiled NTT WGSL Static Validate

Date: 2026-05-22

## Purpose

Validate the reconciled SP7gj/SP7gl tiled strided-local NTT shader shape before any production runtime code is written.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Scratch Shader

Durable shader path:

```text
.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7gn-strided-local-ntt-tiled-final.wgsl
```

Size:

```text
4820 .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7gn-strided-local-ntt-tiled-final.wgsl
```

This shader supersedes the earlier `/tmp/sp7gj-strided-local-ntt-tiled.wgsl` for implementation handoff because it matches the SP7ge/SP7gl reconciled host contract:

- 12-word / 48-byte uniform params;
- `row_count`;
- `offsets_per_tile`;
- `tiles_per_row`;
- `total_tiles`;
- `total_elems <= 1024` guard before scratch use;
- `tile_linear >= total_tiles` guard for 1D dispatch helper overrun;
- same `scratch[lane * blocks_per_row + block]` tiled layout;
- same `s_original = block_s * 1024 + intra` twiddle lookup.

Param layout:

```text
struct Params {
    out_size: u32,
    row_count: u32,
    output_base: u32,
    twiddles_base: u32,
    n_bits: u32,
    blocks_per_row: u32,
    offsets_per_tile: u32,
    tiles_per_row: u32,
    total_tiles: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};
```

## Validation

Command:

```text
naga --input-kind wgsl /tmp/sp7gn-strided-local-ntt-tiled-final.wgsl
```

Result:

```text
Validation successful
```

Persistent-copy validation:

```text
naga --input-kind wgsl .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7gn-strided-local-ntt-tiled-final.wgsl
Validation successful
```

The persistent copy was verified byte-identical to the original `/tmp` scratch shader with:

```text
cmp -s /tmp/sp7gn-strided-local-ntt-tiled-final.wgsl .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7gn-strided-local-ntt-tiled-final.wgsl
```

## Implementation Handoff

When browser validation returns, the production candidate should add this shader shape as:

```text
BATCH_EXPAND_STRIDED_LOCAL_NTT_WGSL
```

The host side should build both the existing local-prefix bind group and this strided bind group before dispatch, then use:

```text
dispatch_compute_1d_bind_group_sequence(&[
    (&expand_kernel, &expand_bind_group, total_local_prefix_groups),
    (&strided_kernel, &strided_bind_group, total_strided_groups),
])
```

The candidate must remain default-off or candidate-gated until SP7gf browser HAL parity and representative proof gates pass.

## Remaining Risk

Naga validation only proves local WGSL syntax/validation. It does not prove:

- browser proof correctness;
- Chrome/Dawn runtime behavior;
- performance improvement;
- no verifier regression;
- no new fallback/CPU-only path.

Acceptance still requires focused browser HAL parity plus BusyLoop + KeccakUnion and xgboost proof generation with verified receipts, zero fallback/CPU-only, and material wall-time reduction versus SP7fr.
