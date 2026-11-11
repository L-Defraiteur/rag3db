#!/bin/bash
# Docs seules du worktree de G : rebase sur master, rien d'autre que des docs de l'optimiseur, push.
set -u
W=/home/lucied/git_workspaces/rag3db-docs-G
for t in 1 2 3 4 5; do
  git -C "$W" fetch -q origin master || exit 4
  cible=$(git -C "$W" rev-parse FETCH_HEAD)
  if ! git -C "$W" merge-base --is-ancestor "$cible" HEAD; then
    git -C "$W" rebase -q "$cible" > /dev/null 2>&1 || { git -C "$W" rebase --abort; echo "conflit, rien poussé"; exit 8; }
  fi
  [ -d "$(git -C "$W" rev-parse --git-path rebase-merge)" ] && { echo "rebase arrêté"; exit 8; }
  for f in $(git -C "$W" diff --name-only "$cible" HEAD); do
    case "$f" in extension/rag3weaver/docs/optimiseur/*|docs/optimiseur/*|docs/tickets/*) ;; *) echo "autre chose : $f"; exit 8 ;; esac
  done
  git -C "$W" push -q origin HEAD:master && { echo "poussé $(git -C "$W" rev-parse --short HEAD)"; exit 0; }
done
exit 10
