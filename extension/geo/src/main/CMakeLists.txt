add_library(geo_extension_main
    OBJECT
    geo_extension.cpp)

set(GEO_EXTENSION_OBJECT_FILES
    ${GEO_EXTENSION_OBJECT_FILES} $<TARGET_OBJECTS:geo_extension_main>
    PARENT_SCOPE)
