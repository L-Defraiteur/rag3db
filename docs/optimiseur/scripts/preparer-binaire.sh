#!/bin/bash
# Prépare une pose « binaire seul » contre la lib de courant (9 nov. 2026) : bâti à luciepc de <hash> (sous poste
# lourd, -j8), copie dans nuit-<hash>/ là-bas, essai 7991 et check-manifests aux deux postes. Ne pose rien.
# Usage : preparer-binaire.sh <hash de master>
set -u
H=${1:?hash}; S=$(dirname "$0")
# LIB=<id> : une lib du magasin qui n'est pas encore `courant` (bâti avec LIB_EXPLICITE=1)
L=${LIB:-$(basename "$(readlink -f ~/.cache/rag3db-libs/courant)")}
M_ICI="$HOME/.config/rag3weaver/code-rag3db/templates/backends/code/rag3db.json $HOME/.config/rag3weaver/memoire-longue/backends/memory/backend.json $HOME/.config/rag3weaver/code-essais-c7/templates/backends/code/rag3db.json $HOME/.config/rag3weaver/code-rag3db-essai-atelier/templates/backends/code/backend.json $HOME/.config/rag3weaver/code-rag3db-essai-atelier/templates/backends/atelier/backend.json $HOME/.config/rag3weaver/atelier/templates/backends/atelier/backend.json"
# La version de lucivy attendue : celle que déclare le Cargo.toml de master (4.3.3 le 13 nov.).
LV=${LUCIVY_ATTENDU:-$(git -C /home/lucied/git_workspaces/rag3db-docs-G show origin/master:extension/rag3weaver/Cargo.toml | sed -n 's/^lucivy-core = "\(.*\)"/\1/p')}
echo "lucivy attendue : $LV"
echo "== $(date +%T) lib de courant : ${L:13:9} ; tête $H"
ssh -o BatchMode=yes luciepc "bash -s" <<X || { echo ROUGE; exit 4; }
cd ~/.cache/rag3weaver-build
[ -n "${LIB:-}" ] || [ "\$(basename \$(readlink -f ~/.cache/rag3db-libs/courant))" = "$L" ] || { echo "courant diffère à luciepc"; exit 3; }
[ -f ~/.cache/rag3db-libs/$L/librag3db.so ] || { echo "$L absente du magasin de luciepc"; exit 3; }
LIB_EXPLICITE=${LIB:+1} JOBS=8 ./poste lourd bash bati-lucie-luciepc.sh $L $H > bati-$H.log 2>&1
grep -q '== .* fini' bati-$H.log || { echo "bâti rouge"; tail -5 bati-$H.log; exit 4; }
mkdir -p nuit-$H && cp -p target-G/hote-lucie/release/rag3weaver-backend target-G/hote-lucie/release/rag3db-check-lib nuit-$H/
X=\$PWD/nuit-$H/rag3weaver-backend C=\$PWD/nuit-$H/rag3db-check-lib LUCIVY_ATTENDU=$LV bash essai-hote-nouveau.sh $L 2>&1 | grep -E 'md5|check-lib :|lib chargée|^VERT|^ROUGE'
nuit-$H/rag3weaver-backend check-manifests ~/.config/rag3weaver/code-lourd/templates/backends/code/backend.json ~/.config/rag3weaver/code-principal/templates/backends/code/backend.json 2>&1 | tail -1
X
[ -f ~/.cache/rag3db-libs/$L/librag3db.so ] || rsync -a luciepc:.cache/rag3db-libs/$L ~/.cache/rag3db-libs/
echo "lib au portable : $(md5sum ~/.cache/rag3db-libs/$L/librag3db.so | cut -c1-8)"
scp -q luciepc:.cache/rag3weaver-build/nuit-$H/rag3weaver-backend ~/.local/bin/rag3weaver-backend.nouveau && scp -q luciepc:.cache/rag3weaver-build/nuit-$H/rag3db-check-lib ~/.local/bin/rag3db-check-lib.nouveau || { echo ROUGE; exit 5; }
echo "== portable"
LUCIVY_ATTENDU=$LV bash $S/essai-hote-nouveau.sh $L 2>&1 | grep -E 'md5|check-lib :|lib chargée|^VERT|^ROUGE'
~/.local/bin/rag3weaver-backend.nouveau check-manifests $M_ICI 2>&1 | tail -1
