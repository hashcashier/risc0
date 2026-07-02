# SP7fh - Recursion Poseidon2 exec WOM extern probe

Date: 2026-05-22

## Purpose

SP7fg proved that the chunk-complete recursion `exec` Poseidon2 chain is a
browser-capacity-safe WGSL module. SP7fh adds the next correctness substrate:
real GPU-buffer implementations of the generated exec externs needed by that
chunk:

- `extern_womRead`
- `extern_womWrite`
- `extern_plonkWrite`

This is still a probe/test artifact, not production runtime wiring.

## Changed files

- `risc0/circuit/recursion/src/prove/hal/webgpu.rs`
- `risc0/circuit/recursion/src/prove/mod.rs`
- `examples/browser-prove/src/lib.rs`

The probe module binds a preflight WOM buffer plus per-cycle WOM-write,
Plonk-row, and cursor buffers. It decodes Montgomery addresses for
`extern_womRead`, and stores Plonk/WOM rows as Montgomery-packed BabyBear words,
matching the SP7fd finding that generated WGSL expects `Fp::new(addr)` rather
than decoded integer addresses.

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
  recursion_exec_poseidon2_chain_wom_externs_match_layout -- --nocapture
```

Log:

- `2026-05-22-sp7fh-recursion-exec-poseidon2-wom-extern-probe.chrome.txt`

Result:

- Passed in Chrome with high WebGPU limits.
- `test tests::recursion_exec_poseidon2_chain_wom_externs_match_layout ... ok`
- Test runtime: `6.13s`

The test covers:

- `poseidon2_load` reading eight ExtElem values from a preflight WOM GPU buffer
  and emitting nine Plonk rows, including the terminal zero row.
- `poseidon2_store` emitting eight WOM writes and nine matching Plonk rows from
  previous-row Poseidon2 state.
- Exact Montgomery-packed row layout for addresses and field limbs.

## E2E proof gates

Representative BusyLoop + KeccakUnion:

- Log: `2026-05-22-sp7fh-post-probe-default-representative.chrome.txt`
- Passed with verified receipts.
- BusyLoop `wall_ms=5742`, `gpu_active_ms=4196`, `gpu_idle_ratio=0.269`.
- KeccakUnion `wall_ms=85380`, `gpu_active_ms=62938`, `gpu_idle_ratio=0.263`.
- Total test runtime: `91.87s`.
- CPU fallback / CPU-only: `0 / 0`.
- No `recursion_verify_mem` markers.

xgboost:

- Log: `2026-05-22-sp7fh-post-probe-xgboost.chrome.txt`
- Passed with verified receipt and journal.
- xgboost `wall_ms=63951`, `gpu_active_ms=47571`, `gpu_idle_ratio=0.256`.
- Total test runtime: `64.52s`.
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

Accepted as a correctness/layout probe only. Accepted wall-time gain: `0`.

The important retained fact is that the generated Poseidon2-chain exec chunk can
use real WebGPU buffers for preflight WOM reads and Plonk/WOM row emission
without the writable-storage aliasing or Montgomery-packing bugs found in SP7fd.

This does not yet solve the production performance blocker. A profitable
runtime path still needs GPU-resident sorted WOM rows and generated
`verify_mem`; otherwise the path falls back into the upload-heavy shape rejected
in SP7fd/SP7ff.
