#!/usr/bin/env python3
"""comparer_jeu.py <avant.jsonl> <après.jsonl> : les réponses du jeu, une par ligne, identiques hors
`/result/run` (l'identifiant du passage, seul champ qui varie entre deux passages sur la même base).
Imprime VERT ou ROUGE et, pour chaque réponse qui diffère, les chemins qui changent."""
import json, sys
def plat(o, p=""):
    if isinstance(o, dict):
        for k, v in o.items(): yield from plat(v, f"{p}/{k}")
    elif isinstance(o, list):
        for i, v in enumerate(o): yield from plat(v, f"{p}[{i}]")
    else: yield p, o
a = [dict(plat(json.loads(l))) for l in open(sys.argv[1])]
b = [dict(plat(json.loads(l))) for l in open(sys.argv[2])]
ecarts = 0
if len(a) != len(b):
    print(f"{len(a)} réponses avant, {len(b)} après"); ecarts += 1
for i, (x, y) in enumerate(zip(a, b)):
    d = sorted(k for k in set(x) | set(y) if k != "/result/run" and x.get(k) != y.get(k))
    ok = x.get("/ok") is True and y.get("/ok") is True
    print(f"réponse {i + 1} : {'identique' if not d else f'{len(d)} écarts {d[:4]}'}{'' if ok else ' — une réponse en échec'}")
    ecarts += bool(d) + (not ok)
print("VERT" if ecarts == 0 else "ROUGE")
