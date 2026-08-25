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
  cmake --build "$BUILD" -j"$(nproc)"
  echo "▸ Build done."
fi

if [ "$BUILD_ONLY" = true ]; then
  echo "✓ Build complete: $BUILD/src/librag3db.so"
  exit 0
fi

# ── Run tests ──────────────────────────────────────────────────────────────

cd "$WEAVER"

# Build the cargo test filter args
# Chemin produit : burn (wgpu — AMD/NVIDIA/Apple, un seul code). candle n'est
# plus une feature des E2E ; --no-cuda est accepté pour compatibilité et sans effet.
# Tout l'arsenal burn est dans le jeu : une suite qui ne tourne pas n'existe
# pas. burn-llm (Qwen2.5-0.5B, 996 Mo) et burn-ocr (PP-OCRv6 tiny) chargent
# leurs poids depuis ~/.cache/rag3weaver/ — téléchargés au premier passage.
# `--features a,b` ajoute au jeu.
FEATURES="rag3db-native,burn-embedder,burn-llm,burn-ocr${EXTRA_FEATURES:+,$EXTRA_FEATURES}"

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

if [ "$SUMMARY" = true ]; then
  # Capture output, show summary at the end
  TMPLOG=$(mktemp)
  trap 'rm -f "$TMPLOG"' EXIT
  set +e
  cargo test "${CARGO_ARGS[@]}" 2>&1 | tee "$TMPLOG"
  EXIT_CODE=${PIPESTATUS[0]}
  set -e

  echo ""
  echo "═══════════════════════════════════════════════"
  echo "  SUMMARY"
  echo "═══════════════════════════════════════════════"

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
      printf "  %-30s %3d passed\n" "$suite" "$passed"
    else
      printf "  %-30s %3d passed, %d FAILED\n" "$suite" "$passed" "$failed"
    fi
  done

  echo "───────────────────────────────────────────────"
  if [ "$TOTAL_FAILED" -eq 0 ]; then
    printf "  %-30s %3d passed\n" "TOTAL" "$TOTAL_PASSED"
  else
    printf "  %-30s %3d passed, %d FAILED\n" "TOTAL" "$TOTAL_PASSED" "$TOTAL_FAILED"
  fi
  echo "═══════════════════════════════════════════════"
  exit "$EXIT_CODE"
else
  cargo test "${CARGO_ARGS[@]}"
  echo ""
  echo "Tip: run with --summary for a per-suite results table."
fi
