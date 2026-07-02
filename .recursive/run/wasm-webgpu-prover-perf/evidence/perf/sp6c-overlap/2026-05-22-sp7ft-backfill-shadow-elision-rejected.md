# SP7ft: recursion backfill CPU-shadow elision rejected

Date: 2026-05-22

## Decision

Rejected and reverted. Clearing only the CPU shadow for recursion WOM backfill columns
`data[0..4]` did not reduce the sparse zeroize upload or proving wall time. The working
state remains SP7fr, with SP7fs already confirming that broad `recursion_data`
stale-shadow elision is incorrect.

## RED

Temporary focused BusyLoop e2e cap:

- Test: `recursion_witgen_gpu_verify_mem_candidate_busy_loop_e2e_verify`
- Evidence: `2026-05-22-sp7ft-red-busyloop-backfill-shadow-upload-cap.chrome.txt`
- Result: proof generation reached diagnostics and then failed the upload cap.
- `webgpu_zeroize_sparse_values upload_bytes=98031748`
- `recursion_data values=6006595 ranges=831626 sparse_bytes=30679404`
- `raw_compute_dispatches=613`
- `queue_submits=168`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

## Candidate

Tried clearing CPU-shadow cells for `data[0..4]`, cycles `0..work_cycles-1`,
after CPU exec-plan derivation and before post-zeroize GPU backfill/verify.
Those are the columns the existing GPU backfill kernel rewrites.

Focused e2e evidence:

- Evidence: `2026-05-22-sp7ft-green-busyloop-backfill-shadow-elided.chrome.txt`
- Result: receipt path completed, but the RED cap still failed.
- `webgpu_zeroize_sparse_values upload_bytes=98039004`
- `recursion_data values=6008401 ranges=831504 sparse_bytes=30685652`
- `raw_compute_dispatches=613`
- `queue_submits=168`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

The candidate did not reduce upload bytes; it moved within noise and slightly worse than
the RED run. Inspection of the sparse upload planner confirms it already skips `0` and
`BabyBearElem::INVALID`; therefore the attempted columns are not meaningful upload
contributors in the current CPU shadow.

## Post-Reject Restore

Removed the helper and the temporary upload-cap assertion.

Focused BusyLoop e2e proof passed:

- Evidence: `2026-05-22-sp7ft-post-reject-busyloop-restored.chrome.txt`
- `test result: ok. 1 passed; 0 failed; 168 filtered out; finished in 5.97s`
- `prove_segment_async elapsed_ms=3824`
- `recursion_witgen elapsed_ms=160`
- `recursion_data values=6003622 ranges=831418 sparse_bytes=30665848`
- `webgpu_zeroize_sparse_values upload_bytes=98019916`
- `raw_compute_dispatches=613`
- `queue_submits=168`
- `cpu_fallbacks=0`
- `cpu_only_ops=0`

Static checks:

- `cargo fmt --manifest-path examples/browser-prove/Cargo.toml`
- `git diff --check`

## Follow-Up

Do not spend more time on partial CPU-shadow upload trimming unless a fresh profile
identifies a specific upload contributor with measurable wall impact. The viable
larger lever is still expanded GPU-resident recursion witness coverage; otherwise the
next highest-value target is a major FRI/check NTT scheduling or active-time reduction.

