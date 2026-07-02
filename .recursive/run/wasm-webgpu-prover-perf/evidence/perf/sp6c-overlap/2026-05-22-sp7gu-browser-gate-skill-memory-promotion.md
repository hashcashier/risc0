# SP7gu Browser Gate Skill Memory Promotion

Date: 2026-05-22

## Purpose

Promote the browser WebGPU proof-gate availability lesson into durable recursive memory after SP7gt re-confirmed the gate is blocked in this environment.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Run-Local Capture

Relevant events:

- SP7gb: escalated browser command rejected before execution due usage limit.
- SP7gt attempt 1: plain wasm `cargo test` built but failed with `Exec format error` because the wasm browser runner was not configured.
- SP7gt attempt 2: `wasm-bindgen-test-runner` reached startup but failed inside the sandbox with `Operation not permitted`.
- SP7gt attempt 3: required escalated browser-runner command was rejected by approval review due the usage limit.

Operational lesson:

- Browser WebGPU proof validation is not interchangeable with native tests, Naga validation, arithmetic simulation, or wasm compilation.
- Runtime performance code must not be retained while this gate is unavailable.
- Approval rejection must not be worked around indirectly.

## Durable Memory Update

Created:

```text
.recursive/memory/skills/availability/browser-webgpu-proof-gate.md
```

Updated router:

```text
.recursive/memory/skills/SKILLS.md
```

The memory shard records:

- required wasm bindgen browser-runner shape;
- sandbox failure signature;
- escalation boundary;
- allowed and blocked work while unavailable;
- acceptance rule for browser WebGPU performance changes.

## Decision

This is durable repository guidance because future WebGPU prover performance work will repeatedly need the same browser proof gate. The memory update is limited to the capability boundary and does not claim any new performance gain.
