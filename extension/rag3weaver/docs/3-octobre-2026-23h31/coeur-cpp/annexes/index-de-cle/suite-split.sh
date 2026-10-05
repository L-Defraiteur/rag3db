#!/bin/bash
# Division des cases + annulation : ASan puis TSan sur les témoins, extension ordinaire remise,
# liste complète, puis la recette de l'arbre principal (ses uuids, 2 × 50 000) six fois.
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/etape-4
L=/home/lucied/.cache/rag3db-moteur-notes/copy-journalise
E="$D/suite-split-etat"
F='RolledBackBigCopyTest.*:RolledBackCopiesTest.*:*AfterFailedCopy*'
cg=/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup)
total() { grep -E '^\[  (PASSED|FAILED)  \] [0-9]+ tests?' "$1" | tr '\n' ' '; }
cd "$R" || exit 1
: > "$E"
cmake --build build/asan -j 8 --target transaction_test copy_tests local_hash_index_test > "$D/asan-build-s.log" 2>&1; echo "asan build $?" >> "$E"
rm -f extension/vector/build/libvector.rag3db_extension
cmake --build build/asan -j 8 --target rag3db_vector_extension >> "$D/asan-build-s.log" 2>&1
A=$(realpath "$(gcc -print-file-name=libasan.so)")
for b in transaction_test copy_tests local_hash_index_test; do
  B=$(find build/asan -name $b -type f | head -1)
  f="$F"
  [ $b = copy_tests ] && f='CopyTest.RowCountIsRight*:CopyTest.*BMException*:CopyTest.OutOfMemory*'
  [ $b = local_hash_index_test ] && f='HashIndexSplitTest.NoKeyIsLostWhenTheIndexGrowsFivefold:HashIndexSplitTest.NoKeyIsLostWhenInsertsGrowTheIndex:LocalHashIndex*'
  LD_PRELOAD=$A LD_LIBRARY_PATH=$R/build/asan/src ASAN_OPTIONS=detect_leaks=0:verify_asan_link_order=0 timeout 3000 "$B" --gtest_filter="$f" > "$D/asan-s-$b.log" 2>&1
  echo "asan $b $? — $(total "$D/asan-s-$b.log") ; rapports $(grep -c 'ERROR: AddressSanitizer' "$D/asan-s-$b.log")" >> "$E"
done
ASAN_OPTIONS=detect_leaks=0 LD_LIBRARY_PATH=$R/build/asan/src timeout 1500 "$D/essai-reel-asan" "$D/absent/sans-tx/base.rag3db" "$D/reel/a.rag3db" 2 STRING > "$D/reel/asan-s.log" 2>&1
echo "asan recette arbre principal $? — $(grep -a 'introuvables' "$D/reel/asan-s.log" | tr '\n' ' ') ; rapports $(grep -c 'ERROR: AddressSanitizer' "$D/reel/asan-s.log")" >> "$E"
rm -f extension/vector/build/libvector.rag3db_extension
cmake --build build/moteur -j 8 --target rag3db_vector_extension > "$D/ext-s.log" 2>&1; echo "extension ordinaire $?" >> "$E"
cmake --build build/tsan -j 8 --target transaction_test local_hash_index_test > "$D/tsan-build-s.log" 2>&1; echo "tsan build $?" >> "$E"
B=$(find build/tsan -name transaction_test -type f | head -1)
MAX_DB_SIZE=68719476736 TSAN_OPTIONS=halt_on_error=0 timeout 3000 "$B" --gtest_filter="$F" > "$D/tsan-s.log" 2>&1
echo "tsan transaction_test $? — $(total "$D/tsan-s.log") ; rapports $(grep -c 'WARNING: ThreadSanitizer' "$D/tsan-s.log") ; mmap refusés $(grep -c 'Mmap for size' "$D/tsan-s.log")" >> "$E"
B=$(find build/tsan -name local_hash_index_test -type f | head -1)
MAX_DB_SIZE=68719476736 TSAN_OPTIONS=halt_on_error=0 timeout 3000 "$B" --gtest_filter='HashIndexSplitTest.NoKeyIsLostWhenTheIndexTriples' > "$D/tsan-s-index.log" 2>&1
echo "tsan index $? — $(total "$D/tsan-s-index.log") ; rapports $(grep -c 'WARNING: ThreadSanitizer' "$D/tsan-s-index.log") ; mmap refusés $(grep -c 'Mmap for size' "$D/tsan-s-index.log")" >> "$E"
"$L/liste-cpp.sh"
cat "$L/liste/codes" >> "$E"
grep -a "FAILED  \]" "$L/liste"/*.log | sed 's/ (.*//' | sort -u | head -10 | cut -c1-200 >> "$E"
grep -a "differ\|only in\|Segmentation" "$L/liste/banc.log" | head -8 | cut -c1-220 >> "$E"
echo "stderr « rows to remove » : $(cat "$L/liste"/*.log | grep -ac 'rows to remove from a table')" >> "$E"
for i in 1 2 3 4 5 6; do
  LD_LIBRARY_PATH=$R/build/moteur/src timeout 900 "$D/essai-reel-neuf" "$D/absent/sans-tx/base.rag3db" "$D/reel/n.rag3db" 2 STRING 2>&1 | tr '\n' ' ' >> "$E"; echo >> "$E"
done
echo "pic $(( $(cat "$cg/memory.peak") / 1048576 )) Mio ; $(grep oom_kill "$cg/memory.events" | tr '\n' ' ')" >> "$E"
echo fini >> "$E"
