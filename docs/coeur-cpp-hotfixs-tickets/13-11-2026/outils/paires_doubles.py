#!/usr/bin/env python3
"""paires_doubles.py <dossier> <table>… : les paires (source, cible) qui seraient posées plus d'une fois par
accord + plausibles, et la ligne que le code (le texte du Scope source) désigne quand deux lignes s'opposent."""
import sys, collections
D = sys.argv[1]
for t in sys.argv[2:]:
    acc = [tuple(l.rstrip('\n').split('\t')) for l in open(f"{D}/{t}-accord.tsv")]
    pl = [tuple(l.rstrip('\n').split('\t')) for l in open(f"{D}/{t}-plausibles.tsv")]
    n = collections.Counter((x[0], x[1]) for x in acc + pl)
    n_acc = collections.Counter((x[0], x[1]) for x in acc)
    doubles = {k: v for k, v in n.items() if v > 1}
    deja = [x for x in pl if n_acc[(x[0], x[1])] > 0]
    print(f"{t} : {len(doubles)} paires à plus d'une arête (dont {sum(1 for k in doubles if n_acc[k] > 1)} déjà dans l'accord), {len(deja)} plausibles dont la paire est déjà dans l'accord")
    for k in list(doubles)[:5]:
        print("   ", k, [x[2:] for x in acc + pl if (x[0], x[1]) == k])
