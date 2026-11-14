#!/bin/bash
# Le témoin de fils-du-moteur, rouge d'abord puis vert (14 nov. 2026), à luciepc sous `poste lourd`, dans
# rag3db-controle-G : la branche jetable fils-du-moteur-rouge (sans le branchement) puis fils-du-moteur.
set -u
R=$HOME/git_workspaces/rag3db-controle-G; OUT=$HOME/.cache/rag3weaver-build/fils-du-moteur; mkdir -p $OUT; : > $OUT/journal
export CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/target-G/passe
export RAG3DB_LIB=rag3db-build-0c41f9fe6731a203b330ce975c4e26e9089bf807-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF
unset RAG3WEAVER_MOTEUR_ANCIEN RAG3WEAVER_EXEC_THREADS RAG3WEAVER_DB_THREADS
for b in fils-du-moteur-rouge fils-du-moteur; do
  git -C $R fetch -q origin $b && git -C $R checkout -q --detach FETCH_HEAD && git -C $R submodule update -q || { echo "ROUGE checkout $b" >> $OUT/journal; exit 1; }
  cd $R/extension/rag3weaver && . scripts/lib_du_magasin.sh && exporter_la_lib || { echo "ROUGE lib" >> $OUT/journal; exit 1; }
  echo "▸ $(date +%T) $b $(git -C $R rev-parse --short=9 HEAD)" >> $OUT/journal
  cargo test --lib --features rag3db-native,daemon -- les_fils_du_moteur_suivent_rag3weaver_db_threads > $OUT/$b.log 2>&1; echo "  rc $? — $(grep -E 'test result|panicked|le moteur doit dire' $OUT/$b.log | tail -2 | tr '\n' ' ' | cut -c1-220)" >> $OUT/journal
done
echo fin >> $OUT/journal
