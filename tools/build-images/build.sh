#!/bin/bash
# Bâtit une cible dans son image Docker : tools/build-images/build.sh <cible>
# Le dépôt est monté sur /src, les caches sur ~/.cache/rag3weaver-build/paquet-npm/docker-<cible>.
# Sur un poste qui a le verrou `poste`, le bâti passe par `poste lourd`.
set -euo pipefail
cible=${1:?cible : linux-x64-gnu}
racine=$(cd "$(dirname "$0")/../.." && pwd)
dossier=$racine/tools/build-images/$cible
[ -f "$dossier/Dockerfile" ] || { echo "pas de Dockerfile pour $cible" >&2; exit 2; }
cache=$HOME/.cache/rag3weaver-build/paquet-npm/docker-$cible
mkdir -p "$cache"
moteur=$(command -v docker || command -v podman) || { echo "ni docker ni podman sur ce poste" >&2; exit 3; }
image=rag3weaver-build-$cible
poste=$HOME/.cache/rag3weaver-build/poste
lourd=(); [ -x "$poste" ] && lourd=("$poste" lourd)
"${lourd[@]}" "$moteur" build -t "$image" "$dossier"
"${lourd[@]}" "$moteur" run --rm -v "$racine:/src" -v "$cache:/cache" "$image" /src/tools/build-images/$cible/build.sh
cat "$racine/dist/$cible/bati.txt"
