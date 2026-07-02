# SP7ap: bounded RV32IM witgen GPU replacement correctness

Date: 2026-05-19

## Purpose

Move from scratch/diff-only witgen GPU work to an authoritative replacement
slice, while keeping the user-requested proof-generation gate:

- BusyLoop po2=18
- `KeccakUnion(1)`
- xgboost

The target slice is intentionally bounded to the MISC0/ADD arm. The acceptance
bar is stricter than cell diff: the browser must generate and verify real
succinct receipts with zero CPU fallback/CPU-only ops.

## RED

The first replacement attempt still failed after GPU-written cells matched CPU
cells. The direct write diff was clean, but the final trace diverged in control
rows because short-circuiting `step_Top` skipped mutable lookup-table side
effects.

Earlier diagnostic signal:

```text
REPLACE_FINAL_SUMMARY total_cells=55312384 mismatches=4703 rows=262144 cols=211
```

Root cause: `LookupTables::lookup_delta` mutates CPU lookup state used by later
rows. A skipped cycle must still replay those side effects, otherwise later
`Control0` count columns are wrong even if the GPU-generated data cells are
bit-exact.

## GREEN: side-effect replay and final diff

Added bounded lookup side-effect replay for the MISC0/ADD short-circuit. The
replacement path now replays the lookup deltas required by the skipped CPU
`step_Top` cycles before counting them as GPU-replaced.

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_diff_busy_loop -- --nocapture
```

Result: the diagnostic intentionally bailed after comparing the final matrix,
but the correctness signal is green:

```text
max_buffer_size=4294967292
max_storage_buffer_binding_size=2147483644
max_compute_workgroup_storage_size=49152
iter6d_g_pre_witgen_dispatch_async mask=0x0001 dispatched_arms=[0]
REPLACE_FINAL_SUMMARY total_cells=55312384 mismatches=0 rows=262144 cols=211
```

This reduced the replacement final diff from 4703 mismatches to 0 on the
BusyLoop po2=18 trace.

## GREEN: BusyLoop + KeccakUnion e2e

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_busy_loop_e2e_verify -- --nocapture
```

Result: passed in 114.15 s with high WebGPU limits, real succinct receipt
verification, and replacement short-circuit assertions for both workloads.

| Workload | wall_ms | gpu_active_ms | idle ratio | segments | raw dispatches | queue submits | data readbacks |
|---|---:|---:|---:|---:|---:|---:|---:|
| BusyLoop replace | 9728 | 4400 | 0.548 | 1 | 709 | 164 | 1 / 221249536 bytes |
| KeccakUnion(1) replace | 104138 | 69897 | 0.329 | 4 | 12617 | 2988 | 4 / 774373376 bytes |

KeccakUnion retained the representative shape:

- `segments=4`
- `pending_keccaks=9`
- `assumptions=1`

Both workloads reported:

- `cpu_fallbacks=0`
- `cpu_only_ops=0`
- `iter6d_g_pre_witgen_dispatch_async mask=0x0001 dispatched_arms=[0]`

## GREEN: xgboost e2e

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  iter6d_g_replace_xgboost -- --nocapture
```

Result: passed in 104.56 s with high WebGPU limits, succinct receipt
verification, journal `30.528042544062632`, and replacement short-circuit
assertions.

```text
wall_ms=104345
gpu_active_ms=57645
gpu_idle_ratio=0.448
segments=11
gpu_dispatches=5249
raw_compute_dispatches=11320
queue_submits=2605
cpu_fallbacks=0
cpu_only_ops=0
data uploads=476 upload_bytes=7686062080
data readbacks=11 readback_bytes=2433744896
```

## Decision

Correctness accepted for this bounded MISC0/ADD replacement slice. It is
receipt-verified across BusyLoop, KeccakUnion, and xgboost with zero CPU
fallback/CPU-only ops.

Performance rejected for production enablement. The current replacement path
adds full `data` buffer roundtrips:

- BusyLoop: 221249536 bytes read back from `data`.
- KeccakUnion: 774373376 bytes read back from `data`.
- xgboost: 2433744896 bytes read back from `data` and 7686062080 bytes
  uploaded to `data`.

xgboost replace wall was `104345` ms versus the current default SP7an repeated
mean of `99692` ms and SP7am default of `100056` ms. This is about 4.3-4.7 s
slower, so accepted wall-time gain is 0.

## Next Target

Do not broaden to more witgen arms yet. More arms would amplify the same sync
tax. The next performance slice is to remove full `data` CPU/GPU roundtrips:

1. Add a representative RED assertion that witgen replacement cannot introduce
   full `data` readbacks/upload bytes.
2. Keep GPU authoritative for the replaced data rows while repairing only the
   CPU-side shadow/state that later CPU rows actually require.
3. Replace the full CPU-to-GPU invalid seed upload with a GPU-side invalid fill
   or narrow initialization path.
4. Re-run BusyLoop + KeccakUnion and xgboost receipt gates; accept only if the
   xgboost wall beats the SP7an default baseline.
