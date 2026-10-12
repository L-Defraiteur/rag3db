"""**Le .wal est soldé à l'arrêt propre** (relecture de la mémoire, 12 octobre 2026).

Joué vert sous Linux le 12 octobre (binaire du lot C, base de 34 053 octets de .wal
avant /arret, aucun après, sortie en 0,3 s). Recette : copier le manifeste de
`extension/rag3weaver/templates/backends/code/backend.json` à côté de lui (ses graphes
sont relatifs), `database` et `workspace.root` en absolu ; RAG3DB_ROOT en absolu
(pas de `~` sous zsh) ; sans service d'embarquement, l'index est en plein texte.

Témoin : un hôte qui sert un backend de code avec sa montre, indexé, puis
réindexé par la montre après une édition hors outils ; à POST /arret, le .wal
de la base est-il soldé ? (relecture de la mémoire : la montre pourrait
garder un clone d'Arc<Mutex<Catalog>> et la Database ne pas se détruire.)

Usage : wal_code_montre.py <binaire> <manifeste> <base> <fichier de l'espace de travail>
"""
import json, os, re, subprocess, sys, tempfile, time, urllib.request

BIN, MAN, BASE, FICHIER = sys.argv[1:5]
D = tempfile.mkdtemp(prefix="wal-code-", dir=os.path.dirname(BASE))
ENV = dict(os.environ, RAG3WEAVER_DAEMON_DIR=D)
hote = subprocess.Popen([BIN, "daemon", "--address", "127.0.0.1:7992"], env=ENV,
                        stdout=open(D + "/hote.log", "w"), stderr=subprocess.STDOUT)
fin = time.time() + 60
while not os.path.exists(D + "/daemon.json") and time.time() < fin:
    time.sleep(0.2)
fiche = json.load(open(D + "/daemon.json"))

p = subprocess.Popen([BIN, "mcp", "--manifest", MAN, "--daemon", fiche["address"]],
                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=open(D + "/client.log", "w"), text=True, env=ENV)
n = 0


def demander(m, params):
    global n
    n += 1
    p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": n, "method": m, "params": params}) + "\n")
    p.stdin.flush()
    return json.loads(p.stdout.readline())


def outil(nom, args):
    r = demander("tools/call", {"name": nom, "arguments": args})
    return "".join(c.get("text", "") for c in r.get("result", {}).get("content", []))


demander("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "wal-code", "version": "0"}})
demander("tools/list", {})
recu = outil("index", {"confirm": True})
print("index :", recu[:300].replace("\n", " "))
m = re.search(r"journal : (\S+)", recu.replace("\\n", "\n"))
journal = m.group(1).strip('"') if m else None
print("journal :", journal)
fin = time.time() + 300
termine = False
while time.time() < fin and journal:
    texte = open(journal, errors="replace").read() if os.path.exists(journal) else ""
    if re.search(r"indexation terminée|indexation échouée", texte):
        termine = True
        print("journal d'index :", texte.strip().splitlines()[-1][:200])
        break
    time.sleep(1)
print("index terminé :", termine)


def wal():
    d = os.path.dirname(BASE)
    return [(f, os.path.getsize(os.path.join(d, f))) for f in sorted(os.listdir(d)) if os.path.basename(BASE) in f]


# La montre : une édition hors outils, un mot neuf, attendu dans la recherche.
mot = f"zorglubmontre{int(time.time())}"
with open(FICHIER, "a") as f:
    f.write(f"\n// {mot}\npub fn {mot}() {{}}\n")
fin = time.time() + 60
vu = False
while time.time() < fin:
    # La montre le dit dans le journal de l'hôte : « [montre] src/… réindexé ».
    if re.search(r"\[montre\] \S*" + re.escape(os.path.basename(FICHIER)) + r" réindexé", open(D + "/hote.log", errors="replace").read()):
        vu = True
        break
    time.sleep(1)
print(f"la montre a réindexé « {mot} » :", vu)
p.stdin.close()
p.wait(timeout=30)

print("avant /arret :", wal())
req = urllib.request.Request(f"http://{fiche['address']}/arret", method="POST",
                             headers={"Authorization": f"Bearer {fiche['token']}"}, data=b"{}")
urllib.request.urlopen(req, timeout=30).read()
t0 = time.time()
hote.wait(timeout=120)
print(f"code de l'hôte : {hote.returncode}, sorti en {time.time() - t0:.1f} s")
apres = wal()
print("après /arret :", apres)
reste = [t for f, t in apres if f.endswith(".wal")]
if not (termine and vu):
    print("== SANS OBJET : l'index ou la montre n'a pas tourné, le témoin ne dit rien")
else:
    print("== VERT : .wal soldé" if not reste or reste == [0] else f"== ROUGE : .wal de {reste} octets après l'arrêt")
print("--- journal de l'hôte")
print(open(D + "/hote.log").read()[-2000:])
