# --- Build Rust static lib via cargo ---
# The Rust FFI crate lives inside the ld-tantivy repo (workspace member).
# Cargo outputs to the workspace root's target/ directory.
set(RUST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../tantivy/ld-tantivy/tantivy_fts/rust)
set(RUST_WORKSPACE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../tantivy/ld-tantivy)

include_directories(
    ${PROJECT_SOURCE_DIR}/src/include
    ${CMAKE_BINARY_DIR}/src/include
    src/include
    # cxx bridge generated headers (bridge.rs.h + cxx.h runtime)
    ${RUST_WORKSPACE_DIR}/target/cxxbridge/tantivy-fts/src
    ${RUST_WORKSPACE_DIR}/target/cxxbridge/rust)

if(EMSCRIPTEN)
    set(CARGO_TARGET "--target" "wasm32-unknown-emscripten")
    set(CARGO_TARGET_DIR ${RUST_WORKSPACE_DIR}/target/wasm32-unknown-emscripten/release)
    set(CARGO_ENV
        "EMCC_CFLAGS=-pthread -fexceptions -sDISABLE_EXCEPTION_CATCHING=0"
        "RUSTFLAGS=-C target-feature=+atomics,+bulk-memory,+mutable-globals -C panic=abort")
    set(CARGO_TOOLCHAIN "+nightly")
    set(CARGO_EXTRA_FLAGS "-Z" "build-std=std,panic_abort")
else()
    set(CARGO_TARGET "")
    set(CARGO_TARGET_DIR ${RUST_WORKSPACE_DIR}/target/release)
    set(CARGO_ENV "")
    set(CARGO_TOOLCHAIN "")
    set(CARGO_EXTRA_FLAGS "")
endif()

set(TANTIVY_STATIC_LIB ${CARGO_TARGET_DIR}/libtantivy_fts.a)

# Collect Rust sources so cmake detects changes and re-runs cargo.
# Note: new files require cmake re-configuration, but edits are detected immediately.
file(GLOB_RECURSE RUST_SOURCES
    "${RUST_WORKSPACE_DIR}/src/*.rs"
    "${RUST_WORKSPACE_DIR}/tantivy_fts/rust/src/*.rs"
    "${RUST_WORKSPACE_DIR}/Cargo.toml"
    "${RUST_WORKSPACE_DIR}/tantivy_fts/rust/Cargo.toml"
)

add_custom_command(
    OUTPUT ${TANTIVY_STATIC_LIB}
    COMMAND ${CMAKE_COMMAND} -E env ${CARGO_ENV}
            cargo ${CARGO_TOOLCHAIN} build --release ${CARGO_TARGET} ${CARGO_EXTRA_FLAGS}
    WORKING_DIRECTORY ${RUST_SRC_DIR}
    DEPENDS ${RUST_SOURCES}
    COMMENT "Building tantivy-fts Rust static library"
)

add_custom_target(tantivy_fts_rust DEPENDS ${TANTIVY_STATIC_LIB})

add_library(tantivy_fts_lib STATIC IMPORTED GLOBAL)
set_target_properties(tantivy_fts_lib PROPERTIES
    IMPORTED_LOCATION ${TANTIVY_STATIC_LIB})
add_dependencies(tantivy_fts_lib tantivy_fts_rust)

# --- cxx bridge glue (C++ side) ---
set(CXX_BRIDGE_CC ${RUST_WORKSPACE_DIR}/target/cxxbridge/tantivy-fts/src/bridge.rs.cc)
add_library(tantivy_fts_extension_bridge OBJECT ${CXX_BRIDGE_CC})
set(TANTIVY_FTS_EXTENSION_OBJECT_FILES $<TARGET_OBJECTS:tantivy_fts_extension_bridge>)
add_dependencies(tantivy_fts_extension_bridge tantivy_fts_rust)

# --- C++ extension sources ---
add_subdirectory(src/main)
add_subdirectory(src/index)
add_subdirectory(src/function)
add_subdirectory(src/catalog)
add_subdirectory(test)

# --- Build extension lib ---
build_extension_lib(${BUILD_STATIC_EXTENSION} "tantivy_fts")

# --- Link Rust lib + system deps ---
target_link_libraries(rag3db_${EXTENSION_LIB_NAME}_extension
    PRIVATE
    tantivy_fts_lib)

if(NOT EMSCRIPTEN)
    target_link_libraries(rag3db_${EXTENSION_LIB_NAME}_extension
        PRIVATE pthread m dl)
endif()
