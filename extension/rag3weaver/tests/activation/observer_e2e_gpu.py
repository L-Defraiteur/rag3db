#!/usr/bin/env python3
"""Observation des seuls fournisseurs de notre portée, avant leur lancement.
Réutilise les captures et la garde du banc ; aucun lancement de modèle ici.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import sys
import time


def importer(nom, path, attendu=None):
    if attendu and hashlib.sha256(path.read_bytes()).hexdigest() != attendu:
        raise RuntimeError('empreinte de la primitive de capture différente')
    spec = importlib.util.spec_from_file_location(nom, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[nom] = module
    spec.loader.exec_module(module)
    return module


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--banc', type=Path, required=True)
    p.add_argument('--sortie', type=Path, required=True)
    p.add_argument('--fin', type=Path, required=True)
    p.add_argument('--exe', required=True)
    a = p.parse_args()
    a.sortie.mkdir(exist_ok=False)
    root = Path(__file__).resolve().parents[2]
    confinement = importer('confinement', root / 'scripts/confiner_validation.py')
    autorite = confinement.autorite(confinement.processus())
    capture = importer('observation_gpu', a.banc / 'observation_gpu.py',
        '85a4d58312cd6629bea9725159208b3496c5ea096635202dc7b37675b8ff590f')
    banc = importer('controle_banc', a.banc / 'mesurer_prive.py',
        'a29173c8ad1696e1e1c70460356c29049249ee9861ec7a688f296769693125ef')
    qualifies = {}
    erreurs = []
    (a.sortie / 'pret').write_text('capture prête avant modèle\n')
    try:
        n = 0
        while not a.fin.exists():
            if time.monotonic() >= autorite['echeance']:
                raise RuntimeError('budget du superviseur expiré')
            table = confinement.processus()
            for pid, info in table.items():
                # Filtrer AVANT exe/fd : les processus étrangers peuvent être
                # non inspectables et ne font pas partie de cette observation.
                if not confinement.sous(info['cgroup'], autorite['cgroup']):
                    continue
                proc = Path('/proc') / str(pid)
                try:
                    if os.readlink(proc / 'exe') != a.exe or str(pid) in qualifies:
                        continue
                    c = capture.capture(pid, int(info['debut']), allow_partial=True)
                    c['argv'] = [v.decode(errors='replace') for v in c['argv']]
                    c['cgroup'] = info['cgroup']
                    (a.sortie / f'capture-{pid}-{n}.json').write_text(json.dumps(c, indent=2))
                    n += 1
                    banc.memory_guard(c)
                    seat = [v for v in c['clients'] if v['pci'] == '0000:07:00.0']
                    if any(any(v['engine_ns'].values()) for v in seat):
                        raise RuntimeError('activité du privé observée sur PCI07')
                    connus = all(v['engine_ns'] and v['vram_bytes'] is not None and v['gtt_bytes'] is not None for v in seat)
                    primaire = [v for v in c['clients'] if v['pci'] == '0000:04:00.0']
                    actif = any(v['vram_bytes'] is not None and v['vram_bytes'] > 0
                        and v['engine_ns'].get('drm-engine-gfx', 0) > 0 for v in primaire)
                    if c['complete'] and connus and actif:
                        qualifies[str(pid)] = dict(starttime=info['debut'], clients=c['clients'],
                            exe_sha256=c['exe_sha256'], preuve=f'capture-{pid}-{n-1}.json')
                except OSError as e:
                    erreurs.append(dict(pid=pid, erreur=str(e), statut='inconnu'))
            time.sleep(.1)
    except Exception as e:
        (a.sortie / 'erreur.json').write_text(json.dumps({'erreur': str(e)}))
        # Demander l'arrêt du seul superviseur dont nous sommes descendants.
        table = confinement.processus()
        if table.get(autorite['pid'], {}).get('debut') == autorite['debut']:
            fd = os.pidfd_open(autorite['pid'])
            try:
                signal.pidfd_send_signal(fd, signal.SIGTERM)
            finally:
                os.close(fd)
        raise
    finally:
        (a.sortie / 'resume.json').write_text(json.dumps(dict(
            processus=qualifies, erreurs=erreurs, attendu=2), indent=2))
    if len(qualifies) != 2:
        raise RuntimeError(f'placement documenté pour {len(qualifies)}/2 privés ; aucune absence remplacée par zéro')
    print('Observation : 2/2 privés, résidence et gfx PCI04, captures conservées')


if __name__ == '__main__':
    main()
