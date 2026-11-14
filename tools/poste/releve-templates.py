#!/usr/bin/env python3
"""Relevé au fil de l'eau d'un dossier (14 novembre 2026, hotfixs, ticket d'arbre principal
« un fichier de plus apparaît sous templates/ pendant la lib entière ») : chaque nom créé, renommé ou
effacé sous la racine, écrit et vidé sur disque dès l'événement, avec l'heure à la microseconde. Il
suit aussi les sous-dossiers qui apparaissent en route. inotify par ctypes, sans dépendance :
inotifywait n'est pas installé au portable.

Usage : releve-templates.py <racine> <journal>   (s'arrête sur SIGTERM ou SIGINT)

Le journal a une ligne par événement : « HH:MM:SS.µs QUOI chemin », où QUOI vaut CRÉÉ, ÉCRIT, EFFACÉ,
PARTI, ARRIVÉ, VU-DÉJÀ-LÀ (déjà dans un dossier neuf quand sa surveillance a été posée),
DOSSIER-EFFACÉ, SURVEILLANCE-LEVÉE ou DÉBORDEMENT (la file du noyau a perdu des événements). Mode
d'emploi : docs/coeur-cpp-hotfixs-tickets/14-11-2026/01-releve-de-templates.md.
"""
import ctypes
import ctypes.util
import os
import signal
import struct
import sys
import time

IN_MODIFY, IN_CLOSE_WRITE, IN_MOVED_FROM, IN_MOVED_TO = 0x2, 0x8, 0x40, 0x80
IN_CREATE, IN_DELETE, IN_DELETE_SELF, IN_ISDIR, IN_Q_OVERFLOW, IN_IGNORED = 0x100, 0x200, 0x400, 0x40000000, 0x4000, 0x8000
MASQUE = IN_CREATE | IN_DELETE | IN_MOVED_FROM | IN_MOVED_TO | IN_CLOSE_WRITE | IN_DELETE_SELF
NOMS = {IN_CREATE: "CRÉÉ", IN_DELETE: "EFFACÉ", IN_MOVED_FROM: "PARTI", IN_MOVED_TO: "ARRIVÉ",
        IN_CLOSE_WRITE: "ÉCRIT", IN_DELETE_SELF: "DOSSIER-EFFACÉ", IN_Q_OVERFLOW: "DÉBORDEMENT", IN_IGNORED: "SURVEILLANCE-LEVÉE"}

libc = ctypes.CDLL(ctypes.util.find_library("c"), use_errno=True)
racine, journal = os.path.abspath(sys.argv[1]), sys.argv[2]
fd = libc.inotify_init1(0)
if fd < 0:
    sys.exit(f"inotify_init1 : errno {ctypes.get_errno()}")
dossiers = {}
out = open(journal, "a", buffering=1, encoding="utf-8")


def ecrire(texte):
    out.write(f"{time.strftime('%H:%M:%S')}.{int(time.time() * 1e6) % 1000000:06d} {texte}\n")
    out.flush()
    os.fsync(out.fileno())


def surveiller(d):
    wd = libc.inotify_add_watch(fd, d.encode(), MASQUE)
    if wd < 0:
        ecrire(f"ERREUR surveiller {d} : errno {ctypes.get_errno()}")
        return
    dossiers[wd] = d


for d, sous, _ in os.walk(racine):
    surveiller(d)
ecrire(f"DÉBUT {racine} : {len(dossiers)} dossiers surveillés, {sum(len(f) for _, _, f in os.walk(racine))} fichiers")
signal.signal(signal.SIGTERM, lambda *_: (ecrire("FIN"), os._exit(0)))
signal.signal(signal.SIGINT, lambda *_: (ecrire("FIN"), os._exit(0)))
while True:
    tampon = os.read(fd, 1 << 16)
    i = 0
    while i < len(tampon):
        wd, masque, _, longueur = struct.unpack_from("iIII", tampon, i)
        nom = tampon[i + 16:i + 16 + longueur].rstrip(b"\0").decode(errors="replace")
        i += 16 + longueur
        chemin = os.path.join(dossiers.get(wd, "?"), nom)
        quoi = next((v for k, v in NOMS.items() if masque & k), hex(masque))
        if masque & IN_Q_OVERFLOW:
            ecrire("DÉBORDEMENT de la file inotify : des événements sont perdus")
            continue
        ecrire(f"{quoi}{' dossier' if masque & IN_ISDIR else ''} {os.path.relpath(chemin, racine)}")
        if masque & IN_ISDIR and masque & (IN_CREATE | IN_MOVED_TO):
            # Ce qui s'est écrit dans le dossier neuf avant que sa surveillance ne soit posée.
            for d, _, fichiers in os.walk(chemin):
                surveiller(d)
                for f in fichiers:
                    ecrire(f"VU-DÉJÀ-LÀ {os.path.relpath(os.path.join(d, f), racine)}")
