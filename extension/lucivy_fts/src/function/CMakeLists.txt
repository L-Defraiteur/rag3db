add_library(lucivy_fts_extension_function
    OBJECT
    close_lucivy_index.cpp
    create_lucivy_index.cpp
    query_lucivy_index.cpp
    drop_lucivy_index.cpp
    flush_lucivy_index.cpp
    search_function.cpp)

set(LUCIVY_FTS_EXTENSION_OBJECT_FILES
    ${LUCIVY_FTS_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:lucivy_fts_extension_function>
    PARENT_SCOPE)

# Ces sources incluent bridge.rs.h, généré par la cible Rust via cxx. Sans cette
# dépendance, make -j peut compiler le C++ avant que le header existe :
# « fatal error: bridge.rs.h : Aucun fichier ou dossier de ce nom ». L'échec est
# intermittent, il dépend de l'ordonnancement — donc il passait parfois.
add_dependencies(lucivy_fts_extension_function lucivy_fts_rust)
