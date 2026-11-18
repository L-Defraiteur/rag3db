#!/usr/bin/env python3
"""Vector parity between two bases, by tolerance (bench, 16 Nov. 2026; docs/banc/16-11-2026/01-la-parite-de-la-migration.md, § 4).

The same text embedded in batches of a different shape gives different last bits (embarquement's probe on the 7878:
alone against in a batch of longer texts, 3.77e-5). A vector parity between two bases embedded at different times is
therefore judged by tolerance, not bit for bit.

Inputs: two relevés written by `structure-sonde` with a Cypher that returns, per chunk, its text hash first and its
vector (`requetes[0].lignes`, rows like `[String("<hash>"), …, List([Float(…), …])]`), e.g.
`MATCH (c:ScopeContent_Chunk) WHERE c._text_hash STARTS WITH '00' RETURN c._text_hash, c._title, c.embedding__granite_278m`.
Chunks are joined by text hash. Green when every common chunk has a cosine of at least --min-cosine (0.99999) and an
absolute difference of at most --max-abs (1e-4), and at least one chunk is common. The last line is VERT or ROUGE.

Usage: vector_parity.py A.json B.json [--min-cosine 0.99999] [--max-abs 1e-4]
"""
import argparse
import json
import math
import re
import sys

HASH = re.compile(r'String\("([0-9a-fA-F]+)"\)')
FLOAT = re.compile(r"Float\(([-0-9.eE+]+)\)")


def load(path):
    out = {}
    for row in json.load(open(path))["requetes"][0]["lignes"]:
        text = str(row)
        found = HASH.search(text)
        vector = [float(x) for x in FLOAT.findall(text)]
        if found and vector:
            out.setdefault(found.group(1), vector)
    return out


def main():
    p = argparse.ArgumentParser()
    p.add_argument("a")
    p.add_argument("b")
    p.add_argument("--min-cosine", type=float, default=0.99999)
    p.add_argument("--max-abs", type=float, default=1e-4)
    o = p.parse_args()
    A, B = load(o.a), load(o.b)
    common = sorted(set(A) & set(B))
    identical, worst_abs, worst_cos, bad = 0, 0.0, 1.0, []
    for h in common:
        a, b = A[h], B[h]
        if len(a) != len(b):
            bad.append(f"{h}: dimension {len(a)} against {len(b)}")
            continue
        if a == b:
            identical += 1
        diff = max(abs(x - y) for x, y in zip(a, b))
        na, nb = math.sqrt(sum(x * x for x in a)), math.sqrt(sum(y * y for y in b))
        cos = sum(x * y for x, y in zip(a, b)) / (na * nb) if na and nb else 0.0
        worst_abs, worst_cos = max(worst_abs, diff), min(worst_cos, cos)
        if cos < o.min_cosine or diff > o.max_abs:
            bad.append(f"{h}: cosine {cos:.9f}, max abs {diff:.3g}")
    print(f"A {len(A)} chunks, B {len(B)} ; common {len(common)} ; bit-identical {identical} ; min cosine {worst_cos:.9f} ; max abs {worst_abs:.3g}")
    for line in bad[:10]:
        print(f"  out of tolerance: {line}")
    if not common:
        print("no common chunk: nothing was compared")
    green = bool(common) and not bad
    print("VERT" if green else "ROUGE")
    return 0 if green else 1


if __name__ == "__main__":
    sys.exit(main())
