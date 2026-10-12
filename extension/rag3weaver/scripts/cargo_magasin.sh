#!/bin/bash
# **cargo, avec la lib du magasin** (lot 3d, page
# docs/11-octobre-2026-recherche/07-le-magasin-de-libs.md §9) : pose la lib
# par `exporter_la_lib` (le `courant` par défaut, `RAG3DB_LIB=bati|courant|<id>`
# pour choisir), puis lance cargo avec les arguments donnés. À employer pour
# tout `cargo test --lib` ou `cargo check` joué à la main ou par un script de
# push, à la place de `RAG3DB_LIBRARY_DIR` posé sur `build/lecteurs-csv`.
#
#   scripts/cargo_magasin.sh test --lib --features rag3db-native,code
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib_du_magasin.sh"
exporter_la_lib || exit 2
exec cargo "$@"
