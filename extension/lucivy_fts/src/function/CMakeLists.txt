add_library(lucivy_fts_extension_function
    OBJECT
    create_lucivy_index.cpp
    query_lucivy_index.cpp
    drop_lucivy_index.cpp
    flush_lucivy_index.cpp
    search_function.cpp)

set(LUCIVY_FTS_EXTENSION_OBJECT_FILES
    ${LUCIVY_FTS_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:lucivy_fts_extension_function>
    PARENT_SCOPE)
