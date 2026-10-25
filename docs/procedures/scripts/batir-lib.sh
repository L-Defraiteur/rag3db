#!/bin/bash
# **Rebâtir la lib du moteur et la publier au magasin**, depuis un worktree à moi
# (règle de Lucie, 13 octobre 2026 : son arbre ~/git_workspaces/rag3db reste sur
# master et ne porte aucun bâti ; plus de build/lecteurs-csv chez elle).
#
# Usage : [ABI_CHANGEE=1] batir.sh [<commit>]   (défaut : origin/master ; ABI_CHANGEE quand le cœur l'annonce)
# Sous le jeton du lourd de l'orchestration (une compilation à la fois par poste).
# Le worktree et son build/ sont gardés entre les rebâtis (plafond 20 Go), effacés en
# fin de session. L'ancienne recette : batir-arbre-de-lucie.sh.ancien.
set -u
D=/home/lucied/.cache/rag3weaver-notes-A/lib58
P=/home/lucied/git_workspaces/rag3db
W=/home/lucied/.cache/rag3weaver-build/wt-lib-arbre
B=$W/build/lib
L=/home/lucied/.cache/rag3weaver-build/poste
S=$P/extension/rag3weaver/scripts/rag3db_libs.py
mkdir -p $D; rm -f $D/fini
: > $D/etat
journal() { echo "$(date +%H:%M:%S) $*" >> $D/etat; }

# Le worktree, détaché sur le commit demandé : jamais une branche, jamais l'arbre de Lucie
# (le fetch ne touche que les refs distantes du dépôt commun).
git -C $P fetch -q origin master || { journal "fetch en échec"; exit 2; }
CIBLE=${1:-$(git -C $P rev-parse origin/master)}
if [ ! -d $W ]; then
  git -C $P worktree add -q --detach $W "$CIBLE" || { journal "worktree en échec"; exit 2; }
else
  git -C $W checkout -q --detach "$CIBLE" || { journal "checkout en échec"; exit 2; }
fi
[ -z "$(git -C $W status --short --untracked-files=no -- src extension/vector tools)" ] || { journal "ARRÊT : le worktree porte des modifications"; exit 3; }
journal "début, tête $(git -C $W rev-parse --short HEAD)"

# La configuration de build/lecteurs-csv, reprise telle quelle : Release, l'extension
# vector, l'en-tête unique, ni tests ni shell.
if [ ! -f $B/CMakeCache.txt ]; then
  cmake -S $W -B $B -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSIONS=vector \
    -DBUILD_SINGLE_FILE_HEADER=ON -DBUILD_TESTS=FALSE -DBUILD_SHELL=FALSE -DBUILD_BENCHMARK=OFF \
    -DBUILD_EXAMPLES=OFF -DBUILD_PYTHON=OFF -DBUILD_NODEJS=OFF -DBUILD_JAVA=OFF \
    -DENABLE_WERROR=OFF -DENABLE_LTO=OFF > $D/configurer.log 2>&1 \
    || { journal "ARRÊT : cmake en échec ($D/configurer.log)"; exit 4; }
fi
timeout 1h $L lourd cmake --build $B -j 8 > $D/batir.log 2>&1
rc=$?
journal "bâti rc=$rc"
[ $rc = 0 ] || exit 4

# Les symboles, par nom, contre la lib `courant` du magasin : un retrait change l'ABI,
# et `courant` ne se pose qu'après l'avoir annoncé.
AVANT=$(python3 $S courant 2>/dev/null)
nm -D "$AVANT/librag3db.so" 2>/dev/null | awk '{print $NF}' | sort > $D/noms-avant.txt
nm -D $B/src/librag3db.so | awk '{print $NF}' | sort > $D/noms-apres.txt
RETRAITS=$(comm -23 $D/noms-avant.txt $D/noms-apres.txt | wc -l)
AJOUTS=$(comm -13 $D/noms-avant.txt $D/noms-apres.txt | wc -l)
journal "symboles contre $(basename "$AVANT") : $AJOUTS ajoutés, $RETRAITS retirés"
# Une méthode virtuelle ajoutée ne retire aucun nom mais change la taille d'une
# vtable (E10, 18 octobre 2026) : les vtables communes aux deux libs se comparent
# par leur taille, et une taille qui bouge est un changement d'ABI.
nm -D -S "$AVANT/librag3db.so" 2>/dev/null | awk '$NF ~ /^_ZTV/ {print $NF, $2}' | sort > $D/vtables-avant.txt
nm -D -S $B/src/librag3db.so | awk '$NF ~ /^_ZTV/ {print $NF, $2}' | sort > $D/vtables-apres.txt
VTABLES=$(join $D/vtables-avant.txt $D/vtables-apres.txt | awk '$2 != $3 {print $1}' | c++filt | tr '\n' ' ')
[ -n "$VTABLES" ] && journal "vtables qui changent de taille : $VTABLES"
SANS_COURANT=--sans-courant  # c7, 25 oct. : publié sans courant, le verdict e2e_code d abord
# Un membre ajouté à une structure, ou une vtable non exportée, ne se voit ni par
# les noms ni par les tailles (le rollback du cœur, 5e3df4563, 18 octobre) : quand le
# cœur annonce une ABI changée, ABI_CHANGEE=1 le force.
[ -n "${ABI_CHANGEE:-}" ] && journal "ABI_CHANGEE posé : ABI changée par déclaration du cœur"
if [ "$RETRAITS" != 0 ] || [ -n "$VTABLES" ] || [ -n "${ABI_CHANGEE:-}" ]; then
  journal "ABI changée (symboles retirés ou vtables) : publié sans poser courant, à annoncer d'abord, puis rag3db_libs.py courant --poser <id>"
  :
fi

python3 $S publier $B --racine $W $SANS_COURANT >> $D/etat 2>&1
journal "publication rc=$? ; courant : $(python3 $S courant 2>&1)"
# L'identifiant publié, pour la passe qui suit : le magasin nomme une lib par son
# dernier commit moteur, pas par la tête demandée (e5e984a7b → a333c34f2, 19 oct.).
basename "$(sed -n 's/^publié : //p' $D/etat | tail -1)" > $D/id
md5sum $B/src/librag3db.so $W/extension/vector/build/libvector.rag3db_extension >> $D/etat
journal "dernier commit moteur : $(git -C $W log -1 --format='%h %ci' -- src extension/vector)"
touch $D/fini
