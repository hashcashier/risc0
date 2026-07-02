# SP7ge Strided-Local NTT Implementation Blueprint

Date: 2026-05-22

## Purpose

Capture the SP7gc runtime candidate shape while browser WebGPU validation remains blocked. SP7gj refines the original one-offset-per-workgroup shape into a tiled-offset shader that also covers the KeccakUnion `n_bits=16` check bucket, so this blueprint now describes that refined candidate. This is an implementation blueprint only; no production runtime code changed.

Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Code Entry Points

Primary implementation surface:

- `risc0/zkp/src/hal/webgpu.rs`
  - `BATCH_EXPAND_LOCAL_NTT_WGSL`
  - `NTT_STEP_WGSL`
  - `WebGpuHal::dispatch_batch_expand_into_evaluate_ntt`
  - `WebGpuHal::dispatch_compute_1d_bind_group_sequence`

Focused browser HAL parity surface:

- `examples/browser-prove/src/lib.rs`
  - `webgpu_hal_ntt_gpu_results_match_cpu`
  - `assert_gpu_buffer_matches_cpu`
  - `assert_representative_webgpu_limits`

Representative proof gates:

- default BusyLoop + KeccakUnion gate used by SP7fr
- default xgboost gate used by SP7fr

## Runtime Guard

First retained implementation should be default-off or candidate-gated until e2e proof gates pass. The candidate branch should only activate for proof-shaped full-block hot paths:

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

The original `n_bits=20`-only guard is no longer sufficient for acceptance because SP7gi showed it misses the large KeccakUnion `domain=65536` / `n_bits=16` check drain. If validation cycles force an `n_bits=20` implementation first, do not default-enable it unless representative KeccakUnion improves materially or the `n_bits=16` extension follows in the same validation window.

## Shader Shape

Add a second WGSL kernel after the accepted local-10 prefix:

```text
BATCH_EXPAND_STRIDED_LOCAL_NTT_WGSL
```

Suggested params:

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

Keep this as a 12-word / 48-byte uniform, matching the existing local-prefix param layout style.

Suggested workgroup mapping:

```text
tile_linear = workgroup_id.x + workgroup_id.y * 65535
if (tile_linear >= total_tiles) return
row = tile_linear / tiles_per_row
tile = tile_linear - row * tiles_per_row
intra_base = tile * offsets_per_tile
```

Each workgroup handles one `(row, intra-block offset tile)` pair. For each tile lane and local-prefix block, load one element:

```text
intra = intra_base + lane
scratch[lane * blocks_per_row + block] =
    output[output_base + row * out_size + block * 1024 + intra]
```

Then it performs the remaining stages locally over `block in 0..blocks_per_row` independently for each lane:

```text
for t in 1..=(n_bits - 10):
    stage = 10 + t
    block_s_size = 1 << (t - 1)
    for pair in 0..(blocks_per_row / 2):
        g = pair / block_s_size
        block_s = pair - g * block_s_size
        idx1 = g * 2 * block_s_size + block_s
        idx2 = idx1 + block_s_size
        s_original = block_s * 1024 + intra
        twiddle = twiddles[twiddles_base + ((1 << (stage - 1)) - 1) + s_original]
        butterfly(
            scratch[lane * blocks_per_row + idx1],
            scratch[lane * blocks_per_row + idx2],
            twiddle,
        )
    workgroupBarrier()
```

Finally write back to the same strided locations.

Use `var<workgroup> scratch: array<u32, 1024>;` and `@workgroup_size(256)`, matching the accepted local-prefix kernel. Required workgroup storage is `4096` bytes. The final SP7gn shader persisted at `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7gn-strided-local-ntt-tiled-final.wgsl` validates with `naga` and supersedes the earlier SP7gj scratch shader for implementation handoff.

## Host Dispatch Shape

Build both kernels and bind groups before dispatch:

1. existing `webgpu_batch_expand_local_ntt`
2. new `webgpu_batch_expand_strided_local_ntt`

For the candidate branch, compute:

```text
total_local_prefix_groups = count * blocks_per_row
total_strided_groups = count * tiles_per_row
total_tiles = total_strided_groups
```

Then use the existing helper:

```text
dispatch_compute_1d_bind_group_sequence(&[
    (&expand_kernel, &expand_bind_group, total_local_prefix_groups),
    (&strided_kernel, &strided_bind_group, total_strided_groups),
])
```

This preserves the local-prefix dependency ordering in the same compute pass and avoids adding a queue submit. The current code already relies on ordered dispatches inside one compute pass for batched `NTT_STEP_WGSL` stages.

After this candidate branch dispatches, return `Ok(true)` and bypass the current dynamic `NTT_STEP_WGSL` loop.

## Expected Command And Workgroup Delta

Compared with SP7fr for hot calls:

| Shape | Current raw dispatches/call | Candidate raw dispatches/call | Current total workgroups/call | Candidate total workgroups/call | Remaining RW/call delta |
|---|---:|---:|---:|---:|---:|
| KeccakUnion check `n_bits=16 count=16` | 7 | 2 | 13312 | 2048 | 48 MiB -> 8 MiB |
| FRI round0 `count=4` | 11 | 2 | 86016 | 8192 | 320 MiB -> 32 MiB |
| check group `count=16` | 11 | 2 | 344064 | 32768 | 1280 MiB -> 128 MiB |

Queue submits can stay at `1` for the whole expand-plus-strided candidate branch if `dispatch_compute_1d_bind_group_sequence` is used. The current path uses one submit for the local prefix and one submit for all remaining global stages.

The main claim is still memory traffic/workgroup reduction, not submit-count reduction.

## Required Focused Browser Tests

Minimum focused tests before e2e:

1. RED marker test:
   - Enable candidate flag.
   - Run `batch_expand_into_evaluate_ntt`.
   - Assert the expected marker is missing on the pre-candidate path.

2. GREEN parity test:
   - Assert candidate marker upload source appears:
     - `webgpu_batch_expand_strided_local_ntt_params`
   - Compare output against CPU using `assert_gpu_buffer_matches_cpu`.
   - Prefer a smaller parity shape first if full `n_bits=20` parity is too slow.

3. Full-shape smoke:
   - `count=4`, `in_size=262144`, `out_size=1048576`, `expand_bits=2`
   - `count=16`, `in_size=16384`, `out_size=65536`, `expand_bits=2`
   - If full output readback is too slow, use a marker/perf probe plus a smaller parity test, then rely on representative proof gates for full-shape correctness.

## Required E2E Acceptance Gates

Do not retain the runtime candidate unless all pass under high WebGPU limits:

1. BusyLoop + KeccakUnion default proof gate:
   - receipts verify;
   - `cpu_fallbacks=0`;
   - `cpu_only_ops=0`;
   - candidate marker appears in the hot FRI/check path, including the KeccakUnion `n_bits=16` check bucket when the tiled shader is enabled;
   - wall time materially improves versus SP7fr, not just dispatch counts.

2. xgboost default proof gate:
   - succinct receipt/journal verifies;
   - `cpu_fallbacks=0`;
   - `cpu_only_ops=0`;
   - candidate marker appears;
   - wall time materially improves versus SP7fr.

3. Drain attribution recheck:
   - `poly_group_drain_diagnostic_enabled=true`;
   - SP7fy FRI/check `drain_after_expand_evaluate_ntt` bucket moves down.

## Rejection Criteria

Reject and remove the candidate if any of these happen:

- any verifier rejection, panic before receipt, device loss, or WebGPU validation error;
- any new `cpu_fallbacks` or `cpu_only_ops`;
- proof-shaped parity depends on CPU fallback or CPU shadow upload behavior;
- wall time is flat or worse on representative e2e despite reduced raw dispatch/workgroup counts;
- strided memory access moves the drain bucket sideways rather than down.

## Decision

When browser validation is available, this is the next runtime candidate to implement. It has static twiddle correctness evidence from SP7gd and targets the largest measured SP7fy bucket. If it fails for strided-memory reasons, the next NTT design should preserve coalesced access, likely via a transpose/six-step shape, but that should remain deferred until this cheaper candidate is e2e-proven negative.
