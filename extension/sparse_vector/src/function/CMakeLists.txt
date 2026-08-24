add_library(sparse_vector_extension_function
    OBJECT
    create_sparse_vector_index.cpp
    query_sparse_vector_index.cpp
    drop_sparse_vector_index.cpp
    sparse_search_function.cpp)

set(SPARSE_VECTOR_EXTENSION_OBJECT_FILES
    ${SPARSE_VECTOR_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:sparse_vector_extension_function>
    PARENT_SCOPE)

# Ces sources incluent bridge.rs.h, généré par la cible Rust via cxx. Sans cette
# dépendance, make -j peut compiler le C++ avant que le header existe :
# « fatal error: bridge.rs.h : Aucun fichier ou dossier de ce nom ». L'échec est
# intermittent, il dépend de l'ordonnancement — donc il passait parfois.
add_dependencies(sparse_vector_extension_function sparse_vector_rust)
