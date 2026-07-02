# SP7gf Strided-Local NTT Validation Checklist

Date: 2026-05-22

## Purpose

Pin the exact validation sequence for the SP7gc/SP7ge strided-local NTT candidate so runtime work can resume directly when browser WebGPU validation is available again. SP7gj refined the candidate into a tiled-offset shader; this checklist treats KeccakUnion `n_bits=16` coverage as part of the representative gate, not as optional follow-up work.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Current Accepted Baseline

SP7fr is the comparison baseline:

- representative BusyLoop + KeccakUnion gate: `88.54s`
- BusyLoop: `wall_ms=5962`, `raw_compute_dispatches=613`, `queue_submits=168`
- KeccakUnion: `wall_ms=81813`, `raw_compute_dispatches=10900`, `queue_submits=3153`
- xgboost gate: `62.45s`
- xgboost: `wall_ms=61883`, `raw_compute_dispatches=9918`, `queue_submits=2803`

SP7fy is the hot-bucket attribution baseline:

- xgboost FRI round0 `drain_after_expand_evaluate_ntt`: `27242 ms` over `32` calls
- xgboost check-group `drain_after_batch_expand_into_evaluate_ntt`: `9143 ms` over `32` calls
- KeccakUnion FRI round0: `23718 ms`
- KeccakUnion check-group: `23009 ms` at `domain=65536` plus `4424 ms` at `domain=1048576`

## Focused Candidate Gate

Add a focused browser HAL test near `webgpu_hal_ntt_gpu_results_match_cpu` in `examples/browser-prove/src/lib.rs`.

Required assertions:

- candidate flag enabled;
- `batch_expand_into_evaluate_ntt` output matches CPU;
- marker upload/source appears:
  - `webgpu_batch_expand_strided_local_ntt_params`
- diagnostics show no CPU fallback and no CPU-only WebGPU HAL ops;
- for the `n_bits=20` hot-shape smoke, candidate activates only on:
  - `count=4`
  - `in_size=262144`
  - `out_size=1048576`
  - `expand_bits=2`
  - `offsets_per_tile=1`
- for the KeccakUnion `n_bits=16` hot-shape smoke, candidate activates only on:
  - `count=16`
  - `in_size=16384`
  - `out_size=65536`
  - `expand_bits=2`
  - `offsets_per_tile=16`

Focused command shape:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release <new_strided_local_ntt_test_name> -- --nocapture
```

If a full output readback is too expensive for the hot-shape smoke, keep full parity on a smaller shape and use the hot shape only for marker/perf proof that the branch compiles and activates. Full-shape correctness must still be covered by the e2e receipt gates below.

## Representative E2E Gate

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture
```

Required pass criteria:

- high WebGPU limits accepted by `assert_representative_webgpu_limits`;
- BusyLoop receipt verifies;
- KeccakUnion receipt verifies;
- `cpu_fallbacks=0`;
- `cpu_only_ops=0`;
- candidate marker appears in KeccakUnion FRI/check hot paths;
- if using the tiled shader, diagnostics prove both the `n_bits=20` and `n_bits=16` guarded shapes were exercised or explicitly explain why only one shape was reachable in that proof run;
- wall time improves materially versus SP7fr `88.54s`;
- KeccakUnion `wall_ms` improves materially versus SP7fr `81813 ms`;
- BusyLoop does not regress enough to erase the representative win.

## Xgboost E2E Gate

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture
```

Required pass criteria:

- high WebGPU limits accepted by `assert_representative_webgpu_limits`;
- succinct receipt verifies;
- journal remains `30.528042544062632`;
- `cpu_fallbacks=0`;
- `cpu_only_ops=0`;
- candidate marker appears;
- wall time improves materially versus SP7fr `62.45s` / `wall_ms=61883`.

## Drain Attribution Recheck

Use the existing default-off drain diagnostic hook around the same two proof gates, mirroring SP7fy.

Relevant existing focused diagnostic tests:

- `webgpu_prover_commit_group_drain_diagnostic_splits_queue_waits`
- `webgpu_prover_fri_drain_diagnostic_splits_round_work`

Required interpretation:

- FRI/check `drain_after_expand_evaluate_ntt` must move down;
- KeccakUnion `domain=65536` / `n_bits=16` check drain must move down when that guard is enabled;
- total e2e wall time must move down;
- raw dispatch/workgroup reductions alone are not acceptance evidence.

## Acceptance Decision

Accept the SP7gc runtime candidate only if all focused and e2e gates pass. The candidate should be rejected and removed if it is correct but flat, because SP7fr already has submit batching and the user priority is immediate significant wall-time reduction.

Do not accept a partial result based only on:

- shader parity;
- reduced raw dispatch count;
- reduced workgroup count;
- reduced estimated memory traffic;
- one workload improving while xgboost or KeccakUnion regresses materially.

Also do not accept an `n_bits=20`-only default enablement if the representative run remains dominated by the SP7gi KeccakUnion `domain=65536` bucket. An `n_bits=20`-only implementation can be kept only as a temporary, default-off validation step toward the tiled `n_bits=16` extension.
