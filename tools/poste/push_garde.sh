#!/bin/bash
# Sous `poste push rag3db-97 -- <ce script> <worktree> <dossier des journaux>`, à luciepc :
# pousse le worktree seulement si la passe a fini VERT sur ce commit, sur une seule lib, et
# si master n'a apporté aucun code depuis la base testée. Puis les trois contrôles.
set -u
# Le chemin de cette garde, résolu avant tout `cd` (14 novembre 2026) : lancée par `bash push_garde.sh` depuis le
# cache, son `$0` relatif ne se résolvait plus une fois dans le worktree, et « garde exécutée » restait vide.
MOI=$(realpath "$0")
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
# **Les suites C++ transférables** (15 novembre 2026, relevé du cœur) : un lot qui ne touche que le moteur, testé sur une
# base, n'a pas à rejouer ses suites C++ (35 minutes) parce que du Rust a atterri : elles ne lient pas le Rust. Vrai si
# le lot (base..tête) ne touche que les chemins du moteur, plus des .md, et si master n'a rien changé sous ces chemins
# depuis la base. Le symétrique (un lot Rust quand seul du C++ a atterri) passe déjà : `code_apporte` ne regarde pas src/.
# tools/shell/ (18 novembre 2026, ticket de c7) : le shell C++ ne lie que la lib du moteur, jamais le crate Rust.
CHEMINS_MOTEUR="src/ test/ extension/vector/ tools/rust_api/ tools/shell/"
cpp_transferable() { # <base> <master> <tête du lot>
  local base=$1 maitre=$2 tete=$3 f hors=""
  [ -z "$(git diff --name-only "$base" "$maitre" -- $CHEMINS_MOTEUR)" ] || return 1
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in *.md|src/*|test/*|extension/vector/*|tools/rust_api/*|tools/shell/*) ;; *) hors="$hors $f" ;; esac
  done < <(git diff --name-only "$base" "$tete")
  [ -z "$hors" ]
}
# **Le mode pile** (14 novembre 2026, la file en pile, demande de c7 et relevé de recherche) : un lot rejoué
# sur la pile de ceux devant lui, puis posé au dernier pas par `rebase --onto origin/master <avant>`, ne descend
# plus du commit testé. PILE_AVANT=<tête du lot d'avant> remplace alors deux contrôles :
#   (b) <avant> est un ancêtre du commit testé : le rejeu s'est bien fait sur la pile ;
#   (c) `code_apporte(<avant>, master)` est vide : les lots d'avant ont atterri avec le contenu testé, sinon
#       « la pile a changé », et c'est un rejeu ;
#   (a) `git range-diff <avant>..<testé> <master>..HEAD` : `=` pour tout commit ; un `!` toléré si ses deux
#       versions ne diffèrent que par des `.md` (un conflit de nouveautes.md résolu au rebase) ; un `>` (commit
#       ajouté) toléré s'il ne touche que des `.md` ; un `<` (commit perdu) refusé, sauf si son patch est déjà sur master.
# Mode `--pile <avant> <testé> <master> <tête>`, joué par le témoin : rend 0 et « pile tenue », ou 1 et la raison.
# **La tête déjà reposée à la main** (14 novembre 2026, c7, relevé d'embarquement) : quand le `rebase --onto` de la
# garde bute (un conflit de nouveautes.md, où chaque lot ajoute sa ligne en tête), la session repose son lot à la
# main sur une branche à part, sur master, résout le conflit, puis donne PILE_AVANT=<avant> et PILE_TESTE=<commit
# testé>. La garde ne rebase pas : HEAD doit déjà être posé sur le master qu'elle relit (sinon ARRÊT, à reposer de
# nouveau), PILE_TESTE doit être le commit testé du résumé, et les mêmes vérifications (b), (c), (a) jugent.
verifier_pile() {
  local avant=$1 teste=$2 maitre=$3 tete=$4 apporte ligne c
  git merge-base --is-ancestor "$avant" "$teste" || { echo "ARRÊT : la tête d'avant ($avant) n'est pas un ancêtre du commit testé ($teste)"; return 1; }
  apporte=$(code_apporte "$avant" "$maitre")
  [ -z "$apporte" ] || { echo "ARRÊT : la pile a changé — master a du code que la tête d'avant n'avait pas :"; echo "$apporte"; return 1; }
  local ancien signe nouveau range_output header_count=0
  local header_pattern='^[[:blank:]]*([0-9]+|-):[[:blank:]]+([[:xdigit:]]+|-+)[[:blank:]]+([=!<>])[[:blank:]]+([0-9]+|-):[[:blank:]]+([[:xdigit:]]+|-+)([[:blank:]].*)?$'
  echo "range-diff jugé ($(git rev-parse --short "$avant")..$(git rev-parse --short "$teste") contre $(git rev-parse --short "$maitre")..$(git rev-parse --short "$tete")) :"
  # --creation-factor=100 : un petit commit dont seule une ligne de .md a changé (un fichier créé devenu un fichier
  # modifié par un conflit résolu) serait sinon vu « < » puis « > », deux commits sans rapport.
  # Les corps de patch ne sont pas des en-têtes ; leur comparaison reste faite ci-dessous.
  range_output=$(git range-diff --no-color --no-patch --creation-factor=100 "$avant..$teste" "$maitre..$tete") || {
    echo "ARRÊT : range-diff a échoué"; return 1;
  }
  while IFS= read -r ligne; do
    [ -n "$ligne" ] || continue
    # Git aligne les ordinaux sur la largeur maximale, y compris les « -: ».
    [[ "$ligne" =~ $header_pattern ]] || { echo "ARRÊT : en-tête range-diff invalide : $ligne"; return 1; }
    ancien=${BASH_REMATCH[2]}; signe=${BASH_REMATCH[3]}; nouveau=${BASH_REMATCH[5]}
    header_count=$((header_count + 1))
    case "$signe" in
      =) echo "  = égal : $ligne" ;;
      '>')
        [ -z "$(git diff --name-only "$nouveau^" "$nouveau" -- . ':!*.md')" ] || { echo "ARRÊT : un commit ajouté après le rejeu touche autre chose que de la doc : $ligne"; return 1; }
        echo "  > toléré, .md seuls : $ligne" ;;
      '!')
        # Toléré si les deux versions du commit ne diffèrent que par des .md : le même diff hors .md.
        if [ "$(git diff --no-color "$ancien^!" -- . ':!*.md' | grep -vE '^(index |@@)')" != "$(git diff --no-color "$nouveau^!" -- . ':!*.md' | grep -vE '^(index |@@)')" ]; then
          echo "ARRÊT : un commit du lot a changé hors des .md depuis le rejeu : $ligne"; return 1
        fi
        echo "  ! toléré, ne diffère que par des .md : $ligne" ;;
      '<')
        # **Un commit du lot déjà sur master** (15 novembre 2026, relevé de c7) : poussé par un autre lot, son patch tombe au
        # rebase comme déjà appliqué, et le range-diff le dit perdu. Toléré si `git cherry` le marque « - » contre master.
        if [ "$(git cherry "$maitre" "$ancien" "$ancien^" 2>/dev/null)" = "- $(git rev-parse "$ancien")" ]; then
          echo "  < toléré, déjà sur master : $ligne"
        else
          echo "ARRÊT : un commit du lot manque, et son patch n'est pas sur master : $ligne"; return 1
        fi ;;
      *) echo "ARRÊT : le lot n'est plus celui qui a été testé : $ligne"; return 1 ;;
    esac
  done <<< "$range_output"
  [ "$header_count" -gt 0 ] || { echo "ARRÊT : aucun en-tête range-diff jugé"; return 1; }
  echo "pile tenue : les commits du lot sont ceux du rejeu, et la pile d'avant est sur master"
}
if [ "${1:-}" = "--pile" ]; then verifier_pile "$2" "$3" "$4" "$5"; exit $?; fi
# **La dérive se voit** (c7, 13 novembre 2026) : la copie exécutée (au cache, `~/.cache/rag3weaver-build/`)
# fait autorité — un lot ne peut pas affaiblir la garde qui le juge ; elle dit à chaque exécution son hash et
# celui de `tools/poste/push_garde.sh` dans l'arbre poussé, et « garde en retard sur le dépôt » quand ils
# diffèrent, sans refuser. Mode `--derive <arbre>`, joué par le témoin.
derive() {
  local moi depot
  moi=$(sha256sum "$MOI" | cut -c1-16)
  if [ -f "$1/tools/poste/push_garde.sh" ]; then
    depot=$(sha256sum "$1/tools/poste/push_garde.sh" | cut -c1-16)
  else
    depot="absente"
  fi
  echo "garde exécutée $moi, garde du dépôt $depot"
  if [ "$moi" = "$depot" ]; then echo "garde à jour"; return 0; fi
  # Le sens (14 novembre 2026, relevé de c7) : la copie exécutée est-elle une version passée de la garde du dépôt
  # (le dépôt l'a changée depuis) ou une version qu'il n'a pas encore (une branche relue, posée avant d'atterrir) ?
  local blob c passe=0
  blob=$(git hash-object "$MOI" 2>/dev/null)
  if [ -n "$blob" ] && git -C "$1" rev-parse -q --verify HEAD > /dev/null 2>&1; then
    for c in $(git -C "$1" rev-list HEAD -- tools/poste/push_garde.sh 2>/dev/null); do
      [ "$(git -C "$1" rev-parse -q "$c:tools/poste/push_garde.sh" 2>/dev/null)" = "$blob" ] && { passe=1; break; }
    done
  fi
  if [ "$passe" = 1 ]; then
    echo "garde différente : le dépôt a changé la garde depuis la copie exécutée — copie à relire et à recopier (la copie exécutée fait autorité jusque-là)"
  else
    echo "garde différente : la copie exécutée porte ce que le dépôt n'a pas encore (la copie exécutée fait autorité)"
  fi
}
if [ "${1:-}" = "--derive" ]; then derive "$2"; exit 0; fi
# **Les suites qu'exige le lot** (16 novembre 2026, ticket de c7) : master a rougi sur e2e_code_sync quand chaque passe
# disait « e2e_code 36/36 ». `required_suites_played <base> <testé> <journaux>` lit tools/poste/required_suites.txt de
# l'arbre (par required_suites.sh) : chaque suite qu'exige un chemin du lot doit avoir sa ligne « == <suite> » au résumé
# et un journal <suite>.log vert. Un arbre sans la table n'exige rien, et la garde le dit. Mode
# `--required-suites <base> <testé> <journaux>`, joué par le témoin.
required_suites_played() {
  local base=$1 teste=$2 J=$3 exigees ligne suite chemin manque=""
  if [ ! -x tools/poste/required_suites.sh ]; then echo "suites exigées : pas de table dans l'arbre (tools/poste/required_suites.txt)"; return 0; fi
  exigees=$(bash tools/poste/required_suites.sh "$base" "$teste") || { echo "ARRÊT : required_suites.sh a échoué ($base..$teste)"; return 1; }
  [ -n "$exigees" ] || return 0
  while read -r suite chemin; do
    [ -n "$suite" ] || continue
    if grep -qx "== $suite" "$J/resume.log" && grep -q '^test result: ok' "$J/$suite.log" 2>/dev/null && ! grep -q '^test result: FAILED' "$J/$suite.log"; then
      # Un test qui dit « non jugé » et rend la main compte parmi les passés de cargo (18 novembre 2026, ticket de c7).
      if grep -q '^non jugé : ' "$J/$suite.log"; then
        manque="$manque $suite ($chemin) : $(grep -c '^non jugé : ' "$J/$suite.log") test(s) non jugé(s)"
        continue
      fi
      echo "suite exigée jouée verte : $suite ($chemin)"
    else
      manque="$manque $suite ($chemin)"
    fi
  done <<< "$exigees"
  [ -z "$manque" ] || { echo "ARRÊT : le lot exige des suites que la passe n'a pas jouées vertes :$manque"; return 1; }
}
if [ "${1:-}" = "--required-suites" ]; then required_suites_played "$2" "$3" "$4"; exit $?; fi
# **Le mode tests seuls** (16 novembre 2026, ticket d'hotfixs), pendant du mode docs : sous TESTS_ONLY=1, un lot qui ne
# touche que des dossiers de tests déclarés (tools/poste/tests_only_dirs.txt de l'arbre), plus des .md, passe sans lib.
# Sa preuve est la suite jouée contre un binaire : un journal de la passe doit le nommer, avec sa provenance, par une
# ligne « tested binary: <chemin> commit <40 hex> sha256 <64 hex> ». Les autres contrôles restent. Mode
# `--tests-only <base> <testé> <journaux>`, joué par le témoin.
tests_only_check() {
  local base=$1 teste=$2 J=$3 dirs f d ok hors="" prov
  [ -f tools/poste/tests_only_dirs.txt ] || { echo "ARRÊT : mode tests seuls sans dossiers déclarés (tools/poste/tests_only_dirs.txt)"; return 1; }
  dirs=$(grep -vE '^[[:space:]]*(#|$)' tools/poste/tests_only_dirs.txt)
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in *.md) continue ;; esac
    ok=""; for d in $dirs; do case "$f" in "${d%/}"/*) ok=1 ;; esac; done
    [ -n "$ok" ] || hors="$hors $f"
  done < <(git diff --name-only "$base" "$teste")
  [ -z "$hors" ] || { echo "ARRÊT : mode tests seuls, mais le lot touche hors des dossiers de tests déclarés ($(echo $dirs)) :$hors"; return 1; }
  prov=$(grep -hE '^tested binary: [^ ]+ commit [0-9a-f]{40} sha256 [0-9a-f]{64}$' "$J"/*.log 2>/dev/null)
  [ -n "$prov" ] || { echo "ARRÊT : mode tests seuls, mais aucun journal de la passe ($J/*.log) ne dit « tested binary: <chemin> commit <40 hex> sha256 <64 hex> »"; return 1; }
  echo "mode tests seuls : le lot ne touche que $(echo $dirs) ; binaire joué :"; echo "$prov" | sed 's/^/  /'
}
if [ "${1:-}" = "--tests-only" ]; then tests_only_check "$2" "$3" "$4"; exit $?; fi
# Master tenu pour une autre session (fenêtre de c7) : pas de push tant que le fichier existe.
[ -e $HOME/.cache/rag3weaver-build/master-tenu ] && { echo "ARRÊT : master tenu — $(cat $HOME/.cache/rag3weaver-build/master-tenu)"; exit 1; }
J=$HOME/.cache/rag3weaver-build/$2
cd $HOME/git_workspaces/$1 || exit 1
derive "$PWD"
gd=$(git rev-parse --git-dir)
tail -1 $J/resume.log | grep -qx VERT || { echo "ARRÊT : la passe n'est pas VERTE"; exit 1; }
teste=$(head -1 $J/resume.log | awk '{print $2}')
# Les suites qu'exige le lot, sur la base de la passe (ou, sans elle, sur master sous le commit testé).
base_exigees=$(cat "$J/base" 2>/dev/null || git merge-base "$teste" origin/master)
required_suites_played "$base_exigees" "$teste" "$J" || exit 1
if [ -n "${PILE_TESTE:-}" ]; then
  [ -n "${PILE_AVANT:-}" ] || { echo "ARRÊT : PILE_TESTE sans PILE_AVANT (la tête du lot d'avant)"; exit 1; }
  [ "$(git rev-parse -q --verify "$PILE_TESTE^{commit}")" = "$(git rev-parse -q --verify "$teste^{commit}")" ] || { echo "ARRÊT : PILE_TESTE ($PILE_TESTE) n'est pas le commit testé du résumé ($teste)"; exit 1; }
fi
# HEAD est le commit testé, ou en descend par des commits de doc seuls (des .md). En mode pile, c'est
# verifier_pile qui juge, plus bas, une fois master relu.
# Les commits entiers, jamais deux formes courtes (15 novembre 2026) : `--short` rend la longueur qu'exige le dépôt (10 ici),
# le résumé en écrit souvent 9, et la garde disait « de la doc seule » d'une tête égale au testé.
if [ -z "${PILE_AVANT:-}" ] && [ "$(git rev-parse HEAD)" != "$(git rev-parse -q --verify "$teste^{commit}")" ]; then
  git merge-base --is-ancestor "$teste" HEAD || { echo "ARRÊT : HEAD ne descend pas du commit testé ($teste)"; exit 1; }
  ajoute=$(git diff --name-only "$teste" HEAD -- . ':!*.md')
  [ -z "$ajoute" ] || { echo "ARRÊT : HEAD ajoute autre chose que de la doc au commit testé ($teste) :"; echo "$ajoute"; exit 1; }
  echo "HEAD ajoute de la doc seule au commit testé ($teste)"
fi
[ -z "$(git status --porcelain)" ] || { echo "ARRÊT : arbre sale"; exit 1; }
[ "$(git config user.email)" = "luciedefraiteur@gmail.com" ] || { echo "ARRÊT : identité git"; exit 1; }
# La lib se cherche dans **tous les journaux** de la passe (`$J/*.log`, 14 novembre 2026, relevé de tty) : une passe
# courte (crate.log, e2e_tty.log…) n'avait aucun des journaux d'une liste fixe. Une seule lib, et au moins un
# journal qui la nomme : zéro reste un ARRÊT.
if [ "${TESTS_ONLY:-}" = 1 ]; then
  tests_only_check "$base_exigees" "$teste" "$J" || exit 1
else
libs=$(grep -ho 'rag3db-build-[0-9a-f]\{40\}' "$J"/*.log 2>/dev/null | sort -u)
case "$(echo "$libs" | grep -c .)" in
  1) ;;
  0) echo "ARRÊT : aucun journal de la passe ($J/*.log) ne nomme une lib du magasin (un lot de tests seuls : TESTS_ONLY=1)"; exit 1 ;;
  *) echo "ARRÊT : la passe a vu plusieurs libs du magasin :"; echo "$libs"; exit 1 ;;
esac
echo "lib de toute la passe : $libs"
fi
git fetch -q origin master || exit 1
neuf=$(git rev-parse FETCH_HEAD)
# Le C++ du moteur (src/, test/) n'est pas lié par la lib épinglée : il se rebase sans rejeu
# (docs/procedures/push-d-un-lot.md).
# **Revenir sur la tête d'avant le rebase** (14 novembre 2026, relevé de lucivy) : un push refusé (master a
# bougé entre le fetch et le push) ou un ARRÊT après le rebase laissait le worktree rebasé, sur des copies des
# commits testés ; le passage suivant s'arrêtait sur « HEAD ne descend pas du commit testé ». La garde remet le
# worktree où il était avant son rebase, et le dit : le même appel, rejoué, repart du commit testé.
avant_rebase=$(git rev-parse HEAD)
revenir() { git reset -q --keep "$avant_rebase" 2>/dev/null || git checkout -q --detach "$avant_rebase"; echo "worktree remis sur $(git rev-parse --short "$avant_rebase"), la tête d'avant le rebase : rejouer le même appel"; }
if [ -n "${PILE_TESTE:-}" ]; then
  echo "mode pile, tête reposée à la main, sans rebase par la garde : tête d'avant $(git rev-parse --short "$PILE_AVANT"), commit testé $teste, HEAD $(git rev-parse --short HEAD)"
  git merge-base --is-ancestor "$neuf" HEAD || { echo "ARRÊT : la tête reposée à la main n'est pas posée sur master ($(git rev-parse --short "$neuf")) : master a bougé depuis, la reposer de nouveau"; exit 1; }
  verifier_pile "$PILE_AVANT" "$teste" "$neuf" HEAD || exit 1
elif [ -n "${PILE_AVANT:-}" ]; then
  echo "mode pile : tête d'avant $(git rev-parse --short "$PILE_AVANT"), commit testé $teste"
  # `--onto` (relecture de recherche) : HEAD est encore la branche de pile, avec les commits des lots d'avant sous
  # leurs anciens hashes ; un lot d'avant qui a résolu un conflit en atterrissant a un autre patch, et un rebase
  # simple le rejouerait. Seuls les commits du lot se reposent.
  git rebase --onto $neuf "$PILE_AVANT" > /dev/null 2>&1
  if [ -d "$gd/rebase-merge" ] || [ -d "$gd/rebase-apply" ] || git status --short | grep -q '^UU'; then
    git rebase --abort; echo "ARRÊT : le rebase sur $neuf bute — reposer le lot à la main sur une branche à part (rebase --onto origin/master $PILE_AVANT), résoudre, puis rejouer la garde depuis elle avec PILE_TESTE=$teste"; exit 1
  fi
  verifier_pile "$PILE_AVANT" "$teste" "$neuf" HEAD || { revenir; exit 1; }
else
  # La base du rejeu ne sert que hors du mode pile (relevé de tty : lue avant, elle faisait « cat: …/base ») .
  base=$(cat "$J/base") || { echo "ARRÊT : pas de base de rejeu ($J/base)"; exit 1; }
  apporte=$(code_apporte $base $neuf)
  if [ -n "$apporte" ] && cpp_transferable "$base" "$neuf" HEAD; then
    echo "suites C++ transférables : le lot ne touche que le moteur ($CHEMINS_MOTEUR), et master n'y a rien changé depuis la base testée ; le code apporté est hors du moteur :"
    echo "$apporte" | sed 's/^/  /'
    apporte=""
  fi
  if [ -n "$apporte" ]; then echo "ARRÊT : master a apporté du code depuis la base testée"; echo "$apporte"; exit 1; fi
  echo "master $(git rev-parse --short $neuf), base testée $(git rev-parse --short $base) : rien de code entre les deux"
  git rebase $neuf > /dev/null 2>&1
  if [ -d "$gd/rebase-merge" ] || [ -d "$gd/rebase-apply" ] || git status --short | grep -q '^UU'; then
    git rebase --abort; echo "ARRÊT : le rebase sur $neuf bute"; exit 1
  fi
fi
# **La copie ouverte d'un ticket fermé par le lot** (15 novembre 2026, relevé de c7) : un `git mv` vers
# docs/tickets/closed/ rejoué sur une base de pile plus vieille que le ticket ouvert ne fait que créer la copie fermée ;
# master en portait deux et check_tickets rougissait la CI. La garde retire l'homonyme ouvert dans le même push, par un
# commit de docs seules, et le dit.
retires=""
while IFS= read -r ferme; do
  [ -n "$ferme" ] || continue
  nom=$(basename "$ferme")
  while IFS= read -r ouvert; do
    [ -n "$ouvert" ] || continue
    git rm -q -- "$ouvert" && retires="$retires $ouvert"
  done < <(git ls-tree -r --name-only HEAD -- docs | awk -v n="$nom" -F/ '$NF == n && $(NF-1) == "tickets" && $0 !~ "^docs/tickets/closed/"')
done < <(git diff --name-only --diff-filter=A "$neuf" HEAD -- docs/tickets/closed/)
if [ -n "$retires" ]; then
  git commit -q -m "docs(tickets): la copie ouverte d'un ticket que le lot ferme est retirée par la garde —$retires" -- $retires || { echo "ARRÊT : le commit qui retire les copies ouvertes a échoué"; revenir; exit 1; }
  echo "copies ouvertes retirées dans le même push :$retires"
fi
# **La PR du lot** (19 novembre 2026, c7 et G) : le pousseur ouvre une PR par lot avant le push. PRE_PUSH, quand il est
# posé, est une commande appelée avec la tête finale : après le rebase sur master (ou le --onto de la pile) et après le
# commit des copies ouvertes, juste avant le push. Elle ne bloque jamais : son échec est dit, et le push se fait.
if [ -n "${PRE_PUSH:-}" ]; then
  # shellcheck disable=SC2086
  ${PRE_PUSH} "$(git rev-parse HEAD)" || echo "PR absente : le crochet PRE_PUSH a rendu $?"
fi
git push -q origin HEAD:master || { echo "ARRÊT : push refusé"; revenir; exit 1; }
git fetch -q origin master
echo "== trois contrôles"
echo "status : [$(git status --porcelain)]"
if [ -d "$gd/rebase-merge" ] || [ -d "$gd/rebase-apply" ]; then echo "REBASE EN COURS"; else echo "aucun rebase"; fi
echo "master (FETCH_HEAD) $(git rev-parse --short FETCH_HEAD) HEAD $(git rev-parse --short HEAD)"
[ "$(git rev-parse FETCH_HEAD)" = "$(git rev-parse HEAD)" ] && echo POUSSE
