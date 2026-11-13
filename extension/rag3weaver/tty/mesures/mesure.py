#!/usr/bin/env python3
"""La mesure du terminal à travers ssh (lot tty-5).

Lancé sur le poste où l'on regarde, il démarre le faux hôte (`faux_hote.py`) sur l'autre poste, puis
`rag3weaver-tty` là-bas par `ssh -tt`, dans un pseudo-terminal qu'il pilote. Il mesure :
- la latence d'affichage (un token publié par l'hôte → ses octets reçus ici) ;
- l'écho d'une frappe (une touche envoyée → son écho reçu) ;
- le redimensionnement (la fenêtre change → l'écran se redessine dans la nouvelle largeur) ;
- les couleurs de chaque thème (la séquence SGR de la couleur `texte`, en couleur vraie et en 256) ;
- les touches (Échap → `stop`, `o` → l'accord, Ctrl-C → la sortie et le curseur rendu) ;
- le débit (octets reçus par seconde).
Il écrit un JSON de résultats et dit VERT ou ROUGE à la fin. Bibliothèque standard seulement.

    mesure.py <hôte ssh> <binaire distant> <faux_hote.py distant> <dossier distant> <thèmes.json local> <sortie.json>
"""
import fcntl
import json
import os
import pty
import re
import select
import signal
import statistics
import struct
import subprocess
import sys
import termios
import threading
import time
import urllib.request

HOTE, BINAIRE, FAUX, DOSSIER, THEMES, SORTIE = sys.argv[1:7]
PORT = 17990
TUNNEL = 17991


def ssh(cmd, attendre=True):
    p = subprocess.run(["ssh", "-o", "BatchMode=yes", HOTE, "bash", "-s"], input=cmd, text=True, capture_output=True, timeout=120)
    return p.stdout


class Terminal:
    """Le terminal distant, vu d'ici : un pseudo-terminal dont on lit chaque octet, horodaté."""

    def __init__(self, env, theme, colonnes=100, lignes=30):
        self.maitre, esclave = pty.openpty()
        self.taille(colonnes, lignes, signaler=False)
        commande = f"env {env} {BINAIRE} --theme {theme} --depot faux --tache mesure --daemon-json {DOSSIER}/daemon.json --demande go"
        self.p = subprocess.Popen(["ssh", "-tt", "-o", "BatchMode=yes", HOTE, commande], stdin=esclave, stdout=esclave, stderr=esclave, start_new_session=True)
        os.close(esclave)
        self.recu = []  # (t, octets)
        self.verrou = threading.Lock()
        self.fini = False
        threading.Thread(target=self.lire, daemon=True).start()

    def taille(self, colonnes, lignes, signaler=True):
        fcntl.ioctl(self.maitre, termios.TIOCSWINSZ, struct.pack("HHHH", lignes, colonnes, 0, 0))
        if signaler and self.p.poll() is None:
            os.killpg(self.p.pid, signal.SIGWINCH)

    def lire(self):
        while True:
            try:
                r, _, _ = select.select([self.maitre], [], [], 0.5)
                if r:
                    o = os.read(self.maitre, 65536)
                    if not o:
                        break
                    with self.verrou:
                        self.recu.append((time.monotonic(), o))
                elif self.p.poll() is not None:
                    break
            except OSError:
                break
        self.fini = True

    def tout(self):
        with self.verrou:
            return b"".join(o for _, o in self.recu)

    def depuis(self, t0):
        with self.verrou:
            return [(t, o) for t, o in self.recu if t >= t0]

    def attendre(self, motif, apres=0.0, borne=20.0):
        """Le premier instant (après `apres`) où les octets reçus depuis `apres` contiennent `motif`."""
        fin = time.monotonic() + borne
        while time.monotonic() < fin:
            acc = b""
            for t, o in self.depuis(apres):
                acc += o
                if motif in acc:
                    return t
            time.sleep(0.002)
        return None

    def ecrire(self, octets):
        t = time.monotonic()
        os.write(self.maitre, octets)
        return t

    def arreter(self):
        if self.p.poll() is None:
            try:
                os.killpg(self.p.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        self.p.wait(timeout=10)


def indice_256(r, g, b):
    niveaux = [0, 95, 135, 175, 215, 255]
    proche = lambda v: min(range(6), key=lambda i: abs(niveaux[i] - v))
    ri, gi, bi = proche(r), proche(g), proche(b)
    cube = 16 + 36 * ri + 6 * gi + bi
    d = lambda c: (c[0] - r) ** 2 + (c[1] - g) ** 2 + (c[2] - b) ** 2
    gi2 = min((max((r + g + b) // 3 - 8, 0)) // 10, 23)
    gris = 8 + 10 * gi2
    return 232 + gi2 if d((gris, gris, gris)) < d((niveaux[ri], niveaux[gi], niveaux[bi])) else cube


def rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def hote_demarrer():
    ssh(f"mkdir -p {DOSSIER}; rm -f {DOSSIER}/journal.jsonl; setsid nohup python3 {FAUX} {PORT} {DOSSIER}/journal.jsonl {DOSSIER}/daemon.json > {DOSSIER}/faux.out 2>&1 < /dev/null & echo $! > {DOSSIER}/faux.pid; sleep 0.5")


def hote_arreter():
    ssh(f"kill -TERM $(cat {DOSSIER}/faux.pid) 2>/dev/null; rm -f {DOSSIER}/faux.pid")


def journal():
    return [json.loads(l) for l in ssh(f"cat {DOSSIER}/journal.jsonl 2>/dev/null").splitlines() if l.strip()]


def marque(texte):
    req = urllib.request.Request(f"http://127.0.0.1:{TUNNEL}/faux/marque", data=json.dumps({"texte": texte}).encode(), headers={"Authorization": "Bearer jeton-de-mesure", "Content-Type": "application/json"}, method="POST")
    return urllib.request.urlopen(req, timeout=10).read()


def p(valeurs, q):
    v = sorted(valeurs)
    return v[min(len(v) - 1, int(round(q * (len(v) - 1))))]


resultats = {"hote": HOTE, "date": time.strftime("%Y-%m-%d %H:%M"), "rouges": []}
themes = json.load(open(THEMES))


def rouge(quoi):
    resultats["rouges"].append(quoi)
    print("ROUGE", quoi, flush=True)


# 1. La séance principale (daemon, couleur vraie) : latences, redimensionnement, touches, débit.
hote_demarrer()
tunnel = subprocess.Popen(["ssh", "-N", "-o", "BatchMode=yes", "-L", f"{TUNNEL}:127.0.0.1:{PORT}", HOTE])
time.sleep(1.5)
try:
    t = Terminal("COLORTERM=truecolor TERM=xterm-256color", "daemon")
    debut = time.monotonic()
    if t.attendre(b"attend ton accord", borne=40) is None:
        rouge("la demande d'accord ne paraît pas en 40 s")
    else:
        duree_scenario = time.monotonic() - debut
        octets_scenario = len(t.tout())
        resultats["scenario"] = {"secondes": round(duree_scenario, 2), "octets": octets_scenario, "octets_par_seconde": round(octets_scenario / duree_scenario)}
        # La latence d'affichage : un token publié par l'hôte, ses octets reçus ici.
        lat = []
        for i in range(20):
            m = f"MARQ{i:02d}X"
            t0 = time.monotonic()
            marque(" " + m)
            t1 = t.attendre(m.encode(), apres=t0, borne=10)
            if t1 is None:
                rouge(f"la marque {m} ne paraît pas")
            else:
                lat.append((t1 - t0) * 1000)
            time.sleep(0.15)
        if lat:
            resultats["latence_affichage_ms"] = {"n": len(lat), "mediane": round(statistics.median(lat), 1), "p95": round(p(lat, 0.95), 1), "max": round(max(lat), 1)}
        # L'écho d'une frappe : z, son écho, puis effacer.
        echo = []
        for i in range(20):
            t0 = t.ecrire(b"z")
            t1 = t.attendre(b"z", apres=t0, borne=10)
            if t1 is None:
                rouge("l'écho de z ne revient pas")
            else:
                echo.append((t1 - t0) * 1000)
            t.ecrire(b"\x7f")
            time.sleep(0.15)
        if echo:
            resultats["echo_frappe_ms"] = {"n": len(echo), "mediane": round(statistics.median(echo), 1), "p95": round(p(echo, 0.95), 1), "max": round(max(echo), 1)}
        # Le redimensionnement : 60 colonnes, puis retour à 100.
        for colonnes in (60, 100):
            t0 = time.monotonic()
            t.taille(colonnes, 24)
            time.sleep(1.0)
            apres = b"".join(o for _, o in t.depuis(t0))
            cols = [int(c) for c in re.findall(rb"\x1b\[\d+;(\d+)H", apres)]
            entry = {"colonnes": colonnes, "octets_apres": len(apres), "colonne_max_visee": max(cols) if cols else None}
            if not apres:
                rouge(f"rien ne se redessine après le passage à {colonnes} colonnes")
            elif cols and max(cols) > colonnes:
                rouge(f"une écriture vise la colonne {max(cols)} pour une largeur de {colonnes}")
            resultats.setdefault("redimensionnement", []).append(entry)
        # Les touches : Échap → stop ; o → l'accord ; puis la fin dite ; Ctrl-C → la sortie.
        t.ecrire(b"\x1b")
        time.sleep(1.0)
        t0 = t.ecrire(b"o")
        if t.attendre("fini".encode(), apres=t0, borne=15) is None:
            rouge("après o, la fin ne paraît pas")
        t0 = t.ecrire(b"\x03")
        fin = time.monotonic() + 10
        while t.p.poll() is None and time.monotonic() < fin:
            time.sleep(0.05)
        resultats["ctrl_c_sortie_ms"] = round((time.monotonic() - t0) * 1000) if t.p.poll() is not None else None
        if t.p.poll() is None:
            rouge("Ctrl-C ne fait pas sortir en 10 s")
        if b"\x1b[?25h" not in t.tout()[-4096:]:
            rouge("le curseur n'est pas rendu à la sortie")
        j = journal()
        stops = [e for e in j if e["corps"].get("kind") == "stop"]
        accords = [e for e in j if e["corps"].get("kind") == "answer" and e["corps"].get("approved") is True]
        resultats["touches"] = {"echap_stop": len(stops), "o_accord": len(accords), "reply_to": accords[0]["corps"].get("reply_to") if accords else None}
        if len(stops) != 1:
            rouge(f"Échap : {len(stops)} stop au lieu d'un")
        if len(accords) != 1 or not str(accords[0]["corps"].get("reply_to", "")).startswith("faux/mesure:"):
            rouge(f"o : l'accord n'est pas parti tel quel : {accords}")
    t.arreter()
finally:
    tunnel.terminate()
    hote_arreter()

# 2. Les couleurs de chaque thème, en couleur vraie puis en 256 couleurs.
couleurs = []
for nom, th in themes.items():
    r, g, b = rgb(th["couleurs"]["texte"])
    for mode, env, sgr in (
        ("vraie", "COLORTERM=truecolor TERM=xterm-256color", f"38;2;{r};{g};{b}".encode()),
        ("256", "COLORTERM= TERM=xterm-256color", f"38;5;{indice_256(r, g, b)}".encode()),
    ):
        hote_demarrer()
        try:
            t = Terminal(env, nom)
            vu = t.attendre(sgr, borne=20) is not None
            coeur = th["animations"]["reflechit"]["coeur"]
            coeur = th.get("replis", {}).get(coeur, coeur).encode()
            vu_coeur = t.attendre(coeur, borne=5) is not None
            couleurs.append({"theme": nom, "mode": mode, "sgr_texte": sgr.decode(), "vu": vu, "coeur_vu": vu_coeur})
            if not vu:
                rouge(f"{nom} en {mode} : la couleur texte ({sgr.decode()}) ne paraît pas")
            if not vu_coeur:
                rouge(f"{nom} en {mode} : le cœur du Cercle ne paraît pas")
            t.ecrire(b"\x03")
            time.sleep(0.5)
            t.arreter()
        finally:
            hote_arreter()
resultats["couleurs"] = couleurs
resultats["verdict"] = "ROUGE" if resultats["rouges"] else "VERT"
json.dump(resultats, open(SORTIE, "w"), ensure_ascii=False, indent=2)
print(json.dumps(resultats, ensure_ascii=False, indent=2))
print(resultats["verdict"])
