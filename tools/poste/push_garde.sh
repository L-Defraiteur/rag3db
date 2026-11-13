#!/bin/bash
# Sous `poste push rag3db-97 -- <ce script> <worktree> <dossier des journaux>`, à luciepc :
# pousse le worktree seulement si la passe a fini VERT sur ce commit, sur une seule lib, et
# si master n'a apporté aucun code depuis la base testée. Puis les trois contrôles.
set -u
# **Le code qu'apporte un intervalle de commits** (`<base> <neuf>`), un chemin par ligne : ce qui oblige
# à rejouer la passe. Mode `--code-apporte <base> <neuf>`, joué par le témoin `test_push_garde.sh`.
#
# **La règle** (ticket docs/recherche/tickets/2026-11-13-la-garde-du-push-compte-un-outil-de-mesure-comme-du-code.md) :
# est du code ce qui se compile ou se joue — le Rust, le Cargo, `src/`, `ir/`, `ui/`, les gabarits, les
# tests, `tools/rust_api`, `extension/vector`, le sous-module codeparsers, les scripts de passe
# (run_e2e.sh, check_features.sh, temoins_navigateur.sh, cargo_magasin.sh) — et **tout fichier dont le
# nom apparaît dans ce code ou dans un script qu'il appelle** (à transitivité). Le reste (un outil de
# mesure que rien de joué n'appelle, une page) compte comme de la doc.
RACINES_JOUEES="extension/rag3weaver/src extension/rag3weaver/tests extension/rag3weaver/ir extension/rag3weaver/templates extension/rag3weaver/ui extension/rag3weaver/run_e2e.sh extension/rag3weaver/scripts/check_features.sh extension/rag3weaver/scripts/temoins_navigateur.sh extension/rag3weaver/scripts/cargo_magasin.sh"
toujours_du_code() {
  case "$1" in
    *.rs|*Cargo.toml|*Cargo.lock|*/build.rs) return 0 ;;
    tools/rust_api/*|extension/vector/*|extension/rag3weaver/codeparsers) return 0 ;;
    extension/rag3weaver/src/*|extension/rag3weaver/tests/*|extension/rag3weaver/ir/*|extension/rag3weaver/templates/*|extension/rag3weaver/ui/*) return 0 ;;
    extension/rag3weaver/run_e2e.sh|extension/rag3weaver/scripts/check_features.sh|extension/rag3weaver/scripts/temoins_navigateur.sh|extension/rag3weaver/scripts/cargo_magasin.sh) return 0 ;;
  esac
  return 1
}
# Les scripts appelés dans un arbre (un commit), à transitivité : un fichier du crate (hors code toujours,
# hors .md) dont le nom apparaît dans une racine jouée ou dans un script déjà appelé. Les noms sont
# séparés par des espaces : un nom de fichier avec une espace casserait la liste (le crate n'en a pas).
appeles_dans() {
  local arbre=$1 f n change candidats appeles=""
  local -a racines_existantes=()
  for r in $RACINES_JOUEES; do git cat-file -e "$arbre:$r" 2>/dev/null && racines_existantes+=("$r"); done
  candidats=$(git ls-tree -r --name-only "$arbre" -- extension/rag3weaver | grep -v '\.md$' | while read -r f; do toujours_du_code "$f" || echo "$f"; done)
  change=1
  while [ $change = 1 ]; do
    change=0
    for f in $candidats; do
      case " $appeles " in *" $f "*) continue ;; esac
      n=$(basename "$f")
      # shellcheck disable=SC2086
      if git grep -q -F -e "$n" "$arbre" -- "${racines_existantes[@]}" $appeles 2>/dev/null; then
        appeles="$appeles $f"; change=1
      fi
    done
  done
  echo "$appeles"
}
code_apporte() {
  local base=$1 neuf=$2 f appeles
  # Appelé d'un côté ou de l'autre : supprimer ou renommer un script appelé dans la base est du code
  # (relecture de recherche, 13 novembre 2026), comme en ajouter un qu'appelle le nouvel arbre.
  appeles="$(appeles_dans "$base") $(appeles_dans "$neuf")"
  # --no-renames : un renommage se lit comme une suppression et un ajout, les deux chemins jugés.
  git diff --no-renames --name-only "$base" "$neuf" -- extension/rag3weaver tools/rust_api extension/vector Cargo.lock ':!*.md' | while read -r f; do
    if toujours_du_code "$f"; then echo "$f"; continue; fi
    case " $appeles " in *" $f "*) echo "$f" ;; esac
  done
}
if [ "${1:-}" = "--code-apporte" ]; then code_apporte "$2" "$3"; exit 0; fi
# **La dérive se voit** (c7, 13 novembre 2026) : la copie exécutée (au cache, `~/.cache/rag3weaver-build/`)
# fait autorité — un lot ne peut pas affaiblir la garde qui le juge ; elle dit à chaque exécution son hash et
# celui de `tools/poste/push_garde.sh` dans l'arbre poussé, et « garde en retard sur le dépôt » quand ils
# diffèrent, sans refuser. Mode `--derive <arbre>`, joué par le témoin.
derive() {
  local moi depot
  moi=$(sha256sum "$(realpath "$0")" | cut -c1-16)
  if [ -f "$1/tools/poste/push_garde.sh" ]; then
    depot=$(sha256sum "$1/tools/poste/push_garde.sh" | cut -c1-16)
  else
    depot="absente"
  fi
  echo "garde exécutée $moi, garde du dépôt $depot"
  if [ "$moi" = "$depot" ]; then echo "garde à jour"; else echo "garde en retard sur le dépôt (la copie exécutée fait autorité ; recopie après relecture et mot de l'orchestration)"; fi
}
if [ "${1:-}" = "--derive" ]; then derive "$2"; exit 0; fi
# Master tenu pour une autre session (fenêtre de c7) : pas de push tant que le fichier existe.
[ -e $HOME/.cache/rag3weaver-build/master-tenu ] && { echo "ARRÊT : master tenu — $(cat $HOME/.cache/rag3weaver-build/master-tenu)"; exit 1; }
J=$HOME/.cache/rag3weaver-build/$2
cd $HOME/git_workspaces/$1 || exit 1
derive "$PWD"
gd=$(git rev-parse --git-dir)
tail -1 $J/resume.log | grep -qx VERT || { echo "ARRÊT : la passe n'est pas VERTE"; exit 1; }
teste=$(head -1 $J/resume.log | awk '{print $2}')
# HEAD est le commit testé, ou en descend par des commits de doc seuls (des .md).
if [ "$(git rev-parse --short HEAD)" != "$teste" ]; then
  git merge-base --is-ancestor "$teste" HEAD || { echo "ARRÊT : HEAD ne descend pas du commit testé ($teste)"; exit 1; }
  ajoute=$(git diff --name-only "$teste" HEAD -- . ':!*.md')
  [ -z "$ajoute" ] || { echo "ARRÊT : HEAD ajoute autre chose que de la doc au commit testé ($teste) :"; echo "$ajoute"; exit 1; }
  echo "HEAD ajoute de la doc seule au commit testé ($teste)"
fi
[ -z "$(git status --porcelain)" ] || { echo "ARRÊT : arbre sale"; exit 1; }
[ "$(git config user.email)" = "luciedefraiteur@gmail.com" ] || { echo "ARRÊT : identité git"; exit 1; }
libs=$(grep -ho 'rag3db-build-[0-9a-f]\{40\}' $J/temoins.log $J/e2e_lectures.log $J/e2e_code.log $J/check_features.log $J/preuve.log $J/lib.log $J/navigateur.log 2>/dev/null | sort -u)
[ "$(echo "$libs" | grep -c .)" = 1 ] || { echo "ARRÊT : la passe a vu plusieurs libs du magasin :"; echo "$libs"; exit 1; }
echo "lib de toute la passe : $libs"
base=$(cat $J/base)
git fetch -q origin master || exit 1
neuf=$(git rev-parse FETCH_HEAD)
# Le C++ du moteur (src/, test/) n'est pas lié par la lib épinglée : il se rebase sans rejeu
# (docs/procedures/push-d-un-lot.md).
apporte=$(code_apporte $base $neuf)
if [ -n "$apporte" ]; then echo "ARRÊT : master a apporté du code depuis la base testée"; echo "$apporte"; exit 1; fi
echo "master $(git rev-parse --short $neuf), base testée $(git rev-parse --short $base) : rien de code entre les deux"
git rebase $neuf > /dev/null 2>&1
if [ -d "$gd/rebase-merge" ] || [ -d "$gd/rebase-apply" ] || git status --short | grep -q '^UU'; then
  git rebase --abort; echo "ARRÊT : le rebase sur $neuf bute"; exit 1
fi
git push -q origin HEAD:master || { echo "ARRÊT : push refusé"; exit 1; }
git fetch -q origin master
echo "== trois contrôles"
echo "status : [$(git status --porcelain)]"
if [ -d "$gd/rebase-merge" ] || [ -d "$gd/rebase-apply" ]; then echo "REBASE EN COURS"; else echo "aucun rebase"; fi
echo "master (FETCH_HEAD) $(git rev-parse --short FETCH_HEAD) HEAD $(git rev-parse --short HEAD)"
[ "$(git rev-parse FETCH_HEAD)" = "$(git rev-parse HEAD)" ] && echo POUSSE
