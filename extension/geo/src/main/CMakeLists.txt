add_library(geo_extension_main
    OBJECT
    geo_extension.cpp)

# geo_extension.cpp exporte l'identifiant de bâti, régénéré à chaque bâti.
add_dependencies(geo_extension_main rag3db_build_id)

set(GEO_EXTENSION_OBJECT_FILES
    ${GEO_EXTENSION_OBJECT_FILES} $<TARGET_OBJECTS:geo_extension_main>
    PARENT_SCOPE)
