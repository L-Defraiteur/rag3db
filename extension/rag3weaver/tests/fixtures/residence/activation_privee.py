#!/usr/bin/env python3
"""Barrière de banc avant exec d'un fournisseur privé, aucun gestionnaire dupliqué.

Le runtime gère acquisition/verrou/identité. Cette entrée ne fait que vérifier
les artefacts figés et attendre le signal du scénario avant exec, même PID/groupe.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
import time


def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for chunk in iter(lambda: f.read(4 * 1024**2), b''):
            h.update(chunk)
    return h.hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--adresse', required=True)
    p.add_argument('--plan', type=Path, required=True)
    p.add_argument('--autoriser', type=Path, required=True)
    p.add_argument('--journal', type=Path, required=True)
    p.add_argument('--budget-ms', type=int, default=300000)
    args = p.parse_args()
    host, sep, port = args.adresse.rpartition(':')
    if host != '127.0.0.1' or not sep or not port.isdigit() or not 0 < int(port) < 65536:
        p.error('adresse privée locale fixe requise')
    if int(port) in (7878, 7879, 7880, 7882) or args.budget_ms <= 0:
        p.error('port réservé aux services ou budget invalide')
    begin = time.monotonic()
    with args.plan.open() as f:
        plan = json.load(f)
    def event(kind, **details):
        with args.journal.open('a') as f:
            f.write(json.dumps(dict(event=kind, monotonic_ns=time.monotonic_ns(), pid=os.getpid(),
                                    pgrp=os.getpgrp(), session=os.getsid(0),
                                    starttime=int(Path('/proc/self/stat').read_text().rsplit(')', 1)[1].split()[19]), **details)) + '\n')
    try:
        declaration = {key: os.environ.get(key) for key in (
            'RAG3WEAVER_ACTIVATION_PROFIL', 'RAG3WEAVER_ACTIVATION_PERIPHERIQUE', 'RAG3WEAVER_ACTIVATION_IDENTITE')}
        if not all(declaration.values()):
            raise RuntimeError('attestation du runtime manquante')
        if os.environ.get('RAG3WEAVER_EMBED_MODEL') != plan['model']:
            raise RuntimeError('modèle du runtime différent du plan')
        if os.environ.get('RAG3WEAVER_BURN_DEVICE_EMBEDDER') != plan['device']:
            raise RuntimeError('sélection du périphérique différente du plan')
        for name, artifact in plan['artifacts'].items():
            if digest(artifact['path']) != artifact['sha256']:
                raise RuntimeError('artefact modifié : ' + name)
        event('barriere_avant_exec', adresse=args.adresse, declaration=declaration)
        while not args.autoriser.is_file():
            if time.monotonic() - begin >= args.budget_ms / 1000:
                raise TimeoutError('budget avant exec épuisé')
            time.sleep(.01)
        with args.autoriser.open() as f:
            gate = json.load(f)
        if gate.get('ok') is not True or not gate.get('source'):
            raise RuntimeError('signal de scénario invalide')
        if time.monotonic() - begin >= args.budget_ms / 1000:
            raise TimeoutError('budget avant exec épuisé')
        provider = plan['artifacts']['provider']['path']
        argv = [provider, '--adresse', args.adresse, *plan.get('provider_args', [])]
        event('exec_fournisseur', argv=argv, source=gate['source'])
        os.execv(provider, argv)
    except Exception as error:
        event('erreur_avant_exec', cause=repr(error))
        print(str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
