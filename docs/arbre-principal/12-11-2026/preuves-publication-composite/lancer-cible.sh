#!/bin/bash
set -euo pipefail
source_publication="$1"
wt=/home/lucied/.cache/rag3weaver-build/cx-wt-publication-composite
recette="$wt/docs/arbre-principal/12-11-2026/preuves-publication-composite"
preuve=/home/lucied/.cache/rag3weaver-build/cx-publication-composite-cible-v1
[ ! -e "$preuve" ] || { echo 'ARRÊT : preuves existantes, aucun rejeu automatique'; exit 2; }
cd /var/tmp
rc=0
POSTE_MEM_MAX=32G python3 "$wt/extension/rag3weaver/scripts/confiner_validation.py" lancer \
 --delai 600 --preuves /home/lucied/.cache/rag3weaver-build/cx-publication-composite-confinement-v1 \
 -- python3 "$recette/cible.py" "$source_publication" || rc=$?
python3 "$recette/finaliser-cible.py" "$source_publication" "$rc"
