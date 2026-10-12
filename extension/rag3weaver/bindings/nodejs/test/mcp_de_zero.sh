#!/bin/bash
# La recette MCP de zéro, depuis les archives seules, dans un dossier vierge :
# npm install, un manifeste de code tiré des gabarits du paquet, puis le
# client MCP du binaire installé (`rag3weaver-backend mcp`) qui lance l'hôte
# lui-même (le premier client) ; daemon_status le montre. Rien de l'arbre :
# environnement propre (HOME, PATH seulement), aucun chemin vers le dépôt.
#   test/mcp_de_zero.sh <archive de tête .tgz> <archive de la plateforme .tgz>
set -u
TETE=$(realpath "$1"); SOUS=$(realpath "$2")
ICI=$(cd "$(dirname "$0")" && pwd)
V=$(mktemp -d "${TMPDIR:-/tmp}/rag3weaver-mcp-zero-XXXXXX"); cd "$V" || exit 1
echo "== dossier vierge $V"
env -i HOME="$HOME" PATH="$PATH" npm init -y > /dev/null
env -i HOME="$HOME" PATH="$PATH" npm install --no-audit --no-fund "$SOUS" "$TETE" 2>&1 | tail -1
B=$(env -i HOME="$HOME" PATH="$PATH" node -p "require('rag3weaver').binaryPath()")
echo "binaire : $B"
case "$B" in "$V"/node_modules/*) ;; *) echo "✗ le binaire ne vient pas de node_modules"; exit 1 ;; esac
mkdir -p ws && printf 'fn main() {\n    println!("bonjour");\n}\n' > ws/main.rs
env -i HOME="$HOME" PATH="$PATH" node -e "
const fs = require('fs'), path = require('path');
const { prepareManifest, templatesDir } = require('rag3weaver');
const m = prepareManifest(path.join(templatesDir(), 'backends', 'code', 'backend.json'), {
  // vector_extension laissé tel que livré (« vector ») : le binaire trouve
  // l'extension à côté de lui, par le nom (1a1ee04b0).
  name: 'zero', database: path.resolve('base.rag3db'),
  workspace: { root: path.resolve('ws'), commands: 'off', index: 'code' }, tools: { run_command: undefined },
});
fs.writeFileSync('backend.json', JSON.stringify(m, null, 2));
"
D=$V/demon
env -i HOME="$HOME" PATH="$PATH" RAG3WEAVER_DAEMON_DIR="$D" RAG3WEAVER_DAEMON_ADDRESS=127.0.0.1:0 \
  python3 "$ICI/mcp_status.py" "$B" "$V/backend.json" > client.log 2> client.err
code=$?
cat client.log; sed 's/^/  stderr client : /' client.err | head -4
ROUGE=0
verdict() { if eval "$2"; then echo "  ✓ $1"; else echo "  ✗ $1"; ROUGE=1; fi; }
PID=$(python3 -c "import json;print(json.load(open('$D/daemon.json'))['pid'])" 2>/dev/null || echo 0)
verdict "le client a répondu" '[ $code = 0 ]'
verdict "le premier client a lancé l'hôte lui-même" 'grep -q "lancement de" client.err'
verdict "daemon_status montre l'hôte (running, son pid)" 'grep -q "\"state\": \"running\"" client.log && grep -q "\"pid\": $PID" client.log'
verdict "l'hôte est le binaire de node_modules" '[ "$(readlink /proc/$PID/exe)" = "$B" ]'
verdict "le backend de code répond dans l'hôte (list_files voit main.rs)" 'grep -q "main.rs" client.log'
[ "$PID" != 0 ] && kill -TERM "$PID" && for _ in $(seq 1 50); do kill -0 "$PID" 2>/dev/null || break; sleep 0.2; done
verdict "l'hôte s'arrête proprement (SIGTERM, daemon.json retiré)" '! kill -0 $PID 2>/dev/null && [ ! -e "$D/daemon.json" ]'
[ "$ROUGE" = 0 ] && echo "== tout vert" || echo "== ROUGE"
exit $ROUGE
