#!/usr/bin/env python3
"""**The parity of an embedding service between batches.**

Ticket `docs/embarquement/tickets/2026-11-16-la-preuve-de-parite-des-services-n-a-pas-de-tolerance-mesuree.md`.
A GPU service does not return the same bits for a text embedded alone and inside a batch of other texts (16
November 2026 : at most 3.77e-5 on the 7878). This script measures that gap on a fixed set of texts, so that the
proof of a service pose can say « under the measured tolerance » instead of a number nobody measured.

It is a **client only** : it calls `GET /sante` and `POST /embed`, nothing else. It never starts, stops or
configures a service.

For each text, the reference is its vector embedded **alone**. Then the whole set is embedded again :
- alone a second time (the determinism of one batch shape : 0 expected) ;
- in batches of 8, 32 and 128, in three orders : by length, by reverse length, and shuffled with a fixed seed.

For each run it reports the maximal absolute gap to the reference, the minimal cosine, and the share of vectors
identical to the bit. The JSON goes to stdout (or `--out`).

Usage :
  python3 scripts/embed_parity.py --address 127.0.0.1:7878 [--texts tests/fixtures/embed_parity/texts.json]
                                  [--batches 8,32,128] [--seed 18] [--out parity.json]
"""
import argparse
import json
import math
import random
import sys
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_TEXTS = HERE.parent / "tests" / "fixtures" / "embed_parity" / "texts.json"


def arguments():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--address", required=True, help="host:port of the service")
    p.add_argument("--texts", type=Path, default=DEFAULT_TEXTS)
    p.add_argument("--batches", default="8,32,128", help="batch sizes, comma separated")
    p.add_argument("--seed", type=int, default=18)
    p.add_argument("--timeout", type=float, default=120.0, help="seconds per call")
    p.add_argument("--out", type=Path)
    a = p.parse_args()
    try:
        a.batches = [int(b) for b in a.batches.split(",") if b.strip()]
    except ValueError:
        p.error(f"--batches : integers expected, got {a.batches!r}")
    if not a.batches or any(b <= 0 for b in a.batches):
        p.error("--batches : positive sizes expected")
    return a


def call(address, path, body=None, timeout=120.0):
    url = f"http://{address}{path}"
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json"} if data else {})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read().decode())


def embed(address, texts, timeout):
    vectors = call(address, "/embed", {"texts": texts}, timeout)["vectors"]
    if len(vectors) != len(texts):
        raise SystemExit(f"{len(vectors)} vectors for {len(texts)} texts from {address}")
    return vectors


def run(address, texts, order, size, timeout):
    """Embeds `texts` in `order`, by batches of `size` ; returns the vectors in the original order."""
    out = [None] * len(texts)
    for start in range(0, len(order), size):
        part = order[start:start + size]
        for i, v in zip(part, embed(address, [texts[i] for i in part], timeout)):
            out[i] = v
    return out


def compare(reference, vectors):
    worst, low_cos, same = 0.0, 1.0, 0
    for r, v in zip(reference, vectors):
        if len(r) != len(v):
            raise SystemExit(f"dimension {len(v)} against {len(r)}")
        worst = max(worst, max(abs(a - b) for a, b in zip(r, v)))
        nr, nv = math.sqrt(sum(a * a for a in r)), math.sqrt(sum(b * b for b in v))
        if r == v:
            cos = 1.0
        else:
            cos = sum(a * b for a, b in zip(r, v)) / (nr * nv) if nr and nv else 0.0
        low_cos = min(low_cos, cos)
        same += r == v
    return {"max_abs": worst, "min_cosine": low_cos, "bit_exact_share": same / len(reference)}


def main():
    a = arguments()
    texts = json.loads(a.texts.read_text(encoding="utf-8"))["texts"]
    identity = call(a.address, "/sante", timeout=a.timeout)
    n = len(texts)
    alone = list(range(n))
    reference = run(a.address, texts, alone, 1, a.timeout)
    rng = random.Random(a.seed)
    shuffled = alone[:]
    rng.shuffle(shuffled)
    orders = {
        "length": sorted(alone, key=lambda i: len(texts[i])),
        "reverse_length": sorted(alone, key=lambda i: -len(texts[i])),
        "shuffled": shuffled,
    }
    runs = [dict(batch=1, order="alone_again", **compare(reference, run(a.address, texts, alone, 1, a.timeout)))]
    for size in a.batches:
        for name, order in orders.items():
            runs.append(dict(batch=size, order=name, **compare(reference, run(a.address, texts, order, size, a.timeout))))
    between = [r for r in runs if r["batch"] > 1]
    report = {
        "address": a.address,
        "model": identity.get("modele"),
        "dim": identity.get("dim"),
        "executable": identity.get("executable"),
        "texts": n,
        "texts_file": str(a.texts),
        "seed": a.seed,
        "same_shape": runs[0],
        "between_batches": {
            "max_abs": max(r["max_abs"] for r in between),
            "min_cosine": min(r["min_cosine"] for r in between),
            "bit_exact_share": min(r["bit_exact_share"] for r in between),
        },
        "runs": runs,
    }
    text = json.dumps(report, indent=1)
    if a.out:
        a.out.write_text(text + "\n")
    print(text)


if __name__ == "__main__":
    main()
