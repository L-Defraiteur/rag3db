add_library(lucivy_fts_extension_index
    OBJECT
    lucivy_index.cpp)

set(LUCIVY_FTS_EXTENSION_OBJECT_FILES
    ${LUCIVY_FTS_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:lucivy_fts_extension_index>
    PARENT_SCOPE)
