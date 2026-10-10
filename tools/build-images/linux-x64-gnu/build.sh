#!/bin/bash
# Bâtit rag3weaver-backend (moteur en statique) et l'extension vecteur, et les
# dépose dans dist/linux-x64-gnu/ avec un relevé (bati.txt). Tourne dans
# l'image de ce dossier (dépôt monté sur /src, cache sur /cache), ou en natif
# sur un poste Linux qui a cmake, ninja et Rust : la recette est la même.
#
#   SRC     racine du dépôt (défaut : /src, ou le dépôt qui contient ce script)
#   CACHE   dossier des caches cargo (défaut : /cache ; en natif, ~/.cache/rag3weaver-build/paquet-npm)
#   JOBS    tâches de compilation (défaut : 8 — jamais tous les cœurs d'un poste de travail)
set -euo pipefail
ici=$(cd "$(dirname "$0")" && pwd)
if [ -z "${SRC:-}" ]; then
  if [ -d /src/extension/rag3weaver ]; then SRC=/src; else SRC=$(cd "$ici/../../.." && pwd); fi
fi
if [ -z "${CACHE:-}" ]; then
  if [ -d /cache ]; then CACHE=/cache; else CACHE=$HOME/.cache/rag3weaver-build/paquet-npm; fi
fi
JOBS=${JOBS:-8}
CIBLE=linux-x64-gnu
DIST=$SRC/dist/$CIBLE
FEATURES=rag3db-native,code,daemon,openai-llm

export CARGO_HOME=${CARGO_HOME:-$CACHE/cargo}
export CARGO_TARGET_DIR=$CACHE/target-$CIBLE
export CARGO_BUILD_JOBS=$JOBS NUM_JOBS=$JOBS CMAKE_BUILD_PARALLEL_LEVEL=$JOBS
export CMAKE_GENERATOR=Ninja
mkdir -p "$DIST" "$CARGO_HOME"
releve=$DIST/bati.txt
{
  echo "rag3weaver-backend · $CIBLE · $(date -u +%Y-%m-%dT%H:%MZ)"
  echo "dépôt : $(git -C "$SRC" rev-parse --short HEAD 2>/dev/null || echo '?')"
  echo "outils : $(rustc --version) · $(cmake --version | head -1) · $(gcc --version | head -1) · glibc $(ldd --version | head -1 | awk '{print $NF}')"
  echo "features : $FEATURES"
} | tee "$releve"

# 1. Le binaire. La crate rag3db bâtit le moteur en statique par cmake (tools/rust_api/build.rs).
d=$(date +%s)
( cd "$SRC/extension/rag3weaver" && cargo build --release --bin rag3weaver-backend --features "$FEATURES" )
echo "bâti du binaire : $(( ($(date +%s) - d) / 60 )) min $(( ($(date +%s) - d) % 60 )) s" | tee -a "$releve"

# 2. L'extension vecteur, que le backend charge à l'exécution (LOAD EXTENSION).
#    Réutilise le bâti cmake du moteur fait par cargo, en y ajoutant l'extension.
bdir=$(ls -d "$CARGO_TARGET_DIR"/release/build/rag3db-*/out/build | head -1)
d=$(date +%s)
cmake -S "$SRC" -B "$bdir" -DBUILD_EXTENSIONS=vector -DBUILD_SHELL=OFF -DBUILD_SINGLE_FILE_HEADER=OFF -DAUTO_UPDATE_GRAMMAR=OFF > "$DIST/cmake-vector.log" 2>&1
cmake --build "$bdir" --target rag3db_vector_extension -j "$JOBS" >> "$DIST/cmake-vector.log" 2>&1
echo "bâti de l'extension vecteur : $(( ($(date +%s) - d) / 60 )) min $(( ($(date +%s) - d) % 60 )) s" | tee -a "$releve"

# 3. Ce qu'on rend.
bin=$CARGO_TARGET_DIR/release/rag3weaver-backend
ext=$(find "$bdir" "$SRC/extension/vector/build" -name 'libvector.rag3db_extension' 2>/dev/null | head -1)
cp "$bin" "$DIST/rag3weaver-backend"
strip "$DIST/rag3weaver-backend"
[ -n "$ext" ] && cp "$ext" "$DIST/libvector.rag3db_extension" && strip "$DIST/libvector.rag3db_extension"
{
  echo "binaire avant strip : $(du -h "$bin" | cut -f1) ; après : $(du -h "$DIST/rag3weaver-backend" | cut -f1)"
  [ -n "$ext" ] && echo "extension vecteur après strip : $(du -h "$DIST/libvector.rag3db_extension" | cut -f1)" || echo "extension vecteur : INTROUVABLE"
  echo "bibliothèques partagées :"; ldd "$DIST/rag3weaver-backend" | sed 's/^/  /'
  echo "version glibc la plus haute demandée : $(objdump -T "$DIST/rag3weaver-backend" | grep -o 'GLIBC_[0-9.]*' | sort -t. -k2,2n -k3,3n | tail -1)"
  echo "target : $(du -sh "$CARGO_TARGET_DIR" | cut -f1)"
} | tee -a "$releve"
