#!/usr/bin/env python3
"""iter-6d-g: emit per-arm "delta" WGSL modules (just steps fns) for
vendoring. Caller concatenates shared prelude+types+layout at runtime.

Usage: gen_delta.py <out_path> <entry> <target_chunk>
"""

import re
import sys
from collections import defaultdict


def parse_fns(text):
    matches = []
    for m in re.finditer(r'(?:^|\n)fn ([A-Za-z_][A-Za-z0-9_]*)\s*\(', text):
        name = m.group(1)
        fn_offset = m.start() + (0 if m.group(0).startswith('fn') else 1)
        matches.append((name, fn_offset))
    fns = []
    for i, (name, start) in enumerate(matches):
        end = matches[i + 1][1] if i + 1 < len(matches) else len(text)
        fns.append((name, text[start:end]))
    return fns


def chunked_max_idx(steps_names):
    by_base = defaultdict(lambda: -1)
    for n in steps_names:
        idx = n.rfind("Chunk")
        if idx == -1:
            continue
        base = n[:idx]
        suffix = n[idx + len("Chunk"):]
        if not suffix or not suffix.isdigit():
            continue
        if base not in steps_names:
            continue
        by_base[base] = max(by_base[base], int(suffix))
    return dict(by_base)


def rewrite_to_chunk(body, max_by_base, target):
    def repl(m):
        ident = m.group(1)
        if ident in max_by_base:
            k = min(target, max_by_base[ident])
            return f"{ident}Chunk{k}("
        return m.group(0)
    return re.sub(r'\b([A-Za-z_][A-Za-z0-9_]*)\(', repl, body)


def callees_in(body, valid):
    found = set()
    for m in re.finditer(r'\b([A-Za-z_][A-Za-z0-9_]*)\(', body):
        if m.group(1) in valid:
            found.add(m.group(1))
    return found


def emit_delta(types_inc, steps, entry, target):
    steps_fns = parse_fns(steps)
    types_fns = parse_fns(types_inc)
    all_names = {n for n, _ in steps_fns} | {n for n, _ in types_fns}
    steps_names = {n for n, _ in steps_fns}
    if entry not in steps_names:
        raise SystemExit(f"entry not in steps: {entry}")
    max_by_base = chunked_max_idx(steps_names)
    rewritten = {n: rewrite_to_chunk(b, max_by_base, target) for n, b in steps_fns}
    calls = {n: callees_in(b, all_names) for n, b in rewritten.items()}
    closure = set()
    stack = [entry]
    while stack:
        n = stack.pop()
        if n in closure:
            continue
        closure.add(n)
        for c in calls.get(n, ()):
            stack.append(c)
    out = []
    for n, _ in steps_fns:
        if n in closure:
            out.append(rewritten[n])
            if not out[-1].endswith("\n"):
                out.append("\n")
    return "".join(out)


def main():
    out_path = sys.argv[1]
    entry = sys.argv[2]
    target = int(sys.argv[3]) if len(sys.argv) > 3 else 0
    with open("/tmp/zirgen-out8/types.wgsl.inc") as f:
        types_inc = f.read()
    with open("/tmp/zirgen-out8/steps.wgsl") as f:
        steps = f.read()
    delta = emit_delta(types_inc, steps, entry, target)
    with open(out_path, "w") as f:
        f.write(delta)
    print(f"Wrote {out_path}: {len(delta)} bytes")


if __name__ == "__main__":
    main()
