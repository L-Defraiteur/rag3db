#!/bin/bash
# luciepc : le binaire de l'hôte de Lucie (et rag3db-check-lib) sur un master donné, lié par RUNPATH à la lib
# `courant` du magasin (même chemin et même md5 que sur le portable). Usage : bati-lucie-luciepc.sh <id de lib> <hash de master>
set -u
W=$HOME/git_workspaces/rag3db-controle-G
# La lib annoncée : `courant`, ou explicitement (LIB_EXPLICITE=1) une lib prouvée du magasin qui ne l'est pas encore.
if [ -n "${LIB_EXPLICITE:-}" ]; then
  ID=$1; [ -f $HOME/.cache/rag3db-libs/$ID/librag3db.so ] || { echo "$ID n'est pas au magasin : rien bâti"; exit 3; }
else
  ID=$(basename "$(readlink -f $HOME/.cache/rag3db-libs/courant)")
  [ "$ID" = "$1" ] || { echo "courant est $ID, pas $1 : rien bâti"; exit 3; }
fi
L=$HOME/.cache/rag3db-libs/$ID
git -C $W fetch -q origin master || exit 2
# Le commit annoncé, qui doit être dans master (la tête ou un ancêtre) : jamais une branche à côté.
git -C $W merge-base --is-ancestor "$2" FETCH_HEAD || { echo "$2 n'est pas dans origin/master : rien bâti"; exit 3; }
git -C $W checkout -q --detach "$2" && git -C $W submodule update -q extension/rag3weaver/codeparsers || exit 2
# **La borne de la recherche** (8 nov. 2026, 0e528854a, dense-filtre-sous-l6) : à partir de ce commit, la recherche
# filtrée lit sous l'instantané et exige la projection sous READ ONLY (lib dadbf0041 ou plus récente). Jamais sur 17a094d12.
if git -C $W merge-base --is-ancestor 0e528854a "$2" 2>/dev/null; then
  case "$ID" in *17a094d12*) echo "$2 contient 0e528854a (dense filtré sous L6) : il veut une lib qui projette sous READ ONLY, pas $ID — rien bâti"; exit 3 ;; esac
fi
echo "== $(date +%H:%M:%S) tête $(git -C $W log --oneline -1 | cut -c1-12) lib $ID (md5 $(md5sum $L/librag3db.so | cut -c1-8))"
cd $W/extension/rag3weaver
echo "crates lucivy dans le lock : $(grep -oE 'lucivy\?rev=[0-9a-f]+' Cargo.lock | sort | uniq -c | tr '\n' ' ')"
[ -z "${LUCIVY_ATTENDU:-}" ] || [ "$(grep -c "lucivy?rev=$LUCIVY_ATTENDU" Cargo.lock)" = 14 ] || { echo "le lock n'a pas les 14 crates lucivy à $LUCIVY_ATTENDU : rien bâti"; exit 3; }
export RAG3DB_LIB=$ID RUSTFLAGS="-C link-args=-Wl,-rpath,$L" CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/target-G/hote-lucie CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=${JOBS:-12}
bash scripts/cargo_magasin.sh build --release --bin rag3weaver-backend --bin rag3db-check-lib --features rag3db-native,code,daemon,openai-llm > $HOME/.cache/rag3weaver-build/bati-lucie.log 2>&1 || { echo "bâti rouge"; tail -15 $HOME/.cache/rag3weaver-build/bati-lucie.log; exit 4; }
X=$CARGO_TARGET_DIR/release/rag3weaver-backend; C=$CARGO_TARGET_DIR/release/rag3db-check-lib
readelf -d $X | grep -i runpath
$C $X $L/librag3db.so; echo "check-lib : $?"
echo "openai_llm : $(nm -C $X | grep -c 'rag3weaver::openai_llm::') ; finish_commit : $(nm -C $X | grep -c 'ShardActor.*finish_commit')"
md5sum $X $C | cut -c1-8
# Le binaire de tests de la lib, du même commit et contre la même lib : le témoin de l'IDE de Lucie.
if [ -n "${TESTS_LIB:-}" ]; then
  CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/target-G/hote-lucie-tests RUSTFLAGS= bash scripts/cargo_magasin.sh test --lib --features rag3db-native,code,daemon,openai-llm,navigateur --no-run > $HOME/.cache/rag3weaver-build/bati-lucie-tests.log 2>&1 || { echo "bâti des tests rouge"; tail -5 $HOME/.cache/rag3weaver-build/bati-lucie-tests.log; exit 4; }
  T=$(grep -oE '/[^ ]*/deps/rag3weaver-[0-9a-f]+' $HOME/.cache/rag3weaver-build/bati-lucie-tests.log | tail -1)
  echo "binaire de tests : $T ($(md5sum $T | cut -c1-8))"
fi
echo "== $(date +%H:%M:%S) fini"
