#!/bin/bash
set -euo pipefail
wt=/home/lucied/.cache/rag3weaver-build/cx-wt-publication-composite
cd /var/tmp
POSTE_MEM_MAX=32G python3 "$wt/extension/rag3weaver/scripts/confiner_validation.py" lancer \
 --delai 600 --preuves /home/lucied/.cache/rag3weaver-build/cx-publication-composite-confinement-rouge-v5 \
 -- python3 "$wt/docs/arbre-principal/12-11-2026/preuves-publication-composite/rouge.py" "$1"
