add_library(rag3db_geo_catalog
    OBJECT
    spatial_index_catalog_entry.cpp)

set(GEO_EXTENSION_OBJECT_FILES
    ${GEO_EXTENSION_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_geo_catalog>
    PARENT_SCOPE)
