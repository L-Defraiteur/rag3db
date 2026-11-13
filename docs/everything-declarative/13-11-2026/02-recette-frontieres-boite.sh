#!/bin/bash
# À appeler sous confiner_validation.py lancer --delai 1800 --preuves ...
# Arguments absolus : worktree et dossier de preuves privé inexistant.
set -euo pipefail
W="$1"
O="$2"
test ! -e "$O"
mkdir -p "$O/tmp" "$O/elf"
export PATH="$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS=8
export CARGO_TARGET_DIR="$HOME/.cache/rag3weaver-build/target-cx-ed-frontieres-boite"
export TMPDIR="$O/tmp" RAG3WEAVER_STATE_DIR="$O/etat"
export RAG3WEAVER_DAEMON_DIR="$O/fiche-privee" RAG3WEAVER_JOURNAUX_DIR="$O/journaux"
unset RAG3DB_SHARED RAG3DB_LIBRARY_DIR RAG3DB_INCLUDE_DIR RAG3DB_LIB RAG3WEAVER_PG GOOGLE_APPLICATION_CREDENTIALS GOOGLE_CLOUD_PROJECT
cd "$W/extension/rag3weaver"
git rev-parse HEAD > "$O/source"
git status --porcelain > "$O/status-source"
test ! -s "$O/status-source"
# Reprise après le minimum vert : seulement si son équivalence est documentée.
case "${3:-minimum}" in
  minimum) jeux=(minimum daemon natif-daemon);;
  daemon) jeux=(daemon natif-daemon);;
  natif-daemon) jeux=(natif-daemon);;
  *) exit 2;;
esac
for nom in "${jeux[@]}"; do
  jeu=""
  if [ "$nom" = daemon ]; then jeu=daemon; fi
  if [ "$nom" = natif-daemon ]; then
    jeu=rag3db-native,daemon
    export RAG3DB_LIB=rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF
    . scripts/lib_du_magasin.sh
    exporter_la_lib > "$O/lib-native.log" 2>&1
  fi
  echo "$nom/check" > "$O/etape"
  timeout 180 cargo check --tests --no-default-features --features "$jeu" -j8 > "$O/$nom-check.log" 2>&1
  cargo tree --no-default-features --features "$jeu" -e normal --prefix none --format '{p}' > "$O/$nom-dependances.txt"
  if [ "$nom" != natif-daemon ]; then
    if rg -q '^(rag3db|codeparsers) v' "$O/$nom-dependances.txt"; then exit 9; fi
  fi
  echo "$nom/compile" > "$O/etape"
  timeout 420 cargo test --lib --no-default-features --features "$jeu" -j8 --no-run --message-format=json > "$O/$nom-build.jsonl" 2> "$O/$nom-build.log"
  python3 - "$O/$nom-build.jsonl" "$O/elf/$nom" <<'PY'
import json, pathlib, shutil, sys
paths = set()
for line in pathlib.Path(sys.argv[1]).read_text().splitlines():
    obj = json.loads(line)
    if obj.get('reason') == 'compiler-artifact' and obj.get('executable') and obj['target']['name'] == 'rag3weaver' and obj['profile']['test']:
        paths.add(obj['executable'])
assert len(paths) == 1, paths
shutil.copy2(paths.pop(), sys.argv[2])
PY
  sha256sum "$O/elf/$nom" > "$O/$nom-elf.sha256"
  "$O/elf/$nom" --list > "$O/$nom-inventaire.txt"
  echo "$nom/tests" > "$O/etape"
  if [ "$nom" = minimum ]; then
    timeout 60 "$O/elf/$nom" chat::tests::same_agent_runs_without_code_tools_and_session_roundtrips --exact --nocapture > "$O/$nom-chat.log" 2>&1
    rg -q 'test result: ok. 1 passed; 0 failed; 0 ignored;' "$O/$nom-chat.log"
  else
    timeout 60 "$O/elf/$nom" mail::tests:: --nocapture > "$O/$nom-mail.log" 2>&1
    rg -q 'test result: ok. 9 passed; 0 failed; 0 ignored;' "$O/$nom-mail.log"
    timeout 60 "$O/elf/$nom" daemon_lifecycle::tests:: --nocapture > "$O/$nom-lifecycle.log" 2>&1
    attendu=7
    if [ "$nom" = daemon ]; then attendu=8; fi
    rg -q "test result: ok. $attendu passed; 0 failed; 0 ignored;" "$O/$nom-lifecycle.log"
    if [ "$nom" = natif-daemon ]; then
      timeout 60 "$O/elf/$nom" daemon_host::livraison_codex::tests:: --nocapture > "$O/$nom-livraison.log" 2>&1
      rg -q 'test result: ok. 2 passed; 0 failed; 0 ignored;' "$O/$nom-livraison.log"
    fi
  fi
done
printf 'VERT\n' > "$O/verdict"
