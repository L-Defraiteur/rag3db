add_library(rag3db_fts_index_catalog
        OBJECT
        fts_index_catalog_entry.cpp)

set(FTS_EXTENSION_OBJECT_FILES
        ${FTS_EXTENSION_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_fts_index_catalog>
        PARENT_SCOPE)
