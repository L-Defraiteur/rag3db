# --- Build rag3weaver Rust static lib for WASM emscripten ---
# Only used in WASM builds. Native builds use cargo directly.

if(NOT EMSCRIPTEN)
    return()
endif()

set(RUST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})

set(CARGO_TARGET "--target" "wasm32-unknown-emscripten")
set(CARGO_TARGET_DIR ${RUST_SRC_DIR}/target/wasm32-unknown-emscripten/release)
set(CARGO_ENV
    "EMCC_CFLAGS=-pthread -fexceptions -sDISABLE_EXCEPTION_CATCHING=0"
    "RUSTFLAGS=-C target-feature=+atomics,+bulk-memory,+mutable-globals -C panic=abort")
set(CARGO_TOOLCHAIN "+nightly")
set(CARGO_EXTRA_FLAGS "-Z" "build-std=std,panic_abort")

set(RAG3WEAVER_STATIC_LIB ${CARGO_TARGET_DIR}/librag3weaver.a)

# Collect Rust sources for change detection
file(GLOB_RECURSE RUST_SOURCES
    "${RUST_SRC_DIR}/src/*.rs"
    "${RUST_SRC_DIR}/Cargo.toml"
)

add_custom_command(
    OUTPUT ${RAG3WEAVER_STATIC_LIB}
    COMMAND ${CMAKE_COMMAND} -E env ${CARGO_ENV}
            cargo ${CARGO_TOOLCHAIN} build --release ${CARGO_TARGET}
                  ${CARGO_EXTRA_FLAGS}
                  --no-default-features --features wasm-emscripten
    WORKING_DIRECTORY ${RUST_SRC_DIR}
    DEPENDS ${RUST_SOURCES}
    COMMENT "Building rag3weaver Rust static library for WASM"
)

add_custom_target(rag3weaver_rust DEPENDS ${RAG3WEAVER_STATIC_LIB})

add_library(rag3weaver_lib STATIC IMPORTED GLOBAL)
set_target_properties(rag3weaver_lib PROPERTIES
    IMPORTED_LOCATION ${RAG3WEAVER_STATIC_LIB})
add_dependencies(rag3weaver_lib rag3weaver_rust)
