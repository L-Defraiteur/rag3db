add_library(lucivy_fts_extension_catalog
    OBJECT
    lucivy_index_catalog_entry.cpp)

set(LUCIVY_FTS_EXTENSION_OBJECT_FILES
    ${LUCIVY_FTS_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:lucivy_fts_extension_catalog>
    PARENT_SCOPE)
