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
verifier_taille() {
    local octets
    octets=$(du -sB1 "$CARGO_TARGET_DIR" | cut -f1)
    if [ "$octets" -gt 34359738368 ]; then
        echo "ROUGE : target au-delà de 32 Gio ($octets octets), aucune suppression automatique" >&2
        return 1
    fi
}
verifier_taille
git rev-parse HEAD
if [ "$mode" = cpu ]; then
    rustc --edition=2021 --test tests/activation/serveur.rs -o "$CARGO_TARGET_DIR/activation-cpu-temoin"
    "$CARGO_TARGET_DIR/activation-cpu-temoin" --nocapture
    cargo test --lib --features rag3db-native,code,daemon model_source::activation::tests -- --nocapture
    cargo test --lib --features rag3db-native,code,daemon chauffe_partielle_garde_erreurs_et_formes -- --nocapture
else
    export FEATURES=rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm
    timeout -k 10s 2400 bash scripts/check_features.sh
    verifier_taille
    # La mesure du modèle privé est jouée séparément par le banc.
    timeout -k 10s 1800 cargo test --lib --features "$FEATURES"
    verifier_taille
    timeout -k 10s 1200 cargo build --bin rag3weaver-embeddings --example banc_residence --features "$FEATURES"
    verifier_taille
    timeout -k 10s 900 bash scripts/temoins_navigateur.sh "$FEATURES"
fi
garder_le_target "$CARGO_TARGET_DIR" "$PWD"
verifier_taille
