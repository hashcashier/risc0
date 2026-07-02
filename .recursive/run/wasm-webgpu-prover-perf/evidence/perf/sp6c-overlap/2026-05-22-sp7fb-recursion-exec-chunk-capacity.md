# SP7fb: selector-chunked recursion exec WGSL capacity probe

Date: 2026-05-22

## Purpose

SP7fa proved monolithic recursion `exec` WGSL is not browser-viable: the assembled module was `602154` bytes and Chrome/Dawn lost the device on dispatch.

This probe checked whether selector-level chunking is likely to fit under the browser capacity cliff.

## Prototype

Using the SP7fa generated `step_exec.wgsl`, a temporary parser grouped top-level selector blocks. The largest selector group was the `x429` group:

- Kept selector-group bytes: `302417`
- Skipped selector-group bytes: `247160`
- Assembled module with prelude/layout/temporary extern stubs: `354992` bytes

Static validation:

- `naga /tmp/sp7fa-step-exec-x429-assembled.wgsl`
- Result: passed

## Browser capacity result

Raw log:

- `2026-05-22-sp7fb-recursion-exec-x429-chunk-capacity-probe.chrome.txt`

Focused browser probe result:

- High WebGPU limits: `4294967292 / 2147483644 / 49152`
- `recursion_exec_x429_wgsl exec_bytes=354992`
- `recursion_accum_probe recursion_step_exec_main module_bytes=354992 phase=OK`
- Test passed in `33.04s`

## Decision

Selector-level chunking is viable for browser compile/dispatch capacity. No runtime code was retained from this probe.

Next implementation should generate or postprocess recursion `exec` WGSL into selector-group chunks, keep each assembled chunk below the observed capacity cliff, then wire real RISC0 preflight/WOM extern buffers before attempting correctness or e2e proof gates.
