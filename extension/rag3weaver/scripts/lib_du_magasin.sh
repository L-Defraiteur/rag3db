# À sourcer (run_e2e.sh) : `lib_du_magasin <choix>` rend sur sa sortie le
# dossier du magasin de libs à employer pour toute la passe (page
# docs/11-octobre-2026-recherche/07-le-magasin-de-libs.md).
#
# `<choix>` vaut `courant` (le lien, résolu **une fois**, ici : une
# publication pendant la passe ne la change pas) ou un identifiant de bâti.
# Le dossier rendu porte la lib, les en-têtes et l'extension du même bâti. Un
# refus nommé, code 2, si le magasin n'a pas ce qu'on demande.
lib_du_magasin() {
  local choix="$1" outil m dossier
  outil="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/rag3db_libs.py"
  m="${RAG3DB_LIBS_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/rag3db-libs}"
  if [ "$choix" = courant ]; then
    dossier="$(python3 "$outil" courant)" || return 2
  else
    dossier="$m/$choix"
  fi
  if [ ! -f "$dossier/librag3db.so" ] || [ ! -f "$dossier/include/common/build_id.h" ]; then
    echo "✗ le magasin $m n'a pas de lib pour « $choix » ($dossier) : publiez-la" \
         "(python3 $outil publier <dossier de bâti>), ou laissez RAG3DB_LIB vide" >&2
    return 2
  fi
  (cd "$dossier" && pwd -P)
}

# `choisir_la_lib <dossier de bâti>` : le dossier de la lib de la passe, sur
# sa sortie, et une ligne qui dit pourquoi sur l'erreur. Le magasin par défaut
# (décision de l'orchestration, 12 octobre 2026), dans cet ordre :
#   1. `RAG3DB_LIB` posé : `bati` (le dossier de bâti), `courant` ou un identifiant ;
#   2. `RAG3WEAVER_MOTEUR_DU_WORKTREE=1` (le cœur éprouve son moteur) : le dossier de bâti ;
#   3. le `courant` du magasin s'il existe, sinon le dossier de bâti, dit comme un repli.
choisir_la_lib() {
  local bati="$1" m
  m="${RAG3DB_LIBS_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/rag3db-libs}"
  case "${RAG3DB_LIB:-}" in
    bati)
      echo "▸ lib du dossier de bâti (RAG3DB_LIB=bati) : $bati/src" >&2
      echo "$bati/src" ;;
    "")
      if [ "${RAG3WEAVER_MOTEUR_DU_WORKTREE:-0}" = 1 ]; then
        echo "▸ lib du dossier de bâti (le moteur de ce worktree) : $bati/src" >&2
        echo "$bati/src"
      elif [ -e "$m/courant" ]; then
        local d
        d="$(lib_du_magasin courant)" || return 2
        echo "▸ lib du magasin, par défaut (courant) : $d" >&2
        echo "$d"
      else
        echo "▸ le magasin $m est vide : repli sur le dossier de bâti $bati/src (RAG3DB_LIB=bati pour le dire)" >&2
        echo "$bati/src"
      fi ;;
    *)
      local d
      d="$(lib_du_magasin "$RAG3DB_LIB")" || return 2
      echo "▸ lib du magasin ($RAG3DB_LIB) : $d" >&2
      echo "$d" ;;
  esac
}

# `exporter_la_lib [<dossier de bâti>]` : **le pont lit la lib du magasin**,
# comme run_e2e.sh (lot 3d, page 07 §9). Le choix est celui de
# `choisir_la_lib`, fait une fois, et pose pour tout cargo qui suit les quatre
# variables sur le **même** dossier : `RAG3DB_LIBRARY_DIR` et
# `RAG3DB_INCLUDE_DIR` (le pont se compile contre ses en-têtes et lit son
# identifiant), `RAG3DB_SHARED=1`, et `LD_LIBRARY_PATH` en tête (le binaire
# charge cette lib, pas une autre de même nom). Un dossier du magasin est
# immuable : un rebâti en place de `build/lecteurs-csv` pendant un
# `cargo test --lib` ne change plus la lib liée. Un `RAG3DB_LIBRARY_DIR`
# hérité qui diffère est remplacé, et c'est dit. Le dossier de bâti par
# défaut est `RAG3DB_BUILD`, sinon le `build/lecteurs-csv` de l'arbre
# principal (celui que l'arbre principal rebâtit).
exporter_la_lib() {
  local bati="${1:-}" d racine
  if [ -z "$bati" ]; then
    if [ -n "${RAG3DB_BUILD:-}" ]; then
      bati="$RAG3DB_BUILD"
    else
      racine="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" || return 2
      bati="$(dirname "$racine")/build/lecteurs-csv"
    fi
  fi
  d="$(choisir_la_lib "$bati")" || return 2
  if [ -n "${RAG3DB_LIBRARY_DIR:-}" ] && [ "$RAG3DB_LIBRARY_DIR" != "$d" ]; then
    echo "▸ RAG3DB_LIBRARY_DIR hérité ($RAG3DB_LIBRARY_DIR) remplacé par $d (RAG3DB_LIB=bati pour garder le dossier de bâti)" >&2
  fi
  export RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR="$d" RAG3DB_INCLUDE_DIR="$d"
  export LD_LIBRARY_PATH="$d${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
}
