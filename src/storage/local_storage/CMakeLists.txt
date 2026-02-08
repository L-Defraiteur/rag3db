add_library(rag3db_storage_local_storage
        OBJECT
        local_node_table.cpp
        local_rel_table.cpp
        local_storage.cpp)

set(ALL_OBJECT_FILES
        ${ALL_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_storage_local_storage>
        PARENT_SCOPE)
