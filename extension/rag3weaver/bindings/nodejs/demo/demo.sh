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
#   DEMO_VERSION : la version attendue (défaut : ce que `npm view rag3weaver@next version` rend)
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
  # --prefer-online : sans lui, npm prend le packument de son cache local
  # (périmé de quelques minutes après une publication) et installe la
  # version d'avant — vu le 10 octobre, 0.0.1-alpha.0 au lieu de alpha.1.
  echo "━━ npm install rag3weaver@next (--prefer-online)"
  npm install --prefer-online --no-audit --no-fund rag3weaver@next 2>&1 | tail -3
fi
echo
version=$(node -p "require('rag3weaver/package.json').version")
attendue=${DEMO_VERSION:-$(npm view --prefer-online rag3weaver@next version 2>/dev/null || true)}
echo "   installé : rag3weaver $version · binaire $(node -p "require('rag3weaver').binaryPath()")"
if [ -n "$attendue" ] && [ "$version" != "$attendue" ]; then
  echo "   la version installée ($version) n'est pas celle attendue ($attendue) : on s'arrête là" >&2
  exit 3
fi
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
