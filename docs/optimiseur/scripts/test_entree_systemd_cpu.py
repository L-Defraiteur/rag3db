#!/usr/bin/env python3
"""Vraie route executer en unités CPU : horloge/ROCr/exec sont des fixtures.

L'opérateur CLI ne possède aucune option de fausse horloge ; les substitutions
restent dans ce fichier de test et aucune commande GPU ni modèle n'est exécuté.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
from unittest.mock import patch
import fenetre_card0 as f

ROOT = Path.home() / '.cache/optimiseur-notes/card0-20261113/preparation/entree-systemd-v3'


def worker(cas):
    r = ROOT / cas
    p = r / 'specs/recette.json'
    data = p.read_bytes()
    recette = json.loads(data)
    unite = f.valider_spec(recette)
    compteur = {'rocminfo_factice': 0, 'exec_factice': 0}
    verifier_original = f.verifier

    def rocr(argv, **_):
        assert argv == ['/opt/rocm/bin/rocminfo']
        compteur['rocminfo_factice'] += 1
        return 'BDFID: 1792\nDevice Type: GPU\n'

    def executer(*_):
        compteur['exec_factice'] += 1

    with patch.object(f, 'RACINE', r), \
         patch.object(f, 'maintenant', return_value=f.dt.datetime(2026, 11, 13, 1, tzinfo=f.PARIS)), \
         patch.object(f, 'verifier', side_effect=lambda date: verifier_original(date, racine=r)), \
         patch.object(f.subprocess, 'check_output', side_effect=rocr), \
         patch.object(f.os, 'execvpe', side_effect=executer), \
         patch.object(sys, 'argv', ['fenetre_card0.py', 'executer', str(p), f.digest(data)]):
        code = f.main()
    info = f.Systemd().info(unite)
    if cas == 'prive':
        assert code == 0 and compteur == {'rocminfo_factice': 1, 'exec_factice': 1}
        assert info['MainPID'] == str(os.getpid()) and info['Slice'] == f.SLICE
    else:
        assert code == 1 and compteur == {'rocminfo_factice': 0, 'exec_factice': 0}
    preuve = {'cas': cas, 'pid_cpu': os.getpid(), 'code': code, 'compteur': compteur,
              'unite': info, 'horloge_injectee_fixture_uniquement': True,
              'gpu_reel': False, 'modele_reel': False, 'verdict': 'VERT'}
    with (r / 'preuve.json').open('x') as out:
        json.dump(preuve, out, indent=2)
    print(json.dumps(preuve))


def parent():
    assert not ROOT.exists(), 'preuve déjà présente : aucun rejeu inchangé'
    for cas in ['hors-unite', 'etrangere', 'prive']:
        r = ROOT / cas
        (r / 'specs').mkdir(parents=True)
        recette = {'nom': 'entree-v3-' + cas, 'modele': 'fixture-cpu-aucun-modele',
                   'dimensions': None, 'consommateurs': ['test-cpu'], 'profil': 'rocm',
                   'duree_max_sec': 120, 'argv': ['/usr/bin/true', '--device', 'ROCm0', '--no-warmup'],
                   'executable_sha256': f.sha_fichier('/usr/bin/true')}
        data = json.dumps(recette).encode()
        (r / 'specs/recette.json').write_bytes(data)
        (r / 'pret.json').write_text(json.dumps({'source_sha256': f.sha_fichier(f.SCRIPT)}))
        cmd = ['/usr/bin/python3', str(Path(__file__).resolve()), '--worker', cas]
        if cas != 'hors-unite':
            cmd = ['systemd-run', '--user', '--wait', '--collect', '--service-type=exec',
                   '--unit=' + f.valider_spec(recette), '--slice=' + f.SLICE,
                   '--description=' + (f.MARQUE + f.digest(data) if cas == 'prive' else 'fixture volontairement étrangère'),
                   '--property=Restart=no', '--property=KillMode=control-group',
                   '--property=RuntimeMaxSec=5', '--property=TimeoutStopSec=1s', *cmd]
        subprocess.run(cmd, check=True, timeout=12)
        assert json.loads((r / 'preuve.json').read_text())['verdict'] == 'VERT'


if __name__ == '__main__':
    worker(sys.argv[2]) if len(sys.argv) == 3 and sys.argv[1] == '--worker' else parent()
