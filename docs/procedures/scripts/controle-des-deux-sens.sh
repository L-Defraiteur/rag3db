#!/bin/bash
# **Le contrôle des deux sens**, sur les instantanés d'une bascule, sous la lib neuve (demande
# d'hotfixs, 7 novembre 2026) : `tools/check_rel_directions` lit chaque table de relations par ses
# deux rangements et compare, relation par relation, propriété par propriété. Un écart prouve que
# la table est atteinte (le défaut des chaînes permutées au point de reprise, 25b3b45dc, et les
# demi-arêtes de LOCKS du 25 octobre) ; aucun écart ne prouve pas qu'elle est saine.
#
# Usage : controle-des-deux-sens.sh <étiquette> <id de la lib neuve>
#   (ex. controle-des-deux-sens.sh bascule-43 rag3db-build-…)
#
# Il ne touche jamais l'instantané, qui reste la recette de retour : chaque base est recopiée
# par `cp -a --reflink=always` (même btrfs, immédiat) dans …/instantanes/<étiquette>-controle/,
# et c'est cette copie que l'outil ouvre, en lecture seule. L'outil se bâtit contre la lib du
# magasin (son `rag3db.hpp` et sa `librag3db.so`), pas contre un arbre de bâti.
# Code de sortie : 0 aucun écart, 1 un écart dans au moins une base, 2+ le contrôle n'a pas tourné
# (7 : aucune base contrôlée — un contrôle qui ne trouve rien à juger est rouge, jamais vert par
# défaut ; hotfixs, 7 novembre).
set -u
[ $# = 2 ] || { echo "usage : $0 <étiquette> <id de la lib>" >&2; exit 2; }
E=$1; LIB=${RAG3DB_LIBS_DIR:-$HOME/.cache/rag3db-libs}/$2
I=~/.local/share/rag3weaver/instantanes/$E
C=~/.local/share/rag3weaver/instantanes/$E-controle
# L'outil se prend dans origin/master, jamais dans un arbre de travail qui peut avoir du retard
# (7 nov. : l'arbre de Lucie portait encore la version à comptes) ; le fetch ne touche que les refs
# distantes. CHECK_REL_DIRECTIONS_SRC désigne un autre fichier, en le sachant.
DEPOT=${CHECK_REL_DIRECTIONS_DEPOT:-$HOME/git_workspaces/rag3db}
SRC=${CHECK_REL_DIRECTIONS_SRC:-}
[ -d "$I" ] || { echo "REFUS : pas d'instantané $I" >&2; exit 3; }
[ -f "$LIB/rag3db.hpp" ] && [ -f "$LIB/librag3db.so" ] || { echo "REFUS : $LIB n'a pas rag3db.hpp et librag3db.so" >&2; exit 3; }
[ -e "$C" ] && { echo "REFUS : $C existe déjà (un contrôle précédent) ; l'effacer en le sachant" >&2; exit 3; }
OUTIL=$C/check_rel_directions
mkdir -p "$C"
if [ -z "$SRC" ]; then
  git -C "$DEPOT" fetch -q origin master || { echo "ÉCHEC : fetch de $DEPOT" >&2; exit 4; }
  SRC=$C/check_rel_directions.cpp
  git -C "$DEPOT" show origin/master:tools/check_rel_directions/check_rel_directions.cpp > "$SRC" || { echo "ÉCHEC : l'outil absent de origin/master" >&2; exit 4; }
  echo "outil : tools/check_rel_directions de origin/master, commit $(git -C "$DEPOT" log -1 --format=%h origin/master -- tools/check_rel_directions)"
else
  echo "outil : $SRC (désigné)"
fi
nice -n 15 g++ -std=c++20 -O2 -I "$LIB" "$SRC" -L "$LIB" -lrag3db -Wl,-rpath,"$LIB" -o "$OUTIL" \
  || { echo "ÉCHEC : l'outil ne se bâtit pas contre $LIB" >&2; exit 4; }
verdict=0
controlees=0
for sous in "$I"/*/; do
  nom=$(basename "$sous")
  base="$sous$nom.rag3db"
  [ -e "$base" ] || continue
  mkdir -p "$C/$nom"
  cp -a --reflink=always "$sous"/. "$C/$nom/" || { echo "ÉCHEC : copie reflink de $sous" >&2; exit 5; }
  "$OUTIL" "$C/$nom/$nom.rag3db" > "$C/$nom.log" 2>&1
  rc=$?
  controlees=$((controlees + 1))
  echo "$nom : rc=$rc — $(tail -1 "$C/$nom.log" | cut -c1-160)"
  case $rc in 0) ;; 1) verdict=1 ;; *) [ $verdict = 0 ] && verdict=6 ;; esac
done
if [ $controlees = 0 ]; then
  echo "verdict : ROUGE — aucune base contrôlée dans $I (attendu : <nom>/<nom>.rag3db)"
  exit 7
fi
echo "bases contrôlées : $controlees"
echo "verdict : $([ $verdict = 0 ] && echo 'aucun écart' || ([ $verdict = 1 ] && echo 'ÉCART — la bascule s’arrête, ticket' || echo 'le contrôle n’a pas tourné partout'))"
exit $verdict
