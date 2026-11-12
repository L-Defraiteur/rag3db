#!/usr/bin/env bash
set -u
notes="$HOME/.cache/rag3db-moteur-notes/annulation"
cd "$notes/suites-db" || exit 2
bin="$HOME/git_workspaces/rag3db-colonnes/build/moteur/test/transaction/concurrence/concurrence_test"
for testname in 'TwentyWriters.DdlEveryFifteenSeconds' 'TwentyWriters.DdlEveryMinute' 'Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64' 'Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64WithRelation'; do
 label="${testname##*/}"
 date -Is
 printf '%s\n' "$testname"
 "$HOME/.cache/rag3weaver-build/poste" lourd "$bin" "--gtest_filter=$testname" "--gtest_output=xml:$notes/isole-$label.xml" > "$notes/isole-$label.log" 2>&1
 printf 'code %s\n' "$?"
done
