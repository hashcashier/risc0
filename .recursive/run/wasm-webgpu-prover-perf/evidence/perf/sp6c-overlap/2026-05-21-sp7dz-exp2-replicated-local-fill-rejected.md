# SP7dz - expand_bits=2 replicated local-NTT fill rejected

Date: 2026-05-21

Status: rejected and reverted

## Candidate

The accepted `BATCH_EXPAND_LOCAL_NTT_WGSL` path fills a 1024-element local
block by loading `input[col >> expand_bits]` for every expanded output cell.
For the proof-shaped `INV_RATE=4` case (`expand_bits=2`), that means each
coefficient is read four times before the local NTT stages run.

This candidate specialized the full-block `expand_bits=2` fill:

- each local lane loaded one unique input coefficient;
- the lane replicated it into four adjacent scratch cells; and
- the existing local NTT stages and remaining global NTT stages were unchanged.

The intent was lower input-read traffic in the fused expand/local-NTT dispatch,
without changing dispatch count, queue submit count, or proof semantics.

## TDD Evidence

RED:

```text
cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_batch_expand_ntt_uses_exp2_replicated_local_fill -- --nocapture
```

Result: failed as intended after CPU/GPU parity succeeded. High WebGPU limits
were negotiated, and the only assertion failure was the missing marker upload:

```text
webgpu_batch_expand_local_ntt_params uploads=1
webgpu_ntt_step_params uploads=1
webgpu_ntt_twiddles_fwd uploads=1
missing webgpu_batch_expand_local_ntt_exp2_replicate_params
cpu_fallbacks=0 cpu_only_ops=0
```

GREEN:

```text
cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release webgpu_hal_batch_expand_ntt_uses_exp2_replicated_local_fill -- --nocapture
```

Result: passed under high WebGPU limits:

```text
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152
test tests::webgpu_hal_batch_expand_ntt_uses_exp2_replicated_local_fill ... ok
```

## Representative Proof Gate

Command:

```text
cargo test --manifest-path Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture
```

Result: passed with verified BusyLoop and KeccakUnion receipts, high WebGPU
limits, and zero fallback/CPU-only counters.

Key result:

```text
test tests::rv32im_default_representative_e2e_verify ... ok
finished in 99.34s
```

Comparison:

```text
Latest accepted SP7dy BusyLoop+KeccakUnion: 98.15s
SP7dz candidate BusyLoop+KeccakUnion:       99.34s
Movement:                                  +1.19s slower
```

## Decision

Reject and revert before xgboost.

The focused HAL proof showed the specialized fill was correct and used, but
the representative BusyLoop+KeccakUnion wall gate moved the wrong direction by
about 1.2s versus the latest accepted SP7dy state. This is not a significant
immediate improvement, and it fails the current prioritization rule.

Accepted wall-time gain: `0`.

Do not retry this exact replicated-fill micro-optimization as an immediate
lever. The remaining NTT work needs a larger active-time reduction than cutting
the expand-fill input reads inside the already fused local block.

## Revert / Hygiene

Reverted:

- focused marker test in `examples/browser-prove/src/lib.rs`;
- `BATCH_EXPAND_LOCAL_NTT_WGSL` replicated-fill branch; and
- `webgpu_batch_expand_local_ntt_exp2_replicate_params` marker routing.

Post-revert checks:

```text
rg -n "exp2_replicate|replicated_local_fill|webgpu_batch_expand_local_ntt_exp2" examples/browser-prove/src/lib.rs risc0/zkp/src/hal/webgpu.rs
```

No matches.

```text
cargo fmt --check --manifest-path examples/browser-prove/Cargo.toml
cargo fmt --check --manifest-path risc0/zkp/Cargo.toml
git diff --check
```

All passed.
