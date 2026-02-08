add_library(rag3db_function_uuid
        OBJECT
        gen_random_uuid.cpp)

set(ALL_OBJECT_FILES
        ${ALL_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_function_uuid>
        PARENT_SCOPE)
