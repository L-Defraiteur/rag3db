#!/bin/bash
# Source publiée ; créneau, bus, délai et descendants appartiennent au superviseur.
set -euo pipefail
wt=/home/lucied/.cache/rag3weaver-build/cx-wt-raccord-physique
cd /var/tmp
POSTE_MEM_MAX=32G python3 "$wt/extension/rag3weaver/scripts/confiner_validation.py" lancer \
 --delai 599 --preuves /home/lucied/.cache/rag3weaver-build/cx-raccord-physique-confinement-v2 \
 -- python3 "$wt/docs/arbre-principal/13-11-2026/raccord-physique/cible.py" "$1"
