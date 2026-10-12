#!/usr/bin/env python3
"""**L'épreuve du cycle du démon**, sur les runners du paquet (lot C, relecture
de la mémoire, 12 octobre 2026) : sous Windows et macOS, le bâti seul ne
prouvait rien du mode démon. Ici :

1. un hôte lancé à la main (`daemon --address`) écrit sa fiche, et `/sante`
   rend le même pid ;
2. un client `mcp --daemon <adresse>` s'y attache, et `daemon_status` dit
   « running » ;
3. `POST /arret` avec le jeton arrête l'hôte, et la fiche part ;
4. un client sans `--daemon` lance lui-même un hôte détaché (`ensure`), qui
   survit au client ; puis `/arret`.

Usage : epreuve_demon.py <binaire> <manifeste> — rend 0 si tout tient.
Toujours dans un dossier de démon temporaire (RAG3WEAVER_DAEMON_DIR) : jamais
la fiche réelle d'un poste.
"""
import json
import os
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request

# Les accents passent tels quels dans le journal du runner Windows.
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

BINAIRE, MANIFESTE = sys.argv[1], sys.argv[2]
DOSSIER = tempfile.mkdtemp(prefix="epreuve-demon-")
ENV = dict(os.environ, RAG3WEAVER_DAEMON_DIR=DOSSIER)
FICHE = os.path.join(DOSSIER, "daemon.json")
echecs = 0


def cas(ok, texte):
    global echecs
    print(("OK     " if ok else "ÉCHEC  ") + texte, flush=True)
    if not ok:
        echecs += 1


def attendre(predicat, secondes=60):
    fin = time.time() + secondes
    while time.time() < fin:
        if predicat():
            return True
        time.sleep(0.5)
    return False


def fiche():
    try:
        return json.load(open(FICHE))
    except (OSError, ValueError):
        return None


def sante(adresse):
    try:
        return json.load(urllib.request.urlopen(f"http://{adresse}/sante", timeout=5))
    except Exception:
        return None


def journal_de_l_hote():
    print(f"--- journal de l'hôte (code de sortie : {hote.poll()})")
    try:
        print(open(os.path.join(DOSSIER, "hote.log"), errors="replace").read()[-3000:])
    except OSError as e:
        print(f"       illisible : {e}")


def arret(f):
    if not f.get("address") or not f.get("token"):
        print("       pas de fiche : rien à arrêter par /arret")
        return False
    requete = urllib.request.Request(
        f"http://{f['address']}/arret", method="POST", headers={"Authorization": f"Bearer {f['token']}"}, data=b"{}"
    )
    try:
        urllib.request.urlopen(requete, timeout=30).read()
        return True
    except Exception as e:
        print(f"       /arret : {e}")
        return False


def client(arguments, env):
    """Un client MCP : initialize, tools/list, daemon_status ; rend l'état."""
    p = subprocess.Popen(
        [BINAIRE, "mcp", "--manifest", MANIFESTE, *arguments],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env,
    )
    n = 0

    def demander(methode, params):
        nonlocal n
        n += 1
        p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": n, "method": methode, "params": params}) + "\n")
        p.stdin.flush()
        ligne = p.stdout.readline()
        return json.loads(ligne) if ligne else {}

    demander("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "épreuve", "version": "0"}})
    outils = [t["name"] for t in demander("tools/list", {}).get("result", {}).get("tools", [])]
    # Le client démarre avec `daemon_status` seul (« connexion à l'hôte en
    # cours ») et s'attache par un fil, qui réessaie toutes les 10 s : on
    # redemande l'état tant qu'il ne dit pas « running », 90 s au plus. Sous
    # Windows, lancer l'hôte et ouvrir le backend passe la seconde (run
    # 38226368160), et le verrou de la base que l'hôte précédent vient de
    # fermer peut tarder à être rendu par le système (run 38232395492 : « held
    # by another writer process », l'hôte arrêté une seconde avant). Un
    # défaut qui dure passe les 90 s et reste un échec.
    fin = time.time() + 90
    while True:
        r = demander("tools/call", {"name": "daemon_status", "arguments": {}})
        texte = (r.get("result", {}).get("content") or [{}])[0].get("text", "{}")
        try:
            etat = json.loads(texte)
        except ValueError:
            etat = {"brut": texte}
        if etat.get("state") == "running" or time.time() > fin:
            break
        time.sleep(1)
    print(f"       daemon_status : {etat.get('state')} après {90 - max(0, round(fin - time.time()))} s"
          + ("" if etat.get("state") == "running" else f" — {etat.get('reason')}"))
    p.stdin.close()
    # Le client fini, ses tuyaux doivent se fermer : un hôte détaché qui en
    # hériterait les garderait ouverts, et l'agent attendrait sans fin (vu sous
    # Windows le 12 octobre : deux heures, jusqu'à l'annulation du job).
    try:
        p.wait(timeout=30)
    except subprocess.TimeoutExpired:
        p.kill()
    lu = []
    fil = threading.Thread(target=lambda: lu.append(p.stderr.read()), daemon=True)
    fil.start()
    fil.join(30)
    cas(not fil.is_alive(), f"les tuyaux du client se ferment avec lui (client fini : {p.poll() is not None})"
        + ("" if not fil.is_alive() else " — un hôte détaché en a hérité"))
    erreurs = lu[0] if lu else ""
    if etat.get("state") != "running" and erreurs:
        print("       --- stderr du client\n" + erreurs[-2000:])
    return outils, etat, erreurs


# ── 0. Une base qui existe déjà ─────────────────────────────────────────────
# Un premier hôte crée la base du manifeste, l'ouvre par un client, puis
# s'arrête : la phase 1 rouvre ainsi une base existante, comme tout hôte
# relancé. Sous Windows, la phase 4 ne pouvait pas rouvrir la base que la
# phase 1 avait créée (run 38236456277 : verrou refusé pendant 90 s) ; si la
# phase 1 rougit aussi, c'est la réouverture d'une base, pas le lancement
# par le client.
zero = subprocess.Popen([BINAIRE, "daemon", "--address", "127.0.0.1:7990"], env=ENV,
                        stdout=open(os.path.join(DOSSIER, "hote0.log"), "w"), stderr=subprocess.STDOUT)
if attendre(lambda: fiche() is not None or zero.poll() is not None):
    f0 = fiche() or {}
    _, etat0, _ = client(["--daemon", f0.get("address", "127.0.0.1:7990")], ENV)
    cas(etat0.get("state") == "running", f"phase 0 : un premier hôte crée et ouvre la base ({etat0.get('state')})")
    arret(f0)
    attendre(lambda: zero.poll() is not None, 60)
    cas(zero.poll() is not None and not os.path.exists(FICHE), f"phase 0 : le premier hôte s'arrête (code {zero.poll()})")
if zero.poll() is None:
    zero.kill()

# ── 1. Un hôte à la main ────────────────────────────────────────────────────
hote = subprocess.Popen([BINAIRE, "daemon", "--address", "127.0.0.1:7990"], env=ENV,
                        stdout=open(os.path.join(DOSSIER, "hote.log"), "w"), stderr=subprocess.STDOUT)
attendre(lambda: fiche() is not None or hote.poll() is not None)
cas(fiche() is not None, f"l'hôte écrit sa fiche (code de sortie : {hote.poll()})")
if fiche() is None:
    # Un hôte qui ne démarre pas : son journal dit pourquoi, la suite n'a
    # pas d'objet.
    journal_de_l_hote()
    if hote.poll() is None:
        hote.kill()
    print("== ROUGE : l'hôte n'a pas démarré")
    sys.exit(1)
f = fiche() or {}
s = sante(f.get("address", "127.0.0.1:7990")) or {}
cas(s.get("pid") == f.get("pid") == hote.pid, f"/sante rend le pid de la fiche et du processus ({s.get('pid')}, {f.get('pid')}, {hote.pid})")

# ── 2. Un client attaché par --daemon ───────────────────────────────────────
outils, etat, erreurs = client(["--daemon", f.get("address", "127.0.0.1:7990")], ENV)
cas(etat.get("state") == "running" and etat.get("pid") == hote.pid, f"daemon_status : running, pid de l'hôte ({etat})")
cas(len(outils) > 1, f"les outils du manifeste, pas seulement l'état ({len(outils)})")

# ── 3. /arret avec le jeton ─────────────────────────────────────────────────
cas(arret(f), "POST /arret avec le jeton répond")
cas(attendre(lambda: hote.poll() is not None, 60), f"l'hôte s'arrête (code {hote.poll()})")
cas(not os.path.exists(FICHE), "la fiche est retirée")

# ── 4. Un client sans --daemon lance l'hôte, détaché ────────────────────────
env4 = dict(ENV, RAG3WEAVER_DAEMON_ADDRESS="127.0.0.1:0")
outils, etat, erreurs = client([], env4)
f = fiche() or {}
cas(etat.get("state") == "running" and f.get("pid"), f"le client a lancé l'hôte, qui répond ({etat.get('state')}, pid {f.get('pid')})")
if etat.get("state") != "running":
    # Un hôte, ou deux ? (relecture de la mémoire : un second lancement par
    # la reprise du client se verrait refuser la base par le premier.)
    nom = os.path.basename(BINAIRE)
    liste = (["tasklist", "/FI", f"IMAGENAME eq {nom}"] if os.name == "nt" else ["ps", "-C", nom, "-o", "pid,etime,args"])
    try:
        print("       --- processus de l'hôte\n" + subprocess.run(liste, capture_output=True, text=True, timeout=30).stdout)
    except Exception as e:
        print(f"       processus illisibles : {e}")
    journal = os.path.join(DOSSIER, "daemon.log")
    texte = open(journal, errors="replace").read() if os.path.exists(journal) else ""
    print(f"       daemon.json : pid {f.get('pid')} ; démarrages dans daemon.log : {texte.count('rag3weaver-daemon — http://')}")
    print("       --- daemon.log\n" + texte[-2500:])
s = sante(f.get("address", "")) if f else None
cas(bool(s) and s.get("pid") == f.get("pid"), "l'hôte détaché survit au client qui l'a lancé")
if f:
    cas(arret(f), "et s'arrête par /arret")
    cas(attendre(lambda: not os.path.exists(FICHE), 60), "sa fiche retirée")

if echecs:
    journal_de_l_hote()
if hote.poll() is None:
    hote.kill()
print(f"== {'VERT' if not echecs else 'ROUGE'} : {echecs} échec(s)")
sys.exit(1 if echecs else 0)
