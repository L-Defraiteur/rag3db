#!/usr/bin/env bash
set -euo pipefail
out="$HOME/.cache/rag3weaver-build/cx-dictionary-global-01"
[[ -d "$out" && ! -e "$out/resume.log" && -f "$out/recipe.json" ]]
# No CPU override: poste supplies POSTE_FILS and cpu.max after actual lock acquisition.
# entry.py reads that effective quota and applies the same bound to the private native child unit.
unset POSTE_CPU_QUOTA POSTE_FILS POSTE_LOURDS_MAX POSTE_COEURS_LIBRES
POSTE_MEM_MAX=32G POSTE_SWAP_MAX=0 exec "$HOME/.cache/rag3weaver-build/poste" lourd \
 /usr/bin/python3 "$out/entry.py" "$out"
