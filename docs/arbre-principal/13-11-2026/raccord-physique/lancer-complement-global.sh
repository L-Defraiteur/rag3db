#!/bin/bash
set -euo pipefail
source_ro="$1"
source_wt=/home/lucied/.cache/rag3weaver-build/cx-wt-raccord-physique-union
controle=/home/lucied/.cache/rag3weaver-build/cx-wt-qualification-physique/docs/arbre-principal/13-11-2026/raccord-physique
preuve=/home/lucied/.cache/rag3weaver-build/cx-passe-raccord-physique-union-complement-v1
[ ! -e "$preuve" ] || { echo 'ARRET : preuves existantes'; exit 2; }
cd /var/tmp
rc=0
POSTE_MEM_MAX=32G RAG3WEAVER_BUILD_MEMORY_HIGH=0 "$source_wt/extension/rag3weaver/scripts/poste" lourd python3 "$controle/lancer-complement.py" "$source_ro" || rc=$?
python3 "$controle/finaliser-complement.py" "$source_ro" "$rc"
