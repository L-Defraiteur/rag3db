#!/bin/bash
# **Les jeux de features sous lesquels `cargo check --tests` doit passer.**
# Une suite e2e est un module du binaire unique (tests/e2e/main.rs) : une
# suite dont le `#![cfg]` oublie une feature qu'elle emploie (common::burn
# sous burn-embedder) casse la compilation de tout le binaire sous un jeu qui
# ne l'a pas. Chaque jeu employé par une suite, un script ou une session est
# vérifié ici (11 octobre 2026, master rouge sous le jeu de G). À jouer avant
# tout hash qui touche tests/ ou les features.
#
# Usage : scripts/check_features.sh   (depuis extension/rag3weaver). La lib
# est celle du magasin, choisie comme pour run_e2e.sh (`exporter_la_lib`,
# `RAG3DB_LIB=bati|courant|<id>`) : un rebâti en place de build/lecteurs-csv
# ne la change pas pendant le contrôle (lot 3d).
set -u
JEUX=(
  # Celui de run_e2e.sh, la batterie.
  "rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres"
  # Le même, avec les suites qui appellent un vrai modèle.
  "rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm"
  # Sans embarqueur local : le jeu du paquet npm (G) et des postes sans carte.
  "rag3db-native,code,daemon,openai-llm"
  # Le minimum natif du backend de code.
  "rag3db-native,code"
  # Le natif seul, sans le code : le contrôle à part qu'on oubliait (un import
  # qui ne sert que sous `code` y devient une erreur ; 11 octobre 2026).
  "rag3db-native"
)
ICI="$(cd "$(dirname "$0")/.." && pwd)"
. "$ICI/scripts/garde_target.sh"
garder_le_target "${CARGO_TARGET_DIR:-$ICI/target}" "$ICI" || exit 2
. "$ICI/scripts/lib_du_magasin.sh"
exporter_la_lib || exit 2
echec=0
rouges=()
# L'index des tickets, recompté sur les fichiers : ses en-têtes ne se
# corrigent plus à la main.
if ! python3 "$ICI/scripts/check_tickets.py" "$ICI/../.."; then
  echec=1
  rouges+=("tickets")
fi
for jeu in "${JEUX[@]}"; do
  if timeout 1800 cargo check --tests --features "$jeu" > /dev/null 2>&1; then
    echo "VERT  $jeu"
  else
    echo "ROUGE $jeu   (cargo check --tests --features $jeu)"
    echec=1
    rouges+=("$jeu")
  fi
done
# **La conclusion en une ligne, la dernière** : une commande de push lit
# celle-ci et le code de sortie, jamais un compte de lignes VERT (le nombre de
# jeux change ; quatre verts et un rouge sont passés pour un vert le
# 11 octobre au soir).
if [ $echec -eq 0 ]; then
  echo "CHECK_FEATURES VERT ${#JEUX[@]}/${#JEUX[@]}"
else
  echo "CHECK_FEATURES ROUGE ${rouges[*]}"
fi
exit $echec
