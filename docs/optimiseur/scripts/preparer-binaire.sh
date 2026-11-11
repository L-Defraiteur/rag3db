#!/bin/bash
# Prépare une pose « binaire seul » contre la lib de courant (9 nov. 2026) : bâti à luciepc de <hash> (sous poste
# lourd, -j8), copie dans nuit-<hash>/ là-bas, essai 7991 et check-manifests aux deux postes. Ne pose rien.
# Usage : preparer-binaire.sh <hash de master>
set -u
H=${1:?hash}; S=$(dirname "$0")
L=$(basename "$(readlink -f ~/.cache/rag3db-libs/courant)")
M_ICI="$HOME/.config/rag3weaver/code-rag3db/templates/backends/code/rag3db.json $HOME/.config/rag3weaver/memoire-longue/backends/memory/backend.json $HOME/.config/rag3weaver/code-essais-c7/templates/backends/code/rag3db.json $HOME/.config/rag3weaver/code-rag3db-essai-atelier/templates/backends/code/backend.json $HOME/.config/rag3weaver/code-rag3db-essai-atelier/templates/backends/atelier/backend.json $HOME/.config/rag3weaver/atelier/templates/backends/atelier/backend.json"
echo "== $(date +%T) lib de courant : ${L:13:9} ; tête $H"
ssh -o BatchMode=yes luciepc "bash -s" <<X || { echo ROUGE; exit 4; }
cd ~/.cache/rag3weaver-build
[ "\$(basename \$(readlink -f ~/.cache/rag3db-libs/courant))" = "$L" ] || { echo "courant diffère à luciepc"; exit 3; }
JOBS=8 ./poste lourd bash bati-lucie-luciepc.sh $L $H > bati-$H.log 2>&1
grep -q '== .* fini' bati-$H.log || { echo "bâti rouge"; tail -5 bati-$H.log; exit 4; }
mkdir -p nuit-$H && cp -p target-G/hote-lucie/release/rag3weaver-backend target-G/hote-lucie/release/rag3db-check-lib nuit-$H/
X=\$PWD/nuit-$H/rag3weaver-backend C=\$PWD/nuit-$H/rag3db-check-lib LUCIVY_ATTENDU=4.3.2 bash essai-hote-nouveau.sh $L 2>&1 | grep -E 'md5|check-lib :|lib chargée|^VERT|^ROUGE'
nuit-$H/rag3weaver-backend check-manifests ~/.config/rag3weaver/code-lourd/templates/backends/code/backend.json ~/.config/rag3weaver/code-principal/templates/backends/code/backend.json 2>&1 | tail -1
X
scp -q luciepc:.cache/rag3weaver-build/nuit-$H/rag3weaver-backend ~/.local/bin/rag3weaver-backend.nouveau && scp -q luciepc:.cache/rag3weaver-build/nuit-$H/rag3db-check-lib ~/.local/bin/rag3db-check-lib.nouveau || { echo ROUGE; exit 5; }
echo "== portable"
LUCIVY_ATTENDU=4.3.2 bash $S/essai-hote-nouveau.sh $L 2>&1 | grep -E 'md5|check-lib :|lib chargée|^VERT|^ROUGE'
~/.local/bin/rag3weaver-backend.nouveau check-manifests $M_ICI 2>&1 | tail -1
