# SP7gg Strided-Local NTT Guard Audit

Date: 2026-05-22

## Purpose

Audit the SP7gc/SP7ge strided-local NTT candidate for static bounds, dispatch dimensions, and guard conditions before any runtime code is written.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Supersession Note

This audit covers the original `n_bits=20`, one-offset-per-workgroup guard. It remains useful for the `n_bits=20` twiddle and dispatch-dimension sanity check, but the final implementation handoff is the tiled-offset SP7gn/SP7go contract. Do not use this artifact as the final param-layout or shader-shape source.

## Dispatch Ordering

The existing WebGPU NTT implementation already batches all remaining `NTT_STEP_WGSL` stages in one compute pass and one queue submit. It relies on ordered dispatches inside the same compute pass:

```text
pass.set_pipeline(&ntt_kernel.pipeline)
for params_offset in params_offsets:
    pass.set_bind_group(...)
    pass.dispatch_workgroups(...)
```

The SP7gc candidate can reuse the same ordering model through `dispatch_compute_1d_bind_group_sequence`:

```text
local-prefix dispatch -> strided-local dispatch
```

This is important because the strided phase reads the `output` buffer written by the local-prefix phase. The candidate should not add a queue submit merely to create a dependency barrier.

## Bounds

For the first retained guard:

```text
FUSED_BITS = 10
FUSED_BLOCK_SIZE = 1024
n_bits = 20
blocks_per_row = 1024
count = 4 || count = 16
```

Static bounds:

```text
stage=11 t=1 block_s_size=1 max_s_original=1023 max_twiddle_idx=2046 table_len=1048575
stage=12 t=2 block_s_size=2 max_s_original=2047 max_twiddle_idx=4094 table_len=1048575
stage=13 t=3 block_s_size=4 max_s_original=4095 max_twiddle_idx=8190 table_len=1048575
stage=14 t=4 block_s_size=8 max_s_original=8191 max_twiddle_idx=16382 table_len=1048575
stage=15 t=5 block_s_size=16 max_s_original=16383 max_twiddle_idx=32766 table_len=1048575
stage=16 t=6 block_s_size=32 max_s_original=32767 max_twiddle_idx=65534 table_len=1048575
stage=17 t=7 block_s_size=64 max_s_original=65535 max_twiddle_idx=131070 table_len=1048575
stage=18 t=8 block_s_size=128 max_s_original=131071 max_twiddle_idx=262142 table_len=1048575
stage=19 t=9 block_s_size=256 max_s_original=262143 max_twiddle_idx=524286 table_len=1048575
stage=20 t=10 block_s_size=512 max_s_original=524287 max_twiddle_idx=1048574 table_len=1048575
```

The max stage-20 index is exactly the last valid entry in the `n_bits=20` forward twiddle table.

Dispatch dimensions:

```text
count=4 total strided workgroups=4096 fits x dimension=true
count=16 total strided workgroups=16384 fits x dimension=true
scratch bytes=4096
```

So the hot candidate does not need 2D dispatch spilling, though using the generic 1D helper is still fine.

## Guard Notes

The first candidate should require full blocks:

```text
row_size == 1 << 20
blocks_per_row == 1024
out_size == row_size
```

This avoids partial-block `active_size` handling in the strided phase. The accepted local-prefix shader supports partial blocks, but the strided second phase is only justified for the proof-shaped hot buckets and should not grow extra partial-row complexity before e2e evidence.

The candidate should use:

```text
total_offsets = count * 1024
if offset_linear >= total_offsets: return
row = offset_linear / 1024
intra = offset_linear - row * 1024
```

Even though hot dispatches fit the x dimension, retaining the total guard prevents stale y-dimension assumptions if the helper or guard changes later.

## Test Implications

The focused parity test should include:

- one smaller parity case to catch formula and writeback errors cheaply;
- one hot-shape marker or sampled-readback smoke for `count=4`, `n_bits=20`, `expand_bits=2`;
- representative e2e proof gates before any default enablement.

Do not generalize the production guard to smaller `n_bits` based only on the small parity case. The performance thesis is about the hot `n_bits=20` FRI/check buckets measured in SP7fy.

## Decision

The static guard audit found no immediate index, workgroup-storage, or dispatch-dimension blocker for the first SP7gc runtime candidate. The remaining risk is browser performance of strided global memory access, which can only be decided by the focused browser and e2e proof gates from SP7gf.
