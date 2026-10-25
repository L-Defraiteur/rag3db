#!/bin/bash
# **Les instantanés d'une bascule de lib** (première : 25 octobre 2026, lib 04063ee06 : formats
# 41 et 42, requêtes préparées) : les quatre bases partagées, aux deux postes, avant que la lib
# neuve ne les touche. À lancer au mot de c7, quand G a arrêté les hôtes et leurs sockets. Le script
# commun refuse sans rien copier si un processus tient encore un fichier d'une base.
set -u
# Usage : instantanes-bascule.sh <étiquette>   (ex. bascule-66994dd43)
E=${1:?étiquette}
echo "== ici ($(hostname))"
bash ~/.cache/rag3weaver-notes-A/instantanes.sh $E \
  ~/.local/share/rag3weaver/memoire-longue.rag3db \
  ~/.local/share/rag3weaver/code-rag3db-depot.rag3db
ici=$?
echo "== luciepc"
timeout 600 ssh -T -o BatchMode=yes lucied@luciepc "bash ~/.cache/rag3weaver-notes-A/instantanes.sh $E \
  ~/.local/share/rag3weaver/code-principal.rag3db \
  ~/.local/share/rag3weaver/code-lourd.rag3db"
la_bas=$?
echo "ici rc=$ici ; luciepc rc=$la_bas"
[ $ici = 0 ] && [ $la_bas = 0 ]
