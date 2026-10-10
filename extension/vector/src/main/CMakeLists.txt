add_library(vector_extension_main
        OBJECT
        vector_extension.cpp)

# vector_extension.cpp exporte l'identifiant de bâti, régénéré à chaque bâti.
add_dependencies(vector_extension_main rag3db_build_id)

set(VECTOR_EXTENSION_OBJECT_FILES
        ${VECTOR_EXTENSION_OBJECT_FILES} $<TARGET_OBJECTS:vector_extension_main>
        PARENT_SCOPE)
