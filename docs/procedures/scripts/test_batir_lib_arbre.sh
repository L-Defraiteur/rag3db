#!/bin/bash
# Témoin de l'arbre de batir-lib.sh (23 novembre 2026 : une fenêtre C++ a coûté deux heures à toute la file, l'arbre du
# jour étant neuf, donc le moteur recompilé à froid ; la veille, l'arbre chaud donnait la lib en 3 min). L'arbre par
# défaut est fixe (~/git_workspaces/<session>/lib) et gardé entre deux bâtis ; LIB_ARBRE donné, gardé aussi.
# Joué par BATIR_LIB_DIRE_ARBRE=1, qui dit l'arbre et sort avant tout geste git ; jamais le script sans ce mode.
set -u
S=$(realpath "${1:-$(dirname "$0")/batir-lib.sh}"); rouge=0
ok() { echo "vert   $1"; }; ko() { echo "ROUGE  $1"; rouge=1; }
grep -q 'BATIR_LIB_DIRE_ARBRE' "$S" || { ko "batir-lib.sh ne sait pas dire son arbre (BATIR_LIB_DIRE_ARBRE) : non joué"; exit 1; }
o=$(BATIR_LIB_DIRE_ARBRE=1 SESSION=G bash "$S" 2>&1); rc=$?
[ $rc = 0 ] && echo "$o" | grep -qx "arbre : $HOME/git_workspaces/G/lib (gardé)" && ok "par défaut : l'arbre fixe de la session, gardé" || ko "par défaut : rc $rc ; $o"
o=$(BATIR_LIB_DIRE_ARBRE=1 LIB_ARBRE=/tmp/un-arbre bash "$S" 2>&1); rc=$?
[ $rc = 0 ] && echo "$o" | grep -qx "arbre : /tmp/un-arbre (gardé)" && ok "LIB_ARBRE donné : cet arbre, gardé" || ko "LIB_ARBRE : rc $rc ; $o"
exit $rouge
