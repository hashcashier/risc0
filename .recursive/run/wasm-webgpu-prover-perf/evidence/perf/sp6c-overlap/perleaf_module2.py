#!/usr/bin/env python3
"""SP7 iter 6b -- per-leaf module emitter, parameterized chunk-pick.

Like perleaf_module.py but tries chunk0..chunkN per-callee independently
to find the SMALLEST closure for each top-level entry (proxy for the
ideal selector-resolved chunk). Reports the best per-entry size.
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
types_names = {n for n, _ in types_fns}
all_names = steps_names | types_names

chunk_pat = re.compile(r"^(.*?)Chunk(\d+)$")
# base -> sorted [chunk indices] available
chunks_per_base = {}
for n in steps_names:
    m = chunk_pat.match(n)
    if m and m.group(1) in steps_names:
        chunks_per_base.setdefault(m.group(1), []).append(int(m.group(2)))
for b in chunks_per_base:
    chunks_per_base[b].sort()


def rewrite_calls(body, picks):
    # picks: dict base -> chunk_idx
    def repl(m):
        sym = m.group(1)
        if sym in picks:
            target = f"{sym}Chunk{picks[sym]}"
            if target in steps_names:
                return f"{target}("
        return m.group(0)
    return re.sub(r"\b([A-Za-z_][A-Za-z0-9_]*)\(", repl, body)


def closure(root, calls):
    seen, stack = set(), [root]
    while stack:
        n = stack.pop()
        if n in seen:
            continue
        seen.add(n)
        stack.extend(calls.get(n, ()))
    return seen


def build_calls(picks):
    rewritten = {}
    for n, b in steps_fns:
        rewritten[n] = rewrite_calls(b, picks)
    for n, b in types_fns:
        rewritten[n] = rewrite_calls(b, picks)
    calls = {}
    for name, body in rewritten.items():
        callees = set()
        for cm in re.finditer(r"\b([A-Za-z_][A-Za-z0-9_]*)\(", body):
            c = cm.group(1)
            if c in all_names and c != name:
                callees.add(c)
        calls[name] = callees
    return calls, rewritten


prelude_bytes = PRELUDE.read_bytes()
types_bytes = TYPES.read_bytes()
layout_bytes = LAYOUT.read_bytes()
baseline = len(prelude_bytes) + len(types_bytes) + len(layout_bytes)

print(f"baseline: {baseline/1024:.0f} KB | {len(chunks_per_base)} chunked bases")
print()

# Try chunk0-everywhere first (default), then for failing entries
# search per-callee minima.
default_picks = {b: 0 for b in chunks_per_base}
default_calls, default_rewritten = build_calls(default_picks)

targets = sys.argv[1:] or sorted(
    n for n in steps_names
    if re.match(r"^exec_(Top|TopAccum|TopExtract)Chunk\d+$", n)
)


def compute_size(entry, calls, rewritten):
    cl = closure(entry, calls)
    pruned_steps = [(n, rewritten[n]) for n, _ in steps_fns if n in cl]
    reach = sum(len(b) + 1 for _, b in pruned_steps)
    mod = baseline + reach
    return cl, reach, mod, pruned_steps


print(f"{'entry':<32} {'def_KB':>10} {'def_MB':>8} {'best_KB':>10} {'best_MB':>8}")
print("-" * 80)

for entry in targets:
    if entry not in steps_names:
        continue
    cl, reach, mod, pruned = compute_size(entry, default_calls, default_rewritten)
    def_kb, def_mb = reach / 1024, mod / 1024 / 1024

    # Try search: per-callee, find chunk that minimizes the closure
    # contribution. Greedy.
    best_picks = dict(default_picks)
    best_calls, best_rewritten = default_calls, default_rewritten
    best_reach, best_mod = reach, mod

    # Identify chunked bases reachable in default closure
    reachable_chunked = set()
    for n in cl:
        for b, idxs in chunks_per_base.items():
            if any(f"{b}Chunk{i}" in cl for i in idxs):
                reachable_chunked.add(b)

    for base in sorted(reachable_chunked):
        baseline_size = best_reach
        best_for_base = best_picks[base]
        for idx in chunks_per_base[base]:
            tentative = dict(best_picks)
            tentative[base] = idx
            cset, brewr = build_calls(tentative)
            _, r2, m2, _ = compute_size(entry, cset, brewr)
            if r2 < best_reach:
                best_picks[base] = idx
                best_calls, best_rewritten = cset, brewr
                best_reach, best_mod = r2, m2
                best_for_base = idx

    print(
        f"{entry:<32} {def_kb:>10.1f} {def_mb:>8.2f} "
        f"{best_reach/1024:>10.1f} {best_mod/1024/1024:>8.2f}"
    )
