#!/usr/bin/env python3
"""iter-6d-f: emit a pruned WGSL module that calls ALL chunks of each
chunked-base sequentially and OR-merges the returns. Relies on Tint
zero-initializing uninitialized vars (Chrome WebGPU policy) so
non-firing chunks contribute zero to the merge.

Usage: gen_all_chunks.py <out_path> <entry> [target_chunk_default]
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


def fn_signature(body):
    """Extract (name, args_str, return_type) from a fn body."""
    m = re.match(r'fn ([A-Za-z_][A-Za-z0-9_]*)\s*\((.*?)\)\s*(?:->\s*([A-Za-z_][A-Za-z0-9_]*))?', body, re.DOTALL)
    if not m:
        return None
    return (m.group(1), m.group(2), m.group(3))


def parse_struct_fields(text):
    """Yield (struct_name, [(field_name, field_type), ...])."""
    out = {}
    for m in re.finditer(r'struct\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{([^}]*)\}', text):
        name = m.group(1)
        body = m.group(2)
        fields = []
        for line in body.split(','):
            line = line.strip()
            if not line:
                continue
            parts = line.split(':')
            if len(parts) != 2:
                continue
            fields.append((parts[0].strip(), parts[1].strip()))
        out[name] = fields
    return out


# Primitive types where OR-merge is just `a | b`. Other types need recursive merging.
PRIMITIVE_TYPES = {"u32", "i32", "Val"}


def merge_expr(struct_defs, type_name, a, b, depth=0):
    """Returns a WGSL expression that OR-merges `a` and `b` of `type_name`."""
    if depth > 8:
        # Cap recursion to avoid runaway on cycles; in practice 3-4 levels.
        return f"{a} /* deep, no merge */"
    if type_name in PRIMITIVE_TYPES:
        return f"({a} | {b})"
    fields = struct_defs.get(type_name)
    if fields is None:
        # Array or unknown type. For known fixed arrays we can't easily expand;
        # fall back to leaving `a` alone (best effort).
        return f"{a}"
    # Construct struct with field-wise merge.
    parts = []
    for fname, ftype in fields:
        parts.append(merge_expr(struct_defs, ftype, f"{a}.{fname}", f"{b}.{fname}", depth + 1))
    return f"{type_name}({', '.join(parts)})"


def emit_merge_helpers(struct_defs, types_used):
    """Emit fn merge_<T>(a: T, b: T) -> T for each T in types_used."""
    out = []
    for t in sorted(types_used):
        if t in PRIMITIVE_TYPES:
            continue
        expr = merge_expr(struct_defs, t, "a", "b")
        out.append(f"fn merge_{t}(a: {t}, b: {t}) -> {t} {{\n  return {expr};\n}}")
    return "\n".join(out)


def emit_combined(base, max_idx, sig, struct_defs):
    """Emit fn exec_BASE_combined(args) -> RT that calls all chunks and merges."""
    args_decl = sig[1]
    rt = sig[2] or "void"
    # Parse arg names from args_decl.
    arg_names = []
    for a in args_decl.split(','):
        a = a.strip()
        if not a:
            continue
        # arg_name: type
        n = a.split(':')[0].strip()
        arg_names.append(n)
    args_call = ", ".join(arg_names)
    if rt == "void":
        body_calls = "\n".join(f"  {base}Chunk{i}({args_call});" for i in range(max_idx + 1))
        return f"fn {base}_combined({args_decl}) {{\n{body_calls}\n}}"
    # Compute merge expression
    calls = [f"{base}Chunk{i}({args_call})" for i in range(max_idx + 1)]
    if max_idx == 0:
        return f"fn {base}_combined({args_decl}) -> {rt} {{\n  return {calls[0]};\n}}"
    # let r0 = ...; let r1 = ...; ...; return merge(merge(r0,r1),r2)
    lets = "\n".join(f"  let r{i} = {calls[i]};" for i in range(max_idx + 1))
    merge_chain = "r0"
    for i in range(1, max_idx + 1):
        merge_chain = f"merge_{rt}({merge_chain}, r{i})"
    return f"fn {base}_combined({args_decl}) -> {rt} {{\n{lets}\n  return {merge_chain};\n}}"


def rewrite_calls_to_combined(body, chunked_max_idx, base_calls_made):
    """Replace `<base>(` callsites with `<base>_combined(` and track which combineds we need."""
    def repl(m):
        ident = m.group(1)
        if ident in chunked_max_idx:
            base_calls_made.add(ident)
            return f"{ident}_combined("
        return m.group(0)
    return re.sub(r'\b([A-Za-z_][A-Za-z0-9_]*)\(', repl, body)


def callees_in(body, valid):
    found = set()
    for m in re.finditer(r'\b([A-Za-z_][A-Za-z0-9_]*)\(', body):
        if m.group(1) in valid:
            found.add(m.group(1))
    return found


def main():
    prelude_path = "/home/rami/repos/zirgen/zirgen/compiler/codegen/gpu/witgen_prelude.wgsl"
    in_dir = "/tmp/zirgen-out8"
    out_path = sys.argv[1] if len(sys.argv) > 1 else "/tmp/iter6d_f_exec_top_all_chunks.wgsl"
    entry = sys.argv[2] if len(sys.argv) > 2 else "exec_TopChunk0"

    with open(prelude_path) as f:
        prelude = f.read()
    with open(f"{in_dir}/types.wgsl.inc") as f:
        types_inc = f.read()
    with open(f"{in_dir}/layout.wgsl.inc") as f:
        layout_inc = f.read()
    with open(f"{in_dir}/steps.wgsl") as f:
        steps = f.read()

    steps_fns = parse_fns(steps)
    types_fns = parse_fns(types_inc)
    all_names = {n for n, _ in steps_fns} | {n for n, _ in types_fns}
    steps_names = {n for n, _ in steps_fns}

    chunked_max = chunked_max_idx(steps_names)
    struct_defs = parse_struct_fields(types_inc)
    # Get base function signatures for emit_combined.
    base_sigs = {}
    for name, body in steps_fns:
        if name in chunked_max:
            # find the Chunk0 signature
            chunk0_name = f"{name}Chunk0"
            for n2, b2 in steps_fns:
                if n2 == chunk0_name:
                    sig = fn_signature(b2)
                    if sig:
                        base_sigs[name] = sig
                    break

    # Rewrite every fn body: replace `exec_BASE(` with `exec_BASE_combined(`.
    base_calls = set()
    rewritten_steps = {}
    for name, body in steps_fns:
        rewritten_steps[name] = rewrite_calls_to_combined(body, chunked_max, base_calls)
    rewritten_types = {}
    for name, body in types_fns:
        rewritten_types[name] = rewrite_calls_to_combined(body, chunked_max, base_calls)

    # Closure from entry: traverse rewritten bodies. The fn names we'll call
    # are <base>_combined for chunked bases (we'll synthesize these below)
    # plus all ChunkN siblings (the combined helpers call them).
    # Build call graph that includes the synthesized helpers.
    synth_names = {f"{b}_combined" for b in base_calls}
    all_names_with_synth = all_names | synth_names

    # For each used base, the combined helper transitively calls every ChunkN.
    # Add those edges in our closure.
    calls = {}
    for name, body in rewritten_steps.items():
        calls[name] = callees_in(body, all_names_with_synth)
    for name, body in rewritten_types.items():
        calls[name] = callees_in(body, all_names_with_synth)
    for base in base_calls:
        max_idx = chunked_max[base]
        # _combined calls all chunks; each chunk's body is the ORIGINAL (not rewritten in this branch)
        # We want the closure to include all chunks.
        combined_callees = set()
        for i in range(max_idx + 1):
            chunk_name = f"{base}Chunk{i}"
            combined_callees.add(chunk_name)
        # The combined helper also calls the merge helper if multi-chunk.
        rt = base_sigs.get(base, ("", "", None))[2]
        if rt and rt not in PRIMITIVE_TYPES and max_idx > 0:
            # Merge helper name -- we'll emit it.
            pass
        calls[f"{base}_combined"] = combined_callees

    # BFS closure from entry.
    closure = set()
    stack = [entry]
    while stack:
        n = stack.pop()
        if n in closure:
            continue
        closure.add(n)
        for c in calls.get(n, ()):
            stack.append(c)

    # Collect return types of base functions used so we know which merge helpers to emit.
    types_used = set()
    for base in base_calls:
        sig = base_sigs.get(base)
        if sig and sig[2] and sig[2] not in PRIMITIVE_TYPES:
            types_used.add(sig[2])

    # Emit module: prelude + types_inc + layout_inc + merge helpers
    # + (chunks in closure, original bodies) + (combined helpers).
    out = [prelude]
    if not out[-1].endswith("\n"):
        out.append("\n")
    out.append(types_inc)
    if not out[-1].endswith("\n"):
        out.append("\n")
    out.append(layout_inc)
    if not out[-1].endswith("\n"):
        out.append("\n")
    out.append("// SP7 iter-6d-f: OR-merge helpers per return type.\n")
    out.append(emit_merge_helpers(struct_defs, types_used))
    out.append("\n")
    # Emit chunk fns + types fns in closure (rewritten to call _combined).
    for name, _ in steps_fns:
        if name in closure:
            out.append(rewritten_steps[name])
            if not out[-1].endswith("\n"):
                out.append("\n")
    # Emit combined helpers for bases that are actually in the closure
    # (i.e., reachable from the entry via the rewritten call graph).
    reachable_bases = sorted(b for b in base_calls if f"{b}_combined" in closure)
    for base in reachable_bases:
        sig = base_sigs.get(base)
        if sig is None:
            continue
        out.append(emit_combined(base, chunked_max[base], sig, struct_defs))
        out.append("\n")
    module = "".join(out)
    with open(out_path, "w") as f:
        f.write(module)
    print(f"Wrote {out_path}: {len(module)} bytes, base_calls={len(base_calls)}, types_used={len(types_used)}")


if __name__ == "__main__":
    main()
