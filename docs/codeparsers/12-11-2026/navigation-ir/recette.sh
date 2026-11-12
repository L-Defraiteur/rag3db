#!/bin/bash
# À lancer sous poste lourd ; aucun GPU/service, seules sept petites fixtures.
set -euo pipefail
W=$(git rev-parse --show-toplevel)
F=$W/docs/codeparsers/12-11-2026/navigation-ir/fixtures
P=$W/extension/rag3weaver/codeparsers
: "${CARGO_TARGET_DIR:?target privé unique requis}"
: "${PREUVES_NAVIGATION_IR:?dossier de preuve neuf hors worktree requis}"
[ ! -e "$PREUVES_NAVIGATION_IR" ] || { echo 'ROUGE : preuves déjà présentes'; exit 2; }
[ "$(git -C "$P" rev-parse HEAD)" = a730a9ba89ef233423bb7063f85181de26d98ce0 ]
[ -z "$(git -C "$P" status --porcelain)" ]
mkdir "$PREUVES_NAVIGATION_IR"
git rev-parse HEAD > "$PREUVES_NAVIGATION_IR/source.txt"
git -C "$P" rev-parse HEAD >> "$PREUVES_NAVIGATION_IR/source.txt"
sha256sum "$F"/* > "$PREUVES_NAVIGATION_IR/fixtures.sha256"
export CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=8
cd "$P"
cargo build --locked -j8 --example voir_json > "$PREUVES_NAVIGATION_IR/build.log" 2>&1
for source in "$F"/*; do
  "$CARGO_TARGET_DIR/debug/examples/voir_json" "$source" > "$PREUVES_NAVIGATION_IR/${source##*/}.json"
done
printf 'Sept fixtures analysées ; aucun verdict de couverture globale.\n' > "$PREUVES_NAVIGATION_IR/fin.txt"
