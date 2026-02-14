add_library(tantivy_fts_extension_index
    OBJECT
    tantivy_index.cpp)

set(TANTIVY_FTS_EXTENSION_OBJECT_FILES
    ${TANTIVY_FTS_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:tantivy_fts_extension_index>
    PARENT_SCOPE)
