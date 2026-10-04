#!/bin/bash
# e2e_code entier, en boucle, sans gdb : compte les morts par signal.
# Usage : boucle-nue.sh <étiquette> <nombre de passes>. La bibliothèque est celle de
# build/lecteurs-csv au moment du lancement.
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/borne-voisins/boucle
cd "$R/extension/rag3weaver" || exit 1
export CARGO_MANIFEST_DIR=$R/extension/rag3weaver
export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981 RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7891
export RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70 LUCIVY_SCHEDULER_THREADS=8 TMPDIR=/var/tmp
export RAG3DB_ROOT=$R LD_LIBRARY_PATH=$R/build/lecteurs-csv/src
B=$(ls -t target/debug/deps/e2e_code-* | grep -v '\.d$' | head -1)
: > "$D/nue-$1"
for i in $(seq 1 "$2"); do
  "$B" --include-ignored > "$D/nue-$1-derniere.log" 2>&1
  c=$?
  echo "passe $i code $c charge $(cut -d' ' -f1 /proc/loadavg)" >> "$D/nue-$1"
  [ $c -ge 128 ] && cp "$D/nue-$1-derniere.log" "$D/nue-$1-signal-$i.log"
done
echo "FIN" >> "$D/nue-$1"
