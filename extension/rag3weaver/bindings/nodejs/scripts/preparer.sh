#!/bin/bash
# Prépare les paquets à publier : le binaire et l'extension de dist/<cible>/ dans npm/<cible>/,
# et les gabarits (backends/code, tools) dans templates/. Ne publie rien.
set -euo pipefail
ici=$(cd "$(dirname "$0")/.." && pwd)
racine=$(cd "$ici/../../../.." && pwd)
for d in "$ici"/npm/*/; do
  cible=$(basename "$d")
  src="$racine/dist/$cible"
  [ -d "$src" ] || { echo "pas de dist/$cible (tools/build-images/build.sh $cible d'abord)" >&2; continue; }
  cp "$src"/rag3weaver-backend* "$d" 2>/dev/null || true
  cp "$src"/*.rag3db_extension "$d" 2>/dev/null || true
  echo "$cible : $(ls "$d" | grep -v package.json | tr '\n' ' ')"
done
rm -rf "$ici/templates"; mkdir -p "$ici/templates/backends"
cp -r "$racine/extension/rag3weaver/templates/tools" "$ici/templates/tools"
cp -r "$racine/extension/rag3weaver/templates/backends/code" "$ici/templates/backends/code"
echo "templates : $(find "$ici/templates" -type f | wc -l) fichiers"
