add_library(rag3db_function_cast
        OBJECT
        cast_array.cpp)

set(ALL_OBJECT_FILES
        ${ALL_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_function_cast>
        PARENT_SCOPE)
