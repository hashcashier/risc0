# SP7gh Strided-Local NTT WGSL Static Validate

Date: 2026-05-22

## Purpose

Check that the planned SP7gc/SP7ge strided-local NTT shader shape is syntactically valid WGSL before browser validation is available.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Scratch Shader

Scratch path:

```text
/tmp/sp7gh-strided-local-ntt.wgsl
```

The scratch shader implements the SP7ge shape:

- `@workgroup_size(256)`
- `var<workgroup> scratch: array<u32, 1024>`
- one workgroup per `(row, intra-block offset)`
- strided load/write:
  - `row_base + i * 1024 + intra`
- remaining stage loop:
  - `stage = 11..n_bits`
  - `s_original = block_s * 1024 + intra`
  - `twiddle = twiddles[twiddles_base + ((1 << (stage - 1)) - 1) + s_original]`

The scratch shader is `3795` bytes.

## Validation

Command:

```text
naga --input-kind wgsl /tmp/sp7gh-strided-local-ntt.wgsl
```

Result:

```text
Validation successful
```

## Interpretation

This removes a local WGSL syntax/validator blocker for the SP7gc runtime candidate. It does not validate browser/Dawn performance, full HAL parity, proof correctness, or e2e wall-time gains.

The runtime candidate remains blocked until browser proof validation is available. Acceptance still requires the SP7gf focused HAL gate plus BusyLoop, KeccakUnion, xgboost, and drain-attribution proof gates.
