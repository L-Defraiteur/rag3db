#!/usr/bin/env python3
"""The merge driver of the test declaration files (24 Nov. 2026, mémoire by c7: two lots that each add a
`#[path = "…"] mod x;` line at the same place of a tests/*.rs conflicted at every repose, half a dozen times a day).

Git calls it as `merge_test_decls.py %O %A %B %P` (ancestor, ours, theirs, path), for the paths that the pusher's
attributes file sends to it (`merge=test-decls`). It merges as git does; a conflict whose two sides hold only module
declarations (`///` lines, `#[path = "…"]`, `mod x;`, blank lines) is resolved by their union, one block per module,
sorted by module name, a module present on both sides kept once. Any other conflict keeps its markers, and the driver
exits 1: git stops on it as it does today. The replay of the suite judges the merged file afterwards.
"""
import re
import subprocess
import sys

DECL = re.compile(r'^\s*(///.*|#\[path\s*=\s*"[^"]*"\]|(pub(\([a-z]+\))?\s+)?mod\s+\w+\s*;)?\s*$')
MOD = re.compile(r'^\s*(?:pub(?:\([a-z]+\))?\s+)?mod\s+(\w+)\s*;')


def blocks(lines):
    """The declaration blocks of one side: the lines up to and including each `mod x;`."""
    out, cur = [], []
    for line in lines:
        if not line.strip() and not cur:
            continue
        cur.append(line)
        m = MOD.match(line)
        if m:
            out.append((m.group(1), cur))
            cur = []
    if any(l.strip() for l in cur):
        return None  # a doc line or a path attribute with no module after it: not a declaration block
    return out


def union(ours, theirs):
    a, b = blocks(ours), blocks(theirs)
    if a is None or b is None:
        return None
    seen, merged = {}, []
    for name, block in a + b:
        key = "".join(l.strip() for l in block)
        if name in seen:
            if seen[name] != key:
                return None  # one module declared two ways: a real conflict
            continue
        seen[name] = key
        merged.append((name, block))
    merged.sort(key=lambda nb: nb[0])
    return [l for _, block in merged for l in block]


def main(o, a, b, path):
    r = subprocess.run(["git", "merge-file", "-p", "-L", "ours", "-L", "base", "-L", "theirs", a, o, b],
                       capture_output=True, text=True)
    if r.returncode < 0 or r.returncode > 127:
        return 2
    lines = r.stdout.splitlines(keepends=True)
    out, i, left = [], 0, 0
    while i < len(lines):
        if not lines[i].startswith("<<<<<<< "):
            out.append(lines[i]); i += 1; continue
        j = i + 1
        ours, theirs, side = [], [], "ours"
        while j < len(lines) and not lines[j].startswith(">>>>>>> "):
            if lines[j].startswith("=======") and lines[j].rstrip("\n") == "=======":
                side = "theirs"
            elif lines[j].startswith("||||||| "):
                side = "base"
            elif side == "ours":
                ours.append(lines[j])
            elif side == "theirs":
                theirs.append(lines[j])
            j += 1
        hunk = lines[i:j + 1]
        merged = None
        if all(DECL.match(l) for l in ours + theirs):
            merged = union(ours, theirs)
        if merged is None:
            out.extend(hunk); left += 1
        else:
            out.extend(merged)
        i = j + 1
    with open(a, "w", encoding="utf-8") as f:
        f.writelines(out)
    return 1 if left else 0


if __name__ == "__main__":
    sys.exit(main(*sys.argv[1:4], sys.argv[4] if len(sys.argv) > 4 else ""))
