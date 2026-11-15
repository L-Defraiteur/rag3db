#!/usr/bin/env python3
"""final.py <dossier> <table>… : <table>-final.tsv = les contenus en accord, plus les lectures en désaccord jugées
plausibles (bouts hors offset 0, ligne dans le Scope source, nom de la cible dans son texte) dont la paire
(source, cible) n'est pas déjà posée ; une arête par paire. Puis le compte de ce qui est écarté."""
import sys, collections
D = sys.argv[1]
for t in sys.argv[2:]:
    acc = [tuple(l.rstrip('\n').split('\t')) for l in open(f"{D}/{t}-accord.tsv")]
    pl = [tuple(l.rstrip('\n').split('\t')) for l in open(f"{D}/{t}-plausibles.tsv")]
    juge = [l.rstrip('\n').split('\t') for l in open(f"{D}/{t}-desaccord-juge.tsv")]
    vus, final = set(), []
    for x in acc + pl:
        if (x[0], x[1]) in vus: continue
        vus.add((x[0], x[1])); final.append(x)
    with open(f"{D}/{t}-final.tsv", "w") as f:
        for x in final: f.write('\t'.join(x) + '\n')
    faux = sum(1 for c in juge if c[1] == 'faux'); zero = sum(1 for c in juge if c[2] == 'zero')
    print(f"{t} : {len(final)} arêtes ({len(acc)} en accord + {len(final) - len(acc)} plausibles) ; écartées {faux} lectures fausses, dont {zero} par une case à zéro")
