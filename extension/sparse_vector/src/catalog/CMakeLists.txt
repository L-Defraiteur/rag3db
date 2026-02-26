add_library(sparse_vector_extension_catalog
    OBJECT
    sparse_vector_catalog_entry.cpp)

set(SPARSE_VECTOR_EXTENSION_OBJECT_FILES
    ${SPARSE_VECTOR_EXTENSION_OBJECT_FILES}
    $<TARGET_OBJECTS:sparse_vector_extension_catalog>
    PARENT_SCOPE)
