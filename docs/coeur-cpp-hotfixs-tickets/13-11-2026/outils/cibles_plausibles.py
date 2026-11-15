#!/usr/bin/env python3
"""cibles_plausibles.py <dossier> <requete> <base> <table>… : pour chaque lecture en désaccord, la cible est-elle
plausible ? Le nom de la destination (Symbol.name, ou le dernier segment du nom d'un Scope) doit paraître dans le
texte du Scope source, et la ligne (s'il y en a une) dans ses bornes. Écrit <table>-desaccord-juge.tsv."""
import sys, collections, subprocess, json
D, Q, B = sys.argv[1:4]
def requete(q):
    return subprocess.run([Q, B, q], capture_output=True, text=True).stdout.splitlines()
for t in sys.argv[4:]:
    avant, arriere, props, bouts = [], [], None, None
    for l in open(f"{D}/{t}.tsv"):
        c = l.rstrip('\n').split('\t')
        if c[0] == '# props': props = c[1:]; continue
        if c[0] == '# bouts': bouts = c[1:]; continue
        (avant if c[0] == 'avant' else arriere).append(tuple(c[2:]))
    cible = bouts[-1]
    ca, cr = collections.Counter(avant), collections.Counter(arriere)
    acc = ca & cr
    lect = [("avant", x) for x in (ca - acc).elements()] + [("arriere", x) for x in (cr - acc).elements()]
    srcs = sorted({x[0] for _, x in lect}); dsts = sorted({x[1] for _, x in lect})
    S = {}
    for s in srcs:  # une requête par source : le texte peut être long et porter des |
        r = requete(f"MATCH (s:Scope {{_uuid: '{s}'}}) RETURN offset(id(s)), s.start_line, s.end_line, s.content")
        body = [l for l in r if not l.startswith('>')]
        if body:
            a, b_, c_, *txt = '\n'.join(body).split('|', 3) + ['']
            S[s] = (int(a), int(b_), int(c_), '|'.join(txt))
    N = {}; Z = {}
    for d in dsts:
        r = [l for l in requete(f"MATCH (n:{cible} {{_uuid: '{d}'}}) RETURN offset(id(n)), n.name") if not l.startswith('>') and l]
        if r:
            o, _, nm = r[0].partition('|'); N[d] = nm; Z[d] = int(o) == 0
    il = 2 + props.index('line') if 'line' in props else None
    comptes = collections.Counter()
    with open(f"{D}/{t}-desaccord-juge.tsv", "w") as out:
        for sens, x in lect:
            s = S.get(x[0]); nom = N.get(x[1], '')
            court = nom.split('::')[-1].split('.')[-1].split(':')[0]
            zero = (s is not None and s[0] == 0) or Z.get(x[1], False)
            nom_ok = bool(s) and bool(court) and court.lower() in s[3].lower()
            ligne_ok = il is None or (s is not None and x[il] != '\\N' and s[1] <= int(x[il]) <= s[2])
            juge = 'plausible' if (nom_ok and ligne_ok and not zero) else 'faux'
            comptes[(sens, juge)] += 1
            out.write('\t'.join([sens, juge, 'zero' if zero else '', court] + list(x)) + '\n')
    print(t, dict(comptes))
    # Les lectures plausibles, dédoublonnées par contenu, et les conflits : une même source et mêmes propriétés
    # hors cible, deux cibles plausibles.
    garde = {}
    for l in open(f"{D}/{t}-desaccord-juge.tsv"):
        c = l.rstrip('\n').split('\t')
        if c[1] == 'plausible': garde[tuple(c[4:])] = c[0]
    cle = collections.defaultdict(set)
    for x in garde: cle[(x[0],) + x[2:]].add(x[1])
    conflits = {k: v for k, v in cle.items() if len(v) > 1}
    with open(f"{D}/{t}-plausibles.tsv", "w") as f:
        for x in sorted(garde): f.write('\t'.join(x) + '\n')
    print(f"  {len(garde)} contenus plausibles à reposer (avant {sum(1 for v in garde.values() if v=='avant')}, arrière {sum(1 for v in garde.values() if v=='arriere')}), {len(conflits)} conflits")
