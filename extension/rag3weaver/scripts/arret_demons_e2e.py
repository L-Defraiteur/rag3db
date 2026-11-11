#!/usr/bin/env python3
"""Arrêt borné des seuls démons marqués de cette passe, jamais SIGKILL.

Ticket recherche/tickets/2026-11-11-le-service-d-embarquement-laisse-son-journal-dans-le-tmpdir-d-e2e-search.md.
L'identité /proc (pid + naissance) est vérifiée avant signal et pendant l'attente.
Un échec interdit au lanceur de supprimer les ressources encore utilisées.
"""
import os
from pathlib import Path
import signal
import sys
import time


def identite(pid):
    try:
        champs = Path(f'/proc/{pid}/stat').read_text().rsplit(') ', 1)[1].split()
        return champs[19], champs[0]  # starttime, état ; comm peut contenir des espaces.
    except (FileNotFoundError, ProcessLookupError):
        return None


def proprietaire(pid):
    proc = Path(f'/proc/{pid}')
    try:
        if proc.stat().st_uid != os.getuid():
            return None
        avant = identite(pid)
        if avant is None or avant[1] == 'Z':
            return None
        environnement = (proc/'environ').read_bytes().split(b'\0')
        apres = identite(pid)
        if apres is None or apres[1] == 'Z':
            return None
        if avant[0] != apres[0]:
            raise OSError('identité changée pendant inspection')
        cle = b'RAG3WEAVER_PASSE_E2E='
        marque = next((v[len(cle):] for v in environnement if v.startswith(cle)), b'')
        return apres[0], marque
    except (FileNotFoundError, ProcessLookupError):
        return None


def quiescent(prefixe, pids):
    # Le PID de passe figure déjà dans la marque héritée. Un PID recyclé
    # peut conserver trop longtemps, jamais autoriser un effacement précoce.
    if not prefixe.endswith('-') or not prefixe[:-1].isdigit():
        return False
    for pid in set(pids):
        try:
            possession = proprietaire(pid)
            if possession and possession[1].startswith(prefixe.encode()):
                print(f'▸ ressources conservées : processus de la passe encore vivant (pid {pid})')
                return False
        except OSError as exc:
            print(f'▸ quiescence non vérifiable : {exc} ; ressources conservées', file=sys.stderr)
            return False
    return True


def arreter(marque, pids, delai=10):
    if not marque:
        raise ValueError('marque de passe absente')
    retenus = {}
    bon = True
    for pid in set(pids):
        if pid <= 1 or pid == os.getpid():
            continue
        try:
            fd = os.pidfd_open(pid)
            try:
                possession = proprietaire(pid)
                if possession is None or possession[1] != marque.encode():
                    continue
                maintenant = identite(pid)
                if maintenant is None or maintenant[0] != possession[0] or maintenant[1] == 'Z':
                    continue
                # Le descripteur vise ce processus, même si son numéro est recyclé.
                signal.pidfd_send_signal(fd, signal.SIGTERM)
                retenus[pid] = possession[0]
                print(f'▸ arrêt demandé au démon de la passe (pid {pid}, SIGTERM)', flush=True)
            finally:
                os.close(fd)
        except (FileNotFoundError, ProcessLookupError):
            continue
        except OSError as exc:
            print(f'✗ identité ou arrêt du démon {pid} non vérifiable : {exc}', file=sys.stderr)
            bon = False
    fin = time.monotonic()+delai
    while retenus:
        for pid, naissance in list(retenus.items()):
            try:
                maintenant = identite(pid)
            except OSError as exc:
                print(f'✗ attente du démon {pid} non vérifiable : {exc}', file=sys.stderr)
                bon = False
                del retenus[pid]
                continue
            if maintenant is None or maintenant[0] != naissance or maintenant[1] == 'Z':
                print(f'▸ démon de la passe terminé (pid {pid})', flush=True)
                del retenus[pid]
        if not retenus:
            break
        if time.monotonic() >= fin:
            print(f'✗ arrêt non terminé après {delai:g}s : {sorted(retenus)} ; ressources conservées', file=sys.stderr)
            return False
        time.sleep(.02)
    return bon


if __name__ == '__main__':
    if sys.argv[1] == '--quiescence':
        sys.exit(0 if quiescent(sys.argv[2], map(int, sys.argv[3:])) else 1)
    sys.exit(0 if arreter(sys.argv[1], map(int, sys.argv[2:])) else 1)
