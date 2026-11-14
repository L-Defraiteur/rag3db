#!/usr/bin/env python3
"""banc_recherche.py (14 nov. 2026, hotfixs) — le banc des recherches par l'outil search_code.

  autonome : banc_recherche.py autonome <backend.json> <sortie.json> <requête>…
             lance `rag3weaver-backend mcp --standalone` sur le manifeste (une COPIE : il possède sa base)
  hôte     : banc_recherche.py hote <backend.json> <sortie.json> <requête>…
             POST /session puis /call sur l'hôte de daemon.json ; le jeton est lu, jamais affiché.

  fermer   : banc_recherche.py fermer <backend.json> <raison> <jusqu'à>   (fermeture qui tient ; refuse si l'hôte ne la tient pas)
  ouvrir   : banc_recherche.py ouvrir <backend.json>
  comparer : banc_recherche.py comparer <avant.json> <après.json> [<après-large.json>]
             VERT : chaque résultat d'avant se retrouve après, au plus loin dans le banc large (BANC_LIMITE
             plus grand) : une purge change les statistiques BM25, donc le classement à la frontière du top-k,
             mais ne doit faire disparaître aucun résultat vivant.

La sortie : {requête: {"texte": la réponse de l'outil, "cles": [chemin:ligne de chaque résultat]}}.
"""
import json, os, re, subprocess, sys, urllib.error, urllib.request

BIN = os.path.expanduser("~/.local/bin/rag3weaver-backend")
LIMITE = int(os.environ.get("BANC_LIMITE", "20"))

def cles(texte):
    # Un résultat se reconnaît à son « chemin:ligne » ; l'ordre est gardé.
    vues, out = set(), []
    for m in re.finditer(r"([\w./+-]+\.\w+):(\d+)", texte):
        k = f"{m.group(1)}:{m.group(2)}"
        if k not in vues:
            vues.add(k); out.append(k)
    return out

def texte_de(res):
    if isinstance(res, dict) and "content" in res:
        return "\n".join(c.get("text", "") for c in res["content"] if isinstance(c, dict))
    return json.dumps(res, ensure_ascii=False)

def autonome(manifeste, requetes):
    env = dict(os.environ)
    p = subprocess.Popen([BIN, "mcp", "--manifest", manifeste, "--keys", "dev", "--standalone"],
                         stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=open(os.devnull, "w"), text=True, env=env)
    n = 0
    def demande(methode, params, notif=False):
        nonlocal n
        msg = {"jsonrpc": "2.0", "method": methode, "params": params}
        if not notif:
            n += 1; msg["id"] = n
        p.stdin.write(json.dumps(msg) + "\n"); p.stdin.flush()
        if notif:
            return None
        while True:
            ligne = p.stdout.readline()
            if not ligne:
                raise SystemExit(f"le serveur s'est arrêté pendant {methode}")
            r = json.loads(ligne)
            if r.get("id") == n:
                if "error" in r:
                    raise SystemExit(f"{methode} : {r['error']}")
                return r["result"]
    demande("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "banc-purge", "version": "1"}})
    demande("notifications/initialized", {}, notif=True)
    out = {}
    for q in requetes:
        t = texte_de(demande("tools/call", {"name": "search_code", "arguments": {"query": q, "mode": "indexed", "options": {"limit": LIMITE}}}))
        out[q] = {"texte": t, "cles": cles(t)}
    p.stdin.close(); p.wait(timeout=120)
    return out

def post(route, corps, session=None):
    base = os.path.expanduser("~/.local/share/rag3weaver")
    adresse = json.load(open(f"{base}/daemon.json"))["address"]
    jeton = open(f"{base}/daemon.token").read().strip()
    h = {"Content-Type": "application/json", "Authorization": f"Bearer {jeton}"}
    if session:
        h["X-Rag3weaver-Session"] = session
    r = urllib.request.Request(f"http://{adresse}{route}", data=json.dumps(corps).encode(), headers=h)
    try:
        return json.load(urllib.request.urlopen(r, timeout=600))
    except urllib.error.HTTPError as e:
        return json.load(e)

def session():
    return post("/session", {"client": "fenetre-purge (hotfixs)", "keys": ["dev"]})["session"]

def fermer(manifeste, raison, jusqu_a):
    r = post("/backend/close", {"manifest": manifeste, "reason": raison, "until": jusqu_a}, session())
    res = r.get("result") or {}
    print(json.dumps(r, ensure_ascii=False))
    if not r.get("ok") or not res.get("closed") or not res.get("held_closed"):
        raise SystemExit("REFUS : la fermeture ne tient pas (hôte sans la fermeture qui tient, ou fermeture refusée)")

def ouvrir(manifeste):
    r = post("/backend/open", {"manifest": manifeste}, session())
    print(json.dumps(r, ensure_ascii=False)[:400])
    if not r.get("ok"):
        raise SystemExit("REFUS : la réouverture a échoué")

def comparer(avant, apres, large=None):
    a, b = json.load(open(avant)), json.load(open(apres))
    c = json.load(open(large)) if large else b
    perdus_total = 0
    for q in a:
        pa, pb, pc = a[q]["cles"], b.get(q, {}).get("cles", []), c.get(q, {}).get("cles", [])
        sortis = [k for k in pa if k not in pb]
        nouveaux = [k for k in pb if k not in pa]
        perdus = [k for k in pa if k not in pc]
        perdus_total += len(perdus)
        rangs = [pc.index(k) + 1 for k in sortis if k in pc]
        print(f"« {q} » : {len(pa)} → {len(pb)} ; sortis du top {len(sortis)} (rangs dans le banc large {rangs}) ; entrés {len(nouveaux)} ; PERDUS {len(perdus)} {perdus[:5]}")
    print("VERT" if perdus_total == 0 else "ROUGE")

def hote(manifeste, requetes):
    s = session()
    out = {}
    for q in requetes:
        r = post("/call", {"manifest": manifeste, "name": "search_code", "arguments": {"query": q, "mode": "indexed", "options": {"limit": LIMITE}}}, s)
        if not r.get("ok"):
            raise SystemExit(f"« {q} » : {r.get('error')}")
        t = texte_de(r["result"])
        out[q] = {"texte": t, "cles": cles(t)}
    return out

if __name__ == "__main__":
    mode = sys.argv[1]
    if mode == "fermer":
        fermer(*sys.argv[2:5]); sys.exit(0)
    if mode == "ouvrir":
        ouvrir(sys.argv[2]); sys.exit(0)
    if mode == "comparer":
        comparer(*sys.argv[2:5]); sys.exit(0)
    manifeste, sortie, *requetes = sys.argv[2:]
    out = (autonome if mode == "autonome" else hote)(manifeste, requetes)
    json.dump(out, open(sortie, "w"), ensure_ascii=False, indent=1)
    for q, v in out.items():
        print(f"{q} : {len(v['cles'])} résultats")
