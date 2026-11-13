#!/bin/bash
# Témoin de scripts/garde_target.sh, sur un espace de travail jouet (deux
# paquets, `ir` et `app`) copié en deux arbres qui partagent un target.
# Usage : scripts/test_garde_target.sh [chemin/vers/garde_target.sh]
# Rend 0 si tout tient, 1 sinon ; chaque cas écrit OK ou ÉCHEC.
set -u
GARDE="${1:-$(cd "$(dirname "$0")" && pwd)/garde_target.sh}"
. "$GARDE"
ICI="$(mktemp -d "${TMPDIR:-/tmp}/test-garde-target.XXXXXX")"
trap 'rm -rf "$ICI"' EXIT
unset CARGO_TARGET_DIR RAG3WEAVER_TARGET_REPRENDRE
# Le témoin lit la sortie de cargo (« Compiling ir ») : sans couleur, quel que soit l'appelant (la
# CI pose CARGO_TERM_COLOR=always, et ses trois cas de reprise y rougissaient sur les codes).
export CARGO_TERM_COLOR=never
TRIPLET="$(rustc -vV | sed -n 's/^host: //p')"
echec=0
cas() { if [ "$1" = 0 ]; then echo "OK     $2"; else echo "ÉCHEC  $2"; echec=1; fi; }

arbre() { # $1 = dossier, $2 = valeur rendue par ir
  mkdir -p "$1/ir/src" "$1/app/src"
  printf '[workspace]\nmembers=["ir","app"]\nresolver="2"\n' > "$1/Cargo.toml"
  printf '[package]\nname="ir"\nversion="0.1.0"\nedition="2021"\n' > "$1/ir/Cargo.toml"
  printf 'pub fn valeur() -> &'"'"'static str { "%s" }\n' "$2" > "$1/ir/src/lib.rs"
  printf '[package]\nname="app"\nversion="0.1.0"\nedition="2021"\n[dependencies]\nir={path="../ir"}\n' > "$1/app/Cargo.toml"
  printf 'fn main() { println!("{}", ir::valeur()); }\n' > "$1/app/src/main.rs"
}
arbre "$ICI/ancien" ANCIEN
touch -d '2020-01-01' $(find "$ICI/ancien" -type f)
arbre "$ICI/neuf" NEUF
T="$ICI/target"
export CARGO_TARGET_DIR="$T"

# Le neuf bâtit, sur l'hôte et sous un triplet explicite (les bâtis du paquet npm).
garder_le_target "$T" "$ICI/neuf/ir/.." > /dev/null || exit 1
(cd "$ICI/neuf" && cargo build -q -p app && cargo build -q -p app --target "$TRIPLET") 2>/dev/null
garder_le_target "$T" "$ICI/neuf/ir/.." > /dev/null
[ "$(cat "$T/.arbre-du-target")" = "$(cd "$ICI/neuf" && pwd -P)" ]
cas $? "le chemin noté est canonique (pas de « .. »)"

# Le piège lui-même : sans garde, l'ancien arbre reprend l'artefact du neuf.
[ "$(cd "$ICI/ancien" && cargo run -q -p app 2>/dev/null)" = "NEUF" ]
cas $? "le piège existe : l'ancien arbre affiche NEUF sans garde"

garder_le_target "$T" "$ICI/ancien" > /dev/null
[ $? = 2 ]
cas $? "un autre arbre est refusé (code 2)"

RAG3WEAVER_TARGET_REPRENDRE=1 garder_le_target "$T" "$ICI/ancien" > /dev/null
cas $? "la reprise rend 0"
[ -z "$(find "$T" -path '*/.fingerprint/*' -prune -regextype posix-extended \
        -regex '.*/\.fingerprint/(ir|app)-[0-9a-f]{16}' -print)" ]
cas $? "aucune empreinte locale ne reste, hôte et triplet compris"

sortie="$(cd "$ICI/ancien" && cargo build -p app --target "$TRIPLET" 2>&1)"
echo "$sortie" | grep -q 'Compiling ir '
cas $? "après reprise, le triplet rebâtit ir"
sortie="$(cd "$ICI/ancien" && cargo run -p app 2>&1)"
echo "$sortie" | grep -q 'Compiling ir ' && [ "$(echo "$sortie" | tail -1)" = "ANCIEN" ]
cas $? "après reprise, l'hôte rebâtit ir et l'ancien arbre affiche ANCIEN"

touch "$ICI/ancien/ir/src/lib.rs"
(cd "$ICI/ancien" && cargo check -p app 2>&1) | grep -q 'Checking ir '
cas $? "un fichier local touché après reprise est revu (ligne « Checking ir »)"

# **Un target restauré par le cache de la CI** (13 novembre 2026) : Swatinem/rust-cache rend
# target/ non vide et sans son marqueur ; la garde le refusait, et la CI s'arrêtait avant tout
# test. La provenance se déclare (« le cache de la CI », pas l'arbre courant), puis la reprise
# efface les empreintes locales : aucune exemption, sans reprise c'est un refus.
PROVENANCE_CI="$(cd "$(dirname "$GARDE")" && pwd)/provenance_du_cache_ci.sh"
C="$ICI/target-du-cache"
mkdir -p "$C" && cp -a "$T/." "$C/" && rm -f "$C/.arbre-du-target"
garder_le_target "$C" "$ICI/ancien" > /dev/null 2>&1
[ $? = 2 ]
cas $? "un target restauré sans marqueur est refusé tel quel (code 2)"
bash "$PROVENANCE_CI" "$C" > /dev/null 2>&1
[ "$(cat "$C/.arbre-du-target" 2>/dev/null)" = "/cache-de-la-ci/rust-cache" ]
cas $? "la provenance du cache de la CI se déclare dans le marqueur"
bash "$PROVENANCE_CI" "$C" > /dev/null 2>&1 && [ "$(cat "$C/.arbre-du-target" 2>/dev/null)" = "/cache-de-la-ci/rust-cache" ]
cas $? "la déclarer deux fois ne change rien"
garder_le_target "$C" "$ICI/ancien" > /dev/null 2>&1
[ $? = 2 ]
cas $? "sans reprise, le target du cache est refusé comme celui d'un autre arbre"
RAG3WEAVER_TARGET_REPRENDRE=1 garder_le_target "$C" "$ICI/ancien" > /dev/null 2>&1
cas $? "avec la reprise, la garde rend 0"
[ "$(cat "$C/.arbre-du-target" 2>/dev/null)" = "$(cd "$ICI/ancien" && pwd -P)" ] \
  && [ -z "$(find "$C" -path '*/.fingerprint/*' -prune -regextype posix-extended \
        -regex '.*/\.fingerprint/(ir|app)-[0-9a-f]{16}' -print)" ]
cas $? "après la reprise, le marqueur dit l'arbre et aucune empreinte locale ne reste"
V="$ICI/target-du-cache-vide"; mkdir -p "$V"
bash "$PROVENANCE_CI" "$V" > /dev/null 2>&1 && [ ! -e "$V/.arbre-du-target" ]
cas $? "un target vide (pas de cache) ne reçoit aucune provenance"
P="$ICI/target-deja-marque"; mkdir -p "$P" && echo "$ICI/neuf" > "$P/.arbre-du-target" && touch "$P/x"
bash "$PROVENANCE_CI" "$P" > /dev/null 2>&1 && [ "$(cat "$P/.arbre-du-target")" = "$ICI/neuf" ]
cas $? "une provenance déjà dite ne se réécrit pas"

exit $echec
