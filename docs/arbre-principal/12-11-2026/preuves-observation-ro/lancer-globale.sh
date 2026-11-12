#!/bin/bash
# Source argumentée et déjà publiée. Le superviseur commun possède poste/délai/bus.
set -euo pipefail
source_ro="$1"
wt=/home/lucied/.cache/rag3weaver-build/cx-wt-observation-faits
recette="$wt/docs/arbre-principal/12-11-2026/preuves-observation-ro"
preuve=/home/lucied/.cache/rag3weaver-build/cx-passe-observation-ro-v1
[ ! -e "$preuve" ] || { echo 'ARRÊT : preuves existantes, aucun rejeu automatique'; exit 2; }
cd /var/tmp
rc=0
POSTE_MEM_MAX=32G python3 "$wt/extension/rag3weaver/scripts/confiner_validation.py" lancer \
 --delai 3600 --preuves /home/lucied/.cache/rag3weaver-build/cx-observation-ro-global-confinement \
 -- python3 "$recette/globale.py" "$source_ro" || rc=$?
python3 "$recette/finaliser.py" "$source_ro" "$rc"
