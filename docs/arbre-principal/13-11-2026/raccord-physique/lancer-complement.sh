#!/bin/bash
set -euo pipefail
wt=/home/lucied/.cache/rag3weaver-build/cx-wt-raccord-physique
cd /var/tmp
python3 -c 'import time; assert time.monotonic()<887138.455654083-15,"échéance originale insuffisante"'
POSTE_MEM_MAX=32G python3 "$wt/extension/rag3weaver/scripts/confiner_validation.py" lancer --delai 100 --preuves /home/lucied/.cache/rag3weaver-build/cx-raccord-physique-complement-confinement-v3 -- python3 "$wt/docs/arbre-principal/13-11-2026/raccord-physique/cible.py" 10e6c75270a93c4db7a5d42c69c9dbd5d658f46e --existing-v3
