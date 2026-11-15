#!/bin/bash
# reparer-les-deux-sens.sh (14 nov. 2026, hotfixs) — la réparation des tables de relations dont les deux index ne
# s'accordent plus (ticket docs/coeur-cpp/tickets/2026-11-14-les-deux-index-d-une-table-de-relations-ne-s-accordent-plus.md).
#
#   répétition, sur une copie : reparer-les-deux-sens.sh repetition <copie/nom.rag3db> - "<tables>"
#   dedans, sur une base déjà fermée par le pilote et déjà instantanée : reparer-les-deux-sens.sh dedans <nom.rag3db> - "<tables>"
#   fenêtre, sur la base de l'hôte : reparer-les-deux-sens.sh fenetre <nom.rag3db> <backend.json> "<tables>" "<jusqu'à>"
#
# 1. (fenêtre) la fermeture qui tient (refus si held_closed manque) ; 2. l'instantané par instantanes.sh, avec RETOUR.sh ;
# 3. le relevé ancré par tranches de chaque table (releve_sens2), sur la base fermée ; 4. le jugement : l'accord des deux
# sens, plus les lectures plausibles (bouts hors offset 0, ligne dans le Scope source, nom de la cible dans son texte),
# une arête par paire ; 5. DROP, CREATE au même schéma, COPY, CHECKPOINT, puis le contrôle des deux sens de G ;
# 6. le relevé ancré après : chaque table à 0 identifiant d'un seul sens ; 7. (fenêtre) la réouverture ; 8. les comptes.
# Un pas rouge arrête tout ; en fenêtre la base reste fermée, et le retour est le RETOUR.sh de l'instantané.
# Variables : ECRIRE, REQUETE, CONTROLE (bâtis contre la lib de la base, sur ce poste) ; INSTANTANES, ETIQUETTE ;
# OUTILS = le dossier de releve_sens2 et des scripts de jugement (disputes, cibles_plausibles, paires_doubles, final, analyse_sens).
set -u
MODE=${1:?repetition|fenetre}; BASE=${2:?base}; MANIFESTE=${3:?backend.json ou -}; TABLES=${4:?tables}; JUSQU_A=${5:-}
: "${ECRIRE:?}" "${REQUETE:?}" "${CONTROLE:?}" "${INSTANTANES:?}" "${ETIQUETTE:?}" "${OUTILS:?}"
NOM=$(basename "$BASE" .rag3db); O=$OUTILS/sorties/$NOM-$MODE-$(date +%Y%m%d-%H%M%S); mkdir -p "$O"
exec > >(tee -a "$O/reparation.log") 2>&1
dit() { echo "== $(date +%H:%M:%S) $*"; }
rouge() { echo "ROUGE : $*"; [ "$MODE" != repetition ] && echo "La base reste fermée. Retour : ${RETOUR:-aucun instantané}"; exit 1; }
B=$OUTILS/../fenetre-purge/banc_recherche.py
dit "$MODE $BASE ; tables $TABLES ; sorties $O"
if [ "$MODE" = fenetre ]; then
  dit "1. fermeture qui tient"
  python3 -I "$B" fermer "$MANIFESTE" "réparation des deux index de relations (hotfixs)" "$JUSQU_A" || rouge "fermeture"
fi
[ -z "$(fuser "$BASE" 2>/dev/null)" ] || rouge "la base est ouverte par un processus"
if [ "$MODE" != dedans ]; then
  dit "2. instantané $ETIQUETTE"
  bash "$INSTANTANES" "$ETIQUETTE" "$BASE" || rouge "instantané"
fi
RETOUR=${INSTANTANES_DIR:-~/.local/share/rag3weaver/instantanes}/$ETIQUETTE/RETOUR.sh; [ -f "$RETOUR" ] || rouge "pas de RETOUR.sh"
dit "3. relevé ancré"
for t in $TABLES; do "$OUTILS/releve_sens2" "$BASE" $t > "$O/$t.tsv" 2>"$O/$t.err" || rouge "relevé $t"; echo "$t : $(grep -c '^avant' "$O/$t.tsv") avant, $(grep -c '^arriere' "$O/$t.tsv") arrière"; done
dit "4. jugement"
python3 -I "$OUTILS/disputes.py" "$O" $TABLES || rouge "disputes"
python3 -I "$OUTILS/cibles_plausibles.py" "$O" "$REQUETE" "$BASE" $TABLES || rouge "jugement"
python3 -I "$OUTILS/paires_doubles.py" "$O" $TABLES | grep -v '^    '
python3 -I "$OUTILS/final.py" "$O" $TABLES || rouge "ensemble final"
dit "5. reconstruction et contrôle des deux sens"
bash "$OUTILS/../reconstruire.sh" "$BASE" "$O" $TABLES | tee "$O/reconstruire.txt"
tail -1 "$O/reconstruire.txt" | grep -qx VERT || rouge "reconstruction"
dit "6. relevé ancré après"
mkdir -p "$O/apres"
for t in $TABLES; do
  "$OUTILS/releve_sens2" "$BASE" $t > "$O/apres/$t.tsv" 2>/dev/null || rouge "relevé après $t"
  seul=$(python3 -I -c "
import collections,sys
a,r=set(),set()
for l in open('$O/apres/$t.tsv'):
    if l.startswith('#'): continue
    c=l.split('\t',2); (a if c[0]=='avant' else r).add(c[1])
print(len(a^r))")
  echo "$t : $(grep -c '^avant' "$O/apres/$t.tsv") avant, $(grep -c '^arriere' "$O/apres/$t.tsv") arrière, $seul d'un seul sens"
  [ "$seul" = 0 ] || rouge "$t garde $seul identifiants d'un seul sens"
done
if [ "$MODE" = fenetre ]; then
  dit "7. réouverture"
  python3 -I "$B" ouvrir "$MANIFESTE" || rouge "réouverture"
fi
dit "VERT ; sorties $O"
