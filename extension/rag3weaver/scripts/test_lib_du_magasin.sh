#!/bin/bash
# Témoin de scripts/lib_du_magasin.sh : `courant` résolu une fois (une
# publication ensuite ne change pas le dossier rendu), un identifiant nommé,
# un refus nommé quand le magasin n'a rien. Des bâtis factices, comme
# test_rag3db_libs.py. Rend 0 si tout tient.
set -u
ICI="$(cd "$(dirname "$0")" && pwd)"
. "$ICI/lib_du_magasin.sh"
T="$(mktemp -d "${TMPDIR:-/tmp}/test-lib-magasin.XXXXXX")"
trap 'rm -rf "$T"' EXIT
export RAG3DB_LIBS_DIR="$T/magasin"
# Sans planchers (un fichier absent) : les bâtis factices n'ont pas de commit.
export RAG3DB_LIBS_PLANCHERS="$T/sans-planchers"
echec=0
cas() { if [ "$1" = 0 ]; then echo "OK     $2"; else echo "ÉCHEC  $2"; echec=1; fi; }

bati() { # $1 nom, $2 identifiant
  local src="$T/$1/src"
  mkdir -p "$src/include/common" "$T/$1/arbre"
  cp /bin/true "$src/librag3db.so"
  printf '%s\0' "$2" > "$T/$1.section"
  objcopy --add-section=.rag3db_build_id="$T/$1.section" "$src/librag3db.so"
  echo "// en-tête" > "$src/rag3db.hpp"
  printf '#define RAG3DB_BUILD_ID "%s"\n' "$2" > "$src/include/common/build_id.h"
  python3 "$ICI/rag3db_libs.py" publier "$T/$1" --racine "$T/$1/arbre" > /dev/null
}

lib_du_magasin courant > /dev/null 2>&1
[ $? = 2 ]
cas $? "un magasin vide : refus nommé (code 2)"

bati a rag3db-build-A
d="$(lib_du_magasin courant)"
[ "$d" = "$(cd "$RAG3DB_LIBS_DIR/rag3db-build-A" && pwd -P)" ]
cas $? "courant rend le dossier de l'identifiant publié"

bati b rag3db-build-B
[ "$d" = "$(cd "$RAG3DB_LIBS_DIR/rag3db-build-A" && pwd -P)" ] && [ -f "$d/librag3db.so" ]
cas $? "une publication ensuite ne change pas le dossier déjà résolu"
[ "$(lib_du_magasin courant)" = "$(cd "$RAG3DB_LIBS_DIR/rag3db-build-B" && pwd -P)" ]
cas $? "une nouvelle résolution prend la nouvelle publication"

[ "$(lib_du_magasin rag3db-build-A)" = "$(cd "$RAG3DB_LIBS_DIR/rag3db-build-A" && pwd -P)" ]
cas $? "un identifiant nommé rend son dossier"
lib_du_magasin rag3db-build-Z > /dev/null 2>&1
[ $? = 2 ]
cas $? "un identifiant absent : refus nommé"
# Le choix par défaut (choisir_la_lib).
unset RAG3DB_LIB RAG3WEAVER_MOTEUR_DU_WORKTREE
[ "$(choisir_la_lib /bati 2>/dev/null)" = "$(cd "$RAG3DB_LIBS_DIR/rag3db-build-B" && pwd -P)" ]
cas $? "sans rien poser : le courant du magasin"
[ "$(RAG3DB_LIB=bati choisir_la_lib /bati 2>/dev/null)" = "/bati/src" ]
cas $? "RAG3DB_LIB=bati : le dossier de bâti"
[ "$(RAG3WEAVER_MOTEUR_DU_WORKTREE=1 choisir_la_lib /bati 2>/dev/null)" = "/bati/src" ]
cas $? "le moteur du worktree : le dossier de bâti"
[ "$(RAG3DB_LIB=rag3db-build-A choisir_la_lib /bati 2>/dev/null)" = "$(cd "$RAG3DB_LIBS_DIR/rag3db-build-A" && pwd -P)" ]
cas $? "un identifiant nommé l'emporte"
vide="$(RAG3DB_LIBS_DIR="$T/vide" choisir_la_lib /bati 2>&1 >/dev/null)"
[ "$(RAG3DB_LIBS_DIR="$T/vide" choisir_la_lib /bati 2>/dev/null)" = "/bati/src" ] && [[ "$vide" == *repli* ]]
cas $? "un magasin vide : repli sur le dossier de bâti, dit"

# Le pont (exporter_la_lib, lot 3d) : les quatre variables sur un seul
# dossier, l'hérité remplacé et dit. Dans un sous-shell : l'export ne fuit pas.
B="$(cd "$RAG3DB_LIBS_DIR/rag3db-build-B" && pwd -P)"
dit="$( (RAG3DB_LIBRARY_DIR=/bati/src RAG3DB_INCLUDE_DIR=/bati/src LD_LIBRARY_PATH=/bati/src
  exporter_la_lib /bati 2>&1 >/dev/null
  [ "$RAG3DB_LIBRARY_DIR" = "$B" ] && [ "$RAG3DB_INCLUDE_DIR" = "$B" ] && [ "$RAG3DB_SHARED" = 1 ] \
    && [ "${LD_LIBRARY_PATH%%:*}" = "$B" ] && echo TIENT) )"
[[ "$dit" == *TIENT* ]] && [[ "$dit" == *hérité*remplacé* ]]
cas $? "exporter_la_lib : le courant pour les quatre variables, l'hérité remplacé et dit"
( RAG3DB_LIB=bati exporter_la_lib /bati 2>/dev/null
  [ "$RAG3DB_LIBRARY_DIR" = /bati/src ] && [ "${LD_LIBRARY_PATH%%:*}" = /bati/src ] )
cas $? "exporter_la_lib sous RAG3DB_LIB=bati : le dossier de bâti"
( RAG3DB_BUILD=/autre exporter_la_lib 2>/dev/null; [ "$RAG3DB_LIBRARY_DIR" = "$B" ] )
cas $? "exporter_la_lib sans argument : RAG3DB_BUILD pour le bâti, le courant choisi"
( RAG3DB_LIB=bati RAG3DB_BUILD=/autre exporter_la_lib 2>/dev/null; [ "$RAG3DB_LIBRARY_DIR" = /autre/src ] )
cas $? "exporter_la_lib sans argument, RAG3DB_LIB=bati : le bâti de RAG3DB_BUILD"
( RAG3DB_LIB=rag3db-build-Z exporter_la_lib /bati 2>/dev/null )
[ $? = 2 ]
cas $? "exporter_la_lib sur un identifiant absent : refus (code 2), rien d'exporté"
# Les planchers : une lib dont le bâti ne contient pas un correctif dont le
# crate dépend est refusée au choix, en le nommant (seulement si le dépôt a
# l'historique du moteur ; un clone superficiel ne prouve rien).
SOUS=rag3db-build-87a362e304200776000095c5a3d6531373560d9a-Release-p12
if git -C "$ICI" cat-file -e 87a362e304200776000095c5a3d6531373560d9a^{commit} 2>/dev/null \
   && git -C "$ICI" cat-file -e 4f9274a38769a085ab0009a89d1b7dba7597e568^{commit} 2>/dev/null; then
  bati sous "$SOUS"
  export RAG3DB_LIBS_PLANCHERS="$ICI/planchers-de-la-lib"
  dit="$(lib_du_magasin "$SOUS" 2>&1 >/dev/null)"; c=$?
  export RAG3DB_LIBS_PLANCHERS="$T/sans-planchers"
  [ $c = 2 ] && [[ "$dit" == *4f9274a38* ]]
  cas $? "une lib sous un plancher : refus au choix (code 2), le correctif manquant nommé"
else
  echo "SAUTÉ  les planchers : le dépôt n'a pas l'historique du moteur"
fi

exit $echec
