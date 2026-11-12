#!/usr/bin/env python3
"""Portée privée de la seule nuit du 13 novembre 2026 ; aucun modèle implicite.

Ticket docs/optimiseur/tickets/2026-11-12-borner-la-fenetre-nocturne-de-card0.md.
L'horloge injectée appartient aux fixtures Python, jamais aux options du CLI.
"""
import argparse
from contextlib import contextmanager
import datetime as dt
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import re
import subprocess
import sys
import time
from zoneinfo import ZoneInfo

PARIS = ZoneInfo('Europe/Paris')
DEBUT = dt.datetime(2026, 11, 13, 0, 0, tzinfo=PARIS)
COUPURE = dt.datetime(2026, 11, 13, 7, 54, tzinfo=PARIS)
RACINE = Path.home() / '.cache/optimiseur-notes/card0-20261113'
SCRIPT = RACINE / 'fenetre_card0.py'
SLICE = 'cx-card0-20261113.slice'
TIMER = 'cx-card0-20261113-cutoff.timer'
PREFIXE = 'cx-card0-20261113-modele-'
MARQUE = 'rag3db privé card0 20261113:'


class Refus(ValueError):
    pass


def exiger(ok, raison):
    if not ok:
        raise Refus(raison)


def maintenant():
    return dt.datetime.now(dt.timezone.utc)


def budget(date):
    exiger(date.tzinfo is not None, 'horloge sans fuseau refusée')
    exiger(DEBUT <= date < COUPURE, 'hors fenêtre du 13 novembre 00:00–07:54 Paris')
    secondes = math.floor((COUPURE - date).total_seconds())
    exiger(secondes >= 60, 'moins de 60 s avant coupure : nouvelle acquisition refusée')
    return secondes


def carte():
    exiger(Path('/sys/class/drm/card0/device').resolve().name == '0000:07:00.0', 'card0/PCI changé')
    exiger(Path('/sys/class/drm/card2/device').resolve().name == '0000:04:00.0', 'card2/PCI changé')


def verifier(date, racine=RACINE):
    restant = budget(date)
    exiger(not (racine / 'ferme.json').exists(), 'portée définitivement fermée')
    carte()
    pret = racine / 'pret.json'
    exiger(pret.is_file(), 'préparation non qualifiée : entrée fermée')
    exiger(json.loads(pret.read_text()).get('source_sha256') == sha_fichier(SCRIPT),
           'source différente de la préparation qualifiée')
    return restant


def valider_spec(spec):
    exiger(isinstance(spec, dict), 'spec non objet')
    exiger(re.fullmatch('[a-z0-9][a-z0-9-]{0,39}', spec.get('nom', '')), 'nom privé invalide')
    exiger(bool(spec.get('modele')) and bool(spec.get('consommateurs')), 'modèle/consommateurs requis')
    exiger(spec.get('dimensions') is None or isinstance(spec['dimensions'], int) and spec['dimensions'] > 0,
           'dimensions invalides')
    exiger(spec.get('profil') == 'rocm', 'profil non qualifié : seul ROCm explicite est préparé')
    exiger(type(spec.get('duree_max_sec')) is int and spec['duree_max_sec'] > 0,
           'durée maximale explicite positive requise')
    argv = spec.get('argv', [])
    exiger(isinstance(argv, list) and len(argv) >= 4 and all(isinstance(x, str) for x in argv), 'argv invalide')
    exiger(Path(argv[0]).is_absolute(), 'exécutable non absolu')
    exiger(argv.count('--device') == 1 and argv[argv.index('--device') + 1:argv.index('--device') + 2] == ['ROCm0'],
           '--device ROCm0 explicite requis après filtre ROCr')
    exiger('--no-warmup' in argv and '-dev' not in argv and not any(x.startswith('--device=') for x in argv),
           'échauffement préalable ou sélection GPU concurrente refusés')
    exiger(re.fullmatch('[0-9a-f]{64}', spec.get('executable_sha256', '')), 'SHA exécutable requis')
    return PREFIXE + spec['nom'] + '.service'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def sha_fichier(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for b in iter(lambda: f.read(1024 * 1024), b''):
            h.update(b)
    return h.hexdigest()


@contextmanager
def verrou(racine):
    racine.mkdir(parents=True, exist_ok=True)
    with (racine / 'registre.lock').open('a') as f:
        fin = time.monotonic() + 20
        while True:
            try:
                fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                exiger(time.monotonic() < fin, 'registre occupé au-delà de 20 s')
                time.sleep(.05)
        yield


def commande_unite(spec, fichier, empreinte, restant):
    unite = valider_spec(spec)
    exiger(spec['duree_max_sec'] <= restant, 'durée demandée au-delà du restant : acquisition refusée')
    return ['systemd-run', '--user', '--collect', '--service-type=exec', '--unit=' + unite,
            '--slice=' + SLICE, '--description=' + MARQUE + empreinte,
            '--property=Restart=no', '--property=KillMode=control-group',
            '--property=TimeoutStopSec=15s', '--property=RuntimeMaxSec=' + str(spec['duree_max_sec']),
            '--property=MemoryMax=32G',
            '--property=ExecCondition=/usr/bin/python3 ' + str(SCRIPT) + ' condition',
            '/usr/bin/python3', str(SCRIPT), 'executer', str(fichier), empreinte]


class Systemd:
    def info(self, unite):
        fields = ['LoadState', 'ActiveState', 'MainPID', 'Description', 'Slice', 'ControlGroup', 'Restart', 'KillMode']
        p = subprocess.run(['systemctl', '--user', 'show', unite] + ['-p' + x for x in fields],
                           capture_output=True, text=True, timeout=10)
        valeurs = dict(x.split('=', 1) for x in p.stdout.splitlines() if '=' in x)
        if valeurs.get('LoadState') == 'not-found':
            return valeurs
        p.check_returncode()
        return valeurs

    def arreter(self, unites):
        if unites:
            subprocess.run(['systemctl', '--user', 'stop', *unites], check=True, timeout=30)

    def pids(self, info):
        groupe = info.get('ControlGroup', '')
        if not groupe:
            exiger(info.get('MainPID', '0') == '0', 'cgroup du processus actif inconnu')
            return []
        base = Path('/sys/fs/cgroup')
        cible = (base / groupe.lstrip('/')).resolve()
        exiger(cible.is_relative_to(base), 'cgroup hors racine autorisée')
        resultat = set()
        for fichier in cible.rglob('cgroup.procs'):
            try:
                resultat.update(int(x) for x in fichier.read_text().split())
            except FileNotFoundError:
                pass
        return sorted(resultat)


def propriete_privee(info, empreinte):
    exiger(info.get('Description') == MARQUE + empreinte and info.get('Slice') == SLICE,
           'unité étrangère ou métadonnées modifiées : aucun arrêt')
    exiger(info.get('Restart') == 'no' and info.get('KillMode') == 'control-group',
           'politique privée de groupe/redémarrage modifiée')


def couper(date, racine=RACINE, manager=None):
    exiger(date >= COUPURE, 'coupure anticipée refusée')
    manager = manager or Systemd()
    with verrou(racine):
        ferme = racine / 'ferme.json'
        if not ferme.exists():
            with ferme.open('x') as f:
                json.dump({'date': date.isoformat(), 'reouverture': False}, f)
        unites, avant, pids_avant = [], {}, {}
        for p in sorted((racine / 'specs').glob('*.json')):
            data = p.read_bytes()
            nom = valider_spec(json.loads(data))
            info = manager.info(nom)
            avant[nom] = info
            if info.get('LoadState') == 'not-found':
                continue
            propriete_privee(info, digest(data))
            pids_avant[nom] = manager.pids(info)
            unites.append(nom)
        manager.arreter(unites)
        apres = {u: manager.info(u) for u in unites}
        for u, info in apres.items():
            exiger(info.get('LoadState') == 'not-found' or
                   info.get('ActiveState') in {'inactive', 'failed'} and info.get('MainPID') == '0',
                   'processus privé encore présent : ' + u)
            exiger(not manager.pids(info), 'descendants privés encore présents : ' + u)
        preuve = {'date': maintenant().isoformat(), 'couper_au_plus_tard': COUPURE.isoformat(),
                  'avant': avant, 'apres': apres, 'pids_groupes_avant': pids_avant,
                  'unites_privees_arretees': unites, 'descendants_restants': [],
                  'aucun_redemarrage': True, 'verdict': 'VERT'}
        with (racine / 'restitution.json').open('w') as f:
            json.dump(preuve, f, indent=2)
        return preuve


def placement_rocr(texte):
    bdf = re.findall(r'^\s*BDFID:\s*(\d+)\s*$', texte, re.MULTILINE)
    exiger(bdf == ['1792'], 'agent ROCr autre que PCI07:00.0 ou ambigu')
    exiger(len(re.findall(r'Device Type:\s*GPU', texte)) == 1, 'agent GPU ROCr non unique')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['condition', 'lancer', 'executer', 'couper'])
    parser.add_argument('fichier', nargs='?', type=Path)
    parser.add_argument('empreinte', nargs='?')
    a = parser.parse_args()
    try:
        if a.mode == 'couper':
            print(json.dumps(couper(maintenant()), ensure_ascii=False))
            return 0
        restant = verifier(maintenant())  # Avant lecture de recette ou commande GPU.
        exiger(a.mode == 'condition' or a.fichier is not None, 'recette requise')
        if a.mode == 'condition':
            return 0
        data = a.fichier.read_bytes()
        spec = json.loads(data)
        unite = valider_spec(spec)
        empreinte = digest(data)
        if a.mode == 'lancer':
            with verrou(RACINE):
                restant = verifier(maintenant())
                exiger(Systemd().info(TIMER).get('ActiveState') == 'active', 'cutoff.timer non actif')
                exiger(Systemd().info(unite).get('LoadState') == 'not-found', 'nom déjà occupé : aucun remplacement')
                exiger(sha_fichier(spec['argv'][0]) == spec['executable_sha256'], 'exécutable modifié')
                dossier = RACINE / 'specs'
                dossier.mkdir(exist_ok=True)
                copie = dossier / (spec['nom'] + '.json')
                with copie.open('xb') as f:
                    f.write(data)
                subprocess.run(commande_unite(spec, copie, empreinte, restant), check=True, timeout=20)
                print(json.dumps({'unite': unite, 'date': maintenant().isoformat(), 'spec_sha256': empreinte,
                                  'phase': 'démarrage demandé, placement poids/calcul à qualifier par le consommateur'}))
            return 0
        exiger(a.fichier.resolve().parent == RACINE / 'specs' and empreinte == a.empreinte,
               'recette non enregistrée ou modifiée')
        exiger(sha_fichier(spec['argv'][0]) == spec['executable_sha256'], 'exécutable modifié')
        env = dict(os.environ)
        for cle in ['ROCR_VISIBLE_DEVICES', 'HIP_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES',
                    'GPU_DEVICE_ORDINAL', 'GGML_VK_VISIBLE_DEVICES']:
            env.pop(cle, None)
        env['ROCR_VISIBLE_DEVICES'] = '1'
        rocminfo = subprocess.check_output(['/opt/rocm/bin/rocminfo'], env=env, text=True, timeout=10)
        placement_rocr(rocminfo)
        # RuntimeMaxSec est fixé à la création ; systemd refuse sa modification
        # par set-property. Le cutoff absolu ferme aussi les départs différés.
        restant = verifier(maintenant())
        exiger(spec['duree_max_sec'] <= restant, 'durée après attente trop longue : aucun modèle lancé')
        pid = os.getpid()
        stat = (Path('/proc') / str(pid) / 'stat').read_text().rsplit(')', 1)[1].split()
        with (RACINE / (spec['nom'] + '-entree.json')).open('x') as f:
            json.dump({'date': maintenant().isoformat(), 'pid': pid, 'starttime': stat[19],
                       'unite': unite, 'spec': spec, 'spec_sha256': empreinte, 'pci_rocr': '0000:07:00.0',
                       'poids_et_calcul': 'à qualifier sur le modèle par le consommateur avant mesure'}, f, indent=2)
        os.execvpe(spec['argv'][0], spec['argv'], env)
    except (Refus, OSError, ValueError, KeyError, subprocess.SubprocessError) as e:
        print('ROUGE fenêtre card0 : ' + str(e), file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
