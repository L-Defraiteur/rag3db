#!/bin/bash
# Témoin : le pont cxx (crate rag3db, tools/rust_api) se recompile quand la
# lib ou l'en-tête qu'il emploie change — un autre dossier d'en-têtes, ou
# l'en-tête modifié en place. Sans cela, un pont compilé contre un vieil
# en-tête se lie à une lib neuve (l'incident B2 du 11 octobre 2026, page
# 07-le-magasin-de-libs §3.1).
#
# Usage, depuis extension/rag3weaver, avec RAG3DB_LIBRARY_DIR et
# RAG3DB_INCLUDE_DIR posés sur un bâti partagé, et CARGO_TARGET_DIR :
#   scripts/test_pont_recompile.sh
# Rend 0 si les trois cas tiennent, 1 sinon ; chaque cas écrit OK ou ÉCHEC.
set -u
: "${RAG3DB_INCLUDE_DIR:?le dossier des en-têtes du bâti}"
: "${RAG3DB_LIBRARY_DIR:?le dossier de la lib du bâti}"
ICI="$(mktemp -d "${TMPDIR:-/tmp}/test-pont.XXXXXX")"
trap 'rm -rf "$ICI"' EXIT
JOURNAL="$ICI/cargo.log"
echec=0
cas() { if [ "$1" = 0 ]; then echo "OK     $2"; else echo "ÉCHEC  $2"; echec=1; fi; }
verifier() { # le check, écrit dans un fichier ; rend 0 si la crate rag3db a été recompilée
  cargo check --lib --features rag3db-native > "$JOURNAL" 2>&1 || { echo "  cargo check a échoué :"; tail -5 "$JOURNAL"; return 2; }
  grep -q 'Compiling rag3db v' "$JOURNAL"
}

# Point de départ : le pont à jour contre le bâti donné.
cargo check --lib --features rag3db-native > "$JOURNAL" 2>&1 || { echo "cargo check de départ en échec"; tail -5 "$JOURNAL"; exit 1; }

# 0. Rien ne change : le pont ne se recompile pas (le correctif ne coûte rien).
verifier
code=$?
[ $code = 1 ]
cas $? "rien ne change : le pont ne se recompile pas"

# 1. Un autre dossier d'en-têtes (le magasin : un chemin neuf, un contenu neuf).
mkdir -p "$ICI/entetes"
cp "$RAG3DB_INCLUDE_DIR/rag3db.hpp" "$ICI/entetes/rag3db.hpp"
printf '\n// témoin test_pont_recompile\n' >> "$ICI/entetes/rag3db.hpp"
RAG3DB_INCLUDE_DIR="$ICI/entetes" verifier
cas $? "un autre dossier d'en-têtes recompile le pont"

# 2. Le même dossier, l'en-tête modifié en place (un rebâti en place).
RAG3DB_INCLUDE_DIR="$ICI/entetes" cargo check --lib --features rag3db-native > "$JOURNAL" 2>&1
printf '// encore\n' >> "$ICI/entetes/rag3db.hpp"
RAG3DB_INCLUDE_DIR="$ICI/entetes" verifier
cas $? "l'en-tête modifié en place recompile le pont"

# Le pont revient au bâti donné, pour ne pas laisser le target sur l'en-tête du témoin.
cargo check --lib --features rag3db-native > "$JOURNAL" 2>&1
exit $echec
