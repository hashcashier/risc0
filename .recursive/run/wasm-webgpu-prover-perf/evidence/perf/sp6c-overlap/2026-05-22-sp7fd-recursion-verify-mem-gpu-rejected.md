# SP7fd: recursion verify_mem GPU candidate rejected

Date: 2026-05-22

## Candidate

Implemented an opt-in hybrid recursion witness slice:

- CPU `step_exec`
- CPU WOM sort/prefix/inject
- WebGPU generated `step_verify_mem`
- real sorted WOM row and per-cycle offset buffers

The goal was to move part of `recursion_witgen` to GPU without taking on selector-chunked `exec` yet.

## Validation

Failed first attempt:

- Log: `2026-05-22-sp7fd-representative-recursion-verify-mem-gpu-candidate.chrome.txt`
- Result: BusyLoop failed during lift verification.
- Root cause: the same writable dummy buffer was bound at bindings 3, 4, and 5, which WebGPU rejected as writable storage aliasing. The invalid command buffer poisoned the proof path.

Failed second attempt:

- Log: `2026-05-22-sp7fd-rerun-representative-recursion-verify-mem-gpu-candidate.chrome.txt`
- Result: BusyLoop failed during lift verification without WebGPU validation errors.
- Root cause: WOM rows were packed as decoded integers. The generated WGSL expects BabyBear Montgomery words, matching CPU `Fp::new(addr)` and `Fp` value limbs.

Correctness-positive repaired attempt:

- Log: `2026-05-22-sp7fd-rerun2-representative-recursion-verify-mem-gpu-candidate.chrome.txt`
- BusyLoop proof verified: `wall_ms=5701`
- KeccakUnion proof verified: `wall_ms=85557`
- Total test runtime: `92.05s`
- Fallback counters: zero CPU fallback / zero CPU-only ops

## Performance Result

Current accepted SP7ez representative baseline:

- BusyLoop: `5665 ms`
- KeccakUnion: `85171 ms`
- Total runtime: `91.58s`
- `recursion_witgen` summed across the representative run: `6955 ms`

SP7fd repaired candidate:

- BusyLoop: `5701 ms` (`+36 ms`)
- KeccakUnion: `85557 ms` (`+386 ms`)
- Total runtime: `92.05s` (`+0.47s`)
- `recursion_witgen` summed across the representative run: `7142 ms` (`+187 ms`)

The candidate also introduced large uploads that are absent from the accepted path:

- `recursion_data`: `3489660928` bytes
- `recursion_verify_mem_rows`: `1009457280` bytes
- `recursion_verify_mem_offsets`: `22432384` bytes

## Decision

Rejected and removed. This slice is correctness-positive after the packing fix, but it is structurally the wrong performance shape: CPU exec still owns the data, so GPU `verify_mem` requires full recursion data upload plus nearly 1 GiB of sorted WOM row uploads on the representative gate.

Do not promote CPU-exec + GPU-verify_mem as a runtime path. The useful retained lesson is that generated `verify_mem` semantics work when WOM rows are Montgomery-packed and buffers do not alias, but the performance path must keep exec/WOM production GPU-resident before running `verify_mem`.
