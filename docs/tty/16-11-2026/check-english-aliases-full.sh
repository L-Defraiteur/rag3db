#!/bin/bash
# Proposition F8+harnais : aucun lancement hors GO exact C7.
set -u
W=$(git rev-parse --show-toplevel) || exit 2
J=$HOME/.cache/rag3weaver-build/cx-tty-english-aliases-full-v1
T=$HOME/.cache/rag3weaver-build/target-cx-tty-english-aliases-2
LIB=rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF
[ ! -e "$J" ] && [ ! -L "$J" ] || exit 2
mkdir -m 700 "$J" || exit 2
stop() { echo "ROUGE $1" | tee "$J/verdict.txt"; exit 1; }
git rev-parse HEAD > "$J/source.txt"
git status --porcelain > "$J/status-before.txt"
[ ! -s "$J/status-before.txt" ] || stop dirty
[ -d "$T" ] && [ ! -L "$T" ] || stop target
cat "$T/.arbre-du-target" > "$J/target-owner-before.txt"
[ "$(cat "$T/.arbre-du-target")" = "$W/extension/rag3weaver/tty" ] || stop target-owner
command -v tmux > "$J/tmux.txt" || stop tmux
ss -ltn 'sport = :17990' > "$J/port-before.txt" || stop port-inspection
[ "$(wc -l < "$J/port-before.txt")" = 1 ] || stop port-busy
mkdir -m 700 "$J/cache" "$J/xdg-state" "$J/state-fixture" "$J/config" "$J/data" "$J/harness" "$J/features" || stop directories
export RAG3DB_LIBS_DIR=$HOME/.cache/rag3db-libs RAG3DB_LIB="$LIB"
export XDG_CACHE_HOME="$J/cache" XDG_STATE_HOME="$J/xdg-state" XDG_CONFIG_HOME="$J/config" XDG_DATA_HOME="$J/data"
export RAG3WEAVER_STATE_DIR="$J/state-fixture"
export CARGO_TARGET_DIR="$T" CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 CARGO_NET_OFFLINE=true
export RAG3WEAVER_TARGET_REPRENDRE=1 RAG3WEAVER_TARGET_MAX_GO=32
unset RAG3WEAVER_MOTEUR_ANCIEN RAG3WEAVER_EMBED_SERVICE RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE
cd "$W/extension/rag3weaver" || stop worktree
. scripts/lib_du_magasin.sh
exporter_la_lib > "$J/sdk-resolve.log" 2>&1 || stop sdk-resolve
sha256sum "$RAG3DB_LIBRARY_DIR/librag3db.so" "$RAG3DB_LIBRARY_DIR/libvector.rag3db_extension" > "$J/sdk-sha.txt" || stop sdk-sha
grep -q '^b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101 ' "$J/sdk-sha.txt" || stop sdk-so
grep -q '^1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561 ' "$J/sdk-sha.txt" || stop sdk-vector
export CHECK_FEATURES_LOGS="$J/features" CHECK_FEATURES_TIMEOUT=1200
printf '%s\n' 'bash scripts/check_features.sh (CARGO_NET_OFFLINE=true, real shared target guard)' > "$J/f8.argv"
bash scripts/check_features.sh > "$J/f8.log" 2>&1
rc=$?; printf '%s\n' "$rc" > "$J/f8.rc"
[ "$rc" = 0 ] && tail -1 "$J/f8.log" | grep -qx 'CHECK_FEATURES VERT 8/8' || stop f8
cd tty || stop tty
# Reprise du même target par la vraie garde pour le crate terminal.
python3 -B ../scripts/cargo_garde.py build --offline --locked -j4 --bin rag3weaver-tty > "$J/build.log" 2>&1
rc=$?; printf '%s\n' "$rc" > "$J/build.rc"
[ "$rc" = 0 ] || stop build
sha256sum "$T/debug/rag3weaver-tty" > "$J/tty-elf-sha.txt"
python3 - "$W" "$J/themes.json" <<'PY'
import json,sys
from pathlib import Path
root=Path(sys.argv[1])/'extension/rag3weaver/templates/themes'
Path(sys.argv[2]).write_text(json.dumps({p.stem:json.loads(p.read_text()) for p in sorted(root.glob('*.json'))}))
PY
python3 -B "$W/docs/tty/16-11-2026/run_private_harness.py" "$W/extension/rag3weaver/tty/mesures/mesure.py" "$T/debug/rag3weaver-tty" "$W/extension/rag3weaver/tty/mesures/faux_hote.py" "$J/harness" "$J/themes.json" "$J/harness-result.json" "$J/xdg-state" > "$J/harness.log" 2>&1
rc=$?; printf '%s\n' "$rc" > "$J/harness.rc"
[ "$rc" = 0 ] && tail -1 "$J/harness.log" | grep -qx VERT || stop harness
git -C "$W" status --porcelain > "$J/status-after.txt"
[ ! -s "$J/status-after.txt" ] || stop dirty-after
[ "$(du -sm "$T" | cut -f1)" -le 32768 ] || stop target-budget
echo 'VERT F8 8/8 et harnais tty privé entier' | tee "$J/verdict.txt"
