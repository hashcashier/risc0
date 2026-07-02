# SP7fa: monolithic recursion exec WGSL rejected

Date: 2026-05-22

## Candidate

Temporarily extended the external Zirgen WGSL emission guard to emit recursion `exec` WGSL in addition to `compute_accum` and `verify_accum`.

The generated output in `/tmp/sp7fa-recursion-wgsl-exec` included:

| Artifact | Size |
|---|---:|
| `step_exec.wgsl` | `550440` bytes |
| `step_compute_accum.wgsl` | `266817` bytes |
| `step_verify_accum.wgsl` | `131542` bytes |
| `layout.wgsl.inc` | `36358` bytes |

The assembled exec module with the RISC0 prelude, layout, temporary exec extern stubs, and compute entry was `602154` bytes.

## Static validation

`naga /tmp/sp7fa-recursion-step-exec-assembled.wgsl` passed.

The generated exec module references these extern families:

- `extern_womRead`
- `extern_womWrite`
- `extern_readIOPHeader`
- `extern_readIOPBody`
- `extern_readCoefficients`
- `extern_plonkWrite`
- `extern_noop`

Those externs still need real preflight/WOM buffer semantics before correctness can be attempted.

## Browser capacity probe

Raw log:

- `2026-05-22-sp7fa-recursion-exec-wgsl-capacity-probe.chrome.txt`

Focused browser probe:

- High WebGPU limits were negotiated: `4294967292 / 2147483644 / 49152`.
- The module passed static validation but failed on Chrome dispatch/readback.
- Failure mode: `AbortError: Failed to execute 'mapAsync' on 'GPUBuffer': A valid external Instance reference no longer exists.`

This reproduces the earlier large-module capacity cliff on real Chrome/Dawn. The failure happened with zeroed buffers and a one-row dispatch, so wiring real extern semantics would not make the monolithic module viable.

## Decision

Rejected and reverted. No RISC0 runtime code or Zirgen exec-WGSL emission change was retained.

The next viable recursion witness offload path is chunked/split exec WGSL, with each generated module kept below the observed browser capacity cliff, plus real RISC0 preflight/WOM extern buffer wiring. Monolithic recursion exec WGSL should not be retried.
