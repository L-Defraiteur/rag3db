#!/bin/bash
# Ticket docs/embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md
# CPU seul ou passe lib/fournisseur, sous portée commune et sans pose de service.
set -euo pipefail
ICI="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ICI"
mode=${1:-cpu}
case "$mode" in cpu|lib) ;; *) echo 'mode attendu : cpu ou lib' >&2; exit 2 ;; esac
if ! python3 scripts/confiner_validation.py verifier >/dev/null 2>&1; then
    exec python3 scripts/confiner_validation.py lancer --delai "${RAG3WEAVER_VALIDATION_TIMEOUT:-2400}" -- bash "$ICI/scripts/qualifier_activation_modeles.sh" "$mode"
fi
: "${RAG3DB_LIB:?identifiant complet de lib requis}"
: "${CARGO_TARGET_DIR:?target privé requis}"
export CARGO_BUILD_JOBS=8
. scripts/lib_du_magasin.sh
exporter_la_lib
. scripts/garde_target.sh
garder_le_target "$CARGO_TARGET_DIR" "$PWD"
git rev-parse HEAD
if [ "$mode" = cpu ]; then
    rustc --edition=2021 --test tests/activation/serveur.rs -o "$CARGO_TARGET_DIR/activation-cpu-temoin"
    "$CARGO_TARGET_DIR/activation-cpu-temoin" --nocapture
    cargo test --lib --features rag3db-native,code,daemon model_source::activation::tests -- --nocapture
    cargo test --lib --features rag3db-native,code,daemon chauffe_partielle_garde_erreurs_et_formes -- --nocapture
else
    export FEATURES=rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm
    scripts/check_features.sh
    # La mesure du modèle privé est jouée séparément par le banc.
    cargo test --lib --features "$FEATURES"
    cargo build --bin rag3weaver-embeddings --features "$FEATURES"
    scripts/temoins_navigateur.sh "$FEATURES"
fi
garder_le_target "$CARGO_TARGET_DIR" "$PWD"
