#!/bin/bash
set -u
cd ~/git_workspaces/rag3db-docs-cc || exit 1
git fetch -q origin master || { echo fetch; exit 2; }
M=$(git rev-parse FETCH_HEAD)
git rebase -q "$M"
gd=$(git rev-parse --absolute-git-dir)
if [ -d "$gd/rebase-merge" ] || [ -d "$gd/rebase-apply" ]; then git rebase --abort; echo "conflit, rien poussé"; exit 8; fi
test -n "$(git log --oneline "$M"..HEAD)" || { echo "rien à pousser"; exit 4; }
git diff --name-only "$M" HEAD | grep -qvE '^(docs/|extension/rag3weaver/docs/)' && { echo "autre chose que des docs, rien poussé"; exit 5; }
python3 extension/rag3weaver/scripts/check_tickets.py || { echo "tickets rouges"; exit 7; }
git push -q origin HEAD:master; echo "push code $?"
H=$(git rev-parse HEAD)
echo "contrôle 1 : [$(git status --short --untracked-files=no | tr '\n' ' ')]"
if [ -d "$gd/rebase-merge" ] || [ -d "$gd/rebase-apply" ]; then echo "contrôle 2 : REBASE EN COURS"; else echo "contrôle 2 : aucun rebase en cours"; fi
git fetch -q origin master
git merge-base --is-ancestor "$H" FETCH_HEAD && echo "contrôle 3 : $H dans master" || echo "contrôle 3 : ABSENT"
