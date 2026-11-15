#!/usr/bin/env python3
"""analyse_sens.py <table.tsv> : les deux sens comparés par identifiant et par contenu (bouts + propriétés)."""
import sys, collections
avant, arriere = [], []
for l in open(sys.argv[1]):
    if l.startswith('#'): continue
    c = l.rstrip('\n').split('\t')
    (avant if c[0] == 'avant' else arriere).append((c[1], tuple(c[2:])))
def par_id(rs):
    d = collections.defaultdict(list)
    for i, x in rs: d[i].append(x)
    return d
A, R = par_id(avant), par_id(arriere)
accord = [i for i in A if len(A[i]) == 1 and len(R.get(i, [])) == 1 and A[i][0] == R[i][0]]
ca, cr = collections.Counter(x for _, x in avant), collections.Counter(x for _, x in arriere)
print(f"lectures : avant {len(avant)}, arrière {len(arriere)} ; identifiants en accord {len(accord)}")
print(f"contenus distincts : avant {len(ca)}, arrière {len(cr)} ; communs {len(set(ca) & set(cr))}")
seul_a = ca - cr; seul_r = cr - ca
print(f"contenus en trop au sens avant {sum(seul_a.values())}, au sens arrière {sum(seul_r.values())}")
for x, n in list(seul_a.items())[:8]: print("  avant seul ×%d" % n, x[:2], x[2:])
for x, n in list(seul_r.items())[:8]: print("  arrière seul ×%d" % n, x[:2], x[2:])
vides = [x for _, x in avant + arriere if x[0] in ('', '\\N') or x[1] in ('', '\\N')]
print(f"lectures à bout sans uuid : {len(vides)}")
dup_contenu = {x: n for x, n in (ca | cr).items() if n > 1}
print(f"contenus présents plus d'une fois dans un sens : {len(dup_contenu)}")
