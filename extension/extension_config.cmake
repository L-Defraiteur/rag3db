set(EXTENSION_LIST azure delta duckdb httpfs iceberg json llm postgres sqlite unity_catalog vector neo4j algo geo)

# Default extensions for native builds (vector, geo).
# FTS et index sparse vivent dans rag3weaver (lucivy-core / sparse-vector en Rust,
# dans le processus) — plus d'extension C++ pour eux. L'extension fts de l'amont
# a été retirée du dépôt le 10 octobre 2026 (elle ne se bâtissait plus depuis le
# 24 août) ; l'historique git la garde si rag3db est un jour donné seul.
# Override with -DBUILD_EXTENSIONS="ext1;ext2" on the cmake command line.
if("${BUILD_EXTENSIONS}" STREQUAL "")
    set(BUILD_EXTENSIONS "vector;geo" PARENT_SCOPE)
    set(BUILD_EXTENSIONS "vector;geo")
    message(STATUS "BUILD_EXTENSIONS not set, using default: vector;geo")
endif()

string(JOIN ", " joined_extensions ${EXTENSION_STATIC_LINK_LIST})
message(STATUS "Static link extensions: ${joined_extensions}")
foreach(extension IN LISTS EXTENSION_STATIC_LINK_LIST)
    add_static_link_extension(${extension})
endforeach()

if(${BUILD_WASM})
    message(STATUS "Building for WASM, extension static linking is enabled by default")
    set(WASM_DEFAULT_EXTENSIONS json vector algo)
    foreach(ext IN LISTS WASM_DEFAULT_EXTENSIONS)
        if(NOT ext IN_LIST WASM_EXCLUDE_EXTENSIONS)
            add_static_link_extension(${ext})
        else()
            message(STATUS "  Excluding WASM extension: ${ext}")
        endif()
    endforeach()
endif()

if(ANDROID_ABI)
    message(STATUS "Building for Android, extension static linking is enabled by default")
    add_static_link_extension(json)
    add_static_link_extension(vector)
    add_static_link_extension(algo)
endif()

if(${BUILD_SWIFT})
    message(STATUS "Building for Swift binding, extension static linking is enabled by default")
    add_static_link_extension(json)
    add_static_link_extension(vector)
    add_static_link_extension(algo)
endif()
