add_library(tantivy_fts_extension_catalog
    OBJECT
    tantivy_index_catalog_entry.cpp)

set(TANTIVY_FTS_EXTENSION_OBJECT_FILES
    ${TANTIVY_FTS_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:tantivy_fts_extension_catalog>
    PARENT_SCOPE)
