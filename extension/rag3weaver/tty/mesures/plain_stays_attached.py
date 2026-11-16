#!/usr/bin/env python3
"""**`--plain --demande` reste attaché jusqu'à la fin de sa demande** (défaut T1 de la campagne D2–D6, 16 nov. 2026).

Le terminal en texte brut rendait la main en 3 s avec le code 0 quand une demande précédente de la même tâche avait
laissé une ligne `ended` dans la boîte : la relecture repartait du début de la boîte et prenait cette fin pour la
sienne. Ce témoin, sur le faux hôte :
1. pose dans la boîte la fin d'une demande précédente ;
2. lance `rag3weaver-tty --plain --depot faux --tache mesure --demande go --journal …` ;
3. exige qu'il soit encore attaché 8 s plus tard, et que son journal porte les outils du pas (read_file, edit_file) ;
4. pose la fin de cette demande-ci, et exige qu'il rende la main dans les 10 s, avec le code 0.

    plain_stays_attached.py <binaire> <faux_hote.py> <dossier de travail>

Il dit VERT ou ROUGE en dernière ligne. Bibliothèque standard seulement.
"""
import json
import os
import subprocess
import sys
import time
import urllib.request

BINAIRE, FAUX, DOSSIER = sys.argv[1:4]
PORT = 17995
os.makedirs(DOSSIER, exist_ok=True)
DJ, JOURNAL, BOITE = os.path.join(DOSSIER, "daemon.json"), os.path.join(DOSSIER, "plain.journal"), os.path.join(DOSSIER, "faux.jsonl")
for f in (DJ, JOURNAL, BOITE):
    if os.path.exists(f):
        os.remove(f)
rouges = []


def poster(chemin, corps):
    r = urllib.request.Request(f"http://127.0.0.1:{PORT}{chemin}", data=json.dumps(corps).encode(), headers={"Authorization": "Bearer jeton-de-mesure", "Content-Type": "application/json"})
    return json.load(urllib.request.urlopen(r, timeout=5))


faux = subprocess.Popen([sys.executable, FAUX, str(PORT), BOITE, DJ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    for _ in range(50):
        if os.path.exists(DJ) and os.path.getsize(DJ) > 0:
            break
        time.sleep(0.1)
    poster("/faux/ligne", {"from": "host", "kind": "ended", "text": "fait (la demande d'avant)"})
    tty = subprocess.Popen([BINAIRE, "--plain", "--depot", "faux", "--tache", "mesure", "--demande", "go", "--daemon-json", DJ, "--journal", JOURNAL], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    time.sleep(8)
    if tty.poll() is not None:
        rouges.append(f"il a rendu la main avant la fin de sa demande (code {tty.returncode}, au bout de moins de 8 s)")
    journal = open(JOURNAL).read() if os.path.exists(JOURNAL) else ""
    for outil in ("read_file", "edit_file"):
        if f"» {outil}" not in journal:
            rouges.append(f"le journal ne porte pas l'outil {outil}")
    poster("/faux/ligne", {"from": "host", "kind": "ended", "text": "fait (cette demande)"})
    try:
        code = tty.wait(timeout=10)
        if code != 0:
            rouges.append(f"fini, il sort avec le code {code}")
    except subprocess.TimeoutExpired:
        rouges.append("la fin de sa demande posée, il ne rend pas la main en 10 s")
        tty.kill()
finally:
    faux.terminate()
for r in rouges:
    print("ROUGE", r)
print("ROUGE" if rouges else "VERT")
