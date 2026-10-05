#!/bin/bash
set -u
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/etape-4/apres-rebase
: > "$D/codes"
cd "$R" || exit 1
total() { grep -E '^\[  (PASSED|FAILED)  \] [0-9]+ tests?' "$1" | tr '\n' ' '; }
cmake --build build/moteur -j 8 > "$D/build.log" 2>&1
echo "build $?" >> "$D/codes"
for t in test/transaction/transaction_test test/copy/copy_tests test/storage/local_hash_index_test; do
  n=$(basename $t)
  "./build/moteur/$t" > "$D/$n.log" 2>&1
  echo "$n code $? — $(total "$D/$n.log")" >> "$D/codes"
done
cmake -DBENCH=$R/build/moteur/test/transaction/concurrence/concurrence_test \
  -DKNOWN_RED_FILE=$R/test/transaction/concurrence/known_red.txt \
  -DPROBABILISTIC_FILE=$R/test/transaction/concurrence/probabilistic.txt \
  -DLONG_FILE=$R/test/transaction/concurrence/long.txt \
  -DRESULT_FILE=$D/banc.json -P test/transaction/concurrence/compare_known_red.cmake > "$D/banc.log" 2>&1
echo "banc code $? — $(grep -E 'bench matches|differ' "$D/banc.log" | tr '\n' ' ')" >> "$D/codes"
echo "stderr « rows to remove » : $(cat "$D"/*.log | grep -c 'rows to remove from a table')" >> "$D/codes"
echo fini >> "$D/codes"
