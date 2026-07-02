# SP7gs Persist Final Tiled NTT Shader

Date: 2026-05-22

## Purpose

Persist the final SP7gn tiled strided-local NTT WGSL shader inside the run evidence so future implementation does not depend on a `/tmp` scratch file.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Durable Shader

Persisted path:

```text
.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7gn-strided-local-ntt-tiled-final.wgsl
```

Original scratch path:

```text
/tmp/sp7gn-strided-local-ntt-tiled-final.wgsl
```

The durable shader is `4820` bytes.

## Verification

Validation command:

```text
naga --input-kind wgsl .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7gn-strided-local-ntt-tiled-final.wgsl
```

Result:

```text
Validation successful
```

Byte identity check:

```text
cmp -s /tmp/sp7gn-strided-local-ntt-tiled-final.wgsl .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7gn-strided-local-ntt-tiled-final.wgsl
```

Result: exit code `0`.

## Handoff Update

Updated SP7gn, SP7go, SP7ge, and SP7gq to point implementation handoff at the durable run-evidence WGSL path rather than the temp path.

The browser proof gate remains blocked by SP7gb, so this is a durability/readiness fix only.
