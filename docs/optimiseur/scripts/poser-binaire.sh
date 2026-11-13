#!/bin/bash
# Pose « binaire seul » préparée par preparer-binaire.sh : luciepc puis le portable, sans instantané neuf (même lib,
# mêmes bases). Usage : poser-binaire.sh <hash> luciepc|portable
set -u
H=${1:?hash}; OU=${2:?luciepc|portable}; S=$(dirname "$0")
L=$(basename "$(readlink -f ~/.cache/rag3db-libs/courant)"); T=avant-${H:0:8}
if [ "$OU" = luciepc ]; then
ssh -o BatchMode=yes luciepc "bash -s" <<X
cd ~/.cache/rag3weaver-build; date +%T
cp -p nuit-$H/rag3weaver-backend ~/.local/bin/rag3weaver-backend.nouveau && cp -p nuit-$H/rag3db-check-lib ~/.local/bin/rag3db-check-lib.nouveau
INSTALLEUR=\$HOME/git_workspaces/rag3db-controle-G/extension/rag3weaver/scripts/installer-l-unite-de-l-hote.sh bash bascule-pose.sh $L $T 2>&1 | grep -E '^==|^VERT|^ROUGE'
# 13 nov. (mot de c7, sur la demande de Lucie du 12 : décharger les usages absents ; 7880 granite-107m arrêté)
sed -i 's#,127.0.0.1:7880##' ~/.config/rag3weaver/rag3weaver-daemon.env; grep EMBED ~/.config/rag3weaver/rag3weaver-daemon.env
bash bascule-reveil.sh $L 127.0.0.1:7878 127.0.0.1:7879 2>&1 | grep -E 'service|MainPID|LISTEN|AVERT|^VERT|^ROUGE'
python3 ouvrir-bases.py ~/.config/rag3weaver/code-lourd/templates/backends/code/backend.json ~/.config/rag3weaver/code-principal/templates/backends/code/backend.json 2>&1 | grep -oE 'ouverture : \{"ok": [a-z]+'
date +%T
X
else
date +%T
bash $S/bascule-pose.sh $L $T 2>&1 | grep -E '^==|^VERT|^ROUGE'
# 13 nov. : 7981 (granite-107m à luciepc) n'a plus de service derrière lui ; le tunnel reste dans son unité
sed -i 's#,127.0.0.1:7981##' ~/.config/rag3weaver/rag3weaver-daemon.env; grep EMBED ~/.config/rag3weaver/rag3weaver-daemon.env
bash $S/bascule-reveil.sh $L 127.0.0.1:7979 127.0.0.1:7980 2>&1 | grep -E 'service|MainPID|LISTEN|AVERT|^VERT|^ROUGE'
python3 $S/ouvrir-bases.py ~/.config/rag3weaver/code-rag3db/templates/backends/code/rag3db.json ~/.config/rag3weaver/memoire-longue/backends/memory/backend.json ~/.config/rag3weaver/code-essais-c7/templates/backends/code/rag3db.json ~/.config/rag3weaver/code-rag3db-essai-atelier/templates/backends/code/backend.json ~/.config/rag3weaver/atelier/templates/backends/atelier/backend.json 2>&1 | grep -oE '^== .*|ouverture : \{"ok": [a-z]+|"error": "[^"]{0,160}'
date +%T
fi
