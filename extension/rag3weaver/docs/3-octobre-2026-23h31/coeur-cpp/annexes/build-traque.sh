#!/bin/bash
# Rebâtit la bibliothèque ASan instrumentée et l'extension vector SOUS ASan (forcée).
R=/home/lucied/git_workspaces/rag3db-moteur
D=/home/lucied/.cache/rag3db-moteur-notes/asan
cd "$R" || exit 1
cmake --build build/asan -j 8 --target rag3db_shared > "$D/build-traque.log" 2>&1
echo "lib $?" > "$D/etat-traque"
rm -f extension/vector/build/libvector.rag3db_extension
cmake --build build/asan -j 8 --target rag3db_vector_extension >> "$D/build-traque.log" 2>&1
echo "extension $?" >> "$D/etat-traque"
ls -la extension/vector/build/ >> "$D/etat-traque"
echo FIN >> "$D/etat-traque"
