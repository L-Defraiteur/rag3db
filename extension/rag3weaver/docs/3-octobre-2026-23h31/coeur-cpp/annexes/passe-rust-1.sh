#!/bin/bash
# Passe Rust de la livraison du cœur C++, première partie : la suite lib, puis les
# quinze suites e2e une à une (liste et comptes donnés par la session de l'arbre
# principal). Chaque suite écrit son journal ; `codes` reçoit une ligne par suite.
set -u
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/plantage-reouverture/rust
mkdir -p "$D"
: > "$D/codes"

export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981
export RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7891
export RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70 LUCIVY_SCHEDULER_THREADS=8
export TMPDIR=/var/tmp RAG3DB_SHARED=1
export RAG3DB_LIBRARY_DIR=$R/build/lecteurs-csv/src RAG3DB_INCLUDE_DIR=$R/build/lecteurs-csv/src
export RAG3DB_ROOT=$R CARGO_BUILD_JOBS=6

cd "$R/extension/rag3weaver" || exit 1

env -u RAG3WEAVER_EMBED_CHAR_BUDGET LD_LIBRARY_PATH=$R/build/lecteurs-csv/src \
  cargo test -j6 --lib --features rag3db-native,burn-embedder,burn-ocr,code,daemon \
  > "$D/lib.log" 2>&1
echo "lib code $? — $(grep -E '^test result' "$D/lib.log" | tr '\n' ' ')" >> "$D/codes"

for suite in e2e_synchronisation e2e_prise_atomique e2e_checkpoint e2e_undo e2e_search \
    e2e_chemin_de_masse e2e_simple_entity e2e_entites_derivees e2e_code \
    e2e_idempotent_registration e2e_phase0b e2e_generic_search e2e_rouvrir e2e_rag3daemon \
    e2e_recherche_dense_apres_suppressions e2e_estimate e2e_code_sync e2e_working_tree; do
  ./run_e2e.sh --test "$suite" > "$D/$suite.log" 2>&1
  echo "$suite code $? — $(grep -E '^test result' "$D/$suite.log" | tr '\n' ' ')" >> "$D/codes"
done
echo "FIN partie 1" >> "$D/codes"
