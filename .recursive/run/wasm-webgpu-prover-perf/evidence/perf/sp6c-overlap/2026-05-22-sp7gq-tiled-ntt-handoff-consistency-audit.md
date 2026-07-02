# SP7gq Tiled NTT Handoff Consistency Audit

Date: 2026-05-22

## Purpose

Remove stale wording from the tiled strided-local NTT handoff artifacts before runtime implementation resumes.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Findings

The consistency scan checked the SP7g* NTT evidence files for:

- `offsets_per_tile` formulas;
- `total_tiles` versus `total_strided_groups` naming;
- old `total_offsets` wording;
- final shader handoff path;
- WebGPU workgroup-storage limit language.

Issues found:

1. SP7ge, SP7gj, and SP7gk still used `min_power_of_two(16, 1024 / blocks_per_row)` wording even though the final SP7go host checklist uses `min(16, 1024 / blocks_per_row)`.
2. SP7ge still pointed at `/tmp/sp7gj-strided-local-ntt-tiled.wgsl` as the validated scratch shader, while SP7gn supersedes it with `/tmp/sp7gn-strided-local-ntt-tiled-final.wgsl`.
3. SP7gg was still easy to misread as the final guard audit even though it only audits the original `n_bits=20`, one-offset-per-workgroup shape.

## Fixes

Updated SP7ge:

- changed the guard formula to `offsets_per_tile == min(16, 1024 / blocks_per_row)`;
- changed the shader handoff text to reference `/tmp/sp7gn-strided-local-ntt-tiled-final.wgsl`;
- explicitly states the SP7gn scratch shader supersedes the earlier SP7gj scratch shader for implementation handoff.

Updated SP7gj and SP7gk:

- changed the formula to `offsets_per_tile = min(16, 1024 / blocks_per_row)`;
- added that `1024 / blocks_per_row` is already a power of two for guarded shapes, so no new helper is required.

Updated SP7gg:

- added a supersession note: SP7gg remains useful only for original `n_bits=20` twiddle/dispatch sanity, not final param layout or shader shape.

## Authoritative Handoff After This Audit

Use these artifacts for implementation when the browser proof gate returns:

1. SP7gn for final WGSL shape:
   - `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7gn-strided-local-ntt-tiled-final.wgsl`
2. SP7go for runtime patch sequence and acceptance gates.
3. SP7gf for proof validation checklist.
4. SP7gp for objective gap and blocker status.

Older SP7gc/SP7gd/SP7gg/SP7gh artifacts are historical/static support and should not override SP7gn/SP7go.

## Verification

The consistency scan used:

```text
rg -n "min_power_of_two|min\\(16|total_offsets|total_tiles|total_strided_groups|offsets_per_tile|48-byte|48 bytes|SP7g" .recursive/run/wasm-webgpu-prover-perf/evidence/perf/sp6c-overlap/2026-05-22-sp7g*.md
```

No production code changed; browser proof validation remains blocked by SP7gb.

Post-fix scan note: remaining `/tmp/sp7gj-strided-local-ntt-tiled.wgsl` hits are historical validation references in SP7gj, and remaining `total_offsets` hits are historical one-offset guard references in SP7gg/SP7gk. The authoritative implementation handoff is SP7gn/SP7go.
