#!/usr/bin/env bash
# build.sh — Build rag3db extensions and tests
#
# Usage:
#   ./build.sh              # configure + build all extensions + tests
#   ./build.sh test         # build + run all extension tests
#   ./build.sh clean        # delete build/release and reconfigure
#   ./build.sh <ext>        # build + test a single extension (e.g. ./build.sh tantivy_fts)
#
# Extensions built by default: vector, geo (FTS and sparse index live in rag3weaver, Rust)
# Override with: EXTENSIONS="vector" ./build.sh

set -euo pipefail
cd "$(dirname "$0")"

BUILD_DIR="build/release"
EXTENSIONS="${EXTENSIONS:-vector;geo}"
JOBS="${JOBS:-$(nproc)}"

# Check disk space (lesson from doc 13)
AVAIL_GB=$(df -BG --output=avail / | tail -1 | tr -d ' G')
if [ "$AVAIL_GB" -lt 10 ]; then
    echo "WARNING: Only ${AVAIL_GB}G free on /. Build may fail. Need at least 10G."
    read -p "Continue anyway? [y/N] " -r
    [[ $REPLY =~ ^[Yy]$ ]] || exit 1
fi

configure() {
    mkdir -p "$BUILD_DIR"
    cd "$BUILD_DIR"
    cmake ../.. \
        -DCMAKE_BUILD_TYPE=Release \
        -DBUILD_EXTENSION_TESTS=TRUE \
        -DBUILD_EXTENSIONS="$EXTENSIONS" \
        -DBUILD_SHELL=FALSE \
        -DBUILD_TESTS=FALSE
    cd ../..
}

build_all() {
    cd "$BUILD_DIR"
    cmake --build . -j"$JOBS"
    cd ../..
}

build_ext() {
    local ext="$1"
    cd "$BUILD_DIR"
    cmake --build . --target "rag3db_${ext}_extension" --target "${ext}_test" -j"$JOBS"
    cd ../..
}

run_tests() {
    local ext="$1"
    echo "=== Running ${ext} tests ==="
    LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu \
        "./$BUILD_DIR/extension/${ext}/test/${ext}_test"
}

run_all_tests() {
    IFS=';' read -ra EXT_ARRAY <<< "$EXTENSIONS"
    for ext in "${EXT_ARRAY[@]}"; do
        local test_bin="./$BUILD_DIR/extension/${ext}/test/${ext}_test"
        if [ -f "$test_bin" ]; then
            run_tests "$ext"
        else
            echo "=== No test binary for ${ext}, skipping ==="
        fi
    done
}

case "${1:-build}" in
    clean)
        echo "Cleaning $BUILD_DIR..."
        rm -rf "$BUILD_DIR"
        # Also clean stale .so in source tree
        IFS=';' read -ra EXT_ARRAY <<< "$EXTENSIONS"
        for ext in "${EXT_ARRAY[@]}"; do
            rm -f "extension/${ext}/build/"*.rag3db_extension
        done
        configure
        ;;
    test)
        if [ ! -f "$BUILD_DIR/CMakeCache.txt" ]; then
            configure
        fi
        build_all
        run_all_tests
        ;;
    build)
        if [ ! -f "$BUILD_DIR/CMakeCache.txt" ]; then
            configure
        fi
        build_all
        ;;
    *)
        # Assume it's an extension name
        EXT="$1"
        if [ ! -f "$BUILD_DIR/CMakeCache.txt" ]; then
            configure
        fi
        build_ext "$EXT"
        run_tests "$EXT"
        ;;
esac
