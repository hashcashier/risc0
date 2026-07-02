# SP7fj recursion WOM row coverage guard

Date: 2026-05-22

## Summary

Accepted a browser-test guard for generated recursion WOM row coverage before attempting production GPU-resident `verify_mem` again.

The generated recursion `step_exec` writes exactly 47 WOM/Plonk rows, and generated `step_verify_mem` reads exactly the same 47 rows by family. SP7fi's Poseidon2-chain scatter/backfill probe covers 18 of those rows. The remaining production blocker is 29 non-Poseidon2 rows.

No production proving path was changed. Accepted wall-time gain: 0.

## Row coverage

| Family | `step_exec` writes | `step_verify_mem` reads |
|---|---:|---:|
| `micro_ops(recursion::MicroOps)` | 9 | 9 |
| `macro_ops/.../bit_and_elem(recursion::BitAndElem)` | 3 | 3 |
| `macro_ops/.../bit_op_shorts(recursion::BitOpShorts)` | 3 | 3 |
| `macro_ops/.../sha_init(recursion::ShaWrap)/sha_cycle(recursion::ShaCycle)` | 2 | 2 |
| `macro_ops/.../sha_fini(recursion::ShaWrap)/sha_cycle(recursion::ShaCycle)` | 2 | 2 |
| `macro_ops/.../sha_load(recursion::ShaWrap)/sha_cycle(recursion::ShaCycle)` | 2 | 2 |
| `macro_ops/.../sha_mix(recursion::ShaWrap)/sha_cycle(recursion::ShaCycle)` | 2 | 2 |
| `macro_ops/.../set_global(recursion::SetGlobal)` | 4 | 4 |
| `poseidon2_load(recursion::Poseidon2Load)` | 9 | 9 |
| `poseidon2_store(recursion::Poseidon2Store)` | 9 | 9 |
| `checked_bytes(recursion::CheckedBytes)` | 2 | 2 |

Totals:

- Total generated WOM rows: 47.
- Poseidon2 rows already covered by SP7fi probe: 18.
- Remaining non-Poseidon2 rows: 29.

## Changed files

- `risc0/circuit/recursion/src/prove/hal/webgpu.rs`
  - Added `recursion_wom_generated_row_coverage_for_test()`.
  - The helper parses `rust_kernels_generated.rs.inc`, counts `ctx.plonk_write_wom` and `ctx.plonk_read_wom` by generated component family, and asserts the write/read family sets match.
- `risc0/circuit/recursion/src/prove/mod.rs`
  - Exposed the browser-test helper.
- `examples/browser-prove/src/lib.rs`
  - Added `recursion_wom_generated_row_coverage_is_stable`.
  - The test pins the exact row-family table and the 47/18/29 totals.
- `risc0/circuit/recursion/src/prove/hal/rust_kernels.rs`
  - Retains the SP7fi `prepare_wom_for_verify` split used by the scatter/backfill probe.

## Validation

Static checks:

```text
cargo fmt --manifest-path examples/browser-prove/Cargo.toml --check
git diff --check
```

Focused browser guard:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release recursion_wom_generated_row_coverage_is_stable -- --nocapture
```

Evidence: `2026-05-22-sp7fj-recursion-wom-row-coverage.chrome.txt`

Result:

```text
test tests::recursion_wom_generated_row_coverage_is_stable ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 161 filtered out; finished in 0.01s
```

Representative e2e proof gate:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release rv32im_default_representative_e2e_verify -- --nocapture
```

Evidence: `2026-05-22-sp7fj-default-representative.chrome.txt`

Result:

- High WebGPU limits: `4294967292 / 2147483644 / 49152`.
- BusyLoop: `wall_ms=5704`, `gpu_active_ms=4134`, `gpu_idle_ratio=0.275`.
- KeccakUnion: `wall_ms=85070`, `gpu_active_ms=62920`, `gpu_idle_ratio=0.260`.
- Test runtime: `91.50s`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.

xgboost e2e proof gate:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release xgboost_succinct_receipt_verifies -- --nocapture
```

Evidence: `2026-05-22-sp7fj-xgboost.chrome.txt`

Result:

- High WebGPU limits: `4294967292 / 2147483644 / 49152`.
- xgboost: `wall_ms=64048`, `gpu_active_ms=47686`, `gpu_idle_ratio=0.255`.
- Test runtime: `64.63s`.
- `cpu_fallbacks=0`, `cpu_only_ops=0`.

Rejected-marker sweep:

```text
rg -n "recursion_verify_mem|verify_mem_post_zeroize|set_recursion_verify_mem_gpu|WomVerifyPreflight|generate_witness_exec_prepare_wom|webgpu_step_verify_mem" \
  2026-05-22-sp7fj-default-representative.chrome.txt \
  2026-05-22-sp7fj-xgboost.chrome.txt \
  risc0/circuit/recursion/src/prove \
  examples/browser-prove/src/lib.rs
```

Result: no output.

## Interpretation

SP7fj does not reduce wall time directly. It prevents the next GPU `verify_mem` attempt from repeating SP7fd/SP7ff's bad decision: running generated verify work with incomplete or upload-heavy row production.

Next production-relevant step: generate the 29 remaining non-Poseidon2 WOM rows on GPU-resident buffers, starting with the 2-row `checked_bytes` family, then macro-op families, then micro-ops. Only after all 47 rows can stay GPU-resident should generated `verify_mem` return as a production candidate.
