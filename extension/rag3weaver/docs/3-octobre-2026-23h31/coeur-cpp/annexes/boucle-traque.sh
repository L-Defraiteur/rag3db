#!/bin/bash
# e2e_code sous AddressSanitizer, bibliothèque instrumentée (TRAQUE) et extension vector sous
# ASan ; huit passes au plus ; s'arrête au premier rapport ou à la première ligne TRAQUE.
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/asan
cd "$R/extension/rag3weaver" || exit 1
export CARGO_MANIFEST_DIR=$R/extension/rag3weaver
export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981 RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7891
export RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70 LUCIVY_SCHEDULER_THREADS=8 TMPDIR=/var/tmp
export RAG3DB_ROOT=$R LD_LIBRARY_PATH=$R/build/asan/src
B=$(ls -t target/debug/deps/e2e_code-* | grep -v '\.d$' | grep -v dwo | head -1)
A=$(realpath "$(gcc -print-file-name=libasan.so)")
: > "$D/boucle-traque"
for i in 5 6 7 8; do
  rm -f "$D"/rapport-t$i.*
  LD_PRELOAD=$A ASAN_OPTIONS=detect_leaks=0:halt_on_error=1:verify_asan_link_order=0:log_path=$D/rapport-t$i \
    "$B" --include-ignored > "$D/passe-traque-$i.log" 2>&1
  c=$?
  echo "passe $i code $c traque $(grep -ac '^TRAQUE\| TRAQUE ' "$D/passe-traque-$i.log") rapport $(ls "$D" | grep -c "^rapport-t$i\.") charge $(cut -d' ' -f1 /proc/loadavg) $(date +%H:%M)" >> "$D/boucle-traque"
  ls "$D" | grep -q "^rapport-t$i\." && break
  grep -aq 'TRAQUE asan\|TRAQUE reserve\|TRAQUE CONCURRENT' "$D/passe-traque-$i.log" && break
done
echo "FIN" >> "$D/boucle-traque"
