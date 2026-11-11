#!/bin/bash
# push-lot.sh <worktree> <base rejouée> : le dernier pas d'un lot, sous `poste push` — fetch, garde
# (master n'a rien reçu de moteur ni de test depuis la base rejouée, docs exceptées), rebase, garde
# des tickets, push, trois contrôles.
set -u
W=${1:?worktree}; BASE=${2:?base}
ENGINE='src cmake CMakeLists.txt test extension/vector extension/geo tools/rust_api third_party scripts'
cd "$W" || exit 1
git fetch -q origin master || exit 5
M=$(git rev-parse FETCH_HEAD)
e=$(git diff --name-only "$BASE" "$M" -- $ENGINE | grep -vE '(^|/)docs/')
[ -z "$e" ] || { echo "master a reçu du code depuis $BASE, rien poussé :"; echo "$e" | head; exit 9; }
git rebase -q "$M"
gd=$(git rev-parse --absolute-git-dir)
if [ -d "$gd/rebase-merge" ] || [ -d "$gd/rebase-apply" ]; then git rebase --abort; echo "conflit, rien poussé"; exit 8; fi
git merge-base --is-ancestor "$M" HEAD || { echo "pas en avance rapide"; exit 8; }
python3 extension/rag3weaver/scripts/check_tickets.py . || { echo "tickets rouges"; exit 7; }
python3 extension/rag3weaver/scripts/check_tickets.py --lot "$M" . | head -12
git push -q origin HEAD:master; echo "push code $?"
H=$(git rev-parse HEAD)
echo "contrôle 1, status : [$(git status --short --untracked-files=no | tr '\n' ' ')]"
if [ -d "$gd/rebase-merge" ] || [ -d "$gd/rebase-apply" ]; then echo "contrôle 2 : REBASE EN COURS"; else echo "contrôle 2 : aucun rebase en cours"; fi
git fetch -q origin master
git merge-base --is-ancestor "$H" FETCH_HEAD && echo "contrôle 3 : $H dans master $(git rev-parse --short FETCH_HEAD)" || echo "contrôle 3 : $H ABSENT"
