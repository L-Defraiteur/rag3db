#!/usr/bin/env bash
# rebuild.sh — WASM build with forced clean configure.
# Always deletes CMakeCache + generated loader to avoid stale extension linkage.
#
# Usage:
#   ./rebuild.sh                    # full build (all extensions)
#   ./rebuild.sh --no-sparse        # exclude sparse_vector
#   ./rebuild.sh --only-configure   # configure only, no build
#   ./rebuild.sh --dir build_wasm   # custom build dir (default: build_wasm)
#   ./rebuild.sh -j4                # custom parallelism

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SRC_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILD_DIR="$SRC_DIR/build_wasm"
EXCLUDE_EXTS=""
ONLY_CONFIGURE=false
JOBS="$(nproc)"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-sparse)
            EXCLUDE_EXTS="${EXCLUDE_EXTS:+$EXCLUDE_EXTS;}sparse_vector"
            shift ;;
        --no-tantivy)
            EXCLUDE_EXTS="${EXCLUDE_EXTS:+$EXCLUDE_EXTS;}tantivy_fts"
            shift ;;
        --no-*)
            ext="${1#--no-}"
            EXCLUDE_EXTS="${EXCLUDE_EXTS:+$EXCLUDE_EXTS;}${ext}"
            shift ;;
        --only-configure)
            ONLY_CONFIGURE=true
            shift ;;
        --dir)
            BUILD_DIR="$2"
            shift 2 ;;
        -j*)
            JOBS="${1#-j}"
            shift ;;
        *)
            echo "Unknown option: $1" >&2
            echo "Usage: $0 [--no-sparse] [--no-tantivy] [--only-configure] [--dir DIR] [-jN]" >&2
            exit 1 ;;
    esac
done

# Source emsdk
if [[ -z "${EMSDK:-}" ]]; then
    if [[ -f "$HOME/emsdk/emsdk_env.sh" ]]; then
        source "$HOME/emsdk/emsdk_env.sh" 2>/dev/null
    else
        echo "ERROR: emsdk not found. Source emsdk_env.sh first." >&2
        exit 1
    fi
fi

echo "=== WASM rebuild ==="
echo "  Source:    $SRC_DIR"
echo "  Build dir: $BUILD_DIR"
echo "  Jobs:      $JOBS"
[[ -n "$EXCLUDE_EXTS" ]] && echo "  Excluding: $EXCLUDE_EXTS"

mkdir -p "$BUILD_DIR"

# Force clean configure: delete cache + generated loader
rm -f "$BUILD_DIR/CMakeCache.txt"
rm -f "$BUILD_DIR/src/extension/codegen/generated_extension_loader.cpp"
rm -f "$BUILD_DIR/src/extension/codegen/include/generated_extension_loader.h"

echo "--- Configure ---"
CMAKE_EXTRA=""
if [[ -n "$EXCLUDE_EXTS" ]]; then
    CMAKE_EXTRA="-DWASM_EXCLUDE_EXTENSIONS=$EXCLUDE_EXTS"
fi

(cd "$BUILD_DIR" && emcmake cmake "$SRC_DIR" \
    -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_WASM=TRUE \
    -DBUILD_SHELL=FALSE \
    -DBUILD_TESTS=FALSE \
    -DBUILD_BENCHMARK=FALSE \
    $CMAKE_EXTRA)

if $ONLY_CONFIGURE; then
    echo "=== Configure done (--only-configure) ==="
    exit 0
fi

echo "--- Build ---"
cmake --build "$BUILD_DIR" -j "$JOBS"

echo "=== WASM build complete ==="
