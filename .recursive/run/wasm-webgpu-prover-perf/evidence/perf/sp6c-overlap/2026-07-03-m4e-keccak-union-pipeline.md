# M4e: keccak/union phase pipelining + the CPU-bound finding

Date: 2026-07-03. Status: ACCEPTED — win scales with keccak count; the
measurement also redirects the keccak avenue from scheduling to CPU cuts.

## The change

`prove_session_async`'s keccak phase was a strictly serial loop on the main
HAL: 9 × (keccak proof + union insert) = 30.8 s of KeccakUnion(1)'s 44 s.
Replaced with the M3c scheduler pattern:

- Keccak proofs fan out over the dedicated recursion devices (width =
  available extras; falls back to serial-on-main without them), each future
  wrapped in `with_authoritative_context`.
- The union peak-stack consumes receipts **strictly in request order** — the
  binary-counter tree shape is protocol-fixed (the guest computes the same
  tree over request digests; reordering would change the root claim and break
  assumption resolution). The scheduler overlaps work, never consumption.
- Union inserts run as tasks inside the same `FuturesUnordered` (they own the
  peak stack for their turn and hand it back) so in-flight keccak futures keep
  being polled while a union proof awaits readbacks. A bare `await` on the
  union would stall every other future's CPU continuation — single-threaded
  executor.

## Gate results

| Gate | Before | M4e | Movement |
|---|---:|---:|---:|
| KeccakUnion(1) (representative) | 43867–44060 ms | **43725 ms** | ~−0.6% (bottom of band) |
| **Heavy keccak fixture** (11 segs + 25 keccaks) | 127.7 s (M3) | **119.5 s** | **−6.4%** |
| xgboost (no keccaks — must be flat) | 23237/23299 ms | 22950 ms | flat/noise ✓ |
| BusyLoop | 2587 ms | 2592 ms | flat ✓ |

All receipts verified, zero fallbacks/CPU-only on all devices.

## The finding: the keccak phase is wasm-thread CPU-bound

Span timestamps prove the pipeline engaged (requests 0+1 in flight together,
pairs throughout) — but paired keccak proofs each take ~2× the serial
duration (~6.3 s vs ~3.4 s), and interleaved `union_async` spans stretch from
~0.6 s to ~4 s. One wasm thread serializes all CPU; device concurrency only
reclaims GPU-wait, which measured ~1.2 s of the 31.4 s phase in KeccakUnion(1)
(hence the small headline win there) and ~8 s in the 25-keccak fixture.

Implication for the next keccak lever: **cut keccak-proof CPU**, not
scheduling. Prime suspect: non-authoritative CPU mirrors inside
`keccak_prover_with_hal(...).prove_async` (the same mirror-discipline cost
found in recursion witgen). Once per-proof CPU shrinks, this scheduler's
overlap pays proportionally more.
