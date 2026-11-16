#!/bin/bash
# Témoin ciblé ; à lancer seulement au créneau accordé.
set -u
W=$(git rev-parse --show-toplevel) || exit 2
J=$HOME/.cache/rag3weaver-build/cx-tty-english-aliases-red-v1
[ ! -e "$J" ] || { echo 'STOP : dossier de preuve déjà présent'; exit 2; }
mkdir -m 700 "$J" || exit 2
git rev-parse HEAD > "$J/source.txt"
export CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/target-cx-tty-english-aliases-2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
cd "$W/extension/rag3weaver/tty" || exit 2
printf '%s\n' 'cargo test --offline --locked -j4 --no-default-features --lib english_ -- --nocapture' > "$J/command.txt"
cargo test --offline --locked -j4 --no-default-features --lib english_ -- --nocapture > "$J/cargo.log" 2>&1
rc=$?
printf '%s\n' "$rc" > "$J/rc.txt"
cat "$J/cargo.log"
# Un refus de dépendance ou de compilation n'est pas le rouge des assertions.
if [ "$rc" = 101 ] && grep -q 'running 2 tests' "$J/cargo.log" && grep -q 'test result: FAILED. 0 passed; 2 failed;' "$J/cargo.log"; then
  echo 'ROUGE OBSERVE : assertions des deux témoins' | tee "$J/verdict.txt"
  exit 1
fi
echo 'STOP : résultat à qualifier, aucun rouge métier présumé' | tee "$J/verdict.txt"
exit 2
