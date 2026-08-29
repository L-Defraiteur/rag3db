#!/bin/bash
# Run rag3weaver E2E tests with a dedicated native build.
#
# This build includes all required extensions (vector, geo)
# and is isolated from other builds (WASM, nodejs, etc.).
#
# Usage:
#   ./run_e2e.sh                          # run all e2e_search tests (skip build if exists)
#   ./run_e2e.sh phase0                   # run tests matching "phase0"
#   ./run_e2e.sh --test e2e_phase0b       # run e2e_phase0b tests instead
#   ./run_e2e.sh --build                  # force rebuild rag3db before tests
#   ./run_e2e.sh --build-only             # just build, don't run tests
#   ./run_e2e.sh --no-cuda phase0         # accepté, sans effet (burn/wgpu, pas de CUDA)
#   ./run_e2e.sh --summary                # show only the per-suite summary at the end
#   ./run_e2e.sh --features openai-llm    # add features to the default set

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUILD="$ROOT/build/native-test"
WEAVER="$ROOT/extension/rag3weaver"

# ── Confiner la pression mémoire ────────────────────────────────────────────
#
# Le 27 août 2026 : le poste ramait, et ce n'était pas le CPU. Un gros build
# fait défiler des gigaoctets à travers le cache de fichiers ; ça déclenche de
# la récupération mémoire en continu, et avec `vm.swappiness=150` (défaut
# CachyOS, pensé pour zram) ce qui part en zram, c'est **le bureau** — Chrome,
# l'éditeur. On revient dessus, il faut décompresser : « ça galère », CPU à 3 %.
#
# La cure serait un `swapoff/swapon` après coup, qui demande root. La
# prévention ne le demande pas : on met le build dans son propre cgroup, avec
# une limite haute. Sous la limite, rien ne change ; au-dessus, c'est **son**
# cache à lui qui est récupéré, pas les pages des applications ouvertes.
#
# `RAG3WEAVER_BUILD_MEMORY_HIGH=0` désactive, une taille (`24G`) la change.
confined() {
  local high="${RAG3WEAVER_BUILD_MEMORY_HIGH:-16G}"
  if [ "$high" = "0" ] || ! command -v systemd-run >/dev/null 2>&1 \
     || [ ! -e /sys/fs/cgroup/user.slice/user-"$(id -u)".slice/cgroup.controllers ]; then
    "$@"
    return $?
  fi
  systemd-run --user --scope --quiet --collect -p MemoryHigh="$high" -- "$@"
}

# ── Tracer la charge, pendant la compilation comme pendant les tests ───────
#
# « Ma machine recommence à galérer » n'est pas une mesure. Un échantillon
# toutes les cinq secondes dans un TSV en est une, et elle survit à la passe :
# on peut y revenir le lendemain pour savoir *quand* ça a basculé.
#
# Ça démarre ici, avant le build, parce que c'est le build C++ qui coûte le
# plus cher — et c'est justement celui qu'on ne voyait pas.
#
# `RAG3WEAVER_CHARGE=0` désactive ; `RAG3WEAVER_CHARGE_INTERVALLE` change le pas.
CHARGE_LOG="${RAG3WEAVER_CHARGE_LOG:-$WEAVER/target/charge-last.tsv}"
CHARGE_PID=""
if [ "${RAG3WEAVER_CHARGE:-1}" != "0" ] && [ -x "$WEAVER/charge.py" ]; then
  mkdir -p "$(dirname "$CHARGE_LOG")"
  : > "$CHARGE_LOG"
  "$WEAVER/charge.py" --sortie "$CHARGE_LOG" \
    --intervalle "${RAG3WEAVER_CHARGE_INTERVALLE:-5}" &
  CHARGE_PID=$!
  trap '[ -n "$CHARGE_PID" ] && kill "$CHARGE_PID" 2>/dev/null || true' EXIT
  echo "▸ Charge tracée dans $CHARGE_LOG (tail -f pour suivre)"
fi

# Parse flags
BUILD_ONLY=false
FORCE_BUILD=false
NO_CUDA=false
SUMMARY=false
TEST_FILE=""
EXTRA_FEATURES=""
TEST_FILTER=""
EXTRA_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --build-only) BUILD_ONLY=true; FORCE_BUILD=true; shift ;;
    --build)      FORCE_BUILD=true; shift ;;
    --no-build)   shift ;;  # kept for compat, now the default
    --no-cuda)    NO_CUDA=true; shift ;;
    --summary)    SUMMARY=true; shift ;;
    --test)       shift; TEST_FILE="$1"; shift ;;
    --features)   shift; EXTRA_FEATURES="$1"; shift ;;
    -*)           EXTRA_ARGS+=("$1"); shift ;;
    *)            TEST_FILTER="$1"; shift ;;
  esac
done

# ── Build ──────────────────────────────────────────────────────────────────
# By default, skip build if librag3db.so already exists.
# Use --build to force rebuild (e.g. after changing rag3db C++ or extensions).

NEED_BUILD=false
if [ "$FORCE_BUILD" = true ]; then
  NEED_BUILD=true
elif [ ! -f "$BUILD/src/librag3db.so" ]; then
  echo "▸ No existing build found, building..."
  NEED_BUILD=true
fi

if [ "$NEED_BUILD" = true ]; then
  # Configure (or reconfigure)
  if [ ! -f "$BUILD/Makefile" ]; then
    echo "▸ Configuring native-test build..."
    mkdir -p "$BUILD"
    cd "$BUILD"
    cmake "$ROOT" \
      -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_EXTENSIONS="vector;geo" \
      -DBUILD_SHELL=FALSE \
      -DBUILD_TESTS=FALSE \
      -DBUILD_EXTENSION_TESTS=FALSE
  fi

  echo "▸ Building rag3db + extensions..."
  # Deux cœurs restent à la machine : c'est la compilation C++ qui fige le
  # poste, pas les tests (voir .cargo/config.toml à la racine).
  JOBS=$(( $(nproc) > 2 ? $(nproc) - 2 : 1 ))
  confined cmake --build "$BUILD" -j"$JOBS"
  echo "▸ Build done."
fi

if [ "$BUILD_ONLY" = true ]; then
  echo "✓ Build complete: $BUILD/src/librag3db.so"
  if [ -n "$CHARGE_PID" ]; then
    kill "$CHARGE_PID" 2>/dev/null || true
    CHARGE_PID=""
    echo "CHARGE"
    "$WEAVER/charge.py" --resume "$CHARGE_LOG" || true
  fi
  exit 0
fi

# ── Run tests ──────────────────────────────────────────────────────────────

cd "$WEAVER"

# Build the cargo test filter args
# Chemin produit : burn (wgpu — AMD/NVIDIA/Apple, un seul code). candle n'est
# plus une feature des E2E ; --no-cuda est accepté pour compatibilité et sans effet.
# Tout l'arsenal burn est dans le jeu : une suite qui ne tourne pas n'existe
# pas. burn-embedder (BGE-M3) et burn-ocr (PP-OCRv6 tiny) chargent leurs poids
# depuis ~/.cache/rag3weaver/ — téléchargés au premier passage.
# `--features a,b` ajoute au jeu.
#
# **Plus de `burn-llm`** (28 août 2026) : notre moteur ne fait pas d'inférence
# de LLM. Il fait l'embedding, le rerank, l'OCR — ce pour quoi un graphe burn
# local a un sens. Un LLM vient de llama.cpp ou d'un fournisseur distant.
FEATURES="rag3db-native,burn-embedder,burn-ocr,code${EXTRA_FEATURES:+,$EXTRA_FEATURES}"

CARGO_ARGS=(
  --features "$FEATURES"
)

if [ -n "$TEST_FILE" ]; then
  # Single test file specified via --test
  CARGO_ARGS+=(--test "$TEST_FILE")
else
  # Run ALL e2e test files
  for f in "$WEAVER"/tests/e2e_*.rs; do
    CARGO_ARGS+=(--test "$(basename "${f%.rs}")")
  done
fi

# Une suite en échec n'arrête pas les autres : sans ça, cargo s'arrête au
# premier binaire de test qui échoue, et le résumé affiche un total partiel
# qui ressemble à un total complet (25 août 2026 : « 89 passed » pour 17
# suites sur 28).
CARGO_ARGS+=(--no-fail-fast)
CARGO_ARGS+=(-- --ignored --nocapture)

if [ -n "$TEST_FILTER" ]; then
  CARGO_ARGS+=("$TEST_FILTER")
fi

CARGO_ARGS+=("${EXTRA_ARGS[@]}")

# Espace d'adressage : une base en mémoire réserve 1 TiB (Rag3dbConnection::
# IN_MEMORY_MAX_DB_SIZE), pas les 8 TiB de kuzu — sinon 24 tests parallèles
# dans un même processus dépassent les 128 TiB adressables et `in_memory()`
# échoue au hasard. Le script ne force rien : c'est le défaut de la
# bibliothèque qui est testé ici. RAG3DB_MAX_DB_SIZE reste surchargeable.

echo "▸ Running: cargo test ${CARGO_ARGS[*]}"

export PATH="/usr/local/cuda/bin:$PATH"
export LD_LIBRARY_PATH="$BUILD/src:/usr/local/cuda/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export CUDA_ROOT="/usr/local/cuda"

export RAG3DB_SHARED=1
export RAG3DB_LIBRARY_DIR="$BUILD/src"
export RAG3DB_INCLUDE_DIR="$BUILD/src"
export RAG3DB_ROOT="$ROOT"

# **Toute passe laisse son journal**, résumé ou non.
#
# Il vivait dans un `mktemp` effacé à la sortie, et seulement dans la branche
# `--summary` : une suite qui cassait ne laissait que des compteurs, et savoir
# *quel* test avait cassé demandait de tout relancer — une demi-heure pour
# retrouver une ligne qu'on avait déjà eue sous les yeux.
#
# Écrire d'abord, filtrer ensuite. Un appelant qui réduit la sortie à trois
# lignes (`| tail | grep`) ne détruit plus rien : le texte entier est ici.
E2E_LOG="${RAG3WEAVER_E2E_LOG:-$WEAVER/target/e2e-last.log}"
mkdir -p "$(dirname "$E2E_LOG")"

# **L'incrémental ne sert à rien ici, et il coûte cher.**
#
# Il paie sur le *deuxième* build de la *même* cible : on change une ligne, on
# rebuild, il réutilise. Une passe construit 34 binaires de test, chacun une
# fois — elle **écrit** le cache et ne le relit pratiquement jamais. C'est
# pourquoi `CARGO_INCREMENTAL=0` est le réglage de toute CI, et une passe E2E
# est de la CI. La boucle d'édition, elle, le garde : c'est là qu'il rapporte.
#
# Mesuré le 27 août 2026 : 124 Go d'incrémental accumulés en trois jours.
export CARGO_INCREMENTAL=0

# **Une pile à la première occasion, pas à la relance.**
#
# Sans ça, un panic obscur ne donne que son fichier:ligne — celui de la macro
# `panic!`, pas le chemin qui y a mené — et il faut relancer la passe pour
# apprendre ce qu'on aurait pu lire du premier coup. Une demi-heure pour une
# information qui était disponible gratuitement.
#
# `full` ne s'impose pas : la pile courte suffit et reste lisible.
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

# **Le ménage passe avant, pas « de temps en temps ».**
#
# Cargo suffixe chaque artefact d'une empreinte et ne supprime jamais les
# anciennes : à trois passes par jour, une centaine de gigas quotidiens. Ici,
# les périmés ne vivent jamais plus d'une passe. `RAG3WEAVER_NO_GC=1` l'évite.
if [ -z "${RAG3WEAVER_NO_GC:-}" ] && [ -x "$WEAVER/menage_target.py" ]; then
  "$WEAVER/menage_target.py" "$WEAVER/target" || true
fi

if [ "$SUMMARY" = true ]; then
  # Capture output, show summary at the end.
  #
  # **Le journal survit à la passe.** Il vivait dans un `mktemp` effacé à la
  # sortie : quand une suite échouait, il ne restait que des compteurs, et
  # savoir *quel* test avait cassé demandait de tout relancer — une demi-heure
  # pour retrouver une ligne qu'on avait déjà eue sous les yeux.
  #
  # Une passe qui échoue doit laisser de quoi regarder. C'est la même règle
  # que partout ici : rendre visible ce dont l'absence ne se voit pas.
  TMPLOG="$E2E_LOG"

  # **Le résumé va au journal, lui aussi.**
  #
  # `tee` ne capture que la sortie de cargo ; le bloc SUMMARY est écrit après,
  # directement à l'écran. Le journal s'arrêtait donc pile avant la partie qui
  # dit si la passe est verte — l'endroit exact où on regarde en premier.
  #
  # Deux `printf` plutôt qu'un pipe : pas de sous-shell, donc les compteurs
  # restent ceux du script, et pas de course à la fermeture du tube.
  say() { printf "$@"; printf "$@" >> "$E2E_LOG"; }
  set +e
  confined cargo test "${CARGO_ARGS[@]}" 2>&1 | tee "$TMPLOG"
  EXIT_CODE=${PIPESTATUS[0]}
  set -e

  say "\n═══════════════════════════════════════════════\n"
  say "  SUMMARY\n"
  say "═══════════════════════════════════════════════\n"

  TOTAL_PASSED=0
  TOTAL_FAILED=0

  # Collect Running/result lines in order
  mapfile -t SUITE_NAMES < <(grep -oP '(?<=Running tests/)\w+' "$TMPLOG")
  mapfile -t RESULTS < <(grep '^test result:' "$TMPLOG")

  for i in "${!RESULTS[@]}"; do
    line="${RESULTS[$i]}"
    suite="${SUITE_NAMES[$i]:-?}"
    passed=$(echo "$line" | grep -oP '\d+ passed' | grep -oP '\d+')
    failed=$(echo "$line" | grep -oP '\d+ failed' | grep -oP '\d+')
    TOTAL_PASSED=$((TOTAL_PASSED + ${passed:-0}))
    TOTAL_FAILED=$((TOTAL_FAILED + ${failed:-0}))

    if [ "${failed:-0}" -eq 0 ]; then
      say "  %-30s %3d passed\n" "$suite" "$passed"
    else
      say "  %-30s %3d passed, %d FAILED\n" "$suite" "$passed" "$failed"
    fi
  done

  # **Nommer ce qui a cassé.** Un compteur dit qu'il y a un problème ; il ne
  # dit pas lequel, et c'est justement ce qu'on vient chercher.
  mapfile -t FAILED_NAMES < <(grep -oP '^test \K[\w:]+(?= \.\.\. FAILED)' "$TMPLOG" | sort -u)
  if [ ${#FAILED_NAMES[@]} -gt 0 ]; then
    say "\n  échecs :\n"
    for t in "${FAILED_NAMES[@]}"; do
      say "    · %s\n" "$t"
    done
  fi

  # Les suites demandées qui n'ont pas rendu de résultat (compilation en
  # échec, abandon) : le total n'est pas un total.
  NOT_RUN=0
  for ((i = 0; i < ${#CARGO_ARGS[@]}; i++)); do
    if [ "${CARGO_ARGS[$i]}" = "--test" ]; then
      expected="${CARGO_ARGS[$((i + 1))]}"
      found=false
      for s in "${SUITE_NAMES[@]}"; do [ "$s" = "$expected" ] && found=true && break; done
      if [ "$found" = false ]; then
        say "  %-30s NOT RUN\n" "$expected"
        NOT_RUN=$((NOT_RUN + 1))
      fi
    fi
  done

  say "───────────────────────────────────────────────\n"
  if [ "$TOTAL_FAILED" -eq 0 ]; then
    say "  %-30s %3d passed\n" "TOTAL" "$TOTAL_PASSED"
  else
    say "  %-30s %3d passed, %d FAILED\n" "TOTAL" "$TOTAL_PASSED" "$TOTAL_FAILED"
  fi
  if [ "$NOT_RUN" -gt 0 ]; then
    say "  %-30s INCOMPLETE — %d suite(s) not run\n" "" "$NOT_RUN"
    [ "$EXIT_CODE" -eq 0 ] && EXIT_CODE=1
  fi
  say "═══════════════════════════════════════════════\n"

  # **Ce que la passe a coûté à la machine.** Un total de tests verts ne dit
  # pas si le poste était inutilisable pendant une demi-heure.
  if [ -n "$CHARGE_PID" ]; then
    kill "$CHARGE_PID" 2>/dev/null || true
    CHARGE_PID=""
    echo ""
    echo "  CHARGE"
    "$WEAVER/charge.py" --resume "$CHARGE_LOG" || true
    echo "  journal de charge : $CHARGE_LOG"
  fi
  echo "  journal complet : $E2E_LOG"
  exit "$EXIT_CODE"
else
  set +e
  confined cargo test "${CARGO_ARGS[@]}" 2>&1 | tee "$E2E_LOG"
  EXIT_CODE=${PIPESTATUS[0]}
  set -e
  echo ""
  if [ -n "$CHARGE_PID" ]; then
    kill "$CHARGE_PID" 2>/dev/null || true
    CHARGE_PID=""
    echo "CHARGE"
    "$WEAVER/charge.py" --resume "$CHARGE_LOG" || true
    echo "Journal de charge : $CHARGE_LOG"
  fi
  echo "Journal complet : $E2E_LOG"
  echo "Tip: run with --summary for a per-suite results table."
  exit "$EXIT_CODE"
fi
