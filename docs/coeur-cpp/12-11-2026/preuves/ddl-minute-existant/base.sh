#!/usr/bin/env bash
set -eu
notes="$HOME/.cache/rag3db-moteur-notes/annulation"
cd "$HOME/git_workspaces/rag3db-colonnes"
git restore --source=65486c02d --worktree -- src/include/storage/index/hash_index.h src/include/storage/index/in_mem_hash_index.h src/include/storage/local_storage/local_hash_index.h src/storage/table/node_table.cpp test/storage/local_hash_index_test.cpp test/transaction/concurrence/copy_rollback_full_buffer_test.cpp test/transaction/concurrence/full_buffer_rollback_test.cpp
git diff --exit-code 65486c02d -- src test
"$HOME/.cache/rag3weaver-build/poste" lourd cmake --build build/moteur -j 8
mkdir -p "$notes/base-db"
cd "$notes/base-db"
bin="$HOME/git_workspaces/rag3db-colonnes/build/moteur/test/transaction"
set +e
for testname in 'TwentyWriters.DdlEveryMinute' 'Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64' 'Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64WithRelation'; do
 label="${testname##*/}"
 date -Is
 printf '%s\n' "$testname"
 "$HOME/.cache/rag3weaver-build/poste" lourd "$bin/concurrence/concurrence_test" "--gtest_filter=$testname" "--gtest_output=xml:$notes/base-$label.xml" > "$notes/base-$label.log" 2>&1
 printf 'code %s\n' "$?"
done
"$HOME/.cache/rag3weaver-build/poste" lourd "$bin/transaction_test" '--gtest_filter=RolledBackCopiesTest.*:RolledBackBigCopyTest.*' "--gtest_output=xml:$notes/base-plusieurs-undo.xml" > "$notes/base-plusieurs-undo.log" 2>&1
printf 'plusieurs-undo code %s\n' "$?"
