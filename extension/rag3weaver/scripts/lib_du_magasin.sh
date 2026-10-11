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
