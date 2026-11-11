#!/usr/bin/env python3
"""Ouvre des backends dans l'hôte 7970 du poste (première ouverture sous une lib neuve), lit /status de chacun :
ouvert, refusé, ses avertissements. Rien n'est écrit. Usage : ouvrir-bases.py <manifeste>…"""
import json, os, sys, urllib.request

J = open(os.path.expanduser("~/.local/share/rag3weaver/daemon.token")).read().strip()

def demander(methode, route, corps=None, session=None):
    req = urllib.request.Request("http://127.0.0.1:7970" + route, method=methode,
        data=json.dumps(corps).encode() if corps is not None else None,
        headers={"Authorization": f"Bearer {J}", "Content-Type": "application/json",
                 **({"X-Rag3weaver-Session": session} if session else {})})
    try:
        with urllib.request.urlopen(req, timeout=600) as r:
            return json.loads(r.read() or b"{}")
    except urllib.error.HTTPError as e:
        return {"http": e.code, "erreur": e.read()[:400].decode(errors="replace")}

s = demander("POST", "/session", {"client": "G-bascule-43", "pid": os.getpid()})["session"]
try:
    for m in sys.argv[1:]:
        o = demander("POST", "/backend/open", {"manifest": m}, s)
        print(f"== {m}\n   ouverture : {json.dumps(o, ensure_ascii=False)[:500]}")
    st = demander("GET", "/status", None, s)
    for b in st.get("result", st).get("backends", []):
        print(f"   {b.get('manifest')} : open={b.get('open')} refused={b.get('refused')} warnings={b.get('warnings')}")
finally:
    demander("DELETE", "/session", None, s)
