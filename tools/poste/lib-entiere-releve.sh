#!/bin/bash
# La lib entière de rag3weaver, avec le relevé au fil de l'eau de templates/ (14 novembre 2026, hotfixs,
# pour le ticket docs/arbre-principal/tickets/2026-11-13-un-fichier-de-plus-apparait-sous-templates-pendant-la-lib-entiere.md).
# Pendant la lib, chaque nom créé, écrit, renommé ou effacé sous templates/ est écrit dès l'événement.
# À la fin, la liste des noms apparus que git ne suit pas, c'est-à-dire le fichier de trop s'il est venu.
#
# Usage : tools/poste/lib-entiere-releve.sh <worktree> [dossier des relevés]
#   à lancer sous ~/.cache/rag3weaver-build/poste lourd ; RAG3DB_LIB (défaut : courant) et CARGO_TARGET_DIR
#   (défaut : celui de l'environnement) sont lus tels quels.
#   FEATURES (défaut : le jeu des tours) : rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm
# Code de sortie : celui de la lib ; 3 si des noms non suivis sont apparus sous templates/.
set -u
ICI=$(cd "$(dirname "$0")" && pwd)
W=${1:?worktree}; R=$W/extension/rag3weaver
O=${2:-$HOME/.cache/rag3weaver-build/releves-templates}; J=$O/$(date +%Y%m%d-%H%M%S); mkdir -p "$J"
export RAG3DB_LIB=${RAG3DB_LIB:-$(readlink "$HOME/.cache/rag3db-libs/courant")}
FEATURES=${FEATURES:-rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm}
cd "$R" || exit 2
echo "tête $(git log --oneline -1 | cut -c1-70) ; lib $RAG3DB_LIB ; features $FEATURES" > "$J/entete.txt"
git ls-files templates | sort > "$J/suivis.txt"
python3 -I "$ICI/releve-templates.py" "$R/templates" "$J/releve.log" & releve=$!
until grep -q ' DÉBUT ' "$J/releve.log" 2>/dev/null; do sleep 0.2; done
timeout --signal=TERM --kill-after=30s 3600 scripts/cargo_magasin.sh test --lib --features "$FEATURES" > "$J/lib.log" 2>&1; lib=$?
sleep 2; kill -TERM "$releve"; wait "$releve" 2>/dev/null
grep -E ' (CRÉÉ|ARRIVÉ|ÉCRIT|VU-DÉJÀ-LÀ) ' "$J/releve.log" | grep -v ' dossier ' | awk '{print $3}' | sort -u |
  while read -r n; do grep -qxF "templates/$n" "$J/suivis.txt" || echo "$n"; done > "$J/inconnus.txt"
{
  echo "lib code $lib"
  grep -E '^test result|every_shipped_template' "$J/lib.log" | tail -3
  echo "événements sous templates/ : $(($(wc -l < "$J/releve.log") - 2)) ; noms non suivis apparus : $(wc -l < "$J/inconnus.txt")"
  grep -E 'DÉBORDEMENT|ERREUR' "$J/releve.log" | head -3
} >> "$J/entete.txt"
cat "$J/entete.txt"; [ -s "$J/inconnus.txt" ] && { echo "NOMS NON SUIVIS :"; cat "$J/inconnus.txt"; }
echo "relevé : $J"
[ -s "$J/inconnus.txt" ] && exit 3
exit $lib
