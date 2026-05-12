# DECISIONS.md

## Recursive Run Index

- `wasm-webgpu-prover-perf` (started 2026-05-12) — performance follow-up to the paused `docs/requirements/wasm-webgpu-prover.md` correctness run. Drives WebGPU browser prover wall-time toward 1.0× native CUDA via 11 ordered sub-phases (SP1–SP11) covering circuit-specific staged WGSL eval_check for rv32im/recursion/keccak, tiled GPU-resident recursion data group, GPU-side witness+accumulation, bounded transcript readbacks, pipeline+bind-group cache, and tiled gather_sample. Phase 0–4 LOCKED in this run iteration; SP2–SP11 production implementation deferred to follow-on runs.
  - Phase 0 Requirements LockHash: `d98a93e75bc9122b110d42e7c33ecc66d60b438828e585babdf5a9b35ba042fc`
  - Phase 0 Worktree LockHash: `71fbfa4cedfc54f30c9e59e28fd54a59a1197e7d1bdf29faeecc8ec029d8d536`
  - Phase 1 AS-IS LockHash: `6ae676746bbd68fe6d9b724d493cdcfa968d04235ae8e40294c3324da67a743d`
  - Phase 2 TO-BE LockHash: `25aced2df2cb0420801faa3e0e6b0666aeedf1f5cd09803df25180d77a146a92`
  - Phase 3 Implementation LockHash: `a6399a2c129508d9eda16edc82aa1c857c689bd21f38d207fabb80fe5fc3a485`
  - Phase 4 Test Summary LockHash: `3082c7320caf49e1ab282fbef37601f408397e17666131b2db4512bda06bbf69`
  - Diff basis (Phase 0): `git diff --name-only d042da45c89a1cd5f9cf7c5eb962a754367b2118`
  - Worktree: `.worktrees/wasm-webgpu-prover-perf/` on branch `recursive/wasm-webgpu-prover-perf`
