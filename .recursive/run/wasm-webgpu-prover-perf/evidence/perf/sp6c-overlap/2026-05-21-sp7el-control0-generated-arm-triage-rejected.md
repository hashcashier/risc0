# SP7el - CONTROL0 Generated Arm Triage Rejected

Date: 2026-05-21

## Purpose

After SP7ek rejected further sparse-gap widening, the remaining RV32IM
TopAccum histogram was rechecked before starting another generated-arm
candidate. The large unhandled xgboost bucket is `major7`, which is
`CONTROL0` in `execute::platform::major`, not `SHA0`.

## Evidence

The local generator/profile guard was rerun without changing runtime code:

```text
rustc --edition=2021 --test risc0/circuit/rv32im/src/prove/wgsl_pruner.rs -o /tmp/wgsl_pruner_test_sp7el
/tmp/wgsl_pruner_test_sp7el topaccum_arm_generator_reproduces_vendored_arm5 topaccum_arm_generator_profiles_all_arms --nocapture
```

Result:

```text
running 2 tests
arm generated_bytes nonblank_lines ext_inv_calls xgboost_cycles xgboost_inv_items
test tests::topaccum_arm_generator_reproduces_vendored_arm5 ... ok
0 207447 1308 25 768461 19211525
1 212711 1359 25 173854 4346350
2 126936 1197 25 399587 9989675
3 194314 1708 37 65151 2410587
4 282275 2398 47 5203 244541
5 112962 1114 26 430888 11203088
6 126258 1163 29 351857 10203853
7 201625 1625 58 388742 22547036
8 125791 1197 26 74670 1941420
9 741353 3593 52 30308 1576016
10 157201 1205 2 194571 389142
11 423126 2015 17 292 4964
12 267381 1902 42 0 0
test tests::topaccum_arm_generator_profiles_all_arms ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.17s
```

The same session also reproduced that package-level Cargo feature testing is
not the valid guard for this file in the current worktree:

```text
cargo test -p risc0-circuit-rv32im --features prove topaccum_arm_generator_profiles_all_arms -- --nocapture
```

failed on unrelated host feature-dependency wiring (`rayon`, `smallvec`,
`risc0_circuit_rv32im_sys`, etc.). This matches earlier SP7dm notes; direct
`rustc --test` remains the valid lightweight generator guard.

## Decision

Rejected by triage; no runtime candidate was retained.

Arm7/CONTROL0 has worse generated-arm inverse pressure than the already
rejected generated-arm candidates:

- arm0: `19,211,525` xgboost inverse-items, rejected after device-loss /
  severe queued work.
- arm10: `389,142` inverse-items, correctness-clean but severe BusyLoop wall
  regression.
- arm7/CONTROL0: `22,547,036` inverse-items.

Running a representative browser proof with this same per-row split-inverse
generated-arm design would likely repeat a known wall-negative pattern. The
next work should stay on larger non-repeated buckets: FRI/check NTT only if
there is a new design, or a chunk-complete GPU-witgen/accumulator design that
removes a broad CPU-owned surface without per-proof prewarm regression.

Accepted wall-time gain: 0.
