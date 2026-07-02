# SP7ez: rejected recursion Poseidon2 fastpath residue removed

Date: 2026-05-22

## Problem

While closing SP7ey, the current worktree still contained runtime markers from SP7ea:

- `recursion_poseidon_identity_fastpath_rows`
- `RECURSION_POSEIDON_IDENTITY_FASTPATH_ROWS`
- `is_poseidon_identity_accum_cycle`
- selector-column constants used to skip generated recursion accumulator rows

This contradicted the existing SP7ea state entry, which says the CPU-side recursion Poseidon2 accumulator fastpath was rejected, reverted, and marker-swept clean after xgboost regressed.

## Fix

Removed the rejected residue from:

- `risc0/circuit/recursion/src/prove/hal/rust_kernels.rs`
- `risc0/circuit/recursion/src/prove/mod.rs`

Marker sweep after cleanup:

```text
rg -n "poseidon_identity|identity_fastpath|recursion_poseidon_identity_fastpath|is_poseidon_identity_accum_cycle|CTRL_POSEIDON2_FULL_SELECTOR_COL|RECURSION_POSEIDON" risc0/circuit/recursion/src/prove examples/browser-prove/src/lib.rs .recursive/STATE.md .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/*.md
```

No matches.

## Validation

Formatting and diff hygiene:

- `cargo fmt --manifest-path examples/browser-prove/Cargo.toml`
- `git diff --check`

Representative e2e proof gates:

| Gate | Result |
|---|---:|
| xgboost proof wall | `63954 ms` |
| xgboost test runtime | `64.52s` |
| xgboost fallbacks / CPU-only | `0 / 0` |
| BusyLoop proof wall | `5665 ms` |
| KeccakUnion proof wall | `85171 ms` |
| BusyLoop + KeccakUnion test runtime | `91.58s` |
| BusyLoop + KeccakUnion fallbacks / CPU-only | `0 / 0` |

Raw logs:

- `2026-05-22-sp7ez-xgboost-post-rejected-residue-cleanup.chrome.txt`
- `2026-05-22-sp7ez-default-representative-post-rejected-residue-cleanup.chrome.txt`

## Decision

Accepted as cleanup of already-rejected runtime residue. No new optimization gain is claimed; the current accepted working state remains SP7ex recursion accumulator GPU default plus this cleanup.
