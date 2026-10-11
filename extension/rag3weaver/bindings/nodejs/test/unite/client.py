#!/usr/bin/env python3
# Un client MCP de témoin : `rag3weaver-backend mcp --manifest <m>` sur stdio,
# qui trouve (ou lance) l'hôte par daemon.json. Lit des commandes sur son
# entrée : « appel <clé> » écrit puis relit une catégorie ; « fin » ferme.
import json, subprocess, sys, os
binaire, manifeste = sys.argv[1], sys.argv[2]
p = subprocess.Popen([binaire, "mcp", "--manifest", manifeste], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
n = 0
def demander(methode, params):
    global n
    n += 1
    p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": n, "method": methode, "params": params}) + "\n"); p.stdin.flush()
    ligne = p.stdout.readline()
    if not ligne: raise SystemExit("le client MCP n'a rien répondu")
    return json.loads(ligne)
def appeler(nom, args):
    r = demander("tools/call", {"name": nom, "arguments": args})
    return r["result"].get("isError") is True, r["result"]["content"][0]["text"]
demander("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "témoin unité", "version": "0"}})
print("client : initialisé", flush=True)
for commande in sys.stdin:
    c = commande.split()
    if not c: continue
    if c[0] == "fin": break
    if c[0] == "appel":
        cle = c[1]
        e1, t1 = appeler("put_category", {"record": {"key": cle, "name": f"catégorie {cle}"}})
        e2, t2 = appeler("list_categories", {})
        ok = (not e1) and (not e2) and cle in t2
        print(f"client : appel {cle} → {'ok' if ok else 'ÉCHEC'}{'' if ok else ' : ' + (t1 if e1 else t2)[:300]}", flush=True)
p.stdin.close(); p.wait(timeout=30)
print("client : fermé", flush=True)
