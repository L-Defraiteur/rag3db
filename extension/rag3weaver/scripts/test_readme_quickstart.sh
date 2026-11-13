#!/bin/bash
# **Le témoin de l'exemple du README** (« Get started ») : les mêmes commandes, telles quelles,
# sur un hôte isolé (HOME, configuration et données sous un dossier temporaire, un port à lui),
# sans service d'embarquement — ce qu'a un premier utilisateur. Un exemple de README qui n'est
# pas gardé pourrit (c7, 13 novembre 2026).
#
#   scripts/test_readme_quickstart.sh [binaire rag3weaver-backend]
#
# Le binaire doit venir de la même source que les gabarits (ceux de ce dossier). VERT ou ROUGE.
set -u
ICI="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${1:-rag3weaver-backend}"
T="$(mktemp -d "${TMPDIR:-/tmp}/readme-quickstart.XXXXXX")"
export HOME="$T/home"
unset RAG3WEAVER_EMBED_SERVICE RAG3WEAVER_SERVICE_EMBED RAG3WEAVER_CONFIG_DIR RAG3WEAVER_DATA_DIR
PORT=$(( 20000 + RANDOM % 20000 ))
ADRESSE="127.0.0.1:$PORT"
rouge() { echo "ROUGE readme-quickstart : $1"; [ -f "$T/hote.log" ] && tail -20 "$T/hote.log"; arreter; exit 1; }
arreter() {
  local jeton
  jeton=$(python3 -c "import json,sys;print(json.load(open(sys.argv[1])).get('token',''))" "$HOME/.local/share/rag3weaver/daemon.json" 2>/dev/null)
  [ -n "${HOTE:-}" ] && curl -s -m 10 -X POST -H "Authorization: Bearer $jeton" -H 'content-type: application/json' -d '{}' "http://$ADRESSE/arret" > /dev/null
  [ -n "${HOTE:-}" ] && wait "$HOTE" 2>/dev/null
}
mkdir -p "$HOME" "$T/my-repo/src"
cd "$T/my-repo" || rouge "dossier"
printf 'pub fn parse_config(path: &str) -> Config {\n    Config { raw: std::fs::read_to_string(path).unwrap() }\n}\n\npub struct Config { pub raw: String }\n' > src/lib.rs
git init -q -b main && git config user.name "Demo User" && git config user.email demo@example.org && git add -A && git commit -qm init || rouge "git"

# 1. Un hôte (le README : `rag3weaver-backend daemon`).
"$BIN" daemon --address "$ADRESSE" > "$T/hote.log" 2>&1 &
HOTE=$!
for _ in $(seq 1 100); do curl -s -m 1 "http://$ADRESSE/sante" | grep -q rag3weaver-daemon && break; sleep 0.1; done
curl -s -m 1 "http://$ADRESSE/sante" | grep -q rag3weaver-daemon || rouge "l'hôte ne répond pas"

# 2. Déclarer le dépôt et l'indexer (le README, mot pour mot, `--daemon` en plus pour le port).
cd "$ICI" || rouge "crate"
"$BIN" code-tree add "$T/my-repo" --from templates/backends/code/backend.json --daemon "$ADRESSE" > "$T/add.log" 2>&1 || { cat "$T/add.log"; rouge "code-tree add"; }
MANIFESTE="$HOME/.config/rag3weaver/code-my-repo/templates/backends/code/backend.json"
[ -f "$MANIFESTE" ] || { cat "$T/add.log"; rouge "pas de manifeste à $MANIFESTE"; }

# 3. Poser une question par le MCP (ce que fait l'agent de code) : search_code, jusqu'à 120 s.
python3 - "$BIN" "$MANIFESTE" "$ADRESSE" > "$T/mcp.log" 2>&1 <<'PY' || { cat "$T/mcp.log"; rouge "search_code par le MCP"; }
import json, subprocess, sys, time
binaire, manifeste, adresse = sys.argv[1:4]
p = subprocess.Popen([binaire, "mcp", "--manifest", manifeste, "--daemon", adresse], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
def demander(i, methode, params):
    p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": i, "method": methode, "params": params}) + "\n"); p.stdin.flush()
    while True:
        r = json.loads(p.stdout.readline())
        if r.get("id") == i:
            return r
demander(1, "initialize", {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "readme", "version": "1"}})
p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n"); p.stdin.flush()
debut, i = time.time(), 2
while time.time() - debut < 120:
    r = demander(i, "tools/call", {"name": "search_code", "arguments": {"query": "where is the config parsed"}})
    texte = json.dumps(r)
    if "parse_config" in texte:
        print(texte[:2000]); p.stdin.close(); p.wait(10); sys.exit(0)
    i += 1; time.sleep(2)
print("search_code n'a pas trouvé parse_config en 120 s :", texte[:2000]); sys.exit(1)
PY
arreter
rm -rf "$T"
echo "VERT readme-quickstart (hôte $ADRESSE)"
