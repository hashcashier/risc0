# SP7ax: MISC1 GPU-Witgen Replacement Rejected

Date: 2026-05-19

## Scope

Tested the next bounded RV32IM GPU-witgen replacement slice beyond MISC0:
MISC1 immediate bitwise ops (`XORI`/`ORI`, major 1 minors 0/1). The goal was
to reuse the existing MISC1 chunk0 and generic chunk1 kernels, replay the
same common U16 lookup side effects as MISC0, and validate with browser
WebGPU e2e proof generation.

## RED

The e2e replacement tests were first tightened to require visible MISC1
short-circuit coverage via a missing `witgen_gpu_short_circuit_major_count`
API. The RED command failed at compile time as expected:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture

error[E0432]: unresolved import
  `risc0_circuit_rv32im::prove::witgen_gpu_short_circuit_major_count`
```

## Candidate Implementation

The prototype:

- exposed per-major short-circuit counters for the browser tests;
- extended `is_witgen_replace_cycle` / `cycle_short_circuited` to MISC1
  minors 0/1;
- compiled MISC1 replacement kernels on demand;
- replayed MISC1 common decode/finalize U16 lookup deltas;
- widened replacement sparse shadow readback so MISC1 rows were copied back
  before the post-witgen `source=data` upload.

## Failure

The first GREEN e2e proof generated a BusyLoop proof but failed segment
verification:

```text
iter6d_g_pre_witgen_dispatch_async mask=0x0003 dispatched_arms=[0, 1]
multi_test/busy_loop_po2_18_witgen_replace: async prove failed: verify segment

Caused by:
    verification indicates proof is invalid
```

A high-limit replace-diff diagnostic localized the failure before proof
verification:

```text
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152

REPLACE_FINAL_MISMATCH idx=10705950 row=220190 col=40 major=1 minor=0 replace=0x77fddde1 cpu=0x2ffdddfa
REPLACE_FINAL_MISMATCH idx=14899665 row=219601 col=56 major=1 minor=0 replace=0x57f1b405 cpu=0x37f1b409
REPLACE_FINAL_MISMATCH idx=15161809 row=219601 col=57 major=1 minor=0 replace=0x00000000 cpu=0x07fffeef
REPLACE_FINAL_MISMATCH idx=16210974 row=220190 col=61 major=1 minor=0 replace=0x47ffffe7 cpu=0x2ffdddfa
REPLACE_FINAL_SUMMARY total_cells=55312384 mismatches=27 rows=262144 cols=211
```

This falsifies the shadow-sync and lookup-replay hypotheses: the final
replacement witness cells themselves differ from the CPU witness for MISC1
`XORI`.

## Root Cause

The vendored MISC1 chunk kernels are not self-contained by opcode. MISC1
`exec_Misc1Chunk0` calls `exec_MiscInput`, which in that module calls
`exec_ReadSourceRegsChunk0`. For `XORI` rows where the decoded `rs1` differs
from the decoded `rs2` field, the CPU witness uses the other source-regs mux
arm, so the GPU chunk computes wrong source-register and writeback/memory
write cells.

Correct MISC1 replacement therefore needs a deeper kernel decomposition:
per-cycle dispatch by nested source-regs mux arm, or newly generated combined
MISC1 kernels that make the source-regs mux complete without corrupting
inactive rows. That is real GPU-witgen compiler/backend work, not an immediate
same-pattern slice.

## Decision

Rejected for default and rejected for the opt-in replacement path. The MISC1
replacement semantics and the test-only per-major counter API were backed out.
The only retained change from the attempt is a neutral generalization of the
on-demand replacement prewarm loop from "arm0 only" to "all arms selected by
`is_witgen_replace_cycle`"; with the accepted MISC0-only predicate this is
behaviorally equivalent and was covered by the post-backout e2e gates.

Do not keep expanding generated witgen replacement by assuming top-level opcode
chunks are complete. The next credible immediate wall-time target remains the
dense post-witgen `source=data` upload or a GPU-resident accumulation consumer.

## Post-Backout Browser E2E Proofs

BusyLoop + KeccakUnion representative gate:

```text
iter6d_g_replace_busy_loop_e2e_verify ... ok

BusyLoop po2=18:
  receipt verifies
  wall_ms=11971
  mask=0x0001 dispatched_arms=[0]
  source=data upload_bytes=355467264

KeccakUnion(1):
  receipt verifies
  wall_ms=103128
  segments=4 pending_keccaks=9 assumptions=1
  source=data upload_bytes=4776263680
  source=witgen_data_shadow_rows readback_bytes=67004128
```

xgboost representative gate:

```text
iter6d_g_replace_xgboost ... ok

xgboost:
  receipt verifies
  wall_ms=103575
  segments=11
  journal=30.528042544062632
  source=data upload_bytes=5252317184
  source=witgen_data_shadow_rows readback_bytes=406998464
```

## Wall-Time Assessment

No accepted wall-time gain. The failed MISC1 path was correctness-negative,
and the validated post-backout path remains the SP7aw MISC0-only replacement
shape. Current observed replacement timings remain behind the SP7an default
xgboost mean (`99692 ms`) and are dominated by dense `source=data` uploads.
