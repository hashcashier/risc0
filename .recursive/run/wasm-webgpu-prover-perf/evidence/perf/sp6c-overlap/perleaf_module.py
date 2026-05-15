#!/usr/bin/env python3
"""SP7 iter 6b -- per-leaf module emitter (chunk-rewriting probe).

For a top-level chunk (e.g. `exec_TopAccumChunk0`), produce a per-leaf
WGSL module by:
  1. Walking its closure over chunked steps.wgsl + types.wgsl.inc
  2. Rewriting every call to a chunked-original (`exec_W(`) -> chunk0
     (`exec_WChunk0(`) -- ONLY for symbols that have at least one ChunkN
  3. Recomputing closure on the rewritten body
  4. Emitting prelude + types + layout + (closure'd steps fns)

The chunk0-everywhere choice is functionally INCORRECT (the right chunk
depends on the per-arm selector that fires, which we don't track yet).
This probe ANSWERS: with one chunk per worker linked, does the per-leaf
module clear the iter-5b/d cliffs (whole < 2 MB, reachable < 400 KB)?
If yes -> the design is sound; the real wiring just needs the
chunk-selection metadata. If no -> we need finer-grained closure
shrinking before the design is viable.

Usage: python3 perleaf_module.py <entry_chunk_name>
"""
import re
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

# A symbol is "chunked" if there's at least one Chunk<N> with the same
# base name in steps.wgsl.
chunk_pat = re.compile(r"^(.*?)Chunk(\d+)$")
chunked_bases = set()
for n in steps_names:
    m = chunk_pat.match(n)
    if m and m.group(1) in steps_names:
        chunked_bases.add(m.group(1))


def rewrite_calls(body):
    # For each callsite `<sym>(`, if <sym> is in chunked_bases AND a
    # Chunk0 exists, swap to <sym>Chunk0(. Skip in identifier prefixes
    # (e.g. `exec_FooBar` should not match a base `exec_Foo`).
    def repl(m):
        sym = m.group(1)
        if sym in chunked_bases and f"{sym}Chunk0" in steps_names:
            return f"{sym}Chunk0("
        return m.group(0)

    return re.sub(r"\b([A-Za-z_][A-Za-z0-9_]*)\(", repl, body)


# Build call graph from rewritten bodies (rewriting first means the
# closure walk doesn't pull in the un-chunked originals).
rewritten_steps = {n: rewrite_calls(b) for n, b in steps_fns}
rewritten_types = {n: rewrite_calls(b) for n, b in types_fns}
rewritten = {**rewritten_steps, **rewritten_types}

calls = {}
for name, body in rewritten.items():
    callees = set()
    for cm in re.finditer(r"\b([A-Za-z_][A-Za-z0-9_]*)\(", body):
        c = cm.group(1)
        if c in all_names and c != name:
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

if len(sys.argv) < 2:
    # Default: probe all top-level chunks
    targets = [n for n in steps_names if re.match(r"^exec_(Top|TopAccum|TopExtract)Chunk\d+$", n)]
    targets.sort()
else:
    targets = sys.argv[1:]

print(f"baseline prelude+types+layout: {baseline/1024:.0f} KB")
print(f"chunked bases: {len(chunked_bases)}")
print()
print(f"{'entry':<36} {'closure':>8} {'reach_KB':>10} {'mod_MB':>8}  verdict")
print("-" * 88)

cliff_module_mb = 2.0
cliff_reach_kb = 400.0

for entry in targets:
    if entry not in steps_names:
        print(f"{entry}: NOT FOUND in steps", file=sys.stderr)
        continue
    cl = closure(entry)
    pruned_steps = [(n, rewritten_steps[n]) for n, _ in steps_fns if n in cl]
    reach_bytes = sum(len(b) + 1 for _, b in pruned_steps)
    mod_bytes = baseline + reach_bytes
    verdict = ""
    if mod_bytes / 1024 / 1024 >= cliff_module_mb:
        verdict += "MOD>2MB "
    if reach_bytes / 1024 >= cliff_reach_kb:
        verdict += "REACH>400KB"
    if not verdict:
        verdict = "OK"
    print(
        f"{entry:<36} {len(cl):>8d} {reach_bytes/1024:>10.1f} "
        f"{mod_bytes/1024/1024:>8.2f}  {verdict}"
    )

    # Emit + naga-validate the first one for verification.
    if entry == targets[0]:
        out_path = Path(f"/tmp/perleaf_{entry}.wgsl")
        steps_block = "\n".join(b for _, b in pruned_steps) + "\n"
        out_path.write_bytes(
            prelude_bytes + types_bytes + layout_bytes + steps_block.encode()
        )
        print(f"  wrote {out_path} ({out_path.stat().st_size/1024:.1f} KB)")
        import subprocess
        rc = subprocess.run(["naga", str(out_path)], capture_output=True, text=True)
        if rc.returncode != 0:
            print(f"  naga FAILED:")
            print(rc.stderr[:600])
        else:
            print(f"  naga OK")
