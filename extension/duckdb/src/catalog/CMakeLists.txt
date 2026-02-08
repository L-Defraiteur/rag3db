add_library(rag3db_duckdb_catalog
        OBJECT
        duckdb_catalog.cpp
        duckdb_table_catalog_entry.cpp)

set(DUCKDB_EXTENSION_OBJECT_FILES
        ${DUCKDB_EXTENSION_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_duckdb_catalog>
        PARENT_SCOPE)
