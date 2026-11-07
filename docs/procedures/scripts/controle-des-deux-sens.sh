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
# Code de sortie : 0 aucun écart, 1 un écart dans au moins une base, 2+ le contrôle n'a pas tourné.
set -u
[ $# = 2 ] || { echo "usage : $0 <étiquette> <id de la lib>" >&2; exit 2; }
E=$1; LIB=${RAG3DB_LIBS_DIR:-$HOME/.cache/rag3db-libs}/$2
I=~/.local/share/rag3weaver/instantanes/$E
C=~/.local/share/rag3weaver/instantanes/$E-controle
SRC=${CHECK_REL_DIRECTIONS_SRC:-$HOME/git_workspaces/rag3db/tools/check_rel_directions/check_rel_directions.cpp}
[ -d "$I" ] || { echo "REFUS : pas d'instantané $I" >&2; exit 3; }
[ -f "$LIB/rag3db.hpp" ] && [ -f "$LIB/librag3db.so" ] || { echo "REFUS : $LIB n'a pas rag3db.hpp et librag3db.so" >&2; exit 3; }
[ -e "$C" ] && { echo "REFUS : $C existe déjà (un contrôle précédent) ; l'effacer en le sachant" >&2; exit 3; }
OUTIL=$C/check_rel_directions
mkdir -p "$C"
nice -n 15 g++ -std=c++20 -O2 -I "$LIB" "$SRC" -L "$LIB" -lrag3db -Wl,-rpath,"$LIB" -o "$OUTIL" \
  || { echo "ÉCHEC : l'outil ne se bâtit pas contre $LIB" >&2; exit 4; }
verdict=0
for sous in "$I"/*/; do
  nom=$(basename "$sous")
  base="$sous$nom.rag3db"
  [ -e "$base" ] || continue
  mkdir -p "$C/$nom"
  cp -a --reflink=always "$sous"/. "$C/$nom/" || { echo "ÉCHEC : copie reflink de $sous" >&2; exit 5; }
  "$OUTIL" "$C/$nom/$nom.rag3db" > "$C/$nom.log" 2>&1
  rc=$?
  echo "$nom : rc=$rc — $(tail -1 "$C/$nom.log" | cut -c1-160)"
  case $rc in 0) ;; 1) verdict=1 ;; *) [ $verdict = 0 ] && verdict=6 ;; esac
done
echo "verdict : $([ $verdict = 0 ] && echo 'aucun écart' || ([ $verdict = 1 ] && echo 'ÉCART — la bascule s’arrête, ticket' || echo 'le contrôle n’a pas tourné partout'))"
exit $verdict
