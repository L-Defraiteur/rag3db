#!/usr/bin/env python3
# Un client MCP minimal : initialize, tools/list, list_files, daemon_status ; imprime l'état.
import json, subprocess, sys
p = subprocess.Popen([sys.argv[1], "mcp", "--manifest", sys.argv[2]], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
n = 0
def demander(m, pa):
    global n; n += 1
    p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": n, "method": m, "params": pa}) + "\n"); p.stdin.flush()
    ligne = p.stdout.readline()
    if not ligne: raise SystemExit("le client MCP n'a rien répondu")
    return json.loads(ligne)
demander("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "mcp de zéro", "version": "0"}})
outils = [t["name"] for t in demander("tools/list", {})["result"]["tools"]]
print("outils :", ", ".join(outils))
l = demander("tools/call", {"name": "list_files", "arguments": {}})
print("list_files :", json.dumps(l.get("result", l))[:300])
r = demander("tools/call", {"name": "daemon_status", "arguments": {}})
print(r["result"]["content"][0]["text"])
p.stdin.close(); p.wait(timeout=30)
