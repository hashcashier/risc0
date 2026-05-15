#!/usr/bin/env python3
"""Show the heaviest fns in exec_TopAccumChunk0's closure to identify
what's pulling in 1.7 MB of code."""
import re
import sys
from pathlib import Path

OUT = Path("/tmp/zirgen-out8")
STEPS = OUT / "steps.wgsl"
TYPES = OUT / "types.wgsl.inc"


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
all_fns = {n: b for n, b in steps_fns + types_fns}
names = set(all_fns)

# Same chunk0 rewriting as perleaf_module.py
chunk_pat = re.compile(r"^(.*?)Chunk(\d+)$")
chunked_bases = {m.group(1) for n in names if (m := chunk_pat.match(n)) and m.group(1) in names}


def rewrite(body):
    def repl(m):
        sym = m.group(1)
        if sym in chunked_bases and f"{sym}Chunk0" in names:
            return f"{sym}Chunk0("
        return m.group(0)
    return re.sub(r"\b([A-Za-z_][A-Za-z0-9_]*)\(", repl, body)


rewritten = {n: rewrite(b) for n, b in all_fns.items()}
calls = {}
for name, body in rewritten.items():
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


entry = sys.argv[1] if len(sys.argv) > 1 else "exec_TopAccumChunk0"
cl = closure(entry)
sized = sorted(
    [(n, len(rewritten[n])) for n in cl if n in {n2 for n2, _ in steps_fns}],
    key=lambda x: -x[1],
)
total = sum(s for _, s in sized)
print(f"Closure of {entry}: {len(cl)} fns reaching {total/1024:.1f} KB (steps fns only)")
print(f"Top 30 by size:")
for name, sz in sized[:30]:
    callers = [c for c, cs in calls.items() if name in cs and c in cl]
    print(f"  {sz/1024:>7.1f} KB  {name:<60} (called by {len(callers)} in closure)")

# Find chunked bases reachable that have NO chunk-rewrite (i.e., we're calling
# non-chunked deeper helpers).
print()
print("Top non-chunk callees (potential next-level chunk targets):")
in_closure = set(cl)
nonchunk_callees = {}
for n in cl:
    for c in calls.get(n, ()):
        if c in in_closure and not chunk_pat.match(c) and c not in chunked_bases:
            nonchunk_callees.setdefault(c, 0)
            nonchunk_callees[c] += 1
nonchunk_sorted = sorted(
    [(c, len(rewritten.get(c, "")), cnt) for c, cnt in nonchunk_callees.items()],
    key=lambda x: -x[1],
)
for name, sz, cnt in nonchunk_sorted[:20]:
    print(f"  {sz/1024:>7.1f} KB  {name:<60} (called by {cnt} fns in closure)")
