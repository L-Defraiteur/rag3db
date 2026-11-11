#!/usr/bin/env python3
"""Lance un index complet (confirm) d'un backend dans l'hôte 7970 du poste, garde la session ouverte et suit le
journal jusqu'à « indexation terminée|échouée|arrêtée », en le recopiant. Usage : index-force.py <manifeste>"""
import json, os, re, sys, time, urllib.request

manifeste = sys.argv[1]
J = open(os.path.expanduser("~/.local/share/rag3weaver/daemon.token")).read().strip()

def demander(methode, route, corps=None, session=None, delai=900):
    req = urllib.request.Request("http://127.0.0.1:7970" + route, method=methode,
        data=json.dumps(corps).encode() if corps is not None else None,
        headers={"Authorization": f"Bearer {J}", "Content-Type": "application/json",
                 **({"X-Rag3weaver-Session": session} if session else {})})
    try:
        with urllib.request.urlopen(req, timeout=delai) as r:
            return json.loads(r.read() or b"{}")
    except urllib.error.HTTPError as e:
        return json.loads(e.read() or b"{}")

def dire(t):
    print(time.strftime("%H:%M:%S"), t, flush=True)

s = demander("POST", "/session", {"client": "G-index-force", "pid": os.getpid()})["session"]
try:
    o = demander("POST", "/backend/open", {"manifest": manifeste}, s)
    dire(f"ouvert : ok={o.get('ok')} {(o.get('error') or '')[:200]}")
    r = demander("POST", "/call", {"manifest": manifeste, "name": "index", "arguments": {"confirm": True}}, s)
    texte = json.dumps(r, ensure_ascii=False)
    dire("index : " + texte[:500])
    m = re.search(r'journal : ([^\\"]+\.out)', texte)
    if not m:
        sys.exit("pas de journal dans la réponse")
    journal = m.group(1)
    dire("journal : " + journal)
    vu = 0
    while True:
        time.sleep(30)
        try:
            lignes = open(journal).read().splitlines()
        except FileNotFoundError:
            dire("journal disparu"); break
        for l in lignes[vu:]:
            if re.search(r"terminée|échouée|arrêtée|erreur|échec|refus|\d+ fichiers? sur \d+", l):
                dire("  " + l[:220])
        vu = len(lignes)
        if lignes and re.match(r"^(indexation terminée|indexation échouée|indexation arrêtée)", lignes[-1]):
            break
finally:
    dire("session : " + json.dumps(demander("DELETE", "/session", None, s)))
