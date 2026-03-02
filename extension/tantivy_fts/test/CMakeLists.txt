if (${BUILD_EXTENSION_TESTS})
    add_rag3db_test(tantivy_fts_test tantivy_fts_test.cpp)
    add_dependencies(tantivy_fts_test rag3db_tantivy_fts_extension)
endif ()
