"""Fin du PID identifié et libération du listener sous une seule deadline.

Ticket : docs/optimiseur/tickets/2026-11-12-attendre-la-disparition-du-port-apres-l-arret-du-prive-7882.md
Ce module observe seulement : il n'envoie aucun signal et ne relance rien.
"""
from pathlib import Path
import re
import subprocess
import time


class ArretRefuse(RuntimeError):
    """Identité changée ou occupant du port différent/non identifiable."""


def etat_processus(pid, starttime):
    try:
        champs = (Path('/proc') / str(pid) / 'stat').read_text().rsplit(')', 1)[1].split()
    except FileNotFoundError:
        return 'absent'
    if champs[19] != starttime:
        return 'remplace'
    return 'zombie' if champs[0] == 'Z' else 'actif'


def occupants_port(numero, restant):
    sortie = subprocess.run(
        ['ss', '-H', '-ltnp', f'( sport = :{numero} )'],
        check=True, capture_output=True, text=True, timeout=restant,
    ).stdout
    occupants = set()
    for ligne in sortie.splitlines():
        pids = re.findall(r'\bpid=(\d+)', ligne)
        occupants.update(int(pid) for pid in pids)
        if not pids:
            occupants.add(None)
    return occupants


def attendre_arret(pid, starttime, *, deadline, numero_port=7882,
                   processus=etat_processus, port=occupants_port,
                   horloge=time.monotonic, dormir=time.sleep):
    """Attend l'identité disparue/zombie ET le port libre, sans seconde borne.

    Le wrapper fixe deadline juste avant SIGTERM. Les sondes et le sommeil
    consomment ce même budget ; une erreur de lecture reste une erreur.
    Un autre occupant est refusé immédiatement, jamais arrêté à notre place.
    """
    etat, occupants = 'non observé', set()
    while True:
        restant = deadline - horloge()
        if restant <= 0:
            raise TimeoutError(f'expiration arrêt PID {pid} ({etat}), port {numero_port} : {occupants}')
        etat = processus(pid, starttime)
        if etat == 'remplace':
            raise ArretRefuse(f'identité du PID {pid} changée pendant l’attente')
        if etat not in {'absent', 'zombie', 'actif'}:
            raise ArretRefuse(f'état du PID {pid} inconnu : {etat}')
        restant = deadline - horloge()
        if restant <= 0:
            raise TimeoutError(f'expiration arrêt PID {pid} ({etat}), port {numero_port} non relu')
        occupants = port(numero_port, restant)
        if None in occupants:
            raise ArretRefuse(f'port {numero_port} occupé, propriétaire inconnu')
        autres = occupants - {pid}
        if autres:
            raise ArretRefuse(f'port {numero_port} repris par un autre PID : {sorted(autres)}')
        if etat in {'absent', 'zombie'} and not occupants:
            if horloge() > deadline:
                raise TimeoutError(f'expiration arrêt PID {pid}, port {numero_port} libéré hors délai')
            return
        restant = deadline - horloge()
        if restant <= 0:
            raise TimeoutError(f'expiration arrêt PID {pid} ({etat}), port {numero_port} : {occupants}')
        dormir(min(.5, restant))
