# SP7fq recursion witgen GPU verify_mem default

Date: 2026-05-22

## Change

Accepted and default-enabled the recursion witness candidate that keeps recursion exec on CPU but moves WOM row generation, scatter/backfill, and generated `verify_mem` onto GPU-resident buffers.

The candidate path is enabled by default through `RECURSION_WITGEN_GPU_VERIFY_MEM_CANDIDATE_ENABLED = true`. Representative default tests now assert that BusyLoop, KeccakUnion, and xgboost dispatch the GPU recursion witness/verify_mem path and still complete with zero CPU fallback / CPU-only operations.

## Validation

Focused/compile/static gates:

- `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release --no-run recursion_witgen_gpu_verify_mem_candidate_representative_e2e_verify`
- `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release --no-run rv32im_default_representative_e2e_verify`
- `cargo fmt --manifest-path examples/browser-prove/Cargo.toml --check`
- `git diff --check`
- Source marker sweep for rejected `recursion_verify_mem` / preflight residue: clean.

Browser proof gates, all under high WebGPU limits:

- Opt-in representative: BusyLoop and KeccakUnion receipts verified; `test result: ok`, runtime `88.48s`.
- Opt-in xgboost: succinct receipt and journal verified; `test result: ok`, runtime `62.83s`.
- Promoted default representative: BusyLoop and KeccakUnion receipts verified; `test result: ok`, runtime `89.15s`.
- Promoted default xgboost: succinct receipt and journal `30.528042544062632` verified; `test result: ok`, runtime `62.46s`.

All accepted proof gates reported `cpu_fallbacks=0` and `cpu_only_ops=0`.

## Measurements

Compared with SP7fp:

| Workload | SP7fp wall | SP7fq default wall | Movement |
|---|---:|---:|---:|
| BusyLoop | `5689 ms` | `6159 ms` | `+470 ms` |
| KeccakUnion(1) | `84903 ms` | `82208 ms` | `-2695 ms` |
| BusyLoop + KeccakUnion test runtime | `91.32s` | `89.15s` | `-2.17s` |
| xgboost | `65070 ms` | `61841 ms` | `-3229 ms` |
| xgboost test runtime | `65.67s` | `62.46s` | `-3.21s` |

Current accepted default state:

- BusyLoop: `wall_ms=6159`, `gpu_active_ms=4681`
- KeccakUnion(1): `wall_ms=82208`, `gpu_active_ms=62858`
- xgboost: `wall_ms=61841`, `gpu_active_ms=47628`

Command cost increased:

| Workload | SP7fp raw dispatches | SP7fq raw dispatches | SP7fp submits | SP7fq submits |
|---|---:|---:|---:|---:|
| BusyLoop | `606` | `613` | `167` | `171` |
| KeccakUnion(1) | `10725` | `10900` | `3128` | `3228` |
| xgboost | `9771` | `9918` | `2782` | `2866` |

## Decision

Promote despite the BusyLoop regression because the representative combined gate and xgboost both improve under full receipt verification, and the path removes the rejected SP7fd/SP7ff sorted-row upload shape by keeping row generation/scatter/backfill/verify_mem GPU-resident.

The remaining immediate concern is command overhead: SP7fq buys multi-segment wins but adds dispatches/submits and regresses the single-segment BusyLoop case. Future work should either reduce that overhead or move to a larger hot bucket such as FRI/check `batch_expand_into_evaluate_ntt`.

## Evidence

- `2026-05-22-sp7fq-recursion-witgen-gpu-verify-mem-candidate-representative.chrome.txt`
- `2026-05-22-sp7fq-recursion-witgen-gpu-verify-mem-candidate-xgboost.chrome.txt`
- `2026-05-22-sp7fq-default-representative-recursion-witgen-gpu-verify-mem.chrome.txt`
- `2026-05-22-sp7fq-default-xgboost-recursion-witgen-gpu-verify-mem.chrome.txt`
