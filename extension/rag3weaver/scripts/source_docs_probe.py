#!/usr/bin/env python3
"""**Doc comments written above the wrong item** — a probe, run by hand, not a guard.

A `///` documents the item that follows it (rustdoc, codeparsers and the search all agree). When an item is inserted
between a section's doc and its type, it takes that doc, and the type is left without one (22 November 2026: the doc
of `FuseResultsNode` was served as the doc of `DuplicatePolicy`). This probe lists candidates; about half are false
positives (a doc that merely cites another type), so each one is read by hand before anything moves.

Signal A: the section separator `// ─── X ───` above the block names X, X has no doc, and the block does not name the
item it documents. Signal B: the block names a type declared without a doc in the next 40 lines, and does not name
the item it documents.

Usage: python3 -I scripts/source_docs_probe.py <crate root>   (one tab-separated line per candidate)
"""
import re, sys, pathlib
ITEM = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+|const\s+|unsafe\s+|extern\s+"[^"]*"\s+)*(struct|enum|fn|trait|type|union|mod|const|static|impl)\b\s*(?:<[^>]*>\s*)?([A-Za-z_][A-Za-z0-9_]*)?')
SEP = re.compile(r'^\s*//\s*[─-]{3,}\s*([A-Za-z_][A-Za-z0-9_]*)\s*[─-]{3,}')
root = pathlib.Path(sys.argv[1])
for f in sorted(root.rglob('*.rs')):
    if 'target' in f.parts: continue
    L = f.read_text(errors='replace').split('\n')
    i = 0; blocks = []
    while i < len(L):
        if L[i].strip().startswith('///'):
            d = i
            while i < len(L) and L[i].strip().startswith('///'): i += 1
            j = i
            while j < len(L) and (L[j].strip().startswith('#[') or L[j].strip().startswith('//') and not L[j].strip().startswith('///') or L[j].strip()==''):
                if L[j].strip()=='' : break
                j += 1
            m = ITEM.match(L[j]) if j < len(L) else None
            blocks.append((d, i, j, m.group(2) if m else None, m.group(1) if m else None))
            continue
        i += 1
    documented = {b[3] for b in blocks if b[3]}
    declared = {}; kinds = {}
    for n, l in enumerate(L):
        m = ITEM.match(l)
        if m and m.group(2) and m.group(1) in ('struct','enum','trait','type','union','fn'):
            declared.setdefault(m.group(2), n); kinds.setdefault(m.group(2), m.group(1))
    for d, end, j, name, kind in blocks:
        if not name: continue
        text = ' '.join(L[d:end])
        # A: the section separator names another item
        k = d - 1
        while k >= 0 and L[k].strip()=='' : k -= 1
        s = SEP.match(L[k]) if k >= 0 else None
        if s and s.group(1) != name and s.group(1) in declared and s.group(1) not in documented and not re.search(r'\b'+re.escape(name)+r'\b', text):
            print(f"A\t{f.relative_to(root)}:{d+1}\tdoc on {kind} {name}\tseparator « {s.group(1)} » (declared l.{declared[s.group(1)]+1}{', without doc' if s.group(1) not in documented else ''})")
            continue
        # B: the block names a nearby type without a doc
        if re.search(r'\b'+re.escape(name)+r'\b', text): continue
        for other, n in declared.items():
            if other == name or other in documented or kinds[other] not in ('struct','enum','trait','type','union') or not (j < n <= j + 40): continue
            if re.search(r'`?\b'+re.escape(other)+r'\b`?', text):
                print(f"B\t{f.relative_to(root)}:{d+1}\tdoc on {kind} {name}\tnames {other} (l.{n+1}, without doc)")
