# M9a — keccak witgen/preflight pool offload

**Date:** 2026-07-04 · **Branch:** wasm-webgpu-prover-perf
**Thesis (post-M8 lever #1):** the keccak/union phase is ~95% main-thread
CPU and hosts the densest readback traffic in the rep/heavy fixtures
(40 `check_group` readbacks in KeccakUnion(1)). Each keccak proof opens
with three consecutive main-thread blocks — `PreflightTrace::new`
(untimed), the SERIAL `scatter_preflight` loop (~123 ms/proof), and the
main-driven witgen join (~42 ms/proof) — all pre-transcript, i.e.
phase-head, i.e. exactly the shape the M6d/M7a offloads won with and the
M8a physics blesses (no mid-transcript foreground wait rides the pool's
injector queue).

## Shape

`WebGpuKeccakProver::witgen_offloaded_async`: inputs (`Vec<KeccakState>`,
cheap copy) move to a `rayon::spawn` worker which builds AND consumes the
preflight, runs the scatter and the witness pass over `Send` CPU-shadow
handles (`MetaBuffer<CpuHal>` wrappers — the scatter body was extracted
to a HAL-generic `scatter_preflight_into`, mirroring the already-generic
`rust_steps::generate_witness`), and sends the result back through a
futures oneshot. Fresh buffers ⇒ shadows current by construction; both
`finish_cpu_shadow_offload_mut` calls happen on the main thread after the
await. Keccak has no eqz-elision machinery to guard (no GPU witgen
replacement in this circuit). The keccak crate gains the wasm-target
`futures` dep, mirroring rv32im/recursion.

## Same-session gates (the drift lesson applied)

All baselines re-measured back-to-back with the change tonight (the
morning M7-landing numbers are stale by +3-9% environment drift — see
the M8 evidence):

| gate | baseline (same session) | M9a | delta |
|---|---:|---:|---:|
| parity suite | 4/4 | 4/4 | — |
| BusyLoop | 2294 ms | 2309 ms | noise |
| KeccakUnion(1) | 38239 ms | **37405 ms** | **−834 ms (−2.2%)** |
| xgboost | (no keccaks — drift tracker) | 15109 ms | n/a |
| heavy 25-keccak | 109416 ms | **106415 ms** | **−3001 ms (−2.7%)** |

All receipts verified, `cpu_fallbacks=0` on every device. The xgboost
number is untouched by this change (its fixture proves zero keccaks) and
documents the continuing intra-day drift: 14509 (15:21) → 14982 (17:00)
→ 15109 (21:45) for equivalent-or-better code.

Landing re-gate on the exact formatted bytes (cargo fmt reflowed the
import block; per the M6c lesson formatted bytes = different binary):
numbers recorded in the landing commit.

## Notes

- The offloaded phase logs one main-side span
  (`keccak_witgen_phase ... offload=pool`); the inner scatter/witgen
  timers only fire on the sync path (worker console output is not
  captured by the test runner).
- Multiple keccak proofs' offloaded witgens can overlap on the pool
  (M4e fans keccak proofs across devices) — phase-head passes tolerate
  this per M6d/M7a precedent: spans stretch, wall wins.
- Remaining keccak-phase main-thread CPU after this: the union chain's
  recursion proofs (M7a-offloaded except zkp transcript/FRI residue) and
  the keccak proofs' own transcript/FRI — both mid-transcript, both
  gated on elimination-style ideas per the M8a physics.
