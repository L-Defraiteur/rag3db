#!/bin/bash
# **Pas de chemin de worktree à plat en dur dans un script de procédure** (20 novembre 2026, c7, après la purge du 19 :
# bascule-pose.sh lisait son installeur dans un worktree retiré, et l'hôte de luciepc est resté coupé 36 s). Un script
# de tools/ ou de docs/procedures/scripts prend son arbre en argument, ou le crée sous
# ~/git_workspaces/<session>/<jj-mm-aaaa>-<nom> (règle de Lucie du 19 novembre). Seuls les deux créneaux permanents du
# pousseur, rag3db-pusher et rag3db-pusher-2, sont nommés en dur.
# Usage : chemins_a_plat.sh [racine du dépôt] — rc 0 : aucun ; 1 : les lignes fautives, sur la sortie.
R=${1:-.}
h=$(grep -rnE 'git_workspaces/rag3db-[A-Za-z0-9_.-]+|rag3weaver-build/wt-[A-Za-z0-9_.-]*' "$R/tools" "$R/docs/procedures/scripts" 2>/dev/null \
  | grep -vE 'git_workspaces/rag3db-pusher(-2)?([^A-Za-z0-9_.-]|$)' | grep -v '/tools/poste/chemins_a_plat.sh:' | grep -v '/tools/poste/test_chemins_a_plat.sh:')
[ -z "$h" ] && exit 0
echo "un chemin de worktree à plat en dur (prendre l'arbre en argument, ou le créer sous ~/git_workspaces/<session>/<date>-<nom>) :"
echo "$h"
exit 1
