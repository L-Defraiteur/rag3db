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

    mesure.py <hôte ssh|local> <binaire distant> <faux_hote.py distant> <dossier distant> <thèmes.json local> <sortie.json> [lanceur]

Le lanceur (tty-6, 14 nov. 2026) : `ssh` (par défaut, `ssh -tt`), `tmux-local` (tmux sur ce poste, hôte
`local`), `tmux-ssh` (tmux sur l'autre poste, vu par ssh), `mosh` (mosh-server sur l'autre poste). Sous
tmux et mosh, c'est le multiplexeur qui répond à la position du curseur : on note ce qui arrive (la vue
inline démarre-t-elle, la couleur vraie passe-t-elle ou tombe-t-elle en 256), sans l'exiger — rag3weaver
n'exige ni tmux ni mosh (Lucie, 14 nov.), il doit s'y tenir et se dégrader proprement.
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
LANCEUR = sys.argv[7] if len(sys.argv) > 7 else "ssh"
LOCAL = HOTE == "local"
MULTIPLEXEUR = LANCEUR in ("tmux-local", "tmux-ssh", "mosh")
PORT = 17990
TUNNEL = PORT if LOCAL else 17991
seance = [0]


def ssh(cmd, attendre=True):
    argv = ["bash", "-s"] if LOCAL else ["ssh", "-o", "BatchMode=yes", HOTE, "bash", "-s"]
    p = subprocess.run(argv, input=cmd, text=True, capture_output=True, timeout=120)
    return p.stdout


def commande(base):
    """La commande qui lance le terminal, selon le lanceur."""
    seance[0] += 1
    nom = f"mesure{os.getpid()}-{seance[0]}"
    seance.append(nom)
    if LANCEUR == "ssh":
        return ["ssh", "-tt", "-o", "BatchMode=yes", HOTE, base]
    if LANCEUR == "tmux-local":
        return ["tmux", "-L", nom, "-f", "/dev/null", "new-session", "-s", nom, base]
    if LANCEUR == "tmux-ssh":
        return ["ssh", "-tt", "-o", "BatchMode=yes", HOTE, f"tmux -L {nom} -f /dev/null new-session -s {nom} '{base}'"]
    if LANCEUR == "mosh":
        return ["mosh", HOTE, "--", "bash", "-c", base]
    raise SystemExit(f"lanceur inconnu : {LANCEUR}")


commande_ = commande


class Terminal:
    """Le terminal distant, vu d'ici : un pseudo-terminal dont on lit chaque octet, horodaté."""

    def __init__(self, env, theme, colonnes=100, lignes=30, repond_au_curseur=True, extra="", kitty=False):
        self.repond_au_curseur = repond_au_curseur
        # Un terminal qui annonce le protocole clavier de kitty (tty-10) répond à ESC[?u ; tout
        # terminal répond à la demande d'attributs ESC[c.
        self.kitty = kitty
        self.lignes = lignes
        self.maitre, esclave = pty.openpty()
        self.taille(colonnes, lignes, signaler=False)
        commande = f"env {env} {BINAIRE} --theme {theme} --depot faux --tache mesure --daemon-json {DOSSIER}/daemon.json --demande go {extra}"
        self.p = subprocess.Popen(commande_(commande), stdin=esclave, stdout=esclave, stderr=esclave, start_new_session=True, env=dict(os.environ, TERM="xterm-256color", LANG=os.environ.get("LANG", "fr_FR.UTF-8")))
        os.close(esclave)
        self.recu = []  # (t, octets)
        self.verrou = threading.Lock()
        self.fini = False
        threading.Thread(target=self.lire, daemon=True).start()

    def taille(self, colonnes, lignes, signaler=True):
        self.lignes = lignes
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
                    # Un vrai terminal répond à la demande de position du curseur (DSR,
                    # ESC[6n) ; la vue inline en a besoin pour démarrer.
                    if self.repond_au_curseur:
                        for _ in range(o.count(b"\x1b[6n")):
                            os.write(self.maitre, f"\x1b[{self.lignes};1R".encode())
                    if self.kitty:
                        for _ in range(o.count(b"\x1b[?u")):
                            os.write(self.maitre, b"\x1b[?0u")
                    for _ in range(o.count(b"\x1b[c")):
                        os.write(self.maitre, b"\x1b[?62;22c")
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
        # Le serveur tmux de la séance et sa socket partent avec elle (rien ne reste sous /tmp/tmux-*).
        if LANCEUR in ("tmux-local", "tmux-ssh"):
            nom = seance[-1]
            ssh(f"tmux -L {nom} kill-server 2>/dev/null; rm -f /tmp/tmux-$(id -u)/{nom}")
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
    # Le faux hôte écrit lui-même son pid dans faux.pid (pas celui d'un lanceur qui rend la main).
    ssh(f"mkdir -p {DOSSIER}; rm -f {DOSSIER}/journal.jsonl {DOSSIER}/faux.pid; nohup python3 {FAUX} {PORT} {DOSSIER}/journal.jsonl {DOSSIER}/daemon.json > {DOSSIER}/faux.out 2>&1 < /dev/null & for i in $(seq 50); do [ -s {DOSSIER}/faux.pid ] && break; sleep 0.1; done")


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


# Sous mosh, chaque séance ouvre un journal de lui-même (tty-7) : le harnais efface à la fin ceux qu'il a fait naître.
ETAT_TTY = "$HOME/.local/state/rag3weaver/tty"
JOURNAUX_AVANT = set(ssh(f"ls -1 {ETAT_TTY} 2>/dev/null").split())
resultats = {"hote": HOTE, "lanceur": LANCEUR, "date": time.strftime("%Y-%m-%d %H:%M"), "rouges": []}
themes = json.load(open(THEMES))


def rouge(quoi):
    resultats["rouges"].append(quoi)
    print("ROUGE", quoi, flush=True)


# 0. Un terminal qui ne rend pas la position du curseur : la vue ne démarre pas, le texte brut prend la
#    suite (le défaut trouvé par ce harnais le 13 nov. 2026 : ratatui paniquait).
hote_demarrer()
tunnel = subprocess.Popen(["sleep", "86400"] if LOCAL else ["ssh", "-N", "-o", "BatchMode=yes", "-L", f"{TUNNEL}:127.0.0.1:{PORT}", HOTE])
time.sleep(1.5)
try:
    t = Terminal("COLORTERM=truecolor TERM=xterm-256color", "daemon", repond_au_curseur=False)
    t0 = time.monotonic()
    repli = t.attendre(b"je passe en texte brut", borne=8 if MULTIPLEXEUR else 20)
    vu_texte = t.attendre(b"mot199", apres=t0, borne=30) if repli else None
    resultats["sans_position_du_curseur"] = {"repli_dit": repli is not None, "texte_brut_suit": vu_texte is not None, "panique": b"panicked" in t.tout()}
    if MULTIPLEXEUR:
        # Le multiplexeur répond lui-même au curseur : la vue inline doit démarrer, sans panique.
        vue = t.attendre(b"attend ton accord", apres=t0, borne=40) is not None
        resultats["sans_position_du_curseur"]["vue_inline_demarre"] = vue
        if b"panicked" in t.tout() or not (vue or (repli and vu_texte)):
            rouge(f"sous {LANCEUR}, sans réponse du harnais au curseur : {resultats['sans_position_du_curseur']}")
    elif repli is None or vu_texte is None or b"panicked" in t.tout():
        rouge(f"sans position du curseur : {resultats['sans_position_du_curseur']}")
    t.ecrire(b"\x03")
    time.sleep(0.5)
    t.arreter()
finally:
    tunnel.terminate()
    hote_arreter()

# 1. La séance principale (daemon, couleur vraie) : latences, redimensionnement, touches, débit.
hote_demarrer()
tunnel = subprocess.Popen(["sleep", "86400"] if LOCAL else ["ssh", "-N", "-o", "BatchMode=yes", "-L", f"{TUNNEL}:127.0.0.1:{PORT}", HOTE])
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
        # Ctrl-G arrête sans le délai d'échappement d'un multiplexeur (tty-6) ; puis Échap.
        ctrlg_mur = time.time()
        t.ecrire(b"\x07")
        time.sleep(1.0)
        echap_mur = time.time()
        t.ecrire(b"\x1b")
        time.sleep(1.5)
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
        resultats["curseur_rendu_a_la_sortie"] = b"\x1b[?25h" in t.tout()[-4096:]
        if not resultats["curseur_rendu_a_la_sortie"] and not MULTIPLEXEUR:
            rouge("le curseur n'est pas rendu à la sortie")
        j = journal()
        stops = [e for e in j if e["corps"].get("kind") == "stop"]
        accords = [e for e in j if e["corps"].get("kind") == "answer" and e["corps"].get("approved") is True]
        # La frontière au milieu de la seconde entre Ctrl-G et Échap : les horloges des deux postes
        # s'écartent de quelques millisecondes (tty-9 : un stop d'Échap daté 6 ms trop tôt comptait pour Ctrl-G).
        frontiere = (ctrlg_mur + echap_mur) / 2
        stops_g = [e for e in stops if e["t"] < frontiere]
        stops_e = [e for e in stops if e["t"] >= frontiere]
        resultats["touches"] = {"ctrl_g_stop": len(stops_g), "echap_stop": len(stops_e), "o_accord": len(accords), "reply_to": accords[0]["corps"].get("reply_to") if accords else None,
                                "ctrl_g_vers_stop_ms": round((stops_g[0]["t"] - ctrlg_mur) * 1000) if stops_g else None,
                                "echap_vers_stop_ms": round((stops_e[0]["t"] - echap_mur) * 1000) if stops_e else None}
        if len(stops_g) != 1:
            rouge(f"Ctrl-G : {len(stops_g)} stop au lieu d'un")
        if len(stops_e) != 1:
            rouge(f"Échap : {len(stops_e)} stop au lieu d'un")
        if len(accords) != 1 or not str(accords[0]["corps"].get("reply_to", "")).startswith("faux/mesure:"):
            rouge(f"o : l'accord n'est pas parti tel quel : {accords}")
    t.arreter()
finally:
    tunnel.terminate()
    hote_arreter()

# 1b. Le journal de séance (tty-7) : `--journal` écrit en texte brut ce qui part dans l'historique du
#     terminal, sous tous les lanceurs. Sous mosh, qui ne garde aucun historique au-delà de l'écran, la
#     zone vivante dit la commande qui relit le journal ; sans `--journal`, un journal s'ouvre de lui-même.
ATTENDUS = {f"mot{i}" for i in range(200)}


def verifier_journal(texte, quoi):
    vus = set(re.findall(r"mot\d+", texte))
    manquent = sorted(ATTENDUS - vus, key=lambda m: int(m[3:]))
    code = 'println!("bonjour");' in texte
    resultats.setdefault("journal", {})[quoi] = {"octets": len(texte), "mots_manquants": len(manquent), "code": code}
    if manquent or not code:
        rouge(f"{quoi} : le journal ne porte pas tout l'historique ({len(manquent)} mots manquants, dont {manquent[:3]} ; bloc de code {'présent' if code else 'absent'})")


def seance_journal(extra, quoi):
    hote_demarrer()
    tunnel = subprocess.Popen(["sleep", "86400"] if LOCAL else ["ssh", "-N", "-o", "BatchMode=yes", "-L", f"{TUNNEL}:127.0.0.1:{PORT}", HOTE])
    time.sleep(1.5)
    try:
        t = Terminal("COLORTERM=truecolor TERM=xterm-256color", "daemon", extra=extra)
        if t.attendre(b"attend ton accord", borne=40) is None:
            rouge(f"{quoi} : la demande d'accord ne paraît pas en 40 s")
        time.sleep(1.0)
        ecran = t.tout()
        t.ecrire(b"\x03")
        time.sleep(0.5)
        t.arreter()
        return ecran
    finally:
        tunnel.terminate()
        hote_arreter()


chemin = f"{DOSSIER}/seance-journal.txt"
ssh(f"rm -f {chemin}")
ecran = seance_journal(f"--journal {chemin}", "journal_explicite")
dit = f"less {chemin}".encode() in ecran
resultats["journal_explicite_relecture_dite"] = dit
if LANCEUR == "mosh" and not dit:
    rouge(f"sous mosh, la zone vivante ne dit pas « less {chemin} »")
if LANCEUR != "mosh" and b"sous mosh" in ecran:
    rouge(f"sous {LANCEUR}, la vue se croit sous mosh")
verifier_journal(ssh(f"cat {chemin} 2>/dev/null"), "journal_explicite")
ssh(f"rm -f {chemin}")

if LANCEUR == "mosh":
    etat_tty = ETAT_TTY
    avant = set(ssh(f"ls -1 {etat_tty} 2>/dev/null").split())
    ecran = seance_journal("", "journal_de_lui_meme")
    nouveaux = sorted(set(ssh(f"ls -1 {etat_tty} 2>/dev/null").split()) - avant)
    resultats["journal_de_lui_meme_fichiers"] = nouveaux
    if b"rag3weaver/tty/seance-" not in ecran or b"less " not in ecran:
        rouge("sous mosh sans --journal, la zone vivante ne dit pas le journal ouvert")
    if len(nouveaux) != 1:
        rouge(f"sous mosh sans --journal : {len(nouveaux)} journaux neufs au lieu d'un ({nouveaux})")
    else:
        verifier_journal(ssh(f"cat {etat_tty}/{nouveaux[0]}"), "journal_de_lui_meme")
    for n in nouveaux:
        ssh(f"rm -f {etat_tty}/{n}")

# 1d. La saisie au clavier (tty-8) : les séquences d'un vrai terminal (flèches, Début et Fin, Suppr,
#     Ctrl-← et Alt-B, Ctrl-W, Alt-Entrée, collage d'un bloc, ↑), à travers le lanceur ; la boîte du
#     faux hôte doit recevoir le texte édité, tel quel. Puis Ctrl-C vide une saisie avant de quitter.
CLAVIER = [
    b"lis vue.rs", b"\x1b[D" * 6, b"src/", b"\x1b[F", b"\x1b[1;5D", b"\x1b[3~", b"R", b"\x1b[H", b"\x1b[C", b"\x7f", b"L",
    b"\x1b[F", b"\x1b\r", "\x1b[200~puis\nrésume\x1b[201~".encode(), b"\x17", "résume tout".encode(), b"\r",
]
ATTENDU = "Lis src/vue.Rs\npuis\nrésume tout"
hote_demarrer()
tunnel = subprocess.Popen(["sleep", "86400"] if LOCAL else ["ssh", "-N", "-o", "BatchMode=yes", "-L", f"{TUNNEL}:127.0.0.1:{PORT}", HOTE])
time.sleep(1.5)
try:
    t = Terminal("COLORTERM=truecolor TERM=xterm-256color", "daemon")
    if t.attendre(b"attend ton accord", borne=40) is None:
        rouge("clavier : la demande d'accord ne paraît pas en 40 s")
    for morceau in CLAVIER:
        t.ecrire(morceau)
        time.sleep(0.15)
    time.sleep(1.5)
    # ↑ rappelle l'envoi, Fin, « ! », Entrée : un second envoi, édité depuis l'historique.
    for morceau in (b"\x1b[A", b"\x1b[F", b" !", b"\r"):
        t.ecrire(morceau)
        time.sleep(0.15)
    time.sleep(1.5)
    textes = [e["corps"].get("text") for e in journal() if e["corps"].get("kind") in ("answer", "instruction", "request") and e["corps"].get("text") not in (None, "go")]
    resultats["clavier"] = {"textes_recus": textes}
    if textes[:2] != [ATTENDU, ATTENDU + " !"]:
        rouge(f"clavier : la boîte a reçu {textes!r} au lieu de {[ATTENDU, ATTENDU + ' !']!r}")
    # Ctrl-C sur une saisie non vide la vide, sans quitter ; le second quitte.
    t.ecrire(b"zzz")
    time.sleep(0.3)
    t.ecrire(b"\x03")
    time.sleep(1.0)
    vivant = t.p.poll() is None
    t.ecrire(b"\x03")
    fin = time.monotonic() + 10
    while t.p.poll() is None and time.monotonic() < fin:
        time.sleep(0.05)
    resultats["clavier"]["ctrl_c_vide_d_abord"] = vivant
    resultats["clavier"]["ctrl_c_quitte_ensuite"] = t.p.poll() is not None
    if not vivant:
        rouge("clavier : Ctrl-C a quitté au lieu de vider la saisie")
    if t.p.poll() is None:
        rouge("clavier : le second Ctrl-C ne quitte pas")
    t.arreter()
finally:
    tunnel.terminate()
    hote_arreter()

# 1e. La saisie sur plusieurs lignes (tty-10) : « un », Maj-Entrée en CSI u (ESC[13;2u), « deux »,
#     Alt-Entrée, « trois », Entrée. Un terminal qui annonce le protocole de kitty reçoit la demande et
#     l'indication « Maj-Entrée » ; un terminal qui ne l'annonce pas ne la reçoit pas, garde « Alt-Entrée »,
#     et un CSI u qu'il enverrait quand même n'entre jamais comme texte. Sous un multiplexeur, c'est lui qui
#     répond à la demande : on note ce qui arrive.
for kitty in (True, False):
    quoi = "multiligne_kitty" if kitty else "multiligne_sans_kitty"
    hote_demarrer()
    tunnel = subprocess.Popen(["sleep", "86400"] if LOCAL else ["ssh", "-N", "-o", "BatchMode=yes", "-L", f"{TUNNEL}:127.0.0.1:{PORT}", HOTE])
    time.sleep(1.5)
    try:
        t = Terminal("COLORTERM=truecolor TERM=xterm-256color", "daemon", kitty=kitty)
        if t.attendre(b"attend ton accord", borne=40) is None:
            rouge(f"{quoi} : la demande d'accord ne paraît pas en 40 s")
        tout = t.tout()
        demande_kitty = b"\x1b[>1u" in tout
        aide_maj = "Maj-Entrée : à la ligne".encode() in tout
        aide_alt = "Alt-Entrée : à la ligne".encode() in tout
        for morceau in (b"un", b"\x1b[13;2u", b"deux", b"\x1b\r", b"trois", b"\r"):
            t.ecrire(morceau)
            time.sleep(0.2)
        time.sleep(1.5)
        textes = [e["corps"].get("text") for e in journal() if e["corps"].get("kind") in ("answer", "instruction", "request") and e["corps"].get("text") not in (None, "go")]
        resultats[quoi] = {"demande_kitty": demande_kitty, "aide_maj_entree": aide_maj, "aide_alt_entree": aide_alt, "textes_recus": textes}
        if textes[:1] != ["un\ndeux\ntrois"]:
            rouge(f"{quoi} : la boîte a reçu {textes!r} au lieu de ['un\\ndeux\\ntrois']")
        if any("13;2u" in (x or "") or "[13" in (x or "") for x in textes):
            rouge(f"{quoi} : un CSI u est entré comme texte : {textes!r}")
        if not MULTIPLEXEUR:
            if kitty and not (demande_kitty and aide_maj):
                rouge(f"{quoi} : le terminal annonce kitty, mais la demande ({demande_kitty}) ou l'indication Maj-Entrée ({aide_maj}) manque")
            if not kitty and (demande_kitty or not aide_alt):
                rouge(f"{quoi} : le terminal n'annonce pas kitty, mais la demande part ({demande_kitty}) ou l'indication Alt-Entrée manque ({aide_alt})")
        t.ecrire(b"\x03")
        time.sleep(0.5)
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
            vu = t.attendre(sgr, borne=20 if not MULTIPLEXEUR else 8) is not None
            repli_256 = None
            if not vu and mode == "vraie":
                # La couleur vraie peut tomber en 256 sous un multiplexeur : un repli, pas un rouge.
                repli_256 = t.attendre(f"38;5;{indice_256(r, g, b)}".encode(), borne=8) is not None
                vu = bool(repli_256)
            coeur = th["animations"]["reflechit"]["coeur"]
            coeur = th.get("replis", {}).get(coeur, coeur).encode()
            vu_coeur = t.attendre(coeur, borne=5) is not None
            couleurs.append({"theme": nom, "mode": mode, "sgr_texte": sgr.decode(), "vu": vu, "tombe_en_256": repli_256, "coeur_vu": vu_coeur})
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
for n in sorted(set(ssh(f"ls -1 {ETAT_TTY} 2>/dev/null").split()) - JOURNAUX_AVANT):
    ssh(f"rm -f {ETAT_TTY}/{n}")
resultats["verdict"] = "ROUGE" if resultats["rouges"] else "VERT"
json.dump(resultats, open(SORTIE, "w"), ensure_ascii=False, indent=2)
print(json.dumps(resultats, ensure_ascii=False, indent=2))
print(resultats["verdict"])
