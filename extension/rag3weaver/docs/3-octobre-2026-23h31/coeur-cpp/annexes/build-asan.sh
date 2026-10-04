#!/bin/bash
# La bibliothèque partagée et l'extension vector bâties avec AddressSanitizer, dans un dossier
# de build à part. ATTENTION : l'extension sort dans extension/vector/build/, commun à tous
# les dossiers de build de cet arbre — après cette passe, rebâtir l'extension ordinaire
# (cmake --build build/lecteurs-csv --target rag3db_vector_extension) avant tout autre test.
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/asan
cd "$R" || exit 1
cmake -S . -B build/asan -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo -DENABLE_ADDRESS_SANITIZER=ON \
  -DBUILD_EXTENSIONS=vector -DBUILD_TESTS=FALSE -DBUILD_SHELL=FALSE -DBUILD_SINGLE_FILE_HEADER=ON \
  -DBUILD_SHARED_LIBS=ON > "$D/cmake.log" 2>&1
echo "cmake $?" > "$D/etat"
cmake --build build/asan -j 8 --target rag3db_shared single_file_header > "$D/build.log" 2>&1
echo "lib $?" >> "$D/etat"
cmake --build build/asan -j 8 --target rag3db_vector_extension >> "$D/build.log" 2>&1
echo "extension $?" >> "$D/etat"
echo "FIN" >> "$D/etat"
