#!/bin/bash
# La démo du paquet npm, depuis un dossier VIDE :
#   1. npm install rag3weaver@next (ou l'archive donnée par DEMO_SOURCE pour répéter avant publication) ;
#   2. le backend sur un dépôt de taille modeste, AVEC les embarquements par les tunnels
#      (RAG3WEAVER_EMBED_SERVICE) : recherche hybride, usages, impact, la section Liens, describe ;
#   3. la même chose SANS service : plein texte seul, l'avertissement visible.
#
#   demo.sh [dépôt]            dépôt par défaut : extension/rag3weaver/codeparsers/src du dépôt rag3db
#   DEMO_SOURCE=/chemin/vers/rag3weaver-0.0.1-alpha.1.tgz (et le sous-paquet à côté) pour répéter sans publier
#   RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981 (défaut) — les tunnels vers luciepc
#   DEMO_QUERY, DEMO_SYMBOL : la question et le symbole montrés
set -euo pipefail

ici=$(cd "$(dirname "$0")" && pwd)
depot=${1:-$(cd "$ici/../../../codeparsers/src" 2>/dev/null && pwd || true)}
[ -n "$depot" ] && [ -d "$depot" ] || { echo "donnez le dossier d'un dépôt : demo.sh <dossier>" >&2; exit 2; }
export RAG3WEAVER_EMBED_SERVICE=${RAG3WEAVER_EMBED_SERVICE:-127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981}

vide=$(mktemp -d "${TMPDIR:-/tmp}/rag3weaver-demo-XXXXXX")
cd "$vide"
echo "━━ Dossier vide : $vide"
echo "   node $(node --version) · npm $(npm --version)"
npm init -y > /dev/null

echo
if [ -n "${DEMO_SOURCE:-}" ]; then
  sous=$(dirname "$DEMO_SOURCE")/rag3weaver-linux-x64-gnu-$(basename "$DEMO_SOURCE" | sed 's/^rag3weaver-//')
  echo "━━ npm install (répétition, depuis les archives) : $DEMO_SOURCE"
  npm install --no-audit --no-fund "$sous" "$DEMO_SOURCE" 2>&1 | tail -3
else
  echo "━━ npm install rag3weaver@next"
  npm install --no-audit --no-fund rag3weaver@next 2>&1 | tail -3
fi
echo
echo "   installé : $(node -p "require('rag3weaver/package.json').version") · binaire $(node -p "require('rag3weaver').binaryPath()")"
cp "$ici/demo.js" .

# Les tunnels sont-ils là ? Sinon, on le dit et on ne joue que la passe sans service.
premier=${RAG3WEAVER_EMBED_SERVICE%%,*}
if node -e "const n=require('node:net');const [h,p]='$premier'.split(':');const s=n.connect(+p,h,()=>{s.end();process.exit(0)});s.on('error',()=>process.exit(1));setTimeout(()=>process.exit(1),1500)"; then
  node demo.js "$depot"
else
  echo
  echo "   (pas de service d'embarquement sur $premier : la passe « avec service » est sautée)"
fi

node demo.js "$depot" sans-service

echo
echo "━━ Fin. Le dossier de la démo : $vide (à supprimer quand vous voulez)"
