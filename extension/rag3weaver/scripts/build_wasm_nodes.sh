#!/bin/bash
# Builds the WebAssembly nodes shipped with the templates: each crate under wasm/ into tests/fixtures/wasm_nodes/ (the trial: not shipped in a template, it needs the feature wasm-nodes); the nodes/ folder that uses
# it. Needs the target wasm32-unknown-unknown (rustup target add wasm32-unknown-unknown).
set -euo pipefail
cd "$(dirname "$0")/.."
target=${CARGO_TARGET_DIR:-$PWD/target}/wasm-nodes
(cd wasm/previous_step && CARGO_TARGET_DIR="$target" cargo build --release --target wasm32-unknown-unknown)
cp "$target/wasm32-unknown-unknown/release/previous_step_wasm.wasm" tests/fixtures/wasm_nodes/previous_step.wasm
ls -l tests/fixtures/wasm_nodes/previous_step.wasm
