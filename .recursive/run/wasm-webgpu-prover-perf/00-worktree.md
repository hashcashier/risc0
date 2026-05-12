Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `00 Worktree`
Status: `LOCKED`
LockedAt: `2026-05-12T04:40:59Z`
LockHash: `71fbfa4cedfc54f30c9e59e28fd54a59a1197e7d1bdf29faeecc8ec029d8d536`
Inputs:
- `/.recursive/run/wasm-webgpu-prover-perf/00-requirements.md`
- Current git repository state
Outputs:
- `/.recursive/run/wasm-webgpu-prover-perf/00-worktree.md`
Scope note: This document records the Phase 0 worktree context and the executable diff basis that all later audited phases must reuse.

## TODO

- [x] Confirm the selected worktree location and isolation approach
- [x] Confirm the base branch and worktree branch values
- [x] Run setup and verify the clean test baseline
- [x] Confirm the diff basis fields still match live git state
- [x] Complete Coverage Gate checklist
- [x] Complete Approval Gate checklist

## Directory Selection

- Repository root: `/home/rami/repos/risc0`
- Selected worktree location: `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf/`
- Git-ignore verification: `.worktrees/` added to `/home/rami/repos/risc0/.gitignore` (line 23) so worktree contents stay out of the controller checkout. `git check-ignore -v .worktrees/foo` resolves to `.gitignore:23:.worktrees/`.

## Safety Verification

- Original branch observed at init time: `wasm` (controller checkout at `/home/rami/repos/risc0`).
- Controller worktree remained on `wasm` throughout setup; `git worktree list` confirms two worktrees on different branches.
- Worktree is fully isolated: `git -C /home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf status --short` returns empty after the baseline build, and the wasm artifact lives in the worktree-local `examples/target/` tree.

## Worktree Creation

- Command executed (from controller `/home/rami/repos/risc0`):
  ```
  git worktree add .worktrees/wasm-webgpu-prover-perf -b recursive/wasm-webgpu-prover-perf
  ```
- Output:
  ```
  Preparing worktree (new branch 'recursive/wasm-webgpu-prover-perf')
  HEAD is now at d042da45c recursive mode
  ```
- Resulting `git worktree list`:
  ```
  /home/rami/repos/risc0                                     d042da45c [wasm]
  /home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf  d042da45c [recursive/wasm-webgpu-prover-perf]
  ```
- Branch naming follows the recursive-worktree skill convention: `recursive/<run-id>` = `recursive/wasm-webgpu-prover-perf`.

## Main Branch Protection

- Base branch source of truth: `wasm` (the WebGPU prover feature branch, not `main` or `master`).
- The controller checkout never switches off `wasm`; all Phase 1+ work occurs inside `.worktrees/wasm-webgpu-prover-perf/` on the new `recursive/wasm-webgpu-prover-perf` branch.
- `main` and `master` remain untouched by this run.

## Project Setup

- Setup approach: rely on Cargo's incremental compilation by running the R1 baseline command directly, which transitively fetches dependencies and produces both the workspace setup state and the canonical wasm artifact. No separate `cargo fetch`/`cargo build` was needed.
- Command executed (from the worktree):
  ```
  cd /home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
    --target wasm32-unknown-unknown --release --no-run
  ```
- Result: exit code 0, finished in `13m 08s` (full clean compile because the worktree has no shared `examples/target/` with the controller checkout).
- Produced artifact: `examples/target/wasm32-unknown-unknown/release/deps/browser_prove-ea184bbfc45f01b0.wasm` (116,352,946 bytes). This artifact path replaces the controller's `browser_prove-0d71f73dbf7c3024.wasm` in all worktree-local Chrome runs.

## Test Baseline Verification

- Baseline gate (R1 acceptance: rebuild the wasm harness): the build above passes with exit code 0 and produces the wasm artifact. This is the regression line for every later phase.
- Native CUDA baselines for `risc0-zkvm-methods/cfg`, `hello-world`, `json`, `multi_test/poseidon2_basic`, `multi_test/libm`, and `multi_test/keccak_union_small` are explicitly deferred to Phase 1+ work (they exercise the proof path and belong in `01-as-is.md`/`02-to-be-plan.md` evidence rather than the worktree-bring-up baseline).
- Browser Chrome/WebGPU smoke runs are likewise deferred to Phase 1+ to keep Phase 0 short and to avoid spending the ChromeDriver/wasm-bindgen-test-runner budget before AS-IS analysis chooses the smoke-suite ordering.
- The R1 wasm build is sufficient as the Phase 0 baseline because it is the documented gate in `docs/wasm-webgpu-validation.md` and `docs/requirements/wasm-webgpu-prover.md` and because the controller checkout already has the previously validated `browser_prove-0d71f73dbf7c3024.wasm` artifact for comparison.

## Router State In Worktrees

- `/.recursive/config/recursive-router.json`: present in the worktree (carried in via the branch's tracked content).
- `/.recursive/config/recursive-router-discovered.json`: absent in the worktree (untracked per `.gitignore` line 22). This is expected; no delegated/routed work runs during Phase 0.
- Action item for Phase 2+: before invoking any routed/delegated role (e.g., delegated review), run `python3 /home/rami/.agents/skills/recursive-mode/scripts/recursive-router-probe.py --repo-root /home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf --json` from inside the worktree to refresh the discovery inventory, then record the route decision in the relevant phase artifact per the `recursive-worktree` skill.

## Worktree Context

- Base branch: `wasm`
- Worktree branch: `recursive/wasm-webgpu-prover-perf`
- Base commit: `d042da45c89a1cd5f9cf7c5eb962a754367b2118`
- Worktree HEAD verified (post-baseline): `d042da45c89a1cd5f9cf7c5eb962a754367b2118 recursive mode` (no commits added during Phase 0).
- Worktree status post-baseline: clean (build artifacts live under the gitignored `examples/target/` tree; `git status --short` returns empty).
- All subsequent phases (`01-as-is.md`, `02-to-be-plan.md`, `03-implementation-summary.md`, …) run from the worktree at `/home/rami/repos/risc0/.worktrees/wasm-webgpu-prover-perf/` and commit to `recursive/wasm-webgpu-prover-perf`.

## Diff Basis For Later Audits

- Baseline type: `local commit`
- Baseline reference: `d042da45c89a1cd5f9cf7c5eb962a754367b2118`
- Comparison reference: `working-tree`
- Normalized baseline: `d042da45c89a1cd5f9cf7c5eb962a754367b2118`
- Normalized comparison: `working-tree`
- Normalized diff command: `git diff --name-only d042da45c89a1cd5f9cf7c5eb962a754367b2118`
- Base branch: `wasm`
- Worktree branch: `recursive/wasm-webgpu-prover-perf`
- Diff basis notes: `recursive-init prefilled this diff basis from the controller HEAD commit d042da45c89a1cd5f9cf7c5eb962a754367b2118. The worktree branch was updated from the scaffold default (\`wasm\`) to \`recursive/wasm-webgpu-prover-perf\` after the Phase 0 worktree was created on the new branch. The normalized baseline commit and diff command remain executable from either branch because they are pinned to the commit hash.`

## Traceability

- Recursive workflow safety -> Phase 0 records a reusable executable diff basis before audited phases begin.

## Coverage Gate

- [X] Worktree location and branch context are recorded
- [X] Setup and clean baseline verification are recorded
- [X] Diff basis fields are executable against live git state

Coverage: PASS

## Approval Gate

- [X] Phase 0 context is ready for downstream audited phases
- [X] No unresolved setup or diff-basis inconsistencies remain

Approval: PASS
