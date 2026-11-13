#!/usr/bin/env python3
"""Faux ssh des témoins de l'activation distante : aucun réseau, aucun systemd.

Ticket docs/embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md.
`faux_ssh.py --journal J [--echec] <options ssh…> <hôte> [commande…]` :
- journalise ses arguments, une ligne JSON par appel, dans J ;
- `--echec` : écrit une erreur sur stderr et sort en 255, comme un ssh refusé ;
- un tunnel (`-N`) : attend qu'on le tue, sans rien relayer (le témoin place le
  fournisseur sur le port local même) ;
- `systemd-run … -- <programme> <args>` : lance le programme ici, détaché, avec
  les `--setenv` donnés, et rend la main en 0.
Les arguments de la commande distante arrivent entre apostrophes, comme les
cite l'activation pour le shell distant : on les retire.
"""
import json
import os
import subprocess
import sys
import time

def main():
    args = sys.argv[1:]
    journal = args[args.index('--journal') + 1]
    del args[args.index('--journal'):args.index('--journal') + 2]
    echec = '--echec' in args
    if echec:
        args.remove('--echec')
    with open(journal, 'a', encoding='utf-8') as j:
        j.write(json.dumps({'args': args}) + '\n')
    if echec:
        print('ssh: connect to host faux: Connection refused', file=sys.stderr)
        return 255
    if '-N' in args:
        while True:
            time.sleep(1)
    distants = [a[1:-1] if len(a) >= 2 and a[0] == a[-1] == "'" else a for a in args]
    if 'systemd-run' not in distants:
        print('faux ssh : commande inattendue', file=sys.stderr)
        return 2
    i = distants.index('systemd-run')
    commande = distants[i:]
    env = dict(os.environ)
    for a in commande:
        if a.startswith('--setenv='):
            cle, _, valeur = a[len('--setenv='):].partition('=')
            env[cle] = valeur
    programme = commande[commande.index('--') + 1:]
    sortie = open(journal + '.fournisseur', 'a', encoding='utf-8')
    subprocess.Popen(programme, env=env, stdout=sortie, stderr=sortie, stdin=subprocess.DEVNULL, start_new_session=True)
    return 0

if __name__ == '__main__':
    sys.exit(main())
