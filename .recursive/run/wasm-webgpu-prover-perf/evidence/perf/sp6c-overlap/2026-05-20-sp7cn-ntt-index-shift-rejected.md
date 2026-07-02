# SP7cn: NTT index shift/mask candidate rejected

Date: 2026-05-20
Status: rejected and reverted

## Candidate

Replace power-of-two division/multiplication in the hot WebGPU NTT WGSL
indexing path with shifts and masks:

- `row = idx / pairs_per_row` -> `row = idx >> (n_bits - 1)`
- `pair = idx - row * pairs_per_row` -> `pair = idx & (pairs_per_row - 1)`
- `g = pair / s_size` -> `g = pair >> (s_bits - 1)`
- `idx1 = row_base + g * 2 * s_size + s` -> `idx1 = row_base + (g << s_bits) + s`

The same addressing rewrite was applied to both `NTT_STEP_WGSL` and the fused
first inverse stage `BATCH_INTERPOLATE_NTT_FROM_WGSL`.

## Compile gate

Command:

```text
cargo test --manifest-path examples/browser-prove/Cargo.toml --target wasm32-unknown-unknown --release iter6d_g_replace_busy_loop_e2e_verify --no-run
```

Result: passed in `4m52s`.

## Representative e2e proof gate

Command:

```text
env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  __VK_LAYER_NV_optimus=NVIDIA_only \
  __NV_PRIME_RENDER_OFFLOAD=1 \
  __GLX_VENDOR_LIBRARY_NAME=nvidia \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  WASM_BINDGEN_TEST_TIMEOUT=420 \
  cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Log: `/tmp/sp7cn-busy-keccak-ntt-index-shift.log`

Result: passed with verified BusyLoop and KeccakUnion receipts, high WebGPU
limits, and zero fallback/CPU-only counters.

BusyLoop:

- `wall_ms=7667`
- `gpu_active_ms=3296`
- `raw_compute_dispatches=721`
- `queue_submits=173`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

KeccakUnion:

- `wall_ms=92697`
- `gpu_active_ms=61094`
- `raw_compute_dispatches=12716`
- `queue_submits=3075`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

Aggregated KeccakUnion FRI round-0:

- `fri_prove round=0 domain_in=1048576`: `25689 ms` across 30 calls
- `fri_prove round=0 merkle_new rows=65536 cols=64`: `25647 ms` across 30 calls

## Comparison against SP7cm

SP7cm final exact-code gate:

- BusyLoop `wall_ms=7276`
- KeccakUnion `wall_ms=92303`
- KeccakUnion FRI round-0 aggregate `25591 ms`

SP7cn candidate:

- BusyLoop `7276 -> 7667` (`+391 ms`, `+5.4%`)
- KeccakUnion `92303 -> 92697` (`+394 ms`, `+0.4%`)
- KeccakUnion FRI round-0 `25591 -> 25689` (`+98 ms`, flat/slightly worse)

## Decision

Rejected before xgboost. Correctness was clean, but the short representative
gate had no FRI/NTT improvement and regressed BusyLoop enough to fail the wall
gate. The candidate was reverted manually; marker search for the introduced
`row_shift` / `s_shift` variables is clean, and `git diff --check` passed after
revert.

Do not retry this same shift/mask rewrite as an e2e optimization. A future NTT
attempt needs a different mechanism, ideally one that reduces memory traffic,
stage count, or dispatch work rather than only replacing power-of-two index
arithmetic.
