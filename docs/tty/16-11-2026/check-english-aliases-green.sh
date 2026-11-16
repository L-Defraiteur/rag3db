#!/bin/bash
# Qualification du crate seul ; harnais et features produit restent distincts.
set -u
W=$(git rev-parse --show-toplevel) || exit 2
J=$HOME/.cache/rag3weaver-build/cx-tty-english-aliases-green-v1
T=$HOME/.cache/rag3weaver-build/target-cx-tty-english-aliases-2
[ ! -e "$J" ] && [ ! -L "$J" ] || exit 2
mkdir -m 700 "$J" || exit 2
git rev-parse HEAD > "$J/source.txt"
git status --porcelain > "$J/status.txt"
[ ! -s "$J/status.txt" ] || exit 2
[ -d "$T" ] && [ ! -L "$T" ] || exit 2
[ "$(cat "$T/.arbre-du-target")" = "$W/extension/rag3weaver/tty" ] || exit 2
. "$W/extension/rag3weaver/scripts/garde_target.sh"
garder_le_target "$T" "$W/extension/rag3weaver/tty" --offline --locked > "$J/target-guard.log" 2>&1
rc=$?
printf '%s\n' "$rc" > "$J/target-guard.rc"
[ "$rc" = 0 ] || exit 2
mkdir -m 700 "$J/state" "$J/config" "$J/data" || exit 2
export XDG_STATE_HOME="$J/state" XDG_CONFIG_HOME="$J/config" XDG_DATA_HOME="$J/data"
export CARGO_TARGET_DIR="$T" CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
cd "$W/extension/rag3weaver/tty" || exit 2
for profile in vue minimal; do
  args=()
  [ "$profile" = minimal ] && args=(--no-default-features)
  printf '%s\n' "cargo test --offline --locked -j4 ${args[*]}" > "$J/$profile.argv"
  cargo test --offline --locked -j4 "${args[@]}" > "$J/$profile.log" 2>&1
  rc=$?
  printf '%s\n' "$rc" > "$J/$profile.rc"
  if [ "$rc" != 0 ] || ! grep -Eq 'test result: ok. [1-9][0-9]* passed; 0 failed; 0 ignored;' "$J/$profile.log" || grep -Eq 'test result:.*[1-9][0-9]* ignored;' "$J/$profile.log"; then
    echo "ROUGE $profile" | tee "$J/verdict.txt"
    exit 1
  fi
  grep '^test result:' "$J/$profile.log"
done
echo 'VERT crate vue et minimal (hors harnais et features produit)' | tee "$J/verdict.txt"
