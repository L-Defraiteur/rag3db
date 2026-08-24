add_subdirectory(${PROJECT_SOURCE_DIR}/extension/rag3weaver ${CMAKE_BINARY_DIR}/rag3weaver)

add_executable(rag3db_wasm
    src_cpp/main.cpp
    src_cpp/weaver_bindings.cpp
    ${PROJECT_SOURCE_DIR}/src/c_api/connection.cpp
    ${PROJECT_SOURCE_DIR}/src/c_api/database.cpp
    ${PROJECT_SOURCE_DIR}/src/c_api/data_type.cpp
    ${PROJECT_SOURCE_DIR}/src/c_api/helpers.cpp
    ${PROJECT_SOURCE_DIR}/src/c_api/flat_tuple.cpp
    ${PROJECT_SOURCE_DIR}/src/c_api/prepared_statement.cpp
    ${PROJECT_SOURCE_DIR}/src/c_api/query_result.cpp
    ${PROJECT_SOURCE_DIR}/src/c_api/query_summary.cpp
    ${PROJECT_SOURCE_DIR}/src/c_api/value.cpp
    ${PROJECT_SOURCE_DIR}/src/c_api/version.cpp
)

file(GLOB JS_SOURCE_FILES ./src_js/*)

# Allow duplicate symbols from rayon_core (linked independently by both
# rag3weaver static Rust library).
target_link_options(rag3db_wasm PRIVATE "LINKER:--allow-multiple-definition")
target_link_libraries(rag3db_wasm PRIVATE rag3db rag3weaver_lib)

set_target_properties(rag3db_wasm
    PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/package/nodejs/rag3db"
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/package/nodejs/rag3db"
    ARCHIVE_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/package/nodejs/rag3db"
)

file(COPY ${JS_SOURCE_FILES} DESTINATION  "${CMAKE_CURRENT_SOURCE_DIR}/build")
