#!/bin/bash
# **Les témoins navigateur, leur étape de passe** (c7, 25 octobre 2026) : joués seuls, après la
# lib, sous la feature `navigateur`, jamais dans `cargo test --lib`. Sous la lib entière, les cœurs
# tenus, Chrome sans tête ne rendait pas la page en 20 s : quatre rouges, une charge qu'aucune
# personne devant l'IDE n'aura. Le même témoin, pas affaibli, dans de bonnes conditions.
#
# Les témoins sont ceux que marque `#[cfg_attr(not(feature = "navigateur"), ignore = …)]` dans
# src/ : un témoin neuf qui porte la marque entre dans l'étape sans rien toucher ici.
# RAG3WEAVER_EXIGER_NAVIGATEUR=1 par défaut : sans navigateur, un rouge, jamais un saut.
#
# Usage : scripts/temoins_navigateur.sh [features de la lib]
#   (défaut : $FEATURES, sinon rag3db-native,code,daemon ; `navigateur` s'ajoute)
# La lib du moteur se choisit comme pour la lib (RAG3DB_LIB, scripts/cargo_magasin.sh).
set -u
ICI="$(cd "$(dirname "$0")/.." && pwd)"
F="${1:-${FEATURES:-rag3db-native,code,daemon}}"
noms=$(grep -rh -A4 'feature = "navigateur"), ignore' "$ICI/src" | grep -oE 'fn [a-z0-9_]+\(' | sed 's/^fn //; s/($//' | sort -u)
if [ -z "$noms" ]; then
  echo "ROUGE : aucun témoin navigateur marqué dans src/" >&2
  exit 1
fi
echo "▸ témoins navigateur ($(echo $noms | wc -w)) : $(echo $noms | tr '\n' ' ')" >&2
export RAG3WEAVER_EXIGER_NAVIGATEUR="${RAG3WEAVER_EXIGER_NAVIGATEUR:-1}"
cd "$ICI"
# shellcheck disable=SC2086
exec bash scripts/cargo_magasin.sh test --lib --features "$F,navigateur" -- $noms
