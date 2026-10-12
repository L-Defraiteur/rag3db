#!/bin/bash
# **Un rebâti en place pendant un `cargo test --lib` ne change pas la lib
# liée** (lot 3d, page docs/11-octobre-2026-recherche/07-le-magasin-de-libs.md
# §9). Sur le vrai magasin (son `courant`) et un vrai binaire de tests :
#   1. un dossier de bâti factice, copie du courant, et l'environnement que
#      les scripts posaient jusqu'ici (RAG3DB_LIBRARY_DIR, RAG3DB_INCLUDE_DIR
#      et LD_LIBRARY_PATH sur ce dossier) ;
#   2. le binaire des tests bâti par scripts/cargo_magasin.sh ;
#   3. le rebâti en place : la lib du dossier de bâti remplacée par un fichier
#      à moitié écrit ;
#   4. lancé sur l'ancien environnement, le binaire ne démarre plus : c'est le
#      défaut que le lot corrige ;
#   5. par cargo_magasin.sh, les mêmes tests passent, sans rien recompiler.
#
# Usage, depuis extension/rag3weaver, avec CARGO_TARGET_DIR (sous `poste
# lourd`) : scripts/test_cargo_magasin.sh. Rend 0 si tout tient.
set -u
ICI="$(cd "$(dirname "$0")/.." && pwd)"
. "$ICI/scripts/lib_du_magasin.sh"
COURANT="$(lib_du_magasin courant)" || { echo "ÉCHEC  pas de courant au magasin"; exit 2; }
T="$(mktemp -d "${TMPDIR:-/tmp}/test-cargo-magasin.XXXXXX")"
trap 'rm -rf "$T"' EXIT
echec=0
cas() { if [ "$1" = 0 ]; then echo "OK     $2"; else echo "ÉCHEC  $2"; echec=1; fi; }
FEATURES=rag3db-native

mkdir -p "$T/bati/src"
cp -r "$COURANT/." "$T/bati/src/"
export RAG3DB_BUILD="$T/bati" RAG3DB_SHARED=1
export RAG3DB_LIBRARY_DIR="$T/bati/src" RAG3DB_INCLUDE_DIR="$T/bati/src" LD_LIBRARY_PATH="$T/bati/src"
unset RAG3DB_LIB RAG3WEAVER_MOTEUR_DU_WORKTREE

cd "$ICI" || exit 2
scripts/cargo_magasin.sh test --lib --features "$FEATURES" --no-run --message-format=json \
  > "$T/no-run.json" 2> "$T/no-run.err"
cas $? "le binaire des tests se bâtit par cargo_magasin.sh"
grep -q "lib du magasin, par défaut (courant)" "$T/no-run.err" && grep -q "hérité" "$T/no-run.err"
cas $? "cargo_magasin.sh dit le courant choisi et l'hérité remplacé"
BIN="$(python3 -c '
import json, sys
for l in open(sys.argv[1]):
    try: m = json.loads(l)
    except ValueError: continue
    if m.get("reason") == "compiler-artifact" and m.get("profile", {}).get("test") and m.get("executable"):
        print(m["executable"])' "$T/no-run.json" | tail -1)"
[ -x "$BIN" ]
cas $? "le binaire des tests est trouvé ($BIN)"

# Le rebâti en place, à moitié écrit.
printf 'une lib à moitié écrite' > "$T/bati/src/librag3db.so"

LD_LIBRARY_PATH="$T/bati/src" "$BIN" build_check > "$T/ancien.log" 2>&1
[ $? != 0 ]
cas $? "sur l'ancien environnement, le binaire charge le rebâti en place et ne démarre plus"

scripts/cargo_magasin.sh test --lib --features "$FEATURES" build_check > "$T/neuf.log" 2>&1
code=$?
[ $code = 0 ] && grep -q '^test result: ok' "$T/neuf.log"
cas $? "par cargo_magasin.sh, les mêmes tests passent sur la lib du magasin"
! grep -q 'Compiling rag3' "$T/neuf.log"
cas $? "et rien n'est recompilé : le rebâti du dossier de bâti ne touche pas le pont"
[ $echec = 0 ] || { echo "--- ancien"; tail -5 "$T/ancien.log"; echo "--- neuf"; tail -15 "$T/neuf.log"; }
exit $echec
