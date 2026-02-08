add_library(tantivy_fts_extension_main
    OBJECT
    tantivy_fts_extension.cpp)

set(TANTIVY_FTS_EXTENSION_OBJECT_FILES
    ${TANTIVY_FTS_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:tantivy_fts_extension_main>
    PARENT_SCOPE)
