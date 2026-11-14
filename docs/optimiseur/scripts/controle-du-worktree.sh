#!/bin/bash
# Le contrôle de début de passe (docs/procedures/push-d-un-lot.md, règle du 7 novembre 2026) :
# un worktree dont la branche est déjà atterrie, ou dont la base a plus de RETARD_MAX (200)
# commits de retard sur master, se refuse — on l'efface et on repart de master.
#   controle-du-worktree.sh <worktree>
set -u
cd "$1" || { echo "ARRÊT : worktree $1 introuvable"; exit 1; }
git fetch -q origin master || { echo "ARRÊT : fetch impossible"; exit 1; }
# Une branche sans commit à pousser est atterrie — sauf si du travail non commité est en cours.
if [ -z "$(git cherry origin/master HEAD | grep '^+')" ] && [ -z "$(git status --porcelain --untracked-files=no)" ]; then
  echo "ARRÊT : rien à pousser depuis $1 — la branche $(git rev-parse --abbrev-ref HEAD) est déjà atterrie : effacer ce worktree"; exit 1
fi
# Le retard se compte **par contenu** (14 novembre 2026, relevé de codeparsers) : sur une tête de pile, les lots d'avant
# déjà atterris sous d'autres hashes ne manquent pas ; `rev-list --count HEAD..origin/master` les comptait (275 au lieu
# de 133 sur dual-du-creux-pile). `--cherry-pick --right-only` ne compte que les commits de master dont le patch manque.
r=$(git log --cherry-pick --right-only --no-merges --format=%h HEAD...origin/master | wc -l)
if [ "$r" -gt "${RETARD_MAX:-200}" ]; then
  echo "ARRÊT : $r commits de retard sur master (plafond ${RETARD_MAX:-200}) — effacer $1 et repartir de master"; exit 1
fi
echo "worktree $1 : $(git cherry origin/master HEAD | grep -c '^+') commit(s) à pousser, $r de retard sur master"
