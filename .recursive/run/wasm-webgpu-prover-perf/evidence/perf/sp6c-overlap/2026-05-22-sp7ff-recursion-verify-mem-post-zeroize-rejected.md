# SP7ff - Recursion verify_mem post-zeroize candidate rejected

Date: 2026-05-22

## Candidate

Tested a narrower CPU-exec plus GPU `verify_mem` recursion witness variant:

- CPU still ran generated `step_exec`.
- CPU sorted WOM rows and injected WOM backrefs.
- Normal sparse zeroize made `recursion_data` GPU-current before `verify_mem`.
- Generated `step_verify_mem.wgsl` ran after zeroize using uploaded sorted WOM rows and per-cycle offsets.

This avoided the dense `recursion_data` upload that made SP7fd clearly bad, but it still uploaded the sorted WOM rows.

## E2E proof evidence

Command:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
__VK_LAYER_NV_optimus=NVIDIA_only \
__NV_PRIME_RENDER_OFFLOAD=1 \
__GLX_VENDOR_LIBRARY_NAME=nvidia \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
WASM_BINDGEN_TEST_TIMEOUT=420 \
cargo test --manifest-path examples/browser-prove/Cargo.toml \
  --target wasm32-unknown-unknown --release \
  recursion_verify_mem_post_zeroize_candidate_representative_e2e_verify \
  -- --nocapture
```

Log:

- `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7ff-representative-recursion-verify-mem-post-zeroize-candidate.chrome.txt`

Result:

- Test passed with verified BusyLoop and KeccakUnion receipts.
- CPU fallback and CPU-only counters stayed at zero.
- High WebGPU limits were negotiated.

## Performance comparison

Accepted SP7ez baseline:

- BusyLoop: `5665 ms`
- KeccakUnion: `85171 ms`
- BusyLoop plus KeccakUnion test runtime: `91.58s`

SP7ff candidate:

- BusyLoop: `6456 ms`
- KeccakUnion: `84512 ms`
- BusyLoop plus KeccakUnion test runtime: `91.75s`

Net:

- BusyLoop regressed by `+791 ms`.
- KeccakUnion improved by `-659 ms`.
- Combined representative runtime regressed by `+0.17s`.

Data movement:

- BusyLoop uploaded `19,888,720` bytes of `recursion_verify_mem_rows` and `581,880` bytes of offsets.
- KeccakUnion uploaded `753,684,300` bytes of `recursion_verify_mem_rows` and `21,850,504` bytes of offsets.
- No dense `source=recursion_data` host upload appeared, but KeccakUnion still copied `13,107,200` bytes of `recursion_data` on-device.

## Decision

Rejected and removed. The candidate is correctness-positive but wall-time flat/regressive on the representative gate and still pays the sorted-WOM upload cost. It should not proceed to xgboost.

Generated `verify_mem` remains semantically useful, but only after exec/WOM production are GPU-resident. Retrying CPU-exec plus uploaded sorted WOM rows repeats the same non-profitable shape.

## Cleanup

Removed:

- opt-in `set_recursion_verify_mem_gpu_enabled` / `recursion_verify_mem_gpu_dispatches` APIs
- `webgpu_step_verify_mem.wgsl`
- post-zeroize witness hook
- CPU preflight packing for uploaded `verify_mem` rows
- representative candidate browser test

Marker sweep after removal:

```bash
rg -n "verify_mem_post_zeroize|set_recursion_verify_mem_gpu|recursion_verify_mem_gpu|WomVerifyPreflight|generate_witness_exec_prepare_wom|webgpu_step_verify_mem|recursion_verify_mem" risc0/circuit/recursion/src/prove examples/browser-prove/src/lib.rs
```

Result: clean.

Post-removal e2e gates:

- Default BusyLoop plus KeccakUnion representative log: `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7ff-post-reject-default-representative.chrome.txt`
- xgboost log: `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7ff-post-reject-xgboost.chrome.txt`

Results:

- `rv32im_default_representative_e2e_verify`: passed, BusyLoop `5733 ms`, KeccakUnion `85393 ms`, total `91.88s`, zero fallback/CPU-only.
- `xgboost_succinct_receipt_verifies`: passed, proof wall `64114 ms`, total `64.69s`, zero fallback/CPU-only.
- Both logs have no `recursion_verify_mem` upload/stage/source markers.
