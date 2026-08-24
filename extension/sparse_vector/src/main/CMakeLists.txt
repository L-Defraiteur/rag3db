add_library(sparse_vector_extension_main
    OBJECT
    sparse_vector_extension.cpp)

set(SPARSE_VECTOR_EXTENSION_OBJECT_FILES
    ${SPARSE_VECTOR_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:sparse_vector_extension_main>
    PARENT_SCOPE)

# Ces sources incluent bridge.rs.h, généré par la cible Rust via cxx. Sans cette
# dépendance, make -j peut compiler le C++ avant que le header existe :
# « fatal error: bridge.rs.h : Aucun fichier ou dossier de ce nom ». L'échec est
# intermittent, il dépend de l'ordonnancement — donc il passait parfois.
add_dependencies(sparse_vector_extension_main sparse_vector_rust)
