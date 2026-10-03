#!/bin/bash
# Coût d'A5 sur les chemins de rag3weaver : une suite d'ingestion (vrai modèle, carte
# d'ici, régime doux) et une suite de recherche, deux passes chacune, en temps mur.
# Usage : mesure-cout.sh <avant|apres>. La bibliothèque de build/lecteurs-csv doit être
# celle de l'état mesuré. Une ligne par passe dans `cout-<état>`.
set -u
ETAT=$1
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/a5bis/cout
mkdir -p "$D"
: > "$D/cout-$ETAT"

export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981
export RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7891
export RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70
export TMPDIR=/var/tmp RAG3DB_SHARED=1
export RAG3DB_LIBRARY_DIR=$R/build/lecteurs-csv/src RAG3DB_INCLUDE_DIR=$R/build/lecteurs-csv/src
export RAG3DB_ROOT=$R CARGO_BUILD_JOBS=6 RAG3WEAVER_NO_GC=1

cd "$R/extension/rag3weaver" || exit 1

passe() { # nom, journal, commande…
  local nom=$1 journal=$2; shift 2
  local debut fin
  debut=$(date +%s%N)
  "$@" > "$journal" 2>&1
  local code=$?
  fin=$(date +%s%N)
  echo "$nom code $code mur $(( (fin - debut) / 1000000 )) ms — $(grep -E '^test result' "$journal" | tr '\n' ' ')" >> "$D/cout-$ETAT"
}

# Une passe à blanc par suite : elle compile, et son temps ne compte pas.
passe "search (à blanc)" "$D/$ETAT-search-0.log" ./run_e2e.sh --test e2e_search
passe "ingestion (à blanc)" "$D/$ETAT-ingestion-0.log" ./run_e2e.sh --test e2e_mesure_ingestion_code combien_coute
for n in 1 2; do
  passe "search $n" "$D/$ETAT-search-$n.log" ./run_e2e.sh --test e2e_search
  passe "ingestion $n" "$D/$ETAT-ingestion-$n.log" ./run_e2e.sh --test e2e_mesure_ingestion_code combien_coute
done
echo "FIN" >> "$D/cout-$ETAT"
