#!/usr/bin/env python3
"""SP7 iter 6b -- per-entry pruned modules from the iter-6a chunked output.

For each chunk fn (`exec_TopChunkN` etc.), compute the transitive call
closure over the chunked steps.wgsl + types.wgsl.inc, then emit a pruned
module = witgen_prelude + types + layout + (steps fns in closure).

Checks each pruned module against:
  - whole-module ceiling: should be < 2 MB (iter-5b: in (1.99, 3.27] MB)
  - reachable-closure size: should be < 400 KB (iter-5d: 0.39 MB device-loses)
"""
import re
import subprocess
import sys
from pathlib import Path

OUT = Path("/tmp/zirgen-out8")
ZIRGEN = Path("/home/rami/repos/zirgen")
PRELUDE = ZIRGEN / "zirgen/compiler/codegen/gpu/witgen_prelude.wgsl"
STEPS = OUT / "steps.wgsl"
TYPES = OUT / "types.wgsl.inc"
LAYOUT = OUT / "layout.wgsl.inc"


def parse_fns(path):
    lines = path.read_text().split("\n")
    starts = []
    for i, ln in enumerate(lines):
        m = re.match(r"fn ([A-Za-z_][A-Za-z0-9_]*)\(", ln)
        if m:
            starts.append((m.group(1), i))
    fns = []
    for k, (name, s) in enumerate(starts):
        e = starts[k + 1][1] if k + 1 < len(starts) else len(lines)
        fns.append((name, "\n".join(lines[s:e])))
    return fns


steps_fns = parse_fns(STEPS)
types_fns = parse_fns(TYPES)
steps_names = {n for n, _ in steps_fns}
all_fns = {n: b for n, b in steps_fns + types_fns}
names = set(all_fns)

calls = {}
for name, body in all_fns.items():
    callees = set()
    for cm in re.finditer(r"\b([A-Za-z_][A-Za-z0-9_]*)\(", body):
        c = cm.group(1)
        if c in names and c != name:
            callees.add(c)
    calls[name] = callees


def closure(root):
    seen, stack = set(), [root]
    while stack:
        n = stack.pop()
        if n in seen:
            continue
        seen.add(n)
        stack.extend(calls.get(n, ()))
    return seen


prelude_bytes = PRELUDE.read_bytes()
types_bytes = TYPES.read_bytes()
layout_bytes = LAYOUT.read_bytes()
baseline = len(prelude_bytes) + len(types_bytes) + len(layout_bytes)
print(f"baseline prelude+types+layout: {baseline/1024:.0f} KB ({baseline/1024/1024:.2f} MB)")
print(f"steps.wgsl total: {len(STEPS.read_bytes())/1024/1024:.2f} MB ({len(steps_fns)} fns)")
print()

# All chunk entries we care about: anything ending in ChunkN (the chunks
# of every wide switch) plus the entry shims step$Top/step$Top$accum.
chunk_entries = sorted(
    n for n in steps_names if re.match(r"^exec_.*Chunk\d+$", n)
)
shim_entries = [n for n in ("step_Top", "step_TopAccum") if n in steps_names]
entries = shim_entries + chunk_entries

print(f"{'entry':<48} {'closure_size':>12} {'reach_KB':>10} {'mod_MB':>8}  verdict")
print("-" * 100)

cliff_module_mb = 2.0
cliff_reach_kb = 400.0

failures = []
sizes = []
for entry in entries:
    cl = closure(entry)
    pruned_steps = [(n, b) for n, b in steps_fns if n in cl]
    reach_bytes = sum(len(b) + 1 for _, b in pruned_steps)
    mod_bytes = baseline + reach_bytes
    reach_kb = reach_bytes / 1024
    mod_mb = mod_bytes / 1024 / 1024
    verdict = ""
    if mod_mb >= cliff_module_mb:
        verdict += "MOD>2MB "
        failures.append(entry)
    if reach_kb >= cliff_reach_kb:
        verdict += "REACH>400KB"
        if entry not in failures:
            failures.append(entry)
    if not verdict:
        verdict = "OK"
    sizes.append((entry, reach_kb, mod_mb))
    print(f"{entry:<48} {len(cl):>12d} {reach_kb:>10.1f} {mod_mb:>8.2f}  {verdict}")

print()
print(f"total entries: {len(entries)}, failures: {len(failures)}")
if failures:
    print(f"FAILED: {failures[:10]}{'...' if len(failures) > 10 else ''}")
    sys.exit(1)
print("ALL UNDER CLIFFS")
