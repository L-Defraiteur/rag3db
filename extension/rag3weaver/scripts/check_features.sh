#!/bin/bash
# **Les jeux de features sous lesquels `cargo check --tests` doit passer.**
# Une suite e2e est un module du binaire unique (tests/e2e/main.rs) : une
# suite dont le `#![cfg]` oublie une feature qu'elle emploie (common::burn
# sous burn-embedder) casse la compilation de tout le binaire sous un jeu qui
# ne l'a pas. Chaque jeu employé par une suite, un script ou une session est
# vérifié ici (11 octobre 2026, master rouge sous le jeu de G). À jouer avant
# tout hash qui touche tests/ ou les features.
#
# Usage : scripts/check_features.sh   (depuis extension/rag3weaver, avec le
# moteur partagé dans RAG3DB_LIBRARY_DIR/RAG3DB_INCLUDE_DIR)
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
echec=0
# L'index des tickets, recompté sur les fichiers : ses en-têtes ne se
# corrigent plus à la main.
python3 "$ICI/scripts/check_tickets.py" "$ICI/../.." || echec=1
for jeu in "${JEUX[@]}"; do
  if timeout 1800 cargo check --tests --features "$jeu" > /dev/null 2>&1; then
    echo "VERT  $jeu"
  else
    echo "ROUGE $jeu   (cargo check --tests --features $jeu)"
    echec=1
  fi
done
exit $echec
