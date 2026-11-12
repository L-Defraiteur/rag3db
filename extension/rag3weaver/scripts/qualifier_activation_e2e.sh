#!/bin/bash
# Ticket docs/embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md
set -euo pipefail
ICI="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ICI"
mode=${1:?mode demon ou spans requis}
case "$mode" in
    demon) verrou=mesure; delai=1800; suite=e2e_demon_embeddings ;;
    spans) verrou=lourd; delai=600; suite=e2e_service_spans ;;
    *) echo 'mode demon ou spans requis' >&2; exit 2 ;;
esac
if ! python3 scripts/confiner_validation.py verifier >/dev/null 2>&1; then
    exec python3 scripts/confiner_validation.py lancer --mode "$verrou" --delai "$delai" -- bash "$ICI/scripts/qualifier_activation_e2e.sh" "$mode"
fi
: "${RAG3DB_LIB:?lib du magasin épinglée requise}"
: "${CARGO_TARGET_DIR:?target privé requis}"
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=1
export RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70
export RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0 RAG3WEAVER_DEMON_CHAUFFE=1
export RAG3WEAVER_RERANK_MODEL= RAG3WEAVER_OCR_MODEL=
unset RAG3WEAVER_ACTIVATION_PROFIL RAG3WEAVER_ACTIVATION_PERIPHERIQUE RAG3WEAVER_ACTIVATION_IDENTITE
if [ "$mode" = demon ]; then
    # Deux modèles privés successifs ; la comparaison attache/chargement est
    # exclusive. Aucun redémarrage des services installés.
    export RAG3WEAVER_MESURE=1
    unset RAG3WEAVER_SERVICE_EMBED RAG3WEAVER_EMBED_SERVICE
else
    unset RAG3WEAVER_MESURE
    export RAG3WEAVER_SERVICE_EMBED=127.0.0.1:7879
    export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7879
fi
git rev-parse HEAD
bash ./run_e2e.sh --test "$suite"
octets=$(du -sB1 "$CARGO_TARGET_DIR" | cut -f1)
if [ "$octets" -gt 34359738368 ]; then
    echo "ROUGE : target au-delà de 32 Gio après la suite ($octets octets)" >&2
    exit 1
fi
