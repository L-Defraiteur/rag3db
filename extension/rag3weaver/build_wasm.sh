#!/usr/bin/env bash
# Build rag3weaver Rust static library for WASM emscripten.
# This is the ONLY way to build for WASM — do NOT use cargo build directly,
# the EMCC_CFLAGS and RUSTFLAGS are required for atomics/exceptions support.
#
# Usage: ./build_wasm.sh [--clean]
#
# The cmake custom_command in CMakeLists.txt calls the same cargo command
# with the same env vars. This script exists for manual rebuilds after
# modifying .rs files (cmake doesn't always detect Rust source changes).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if [[ "${1:-}" == "--clean" ]]; then
    echo "Cleaning WASM target..."
    rm -rf target/wasm32-unknown-emscripten/release/librag3weaver.a
    rm -rf target/wasm32-unknown-emscripten/release/deps/
fi

# These flags MUST match extension/rag3weaver/CMakeLists.txt exactly
export EMCC_CFLAGS="-pthread -fexceptions -sDISABLE_EXCEPTION_CATCHING=0"
export RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals -C panic=abort"

echo "Building rag3weaver for wasm32-unknown-emscripten (nightly + build-std)..."
echo "  EMCC_CFLAGS=$EMCC_CFLAGS"
echo "  RUSTFLAGS=$RUSTFLAGS"

cargo +nightly build --release \
    --target wasm32-unknown-emscripten \
    -Z build-std=std,panic_abort \
    --no-default-features --features wasm-emscripten

echo "Done: target/wasm32-unknown-emscripten/release/librag3weaver.a"
