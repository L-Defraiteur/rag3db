#!/bin/bash
# À lancer seulement au créneau assigné, après coordination des usages de la passe.
set -euo pipefail
rate_key_root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
: "${1:?full source commit required}"
[[ "$1" =~ ^[0-9a-f]{40}$ ]] || exit 2
cd "$rate_key_root/extension/rag3weaver"
export POSTE_MEM_MAX=32G RAG3WEAVER_BUILD_MEMORY_HIGH=0
unset RAG3WEAVER_MOTEUR_ANCIEN
exec "$HOME/.cache/rag3weaver-build/poste" lourd \
    python3 scripts/confiner_validation.py lancer --mode lourd --delai 5400 -- \
    python3 "$rate_key_root/docs/embarquement/16-11-2026/rate-key/qualify.py" "$1"
