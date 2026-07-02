# SP7fc: recursion verify_mem WGSL capacity probe

Date: 2026-05-22

## Purpose

SP7fa rejected monolithic recursion `exec` WGSL and SP7fb showed selector-chunked `exec` is browser-capacity viable. This probe checked whether the sibling recursion `verify_mem` stage also needs chunking before runtime wiring.

## Prototype

A temporary external Zirgen patch emitted WGSL for the `wom_verify` stage, producing `step_verify_mem.wgsl`.

Generated artifact sizes:

| Artifact | Size |
|---|---:|
| `step_verify_mem.wgsl` | `53795` bytes |
| `layout.wgsl.inc` | `36358` bytes |

The generated `verify_mem` WGSL references only `extern_plonkRead`.

The temporary assembled module used:

- current RISC0 recursion WGSL prelude
- generated layout
- generated `step_verify_mem.wgsl`
- temporary `extern_plonkRead` stub
- `recursion_step_verify_mem_main` entry point

Assembled module size: `105068` bytes.

Static validation:

- `naga /tmp/sp7fc-recursion-step-verify-mem-assembled.wgsl`
- Result: passed

## Browser capacity result

Raw log:

- `2026-05-22-sp7fc-recursion-verify-mem-capacity-probe.chrome.txt`

Focused browser probe result:

- High WebGPU limits: `4294967292 / 2147483644 / 49152`
- `recursion_verify_mem_wgsl verify_mem_bytes=105068`
- `recursion_accum_probe recursion_step_verify_mem_main module_bytes=105068 phase=OK`
- Test passed in `0.81s`

## Cleanup

No RISC0 runtime code was retained from this probe. The temporary browser test that included the `/tmp` assembled module was removed after the log was captured.

The temporary external Zirgen `wom_verify` emission patch was reverted. Zirgen still retains the accepted accumulator WGSL generator changes for `compute_accum` and `verify_accum`.

## Decision

Monolithic `verify_mem` WGSL is browser-capacity safe. Only recursion `exec` needs selector-level chunking for capacity.

This is not a correctness or performance acceptance gate: externs were temporary stubs, no real WOM rows were read, no proof path used the shader, and no e2e proof was run. The next implementation step remains real recursion witness offload wiring: selector-chunked `exec`, real preflight/WOM extern buffers, and then representative proof gates.
