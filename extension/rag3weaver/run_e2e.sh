#!/bin/bash
# Run rag3weaver E2E tests with a dedicated native build.
#
# This build includes all required extensions (vector, tantivy_fts, sparse_vector)
# and is isolated from other builds (WASM, nodejs, etc.).
#
# Usage:
#   ./run_e2e.sh                          # run all e2e_search tests
#   ./run_e2e.sh phase0                   # run tests matching "phase0"
#   ./run_e2e.sh --test e2e_native        # run e2e_native tests instead
#   ./run_e2e.sh --build-only             # just build, don't run tests
#   ./run_e2e.sh --no-build phase1        # skip build, just run tests

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUILD="$ROOT/build/native-test"
WEAVER="$ROOT/extension/rag3weaver"

# Parse flags
BUILD_ONLY=false
SKIP_BUILD=false
TEST_FILE="e2e_search"
TEST_FILTER=""
EXTRA_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --build-only) BUILD_ONLY=true; shift ;;
    --no-build)   SKIP_BUILD=true; shift ;;
    --test)       shift; TEST_FILE="$1"; shift ;;
    -*)           EXTRA_ARGS+=("$1"); shift ;;
    *)            TEST_FILTER="$1"; shift ;;
  esac
done

# ── Build ──────────────────────────────────────────────────────────────────

if [ "$SKIP_BUILD" = false ]; then
  # Configure once
  if [ ! -f "$BUILD/Makefile" ]; then
    echo "▸ Configuring native-test build..."
    mkdir -p "$BUILD"
    cd "$BUILD"
    cmake "$ROOT" \
      -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_EXTENSIONS="vector;tantivy_fts;sparse_vector" \
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
CARGO_ARGS=(
  --features "rag3db-native,candle-embedder,bge-m3,cuda"
  --test "$TEST_FILE"
  --
  --ignored
  --nocapture
)

if [ -n "$TEST_FILTER" ]; then
  CARGO_ARGS+=("$TEST_FILTER")
fi

CARGO_ARGS+=("${EXTRA_ARGS[@]}")

echo "▸ Running: cargo test ${CARGO_ARGS[*]}"

export PATH="/usr/local/cuda/bin:$PATH"
export LD_LIBRARY_PATH="$BUILD/src:/usr/local/cuda/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export CUDA_ROOT="/usr/local/cuda"

RAG3DB_SHARED=1 \
RAG3DB_LIBRARY_DIR="$BUILD/src" \
RAG3DB_INCLUDE_DIR="$BUILD/src" \
RAG3DB_ROOT="$ROOT" \
exec cargo test "${CARGO_ARGS[@]}"
