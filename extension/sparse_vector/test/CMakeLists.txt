if (${BUILD_EXTENSION_TESTS})
    add_rag3db_test(sparse_vector_test sparse_vector_test.cpp)
    # Ensure the extension shared lib is rebuilt when test sources change.
    add_dependencies(sparse_vector_test rag3db_sparse_vector_extension)
endif ()
