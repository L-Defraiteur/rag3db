add_library(sparse_vector_extension_index
    OBJECT
    sparse_vector_index.cpp)

set(SPARSE_VECTOR_EXTENSION_OBJECT_FILES
    ${SPARSE_VECTOR_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:sparse_vector_extension_index>
    PARENT_SCOPE)
