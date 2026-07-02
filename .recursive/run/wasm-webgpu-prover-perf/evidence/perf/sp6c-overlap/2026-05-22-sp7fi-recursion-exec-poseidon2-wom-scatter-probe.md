# SP7fi - Recursion Poseidon2 exec WOM scatter probe

Date: 2026-05-22

## Purpose

SP7fh proved that the generated Poseidon2-chain `exec` chunk can use real
GPU-buffer `extern_womRead`, `extern_womWrite`, and `extern_plonkWrite`
implementations. SP7fi adds the next missing substrate: keep the generated
Plonk rows on GPU and scatter them into sorted address buckets without uploading
the sorted row stream from CPU.

This is still a probe/test artifact, not production runtime wiring.

## Changed files

- `risc0/circuit/recursion/src/prove/hal/webgpu.rs`
- `risc0/circuit/recursion/src/prove/mod.rs`
- `examples/browser-prove/src/lib.rs`

The new test-only module
`recursion_exec_poseidon2_chain_wom_scatter_probe_wgsl_module_for_test`
assembles the generated/pruned Poseidon2-chain exec chunk with:

- a preflight WOM storage buffer for `extern_womRead`,
- an unsorted GPU Plonk-row buffer written by generated `extern_plonkWrite`,
- per-cycle row cursors,
- a second GPU scatter entry point,
- uploaded bucket-base metadata,
- atomic per-bucket counters,
- a sorted GPU Plonk-row output buffer.
- uploaded per-cycle prefix metadata,
- a third GPU backfill entry point that mirrors CPU `inject_wom_backs` by
  writing the previous sorted row into data columns 0..4 for the verify phase.

The focused test deliberately emits shuffled load rows plus store rows from a
separate dependency-safe cycle, then verifies the sorted GPU output exactly:
terminal zero rows first, then decoded address order `0..7`, then `100..107`.
It also verifies that the GPU backfill writes the expected previous sorted row
into the data columns consumed by generated `verify_mem`.

## Focused browser probe

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
__VK_LAYER_NV_optimus=NVIDIA_only \
__NV_PRIME_RENDER_OFFLOAD=1 \
__GLX_VENDOR_LIBRARY_NAME=nvidia \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=120 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  recursion_exec_poseidon2_chain_wom_scatter_sorts_rows_on_gpu -- --nocapture
```

Log:

- `2026-05-22-sp7fi-recursion-exec-poseidon2-wom-scatter-probe.chrome.txt`

Result:

- Passed in Chrome with high WebGPU limits.
- `test tests::recursion_exec_poseidon2_chain_wom_scatter_sorts_rows_on_gpu ... ok`
- Test runtime after adding backfill: `6.33s`.

Correction made during probe:

- The first attempt placed a `poseidon2_load` at cycle 0 and `poseidon2_store`
  at cycle 1 in the same parallel dispatch. That let cycle 0 overwrite the
  previous-row data cycle 1 read, producing zero store limbs.
- The retained test uses three cycles, with store at cycle 2 and its input
  seeded at cycle 1. This preserves the intended dependency while still testing
  load and store rows in one GPU-resident scatter pass.
- A follow-up extension added the verify-prep backfill pass. The same focused
  test now checks that per-cycle prefixes `[0, 9, 9]` backfill data rows 0 and 1
  from sorted row index 8, matching the CPU prefix behavior for an empty middle
  cycle.

## E2E proof gates

Representative BusyLoop + KeccakUnion:

- Initial log: `2026-05-22-sp7fi-post-probe-default-representative.chrome.txt`
  failed before proof generation because Chrome negotiated low WebGPU limits
  (`1073741824 / 1073741824 / 32768`).
- Earlier accepted rerun before backfill:
  `2026-05-22-sp7fi-rerun-default-representative.chrome.txt`
- Final accepted log after backfill:
  `2026-05-22-sp7fi-final-default-representative.chrome.txt`
- Passed with verified receipts under high WebGPU limits.
- BusyLoop `wall_ms=5907`, `gpu_active_ms=4307`, `gpu_idle_ratio=0.271`.
- KeccakUnion `wall_ms=85428`, `gpu_active_ms=63095`, `gpu_idle_ratio=0.261`.
- Total test runtime: `92.06s`.
- CPU fallback / CPU-only: `0 / 0`.
- No `recursion_verify_mem` markers.

xgboost:

- Initial log: `2026-05-22-sp7fi-post-probe-xgboost.chrome.txt` failed before
  proof generation because Chrome negotiated the same low WebGPU limits.
- Earlier accepted rerun before backfill:
  `2026-05-22-sp7fi-rerun-xgboost.chrome.txt`
- Final accepted log after backfill: `2026-05-22-sp7fi-final-xgboost.chrome.txt`
- Passed with verified receipt and journal under high WebGPU limits.
- xgboost `wall_ms=64221`, `gpu_active_ms=47709`, `gpu_idle_ratio=0.257`.
- Total test runtime: `64.79s`.
- CPU fallback / CPU-only: `0 / 0`.
- No `recursion_verify_mem` markers.

## Static checks

Passed:

```bash
cargo fmt --manifest-path examples/browser-prove/Cargo.toml --check
git diff --check
rg -n "verify_mem_post_zeroize|set_recursion_verify_mem_gpu|recursion_verify_mem_gpu|WomVerifyPreflight|generate_witness_exec_prepare_wom|webgpu_step_verify_mem|recursion_verify_mem" risc0/circuit/recursion/src/prove examples/browser-prove/src/lib.rs
```

The marker sweep exits with no matches.

## Decision

Accepted as a correctness/layout/profitability probe only. Accepted wall-time
gain: `0`.

The important retained fact is that a GPU-resident sorted-row preparation shape
is viable for the generated Poseidon2-chain chunk: generated exec can write
unsorted Plonk rows on GPU, and a follow-up GPU pass can scatter those rows into
sorted address buckets using only compact bucket metadata plus atomic counters.
The probe now also proves that the previous-row backfill required by
`verify_mem` can be driven from the sorted row stream and compact per-cycle
prefix metadata on GPU.

This avoids the SP7fd/SP7ff rejected shape where CPU uploads the full sorted WOM
row stream. Production runtime wiring is still blocked on scaling this from the
Poseidon2-chain probe to all rows needed by generated `verify_mem`.
