#!/bin/bash
# GO2 distinct du relogement ; jamais lancé implicitement.
set -u
W=$(git rev-parse --show-toplevel) || exit 2
J=$HOME/.cache/rag3weaver-build/cx-tty-english-aliases-go2-v1
T=$HOME/.cache/rag3weaver-build/target-cx-tty-english-aliases-2
[ ! -e "$J" ] && [ ! -L "$J" ] || exit 2
mkdir -m 700 "$J" || exit 2
git rev-parse HEAD > "$J/source.txt"
git status --porcelain > "$J/status.txt"
[ ! -s "$J/status.txt" ] || exit 2
[ ! -e "$T" ] && [ ! -L "$T" ] || exit 2
mkdir -m 700 "$T" || exit 2
stat -c '%u %d %i %a' "$T" > "$J/target-initial.txt"
. "$W/extension/rag3weaver/scripts/garde_target.sh"
garder_le_target "$T" "$W/extension/rag3weaver/tty" --offline --locked > "$J/target-guard.log" 2>&1
rc=$?
printf '%s\n' "$rc" > "$J/target-guard.rc"
[ "$rc" = 0 ] || exit 2
export CARGO_TARGET_DIR="$T" CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
cd "$W/extension/rag3weaver/tty" || exit 2
printf '%s\n' 'cargo test --offline --locked -j4 --no-default-features --lib english_ -- --nocapture' > "$J/commands.argv"
cargo test --offline --locked -j4 --no-default-features --lib english_ -- --nocapture > "$J/commands.log" 2>&1
rc=$?
printf '%s\n' "$rc" > "$J/commands.rc"
[ "$rc" = 0 ] && grep -q 'test result: ok. 2 passed; 0 failed; 0 ignored;' "$J/commands.log" || exit 2
printf '%s\n' 'cargo test --offline --locked -j4 --bin rag3weaver-tty english_options_tests -- --nocapture' > "$J/options.argv"
cargo test --offline --locked -j4 --bin rag3weaver-tty english_options_tests -- --nocapture > "$J/options.log" 2>&1
rc=$?
printf '%s\n' "$rc" > "$J/options.rc"
if [ "$rc" = 101 ] && grep -q 'test result: FAILED. 0 passed; 1 failed; 0 ignored;' "$J/options.log"; then
  echo 'COMMANDES VERTES 2/2 ; OPTIONS ROUGES 0/1' | tee "$J/verdict.txt"
  exit 1
fi
echo 'STOP : résultat inattendu ou refus de préparation' | tee "$J/verdict.txt"
exit 2
