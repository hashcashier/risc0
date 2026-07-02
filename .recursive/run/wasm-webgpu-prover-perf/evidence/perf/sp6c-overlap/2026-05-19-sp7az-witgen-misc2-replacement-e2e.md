# SP7az: GPU-Witgen MISC2 Replacement E2E

Date: 2026-05-19

## Scope

Moved GPU-witgen replacement from MISC0-only toward MISC0+MISC2
authoritative proof generation.

The target was correctness first: prove that MISC2 replacement can generate
succinct receipts across the representative browser WebGPU workloads before
using it as a performance lever.

## Bug Found

The first xgboost authoritative replacement run failed receipt verification on
segment index 8.

Targeted replacement-diff mode isolated the witness mismatch to MISC2/BGE rows:

```text
REPLACE_FINAL_SUMMARY segment_ord=8 total_cells=55312384
  mismatches=1773 rows=262144 cols=211
first mismatches: major=2 minor=0 col=30
replace=0x00000000 cpu=nonzero compare diff.low16 values
```

Root cause: `exec_misc2_chunk0_delta.wgsl` called only
`exec_ReadSourceRegsChunk0`, which covers the `rs1 == rs2` arm. xgboost segment
8 contains BGE rows with distinct source registers, so the GPU compare path was
missing source-register data and wrote zeros into compare witness cells.

## Fix

Added the missing distinct-register source path to the MISC2 chunk0 replacement
kernel:

- `exec_ReadSourceRegsChunk1`
- `exec_ReadSourceRegs_combined`
- `merge_ReadSourceRegsStruct`

`exec_MiscInput` now calls the combined source-register path.

MISC2 minor 1 remains CPU-covered for now:

```text
MISC2 replacement minors: 0, 2, 3, 4, 5, 6, 7
MISC2 minor 1: excluded until its nested source-reg mux is complete
```

The replacement-cycle gate in `webgpu.rs` and the CPU short-circuit gate in
`rust_steps.rs` were kept in sync so the CPU skips only rows whose GPU witness
cells and lookup-table side effects are covered.

## RED / GREEN

RED compile for the targeted xgboost segment selector failed as intended:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_diff_xgboost --no-run

error[E0432]: unresolved import
  `risc0_circuit_rv32im::prove::set_witgen_gpu_replace_diff_target_segment`
```

GREEN added `set_witgen_gpu_replace_diff_target_segment` plus replacement-diff
plumbing that proves earlier segments normally and stops on the requested
segment for final matrix diff.

The same compile-only command then passed.

## Diagnostic Proof

After the source-register fix, targeted xgboost segment 8 replacement diff was
clean:

```text
REPLACE_FINAL_SUMMARY segment_ord=8 total_cells=55312384
  mismatches=0 rows=262144 cols=211
```

The diagnostic test exits nonzero by design after logging the diff summary; the
zero-mismatch summary is the pass signal.

## E2E Proof Gates

All e2e proof-generation runs used high-limit Chrome WebGPU:

```text
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152
```

### xgboost

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_xgboost -- --nocapture
```

Result:

```text
test tests::iter6d_g_replace_xgboost ... ok

wall_ms=108174
segments=11
user_cycles=2294890
total_cycles=2883584
cpu_fallbacks=0
cpu_only_ops=0
source=data upload_bytes=5252317184
witgen_data_shadow_rows readbacks=33
witgen_data_shadow_rows readback_bytes=744245672
```

The stale xgboost sparse-readback ceiling was updated from 600 MB to 800 MB so
the test accepts the current MISC0+MISC2 row-set while still bounding the known
readback cost.

### BusyLoop po2=18

Same browser test command with `iter6d_g_replace_busy_loop_e2e_verify`.

Result:

```text
receipt verifies
wall_ms=16906
segments=1
user_cycles=202872
total_cycles=262144
cpu_fallbacks=0
cpu_only_ops=0
readback_bytes=58837712
```

### KeccakUnion(1)

Covered in the same `iter6d_g_replace_busy_loop_e2e_verify` browser test.

Result:

```text
receipt verifies
wall_ms=102078
segments=4
pending_keccaks=9
assumptions=1
user_cycles=747265
total_cycles=917504
cpu_fallbacks=0
cpu_only_ops=0
source=data upload_bytes=4776263680
witgen_data_shadow_rows readbacks=12
witgen_data_shadow_rows readback_bytes=117906612
```

## Non-Signal

This native command currently lists zero tests for the crate and is not counted
as validation:

```text
cargo test -p risc0-circuit-rv32im \
  misc2_cycles_are_short_circuitable_when_arm_mask_enabled

running 0 tests
```

The representative browser proof-generation gates above are the correctness
evidence for this step.

## Wall-Time Assessment

Accepted wall-time gain from this step: 0.

The correctness gap is closed for the promoted MISC2 replacement slice, but
MISC2 currently increases sync cost:

- xgboost is 108.174 s in this run, not faster than the recent 101-103 s range.
- `witgen_data_shadow_rows` rose to 744,245,672 bytes on xgboost.
- `source=data` upload remains 5,252,317,184 bytes on xgboost.

This makes MISC2 replacement a correctness milestone, not yet a performance
win. The immediate performance work should reduce shadow-row repair and
post-witgen data upload before broadening GPU-witgen coverage again.

