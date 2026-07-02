# SP7fu-SP7fw Recursion Data Write Elision Rejected

Date: 2026-05-22

## Context

SP7fr is the accepted browser WebGPU recursion witness/verify_mem path. It still pays a large sparse zeroize upload before the post-zeroize GPU witness rewrite:

- BusyLoop accepted shape: `webgpu_zeroize_sparse_values ~= 98.0 MB`
- `recursion_data` sparse upload: about `6.0M` values / `30.6 MB` value payload
- Correctness gate: full browser e2e proof generation and receipt verification

This experiment tested whether CPU-shadow cells written by the generated CPU recursion exec plan could be marked invalid before sparse zeroize, on the assumption that later GPU witness kernels would rewrite them.

## Evidence

- Diagnostic profile: `2026-05-22-sp7fu-busyloop-recursion-data-column-profile.chrome.txt`
- RED upload cap: `2026-05-22-sp7fv-red-busyloop-exact-data-write-upload-cap.chrome.txt`
- All-write invalidation candidate: `2026-05-22-sp7fv-green-busyloop-exact-data-write-invalidation.chrome.txt`
- Previous-row-safe invalidation candidate: `2026-05-22-sp7fw-green-busyloop-prev-read-safe-data-invalidation.chrome.txt`
- Post-reject restored proof: `2026-05-22-sp7fw-post-reject-busyloop-restored.chrome.txt`

## Results

SP7fu was diagnostic only. The focused BusyLoop proof passed with:

- `recursion_data values=6004090`
- `recursion_data sparse_bytes=30663952`
- `webgpu_zeroize_sparse_values=98021828`
- `raw_compute_dispatches=613`
- `queue_submits=168`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

The profile showed all sparse `recursion_data` values in columns `5..127`; Poseidon-heavy columns dominated the upload, but that was not sufficient proof that the cells were safely recomputable after zeroize.

SP7fv invalidated every recorded CPU write to `recursion_data`. It reduced upload materially, but broke proof correctness:

- `writes=11616636 invalidated=11616636`
- `recursion_data values=0`
- `webgpu_zeroize_sparse_values=74005328`
- `raw_compute_dispatches=612`
- `queue_submits=167`
- Result: `verify lift` failed with `verification indicates proof is invalid`

SP7fw narrowed invalidation to columns that were not identified as previous-row reads in generated WGSL. It still broke proof correctness:

- `writes=4125260 invalidated=4125260`
- `recursion_data values=3963198`
- `recursion_data sparse_bytes=21061656`
- `webgpu_zeroize_sparse_values=89858196`
- `raw_compute_dispatches=613`
- `queue_submits=168`
- Result: `verify lift` failed with `verification indicates proof is invalid`

After removing the candidate code and temporary upload assertion, the focused BusyLoop browser proof passed again:

- `test result: ok. 1 passed; 0 failed; 168 filtered out; finished in 6.06s`
- `recursion_data values=6003399`
- `recursion_data sparse_bytes=30655564`
- `webgpu_zeroize_sparse_values=98018944`
- `wall_ms=5891`
- `gpu_active_ms=4406`
- `raw_compute_dispatches=613`
- `queue_submits=168`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

## Decision

Reject CPU-shadow upload elision based on CPU write tracking or generated store-column masks.

The generated recursion GPU sequence does not yet provide a complete dependency/read-set proof for those cells. Even columns that were not found in the simple previous-row read set can still be semantically required through current-row chains, branch interactions, or incomplete post-zeroize recomputation.

Do not retry this avenue without a stronger generated dependency mask or a GPU-produced authoritative-validity mask that is itself covered by e2e proof generation.

Accepted wall-time gain: `0`.

Current accepted working state remains SP7fr.
