add_library(lucivy_fts_extension_main
    OBJECT
    lucivy_fts_extension.cpp)

set(LUCIVY_FTS_EXTENSION_OBJECT_FILES
    ${LUCIVY_FTS_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:lucivy_fts_extension_main>
    PARENT_SCOPE)
