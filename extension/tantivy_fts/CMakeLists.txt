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

# Always invoke cargo — it has its own incremental compilation, so it's fast (<1s)
# when nothing changed. This eliminates cmake cache issues:
#   1. No stale .a from manual cargo builds
#   2. No missed changes when GLOB_RECURSE doesn't pick up new files
#   3. No need to delete binaries or re-configure cmake after editing .rs files
add_custom_target(tantivy_fts_rust
    COMMAND ${CMAKE_COMMAND} -E env ${CARGO_ENV}
            cargo ${CARGO_TOOLCHAIN} build --release ${CARGO_TARGET} ${CARGO_EXTRA_FLAGS}
    WORKING_DIRECTORY ${RUST_SRC_DIR}
    BYPRODUCTS ${TANTIVY_STATIC_LIB}
    COMMENT "Building tantivy-fts Rust static library"
)

# Use INTERFACE library (not IMPORTED STATIC) so cmake tracks the .a file's
# modification time and re-links C++ targets when cargo produces a new .a.
# IMPORTED STATIC doesn't trigger re-linking — this is a known cmake limitation.
# --whole-archive forces the linker to include ALL objects from the .a, preventing
# it from stripping Rust code that is only called internally (e.g. intersection.rs
# called by boolean_weight.rs). Without this, the linker only pulls in .o files
# that resolve undefined symbols from C++, missing internal Rust-to-Rust calls.
add_library(tantivy_fts_lib INTERFACE)
target_link_libraries(tantivy_fts_lib INTERFACE
    -Wl,--whole-archive ${TANTIVY_STATIC_LIB} -Wl,--no-whole-archive)
add_dependencies(tantivy_fts_lib tantivy_fts_rust)

# --- cxx bridge glue ---
# The C++ side of the cxx bridge (bridge.rs.cc) is compiled by the `cc` crate
# during `cargo build` and included in libtantivy_fts.a as bridge.rs.o.
# With --whole-archive, the linker includes it from the .a — no need to compile
# it separately in cmake (doing so causes "multiple definition" link errors).
set(TANTIVY_FTS_EXTENSION_OBJECT_FILES "")

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
