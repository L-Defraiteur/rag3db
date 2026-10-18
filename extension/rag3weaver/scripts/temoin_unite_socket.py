#!/usr/bin/env python3
"""**Le témoin de l'hôte activé par socket** (18 octobre 2026), sous systemd --user.

Une unité d'ESSAI à part (`rag3weaver-daemon-essai`, son port, son dossier de démon), jamais
celle de service ; posée par `installer-l-unite-de-l-hote.sh`, retirée à la fin.

1. un appel sans hôte démarre l'unité : le client ne lance rien (pas de « lancement de »),
   l'hôte est le MainPID du service ;
2. l'hôte inactif s'arrête seul (délai court), la socket reste à l'écoute ;
3. un client resté ouvert continue : son appel suivant réveille l'unité (jeton neuf relu) ;
4. un client dont le binaire n'est pas celui de l'unité est refusé, en le disant.

Usage : temoin_unite_socket.py <binaire> <racine de l'arbre> <dossier de travail> [port]
La racine porte le gabarit `templates/proto/boutique` et l'extension vector du même bâti
(RAG3DB_ROOT). Le dossier de travail est sur disque (une copie du binaire y va).
"""
import json, os, shutil, subprocess, sys, time

BINAIRE, RACINE, T = os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2]), os.path.abspath(sys.argv[3])
PORT = sys.argv[4] if len(sys.argv) > 4 else "7978"
NOM = "rag3weaver-daemon-essai"
ADRESSE = f"127.0.0.1:{PORT}"
INSTALLER = os.path.join(RACINE, "extension/rag3weaver/scripts/installer-l-unite-de-l-hote.sh")
DEMON = os.path.join(T, "demon")
UNITES = os.path.join(T, "unites")  # le dossier d'unités que le client lit (liens vers l'essai)
systemd = os.path.join(os.environ.get("XDG_CONFIG_HOME", os.path.expanduser("~/.config")), "systemd/user")
echecs = 0


def cas(ok, texte):
    global echecs
    print(("OK     " if ok else "ÉCHEC  ") + texte, flush=True)
    echecs += 0 if ok else 1


def sh(*a, **kw):
    return subprocess.run(list(a), capture_output=True, text=True, **kw)


def main_pid():
    p = sh("systemctl", "--user", "show", "-p", "MainPID", "--value", f"{NOM}.service").stdout.strip()
    return int(p or 0)


def actif(unite):
    return sh("systemctl", "--user", "is-active", unite).stdout.strip()


# ── Le terrain : un dossier à soi, le manifeste du gabarit, l'environnement de l'hôte ──
shutil.rmtree(T, ignore_errors=True)
os.makedirs(DEMON)
os.makedirs(UNITES)
g = os.path.join(RACINE, "extension/rag3weaver/templates/proto/boutique")
m = json.load(open(os.path.join(g, "backend.json")))
m["database"] = os.path.join(T, "boutique.rag3db")
MANIFESTE = os.path.join(g, "temoin-socket.json")
json.dump(m, open(MANIFESTE, "w"))
env_hote = os.path.join(T, "hote.env")
with open(env_hote, "w") as f:
    f.write(f"RAG3WEAVER_DAEMON_DIR={DEMON}\nRAG3DB_ROOT={RACINE}\n")
ENV = dict(os.environ, RAG3WEAVER_DAEMON_DIR=DEMON, RAG3WEAVER_SYSTEMD_USER_DIR=UNITES,
           RAG3WEAVER_DAEMON_ADDRESS=ADRESSE, RAG3DB_ROOT=RACINE)


def client(binaire, appels):
    """Un client MCP : initialize, puis `appels` (des noms d'outils) ; rend (réponses, stderr)."""
    p = subprocess.Popen([binaire, "mcp", "--manifest", MANIFESTE], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.PIPE, text=True, env=ENV)
    n = 0

    def demander(methode, params):
        nonlocal n
        n += 1
        p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": n, "method": methode, "params": params}) + "\n")
        p.stdin.flush()
        return json.loads(p.stdout.readline() or "{}")

    demander("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "témoin", "version": "0"}})
    rendus = []
    for a in appels:
        if callable(a):
            a()
            continue
        r = demander("tools/call", {"name": a, "arguments": {}})
        resultat = r.get("result", {})
        rendus.append(((resultat.get("content") or [{}])[0].get("text", ""), resultat.get("isError", False)))
    p.stdin.close()
    p.wait(timeout=60)
    return rendus, p.stderr.read()


def etat(texte):
    try:
        return json.loads(texte)
    except ValueError:
        return {"brut": texte}


try:
    r = sh("bash", INSTALLER, "--nom", NOM, "--adresse", ADRESSE, "--inactivite", "0.1", "--binaire", BINAIRE,
           "--env", env_hote, "--maintenant")
    print(r.stdout.strip())
    cas(r.returncode == 0, f"l'unité d'essai est posée ({r.stderr.strip()[:200]})")
    for ext in ("socket", "service"):
        os.symlink(os.path.join(systemd, f"{NOM}.{ext}"), os.path.join(UNITES, f"rag3weaver-daemon.{ext}"))
    cas(actif(f"{NOM}.socket") == "active" and actif(f"{NOM}.service") != "active", "la socket écoute, aucun hôte avant l'appel")

    # ── 1. Un appel sans hôte démarre l'unité ──
    ((premier, _),), err = client(BINAIRE, ["daemon_status"])
    e = etat(premier)
    pid = main_pid()
    cas(e.get("state") == "running" and e.get("pid") == pid != 0,
        f"un appel sans hôte démarre l'unité : running, pid de l'hôte = MainPID ({e.get('state')}, {e.get('pid')}, {pid})")
    cas("lancement de" not in err, "le client n'a lancé aucun hôte lui-même")

    # ── 2. L'arrêt après l'inactivité ──
    fin = time.time() + 60
    while actif(f"{NOM}.service") == "active" and time.time() < fin:
        time.sleep(1)
    cas(actif(f"{NOM}.service") != "active", f"l'hôte inactif s'arrête seul ({actif(f'{NOM}.service')})")
    cas(actif(f"{NOM}.socket") == "active", "la socket reste à l'écoute")
    cas(not os.path.exists(os.path.join(DEMON, "daemon.json")), "sa fiche est retirée")

    # ── 3. Un client resté ouvert continue après l'arrêt ──
    def attendre_l_arret():
        fin = time.time() + 60
        while actif(f"{NOM}.service") == "active" and time.time() < fin:
            time.sleep(1)
    pids = []
    rendus, err = client(BINAIRE, ["list_products", lambda: pids.append(main_pid()), attendre_l_arret,
                                   "list_products", lambda: pids.append(main_pid())])
    (t1, e1), (t2, e2) = rendus
    cas(not e1 and not e2 and pids[0] != 0 and pids[1] not in (0, pids[0]),
        f"le même client continue après l'arrêt : un appel d'outil réveille l'unité, un autre hôte répond "
        f"(hôtes {pids}, erreurs {e1}/{e2} : {t2[:120] if e2 else ''})")

    # ── 4. Un client périmé est refusé ──
    copie = os.path.join(T, "rag3weaver-backend-ancien")
    shutil.copy2(BINAIRE, copie)
    ((perime, _),), err = client(copie, ["daemon_status"])
    e = etat(perime)
    cas(e.get("state") == "périmé" and "relancez la session" in str(e.get("reason")),
        f"un client dont le binaire n'est pas celui de l'unité est refusé ({e.get('state')} : {str(e.get('reason'))[:120]})")
finally:
    sh("bash", INSTALLER, "--nom", NOM, "--retirer", "--maintenant")
    for f in (MANIFESTE,):
        try:
            os.remove(f)
        except OSError:
            pass

print(f"== {'VERT' if not echecs else 'ROUGE'} : {echecs} échec(s)")
sys.exit(1 if echecs else 0)
