#!/bin/bash
# Liste C++ de livraison d'A5 bis (branche a5-bis-ajout-en-memoire-sous-les-lecteurs).
# Une ligne par suite dans `codes`, son journal à côté.
set -u
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/a5bis/liste
mkdir -p "$D"
: > "$D/codes"
cd "$R" || exit 1

total() { grep -E '^\[  (PASSED|FAILED)  \] [0-9]+ tests?' "$1" | tr '\n' ' '; }

cmake --build build/moteur -j 8 > "$D/build.log" 2>&1
echo "build $?" >> "$D/codes"

for t in test/transaction/transaction_test test/api/api_test test/c_api/c_api_test test/copy/copy_tests \
    test/storage/buffer_manager_test test/storage/column_chunk_metadata_test test/storage/compression_test \
    test/storage/local_hash_index_test test/storage/node_insertion_deletion_test test/storage/node_update_test \
    test/storage/rel_tests test/storage/string_finalize_test; do
  n=$(basename $t)
  if [ ! -x "build/moteur/$t" ]; then echo "$n ABSENT" >> "$D/codes"; continue; fi
  "./build/moteur/$t" > "$D/$n.log" 2>&1
  echo "$n code $? — $(total "$D/$n.log")" >> "$D/codes"
done

cmake -DBENCH=$R/build/moteur/test/transaction/concurrence/concurrence_test \
  -DKNOWN_RED_FILE=$R/test/transaction/concurrence/known_red.txt \
  -DPROBABILISTIC_FILE=$R/test/transaction/concurrence/probabilistic.txt \
  -DRESULT_FILE=$D/banc.json -P test/transaction/concurrence/compare_known_red.cmake > "$D/banc.log" 2>&1
echo "banc code $? — $(grep -E 'bench matches|differ' "$D/banc.log" | tr '\n' ' ')" >> "$D/codes"

for m in false true; do
  IN_MEM_MODE=$m E2E_TEST_FILES_DIRECTORY=extension/vector/test/test_files \
    ./build/moteur/test/runner/e2e_test . > "$D/vector-$m.log" 2>&1
  echo "vector IN_MEM=$m code $? — $(total "$D/vector-$m.log")" >> "$D/codes"
done

E2E_TEST_FILES_DIRECTORY=test/test_files ./build/moteur/test/runner/e2e_test . > "$D/e2e.log" 2>&1
echo "e2e complet code $? — $(total "$D/e2e.log")" >> "$D/codes"

cmake --build build/tsan -j 8 --target concurrence_test transaction_test > "$D/tsan-build.log" 2>&1
echo "tsan build $?" >> "$D/codes"
mkdir -p "$D/tsan"
python3 test/transaction/concurrence/compare_tsan_signatures.py \
  build/tsan/test/transaction/concurrence/concurrence_test \
  test/transaction/concurrence/tsan_signatures.txt "$D/tsan" --print --repeat 40 > "$D/tsan.log" 2>&1
echo "tsan signatures code $? — $(grep -E '^tsan (pass|exploration)' "$D/tsan.log" | tr '\n' ' ')" >> "$D/codes"
TSAN_OPTIONS="halt_on_error=0" ./build/tsan/test/transaction/transaction_test \
  --gtest_filter='VectorSearchDuringInsert*:ReadersDuringDelete*' > "$D/tsan-temoins.log" 2>&1
echo "tsan témoins code $? — avertissements $(grep -c 'WARNING: ThreadSanitizer' "$D/tsan-temoins.log") — $(total "$D/tsan-temoins.log")" >> "$D/codes"
echo "FIN" >> "$D/codes"
