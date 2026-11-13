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
git add -A && git commit -qm base && BASE=$(git rev-parse HEAD)
rouge=0
cas() { # cas <attendu: code|docs> <chemin> <contenu>
  git checkout -q "$BASE" 2>/dev/null; mkdir -p "$(dirname "$2")"; printf '%s\n' "$3" > "$2"
  git add -A && git commit -qm "$2" && N=$(git rev-parse HEAD)
  sortie=$(bash "$GARDE" --code-apporte "$BASE" "$N")
  if [ -n "$sortie" ]; then vu=code; else vu=docs; fi
  if [ "$vu" = "$1" ]; then echo "vert   $1 : $2"; else echo "ROUGE  attendu $1, vu $vu : $2"; rouge=1; fi
}
# Le ticket : un outil que rien de joué n'appelle compte comme de la doc.
cas docs extension/rag3weaver/scripts/outil_de_mesure.py 'print("mesure")'
cas docs extension/rag3weaver/docs/note.md '# note'
cas docs extension/rag3weaver/docs/outils/sonde.sh 'echo sonde'
# Ce qui reste du code.
cas code extension/rag3weaver/scripts/appele.py 'print("appelé par run_e2e.sh")'
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
exit $rouge
