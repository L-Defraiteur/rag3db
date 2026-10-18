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
# Un worktree n'a qu'un fichier .git qui pointe vers le dossier git commun : monté seul, le
# conteneur ne lit pas le commit et l'identifiant de bâti tombe à « nogit » (vu le 18 octobre
# 2026, archive refusée). Le dossier commun est donc monté aussi, en lecture, au même chemin.
git_commun=()
if [ -f "$racine/.git" ]; then
  commun=$(git -C "$racine" rev-parse --path-format=absolute --git-common-dir)
  git_commun=(-v "$commun:$commun:ro")
fi
"${lourd[@]}" "$moteur" run --rm -v "$racine:/src" "${git_commun[@]}" -v "$cache:/cache" "$image" /src/tools/build-images/$cible/build.sh
cat "$racine/dist/$cible/bati.txt"
