#!/usr/bin/env bash
set -euo pipefail
out="$HOME/.cache/rag3weaver-build/cx-reference-strings-01"
[[ -d "$out" && ! -e "$out/resume.log" && ! -e "$out/raw" && -f "$out/recipe.json" ]]
unset POSTE_CPU_QUOTA POSTE_FILS POSTE_LOURDS_MAX POSTE_COEURS_LIBRES
POSTE_MEM_MAX=32G POSTE_SWAP_MAX=0 exec "$HOME/.cache/rag3weaver-build/poste" lourd \
 /usr/bin/timeout --signal=TERM --kill-after=10s 890s /usr/bin/python3 "$out/entry.py" "$out"
