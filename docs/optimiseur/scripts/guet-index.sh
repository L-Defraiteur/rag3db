#!/bin/bash
# Sort quand l'index du run <étiquette> du départage écrit sa dernière ligne (terminée/échouée/arrêtée), avec l'heure.
ssh -o BatchMode=yes luciepc "bash -s" <<X
T=~/.cache/rag3weaver-build/departage/$1/index.txt
until [ -s \$T ] && J=\$(sed -n 's/^journal : //p' \$T) && [ -n "\$J" ] && tail -1 "\$J" | grep -qE '^indexation (terminée|échouée|arrêtée)'; do sleep 10; [ \$SECONDS -gt 5400 ] && { echo "guet : 90 min sans fin"; exit 3; }; done
echo "$1 : fin de la phase d'index \$(stat -c %y "\$J" | cut -c12-19) — \$(tail -1 "\$J")"; grep -E 'vecteurs|embarqu' "\$J" | tail -2 | cut -c1-160
X
