#!/bin/bash
# Témoin de controle-du-worktree.sh : une tête de pile dont le lot d'avant a atterri sous un autre hash ne doit pas
# compter ce lot comme du retard. Usage : test_controle_du_worktree.sh <script>
set -u; X=$(realpath "$1"); D=$(mktemp -d -p "${TMPDIR:-$HOME/.cache}"); trap 'rm -rf "$D"' EXIT
cd "$D" && git init -q --bare nu.git && git clone -q nu.git w 2>/dev/null && cd w && git config user.email t@t && git config user.name t
echo a > a && git add a && git commit -qm base && git push -q origin HEAD:master
git checkout -q -b pile && echo p1 > p1 && git add p1 && git commit -qm p1 && echo p2 > p2 && git add p2 && git commit -qm p2
# master : un autre lot, puis P1 atterri sous un autre hash (rebasé sur lui), puis deux lots de plus
git checkout -q -B m origin/master && echo x > x && git add x && git commit -qm autre && git cherry-pick pile~1 >/dev/null 2>&1 \
  && for i in 1 2; do echo $i > f$i && git add f$i && git commit -qm "lot $i"; done && git push -q origin HEAD:master && git checkout -q pile
rouge=0
# le vrai retard : « autre », « lot 1 », « lot 2 » = 3 ; P1 atterri n'en est pas
out=$(RETARD_MAX=3 bash "$X" "$D/w" 2>&1); echo "$out" | grep -q ', 3 de retard sur master' && echo "vert   P1 atterri sous un autre hash ne compte pas : 3 de retard ($out)" || { echo "ROUGE  le retard compte P1 atterri : $out"; rouge=1; }
out=$(RETARD_MAX=2 bash "$X" "$D/w" 2>&1); echo "$out" | grep -q '^ARRÊT : 3 commits de retard' && echo "vert   au-dessus du plafond, ARRÊT avec le compte vrai" || { echo "ROUGE  plafond : $out"; rouge=1; }
exit $rouge
