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
# Les empreintes des paquets locaux, dans les deux dispositions de cargo : avant 1.100,
# `<profil>/.fingerprint/<paquet>-<hachage>` ; depuis, `<profil>/build/<paquet>/<hachage>/fingerprint`.
empreintes_locales() {
  find "$1" \( -path '*/.fingerprint/*' -prune -regextype posix-extended \
      -regex '.*/\.fingerprint/(ir|app)-[0-9a-f]{16}' -print \) -o \
    \( -regextype posix-extended -regex '.*/build/(ir|app)/[0-9a-f]{16}/fingerprint' -print \) 2>/dev/null
}
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
[ -z "$(empreintes_locales "$T")" ]
cas $? "aucune empreinte locale ne reste, hôte et triplet compris, dans l'une ou l'autre disposition de cargo"

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
  && [ -z "$(empreintes_locales "$C")" ]
cas $? "après la reprise, le marqueur dit l'arbre et aucune empreinte locale ne reste"
V="$ICI/target-du-cache-vide"; mkdir -p "$V"
bash "$PROVENANCE_CI" "$V" > /dev/null 2>&1 && [ ! -e "$V/.arbre-du-target" ]
cas $? "un target vide (pas de cache) ne reçoit aucune provenance"
P="$ICI/target-deja-marque"; mkdir -p "$P" && echo "$ICI/neuf" > "$P/.arbre-du-target" && touch "$P/x"
bash "$PROVENANCE_CI" "$P" > /dev/null 2>&1 && [ "$(cat "$P/.arbre-du-target")" = "$ICI/neuf" ]
cas $? "une provenance déjà dite ne se réécrit pas"

# **Un sous-module ramené à un commit plus vieux** (14 novembre 2026, faux rouge de codeparsers) : le même arbre, mais le
# checkout du commit d'avant porte des dates plus vieilles que l'artefact, que cargo juge frais. La garde note le commit
# de chaque sous-module ; un commit changé efface les empreintes de ses paquets.
G="$ICI/git"; mkdir -p "$G"
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
mkdir -p "$G/sm/src" && (cd "$G/sm" && git init -q && printf '[package]\nname="ir"\nversion="0.1.0"\nedition="2021"\n' > Cargo.toml \
  && echo 'pub fn valeur() -> &'"'"'static str { "UN" }' > src/lib.rs && git add -A && git commit -qm un \
  && echo 'pub fn valeur() -> &'"'"'static str { "DEUX" }' > src/lib.rs && git commit -qam deux)
UN=$(git -C "$G/sm" rev-parse HEAD~1)
mkdir -p "$G/racine/app/src" && (cd "$G/racine" && git init -q \
  && git -c protocol.file.allow=always submodule add -q "$G/sm" sm > /dev/null 2>&1 \
  && printf '[workspace]\nmembers=["sm","app"]\nresolver="2"\n' > Cargo.toml \
  && printf '[package]\nname="app"\nversion="0.1.0"\nedition="2021"\n[dependencies]\nir={path="../sm"}\n' > app/Cargo.toml \
  && echo 'fn main() { println!("{}", ir::valeur()); }' > app/src/main.rs && git add -A && git commit -qm racine)
S="$ICI/target-sm"; export CARGO_TARGET_DIR="$S"
garder_le_target "$S" "$G/racine" > /dev/null 2>&1
[ "$(cd "$G/racine" && cargo run -q -p app 2>/dev/null)" = "DEUX" ]
cas $? "le sous-module à son dernier commit : l'application affiche DEUX"
# le commit d'avant, ses fichiers datés d'avant l'artefact (le checkout ne date pas, une restauration si)
git -C "$G/racine/sm" checkout -q "$UN" && touch -d '2020-01-01' $(find "$G/racine/sm" -type f -not -path '*/.git/*') "$G/racine/sm"
sortie="$(garder_le_target "$S" "$G/racine" 2>&1)"; rc=$?
[ $rc = 0 ] && echo "$sortie" | grep -q "autre commit du sous-module"
cas $? "un sous-module à un autre commit : la garde le dit et rend 0"
[ "$(cd "$G/racine" && cargo run -q -p app 2>/dev/null)" = "UN" ]
cas $? "après la garde, le paquet du sous-module est rebâti : l'application affiche UN"
sortie="$(garder_le_target "$S" "$G/racine" 2>&1)"
[ -z "$sortie" ]
cas $? "le même commit, la garde ne dit rien et n'efface rien"
# **--reprendre** (suggestion de codeparsers), avant un cargo direct : efface toujours, même pour l'arbre noté
echo 'fn main() { println!("TROIS"); }' > "$G/racine/app/src/main.rs"; touch -d '2020-01-01' "$G/racine/app/src/main.rs"
bash "$GARDE" --reprendre "$S" "$G/racine" > /dev/null 2>&1
[ $? = 0 ] && [ "$(cd "$G/racine" && cargo run -q -p app 2>/dev/null)" = "TROIS" ]
cas $? "garde_target.sh --reprendre efface les empreintes locales : un fichier daté d'avant est rebâti"
unset CARGO_TARGET_DIR

# **Le ménage au plafond** (15 novembre 2026, ticket d'archi) : `cargo clean -p` des paquets locaux, hôte et triplet, au
# lieu d'effacer tout le target ; un artefact tiers reste ; un target qui reste au-dessus rend 1.
P="$ICI/target-plafond"; export CARGO_TARGET_DIR="$P"
(cd "$ICI/neuf" && cargo build -q -p app && cargo build -q -p app --target "$TRIPLET") 2>/dev/null
mkdir -p "$P/debug/deps"; head -c 3145728 /dev/zero > "$P/debug/deps/libtiers-0123456789abcdef.rlib"
sortie="$(menager_au_plafond "$P" "$ICI/neuf" 1 2>&1)"; rc=$?
[ $rc = 1 ] && echo "$sortie" | grep -q "ménage des paquets locaux" && [ -f "$P/debug/deps/libtiers-0123456789abcdef.rlib" ]
cas $? "au plafond, le ménage garde l'artefact tiers, et rend 1 si le target reste au-dessus"
[ -z "$(empreintes_locales "$P")" ] && [ -z "$(find "$P" -path '*/incremental/*' -name 'app-*' -o -path '*/incremental/*' -name 'ir-*' 2>/dev/null)" ]
cas $? "le ménage efface les paquets locaux, incrémental compris, à l'hôte comme au triplet"
rm -f "$P/debug/deps/libtiers-0123456789abcdef.rlib"
(cd "$ICI/neuf" && cargo build -q -p app && cargo build -q -p app --target "$TRIPLET") 2>/dev/null
avant=$(du -xsm "$P" | cut -f1)
sortie="$(menager_au_plafond "$P" "$ICI/neuf" 1 2>&1)"; rc=$?
[ "$avant" -gt 1 ] && [ $rc = 0 ] && echo "$sortie" | grep -q "Mio →"
cas $? "un target local au-dessus du plafond redescend sous lui après le ménage, et le dit (avant ${avant} Mio)"
sortie="$(menager_au_plafond "$P" "$ICI/neuf" 1000 2>&1)"; rc=$?
[ $rc = 0 ] && [ -z "$sortie" ]
cas $? "sous le plafond, le ménage ne fait rien et ne dit rien"
export CARGO_TARGET_DIR="$T"

# **Un target né de la garde, sans CACHEDIR.TAG** (15 novembre 2026, relevé d'archi) : cargo refusait de le nettoyer, et le
# ménage tombait en silence. Le target naît de `garder_le_target` (mkdir puis le marqueur), comme ceux des passes.
Q="$ICI/target-ne-de-la-garde"; export CARGO_TARGET_DIR="$Q"
garder_le_target "$Q" "$ICI/neuf" > /dev/null 2>&1
(cd "$ICI/neuf" && cargo build -q -p app) 2>/dev/null
[ ! -f "$Q/CACHEDIR.TAG" ]
cas $? "le piège existe : un target né de la garde n'a pas de CACHEDIR.TAG"
sortie="$(menager_au_plafond "$Q" "$ICI/neuf" 1 2>&1)"; rc=$?
[ $rc = 0 ] && echo "$sortie" | grep -q "Mio →" && [ -z "$(empreintes_locales "$Q")" ]
cas $? "le ménage nettoie un target né de la garde (l'étiquette du cache posée), et le dit"
export CARGO_TARGET_DIR="$T"

exit $echec
