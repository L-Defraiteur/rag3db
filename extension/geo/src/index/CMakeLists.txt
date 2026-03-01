add_library(rag3db_geo_index
    OBJECT
    rtree.cpp
    rtree_index.cpp)

set(GEO_EXTENSION_OBJECT_FILES
    ${GEO_EXTENSION_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_geo_index>
    PARENT_SCOPE)
