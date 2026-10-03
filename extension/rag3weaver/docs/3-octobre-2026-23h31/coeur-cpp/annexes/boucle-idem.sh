#!/bin/bash
# e2e_idempotent_registration en boucle : vingt passes sur la bibliothèque de master,
# puis vingt avec le correctif du DROP au rejeu. Une ligne par passe dans `boucle`.
set -u
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/drop-index-rejeu/boucle
mkdir -p "$D"; : > "$D/boucle"
export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981 RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7891
export RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70 LUCIVY_SCHEDULER_THREADS=8
export TMPDIR=/var/tmp RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR=$R/build/lecteurs-csv/src RAG3DB_INCLUDE_DIR=$R/build/lecteurs-csv/src
export RAG3DB_ROOT=$R CARGO_BUILD_JOBS=6 RAG3WEAVER_NO_GC=1
bibliotheque() {
  cd "$R" || exit 1
  git checkout -q "$@" || exit 1
  cmake --build build/lecteurs-csv -j 8 --target rag3db_shared single_file_header > "$D/build.log" 2>&1 &&
    cmake --build build/lecteurs-csv -j 8 --target rag3db_vector_extension >> "$D/build.log" 2>&1
  echo "bibliothèque $* : build $?" >> "$D/boucle"
}
boucle() {
  cd "$R/extension/rag3weaver" || exit 1
  for i in $(seq 1 20); do
    ./run_e2e.sh --test e2e_idempotent_registration > "$D/$1-$i.log" 2>&1
    echo "$1 $i code $? — $(grep -E '^test result' "$D/$1-$i.log" | cut -c1-48) lecture-au-delà=$(grep -c 'Reading past the end' "$D/$1-$i.log")" >> "$D/boucle"
  done
}
bibliotheque --detach origin/master
boucle master
bibliotheque drop-d-index-au-rejeu-2
boucle correctif
echo "FIN" >> "$D/boucle"
