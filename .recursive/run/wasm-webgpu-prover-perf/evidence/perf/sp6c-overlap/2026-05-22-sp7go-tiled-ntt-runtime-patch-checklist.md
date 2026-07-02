# SP7go Tiled NTT Runtime Patch Checklist

Date: 2026-05-22

## Purpose

Pin the exact runtime patch sequence for the SP7gj/SP7gn tiled strided-local NTT candidate so implementation can start directly when browser WebGPU proof validation is available again.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Objective

Reduce FRI/check `batch_expand_into_evaluate_ntt` wall time by replacing the current local-prefix plus many global `NTT_STEP_WGSL` passes with:

1. the accepted local-10 prefix;
2. one tiled strided-local second phase;
3. one compute pass / one submit through `dispatch_compute_1d_bind_group_sequence`.

This targets the current dominant SP7fy bucket:

- xgboost FRI/check drains: `27242 ms + 9143 ms`;
- KeccakUnion FRI/check drains: `23718 ms + 23009 ms + 4424 ms`.

## Edit Surface

Primary production file:

```text
risc0/zkp/src/hal/webgpu.rs
```

Relevant current anchors:

- `BATCH_EXPAND_LOCAL_NTT_WGSL` near line `4028`;
- `NTT_STEP_WGSL` near line `4185`;
- `dispatch_compute_1d_bind_group_sequence` near line `10347`;
- `dispatch_batch_expand_into_evaluate_ntt` near line `12535`.

Focused browser test surface:

```text
examples/browser-prove/src/lib.rs
```

Relevant current anchors:

- `webgpu_hal_ntt_gpu_results_match_cpu` near line `786`;
- `rv32im_default_representative_e2e_verify` near line `9502`;
- `xgboost_succinct_receipt_verifies` near line `11865`.

## Runtime Patch Steps

1. Add the SP7gn shader as:

```text
BATCH_EXPAND_STRIDED_LOCAL_NTT_WGSL
```

Use scratch source:

```text
.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7gn-strided-local-ntt-tiled-final.wgsl
```

Do not use the older `/tmp/sp7gj-strided-local-ntt-tiled.wgsl`; it validated, but it does not match the final 48-byte param layout. The original `/tmp/sp7gn-strided-local-ntt-tiled-final.wgsl` was copied into the run evidence and verified byte-identical, so the durable path above is the source of truth.

2. Add an opt-in/candidate gate.

The branch must remain default-off or test/candidate gated until all browser gates pass. The initial runtime guard should require:

```text
actual_expand_bits == 2
local_ntt_fused_bits == 10
row_size == 1 << n_bits
blocks_per_row == 1 << (n_bits - 10)
offsets_per_tile == min(16, 1024 / blocks_per_row)
blocks_per_row * offsets_per_tile <= 1024
((n_bits == 20 && offsets_per_tile == 1 && (count == 4 || count == 16)) ||
 (n_bits == 16 && offsets_per_tile == 16 && count == 16))
max_compute_workgroup_storage_size >= 4096
output_gpu != input_gpu
storage_binding_fits(output)
storage_binding_fits(input)
storage_binding_fits(twiddles)
```

3. Restructure `dispatch_batch_expand_into_evaluate_ntt` candidate path.

Current code creates the local-prefix kernel and bind group, then immediately calls:

```text
self.dispatch_compute_1d(&expand_kernel, &expand_bind_group, expand_workgroups);
```

The candidate path must not do that. It should:

- create the existing local-prefix params, kernel, and bind group unchanged;
- if the tiled guard does not match, keep the current immediate dispatch and dynamic `NTT_STEP_WGSL` path unchanged;
- if the tiled guard matches, create the strided params, layout, kernel, and bind group before any dispatch;
- submit both dispatches together with `dispatch_compute_1d_bind_group_sequence`;
- return `Ok(true)` and skip the dynamic `NTT_STEP_WGSL` loop.

4. Pack strided params as 12 `u32` / 48 bytes:

```text
[
  out_size,
  count,
  output_base,
  twiddles_base,
  n_bits,
  blocks_per_row,
  offsets_per_tile,
  tiles_per_row,
  total_strided_groups,
  0,
  0,
  0,
]
```

Bind group:

```text
binding 0: output storage read-write
binding 1: twiddles read-only storage
binding 2: uniform(48)
```

Labels:

```text
webgpu_batch_expand_strided_local_ntt_layout
webgpu_batch_expand_strided_local_ntt_params
webgpu_batch_expand_strided_local_ntt
webgpu_batch_expand_strided_local_ntt_bind_group
```

5. Compute workgroups:

```text
total_local_prefix_groups = count * blocks_per_row
tiles_per_row = 1024 / offsets_per_tile
total_strided_groups = count * tiles_per_row
```

Dispatch:

```text
self.dispatch_compute_1d_bind_group_sequence(&[
    (&expand_kernel, &expand_bind_group, total_local_prefix_groups),
    (&strided_kernel, &strided_bind_group, total_strided_groups),
]);
```

6. Add focused diagnostics/marker coverage.

At minimum, the focused browser test must assert the upload source:

```text
webgpu_batch_expand_strided_local_ntt_params
```

Expected focused hot shapes:

- `n_bits=16`, `count=16`, `in_size=16384`, `out_size=65536`, `expand_bits=2`, `offsets_per_tile=16`;
- `n_bits=20`, `count=4`, `in_size=262144`, `out_size=1048576`, `expand_bits=2`, `offsets_per_tile=1`.

If full `n_bits=20` readback is too slow in focused parity, use smaller parity plus full-shape marker coverage, then rely on receipt-verified e2e for full-shape correctness.

## Verification Sequence

Do not retain the runtime patch unless each gate passes.

1. Focused HAL parity/marker test near `webgpu_hal_ntt_gpu_results_match_cpu`.

Command shape:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release <new_strided_local_ntt_test_name> -- --nocapture
```

Required:

- CPU parity passes;
- `webgpu_batch_expand_strided_local_ntt_params` appears;
- `cpu_fallbacks=0`;
- `cpu_only_ops=0`;
- no WebGPU validation error or device loss.

2. Representative BusyLoop + KeccakUnion proof gate.

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture
```

Required:

- high WebGPU limits accepted by `assert_representative_webgpu_limits`;
- BusyLoop receipt verifies;
- KeccakUnion receipt verifies;
- `cpu_fallbacks=0`;
- `cpu_only_ops=0`;
- candidate marker appears in FRI/check hot paths;
- representative wall improves materially versus SP7fr `88.54s`;
- KeccakUnion wall improves materially versus SP7fr `81813 ms`;
- `n_bits=16` KeccakUnion bucket is exercised when the tiled guard is enabled.

3. xgboost proof gate.

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture
```

Required:

- high WebGPU limits accepted;
- succinct receipt verifies;
- journal remains `30.528042544062632`;
- `cpu_fallbacks=0`;
- `cpu_only_ops=0`;
- candidate marker appears;
- wall improves materially versus SP7fr `62.45s` / `wall_ms=61883`.

4. Drain attribution recheck.

Mirror SP7fy with `poly_group_drain_diagnostic_enabled=true`.

Required:

- xgboost FRI/check `drain_after_expand_evaluate_ntt` bucket moves down;
- KeccakUnion `domain=65536` / `n_bits=16` check drain moves down;
- total e2e wall time moves down.

## Rejection Criteria

Reject and remove the patch if any of these occur:

- verifier rejection;
- panic before receipt;
- Chrome device loss;
- WebGPU validation error;
- any new `cpu_fallbacks` or `cpu_only_ops`;
- focused parity depends on CPU fallback;
- wall time is flat or worse despite fewer dispatch/workgroup counts;
- xgboost improves but representative KeccakUnion remains dominated by the `domain=65536` bucket.

## Expected Upside

If browser strided memory behavior is favorable, expected wall-time reduction remains:

- representative BusyLoop + KeccakUnion: roughly `8-15%`;
- xgboost: roughly `8-15%`;
- aggressive `15-25%` only if this exposes additional FRI/check or GPU-resident recursion follow-on wins.

These are estimates only. Accepted gains require receipt-verified e2e measurements.
