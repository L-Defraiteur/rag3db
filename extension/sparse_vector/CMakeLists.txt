# --- Build Rust static lib via cargo ---
# The Rust FFI crate lives inside the extension directory (standalone, not in ld-tantivy).
set(RUST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/rust)

include_directories(
    ${PROJECT_SOURCE_DIR}/src/include
    ${CMAKE_BINARY_DIR}/src/include
    src/include
    # cxx bridge generated headers (bridge.rs.h + cxx.h runtime)
    ${RUST_SRC_DIR}/target/cxxbridge/sparse-vector/src
    ${RUST_SRC_DIR}/target/cxxbridge/rust)

if(EMSCRIPTEN)
    set(CARGO_TARGET "--target" "wasm32-unknown-emscripten")
    set(CARGO_TARGET_DIR ${RUST_SRC_DIR}/target/wasm32-unknown-emscripten/release)
    set(CARGO_ENV
        "EMCC_CFLAGS=-pthread -fexceptions -sDISABLE_EXCEPTION_CATCHING=0"
        "RUSTFLAGS=-C target-feature=+atomics,+bulk-memory,+mutable-globals -C panic=abort")
    set(CARGO_TOOLCHAIN "+nightly")
    set(CARGO_EXTRA_FLAGS "-Z" "build-std=std,panic_abort")
else()
    set(CARGO_TARGET "")
    set(CARGO_TARGET_DIR ${RUST_SRC_DIR}/target/release)
    set(CARGO_ENV "")
    set(CARGO_TOOLCHAIN "")
    set(CARGO_EXTRA_FLAGS "")
endif()

set(SPARSE_VECTOR_STATIC_LIB ${CARGO_TARGET_DIR}/libsparse_vector.a)

# Always invoke cargo — it has its own incremental compilation, so it's fast (<1s)
# when nothing changed.
add_custom_target(sparse_vector_rust
    COMMAND ${CMAKE_COMMAND} -E env ${CARGO_ENV}
            cargo ${CARGO_TOOLCHAIN} build --release ${CARGO_TARGET} ${CARGO_EXTRA_FLAGS}
    WORKING_DIRECTORY ${RUST_SRC_DIR}
    BYPRODUCTS ${SPARSE_VECTOR_STATIC_LIB}
    COMMENT "Building sparse-vector Rust static library"
)

# INTERFACE library with --whole-archive to ensure all Rust symbols are linked.
add_library(sparse_vector_lib INTERFACE)
target_link_libraries(sparse_vector_lib INTERFACE
    -Wl,--whole-archive ${SPARSE_VECTOR_STATIC_LIB} -Wl,--no-whole-archive)
add_dependencies(sparse_vector_lib sparse_vector_rust)

set(SPARSE_VECTOR_EXTENSION_OBJECT_FILES "")

# --- C++ extension sources ---
add_subdirectory(src/main)
add_subdirectory(src/index)
add_subdirectory(src/function)
add_subdirectory(src/catalog)
add_subdirectory(test)

# --- Build extension lib ---
build_extension_lib(${BUILD_STATIC_EXTENSION} "sparse_vector")

# --- Link Rust lib + system deps ---
target_link_libraries(rag3db_${EXTENSION_LIB_NAME}_extension
    PRIVATE
    sparse_vector_lib)

if(NOT EMSCRIPTEN)
    target_link_libraries(rag3db_${EXTENSION_LIB_NAME}_extension
        PRIVATE pthread m dl)
endif()
