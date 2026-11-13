#!/bin/bash
set -euo pipefail
source_ro="$1"
wt=/home/lucied/.cache/rag3weaver-build/cx-wt-raccord-physique-union
recette="$wt/docs/arbre-principal/13-11-2026/raccord-physique"
preuve=/home/lucied/.cache/rag3weaver-build/cx-passe-raccord-physique-union-v1
[ ! -e "$preuve" ] || { echo 'ARRET : preuves existantes'; exit 2; }
cd /var/tmp
rc=0
POSTE_MEM_MAX=32G RAG3WEAVER_BUILD_MEMORY_HIGH=0 python3 "$wt/extension/rag3weaver/scripts/confiner_validation.py" lancer --delai 6000 --preuves /home/lucied/.cache/rag3weaver-build/cx-raccord-physique-global-confinement-v1 -- python3 "$recette/globale.py" "$source_ro" || rc=$?
python3 "$recette/finaliser-globale.py" "$source_ro" "$rc"
