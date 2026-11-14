#!/bin/bash
# Témoin de push_garde.sh --code-apporte (ticket
# docs/recherche/tickets/2026-11-13-la-garde-du-push-compte-un-outil-de-mesure-comme-du-code.md) :
# dans un dépôt jetable, chaque cas ajoute un fichier et dit s'il oblige à rejouer la passe.
# Usage : bash tools/poste/test_push_garde.sh [chemin de push_garde.sh] — sortie 0 si tout est vert.
set -u
GARDE=$(realpath "${1:-$(dirname "$0")/push_garde.sh}")
D=$(mktemp -d); trap 'rm -rf "$D"' EXIT
cd "$D" && git init -q && git config user.email t@t && git config user.name t
mkdir -p extension/rag3weaver/{src,tests,scripts,templates/backends/code,docs}
echo 'fn main() {}' > extension/rag3weaver/src/lib.rs
printf '#!/bin/bash\nbash scripts/cargo_magasin.sh test\npython3 scripts/appele.py\nbash scripts/relais.sh\n' > extension/rag3weaver/run_e2e.sh
printf '#!/bin/bash\ncargo check\n' > extension/rag3weaver/scripts/check_features.sh
printf '#!/bin/bash\ncargo test\n' > extension/rag3weaver/scripts/cargo_magasin.sh
printf '#!/bin/bash\npython3 scripts/sous_relais.py\n' > extension/rag3weaver/scripts/relais.sh
echo '#[test] fn t() { std::process::Command::new("python3").arg("scripts/par_un_test.py"); }' > extension/rag3weaver/tests/e2e_x.rs
echo 'print(1)' > extension/rag3weaver/scripts/appele.py
git add -A && git commit -qm base && BASE=$(git rev-parse HEAD)
rouge=0
cas() { # cas <attendu: code|docs> <chemin> <contenu>
  git checkout -q "$BASE" 2>/dev/null; mkdir -p "$(dirname "$2")"; printf '%s\n' "$3" > "$2"
  git add -A && git commit -qm "$2" && N=$(git rev-parse HEAD)
  sortie=$(bash "$GARDE" --code-apporte "$BASE" "$N")
  if [ -n "$sortie" ]; then vu=code; else vu=docs; fi
  if [ "$vu" = "$1" ]; then echo "vert   $1 : $2"; else echo "ROUGE  attendu $1, vu $vu : $2"; rouge=1; fi
}
cas_git() { # cas_git <attendu> <description> <commande git…> : supprime ou renomme au lieu d'écrire
  local attendu=$1 desc=$2; shift 2
  git checkout -q "$BASE" 2>/dev/null; git "$@" > /dev/null && git commit -qm "$desc" && N=$(git rev-parse HEAD)
  sortie=$(bash "$GARDE" --code-apporte "$BASE" "$N")
  if [ -n "$sortie" ]; then vu=code; else vu=docs; fi
  if [ "$vu" = "$attendu" ]; then echo "vert   $attendu : $desc"; else echo "ROUGE  attendu $attendu, vu $vu : $desc"; rouge=1; fi
}
# Le ticket : un outil que rien de joué n'appelle compte comme de la doc.
cas docs extension/rag3weaver/scripts/outil_de_mesure.py 'print("mesure")'
cas docs extension/rag3weaver/docs/note.md '# note'
cas docs extension/rag3weaver/docs/outils/sonde.sh 'echo sonde'
# Ce qui reste du code.
cas code extension/rag3weaver/scripts/appele.py 'print("appelé par run_e2e.sh")'
# Supprimer ou renommer un script appelé : la passe appellerait un fichier absent (relecture de recherche).
cas_git code "suppression du script appelé scripts/appele.py" rm extension/rag3weaver/scripts/appele.py
cas_git code "renommage du script appelé scripts/appele.py" mv extension/rag3weaver/scripts/appele.py extension/rag3weaver/scripts/renomme.py
cas code extension/rag3weaver/scripts/sous_relais.py 'print("appelé par un script appelé")'
cas code extension/rag3weaver/scripts/par_un_test.py 'print("appelé par un test")'
cas code extension/rag3weaver/scripts/check_features.sh 'cargo check --all'
cas code extension/rag3weaver/run_e2e.sh 'cargo test'
cas code extension/rag3weaver/src/lib.rs 'fn main() { println!(); }'
cas code extension/rag3weaver/src/ide/page.html '<html></html>'
cas code extension/rag3weaver/templates/backends/code/backend.json '{}'
cas code extension/rag3weaver/Cargo.toml '[package]'
cas code Cargo.lock 'version = 3'
cas code tools/rust_api/src/lib.rs 'fn f() {}'
# Le mode pile (14 novembre 2026) : une pile de deux lots, P1 (code) puis P2 (code), P2 rejoué sur P1 ;
# au dernier pas, P1 a atterri sur master et P2 est reposé par `rebase --onto master P1`.
pile() { # pile <attendu: tenue|arret> <description> [motif] : juge la pile préparée dans M, P1, P2T (testé), TETE.
  # Un arrêt doit porter son motif (relecture de recherche) : sans lui, n'importe quel arrêt ferait passer le cas.
  local sortie rc motif=${3:-}
  sortie=$(bash "$GARDE" --pile "$P1" "$P2T" "$M" "$TETE" 2>&1); rc=$?;
  if [ "$1" = arret ] && [ -z "$motif" ]; then echo "ROUGE  pile arret sans motif attendu : $2"; rouge=1; return; fi
  if { [ "$1" = tenue ] && [ $rc = 0 ] && { [ -z "$motif" ] || echo "$sortie" | grep -qF -- "$motif"; }; } || { [ "$1" = arret ] && [ $rc != 0 ] && echo "$sortie" | grep -qF -- "$motif"; }; then echo "vert   pile $1 : $2"
  else echo "ROUGE  pile, attendu $1 (rc $rc) : $2 — $sortie"; rouge=1; fi
}
preparer_pile() { # le master d'origine, P1 sur lui, P2 sur P1 (le commit testé)
  git checkout -q -B m0 "$BASE"; M0=$(git rev-parse HEAD)
  git checkout -q -B p1 "$M0"; echo 'fn p1() {}' > extension/rag3weaver/src/p1.rs; git add -A; git commit -qm p1; P1=$(git rev-parse HEAD)
  git checkout -q -B p2 "$P1"; echo 'fn p2() {}' > extension/rag3weaver/src/p2.rs; mkdir -p extension/rag3weaver/docs; echo '- p2' >> extension/rag3weaver/docs/nouveautes.md; git add -A; git commit -qm p2; P2T=$(git rev-parse HEAD)
}
# 1. Le cas vert : P1 atterri tel quel sur master, P2 reposé dessus.
preparer_pile
git checkout -q -B maitre "$P1"; M=$(git rev-parse HEAD)
git checkout -q -B p2b "$P2T"; git rebase -q --onto "$M" "$P1" >/dev/null 2>&1; TETE=$(git rev-parse HEAD)
pile tenue "P1 atterri tel quel, P2 reposé dessus (le range-diff jugé imprimé)" "  = égal :"
# 2. La pile a changé : P1 a atterri avec un autre code que celui sur lequel P2 a été rejoué.
git checkout -q -B maitre "$M0"; echo 'fn p1() { autre(); }' > extension/rag3weaver/src/p1.rs; git add -A; git commit -qm "p1 changé"; M=$(git rev-parse HEAD)
git checkout -q -B p2b "$P2T"; git rebase -q --onto "$M" "$P1" >/dev/null 2>&1; TETE=$(git rev-parse HEAD)
pile arret "P1 a atterri avec un autre code : la pile a changé" "la pile a changé"
# 3. Un .md ajouté après le rejeu (le ticket posé après le rebase) : toléré.
git checkout -q -B maitre "$P1"; M=$(git rev-parse HEAD)
git checkout -q -B p2b "$P2T"; git rebase -q --onto "$M" "$P1" >/dev/null 2>&1
echo '# ticket' > extension/rag3weaver/docs/ticket.md; git add -A; git commit -qm "ticket"; TETE=$(git rev-parse HEAD)
pile tenue "un commit de .md ajouté après le rejeu" "  > toléré, .md seuls :"
# 4. Du code ajouté après le rejeu : refusé.
echo 'fn tard() {}' > extension/rag3weaver/src/tard.rs; git add -A; git commit -qm "code tardif"; TETE=$(git rev-parse HEAD)
pile arret "un commit de code ajouté après le rejeu" "un commit ajouté après le rejeu touche autre chose"
# 5. Un conflit de nouveautes.md résolu au rebase (un `!` qui ne diffère que par des .md) : toléré.
git checkout -q -B maitre "$P1"; mkdir -p extension/rag3weaver/docs; echo '- autre lot' >> extension/rag3weaver/docs/nouveautes.md; git add -A; git commit -qm "nouveauté d'un autre"; M=$(git rev-parse HEAD)
git checkout -q -B p2b "$P2T"; git rebase -q --onto "$M" "$P1" >/dev/null 2>&1 || { printf -- '- autre lot\n- p2\n' > extension/rag3weaver/docs/nouveautes.md; git add -A; GIT_EDITOR=true git rebase --continue >/dev/null 2>&1; }
TETE=$(git rev-parse HEAD)
git range-diff --no-color --creation-factor=100 "$P1..$P2T" "$M..$TETE" | grep -q ' ! ' || { echo "ROUGE  le cas 5 n'a pas produit de « ! » : il ne prouve rien"; rouge=1; }
pile tenue "un conflit de nouveautes.md résolu au rebase (! sur des .md seuls)" "  ! toléré, ne diffère que par des .md :"
# 6. Un commit du lot changé hors .md au rebase : refusé.
git checkout -q -B p2b "$P2T"; git rebase -q --onto "$M" "$P1" >/dev/null 2>&1 || { printf -- '- autre lot\n- p2\n' > extension/rag3weaver/docs/nouveautes.md; git add -A; GIT_EDITOR=true git rebase --continue >/dev/null 2>&1; }
echo 'fn p2() { change(); }' > extension/rag3weaver/src/p2.rs; git add -A; git commit -q --amend --no-edit; TETE=$(git rev-parse HEAD)
pile arret "un commit du lot changé hors des .md au rebase (! sur du code)" "a changé hors des .md"
# 7. La tête d'avant n'est pas un ancêtre du commit testé : le rejeu ne s'est pas fait sur la pile.
git checkout -q -B maitre "$P1"; M=$(git rev-parse HEAD)
git checkout -q -B p2b "$P2T"; git rebase -q --onto "$M" "$P1" >/dev/null 2>&1; TETE=$(git rev-parse HEAD)
git checkout -q -B ailleurs "$M0"; mkdir -p extension/rag3weaver/docs; echo '# ailleurs' > extension/rag3weaver/docs/ailleurs.md; git add -A; git commit -qm ailleurs
P1_VRAI=$P1; P1=$(git rev-parse HEAD); pile arret "la tête d'avant n'est pas un ancêtre du commit testé" "n'est pas un ancêtre"; P1=$P1_VRAI
git checkout -q "$BASE" 2>/dev/null
# 8. Le chemin de push complet en mode pile (relecture de recherche, 14 novembre 2026) : HEAD est encore la
# branche de pile (P1 puis P2) ; P1 a atterri sur master après un conflit de nouveautes.md, donc sous un autre
# hash et avec un autre patch. La garde doit pousser P2 seul. Rouge avec un rebase simple, vert avec `--onto`.
(
  H="$D/home8"; mkdir -p "$H/git_workspaces" "$H/.cache/rag3weaver-build/j"
  git clone -q --bare "$D" "$D/nu8.git" 2>/dev/null; git -C "$D/nu8.git" update-ref refs/heads/master "$BASE"
  git clone -q "$D/nu8.git" "$H/git_workspaces/wt" 2>/dev/null
  cd "$H/git_workspaces/wt" && git config user.email luciedefraiteur@gmail.com && git config user.name t
  git checkout -q -B pile origin/master
  echo 'fn p1() {}' > extension/rag3weaver/src/p1.rs; mkdir -p extension/rag3weaver/docs; echo '- p1' >> extension/rag3weaver/docs/nouveautes.md; git add -A; git commit -qm p1; P1=$(git rev-parse HEAD)
  echo 'fn p2() {}' > extension/rag3weaver/src/p2.rs; git add -A; git commit -qm p2; TESTE=$(git rev-parse --short HEAD)
  # master : la nouveauté d'un autre, puis P1 atterri dessus avec le conflit de nouveautes.md résolu.
  git checkout -q -B atterrissage origin/master; mkdir -p extension/rag3weaver/docs; echo '- autre' >> extension/rag3weaver/docs/nouveautes.md; git add -A; git commit -qm autre
  git cherry-pick "$P1" > /dev/null 2>&1 || { printf -- '- autre\n- p1\n' > extension/rag3weaver/docs/nouveautes.md; git add -A; GIT_EDITOR=true git cherry-pick --continue > /dev/null 2>&1; }
  git push -q origin HEAD:master 2>/dev/null; git checkout -q pile
  printf 'teste %s\nVERT\n' "$TESTE" > "$H/.cache/rag3weaver-build/j/resume.log"
  # une passe courte : la lib n'est nommée que dans e2e_tty.log (relevé de tty), et pas de fichier base en pile
  echo "rag3db-build-0123456789012345678901234567890123456789-Release" > "$H/.cache/rag3weaver-build/j/e2e_tty.log"
  sortie=$(HOME="$H" PILE_AVANT="$P1" bash "$GARDE" wt j 2>&1); rc=$?
  sujets=$(git -C "$D/nu8.git" log --format=%s master | tr '\n' ' ')
  if [ $rc = 0 ] && echo "$sortie" | grep -qx POUSSE && [ "$sujets" = "p2 p1 autre base " ] && ! echo "$sortie" | grep -q "Aucun fichier"; then
    echo "vert   pile, chemin de push : P1 atterri après un conflit, P2 seul poussé ($sujets), lib lue dans e2e_tty.log, base non lue"; exit 0
  fi
  echo "ROUGE  pile, chemin de push : rc $rc, master « $sujets »"; echo "$sortie" | tail -4; exit 1
) || rouge=1
# 9 à 11. La tête déjà reposée à la main (c7, 14 novembre 2026, relevé d'embarquement) : le `rebase --onto` de la
# garde bute sur un conflit de nouveautes.md (chaque lot y ajoute sa ligne en tête) ; la session le résout à la main
# sur une branche à part, déjà posée sur master, et la donne avec PILE_TESTE=<commit testé>. Mêmes vérifications
# qu'en mode pile, **sans rebase par la garde**, puis le push. Un HOME jetable, un dépôt nu.
tete_a_la_main() { # tete_a_la_main <n> <ajout: rien|code|master-bouge> ; écrit dans $D/res-<n> : rc, sortie, sujets, tête avant/après
  local n=$1 ajout=$2
  (
    H="$D/homem$n"; mkdir -p "$H/git_workspaces" "$H/.cache/rag3weaver-build/j"
    git clone -q --bare "$D" "$D/num$n.git" 2>/dev/null; git -C "$D/num$n.git" update-ref refs/heads/master "$BASE"
    git clone -q "$D/num$n.git" "$H/git_workspaces/wt" 2>/dev/null
    cd "$H/git_workspaces/wt" && git config user.email luciedefraiteur@gmail.com && git config user.name t
    # la pile : P1 puis P2 (le commit testé), chacun avec sa ligne de nouveautes.md
    git checkout -q -B pile origin/master; mkdir -p extension/rag3weaver/docs
    echo 'fn p1() {}' > extension/rag3weaver/src/p1.rs; echo '- p1' >> extension/rag3weaver/docs/nouveautes.md; git add -A; git commit -qm p1; P1=$(git rev-parse HEAD)
    echo 'fn p2() {}' > extension/rag3weaver/src/p2.rs; echo '- p2' >> extension/rag3weaver/docs/nouveautes.md; git add -A; git commit -qm p2; TESTE=$(git rev-parse --short HEAD)
    # master : P1 atterri tel quel, puis la nouveauté du cœur : P2 ne se repose plus sans conflit
    git checkout -q -B atterri origin/master; git cherry-pick "$P1" > /dev/null 2>&1
    printf -- '- p1\n- coeur\n' > extension/rag3weaver/docs/nouveautes.md; git add -A; git commit -qm coeur; git push -q origin HEAD:master 2>/dev/null
    # la tête reposée à la main : P2 sur master, le conflit résolu (les deux lignes)
    git checkout -q -B a-la-main origin/master
    git cherry-pick "$TESTE" > /dev/null 2>&1 || { printf -- '- p1\n- coeur\n- p2\n' > extension/rag3weaver/docs/nouveautes.md; git add -A; GIT_EDITOR=true git cherry-pick --continue > /dev/null 2>&1; }
    case $ajout in
      code) echo 'fn glisse() {}' > extension/rag3weaver/src/glisse.rs; git add -A; git commit -qm "code glissé" ;;
      master-bouge) ( cd "$D" && git clone -q "$D/num$n.git" "$D/autre$n" 2>/dev/null && cd "$D/autre$n" && git config user.email t@t && git config user.name t && echo '# autre' > autre.md && git add -A && git commit -qm "un autre lot" && git push -q origin HEAD:master 2>/dev/null ) ;;
    esac
    printf 'teste %s\nVERT\n' "$TESTE" > "$H/.cache/rag3weaver-build/j/resume.log"
    echo "rag3db-build-0123456789012345678901234567890123456789-Release" > "$H/.cache/rag3weaver-build/j/lib.log"
    avant=$(git rev-parse HEAD)
    sortie=$(HOME="$H" PILE_AVANT="$P1" PILE_TESTE="$TESTE" bash "$GARDE" wt j 2>&1); rc=$?
    { echo "$rc"; echo "$avant"; git rev-parse HEAD; git -C "$D/num$n.git" log --format=%s master | tr '\n' ' '; echo; echo "$sortie"; } > "$D/res-$n"
  )
}
tete_a_la_main 9 rien; { read -r rc; read -r avant; read -r apres; read -r sujets; sortie=$(cat); } < "$D/res-9"
if [ "$rc" = 0 ] && echo "$sortie" | grep -qx POUSSE && [ "$sujets" = "p2 coeur p1 base" ] && [ "$avant" = "$apres" ] && echo "$sortie" | grep -q "sans rebase par la garde" && echo "$sortie" | grep -q "  ! toléré"; then
  echo "vert   tête reposée à la main : conflit de nouveautes.md résolu, pile tenue sans rebase par la garde, P2 poussé ($sujets)"
else echo "ROUGE  tête reposée à la main : rc $rc, master « $sujets », tête $avant → $apres"; echo "$sortie" | tail -4; rouge=1; fi
tete_a_la_main 10 code; { read -r rc; read -r avant; read -r apres; read -r sujets; sortie=$(cat); } < "$D/res-10"
if [ "$rc" != 0 ] && echo "$sortie" | grep -q "un commit ajouté après le rejeu touche autre chose" && [ "$sujets" = "coeur p1 base" ]; then
  echo "vert   tête reposée à la main qui porte du code de plus : ARRÊT, rien poussé"
else echo "ROUGE  tête reposée à la main, code de plus : rc $rc, master « $sujets »"; echo "$sortie" | tail -3; rouge=1; fi
tete_a_la_main 11 master-bouge; { read -r rc; read -r avant; read -r apres; read -r sujets; sortie=$(cat); } < "$D/res-11"
if [ "$rc" != 0 ] && echo "$sortie" | grep -q "n'est pas posée sur master" && [ "$sujets" = "un autre lot coeur p1 base" ] && [ "$avant" = "$apres" ]; then
  echo "vert   tête reposée à la main, master a bougé depuis : ARRÊT sans rebase par la garde"
else echo "ROUGE  tête reposée à la main, master bougé : rc $rc, master « $sujets », tête $avant → $apres"; echo "$sortie" | tail -3; rouge=1; fi
# Un push refusé une fois, puis le même appel rejoué, doit pousser sans geste à la main (relevé de lucivy,
# 14 novembre 2026) : la garde remet le worktree sur sa tête d'avant le rebase. Un HOME jetable porte le
# worktree, le dossier des journaux et un dépôt nu dont le crochet refuse le premier push.
(
  H="$D/home"; mkdir -p "$H/git_workspaces" "$H/.cache/rag3weaver-build/j"
  git clone -q --bare "$D" "$D/nu.git" 2>/dev/null
  git -C "$D/nu.git" symbolic-ref HEAD refs/heads/master 2>/dev/null
  git -C "$D/nu.git" update-ref refs/heads/master "$BASE"
  printf '#!/bin/sh\n[ -e "%s/refus-fait" ] && exit 0\ntouch "%s/refus-fait"; echo "refusé une fois" >&2; exit 1\n' "$D" "$D" > "$D/nu.git/hooks/pre-receive"; chmod +x "$D/nu.git/hooks/pre-receive"
  git clone -q "$D/nu.git" "$H/git_workspaces/wt" 2>/dev/null
  cd "$H/git_workspaces/wt" && git config user.email luciedefraiteur@gmail.com && git config user.name t
  git checkout -q --detach origin/master; echo 'fn lot() {}' > extension/rag3weaver/src/lot.rs; git add -A; git commit -qm lot
  TESTE=$(git rev-parse --short HEAD)
  printf 'teste %s\nVERT\n' "$TESTE" > "$H/.cache/rag3weaver-build/j/resume.log"
  git rev-parse origin/master > "$H/.cache/rag3weaver-build/j/base"
  echo "rag3db-build-0123456789012345678901234567890123456789-Release" > "$H/.cache/rag3weaver-build/j/lib.log"
  # lancée comme au cache, par un chemin relatif : son empreinte doit rester lisible après son `cd` dans le worktree
  mkdir -p "$D/cache"; cp "$GARDE" "$D/cache/push_garde.sh"
  un=$(cd "$D/cache" && HOME="$H" bash push_garde.sh wt j 2>&1); r1=$?
  echo "$un" | grep -qE '^garde exécutée [0-9a-f]{16},' && echo "vert   la garde lancée par un chemin relatif dit son empreinte" || { echo "ROUGE  garde exécutée sans empreinte : $(echo "$un" | grep 'garde exécutée')"; rouge=1; }
  deux=$(HOME="$H" bash "$GARDE" wt j 2>&1); r2=$?
  if [ $r1 != 0 ] && echo "$un" | grep -q "push refusé" && echo "$un" | grep -q "worktree remis sur" && [ $r2 = 0 ] && echo "$deux" | grep -qx POUSSE; then
    echo "vert   push refusé une fois, puis le même appel pousse (worktree remis sur la tête testée)"; exit 0
  fi
  echo "ROUGE  push refusé puis rejoué : rc $r1 puis $r2"; echo "$un" | tail -3; echo "$deux" | tail -3; exit 1
) || rouge=1
# **La copie ouverte d'un ticket fermé par le lot** (15 novembre 2026, relevé de c7) : le lot crée
# docs/tickets/closed/t.md sur une base plus vieille que l'ouverte docs/a/tickets/t.md, arrivée sur master entre-temps.
# Après le push, master ne doit porter que la fermée.
(
  H="$D/homet"; mkdir -p "$H/git_workspaces" "$H/.cache/rag3weaver-build/j"
  git clone -q --bare "$D" "$D/nut.git" 2>/dev/null; git -C "$D/nut.git" update-ref refs/heads/master "$BASE"
  git clone -q "$D/nut.git" "$H/git_workspaces/wt" 2>/dev/null
  cd "$H/git_workspaces/wt" && git config user.email luciedefraiteur@gmail.com && git config user.name t
  git checkout -q --detach origin/master; mkdir -p docs/tickets/closed; printf '# t\n- **État** : fermé\n' > docs/tickets/closed/t.md
  echo 'fn lot() {}' > extension/rag3weaver/src/lot_t.rs; git add -A; git commit -qm "lot qui ferme t"
  printf 'teste %s\nVERT\n' "$(git rev-parse --short HEAD)" > "$H/.cache/rag3weaver-build/j/resume.log"; git rev-parse origin/master > "$H/.cache/rag3weaver-build/j/base"
  echo "rag3db-build-0123456789012345678901234567890123456789-Release" > "$H/.cache/rag3weaver-build/j/lib.log"
  # master : l'ouverte arrive après la base du lot (un commit de docs seules)
  ( git clone -q "$D/nut.git" "$D/autret" 2>/dev/null && cd "$D/autret" && git config user.email t@t && git config user.name t \
    && mkdir -p docs/a/tickets && printf '# t\n- **État** : ouvert\n' > docs/a/tickets/t.md && git add -A && git commit -qm "ticket t ouvert" && git push -q origin HEAD:master 2>/dev/null )
  sortie=$(HOME="$H" bash "$GARDE" wt j 2>&1); rc=$?
  ouverte=$(git -C "$D/nut.git" ls-tree -r --name-only master -- docs/a/tickets/t.md)
  fermee=$(git -C "$D/nut.git" ls-tree -r --name-only master -- docs/tickets/closed/t.md)
  if [ $rc = 0 ] && echo "$sortie" | grep -qx POUSSE && [ -z "$ouverte" ] && [ -n "$fermee" ] && echo "$sortie" | grep -q "copies ouvertes retirées dans le même push : docs/a/tickets/t.md"; then
    echo "vert   un ticket fermé par le lot : la garde retire sa copie ouverte dans le même push"; exit 0
  fi
  echo "ROUGE  ticket fermé : rc $rc, ouverte « $ouverte », fermée « $fermee »"; echo "$sortie" | tail -3; exit 1
) || rouge=1
# Aucun journal qui nomme une lib : ARRÊT, même si la liste des journaux s'élargit à $J/*.log.
(
  H="$D/home0"; mkdir -p "$H/git_workspaces" "$H/.cache/rag3weaver-build/j"
  git clone -q --bare "$D" "$D/nu0.git" 2>/dev/null; git -C "$D/nu0.git" update-ref refs/heads/master "$BASE"
  git clone -q "$D/nu0.git" "$H/git_workspaces/wt" 2>/dev/null
  cd "$H/git_workspaces/wt" && git config user.email luciedefraiteur@gmail.com && git config user.name t
  git checkout -q --detach origin/master; echo 'fn z() {}' > extension/rag3weaver/src/z.rs; git add -A; git commit -qm z
  printf 'teste %s\nVERT\n' "$(git rev-parse --short HEAD)" > "$H/.cache/rag3weaver-build/j/resume.log"; git rev-parse origin/master > "$H/.cache/rag3weaver-build/j/base"
  echo "aucune lib ici" > "$H/.cache/rag3weaver-build/j/crate.log"
  sortie=$(HOME="$H" bash "$GARDE" wt j 2>&1); rc=$?
  if [ $rc != 0 ] && echo "$sortie" | grep -q "aucun journal de la passe"; then echo "vert   aucun journal ne nomme de lib : ARRÊT"; exit 0; fi
  echo "ROUGE  aucun journal ne nomme de lib : rc $rc — $(echo "$sortie" | tail -2)"; exit 1
) || rouge=1
# La dérive : la garde exécutée et celle du dépôt, identiques puis différentes ; jamais un refus.
git checkout -q "$BASE" 2>/dev/null
mkdir -p tools/poste && cp "$GARDE" tools/poste/push_garde.sh
d=$(bash "$GARDE" --derive "$D"); rc=$?
if [ $rc = 0 ] && echo "$d" | grep -qx "garde à jour"; then echo "vert   dérive : garde identique, « garde à jour »"; else echo "ROUGE  dérive identique : $d"; rouge=1; fi
echo "# une ligne de plus" >> tools/poste/push_garde.sh
d=$(bash "$GARDE" --derive "$D"); rc=$?
if [ $rc = 0 ] && echo "$d" | grep -q "la copie exécutée porte ce que le dépôt n'a pas encore"; then echo "vert   dérive : la copie exécutée en avance sur le dépôt, dit dans ce sens, sans refus"; else echo "ROUGE  dérive, copie en avance : $d"; rouge=1; fi
# le dépôt a changé la garde depuis la copie : la copie exécutée est une version passée de son historique
cp "$GARDE" tools/poste/push_garde.sh; git add tools/poste/push_garde.sh; git commit -qm "garde v1"
echo "# le dépôt change la garde" >> tools/poste/push_garde.sh; git add tools/poste/push_garde.sh; git commit -qm "garde v2"
d=$(bash "$GARDE" --derive "$D"); rc=$?
if [ $rc = 0 ] && echo "$d" | grep -q "le dépôt a changé la garde depuis la copie exécutée"; then echo "vert   dérive : le dépôt en avance sur la copie, dit dans ce sens, sans refus"; else echo "ROUGE  dérive, dépôt en avance : $d"; rouge=1; fi
git checkout -q "$BASE" 2>/dev/null
rm -f tools/poste/push_garde.sh
exit $rouge
