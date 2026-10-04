#!/bin/bash
# e2e_code sous gdb, en boucle, contre la bibliothèque de mon arbre (garde 2 + borne) :
# s'arrête au premier signal et garde la pile. Une ligne par passe dans `gdb-boucle`.
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/borne-voisins/boucle
cd "$R/extension/rag3weaver" || exit 1
export CARGO_MANIFEST_DIR=$R/extension/rag3weaver
export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981 RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7891
export RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70 LUCIVY_SCHEDULER_THREADS=8 TMPDIR=/var/tmp
export RAG3DB_ROOT=$R LD_LIBRARY_PATH=$R/build/lecteurs-csv/src
B=$(ls -t target/debug/deps/e2e_code-* | grep -v '\.d$' | head -1)
: > "$D/gdb-boucle"
for i in $(seq 1 60); do
  gdb -q -batch -ex "handle SIGPIPE nostop noprint pass" -ex "handle SIGUSR1 nostop noprint pass" \
    -ex run -ex "bt 30" -ex "info threads" --args "$B" --include-ignored > "$D/gdb-$i.log" 2>&1
  if grep -q "received signal SIG\(SEGV\|ABRT\|BUS\)" "$D/gdb-$i.log"; then
    echo "passe $i : SIGNAL" >> "$D/gdb-boucle"; break
  fi
  echo "passe $i : $(grep -E '^test result' "$D/gdb-$i.log" | cut -c1-50)" >> "$D/gdb-boucle"
done
echo "FIN" >> "$D/gdb-boucle"
