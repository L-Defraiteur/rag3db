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
