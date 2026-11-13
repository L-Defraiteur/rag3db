#!/bin/bash
# **La provenance d'un target restauré par le cache de la CI** (13 novembre 2026).
#
# Le cache de la CI (Swatinem/rust-cache) restaure target/ sans son marqueur
# `.arbre-du-target`. La garde (scripts/garde_target.sh) refuse un target non vide sans
# provenance, et la CI s'arrêtait avant tout test (runs 43447654055 et 43446953980). Ce
# script déclare la vraie provenance de ce target — « le cache de la CI », pas l'arbre
# courant —, puis la garde, sous RAG3WEAVER_TARGET_REPRENDRE=1, efface les empreintes des
# paquets locaux et note l'arbre courant. Aucune exemption : sans la reprise, un target
# déclaré ainsi est refusé comme celui d'un autre arbre, sur un poste comme en CI.
#
# Il ne fait rien sur un target absent ou vide (pas de cache), ni sur un target qui porte
# déjà un marqueur (une provenance dite ne se réécrit pas ; un marqueur invalide reste à la
# garde, qui le refuse). Le témoin : scripts/test_garde_target.sh.
#
# Usage : scripts/provenance_du_cache_ci.sh <target>
set -u
target="${1:?usage : provenance_du_cache_ci.sh <target>}"
PROVENANCE_CACHE_CI="/cache-de-la-ci/rust-cache"
[ -d "$target" ] || exit 0
marque="$target/.arbre-du-target"
if [ -e "$marque" ] || [ -L "$marque" ]; then
  exit 0
fi
[ -n "$(find "$target" -mindepth 1 -maxdepth 1 -print -quit)" ] || exit 0
echo "$PROVENANCE_CACHE_CI" > "$marque" || exit 2
echo "▸ target restauré par le cache de la CI : provenance « $PROVENANCE_CACHE_CI » notée ; la garde le reprendra (RAG3WEAVER_TARGET_REPRENDRE=1)"
