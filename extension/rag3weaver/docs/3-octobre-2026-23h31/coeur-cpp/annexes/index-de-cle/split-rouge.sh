#!/bin/bash
# Le témoin de la division des cases : rouge sur le moteur sans le correctif, puis vert avec.
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/etape-4/split
mkdir -p $D; : > $D/codes
cd $R || exit 1
cmake --build build/moteur -j 8 --target local_hash_index_test > $D/build-rouge.log 2>&1; echo "build sans correctif $?" >> $D/codes
./build/moteur/test/storage/local_hash_index_test --gtest_filter='HashIndexSplit*' > $D/rouge.log 2>&1; echo "sans correctif code $? — $(grep -E '^\[  (PASSED|FAILED)  \] [0-9]+ tests?' $D/rouge.log | tr '\n' ' ')" >> $D/codes
git apply $D/../split-slots.patch; echo "patch remis $?" >> $D/codes
cmake --build build/moteur -j 8 > $D/build-vert.log 2>&1; echo "build avec correctif $?" >> $D/codes
./build/moteur/test/storage/local_hash_index_test > $D/vert.log 2>&1; echo "avec correctif code $? — $(grep -E '^\[  (PASSED|FAILED)  \] [0-9]+ tests?' $D/vert.log | tr '\n' ' ')" >> $D/codes
echo fini >> $D/codes
