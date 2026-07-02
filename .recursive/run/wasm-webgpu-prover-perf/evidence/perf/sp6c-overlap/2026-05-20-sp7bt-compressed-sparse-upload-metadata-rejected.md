# SP7bt Compressed Sparse Upload Metadata Rejected

Date: 2026-05-20

## Candidate

Replace the legacy sparse upload metadata table (`values + ranges`) with compressed run/block metadata (`values + runs + blocks`) for:

- `webgpu_zeroize_sparse_*`
- `webgpu_zero_default_sparse_*`

The target was the remaining xgboost-dominant sparse upload streams after SP7br:

- `webgpu_zeroize_sparse_values` around 1.71 GB
- `webgpu_zeroize_sparse_ranges` around 655 MB
- `webgpu_zero_default_sparse_values` around 288 MB
- `webgpu_zero_default_sparse_ranges` around 161 MB

## RED / Diagnosis

Focused RED initially failed as expected against the legacy range table:

- `webgpu_zeroize_sparse_upload name=data values=213 ranges=131 sparse_bytes=1916 dense_bytes=16384`
- assertion failed because `webgpu_zeroize_sparse_ranges=1048`, expected compressed metadata.

The first compressed implementation passed the small focused fixture, but failed real proof generation:

- BusyLoop browser e2e proof failed before receipt verification with `verification indicates proof is invalid`.
- Partial failing run showed zero fallback/CPU-only, so this was a GPU correctness regression, not a fallback issue.
- Example partial metric: `webgpu_zeroize_sparse_upload name=data values=13074099 runs=3686368 blocks=115229 sparse_bytes=68885548 dense_bytes=221249536`.

Root cause: the kernel derived `block_idx = range_idx / 32`, but the encoder can start a new block early when destination gaps exceed `u16::MAX`. That made proof-shaped variable-size blocks map later ranges to the wrong block.

Focused repro after tightening the fixture:

- `webgpu_zeroize_sparse_upload name=data values=222 runs=132 blocks=6 sparse_bytes=1528 dense_bytes=524288`
- failed at `elem 90000: gpu=0 cpu=1127068791`.

## GREEN Attempts

Correctness fix 1: dispatch one 32-lane workgroup per metadata block.

- Focused sparse upload test: PASS.
- BusyLoop + KeccakUnion e2e: PASS.
- xgboost e2e: PASS.

Metrics:

- BusyLoop: `wall_ms=8405`, `upload_bytes=190825512`, zero fallback/CPU-only.
- KeccakUnion: `wall_ms=101889`, `upload_bytes=2701813668`, zero fallback/CPU-only.
- xgboost: `wall_ms=94932`, `gpu_active_ms=56683`, `gpu_idle_ratio=0.403`, `upload_bytes=2837159056`, zero fallback/CPU-only.

Correctness fix 2: pack eight 32-lane metadata blocks per 256-lane workgroup.

- Focused sparse upload test: PASS.
- BusyLoop + KeccakUnion e2e: PASS.
- xgboost e2e: PASS.

Metrics:

- BusyLoop: `wall_ms=8907`, `upload_bytes=190877992`, zero fallback/CPU-only.
- KeccakUnion: `wall_ms=101333`, `upload_bytes=2701924688`, zero fallback/CPU-only.
- xgboost: `wall_ms=94621`, `gpu_active_ms=56510`, `gpu_idle_ratio=0.403`, `upload_bytes=2837373340`, zero fallback/CPU-only.

## Decision

Rejected.

Compared to the latest working range-table state:

- xgboost latest working/post-revert baseline: `wall_ms=94385`, `upload_bytes=3193577424`.
- Best compressed xgboost: `wall_ms=94621`, `upload_bytes=2837373340`.
- Upload fell by about `356 MB`, but wall regressed by `236 ms`.

The project priority is proving wall time with absolute correctness. A metadata-only upload reduction that does not improve xgboost wall time is not accepted.

## Revert Validation

Candidate code/test changes were reverted to the legacy `values + ranges + params` sparse path.

Post-revert gates:

- Compile: `cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run` passed in `4m55s`.
- BusyLoop + KeccakUnion browser e2e: PASS.
  - BusyLoop: `wall_ms=8725`, `gpu_active_ms=4238`, `gpu_idle_ratio=0.514`, `upload_bytes=213557472`, zero fallback/CPU-only.
  - KeccakUnion: `wall_ms=102144`, `gpu_active_ms=69270`, `gpu_idle_ratio=0.322`, `upload_bytes=3007626548`, zero fallback/CPU-only.
- xgboost browser e2e: PASS.
  - `wall_ms=94980`, `segments=11`, `gpu_active_ms=56638`, `gpu_idle_ratio=0.404`, `upload_bytes=3193383376`, `readback_bytes=7488592`, zero fallback/CPU-only, journal `30.528042544062632`.

## Follow-Up

Do not continue sparse metadata compression as an immediate wall-time lever unless a new design reduces both metadata bytes and GPU/kernel overhead in xgboost. The next work should prioritize a confirmed xgboost-dominant blocker with direct wall-time upside.
