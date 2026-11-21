#!/bin/bash
# Témoin de tools/poste/index_master_ff.sh (21 novembre 2026, c7 et Lucie : l'arbre de master que l'hôte de luciepc
# indexe) : une origine jetable, l'arbre sur la branche index-master ; chaque cas dit vert ou ROUGE.
# Usage : bash tools/poste/test_index_master_ff.sh [chemin du script] — sortie 0 si tout est vert.
set -u
X=$(realpath "${1:-$(dirname "$0")/index_master_ff.sh}")
D=$(mktemp -d); trap 'rm -rf "$D"' EXIT
rouge=0
ok() { echo "vert   $1"; }
ko() { echo "ROUGE  $1"; rouge=1; }
[ -f "$X" ] || { echo "ROUGE  le script est absent : $X"; exit 1; }
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
git init -q --bare -b master "$D/origin.git"
git clone -q "$D/origin.git" "$D/auteur" 2>/dev/null
( cd "$D/auteur" && echo a > a && git add a && git commit -qm a && git push -q origin HEAD:master )
git clone -q "$D/origin.git" "$D/arbre" 2>/dev/null
git -C "$D/arbre" checkout -q -b index-master origin/master
S="$D/etat"; export INDEX_MASTER_STATE="$S"
avance() { ( cd "$D/auteur" && echo "$1" >> a && git commit -qam "$1" && git push -q origin HEAD:master ); }
tete() { git -C "$D/arbre" rev-parse HEAD; }
origine() { git -C "$D/auteur" rev-parse HEAD; }
joue() { bash "$X" "$D/arbre" > "$D/sortie" 2>&1; }

# 1. master avance : l'arbre suit, par ff, et le journal le dit.
avance b; joue; rc=$?
[ $rc = 0 ] && [ "$(tete)" = "$(origine)" ] && grep -q 'ff ' "$S/index-master-ff.log" \
  && ok "master avance : l'arbre suit par ff, le journal le dit" || ko "ff : rc $rc, tête $(tete), origine $(origine) ; $(cat "$D/sortie")"
# 2. Rien de neuf : rien au journal.
n=$(wc -l < "$S/index-master-ff.log"); joue
[ "$(wc -l < "$S/index-master-ff.log")" = "$n" ] && ok "rien de neuf : rien d'écrit" || ko "rien de neuf : le journal a grandi"
# 3. Un fetch en échec ne casse rien : dit au journal, l'arbre intact, code 0.
avance c; mv "$D/origin.git" "$D/origin.ailleurs"; avant=$(tete); joue; rc=$?
[ $rc = 0 ] && [ "$(tete)" = "$avant" ] && grep -q 'fetch en échec' "$S/index-master-ff.log" \
  && ok "fetch en échec : dit, l'arbre intact, code 0" || ko "fetch en échec : rc $rc ; $(tail -1 "$S/index-master-ff.log" 2>/dev/null)"
mv "$D/origin.ailleurs" "$D/origin.git"
# 4. Un ff refusé (un commit local sur index-master) : dit, signalé par un fichier, rien de réinitialisé.
git -C "$D/arbre" commit -q --allow-empty -m "un commit local"; avant=$(tete); joue; rc=$?
[ $rc = 0 ] && [ "$(tete)" = "$avant" ] && [ -s "$S/index-master-ff.refused" ] && grep -q 'ff refusé' "$S/index-master-ff.log" \
  && ok "ff refusé : dit au journal et signalé (index-master-ff.refused), rien de réinitialisé" \
  || ko "ff refusé : rc $rc, tête bougée ? $([ "$(tete)" = "$avant" ] && echo non || echo OUI) ; $(tail -1 "$S/index-master-ff.log" 2>/dev/null)"
git -C "$D/arbre" reset -q --hard HEAD~1
# 5. Le ff suivant qui passe efface le signal.
joue; [ ! -e "$S/index-master-ff.refused" ] && [ "$(tete)" = "$(origine)" ] && ok "un ff qui repasse efface le signal" || ko "le signal reste après un ff réussi"
# 6. Un arbre modifié : refusé, dit, rien de touché.
avance d; echo sale >> "$D/arbre/a"; avant=$(tete); joue
[ "$(tete)" = "$avant" ] && grep -q 'modifié' "$S/index-master-ff.log" && [ "$(tail -1 "$D/arbre/a")" = sale ] \
  && ok "arbre modifié : refusé, dit, la modification intacte" || ko "arbre modifié : $(tail -1 "$S/index-master-ff.log")"
git -C "$D/arbre" checkout -q -- a
# 7. Une autre branche : refusé, jamais de checkout.
git -C "$D/arbre" checkout -q -b autre; avant=$(tete); joue
[ "$(git -C "$D/arbre" branch --show-current)" = autre ] && [ "$(tete)" = "$avant" ] && grep -q "pas sur index-master" "$S/index-master-ff.log" \
  && ok "une autre branche : refusé, jamais de checkout" || ko "autre branche : $(git -C "$D/arbre" branch --show-current), $(tail -1 "$S/index-master-ff.log")"
exit $rouge
