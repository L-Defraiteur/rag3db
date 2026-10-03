#!/bin/bash
# Mesure appariée du coût d'A5 bis : après, avant, après — une passe de e2e_search et une
# de e2e_mesure_ingestion_code à chaque état, la charge du poste notée avant chaque passe.
# Attend d'abord que la charge sur une minute passe sous 4.
set -u
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/a5bis/cout2
mkdir -p "$D"
: > "$D/resultats"

export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981
export RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7891
export RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70
export TMPDIR=/var/tmp RAG3DB_SHARED=1
export RAG3DB_LIBRARY_DIR=$R/build/lecteurs-csv/src RAG3DB_INCLUDE_DIR=$R/build/lecteurs-csv/src
export RAG3DB_ROOT=$R CARGO_BUILD_JOBS=6 RAG3WEAVER_NO_GC=1

charge() { cut -d' ' -f1 /proc/loadavg; }
calme() {
  local n=0
  while [ "$(charge | cut -d. -f1)" -ge 4 ] && [ $n -lt 90 ]; do
    python3 -c "import time; time.sleep(20)"; n=$((n + 1))
  done
}
bibliotheque() { # réf git
  cd "$R" || exit 1
  git checkout -q "$@" || exit 1
  cmake --build build/lecteurs-csv -j 8 --target rag3db_shared single_file_header > "$D/build.log" 2>&1 &&
    cmake --build build/lecteurs-csv -j 8 --target rag3db_vector_extension >> "$D/build.log" 2>&1
  echo "bibliothèque $* : build $?" >> "$D/resultats"
}
passe() { # étiquette
  cd "$R/extension/rag3weaver" || exit 1
  for suite in e2e_search e2e_mesure_ingestion_code; do
    calme
    local c; c=$(charge)
    local filtre=""; [ $suite = e2e_mesure_ingestion_code ] && filtre=combien_coute
    ./run_e2e.sh --test $suite $filtre > "$D/$1-$suite.log" 2>&1
    echo "$1 $suite code $? charge $c — $(grep -E '^test result' "$D/$1-$suite.log" | tr '\n' ' ')" >> "$D/resultats"
  done
}

passe apres-1
bibliotheque --detach origin/master
passe avant-1
bibliotheque a5-bis-ajout-en-memoire-sous-les-lecteurs-2
passe apres-2
echo "FIN" >> "$D/resultats"
