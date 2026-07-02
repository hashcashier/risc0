# SP7fx Recursion Witgen Metadata Cache Rejected

## Summary

Rejected and removed exact-content caching for recursion GPU-witgen metadata buffers.

The hypothesis was that repeated recursion-witgen shapes were also repeated metadata contents, so a per-HAL cache for immutable u32 storage buffers could reduce repeated `queue.writeBuffer` traffic for:

- `recursion_witgen_candidate_preflight_wom`
- `recursion_witgen_candidate_iop_body`
- `recursion_witgen_candidate_iop_cursors`
- `recursion_witgen_candidate_bucket_bases`
- `recursion_witgen_candidate_cycle_prefixes`

E2e proof generation disproved that as a useful immediate lever.

## Evidence

RED evidence:

- Log: `2026-05-22-sp7fx-red-recursion-witgen-metadata-cache-representative.chrome.txt`
- The representative BusyLoop + KeccakUnion proof generated receipts and then failed the new assertion as expected:
  - `representative recursion GPU witgen should cache repeated preflight WOM metadata; uploads=25`

First GREEN attempt:

- Log: `2026-05-22-sp7fx-green-recursion-witgen-metadata-cache-representative.chrome.txt`
- Cached `iop_cursors` as if immutable.
- Rejected for correctness: KeccakUnion failed proof verification with a zero-root equality failure.
- Root cause: WGSL mutates `iop_cursor_buf[cycle]` during `extern_readIOPBody`, so the cursor buffer is not cacheable.

Second GREEN attempt:

- Log: `2026-05-22-sp7fx-green2-recursion-witgen-metadata-cache-representative.chrome.txt`
- Restored per-invocation `iop_cursors`.
- Proof generation reached diagnostics for BusyLoop and KeccakUnion, with zero fallback/CPU-only, but failed the upload-count assertion:
  - BusyLoop: `wall_ms=5863`, `gpu_active_ms=4390`, `raw_compute_dispatches=613`, `queue_submits=168`
  - KeccakUnion: `wall_ms=82017`, `gpu_active_ms=62584`, `raw_compute_dispatches=10900`, `queue_submits=3153`
  - KeccakUnion `recursion_witgen_candidate_preflight_wom uploads=25 upload_bytes=191465168`
  - KeccakUnion `recursion_witgen_candidate_bucket_bases uploads=25 upload_bytes=47866392`
  - KeccakUnion `recursion_witgen_candidate_iop_body uploads=25 upload_bytes=33352704`
  - KeccakUnion `recursion_witgen_candidate_iop_cursors uploads=25 upload_bytes=21850504`
  - KeccakUnion `recursion_witgen_candidate_cycle_prefixes uploads=5 upload_bytes=4450688`

Compared with SP7fr, the only meaningful cache hit was `cycle_prefixes`; the dominant preflight WOM, bucket-bases, and IOP-body buffers remained distinct by exact contents. KeccakUnion wall time moved slightly the wrong direction (`81813 -> 82017 ms`), so this is not an accepted performance gain.

## Decision

Candidate removed. Accepted wall-time gain: 0. Current accepted working state remains SP7fr.

Do not retry broad exact-content recursion metadata caching as an immediate performance path. The repeated shapes are not repeated contents for the dominant buffers, and cursor-like metadata can be semantically mutable. Any future metadata-cache attempt needs a specific measured label with repeated exact contents, proof that the bound buffer is read-only, and a representative e2e wall-time win before retention.
