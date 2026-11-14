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
pile() { # pile <attendu: tenue|arret> <description> : juge la pile préparée dans M, P1, P2T (testé), TETE
  local sortie rc
  sortie=$(bash "$GARDE" --pile "$P1" "$P2T" "$M" "$TETE" 2>&1); rc=$?;
  if { [ "$1" = tenue ] && [ $rc = 0 ]; } || { [ "$1" = arret ] && [ $rc != 0 ]; }; then echo "vert   pile $1 : $2"
  else echo "ROUGE  pile, attendu $1 (rc $rc) : $2 — $sortie"; rouge=1; fi
}
preparer_pile() { # le master d'origine, P1 sur lui, P2 sur P1 (le commit testé)
  git checkout -q -B m0 "$BASE"; M0=$(git rev-parse HEAD)
  git checkout -q -B p1 "$M0"; echo 'fn p1() {}' > extension/rag3weaver/src/p1.rs; git add -A; git commit -qm p1; P1=$(git rev-parse HEAD)
  git checkout -q -B p2 "$P1"; echo 'fn p2() {}' > extension/rag3weaver/src/p2.rs; echo '- p2' >> extension/rag3weaver/docs/nouveautes.md; git add -A; git commit -qm p2; P2T=$(git rev-parse HEAD)
}
# 1. Le cas vert : P1 atterri tel quel sur master, P2 reposé dessus.
preparer_pile
git checkout -q -B maitre "$P1"; M=$(git rev-parse HEAD)
git checkout -q -B p2b "$P2T"; git rebase -q --onto "$M" "$P1" >/dev/null 2>&1; TETE=$(git rev-parse HEAD)
pile tenue "P1 atterri tel quel, P2 reposé dessus"
# 2. La pile a changé : P1 a atterri avec un autre code que celui sur lequel P2 a été rejoué.
git checkout -q -B maitre "$M0"; echo 'fn p1() { autre(); }' > extension/rag3weaver/src/p1.rs; git add -A; git commit -qm "p1 changé"; M=$(git rev-parse HEAD)
git checkout -q -B p2b "$P2T"; git rebase -q --onto "$M" "$P1" >/dev/null 2>&1; TETE=$(git rev-parse HEAD)
pile arret "P1 a atterri avec un autre code : la pile a changé"
# 3. Un .md ajouté après le rejeu (le ticket posé après le rebase) : toléré.
git checkout -q -B maitre "$P1"; M=$(git rev-parse HEAD)
git checkout -q -B p2b "$P2T"; git rebase -q --onto "$M" "$P1" >/dev/null 2>&1
echo '# ticket' > extension/rag3weaver/docs/ticket.md; git add -A; git commit -qm "ticket"; TETE=$(git rev-parse HEAD)
pile tenue "un commit de .md ajouté après le rejeu"
# 4. Du code ajouté après le rejeu : refusé.
echo 'fn tard() {}' > extension/rag3weaver/src/tard.rs; git add -A; git commit -qm "code tardif"; TETE=$(git rev-parse HEAD)
pile arret "un commit de code ajouté après le rejeu"
# 5. Un conflit de nouveautes.md résolu au rebase (un `!` qui ne diffère que par des .md) : toléré.
git checkout -q -B maitre "$P1"; echo '- autre lot' >> extension/rag3weaver/docs/nouveautes.md; git add -A; git commit -qm "nouveauté d'un autre"; M=$(git rev-parse HEAD)
git checkout -q -B p2b "$P2T"; git rebase -q --onto "$M" "$P1" >/dev/null 2>&1 || { printf -- '- autre lot\n- p2\n' > extension/rag3weaver/docs/nouveautes.md; git add -A; GIT_EDITOR=true git rebase --continue >/dev/null 2>&1; }
TETE=$(git rev-parse HEAD)
pile tenue "un conflit de nouveautes.md résolu au rebase (! sur des .md seuls)"
# 6. Un commit du lot changé hors .md au rebase : refusé.
git checkout -q -B p2b "$P2T"; git rebase -q --onto "$M" "$P1" >/dev/null 2>&1 || { printf -- '- autre lot\n- p2\n' > extension/rag3weaver/docs/nouveautes.md; git add -A; GIT_EDITOR=true git rebase --continue >/dev/null 2>&1; }
echo 'fn p2() { change(); }' > extension/rag3weaver/src/p2.rs; git add -A; git commit -q --amend --no-edit; TETE=$(git rev-parse HEAD)
pile arret "un commit du lot changé hors des .md au rebase (! sur du code)"
# 7. La tête d'avant n'est pas un ancêtre du commit testé : le rejeu ne s'est pas fait sur la pile.
git checkout -q -B maitre "$P1"; M=$(git rev-parse HEAD)
git checkout -q -B p2b "$P2T"; git rebase -q --onto "$M" "$P1" >/dev/null 2>&1; TETE=$(git rev-parse HEAD)
git checkout -q -B ailleurs "$M0"; echo '# ailleurs' > extension/rag3weaver/docs/ailleurs.md; git add -A; git commit -qm ailleurs
P1_VRAI=$P1; P1=$(git rev-parse HEAD); pile arret "la tête d'avant n'est pas un ancêtre du commit testé"; P1=$P1_VRAI
git checkout -q "$BASE" 2>/dev/null
# La dérive : la garde exécutée et celle du dépôt, identiques puis différentes ; jamais un refus.
git checkout -q "$BASE" 2>/dev/null
mkdir -p tools/poste && cp "$GARDE" tools/poste/push_garde.sh
d=$(bash "$GARDE" --derive "$D"); rc=$?
if [ $rc = 0 ] && echo "$d" | grep -qx "garde à jour"; then echo "vert   dérive : garde identique, « garde à jour »"; else echo "ROUGE  dérive identique : $d"; rouge=1; fi
echo "# une ligne de plus" >> tools/poste/push_garde.sh
d=$(bash "$GARDE" --derive "$D"); rc=$?
if [ $rc = 0 ] && echo "$d" | grep -q "^garde en retard sur le dépôt"; then echo "vert   dérive : garde différente, « garde en retard », sans refus"; else echo "ROUGE  dérive différente : $d"; rouge=1; fi
rm -f tools/poste/push_garde.sh
exit $rouge
