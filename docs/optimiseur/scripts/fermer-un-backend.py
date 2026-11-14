#!/usr/bin/env python3
"""Ferme ou rouvre UN backend de l'hôte 7970 du poste, sans arrêter l'hôte (14 nov. 2026, passe de l'archiviste).
fermer <manifeste> <relevé.json> : POST /backend/close, puis relève taille et mtime de chaque fichier de la base.
rouvrir <manifeste> <relevé.json> : relit les fichiers et dit ce qui a changé depuis le relevé (sans rien refuser :
c'est à l'appelant de juger), puis POST /backend/open."""
import json, os, sys, glob, urllib.request
action, manifeste, releve = sys.argv[1:4]
J = open(os.path.expanduser("~/.local/share/rag3weaver/daemon.token")).read().strip()
def d(m, r, c=None, s=None):
    q = urllib.request.Request("http://127.0.0.1:7970"+r, method=m, data=json.dumps(c).encode() if c is not None else None,
        headers={"Authorization": f"Bearer {J}", "Content-Type": "application/json", **({"X-Rag3weaver-Session": s} if s else {})})
    try:
        with urllib.request.urlopen(q, timeout=300) as x: return json.loads(x.read() or b"{}")
    except urllib.error.HTTPError as e: return json.loads(e.read() or b"{}")
base = json.load(open(manifeste))["database"]
def fichiers():
    r = {}
    for f in sorted(glob.glob(base + "*") + glob.glob(base[:-len(".rag3db")] + ".checkpoints*")):
        for g in ([f] if os.path.isfile(f) else [os.path.join(dp, n) for dp, _, ns in os.walk(f) for n in ns]):
            st = os.stat(g); r[g] = [st.st_size, int(st.st_mtime)]
    return r
s = d("POST", "/session", {"client": "G-" + action, "pid": os.getpid()})["session"]
try:
    if action == "fermer":
        o = d("POST", "/backend/close", {"manifest": manifeste}, s); print("fermeture :", json.dumps(o, ensure_ascii=False)[:300])
        f = fichiers(); json.dump(f, open(releve, "w"), indent=1); print(len(f), "fichiers relevés ;", "wal :", [k for k in f if k.endswith(".wal")])
    else:
        avant = json.load(open(releve)); apres = fichiers()
        diff = sorted(set(avant) ^ set(apres)) + [k for k in avant if k in apres and avant[k] != apres[k]]
        print(len(diff), "fichiers changés depuis la fermeture :", diff[:20])
        o = d("POST", "/backend/open", {"manifest": manifeste}, s); print("ouverture :", o.get("ok"), (o.get("error") or "")[:200])
finally:
    d("DELETE", "/session", None, s)
