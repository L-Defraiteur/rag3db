#!/usr/bin/env python3
"""disputes.py <dossier> <table>… : écrit <table>-accord.tsv (les contenus lus pareils dans les deux sens, une fois
chacun, à identifiant en accord ou non) et la liste des Scope sources des contenus en désaccord (hors le Scope
d'offset 0 lu par une case à zéro) dans sources-en-desaccord.txt."""
import sys, collections, os
D = sys.argv[1]; sources = set(); zero = None
for t in sys.argv[2:]:
    avant, arriere, props = [], [], None
    for l in open(f"{D}/{t}.tsv"):
        c = l.rstrip('\n').split('\t')
        if c[0] == '# props': props = c[1:]; continue
        if c[0].startswith('#'): continue
        (avant if c[0] == 'avant' else arriere).append(tuple(c[2:]))
    ca, cr = collections.Counter(avant), collections.Counter(arriere)
    accord = ca & cr
    with open(f"{D}/{t}-accord.tsv", "w") as f:
        for x, n in sorted(accord.items()):
            for _ in range(n): f.write('\t'.join(x) + '\n')
    desaccord = list((ca - accord).elements()) + list((cr - accord).elements())
    srcs = {x[0] for x in desaccord}
    print(f"{t} : {sum(accord.values())} contenus en accord reposés, {len(desaccord)} lectures en désaccord, {len(srcs)} sources")
    sources |= srcs
open(f"{D}/sources-en-desaccord.txt", "w").write('\n'.join(sorted(sources)) + '\n')
print(f"{len(sources)} Scope sources en désaccord (toutes tables)")
