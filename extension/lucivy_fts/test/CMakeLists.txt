if (${BUILD_EXTENSION_TESTS})
    add_rag3db_test(lucivy_fts_test lucivy_fts_test.cpp)
    add_dependencies(lucivy_fts_test rag3db_lucivy_fts_extension)
endif ()
