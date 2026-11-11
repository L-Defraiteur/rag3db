#!/usr/bin/env python3
"""Ouvre le backend atelier de Lucie par l'hôte 7970 et liste ses outils (le contrôle de la pose, 9 nov. 2026)."""
import json, os, urllib.request
J = open(os.path.expanduser("~/.local/share/rag3weaver/daemon.token")).read().strip()
M = os.path.expanduser("~/.config/rag3weaver/atelier/templates/backends/atelier/backend.json")
def d(m, r, c=None, s=None):
    q = urllib.request.Request("http://127.0.0.1:7970"+r, method=m, data=json.dumps(c).encode() if c is not None else None,
        headers={"Authorization": f"Bearer {J}", "Content-Type": "application/json", **({"X-Rag3weaver-Session": s} if s else {})})
    try:
        with urllib.request.urlopen(q, timeout=300) as x: return json.loads(x.read() or b"{}")
    except urllib.error.HTTPError as e: return json.loads(e.read() or b"{}")
s = d("POST", "/session", {"client": "G-outils-atelier", "pid": os.getpid()})["session"]
o = d("POST", "/backend/open", {"manifest": M}, s)
t = [x.get("name") for x in (o.get("result", {}).get("describe", {}).get("tools") or [])]
print("ouvert :", o.get("ok"), (o.get("error") or "")[:200]); print(len(t), "outils :", " ".join(sorted(t)))
neufs = ["read_file", "read_scope", "replace_text", "replace_lines", "insert_lines", "ask_permission"]
print("neufs présents :", [n for n in neufs if n in t], "manquants :", [n for n in neufs if n not in t])
d("DELETE", "/session", None, s)
