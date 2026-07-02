Run: `wasm-webgpu-prover-perf`
Phase: `SP7cm - WebGPU NTT twiddle cache`
Date: `2026-05-20`
Status: `ACCEPTED`

## Change

`risc0/zkp/src/hal/webgpu.rs` now caches per-`n_bits` forward and inverse NTT twiddle tables in `WebGpuHal`.

The WebGPU NTT shaders previously computed `root^s` independently in every butterfly lane via a WGSL exponentiation loop. The retained shader path fetches the precomputed twiddle from a cached storage buffer:

- `NTT_STEP_WGSL`: in-place forward/inverse NTT steps use `twiddles[stage_base + s]`.
- `BATCH_INTERPOLATE_NTT_FROM_WGSL`: the fused first inverse NTT stage uses the same twiddle table.
- The old 28-element cached `ROU_FWD` / `ROU_REV` buffers were removed because the shaders no longer consume them.

This is intentionally a lower-risk NTT change than SP7ck's rejected grouped-loop shape: dispatch topology, workgroup size, and queue-submit counts are unchanged.

## Compile / Hygiene

Compile gate:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
Finished release target(s) in 4m57s
```

Hygiene after final shader naming cleanup:

```text
cargo fmt --check --manifest-path risc0/zkp/Cargo.toml
git diff --check
```

Both passed.

## Representative E2E Proof Gates

All browser runs used the standard NVIDIA/WebGPU environment, `WASM_BINDGEN_TEST_TIMEOUT=420`, and real receipt verification.

### BusyLoop + KeccakUnion

Final exact-code log: `/tmp/sp7cm-final-busy-keccak-ntt-twiddles.log`

```text
test result: ok. 1 passed; 0 failed
BusyLoop:    wall_ms=7276 gpu_active_ms=3245 raw_compute_dispatches=721 queue_submits=173 cpu_fallbacks=0 cpu_only_ops=0
KeccakUnion: wall_ms=92303 gpu_active_ms=60686 raw_compute_dispatches=12716 queue_submits=3075 cpu_fallbacks=0 cpu_only_ops=0
```

Comparison against SP7cl:

```text
BusyLoop:    7485 -> 7276  (-209 ms, -2.8%)
KeccakUnion: 92641 -> 92303 (-338 ms, -0.4%)
```

An earlier same-candidate pre-rename run was stronger (`BusyLoop=7335`, `KeccakUnion=91603`), but acceptance uses the final exact-code run above.

### xgboost

Final exact-code log: `/tmp/sp7cm-final-xgboost-ntt-twiddles.log`

```text
test result: ok. 1 passed; 0 failed
xgboost: wall_ms=77800 gpu_active_ms=45060 gpu_idle_ratio=0.421
raw_compute_dispatches=11466 queue_submits=2718
upload_bytes=3114378064 readback_bytes=7488592
cpu_fallbacks=0 cpu_only_ops=0
```

Confirmation pre-rename logs:

```text
/tmp/sp7cm-xgboost-ntt-twiddles.log    wall_ms=77688 gpu_active_ms=44967
/tmp/sp7cm-xgboost-ntt-twiddles-r2.log wall_ms=78058 gpu_active_ms=45301
```

Comparison against SP7cl:

```text
xgboost final exact-code: 78509 -> 77800 (-709 ms, -0.9%)
xgboost candidate mean:   78509 -> 77849 (-660 ms, -0.8%)  # final + two confirmation trials
gpu_active final:         45925 -> 45060 (-865 ms, -1.9%)
```

The change adds cached twiddle uploads:

```text
webgpu_ntt_twiddles_fwd uploads=3 upload_bytes=4472820
webgpu_ntt_twiddles_rev uploads=2 upload_bytes=5242872
```

Even with about 9.7 MB of new one-time twiddle uploads, wall time remains directionally positive across all three representative workloads.

## Target Bucket Movement

Top xgboost aggregates after SP7cm final:

```text
77801 prove_session_async
38071 composite_to_succinct_async
29121 finalize_async fri_prove
27383 fri_prove round=0 domain_in=1048576
27332 fri_prove round=0 merkle_new rows=65536 cols=64
19967 join_async
19817 join_prove_async
11490 rv32im_witgen_accum
11131 rv32im_accumulate step_top_accum_cpu_skip_replaced_misc0=true_major_mask=0x0006
 9571 finalize_async check_group
```

The hot round-0 FRI bucket did **not** collapse; SP7cm is a small active-time reduction, not the deeper NTT/expand breakthrough. The largest remaining measured bucket is still the queued `batch_expand_into_evaluate_ntt` / FRI work attributed at the first downstream drain.

## Verdict

Accepted as a small correctness-preserving NTT improvement:

- Representative proof generation passed for BusyLoop, KeccakUnion, and xgboost.
- `cpu_fallbacks=0`, `cpu_only_ops=0`, no verifier failure, no device-loss signal.
- Dispatch and submit counts are unchanged, so the retained mechanism is shader arithmetic reduction via cached twiddle fetches.
- Current accepted xgboost working-state estimate moves from about `78.5 s` to about `77.8 s` on this browser/NVIDIA setup.

Next work should not overcount this as the major NTT solution. Continue with another measured large bucket: deeper NTT/expand design, `rv32im_accumulate` CPU work, or `finalize_async check_group` attribution.
