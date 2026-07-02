# SP7gl Strided-Local NTT Host Integration Preflight

Date: 2026-05-22

## Purpose

Record the exact host-side integration constraints for the SP7gj tiled strided-local NTT candidate while browser WebGPU validation remains blocked.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Current Code Shape

Relevant code is in `risc0/zkp/src/hal/webgpu.rs`:

- `BATCH_EXPAND_LOCAL_NTT_WGSL` starts near line `4028`.
- `NTT_STEP_WGSL` starts near line `4185`.
- `dispatch_compute_1d_bind_group_sequence` starts near line `10347`.
- `dispatch_batch_expand_into_evaluate_ntt` starts near line `12535`.

Current `dispatch_batch_expand_into_evaluate_ntt`:

- computes `out_size`, `in_size`, `actual_expand_bits`, `row_size`, `n_bits`, and `blocks_per_row`;
- creates the local-prefix params as 12 `u32` values / 48 bytes;
- creates `webgpu_batch_expand_local_ntt` and its bind group;
- immediately dispatches the local prefix with `dispatch_compute_1d`;
- if `n_bits > local_ntt_fused_bits`, builds dynamic `NTT_STEP_WGSL` params and dispatches the remaining global stages in one compute pass / one queue submit.

The SP7gj candidate must not keep the immediate local-prefix dispatch on the candidate path. It needs to create the local-prefix bind group and the strided bind group first, then submit both with `dispatch_compute_1d_bind_group_sequence`.

## Candidate Host Shape

For the guarded candidate path:

1. Reuse the existing local-prefix params and bind group unchanged.
2. Compute:
   - `blocks_per_row = 1 << (n_bits - 10)`
   - `offsets_per_tile = min(16, 1024 / blocks_per_row)`
   - `tiles_per_row = 1024 / offsets_per_tile`
   - `total_local_prefix_groups = count * blocks_per_row`
   - `total_strided_groups = count * tiles_per_row`
3. Build a new strided bind group:
   - binding 0: `output` storage read-write
   - binding 1: `twiddles` read-only storage
   - binding 2: 48-byte uniform params
4. Pack strided params as 12 `u32` values:

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

5. Dispatch:

```text
dispatch_compute_1d_bind_group_sequence(&[
    (&expand_kernel, &expand_bind_group, total_local_prefix_groups),
    (&strided_kernel, &strided_bind_group, total_strided_groups),
])
```

6. Return `Ok(true)` and skip the dynamic `NTT_STEP_WGSL` loop.

Non-candidate shapes should keep the existing dispatch order unchanged.

## Static Fit Checks

Initial guarded shapes fit the existing 1D dispatch helper without requiring 2D splitting:

| Shape | Local-prefix groups | Strided groups | Fits x dimension |
|---|---:|---:|---|
| `n_bits=16 count=16` | 1024 | 1024 | yes |
| `n_bits=20 count=4` | 4096 | 4096 | yes |
| `n_bits=20 count=16` | 16384 | 16384 | yes |

`dispatch_compute_1d_bind_group_sequence` already supports larger future shapes by splitting `workgroups_x` / `workgroups_y` at `WEBGPU_MAX_WORKGROUPS_PER_DIMENSION = 65535`.

The tiled shader scratch stays at 1024 `u32` values / 4096 bytes for the guarded shapes. That fits both the already asserted `max_compute_workgroup_storage_size >= 4096` candidate guard and the high-limit browser gate used by the representative tests.

## Twiddle Bounds

The tiled twiddle formula remains within the forward twiddle table:

- `n_bits=16`: last stage base `32767`, max `s_original = 31 * 1024 + 1023 = 32767`, max twiddle index `65534`, which is the last valid index for a `2^16` table.
- `n_bits=20`: last stage base `524287`, max `s_original = 511 * 1024 + 1023 = 524287`, max twiddle index `1048574`, which is the last valid index for a `2^20` table.

## Decision

The next runtime implementation should treat this host shape as the concrete insertion contract. The candidate must be default-off or candidate-gated and cannot be retained without the SP7gf focused parity, representative BusyLoop + KeccakUnion, xgboost, and drain-attribution gates.
