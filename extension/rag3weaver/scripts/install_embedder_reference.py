#!/usr/bin/env python3
"""Install the reference of an embedder beside its weights (20 November 2026).

Usage : install_embedder_reference.py <reference.json of the model> <model.bpk> <tolerance>

The reference holds the texts and the vectors of the original ONNX model (for granite :
tests/fixtures/granite/reference-*.json, by generated/reference_granite.py). This writes
`reference.json` next to `model.bpk`, with the sha256 of those weights and the tolerance :
the service and the in-process embedder check themselves against it at start, and a
reference never covers other weights than the ones it names.
"""
import hashlib
import json
import pathlib
import sys

source, weights, tolerance = sys.argv[1], pathlib.Path(sys.argv[2]), float(sys.argv[3])
reference = json.load(open(source))
sha = hashlib.sha256()
with open(weights, "rb") as f:
    for block in iter(lambda: f.read(1 << 24), b""):
        sha.update(block)
reference["weights_sha256"] = sha.hexdigest()
reference["tolerance"] = tolerance
out = weights.with_name("reference.json")
json.dump(reference, open(out, "w"), ensure_ascii=False)
print(f"{out} : {len(reference.get('phrases', reference.get('texts', [])))} textes, sha256 {sha.hexdigest()[:12]}, tolérance {tolerance}")
