#!/bin/bash
N=~/.cache/rag3weaver-notes-A/lib58
W=~/git_workspaces/rag3db-instable
T=$N/v-etat
cd $W || exit 9
[ -z "$(git status --short --untracked-files=no | grep -v codeparsers)" ] || { echo "ARRÊT sale" > $T; exit 8; }
git fetch -q origin master && git checkout -q --detach FETCH_HEAD && git submodule update -q || { echo "ARRÊT checkout" > $T; exit 7; }
echo "$(date +%H:%M:%S) début, sources $(git rev-parse --short HEAD), lib $(cat $N/id | cut -c1-23)" > $T
cd extension/rag3weaver
unset RAG3WEAVER_LINK_COPY_THRESHOLD RAG3WEAVER_MOTEUR_ANCIEN
export CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-A-instable CARGO_INCREMENTAL=0
export RAG3DB_LIB=$(cat $N/id)
export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880 RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70
timeout 1h ~/.cache/rag3weaver-build/poste lourd ./run_e2e.sh --test e2e_code > $N/e2e_code.log 2>&1
rc=$?
echo "e2e_code rc=$rc : $(grep -m1 'moteur lié.*librag' $N/e2e_code.log | grep -o 'rag3db-build-[0-9a-f]\{9\}') $(grep -E '^running [0-9]+ test' $N/e2e_code.log | tail -1) $(grep 'test result' $N/e2e_code.log | tail -1 | cut -c1-70)" >> $T
grep -E '^test .* FAILED$|SKIPPED|⚠' $N/e2e_code.log | head -10 >> $T
echo fin >> $T
