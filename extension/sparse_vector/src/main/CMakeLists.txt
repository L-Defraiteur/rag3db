add_library(sparse_vector_extension_main
    OBJECT
    sparse_vector_extension.cpp)

set(SPARSE_VECTOR_EXTENSION_OBJECT_FILES
    ${SPARSE_VECTOR_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:sparse_vector_extension_main>
    PARENT_SCOPE)
