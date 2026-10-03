#!/bin/bash
# Passe Rust de la livraison du cœur C++, seconde partie : les binaires, puis les scripts
# Python (liste donnée par la session de l'arbre principal). Une ligne par script dans
# `codes2`, son journal à côté.
set -u
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/plantage-reouverture/rust
PY=/home/lucied/git_workspaces/rag3db/experiments/mtga/.venv/bin/python
mkdir -p "$D"
: > "$D/codes2"

export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981
export RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70 LUCIVY_SCHEDULER_THREADS=8
export TMPDIR=/var/tmp RAG3DB_SHARED=1
export RAG3DB_LIBRARY_DIR=$R/build/lecteurs-csv/src RAG3DB_INCLUDE_DIR=$R/build/lecteurs-csv/src
export RAG3DB_ROOT=$R CARGO_BUILD_JOBS=6
export LD_LIBRARY_PATH=$R/build/lecteurs-csv/src${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}

cd "$R" || exit 1
cargo build -q -j6 --manifest-path extension/rag3weaver/Cargo.toml \
  --features daemon,rag3db-native,openai-llm,code \
  --bin rag3weaver-backend --bin rag3weaver-chat --bin rag3daemon --example db_query \
  > "$D/binaires.log" 2>&1
echo "binaires code $?" >> "$D/codes2"

S=extension/rag3weaver/scripts
for script in test_backend_persistence test_backend_harness test_backend_lifecycle_batch \
    test_backend_snapshot test_backend_must_reopen; do
  "$PY" "$S/$script.py" > "$D/$script.log" 2>&1
  echo "$script code $? — $(tail -1 "$D/$script.log" | cut -c1-120)" >> "$D/codes2"
done
"$PY" "$S/test_migration_v8.py" /var/tmp/migration-v8/backend-v7 > "$D/test_migration_v8.log" 2>&1
echo "test_migration_v8 code $? — $(tail -1 "$D/test_migration_v8.log" | cut -c1-120)" >> "$D/codes2"
for script in test_chat_app test_chat_must_reopen test_backend_mcp_render; do
  "$PY" "$S/$script.py" > "$D/$script.log" 2>&1
  echo "$script code $? — $(tail -1 "$D/$script.log" | cut -c1-120)" >> "$D/codes2"
done
echo "FIN partie 2" >> "$D/codes2"
