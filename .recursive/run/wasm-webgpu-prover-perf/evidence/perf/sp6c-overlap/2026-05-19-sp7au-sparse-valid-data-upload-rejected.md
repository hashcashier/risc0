# SP7au: Sparse Valid Data Upload Rejected

Date: 2026-05-19

## Scope

Follow-up after SP7at tried to cut the remaining RV32IM `source=data`
upload by uploading only CPU-valid cells. The hypothesis was that sparse
valid tracking would reduce bytes without changing the authoritative
MISC0 GPU-witgen replacement path.

## Evidence

RED first generated and verified the BusyLoop receipt, then failed the
new upload cap:

```text
BusyLoop source=data upload_bytes=355467264 > 340000000
```

Attempt 1 reduced BusyLoop data bytes but exploded queue writes:

```text
BusyLoop source=data upload_bytes=266674716 uploads=1426339 wall_ms=10344
KeccakUnion source=data upload_bytes=4545070980 uploads=3591752 failed cap
```

Attempt 2 batched more aggressively but still did not win wall time:

```text
BusyLoop source=data upload_bytes=323401400 uploads=1032 wall_ms=9484
KeccakUnion source=data upload_bytes=4722309012 uploads=4049 failed cap
```

## Decision

Rejected. CPU-driven sparse valid uploads trade a large contiguous upload
for too many small queue writes, or for byte savings too small to move wall
time. Do not retry this route without GPU-side batching or direct
GPU-resident accumulation that removes the final CPU-shadow upload
dependency.
