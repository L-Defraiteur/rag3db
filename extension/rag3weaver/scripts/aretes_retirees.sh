#!/bin/bash
# Les relations d'usage qu'une révision de codeparsers retire sur un corpus,
# classées par cause.
#
#   scripts/aretes_retirees.sh <rév. avant> <rév. après> [dossier du corpus]
#
# Exemple (lot des fausses arêtes, 3 octobre 2026) :
#   scripts/aretes_retirees.sh 924a1d3 f0faa82 src/dataflow
#
# Chaque révision est bâtie dans un worktree détaché du sous-module, sous
# ~/.cache (jamais /tmp, qui est en mémoire vive), avec l'exemple
# `relations_tsv` (scripts/relations_tsv.rs, copié dans chaque révision), et
# dans son propre target : un target partagé laisse cargo prendre l'artefact
# d'une révision pour l'autre. Les relations sont prises avec les options de
# rag3weaver. La liste complète va dans ~/.cache ; le tableau et
# des exemples sortent sur la console.

set -euo pipefail

AVANT="$1"
APRES="$2"
WEAVER="$(cd "$(dirname "$0")/.." && pwd)"
CORPUS="$WEAVER/${3:-src/dataflow}"
CP="$WEAVER/codeparsers"
CACHE="$HOME/.cache/rag3weaver-build/aretes-retirees"
mkdir -p "$CACHE"

mapfile -t FICHIERS < <(find "$CORPUS" -name '*.rs' | sort)

for rev in "$AVANT" "$APRES"; do
  arbre="$CACHE/codeparsers-$rev"
  [ -d "$arbre" ] || git -C "$CP" worktree add --detach "$arbre" "$rev" >/dev/null
  cp "$WEAVER/scripts/relations_tsv.rs" "$arbre/examples/"
  (cd "$arbre" && CARGO_TARGET_DIR="$CACHE/target-$rev" cargo build -j8 --release --example relations_tsv -q)
  "$CACHE/target-$rev/release/examples/relations_tsv" --rag3weaver "$WEAVER" "${FICHIERS[@]}" > "$CACHE/$rev.tsv"
done

python3 "$WEAVER/scripts/classer_aretes_retirees.py" "$CACHE/$AVANT.tsv" "$CACHE/$APRES.tsv" \
  --racine "$WEAVER" --liste "$CACHE/$AVANT-$APRES.liste.tsv"
echo
echo "liste complète : $CACHE/$AVANT-$APRES.liste.tsv"
