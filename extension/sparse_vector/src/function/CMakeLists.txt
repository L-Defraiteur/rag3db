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
