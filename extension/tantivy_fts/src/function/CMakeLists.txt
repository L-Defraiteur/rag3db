add_library(tantivy_fts_extension_function
    OBJECT
    create_tantivy_index.cpp
    query_tantivy_index.cpp
    drop_tantivy_index.cpp
    search_function.cpp)

set(TANTIVY_FTS_EXTENSION_OBJECT_FILES
    ${TANTIVY_FTS_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:tantivy_fts_extension_function>
    PARENT_SCOPE)
