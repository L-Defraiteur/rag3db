#!/bin/bash
# Témoin de chemins_a_plat.sh : un script de procédure qui nomme un worktree à plat est refusé ; les créneaux du pousseur
# et un arbre sous ~/git_workspaces/<session>/ passent.
X=$(realpath "${1:-$(dirname "$0")/chemins_a_plat.sh}")
D=$(mktemp -d -p "${TMPDIR:-$HOME/.cache}"); trap 'rm -rf "$D"' EXIT
mkdir -p "$D/tools/poste" "$D/docs/procedures/scripts"
rouge=0
printf 'W=~/git_workspaces/rag3db-pusher\nV=$HOME/git_workspaces/rag3db-pusher-2/x\nU=~/git_workspaces/G/20-11-2026-lib\n' > "$D/tools/poste/a.sh"
bash "$X" "$D" > /dev/null && echo "vert   les créneaux du pousseur et un arbre de session passent" || { echo "ROUGE  un chemin permis est refusé"; rouge=1; }
echo 'W=/home/lucied/.cache/rag3weaver-build/wt-lib-arbre' > "$D/docs/procedures/scripts/b.sh"
bash "$X" "$D" > "$D/out" && { echo "ROUGE  un wt-* du cache passe"; rouge=1; } || { grep -q 'b.sh' "$D/out" && echo "vert   un wt-* du cache est refusé, la ligne dite" || { echo "ROUGE  refus sans la ligne"; rouge=1; }; }
rm "$D/docs/procedures/scripts/b.sh"; echo 'cd ~/git_workspaces/rag3db-instable' > "$D/tools/c.sh"
bash "$X" "$D" > /dev/null && { echo "ROUGE  un worktree à plat passe"; rouge=1; } || echo "vert   un worktree à plat est refusé"
exit $rouge
