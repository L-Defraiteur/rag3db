include_directories(
    ${PROJECT_SOURCE_DIR}/src/include
    ${CMAKE_BINARY_DIR}/src/include
    src/include
    ${CMAKE_CURRENT_SOURCE_DIR}/../tantivy/ld-tantivy/tantivy_fts/include)   # tantivy_fts.h

# --- Build Rust static lib via cargo ---
# The Rust FFI crate lives inside the ld-tantivy repo (workspace member).
# Cargo outputs to the workspace root's target/ directory.
set(RUST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../tantivy/ld-tantivy/tantivy_fts/rust)
set(RUST_WORKSPACE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../tantivy/ld-tantivy)

if(EMSCRIPTEN)
    set(CARGO_TARGET "--target" "wasm32-unknown-emscripten")
    set(CARGO_TARGET_DIR ${RUST_WORKSPACE_DIR}/target/wasm32-unknown-emscripten/release)
    set(CARGO_ENV EMCC_CFLAGS=-pthread)
else()
    set(CARGO_TARGET "")
    set(CARGO_TARGET_DIR ${RUST_WORKSPACE_DIR}/target/release)
    set(CARGO_ENV "")
endif()

set(TANTIVY_STATIC_LIB ${CARGO_TARGET_DIR}/libtantivy_fts.a)

add_custom_command(
    OUTPUT ${TANTIVY_STATIC_LIB}
    COMMAND ${CMAKE_COMMAND} -E env ${CARGO_ENV}
            cargo build --release ${CARGO_TARGET}
    WORKING_DIRECTORY ${RUST_SRC_DIR}
    COMMENT "Building tantivy-fts Rust static library"
)

add_custom_target(tantivy_fts_rust DEPENDS ${TANTIVY_STATIC_LIB})

add_library(tantivy_fts_lib STATIC IMPORTED GLOBAL)
set_target_properties(tantivy_fts_lib PROPERTIES
    IMPORTED_LOCATION ${TANTIVY_STATIC_LIB})
add_dependencies(tantivy_fts_lib tantivy_fts_rust)

# --- C++ extension sources ---
add_subdirectory(src/main)

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
